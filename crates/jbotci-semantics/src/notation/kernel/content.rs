//! Closed dynamic content, queries, and answer selections.
//!
//! The logical operators, the quantifiers, `Presuppose`, `Supplement`, and
//! `Joi` are kernel forms rather than registered callables because their
//! accessibility and effect behaviour (section 6.2) is exact rather than
//! ordinary application composition. Everything whose behaviour *is* ordinary
//! application goes through [`Intrinsic`].

#[allow(unused_imports)]
use bityzba::{contract_trait, data, ensures, invariant, new, requires};

use super::binder::{Bind, BinderSet, Category, Lambda, Let, LetRec};
use super::document::ScopeFacts;
use super::error::KernelTypeError;
use super::intrinsic::{Intrinsic, atom};
use super::predicate::PredTerm;
use super::types::{AnswerExhaustivity, AnswerPolarity, Row, TypeAtom, TypeExpr, Variable};
use super::value::{FnValue, Operand, Value, operand_types};

/// The associative and nonlogical n-ary content connectives.
#[invariant(
    true,
    "each member is one closed connective; operand arity is a Content invariant"
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum JunctionOp {
    And,
    Or,
    /// The ordered nonlogical connection of section 10.2. It shares
    /// conjunction's context flow but retains its own connection identity, so
    /// it is never normalized into `And`.
    Joi,
}

impl JunctionOp {
    /// Return the canonical spelling.
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::And => "∧",
            Self::Or => "∨",
            Self::Joi => "Joi",
        }
    }
}

/// The binary truth-functional connectives.
#[invariant(true, "each member is one closed binary connective")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BinaryOp {
    Implies,
    Equivalent,
    ExclusiveOr,
}

impl BinaryOp {
    /// Return the canonical spelling.
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Implies => "→",
            Self::Equivalent => "↔",
            Self::ExclusiveOr => "⊕",
        }
    }
}

/// The two primitive quantifiers.
#[invariant(true, "each member is one closed primitive quantifier")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum QuantifierOp {
    /// The nonimporting mathematical universal. Source `ro` uses the importing
    /// `Every` prelude instead.
    ForAll,
    Exists,
}

impl QuantifierOp {
    /// Return the canonical spelling.
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ForAll => "∀",
            Self::Exists => "∃",
        }
    }
}

/// A closed, evaluable proposition-like value.
#[invariant(::Close(predicate) => row_is_closeable(&predicate.row()),
    "Close is defined only for a row whose surviving places can all be closed")]
#[invariant(::Not(_) => true)]
#[invariant(::Junction { operands, .. } => operands.len() >= 2,
    "a printed junction has at least two operands; one contracts to that operand")]
#[invariant(::Binary { .. } => true)]
#[invariant(::Quantified { .. } => true)]
#[invariant(::Presuppose { .. } => true)]
#[invariant(::Supplement { .. } => true)]
#[invariant(::Answer { query, selection } => selection.answers(query),
    "an answer selection matches its query's answer tuple")]
#[invariant(::Intrinsic { intrinsic, arguments } =>
    intrinsic.instantiate(&operand_types(arguments)) == Ok(TypeExpr::Atom(TypeAtom::Content)),
    "a content intrinsic call instantiates to Content")]
#[invariant(::Apply { head, arguments } => head.signature().is_some_and(|signature|
    signature.apply(&operand_types(arguments)) == Ok(&TypeExpr::Atom(TypeAtom::Content))),
    "a content application's callee returns Content")]
#[invariant(::Let(_) => true)]
#[invariant(::Bind(_) => true)]
#[invariant(::LetRec(_) => true)]
#[invariant(::Bound(_) => true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Content {
    Close(PredTerm),
    Not(Box<Content>),
    Junction {
        operator: JunctionOp,
        operands: Vec<Content>,
    },
    Binary {
        operator: BinaryOp,
        left: Box<Content>,
        right: Box<Content>,
    },
    Quantified {
        operator: QuantifierOp,
        lambda: Lambda<Content>,
    },
    Presuppose {
        trigger: Box<Content>,
        body: Box<Content>,
    },
    Supplement {
        body: Box<Content>,
        side: Box<Content>,
    },
    Answer {
        query: Box<Query>,
        selection: AnswerSelection,
    },
    Intrinsic {
        intrinsic: Intrinsic,
        arguments: Vec<Operand>,
    },
    Apply {
        head: FnValue,
        arguments: Vec<Operand>,
    },
    Let(Let<Content>),
    Bind(Bind<Content>),
    LetRec(LetRec<Content>),
    Bound(Variable),
}

impl Content {
    /// Close a predicate term into content.
    ///
    /// The kernel keeps this crossing explicit even where section 5.2 lets the
    /// notation omit it; eliding it is the printer's decision.
    #[requires(true)]
    #[ensures(ret.is_ok() == old(row_is_closeable(&predicate.row())))]
    pub fn close(predicate: PredTerm) -> Result<Self, KernelTypeError> {
        if !row_is_closeable(&predicate.row()) {
            return Err(KernelTypeError::new(
                "Close is undefined for this row: an unknown tail or a place with no ordinary default survives",
            ));
        }
        Ok(new!(Content::Close(predicate)))
    }

    /// Negate content.
    #[requires(true)]
    #[ensures(true)]
    pub fn not(inner: Content) -> Self {
        new!(Content::Not(Box::new(inner)))
    }

    /// Join two or more operands at one connective locus.
    #[requires(true)]
    #[ensures(ret.is_ok() == (old(operands.len()) >= 2))]
    pub fn junction(operator: JunctionOp, operands: Vec<Content>) -> Result<Self, KernelTypeError> {
        if operands.len() < 2 {
            return Err(KernelTypeError::new(
                "a junction has at least two operands; a one-operand occurrence contracts",
            ));
        }
        Ok(new!(Content::Junction { operator, operands }))
    }

    /// Combine two operands with a binary truth-functional connective.
    #[requires(true)]
    #[ensures(true)]
    pub fn binary(operator: BinaryOp, left: Content, right: Content) -> Self {
        new!(Content::Binary {
            operator,
            left: Box::new(left),
            right: Box::new(right),
        })
    }

    /// Quantify over an ordinary typed lambda.
    #[requires(true)]
    #[ensures(true)]
    pub fn quantified(operator: QuantifierOp, lambda: Lambda<Content>) -> Self {
        new!(Content::Quantified { operator, lambda })
    }

    /// Emit a projective trigger at its handler and then evaluate the body.
    #[requires(true)]
    #[ensures(true)]
    pub fn presuppose(trigger: Content, body: Content) -> Self {
        new!(Content::Presuppose {
            trigger: Box::new(trigger),
            body: Box::new(body),
        })
    }

    /// Evaluate the body in situ and commit its supplement at its handler.
    #[requires(true)]
    #[ensures(true)]
    pub fn supplement(body: Content, side: Content) -> Self {
        new!(Content::Supplement {
            body: Box::new(body),
            side: Box::new(side),
        })
    }

    /// State the answer selection of a query.
    #[requires(true)]
    #[ensures(ret.is_ok() == old(selection.answers(&query)))]
    pub fn answer(query: Query, selection: AnswerSelection) -> Result<Self, KernelTypeError> {
        if !selection.answers(&query) {
            return Err(KernelTypeError::new(
                "the answer selection does not match its query's answer tuple",
            ));
        }
        Ok(new!(Content::Answer {
            query: Box::new(query),
            selection,
        }))
    }

    /// Apply a registered callable whose result is content.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    pub fn intrinsic(
        intrinsic: Intrinsic,
        arguments: Vec<Operand>,
    ) -> Result<Self, KernelTypeError> {
        let result = intrinsic.instantiate(&operand_types(&arguments))?;
        if result != atom(TypeAtom::Content) {
            return Err(KernelTypeError::new(
                "this registered callable does not return Content",
            ));
        }
        Ok(new!(Content::Intrinsic {
            intrinsic,
            arguments,
        }))
    }

    /// Apply a callable value whose result is content.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    pub fn apply(head: FnValue, arguments: Vec<Operand>) -> Result<Self, KernelTypeError> {
        let signature = head
            .signature()
            .ok_or_else(|| KernelTypeError::new("only a callable value can be applied"))?;
        if signature.apply(&operand_types(&arguments))? != &atom(TypeAtom::Content) {
            return Err(KernelTypeError::new(
                "this application does not return Content",
            ));
        }
        Ok(new!(Content::Apply { head, arguments }))
    }

    /// Bind a block of inert declarations over a content body.
    #[requires(true)]
    #[ensures(true)]
    pub fn let_form(form: Let<Content>) -> Self {
        new!(Content::Let(form))
    }

    /// Run a reference computation over a content body.
    #[requires(true)]
    #[ensures(true)]
    pub fn bind_form(form: Bind<Content>) -> Self {
        new!(Content::Bind(form))
    }

    /// Tie a recursive function group over a content body.
    #[requires(true)]
    #[ensures(true)]
    pub fn let_rec_form(form: LetRec<Content>) -> Self {
        new!(Content::LetRec(form))
    }

    /// Reference bound content.
    #[requires(true)]
    #[ensures(true)]
    pub fn bound(variable: Variable) -> Self {
        new!(Content::Bound(variable))
    }

    /// Record this value's binder introductions and uses.
    #[requires(true)]
    #[ensures(true)]
    pub(super) fn collect_scope_facts(&self, facts: &mut ScopeFacts) {
        match self.as_data() {
            data!(Content::Close(predicate)) => predicate.collect_scope_facts(facts),
            data!(Content::Not(inner)) => inner.collect_scope_facts(facts),
            data!(Content::Junction { operands, .. }) => {
                for operand in operands {
                    operand.collect_scope_facts(facts);
                }
            }
            data!(Content::Binary { left, right, .. }) => {
                left.collect_scope_facts(facts);
                right.collect_scope_facts(facts);
            }
            data!(Content::Quantified { lambda, .. }) => facts.record_lambda(lambda),
            data!(Content::Presuppose { trigger, body }) => {
                trigger.collect_scope_facts(facts);
                body.collect_scope_facts(facts);
            }
            data!(Content::Supplement { body, side }) => {
                body.collect_scope_facts(facts);
                side.collect_scope_facts(facts);
            }
            data!(Content::Answer { query, selection }) => {
                query.collect_scope_facts(facts);
                selection.collect_scope_facts(facts);
            }
            data!(Content::Intrinsic { arguments, .. }) => {
                for argument in arguments {
                    argument.collect_scope_facts(facts);
                }
            }
            data!(Content::Apply { head, arguments }) => {
                head.collect_scope_facts(facts);
                for argument in arguments {
                    argument.collect_scope_facts(facts);
                }
            }
            data!(Content::Let(form)) => facts.record_let(form),
            data!(Content::Bind(form)) => facts.record_bind(form),
            data!(Content::LetRec(form)) => facts.record_let_rec(form),
            data!(Content::Bound(variable)) => {
                facts.record_use(variable, atom(TypeAtom::Content));
            }
        }
    }
}

#[contract_trait]
impl Category for Content {
    fn value_type(&self) -> TypeExpr {
        atom(TypeAtom::Content)
    }

    fn append_free_binders_to(&self, binders: &mut BinderSet) {
        match self.as_data() {
            data!(Content::Close(predicate)) => predicate.append_free_binders_to(binders),
            data!(Content::Not(inner)) => inner.append_free_binders_to(binders),
            data!(Content::Junction { operands, .. }) => {
                for operand in operands {
                    operand.append_free_binders_to(binders);
                }
            }
            data!(Content::Binary { left, right, .. }) => {
                left.append_free_binders_to(binders);
                right.append_free_binders_to(binders);
            }
            data!(Content::Quantified { lambda, .. }) => {
                binders.extend(lambda.free_binders().iter().cloned());
            }
            data!(Content::Presuppose { trigger, body }) => {
                trigger.append_free_binders_to(binders);
                body.append_free_binders_to(binders);
            }
            data!(Content::Supplement { body, side }) => {
                body.append_free_binders_to(binders);
                side.append_free_binders_to(binders);
            }
            data!(Content::Answer { query, selection }) => {
                query.append_free_binders_to(binders);
                selection.append_free_binders_to(binders);
            }
            data!(Content::Intrinsic { arguments, .. }) => {
                for argument in arguments {
                    argument.append_free_binders_to(binders);
                }
            }
            data!(Content::Apply { head, arguments }) => {
                head.append_free_binders_to(binders);
                for argument in arguments {
                    argument.append_free_binders_to(binders);
                }
            }
            data!(Content::Let(form)) => binders.extend(form.free_binders().iter().cloned()),
            data!(Content::Bind(form)) => binders.extend(form.free_binders().iter().cloned()),
            data!(Content::LetRec(form)) => binders.extend(form.free_binders().iter().cloned()),
            data!(Content::Bound(variable)) => {
                binders.insert(variable.clone());
            }
        }
    }
}

/// A polar or open question value.
#[invariant(::Polar(_) => true)]
#[invariant(::Open(_) => true)]
#[invariant(::Bound { parameters, .. } => !parameters.is_empty() || parameters.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Query {
    Polar(Content),
    Open(Lambda<Content>),
    Bound {
        variable: Variable,
        parameters: Vec<TypeExpr>,
    },
}

impl Query {
    /// Ask a graph truth-slot question.
    #[requires(true)]
    #[ensures(true)]
    pub fn polar(content: Content) -> Self {
        new!(Query::Polar(content))
    }

    /// Bind a nonempty answer domain.
    #[requires(true)]
    #[ensures(true)]
    pub fn open(lambda: Lambda<Content>) -> Self {
        new!(Query::Open(lambda))
    }

    /// Reference a bound query.
    #[requires(true)]
    #[ensures(true)]
    pub fn bound(variable: Variable, parameters: Vec<TypeExpr>) -> Self {
        new!(Query::Bound {
            variable,
            parameters,
        })
    }

    /// Return the ordered answer-tuple types this query binds.
    #[requires(true)]
    #[ensures(true)]
    pub fn parameters(&self) -> Vec<TypeExpr> {
        match self.as_data() {
            data!(Query::Polar(_)) => Vec::new(),
            data!(Query::Open(lambda)) => lambda
                .parameters()
                .iter()
                .map(|parameter| parameter.declared_type().clone())
                .collect(),
            data!(Query::Bound { parameters, .. }) => parameters.clone(),
        }
    }

    /// Record this value's binder introductions and uses.
    #[requires(true)]
    #[ensures(true)]
    pub(super) fn collect_scope_facts(&self, facts: &mut ScopeFacts) {
        match self.as_data() {
            data!(Query::Polar(content)) => content.collect_scope_facts(facts),
            data!(Query::Open(lambda)) => facts.record_lambda(lambda),
            data!(Query::Bound {
                variable,
                parameters,
            }) => facts.record_use(variable, TypeExpr::Query(parameters.clone())),
        }
    }
}

#[contract_trait]
impl Category for Query {
    fn value_type(&self) -> TypeExpr {
        TypeExpr::Query(self.parameters())
    }

    fn append_free_binders_to(&self, binders: &mut BinderSet) {
        match self.as_data() {
            data!(Query::Polar(content)) => content.append_free_binders_to(binders),
            data!(Query::Open(lambda)) => {
                binders.extend(lambda.free_binders().iter().cloned());
            }
            data!(Query::Bound { variable, .. }) => {
                binders.insert(variable.clone());
            }
        }
    }
}

/// The closed answer-selection forms of section 12.2.
#[invariant(::Polar(_) => true)]
#[invariant(::Tuple { values, .. } => !values.is_empty(), "an answer tuple is nonempty")]
#[invariant(::Contextual(_) => true)]
#[invariant(::Unresolved => true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnswerSelection {
    Polar(AnswerPolarity),
    Tuple {
        values: Vec<Value>,
        exhaustivity: Option<AnswerExhaustivity>,
    },
    /// The contextually true answer. Its omitted exhaustivity is the canonical
    /// spelling of genuinely undetermined exhaustivity, not a defaulted value.
    Contextual(Option<AnswerExhaustivity>),
    Unresolved,
}

impl AnswerSelection {
    /// State a polar answer.
    #[requires(true)]
    #[ensures(true)]
    pub fn polar(polarity: AnswerPolarity) -> Self {
        new!(AnswerSelection::Polar(polarity))
    }

    /// State a nonempty answer tuple.
    #[requires(true)]
    #[ensures(ret.is_ok() == !old(values.is_empty()))]
    pub fn tuple(
        values: Vec<Value>,
        exhaustivity: Option<AnswerExhaustivity>,
    ) -> Result<Self, KernelTypeError> {
        if values.is_empty() {
            return Err(KernelTypeError::new("an answer tuple is nonempty"));
        }
        Ok(new!(AnswerSelection::Tuple {
            values,
            exhaustivity,
        }))
    }

    /// State the contextually true answer.
    #[requires(true)]
    #[ensures(true)]
    pub fn contextual(exhaustivity: Option<AnswerExhaustivity>) -> Self {
        new!(AnswerSelection::Contextual(exhaustivity))
    }

    /// State that unresolved answerhood is itself the represented value.
    #[requires(true)]
    #[ensures(true)]
    pub fn unresolved() -> Self {
        new!(AnswerSelection::Unresolved)
    }

    /// Test whether this selection inhabits a query's answer tuple.
    ///
    /// `ContextualAnswer` and `UnresolvedAnswer` infer their tuple parameters
    /// from the query they are paired with, so they answer any query.
    #[requires(true)]
    #[ensures(true)]
    pub fn answers(&self, query: &Query) -> bool {
        let parameters = query.parameters();
        match self.as_data() {
            data!(AnswerSelection::Polar(_)) => parameters.is_empty(),
            data!(AnswerSelection::Tuple { values, .. }) => {
                values.len() == parameters.len()
                    && values.iter().zip(&parameters).all(|(value, declared)| {
                        super::intrinsic::kernel_accepts(&value.value_type(), declared)
                    })
            }
            data!(AnswerSelection::Contextual(_)) | data!(AnswerSelection::Unresolved) => true,
        }
    }

    /// Return the answer-selection type for a query's answer tuple.
    #[requires(true)]
    #[ensures(matches!(ret, TypeExpr::AnswerSelection(_)))]
    pub fn selection_type(&self, query: &Query) -> TypeExpr {
        TypeExpr::AnswerSelection(query.parameters())
    }

    /// Add every freely used binder name to `binders`.
    #[requires(true)]
    #[ensures(true)]
    pub(super) fn append_free_binders_to(&self, binders: &mut BinderSet) {
        if let data!(AnswerSelection::Tuple { values, .. }) = self.as_data() {
            for value in values {
                value.append_free_binders_to(binders);
            }
        }
    }

    /// Record this value's binder introductions and uses.
    #[requires(true)]
    #[ensures(true)]
    pub(super) fn collect_scope_facts(&self, facts: &mut ScopeFacts) {
        if let data!(AnswerSelection::Tuple { values, .. }) = self.as_data() {
            for value in values {
                value.collect_scope_facts(facts);
            }
        }
    }
}

/// Test whether every surviving place of a row can be closed.
///
/// Section 5.1 closes remaining ordinary referential places with a contextual
/// computation and an open event place with a local existential event, and
/// explicitly refuses to invent a default for a higher-order, function,
/// content, sign, or act place. An unknown numbered tail is not closeable
/// either, because its surviving places are not yet known.
#[requires(true)]
#[ensures(ret -> !row.has_open_numbered_tail())]
pub fn row_is_closeable(row: &Row) -> bool {
    !row.has_open_numbered_tail()
        && row
            .slots()
            .iter()
            .all(|slot| matches!(slot.accepted_type(), TypeExpr::Referents(_)))
}
