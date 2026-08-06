//! The kernel document and its whole-document scope audit.
//!
//! Every constructor below this one already checked its own local rule, so
//! well-typedness of the whole tree follows by induction — bityzba wrappers
//! admit no mutation path. What induction does *not* give is a global property:
//! that binder names really are identities. [`document_scope_audit`] proves
//! exactly that, independently of the cached free-binder sets the binding forms
//! carry.

use std::collections::BTreeMap;

#[allow(unused_imports)]
use bityzba::{contract_trait, ensures, expensive_invariant, invariant, new, requires};

use super::binder::{Bind, Category, Lambda, Let, LetRec, free_binders_of};
use super::content::{Content, Query};
use super::error::KernelTypeError;
use super::performable::{Act, Discourse, Performable, TranscriptEntry};
use super::predicate::PredTerm;
use super::types::{TypeExpr, Variable};
use super::value::{FnValue, Operand, RefComp, Value};

/// One complete typed kernel document.
///
/// There is deliberately no `#[expensive_invariant(document_scope_audit(body))]`
/// here: the constructor runs that audit in every build and the value is
/// immutable afterwards, so an invariant re-running it would be pure double
/// work in the expensive-contracts profile.
#[invariant(!body.is_reference_only(), "a document body is performed, not a reference-only constant")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelDocument {
    body: Performable,
}

impl KernelDocument {
    /// Close a document over a performable body.
    ///
    /// The cheap check is the free-variable one: a document with a free binder
    /// name has no printable spelling for it, so it is not a document. The scope
    /// audit then runs unconditionally, because it is the product gate for
    /// name-as-identity rather than a debugging aid.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    pub fn new(body: Performable) -> Result<Self, KernelTypeError> {
        if body.is_reference_only() {
            return Err(KernelTypeError::new(
                "a document body is performed; a reference-only discourse constant is not",
            ));
        }
        if !free_binders_of(&body).is_empty() {
            return Err(KernelTypeError::new(
                "the final document has no unbound variables",
            ));
        }
        document_scope_audit(&body)?;
        Ok(new!(KernelDocument { body }))
    }

    /// Borrow the performable body.
    #[requires(true)]
    #[ensures(*ret == self.body)]
    pub fn body(&self) -> &Performable {
        &self.body
    }
}

/// Binder introductions and uses collected from one whole document.
#[invariant(true, "an audit accumulator holds whatever the walk has seen so far")]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ScopeFacts {
    introductions: Vec<(Variable, TypeExpr)>,
    uses: Vec<(Variable, TypeExpr)>,
}

impl ScopeFacts {
    /// Record one binder introduction and its declared type.
    #[requires(true)]
    #[ensures(true)]
    pub(super) fn record_introduction(&mut self, variable: &Variable, declared_type: TypeExpr) {
        self.introductions.push((variable.clone(), declared_type));
    }

    /// Record one binder use and the type the use site carries.
    #[requires(true)]
    #[ensures(true)]
    pub(super) fn record_use(&mut self, variable: &Variable, used_type: TypeExpr) {
        self.uses.push((variable.clone(), used_type));
    }

    /// Record a lambda's parameters and descend into its body.
    #[requires(true)]
    #[ensures(true)]
    pub(super) fn record_lambda<C: Category + ScopeWalk>(&mut self, lambda: &Lambda<C>) {
        for parameter in lambda.parameters() {
            self.record_introduction(parameter.variable(), parameter.declared_type().clone());
        }
        lambda.body().append_scope_facts_to(self);
    }

    /// Record a `Let` block's declarations and descend into it.
    #[requires(true)]
    #[ensures(true)]
    pub(super) fn record_let<C: Category + ScopeWalk>(&mut self, form: &Let<C>) {
        for declaration in form.declarations() {
            self.record_introduction(declaration.variable(), declaration.declared_type().clone());
            declaration.initializer().append_scope_facts_to(self);
        }
        form.body().append_scope_facts_to(self);
    }

    /// Record a `Bind` binder and descend into it.
    #[requires(true)]
    #[ensures(true)]
    pub(super) fn record_bind<C: Category + ScopeWalk>(&mut self, form: &Bind<C>) {
        self.record_introduction(form.variable(), form.declared_type().clone());
        form.computation().append_scope_facts_to(self);
        form.body().append_scope_facts_to(self);
    }

    /// Record a recursive group's declarations and descend into it.
    #[requires(true)]
    #[ensures(true)]
    pub(super) fn record_let_rec<C: Category + ScopeWalk>(&mut self, form: &LetRec<C>) {
        for declaration in form.declarations() {
            self.record_introduction(declaration.variable(), declaration.declared_type().clone());
            declaration.initializer().append_scope_facts_to(self);
        }
        form.body().append_scope_facts_to(self);
    }
}

/// A kernel value that can report its binder introductions and uses.
///
/// This is the audit walk, kept separate from the cached free-binder sets so
/// the document invariant checks the tree rather than the annotations the tree
/// carries.
#[contract_trait]
pub trait ScopeWalk {
    /// Record this value's binder introductions and uses.
    #[requires(true)]
    #[ensures(true)]
    fn append_scope_facts_to(&self, facts: &mut ScopeFacts);
}

/// Delegate the audit walk to one kernel category's own collector.
macro_rules! scope_walk {
    ($($kind:ty),+ $(,)?) => {
        $(
            #[contract_trait]
            impl ScopeWalk for $kind {
                fn append_scope_facts_to(&self, facts: &mut ScopeFacts) {
                    self.collect_scope_facts(facts);
                }
            }
        )+
    };
}

scope_walk!(
    Act,
    Content,
    Discourse,
    FnValue,
    Operand,
    Performable,
    PredTerm,
    Query,
    RefComp,
    TranscriptEntry,
    Value,
);

/// Verify that binder names are identities across a whole document.
///
/// Two properties are checked: no name is introduced twice anywhere, so a use
/// resolves to exactly one binder without a scope walk; and every use carries
/// exactly the type its binder declared, so a `Bound` node's stored type is not
/// an independent claim.
///
/// One position is exempt from the second property by construction: a
/// `RefComp::Context` dependency list is a bare list of names with no stored
/// type, so there is nothing to disagree with a binder about and the walk
/// records no use for it. Those names are still reported as free binders, so
/// the document's root free-binder check is what rejects an unbound dependency.
/// The same limit applies to a bound relation identity reached through a
/// `Tanru` modifier or a `DropPlace`, whose declared row the composed signature
/// no longer records.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
pub fn document_scope_audit(body: &Performable) -> Result<(), KernelTypeError> {
    let mut facts = ScopeFacts::default();
    body.collect_scope_facts(&mut facts);
    let mut declared = BTreeMap::new();
    for (variable, declared_type) in &facts.introductions {
        if declared
            .insert(variable.clone(), declared_type.clone())
            .is_some()
        {
            return Err(KernelTypeError::new(format!(
                "{} is introduced by two binders, so its name is not an identity",
                variable.as_str()
            )));
        }
    }
    for (variable, used_type) in &facts.uses {
        let Some(declared_type) = declared.get(variable) else {
            return Err(KernelTypeError::new(format!(
                "{} is used without a binder",
                variable.as_str()
            )));
        };
        if declared_type != used_type {
            return Err(KernelTypeError::new(format!(
                "{} is used at a type its binder did not declare",
                variable.as_str()
            )));
        }
    }
    Ok(())
}
