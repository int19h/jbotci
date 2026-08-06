//! Predicate terms and place filling.
//!
//! A predicate term is the central relational value of section 1.1: a relation
//! identity together with the row of places that are still open. Every
//! application node validates through [`super::apply::apply_predicate`], so a
//! wrong-arity fill, a duplicate fill, a deleted place, or an incompatible
//! value cannot be constructed at all.

#[allow(unused_imports)]
use bityzba::{contract_trait, data, ensures, invariant, new, requires};

use super::apply::{
    PredicateApplicationResult, PredicateArgument, PredicateSignature, apply_predicate,
};
use super::binder::{Bind, BinderSet, Category, Let, LetRec};
use super::document::ScopeFacts;
use super::error::KernelTypeError;
use super::types::{
    ComputedPlaceDomain, PlaceCandidates, PositiveInteger, RelationRef, Row, ScalarKind, TypeExpr,
    Variable,
};
use super::value::{Operand, Value};

/// One argument of a predicate application.
///
/// The four productions are kept distinct because their cursor behaviour is
/// distinct (section 4.2); conflating them is exactly the mistake the fill
/// cursor exists to prevent.
#[invariant(::Plain(_) => true)]
#[invariant(::Numbered { .. } => true)]
#[invariant(::Eventuality(_) => true)]
#[invariant(::Computed { place, .. } => matches!(place.value_type(), TypeExpr::PlaceOf { .. }),
    "a computed fill's place expression has PlaceOf type")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlaceFill {
    Plain(Operand),
    Numbered {
        place: PositiveInteger,
        value: Operand,
    },
    Eventuality(Operand),
    Computed {
        place: Value,
        candidates: ComputedPlaceDomain,
        value: Operand,
    },
}

impl PlaceFill {
    /// Fill the current numbered cursor place.
    #[requires(true)]
    #[ensures(true)]
    pub fn plain(value: Operand) -> Self {
        new!(PlaceFill::Plain(value))
    }

    /// Fill one literal numbered place.
    #[requires(true)]
    #[ensures(true)]
    pub fn numbered(place: PositiveInteger, value: Operand) -> Self {
        new!(PlaceFill::Numbered { place, value })
    }

    /// Fill the distinguished event place.
    #[requires(true)]
    #[ensures(true)]
    pub fn eventuality(value: Operand) -> Self {
        new!(PlaceFill::Eventuality(value))
    }

    /// Fill a genuinely computed place.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    pub fn computed(
        place: Value,
        candidates: ComputedPlaceDomain,
        value: Operand,
    ) -> Result<Self, KernelTypeError> {
        if !matches!(place.value_type(), TypeExpr::PlaceOf { .. }) {
            return Err(KernelTypeError::new(
                "an At place expression must have PlaceOf type",
            ));
        }
        Ok(new!(PlaceFill::Computed {
            place,
            candidates,
            value,
        }))
    }

    /// Project the typed argument the application kernel checks.
    #[requires(true)]
    #[ensures(true)]
    pub fn to_argument(&self) -> PredicateArgument {
        match self.as_data() {
            data!(PlaceFill::Plain(value)) => PredicateArgument::Plain {
                value_type: value.value_type(),
            },
            data!(PlaceFill::Numbered { place, value }) => PredicateArgument::Numbered {
                place: place.clone(),
                value_type: value.value_type(),
            },
            data!(PlaceFill::Eventuality(value)) => PredicateArgument::Eventuality {
                value_type: value.value_type(),
            },
            data!(PlaceFill::Computed {
                place,
                candidates,
                value,
            }) => PredicateArgument::Computed {
                place_type: place.value_type(),
                candidates: candidates.clone(),
                value_type: value.value_type(),
            },
        }
    }

    /// Borrow the filled value.
    #[requires(true)]
    #[ensures(true)]
    pub fn value(&self) -> &Operand {
        match self.as_data() {
            data!(PlaceFill::Plain(value))
            | data!(PlaceFill::Eventuality(value))
            | data!(PlaceFill::Numbered { value, .. })
            | data!(PlaceFill::Computed { value, .. }) => value,
        }
    }

    /// Add every freely used binder name to `binders`.
    #[requires(true)]
    #[ensures(true)]
    pub(super) fn append_free_binders_to(&self, binders: &mut BinderSet) {
        if let data!(PlaceFill::Computed { place, .. }) = self.as_data() {
            place.append_free_binders_to(binders);
        }
        self.value().append_free_binders_to(binders);
    }

    /// Record this fill's binder introductions and uses.
    #[requires(true)]
    #[ensures(true)]
    pub(super) fn collect_scope_facts(&self, facts: &mut ScopeFacts) {
        if let data!(PlaceFill::Computed { place, .. }) = self.as_data() {
            place.collect_scope_facts(facts);
        }
        self.value().collect_scope_facts(facts);
    }
}

/// Project the typed arguments of an ordered fill list.
#[requires(true)]
#[ensures(ret.len() == fills.len())]
pub(super) fn fill_arguments(fills: &[PlaceFill]) -> Vec<PredicateArgument> {
    fills.iter().map(PlaceFill::to_argument).collect()
}

/// A predicate term with its effective open row.
#[invariant(::Relation(_) => true)]
#[invariant(::Applied { head, fills, result } =>
    apply_predicate(&head.signature(), &fill_arguments(fills)).as_ref() == Ok(result),
    "an application's stored remainder is exactly what the fill cursor computes")]
#[invariant(::Bound { .. } => true)]
#[invariant(::Let(_) => true)]
#[invariant(::Bind(_) => true)]
#[invariant(::LetRec(_) => true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PredTerm {
    /// An unapplied relation term: a lexical root, a relation former over one,
    /// or a bound relation identity.
    Relation(PredicateSignature),
    Applied {
        head: Box<PredTerm>,
        fills: Vec<PlaceFill>,
        result: PredicateApplicationResult,
    },
    /// A bound predicate term. This covers both a shared assembled term named
    /// by `Let` (section 5.2) and the open-row relation question variable of
    /// the `ti mo zdani` family (sections 4.3 and 12.1), whose row records
    /// every statically required slot plus an unknown surviving tail.
    Bound {
        variable: Variable,
        row: Row,
    },
    Let(Let<PredTerm>),
    Bind(Bind<PredTerm>),
    LetRec(LetRec<PredTerm>),
}

impl PredTerm {
    /// Name one relation with its effective row.
    #[requires(true)]
    #[ensures(true)]
    pub fn relation(signature: PredicateSignature) -> Self {
        new!(PredTerm::Relation(signature))
    }

    /// Reference a bound predicate term or open-row relation variable.
    #[requires(true)]
    #[ensures(true)]
    pub fn bound(variable: Variable, row: Row) -> Self {
        new!(PredTerm::Bound { variable, row })
    }

    /// Fill places of a predicate term.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    pub fn applied(head: PredTerm, fills: Vec<PlaceFill>) -> Result<Self, KernelTypeError> {
        if fills.is_empty() {
            return Err(KernelTypeError::new(
                "an application fills at least one place",
            ));
        }
        let result = apply_predicate(&head.signature(), &fill_arguments(&fills))?;
        Ok(new!(PredTerm::Applied {
            head: Box::new(head),
            fills,
            result,
        }))
    }

    /// Modify a head relation with a vague tanru modifier.
    ///
    /// Both operands must be unapplied relation terms: `Tanru` composes
    /// relation identities and preserves the head's row (section 14.1).
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    pub fn tanru(modifier: &PredTerm, head: &PredTerm) -> Result<Self, KernelTypeError> {
        let modifier = modifier.unapplied_relation()?;
        let head = head.unapplied_relation()?;
        Ok(Self::relation(PredicateSignature::new(
            RelationRef::Tanru {
                modifier: Box::new(modifier.relation().clone()),
                head: Box::new(head.relation().clone()),
            },
            head.row().clone(),
        )))
    }

    /// Form a scalar relation, preserving the row.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    pub fn scalar(kind: ScalarKind, inner: &PredTerm) -> Result<Self, KernelTypeError> {
        let inner = inner.unapplied_relation()?;
        Ok(Self::relation(PredicateSignature::new(
            RelationRef::Scalar {
                kind,
                relation: Box::new(inner.relation().clone()),
            },
            inner.row().clone(),
        )))
    }

    /// Delete one numbered place, preserving every surviving label.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    pub fn drop_place(inner: &PredTerm, place: PositiveInteger) -> Result<Self, KernelTypeError> {
        let inner = inner.unapplied_relation()?;
        Ok(Self::relation(inner.drop_place(place)?))
    }

    /// Bind a block of inert declarations over a predicate-term body.
    #[requires(true)]
    #[ensures(true)]
    pub fn let_form(form: Let<PredTerm>) -> Self {
        new!(PredTerm::Let(form))
    }

    /// Run a reference computation over a predicate-term body.
    #[requires(true)]
    #[ensures(true)]
    pub fn bind_form(form: Bind<PredTerm>) -> Self {
        new!(PredTerm::Bind(form))
    }

    /// Tie a recursive function group over a predicate-term body.
    #[requires(true)]
    #[ensures(true)]
    pub fn let_rec_form(form: LetRec<PredTerm>) -> Self {
        new!(PredTerm::LetRec(form))
    }

    /// Return the still-open effective row.
    #[requires(true)]
    #[ensures(true)]
    pub fn row(&self) -> Row {
        match self.as_data() {
            data!(PredTerm::Relation(signature)) => signature.row().clone(),
            data!(PredTerm::Applied { result, .. }) => result.remaining_row(),
            data!(PredTerm::Bound { row, .. }) => row.clone(),
            data!(PredTerm::Let(form)) => form.body().row(),
            data!(PredTerm::Bind(form)) => form.body().row(),
            data!(PredTerm::LetRec(form)) => form.body().row(),
        }
    }

    /// Return the relation identity and current row of this term.
    ///
    /// A bound term uses its own binder name as its relation identity, which is
    /// exactly the "bound relation identity" case section 4.2 permits in
    /// `PlaceOf<R,T>`.
    #[requires(true)]
    #[ensures(true)]
    pub fn signature(&self) -> PredicateSignature {
        match self.as_data() {
            data!(PredTerm::Relation(signature)) => signature.clone(),
            data!(PredTerm::Applied { head, result, .. }) => {
                PredicateSignature::new(head.signature().relation().clone(), result.remaining_row())
            }
            data!(PredTerm::Bound { variable, row }) => {
                PredicateSignature::new(RelationRef::Variable(variable.clone()), row.clone())
            }
            data!(PredTerm::Let(form)) => form.body().signature(),
            data!(PredTerm::Bind(form)) => form.body().signature(),
            data!(PredTerm::LetRec(form)) => form.body().signature(),
        }
    }

    /// Borrow this term's signature when it has no fills yet.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn unapplied_relation(&self) -> Result<PredicateSignature, KernelTypeError> {
        match self.as_data() {
            data!(PredTerm::Relation(signature)) => Ok(signature.clone()),
            data!(PredTerm::Bound { .. }) => Ok(self.signature()),
            _ => Err(KernelTypeError::new(
                "a relation former composes relation identities, not already filled terms",
            )),
        }
    }

    /// Build a `PlaceOf` type over this term's relation identity.
    #[requires(true)]
    #[ensures(matches!(ret, TypeExpr::PlaceOf { .. }))]
    pub fn place_of(
        &self,
        accepted_type: TypeExpr,
        candidates: Option<PlaceCandidates>,
    ) -> TypeExpr {
        TypeExpr::PlaceOf {
            relation: self.signature().relation().clone(),
            accepted_type: Box::new(accepted_type),
            candidates,
        }
    }

    /// Record this term's binder introductions and uses.
    #[requires(true)]
    #[ensures(true)]
    pub(super) fn collect_scope_facts(&self, facts: &mut ScopeFacts) {
        match self.as_data() {
            data!(PredTerm::Relation(_)) => {}
            data!(PredTerm::Applied { head, fills, .. }) => {
                head.collect_scope_facts(facts);
                for fill in fills {
                    fill.collect_scope_facts(facts);
                }
            }
            data!(PredTerm::Bound { variable, row }) => {
                facts.record_use(variable, TypeExpr::Predicate(row.clone()));
            }
            data!(PredTerm::Let(form)) => facts.record_let(form),
            data!(PredTerm::Bind(form)) => facts.record_bind(form),
            data!(PredTerm::LetRec(form)) => facts.record_let_rec(form),
        }
    }
}

#[contract_trait]
impl Category for PredTerm {
    fn value_type(&self) -> TypeExpr {
        TypeExpr::Predicate(self.row())
    }

    fn append_free_binders_to(&self, binders: &mut BinderSet) {
        match self.as_data() {
            data!(PredTerm::Relation(signature)) => {
                append_relation_binders_to(signature.relation(), binders);
            }
            data!(PredTerm::Applied { head, fills, .. }) => {
                head.append_free_binders_to(binders);
                for fill in fills {
                    fill.append_free_binders_to(binders);
                }
            }
            data!(PredTerm::Bound { variable, .. }) => {
                binders.insert(variable.clone());
            }
            data!(PredTerm::Let(form)) => binders.extend(form.free_binders().iter().cloned()),
            data!(PredTerm::Bind(form)) => binders.extend(form.free_binders().iter().cloned()),
            data!(PredTerm::LetRec(form)) => binders.extend(form.free_binders().iter().cloned()),
        }
    }
}

/// Collect the bound relation identities inside a relation expression.
#[requires(true)]
#[ensures(true)]
pub(super) fn append_relation_binders_to(relation: &RelationRef, binders: &mut BinderSet) {
    match relation {
        RelationRef::Lexical(_) => {}
        RelationRef::Variable(variable) => {
            binders.insert(variable.clone());
        }
        RelationRef::DropPlace { relation, .. } | RelationRef::Scalar { relation, .. } => {
            append_relation_binders_to(relation, binders);
        }
        RelationRef::Tanru { modifier, head } => {
            append_relation_binders_to(modifier, binders);
            append_relation_binders_to(head, binders);
        }
    }
}
