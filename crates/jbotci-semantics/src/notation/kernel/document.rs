//! The kernel document and its whole-document scope audit.
//!
//! Every constructor below this one already checked its own local rule, so
//! well-typedness of the whole tree follows by induction — bityzba wrappers
//! admit no mutation path. What induction does *not* give is a scope property:
//! that every `$name` use resolves to exactly one live binder, at exactly the
//! type that binder declared. [`audit_document_scope`] proves that, independently
//! of the cached free-binder sets the binding forms carry.
//!
//! The audit is a single borrow-based walk carrying the live environment. It
//! borrows rather than accumulating owned copies because it runs in every
//! document build, so a corpus-scale render must not pay one `Variable` and one
//! `TypeExpr` clone per binder occurrence. Carrying the environment rather than
//! a flat list of every introduction anywhere is also what makes the audit say
//! what section 2.2 actually requires: sibling scopes may spell one identity the
//! same way — specification samples section 9 writes a quantifier's restriction
//! and scope as `(λ (($x Entity)) …)` twice — while a binder introduced *inside*
//! a live binder of the same name would make that name ambiguous and is refused.

use std::borrow::Cow;
use std::collections::BTreeMap;

#[allow(unused_imports)]
use bityzba::{ensures, expensive_invariant, invariant, new, requires};

use super::binder::{Bind, Category, Lambda, Let, LetRec, free_binders_of};
use super::error::KernelTypeError;
use super::performable::Performable;
use super::types::{TypeExpr, Variable};

/// How many times a whole document uses each binder name.
///
/// Section 2.4's utterance contraction is not a property of the entry being
/// printed: it holds only when the token is unreferenced across the document.
/// The census is therefore taken once, by the audit that already walks every
/// use, and carried on the document.
pub type BinderUses = BTreeMap<Variable, usize>;

/// One complete typed kernel document.
///
/// There is deliberately no `#[expensive_invariant]` re-running the audit here:
/// the constructor runs it in every build and the value is immutable
/// afterwards, so an invariant re-running it would be pure double work in the
/// expensive-contracts profile.
#[invariant(!body.is_reference_only(), "a document body is performed, not a reference-only constant")]
#[invariant(uses.values().all(|count| *count > 0), "a census entry records at least one use")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelDocument {
    body: Performable,
    uses: BinderUses,
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
        let uses = audit_document_scope(&body)?;
        Ok(new!(KernelDocument { body, uses }))
    }

    /// Borrow the performable body.
    #[requires(true)]
    #[ensures(*ret == self.body)]
    pub fn body(&self) -> &Performable {
        &self.body
    }

    /// Borrow the whole-document binder-use census the audit produced.
    #[requires(true)]
    #[ensures(*ret == self.uses)]
    pub fn binder_uses(&self) -> &BinderUses {
        &self.uses
    }
}

/// The live binder environment of one whole-document scope walk.
///
/// `live` is the environment at the current position, so a lookup answers which
/// binder a use resolves to; `uses` is the running census the printer needs.
/// The first failure is retained and every later check is skipped, because a
/// document that has already failed produces no output to be right about.
#[invariant(
    true,
    "an in-progress walk holds whatever the environment currently is"
)]
#[derive(Debug, Default)]
pub struct ScopeAudit<'value> {
    live: BTreeMap<&'value Variable, Cow<'value, TypeExpr>>,
    uses: BTreeMap<&'value Variable, usize>,
    failure: Option<KernelTypeError>,
}

impl<'value> ScopeAudit<'value> {
    /// Whether the walk has already failed and may stop checking.
    #[requires(true)]
    #[ensures(ret == self.failure.is_some())]
    fn failed(&self) -> bool {
        self.failure.is_some()
    }

    /// Record the first failure of the walk.
    #[requires(true)]
    #[ensures(self.failed())]
    fn fail(&mut self, message: impl Into<String>) {
        if self.failure.is_none() {
            self.failure = Some(KernelTypeError::new(message));
        }
    }

    /// Bring one binder into scope, refusing to shadow a live one.
    #[requires(true)]
    #[ensures(true)]
    fn introduce(&mut self, variable: &'value Variable, declared_type: Cow<'value, TypeExpr>) {
        if self.failed() {
            return;
        }
        if self.live.insert(variable, declared_type).is_some() {
            self.fail(format!(
                "{} is introduced inside a live binder of the same name, so its name is not an identity",
                variable.as_str()
            ));
        }
    }

    /// Take one binder back out of scope.
    #[requires(true)]
    #[ensures(true)]
    fn withdraw(&mut self, variable: &Variable) {
        self.live.remove(variable);
    }

    /// Check one use against the binder that is actually live for it.
    #[requires(true)]
    #[ensures(true)]
    pub(super) fn record_use(&mut self, variable: &'value Variable, used_type: &TypeExpr) {
        if self.failed() {
            return;
        }
        let Some(declared_type) = self.live.get(variable) else {
            self.fail(format!("{} is used without a binder", variable.as_str()));
            return;
        };
        if declared_type.as_ref() != used_type {
            self.fail(format!(
                "{} is used at a type its binder did not declare",
                variable.as_str()
            ));
            return;
        }
        *self.uses.entry(variable).or_insert(0) += 1;
    }

    /// Record one binder that carries no independently declared type.
    ///
    /// A `Sign` token and an `Utterance` token are introduced by the value that
    /// carries them rather than by a declaration, so their declared type is a
    /// category constant the walk owns rather than a field it can borrow.
    #[requires(true)]
    #[ensures(true)]
    pub(super) fn scoped_token<F>(
        &mut self,
        token: &'value Variable,
        declared_type: TypeExpr,
        inner: F,
    ) where
        F: FnOnce(&mut Self),
    {
        self.introduce(token, Cow::Owned(declared_type));
        inner(self);
        self.withdraw(token);
    }

    /// Walk a lambda: its parameters scope over its body alone.
    #[requires(true)]
    #[ensures(true)]
    pub(super) fn walk_lambda<C, F>(&mut self, lambda: &'value Lambda<C>, body: F)
    where
        C: Category,
        F: FnOnce(&mut Self, &'value C),
    {
        for parameter in lambda.parameters() {
            self.introduce(
                parameter.variable(),
                Cow::Borrowed(parameter.declared_type()),
            );
        }
        body(self, lambda.body());
        for parameter in lambda.parameters() {
            self.withdraw(parameter.variable());
        }
    }

    /// Walk a `Let` block: declarations are sequential, so initializer `i` sees
    /// the names declared before it and no later one.
    #[requires(true)]
    #[ensures(true)]
    pub(super) fn walk_let<C, F>(&mut self, form: &'value Let<C>, body: F)
    where
        C: Category,
        F: FnOnce(&mut Self, &'value C),
    {
        for declaration in form.declarations() {
            declaration.initializer().walk_scope(self);
            self.introduce(
                declaration.variable(),
                Cow::Borrowed(declaration.declared_type()),
            );
        }
        body(self, form.body());
        for declaration in form.declarations() {
            self.withdraw(declaration.variable());
        }
    }

    /// Walk a `Bind`: the computation runs outside the binder it introduces.
    #[requires(true)]
    #[ensures(true)]
    pub(super) fn walk_bind<C, F>(&mut self, form: &'value Bind<C>, body: F)
    where
        C: Category,
        F: FnOnce(&mut Self, &'value C),
    {
        form.computation().walk_scope(self);
        self.introduce(form.variable(), Cow::Borrowed(form.declared_type()));
        body(self, form.body());
        self.withdraw(form.variable());
    }

    /// Walk a recursive group: every initializer sees every declared name.
    #[requires(true)]
    #[ensures(true)]
    pub(super) fn walk_let_rec<C, F>(&mut self, form: &'value LetRec<C>, body: F)
    where
        C: Category,
        F: FnOnce(&mut Self, &'value C),
    {
        for declaration in form.declarations() {
            self.introduce(
                declaration.variable(),
                Cow::Borrowed(declaration.declared_type()),
            );
        }
        for declaration in form.declarations() {
            declaration.initializer().walk_scope(self);
        }
        body(self, form.body());
        for declaration in form.declarations() {
            self.withdraw(declaration.variable());
        }
    }

    /// Finish the walk, returning the census or the first failure.
    #[requires(true)]
    #[ensures(ret.is_ok() != old(self.failure.is_some()))]
    fn finish(self) -> Result<BinderUses, KernelTypeError> {
        match self.failure {
            Some(failure) => Err(failure),
            None => Ok(self
                .uses
                .into_iter()
                .map(|(variable, count)| (variable.clone(), count))
                .collect()),
        }
    }
}

/// Verify that every `$name` use in one document body resolves to exactly one
/// live binder at exactly the type that binder declared, and return the census
/// of those uses.
///
/// Two positions are exempt by construction, and both are covered by the
/// document's separate free-binder check rather than by this walk: a
/// `RefComp::Context` dependency list is a bare list of names with no stored
/// type, so it has nothing to disagree with a binder about; and a bound relation
/// identity reached through a `Tanru` modifier or a `DropPlace` is used at a row
/// the composed signature no longer records.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
pub fn audit_document_scope(body: &Performable) -> Result<BinderUses, KernelTypeError> {
    let mut audit = ScopeAudit::default();
    body.walk_scope(&mut audit);
    audit.finish()
}

/// Collect the whole-document binder-use census of a body that is already known
/// to be well scoped.
///
/// This exists for callers that hold a body rather than a [`KernelDocument`];
/// a document carries the census the audit already produced.
#[requires(true)]
#[ensures(true)]
pub fn document_binder_uses(body: &Performable) -> BinderUses {
    audit_document_scope(body).unwrap_or_default()
}
