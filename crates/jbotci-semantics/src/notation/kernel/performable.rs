//! Acts, discourse, transcript entries, and the performable union.
//!
//! Section 1.3 keeps closure, force construction, and performance distinct, and
//! so does the kernel: `Assert` takes content and returns an act, `Perform`
//! takes an act and returns discourse, and neither is an implicit consequence
//! of the other.

#[allow(unused_imports)]
use bityzba::{contract_trait, data, ensures, invariant, new, requires};

use super::binder::{Bind, BinderSet, Category, Let, LetRec};
use super::content::{Content, Query};
use super::document::ScopeFacts;
use super::error::KernelTypeError;
use super::intrinsic::{atom, kernel_accepts, referents};
use super::types::{Force, TypeAtom, TypeExpr, Variable};
use super::value::{Operand, Value};

/// A first-class act.
#[invariant(::Assert(_) => true)]
#[invariant(::Ask(_) => true)]
#[invariant(::Command { addressee, .. } =>
    kernel_accepts(&addressee.value_type(), &referents(atom(TypeAtom::Entity))),
    "a directive names its addressee as a reference to entities")]
#[invariant(::Express(_) => true)]
#[invariant(::Mention(_) => true)]
#[invariant(::Vocative(addressee) =>
    kernel_accepts(&addressee.value_type(), &referents(atom(TypeAtom::Entity))),
    "a vocative names its addressee as a reference to entities")]
#[invariant(::Interpret { sign, .. } =>
    matches!(sign.value_type(), TypeExpr::Sign(_) | TypeExpr::SignToken(_)),
    "InterpretAct interprets a sign or sign token")]
#[invariant(::Bound { .. } => true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Act {
    Assert(Content),
    Ask(Query),
    Command {
        addressee: Value,
        content: Content,
    },
    Express(Content),
    /// `Mention : A -> Act<Mentioning>` is deliberately polymorphic, so its
    /// operand is the joined operand category rather than a first-order value.
    Mention(Box<Operand>),
    Vocative(Value),
    /// `InterpretAct` with the force the graph or expected position fixed. The
    /// force is stored because section 3.3 refuses a force-erased result.
    Interpret {
        sign: Value,
        force: Force,
    },
    Bound {
        variable: Variable,
        force: Force,
    },
}

impl Act {
    /// Construct an assertion act.
    #[requires(true)]
    #[ensures(ret.force() == Force::Assertion)]
    pub fn assert(content: Content) -> Self {
        new!(Act::Assert(content))
    }

    /// Construct a direct question act.
    #[requires(true)]
    #[ensures(ret.force() == Force::Question)]
    pub fn ask(query: Query) -> Self {
        new!(Act::Ask(query))
    }

    /// Construct a directive act with its resolved addressee.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    pub fn command(addressee: Value, content: Content) -> Result<Self, KernelTypeError> {
        require_addressee(&addressee)?;
        Ok(new!(Act::Command { addressee, content }))
    }

    /// Construct an expressive act.
    #[requires(true)]
    #[ensures(ret.force() == Force::Expressive)]
    pub fn express(content: Content) -> Self {
        new!(Act::Express(content))
    }

    /// Construct a mentioning act over any first-class value.
    #[requires(true)]
    #[ensures(ret.force() == Force::Mentioning)]
    pub fn mention(operand: Operand) -> Self {
        new!(Act::Mention(Box::new(operand)))
    }

    /// Construct an address act.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    pub fn vocative(addressee: Value) -> Result<Self, KernelTypeError> {
        require_addressee(&addressee)?;
        Ok(new!(Act::Vocative(addressee)))
    }

    /// Interpret a sign as an act of a determined force.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    pub fn interpret(sign: Value, force: Force) -> Result<Self, KernelTypeError> {
        if !matches!(
            sign.value_type(),
            TypeExpr::Sign(_) | TypeExpr::SignToken(_)
        ) {
            return Err(KernelTypeError::new(
                "InterpretAct interprets a sign or sign token",
            ));
        }
        Ok(new!(Act::Interpret { sign, force }))
    }

    /// Reference a bound act.
    #[requires(true)]
    #[ensures(ret.force() == force)]
    pub fn bound(variable: Variable, force: Force) -> Self {
        new!(Act::Bound { variable, force })
    }

    /// Return this act's force index.
    #[requires(true)]
    #[ensures(true)]
    pub fn force(&self) -> Force {
        match self.as_data() {
            data!(Act::Assert(_)) => Force::Assertion,
            data!(Act::Ask(_)) => Force::Question,
            data!(Act::Command { .. }) => Force::Directive,
            data!(Act::Express(_)) => Force::Expressive,
            data!(Act::Mention(_)) => Force::Mentioning,
            data!(Act::Vocative(_)) => Force::Address,
            data!(Act::Interpret { force, .. }) | data!(Act::Bound { force, .. }) => *force,
        }
    }

    /// Record this act's binder introductions and uses.
    #[requires(true)]
    #[ensures(true)]
    pub(super) fn collect_scope_facts(&self, facts: &mut ScopeFacts) {
        match self.as_data() {
            data!(Act::Assert(content)) | data!(Act::Express(content)) => {
                content.collect_scope_facts(facts);
            }
            data!(Act::Ask(query)) => query.collect_scope_facts(facts),
            data!(Act::Command { addressee, content }) => {
                addressee.collect_scope_facts(facts);
                content.collect_scope_facts(facts);
            }
            data!(Act::Mention(operand)) => operand.collect_scope_facts(facts),
            data!(Act::Vocative(addressee)) => addressee.collect_scope_facts(facts),
            data!(Act::Interpret { sign, .. }) => sign.collect_scope_facts(facts),
            data!(Act::Bound { variable, force }) => {
                facts.record_use(variable, TypeExpr::Act(*force));
            }
        }
    }
}

#[contract_trait]
impl Category for Act {
    fn value_type(&self) -> TypeExpr {
        TypeExpr::Act(self.force())
    }

    fn append_free_binders_to(&self, binders: &mut BinderSet) {
        match self.as_data() {
            data!(Act::Assert(content)) | data!(Act::Express(content)) => {
                content.append_free_binders_to(binders);
            }
            data!(Act::Ask(query)) => query.append_free_binders_to(binders),
            data!(Act::Command { addressee, content }) => {
                addressee.append_free_binders_to(binders);
                content.append_free_binders_to(binders);
            }
            data!(Act::Mention(operand)) => operand.append_free_binders_to(binders),
            data!(Act::Vocative(addressee)) => addressee.append_free_binders_to(binders),
            data!(Act::Interpret { sign, .. }) => sign.append_free_binders_to(binders),
            data!(Act::Bound { variable, .. }) => {
                binders.insert(variable.clone());
            }
        }
    }
}

/// A performed act or discourse computation.
#[invariant(::Perform(_) => true)]
#[invariant(::PerformUtterance(_) => true)]
#[invariant(::Do(items) => items.len() >= 2 && items.iter().all(|item| !item.is_reference_only()),
    "Do sequences at least two performables and never performs a reference-only constant")]
#[invariant(::Joi(operands) => operands.len() >= 2
    && operands.iter().all(|operand| !operand.is_reference_only()),
    "a discourse Joi has at least two performed operands")]
#[invariant(::NewTopic(inner) => !inner.is_reference_only(),
    "a paragraph transition wraps a performed discourse")]
#[invariant(::Resume(inner) => !inner.is_reference_only(),
    "a paragraph transition wraps a performed discourse")]
#[invariant(::Prior => true)]
#[invariant(::Following => true)]
#[invariant(::Bound(_) => true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Discourse {
    Perform(Act),
    PerformUtterance(TranscriptEntry),
    Do(Vec<Performable>),
    Joi(Vec<Discourse>),
    NewTopic(Box<Discourse>),
    Resume(Box<Discourse>),
    /// `PriorDiscourse` and `FollowingDiscourse` are inert graph-owned
    /// references to a neighbouring segment. They are legal only as ordinary
    /// operands of a registered fact, never as something to perform.
    Prior,
    Following,
    Bound(Variable),
}

impl Discourse {
    /// Cross a stored act to discourse.
    #[requires(true)]
    #[ensures(true)]
    pub fn perform(act: Act) -> Self {
        new!(Discourse::Perform(act))
    }

    /// Cross a transcript entry to discourse.
    #[requires(true)]
    #[ensures(true)]
    pub fn perform_utterance(entry: TranscriptEntry) -> Self {
        new!(Discourse::PerformUtterance(entry))
    }

    /// Sequence two or more performables.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    pub fn sequence(items: Vec<Performable>) -> Result<Self, KernelTypeError> {
        if items.len() < 2 {
            return Err(KernelTypeError::new(
                "Do sequences at least two performables; one item contracts to that item",
            ));
        }
        if items.iter().any(Performable::is_reference_only) {
            return Err(KernelTypeError::new(
                "a reference-only discourse constant cannot be performed",
            ));
        }
        Ok(new!(Discourse::Do(items)))
    }

    /// Connect two or more discourse operands at a nonlogical locus.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    pub fn joi(operands: Vec<Discourse>) -> Result<Self, KernelTypeError> {
        if operands.len() < 2 {
            return Err(KernelTypeError::new(
                "a discourse Joi has two or more operands",
            ));
        }
        if operands.iter().any(Discourse::is_reference_only) {
            return Err(KernelTypeError::new(
                "a reference-only discourse constant cannot be performed",
            ));
        }
        Ok(new!(Discourse::Joi(operands)))
    }

    /// Record a paragraph transition.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    pub fn new_topic(inner: Discourse) -> Result<Self, KernelTypeError> {
        require_performed(&inner)?;
        Ok(new!(Discourse::NewTopic(Box::new(inner))))
    }

    /// Record a resumption transition.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    pub fn resume(inner: Discourse) -> Result<Self, KernelTypeError> {
        require_performed(&inner)?;
        Ok(new!(Discourse::Resume(Box::new(inner))))
    }

    /// Reference the immediately preceding represented discourse segment.
    #[requires(true)]
    #[ensures(ret.is_reference_only())]
    pub fn prior() -> Self {
        new!(Discourse::Prior)
    }

    /// Reference the immediately following represented discourse segment.
    #[requires(true)]
    #[ensures(ret.is_reference_only())]
    pub fn following() -> Self {
        new!(Discourse::Following)
    }

    /// Reference a bound discourse value.
    #[requires(true)]
    #[ensures(true)]
    pub fn bound(variable: Variable) -> Self {
        new!(Discourse::Bound(variable))
    }

    /// Report whether this value is one of the reference-only constants.
    ///
    /// The test is structural. Section 7.1's restriction also follows the value
    /// through identity-preserving bindings, which a bare `Bound` spelling
    /// cannot show, and nothing in this increment recovers it: the document
    /// audit checks name identity and use types, not what a name was bound to.
    /// A `PriorDiscourse` rebound by `Let` and then performed through its
    /// `$variable` is therefore still admitted, and closing that gap needs the
    /// binder environment the next increment introduces.
    #[requires(true)]
    #[ensures(true)]
    pub fn is_reference_only(&self) -> bool {
        matches!(
            self.as_data(),
            data!(Discourse::Prior) | data!(Discourse::Following)
        )
    }

    /// Record this value's binder introductions and uses.
    #[requires(true)]
    #[ensures(true)]
    pub(super) fn collect_scope_facts(&self, facts: &mut ScopeFacts) {
        match self.as_data() {
            data!(Discourse::Perform(act)) => act.collect_scope_facts(facts),
            data!(Discourse::PerformUtterance(entry)) => entry.collect_scope_facts(facts),
            data!(Discourse::Do(items)) => {
                for item in items {
                    item.collect_scope_facts(facts);
                }
            }
            data!(Discourse::Joi(operands)) => {
                for operand in operands {
                    operand.collect_scope_facts(facts);
                }
            }
            data!(Discourse::NewTopic(inner)) | data!(Discourse::Resume(inner)) => {
                inner.collect_scope_facts(facts);
            }
            data!(Discourse::Prior) | data!(Discourse::Following) => {}
            data!(Discourse::Bound(variable)) => {
                facts.record_use(variable, atom(TypeAtom::Discourse));
            }
        }
    }
}

#[contract_trait]
impl Category for Discourse {
    fn value_type(&self) -> TypeExpr {
        atom(TypeAtom::Discourse)
    }

    fn append_free_binders_to(&self, binders: &mut BinderSet) {
        match self.as_data() {
            data!(Discourse::Perform(act)) => act.append_free_binders_to(binders),
            data!(Discourse::PerformUtterance(entry)) => entry.append_free_binders_to(binders),
            data!(Discourse::Do(items)) => {
                for item in items {
                    item.append_free_binders_to(binders);
                }
            }
            data!(Discourse::Joi(operands)) => {
                for operand in operands {
                    operand.append_free_binders_to(binders);
                }
            }
            data!(Discourse::NewTopic(inner)) | data!(Discourse::Resume(inner)) => {
                inner.append_free_binders_to(binders);
            }
            data!(Discourse::Prior) | data!(Discourse::Following) => {}
            data!(Discourse::Bound(variable)) => {
                binders.insert(variable.clone());
            }
        }
    }
}

/// An utterance token together with its analyzer facts.
#[invariant(::Utterance { facts, .. } => !facts.is_empty(),
    "an utterance boundary carries at least one analyzer fact")]
#[invariant(::Bound(_) => true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TranscriptEntry {
    Utterance {
        token: Variable,
        facts: Vec<Content>,
    },
    Bound(Variable),
}

impl TranscriptEntry {
    /// Bind a fresh utterance token and its analyzer facts.
    #[requires(true)]
    #[ensures(ret.is_ok() == !old(facts.is_empty()))]
    pub fn utterance(token: Variable, facts: Vec<Content>) -> Result<Self, KernelTypeError> {
        if facts.is_empty() {
            return Err(KernelTypeError::new(
                "an Utterance boundary carries at least one analyzer fact",
            ));
        }
        Ok(new!(TranscriptEntry::Utterance { token, facts }))
    }

    /// Reference a bound transcript entry.
    #[requires(true)]
    #[ensures(true)]
    pub fn bound(variable: Variable) -> Self {
        new!(TranscriptEntry::Bound(variable))
    }

    /// Record this value's binder introductions and uses.
    #[requires(true)]
    #[ensures(true)]
    pub(super) fn collect_scope_facts(&self, entries: &mut ScopeFacts) {
        match self.as_data() {
            data!(TranscriptEntry::Utterance { token, facts }) => {
                entries.record_introduction(token, atom(TypeAtom::UtteranceToken));
                for fact in facts {
                    fact.collect_scope_facts(entries);
                }
            }
            data!(TranscriptEntry::Bound(variable)) => {
                entries.record_use(variable, atom(TypeAtom::TranscriptEntry));
            }
        }
    }
}

#[contract_trait]
impl Category for TranscriptEntry {
    fn value_type(&self) -> TypeExpr {
        atom(TypeAtom::TranscriptEntry)
    }

    fn append_free_binders_to(&self, binders: &mut BinderSet) {
        match self.as_data() {
            data!(TranscriptEntry::Utterance { token, facts }) => {
                let mut inner = BinderSet::new();
                for fact in facts {
                    fact.append_free_binders_to(&mut inner);
                }
                inner.remove(token);
                binders.extend(inner);
            }
            data!(TranscriptEntry::Bound(variable)) => {
                binders.insert(variable.clone());
            }
        }
    }
}

/// The closed union a document body and a `Do` item inhabit.
#[invariant(::Act(_) => true)]
#[invariant(::Discourse(_) => true)]
#[invariant(::Entry(_) => true)]
#[invariant(::Let(_) => true)]
#[invariant(::Bind(_) => true)]
#[invariant(::LetRec(_) => true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Performable {
    Act(Act),
    Discourse(Discourse),
    Entry(TranscriptEntry),
    Let(Let<Performable>),
    Bind(Bind<Performable>),
    LetRec(LetRec<Performable>),
}

impl Performable {
    /// Report whether performing this value would perform a reference-only
    /// discourse constant.
    #[requires(true)]
    #[ensures(true)]
    pub fn is_reference_only(&self) -> bool {
        match self {
            Self::Discourse(discourse) => discourse.is_reference_only(),
            Self::Let(form) => form.body().is_reference_only(),
            Self::Bind(form) => form.body().is_reference_only(),
            Self::LetRec(form) => form.body().is_reference_only(),
            Self::Act(_) | Self::Entry(_) => false,
        }
    }

    /// Record this value's binder introductions and uses.
    #[requires(true)]
    #[ensures(true)]
    pub(super) fn collect_scope_facts(&self, facts: &mut ScopeFacts) {
        match self {
            Self::Act(act) => act.collect_scope_facts(facts),
            Self::Discourse(discourse) => discourse.collect_scope_facts(facts),
            Self::Entry(entry) => entry.collect_scope_facts(facts),
            Self::Let(form) => facts.record_let(form),
            Self::Bind(form) => facts.record_bind(form),
            Self::LetRec(form) => facts.record_let_rec(form),
        }
    }
}

#[contract_trait]
impl Category for Performable {
    fn value_type(&self) -> TypeExpr {
        match self {
            Self::Act(act) => act.value_type(),
            Self::Discourse(discourse) => discourse.value_type(),
            Self::Entry(entry) => entry.value_type(),
            Self::Let(form) => form.body().value_type(),
            Self::Bind(form) => form.body().value_type(),
            Self::LetRec(form) => form.body().value_type(),
        }
    }

    fn append_free_binders_to(&self, binders: &mut BinderSet) {
        match self {
            Self::Act(act) => act.append_free_binders_to(binders),
            Self::Discourse(discourse) => discourse.append_free_binders_to(binders),
            Self::Entry(entry) => entry.append_free_binders_to(binders),
            Self::Let(form) => binders.extend(form.free_binders().iter().cloned()),
            Self::Bind(form) => binders.extend(form.free_binders().iter().cloned()),
            Self::LetRec(form) => binders.extend(form.free_binders().iter().cloned()),
        }
    }
}

/// Require an addressee to be a reference to entities.
#[requires(true)]
#[ensures(ret.is_ok() == kernel_accepts(&addressee.value_type(), &referents(atom(TypeAtom::Entity))))]
fn require_addressee(addressee: &Value) -> Result<(), KernelTypeError> {
    if !kernel_accepts(&addressee.value_type(), &referents(atom(TypeAtom::Entity))) {
        return Err(KernelTypeError::new(
            "an act addressee is a reference to entities",
        ));
    }
    Ok(())
}

/// Require a discourse operand that is actually performed.
#[requires(true)]
#[ensures(ret.is_ok() != inner.is_reference_only())]
fn require_performed(inner: &Discourse) -> Result<(), KernelTypeError> {
    if inner.is_reference_only() {
        return Err(KernelTypeError::new(
            "a reference-only discourse constant cannot be performed",
        ));
    }
    Ok(())
}
