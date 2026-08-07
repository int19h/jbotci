//! Version-0 predicate-place and function application.
//!
//! These are the two deliberately distinct application operations of the
//! kernel: labelled predicate-place filling with its exact fill cursor, and
//! ordered positional function application. Kernel value constructors validate
//! through them, so a well-typed application is the only way to build an
//! application node.

use std::collections::BTreeSet;
use std::fmt;

#[allow(unused_imports)]
use bityzba::{ensures, invariant, new, requires};

use super::types::{
    ComputedPlaceDomain, PlaceLabel, PositiveInteger, RelationRef, Row, RowSlot, TypeExpr,
    labels_are_unique, row_slots_are_canonical,
};

/// One typed argument to predicate-place application.
#[invariant(::Plain { .. } => true)]
#[invariant(::Numbered { .. } => true)]
#[invariant(::Eventuality { .. } => true)]
#[invariant(::Computed { .. } => true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PredicateArgument {
    Plain {
        value_type: TypeExpr,
    },
    Numbered {
        place: PositiveInteger,
        value_type: TypeExpr,
    },
    Eventuality {
        value_type: TypeExpr,
    },
    Computed {
        place_type: TypeExpr,
        candidates: ComputedPlaceDomain,
        value_type: TypeExpr,
    },
}

impl PredicateArgument {
    /// Construct a computed `At` argument with a nonempty injective domain.
    #[requires(!candidates.is_empty() && labels_are_unique(&candidates))]
    #[ensures(matches!(ret, Self::Computed { .. }))]
    pub fn computed(
        place_type: TypeExpr,
        candidates: Vec<PlaceLabel>,
        value_type: TypeExpr,
    ) -> Self {
        Self::Computed {
            place_type,
            candidates: ComputedPlaceDomain::new(candidates),
            value_type,
        }
    }
}
/// A typed predicate term at one application site.
#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PredicateSignature {
    relation: RelationRef,
    row: Row,
}

impl PredicateSignature {
    /// Construct a typed predicate signature.
    #[requires(true)]
    #[ensures(ret.relation == old(relation.clone()) && ret.row == old(row.clone()))]
    pub fn new(relation: RelationRef, row: Row) -> Self {
        Self { relation, row }
    }

    /// Borrow the relation identity.
    #[requires(true)]
    #[ensures(true)]
    pub fn relation(&self) -> &RelationRef {
        &self.relation
    }

    /// Borrow the effective row.
    #[requires(true)]
    #[ensures(true)]
    pub fn row(&self) -> &Row {
        &self.row
    }

    /// Apply ordinary, labelled, event, and computed fills without conflating
    /// any of their cursor behavior.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    pub fn apply(
        &self,
        arguments: &[PredicateArgument],
    ) -> Result<PredicateApplicationResult, ApplicationTypeError> {
        apply_predicate(self, arguments)
    }

    /// Delete one numbered place while preserving every surviving label.
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|signature| signature.row.slots().len() + 1 == self.row.slots().len()) || ret.is_err())]
    pub fn drop_place(&self, place: PositiveInteger) -> Result<Self, ApplicationTypeError> {
        let mut slots = self.row.slots().to_vec();
        let Some(index) = slots.iter().position(
            |slot| matches!(slot.label_ref(), PlaceLabel::Numbered(label) if label == &place),
        ) else {
            return Err(ApplicationTypeError::new(
                "DropPlace target is not in the current row",
            ));
        };
        slots.remove(index);
        Ok(Self::new(
            RelationRef::DropPlace {
                relation: Box::new(self.relation.clone()),
                place,
            },
            Row::new(slots, self.row.has_open_numbered_tail()),
        ))
    }
}
/// The statically known remainder after predicate application.
#[invariant(row_slots_are_canonical(&remaining_slots))]
#[invariant(computed_domains.iter().all(|domain| !domain.is_empty() && labels_are_unique(domain)))]
#[invariant(domains_are_pairwise_disjoint(&computed_domains))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PredicateApplicationResult {
    remaining_slots: Vec<RowSlot>,
    open_numbered_tail: bool,
    computed_domains: Vec<Vec<PlaceLabel>>,
    filled_types: Vec<Option<TypeExpr>>,
}

impl PredicateApplicationResult {
    /// Borrow statically unfilled slots. Computed domains remain separately
    /// reserved because answer substitution chooses which one is consumed.
    #[requires(true)]
    #[ensures(row_slots_are_canonical(ret))]
    pub fn remaining_slots(&self) -> &[RowSlot] {
        &self.remaining_slots
    }

    /// Borrow the type each ordered argument's slot declared.
    ///
    /// A fill's slot is not recoverable from the surface: only a labelled or
    /// event fill names its place, and a plain one is resolved by the cursor.
    /// The cursor is exact rather than a guess, though, so recording what it
    /// selected is what lets a printer apply section 3.3's and section 5.2's
    /// expected-type elisions at every fill instead of only at the named ones.
    /// A computed fill reserves a domain rather than consuming one statically
    /// known slot, so its entry is `None`.
    #[requires(true)]
    #[ensures(true)]
    pub fn filled_types(&self) -> &[Option<TypeExpr>] {
        &self.filled_types
    }

    /// Borrow the ordered computed-place domains.
    #[requires(true)]
    #[ensures(domains_are_pairwise_disjoint(ret))]
    pub fn computed_domains(&self) -> &[Vec<PlaceLabel>] {
        &self.computed_domains
    }

    /// Report whether an unknown numbered tail survives the application.
    #[requires(true)]
    #[ensures(ret == self.open_numbered_tail)]
    pub fn has_open_numbered_tail(&self) -> bool {
        self.open_numbered_tail
    }

    /// Return the statically surviving row of the applied term.
    ///
    /// Computed domains stay reserved rather than consumed, so their candidate
    /// slots remain in this row: answer substitution is what decides which one
    /// a computed fill took.
    #[requires(true)]
    #[ensures(ret.slots().len() == self.remaining_slots.len())]
    pub fn remaining_row(&self) -> Row {
        Row::new(self.remaining_slots.clone(), self.open_numbered_tail)
    }
}
/// An ordered function signature.
#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionSignature {
    parameters: Vec<TypeExpr>,
    result: TypeExpr,
}

impl FunctionSignature {
    /// Construct an ordered function signature, including nullary functions.
    #[requires(true)]
    #[ensures(ret.parameters.len() == old(parameters.len()) && ret.result == old(result.clone()))]
    pub fn new(parameters: Vec<TypeExpr>, result: TypeExpr) -> Self {
        Self { parameters, result }
    }

    /// Borrow the ordered parameter types.
    #[requires(true)]
    #[ensures(ret.len() == self.parameters.len())]
    pub fn parameters(&self) -> &[TypeExpr] {
        &self.parameters
    }

    /// Borrow the declared result type.
    #[requires(true)]
    #[ensures(ret == &self.result)]
    pub fn result(&self) -> &TypeExpr {
        &self.result
    }

    /// Return the `Fn` type this signature denotes.
    #[requires(true)]
    #[ensures(matches!(ret, TypeExpr::Function { .. }))]
    pub fn function_type(&self) -> TypeExpr {
        TypeExpr::Function {
            parameters: self.parameters.clone(),
            result: Box::new(self.result.clone()),
        }
    }

    /// Unify every argument against the complete signature.
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|result| *result == &self.result) || ret.is_err())]
    pub fn apply(&self, arguments: &[TypeExpr]) -> Result<&TypeExpr, ApplicationTypeError> {
        if arguments.len() != self.parameters.len() {
            return Err(ApplicationTypeError::new(
                "function application arity mismatch",
            ));
        }
        if arguments
            .iter()
            .zip(&self.parameters)
            .any(|(actual, expected)| actual.implicit_conversion_to(expected).is_none())
        {
            return Err(ApplicationTypeError::new("function argument type mismatch"));
        }
        Ok(&self.result)
    }
}
/// Predicate/function application failure.
#[invariant(!message.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicationTypeError {
    message: String,
}

impl ApplicationTypeError {
    /// Construct a nonempty application failure.
    #[requires(true)]
    #[ensures(!ret.message.is_empty())]
    pub(super) fn new(message: impl Into<String>) -> Self {
        let message = message.into();
        assert!(!message.is_empty(), "application errors require a message");
        new!(ApplicationTypeError { message })
    }
}

impl fmt::Display for ApplicationTypeError {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for ApplicationTypeError {}
/// Apply one predicate signature with the exact fill-cursor rules.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
pub fn apply_predicate(
    signature: &PredicateSignature,
    arguments: &[PredicateArgument],
) -> Result<PredicateApplicationResult, ApplicationTypeError> {
    let mut remaining = signature.row.slots().to_vec();
    let mut cursor_after = None::<PositiveInteger>;
    let mut computed_seen = false;
    let mut computed_domains = Vec::<Vec<PlaceLabel>>::new();
    let mut filled_types = Vec::<Option<TypeExpr>>::with_capacity(arguments.len());
    for argument in arguments {
        match argument {
            PredicateArgument::Plain { value_type } => {
                if computed_seen {
                    return Err(ApplicationTypeError::new(
                        "plain fills cannot follow a computed At fill",
                    ));
                }
                let Some(index) = remaining.iter().position(|slot| {
                    matches!(slot.label_ref(), PlaceLabel::Numbered(place) if cursor_after.as_ref().is_none_or(|cursor| place > cursor))
                }) else {
                    return Err(ApplicationTypeError::new("no current numbered cursor place"));
                };
                ensure_accepted(value_type, remaining[index].accepted_type())?;
                let label = remaining[index].label();
                filled_types.push(Some(remaining[index].accepted_type().clone()));
                remaining.remove(index);
                let PlaceLabel::Numbered(place) = label else {
                    unreachable!("cursor selects only numbered places")
                };
                cursor_after = Some(place);
            }
            PredicateArgument::Numbered { place, value_type } => {
                if computed_seen {
                    return Err(ApplicationTypeError::new(
                        "literal labelled fills cannot follow computed At",
                    ));
                }
                let Some(index) = remaining
                    .iter()
                    .position(|slot| slot.label_ref() == &PlaceLabel::Numbered(place.clone()))
                else {
                    return Err(ApplicationTypeError::new(
                        "literal place is absent, deleted, or already filled",
                    ));
                };
                ensure_accepted(value_type, remaining[index].accepted_type())?;
                filled_types.push(Some(remaining[index].accepted_type().clone()));
                remaining.remove(index);
                cursor_after = Some(place.clone());
            }
            PredicateArgument::Eventuality { value_type } => {
                if computed_seen {
                    return Err(ApplicationTypeError::new(
                        "event fills cannot follow a computed At fill",
                    ));
                }
                let Some(index) = remaining
                    .iter()
                    .position(|slot| slot.label_ref() == &PlaceLabel::Eventuality)
                else {
                    return Err(ApplicationTypeError::new(
                        "event place is absent or already filled",
                    ));
                };
                ensure_accepted(value_type, remaining[index].accepted_type())?;
                filled_types.push(Some(remaining[index].accepted_type().clone()));
                remaining.remove(index);
            }
            PredicateArgument::Computed {
                place_type,
                candidates,
                value_type,
            } => {
                computed_seen = true;
                let TypeExpr::PlaceOf {
                    relation,
                    accepted_type,
                    candidates: declared,
                } = place_type
                else {
                    return Err(ApplicationTypeError::new(
                        "At place expression must have PlaceOf type",
                    ));
                };
                if relation != &signature.relation {
                    return Err(ApplicationTypeError::new(
                        "At relation identity does not match its predicate host",
                    ));
                }
                ensure_accepted(value_type, accepted_type)?;
                let candidates = candidates.as_slice();
                if declared
                    .as_ref()
                    .is_some_and(|labels| labels.as_slice() != candidates)
                {
                    return Err(ApplicationTypeError::new(
                        "At candidates disagree with the explicit PlaceOf domain",
                    ));
                }
                if declared.is_none()
                    && candidates
                        .iter()
                        .any(|label| matches!(label, PlaceLabel::Eventuality))
                {
                    return Err(ApplicationTypeError::new(
                        "an omitted PlaceOf candidate set derives numbered places only",
                    ));
                }
                if declared.is_none() {
                    let derived = remaining
                        .iter()
                        .filter(|slot| matches!(slot.label_ref(), PlaceLabel::Numbered(_)))
                        .filter(|slot| {
                            accepted_type
                                .implicit_conversion_to(slot.accepted_type())
                                .is_some()
                        })
                        .map(RowSlot::label)
                        .collect::<Vec<_>>();
                    if candidates != derived {
                        return Err(ApplicationTypeError::new(
                            "omitted PlaceOf candidates are not the exact derivable domain",
                        ));
                    }
                }
                for label in candidates {
                    let Some(slot) = remaining.iter().find(|slot| slot.label_ref() == label) else {
                        return Err(ApplicationTypeError::new(
                            "At candidate is absent, reserved, or deleted",
                        ));
                    };
                    if accepted_type
                        .implicit_conversion_to(slot.accepted_type())
                        .is_none()
                    {
                        return Err(ApplicationTypeError::new(
                            "At candidate does not accept its declared value type",
                        ));
                    }
                }
                if computed_domains
                    .iter()
                    .any(|prior| prior.iter().any(|label| candidates.contains(label)))
                {
                    return Err(ApplicationTypeError::new(
                        "computed At candidate domains overlap",
                    ));
                }
                computed_domains.push(candidates.to_vec());
                filled_types.push(None);
            }
        }
    }
    Ok(new!(PredicateApplicationResult {
        remaining_slots: remaining,
        open_numbered_tail: signature.row.has_open_numbered_tail(),
        computed_domains,
        filled_types,
    }))
}

/// Require an implicit conversion into one operand type.
#[requires(true)]
#[ensures(ret.is_ok() == actual.implicit_conversion_to(expected).is_some())]
fn ensure_accepted(actual: &TypeExpr, expected: &TypeExpr) -> Result<(), ApplicationTypeError> {
    if actual.implicit_conversion_to(expected).is_none() {
        return Err(ApplicationTypeError::new("predicate fill type mismatch"));
    }
    Ok(())
}
/// Test pairwise disjoint computed domains.
#[requires(true)]
#[ensures(true)]
fn domains_are_pairwise_disjoint(domains: &[Vec<PlaceLabel>]) -> bool {
    let mut seen = BTreeSet::new();
    domains
        .iter()
        .flat_map(|domain| domain.iter())
        .all(|label| seen.insert(label.clone()))
}

#[cfg(test)]
mod tests {
    use bityzba::requires;

    use super::super::types::{LexicalRoot, PlaceCandidates, TypeAtom};
    use super::*;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn predicate_fill_cursor_and_drop_place_preserve_labels() {
        let referents = TypeExpr::Referents(Box::new(TypeExpr::Atom(TypeAtom::Entity)));
        let relation = RelationRef::Lexical(LexicalRoot::try_new("klama").unwrap());
        let signature = PredicateSignature::new(
            relation,
            Row::new(
                vec![
                    RowSlot::new(PlaceLabel::numbered(1), referents.clone()),
                    RowSlot::new(PlaceLabel::numbered(2), referents.clone()),
                    RowSlot::new(PlaceLabel::numbered(3), referents.clone()),
                ],
                false,
            ),
        );
        let result = signature
            .apply(&[
                PredicateArgument::Numbered {
                    place: PositiveInteger::from_u32(2),
                    value_type: referents.clone(),
                },
                PredicateArgument::Plain {
                    value_type: referents.clone(),
                },
            ])
            .unwrap();
        assert_eq!(
            result
                .remaining_slots()
                .iter()
                .map(RowSlot::label)
                .collect::<Vec<_>>(),
            vec![PlaceLabel::numbered(1)]
        );
        let dropped = signature.drop_place(PositiveInteger::from_u32(2)).unwrap();
        assert_eq!(
            dropped
                .row()
                .slots()
                .iter()
                .map(RowSlot::label)
                .collect::<Vec<_>>(),
            vec![PlaceLabel::numbered(1), PlaceLabel::numbered(3)]
        );

        let huge = PositiveInteger::try_new("429496729600000000000000000000").unwrap();
        let huge_signature = PredicateSignature::new(
            signature.relation().clone(),
            Row::new(
                vec![RowSlot::new(
                    PlaceLabel::Numbered(huge.clone()),
                    referents.clone(),
                )],
                false,
            ),
        );
        assert!(huge_signature.drop_place(huge).is_ok());

        let event_signature = PredicateSignature::new(
            signature.relation().clone(),
            Row::new(
                vec![
                    RowSlot::new(PlaceLabel::numbered(1), referents.clone()),
                    RowSlot::new(PlaceLabel::numbered(2), referents.clone()),
                    RowSlot::new(PlaceLabel::numbered(3), referents.clone()),
                    RowSlot::new(
                        PlaceLabel::Eventuality,
                        TypeExpr::Referents(Box::new(TypeExpr::Atom(TypeAtom::Eventuality))),
                    ),
                ],
                false,
            ),
        );
        let after_event = event_signature
            .apply(&[
                PredicateArgument::Numbered {
                    place: PositiveInteger::from_u32(2),
                    value_type: referents.clone(),
                },
                PredicateArgument::Eventuality {
                    value_type: TypeExpr::Referents(Box::new(TypeExpr::Atom(
                        TypeAtom::Eventuality,
                    ))),
                },
                PredicateArgument::Plain {
                    value_type: referents,
                },
            ])
            .unwrap();
        assert_eq!(
            after_event
                .remaining_slots()
                .iter()
                .map(RowSlot::label)
                .collect::<Vec<_>>(),
            vec![PlaceLabel::numbered(1)]
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn computed_at_domains_must_be_disjoint_and_relation_exact() {
        let referents = TypeExpr::Referents(Box::new(TypeExpr::Atom(TypeAtom::Entity)));
        let relation = RelationRef::Lexical(LexicalRoot::try_new("klama").unwrap());
        let signature = PredicateSignature::new(
            relation.clone(),
            Row::new(
                vec![
                    RowSlot::new(PlaceLabel::numbered(1), referents.clone()),
                    RowSlot::new(PlaceLabel::numbered(2), referents.clone()),
                    RowSlot::new(PlaceLabel::numbered(3), referents.clone()),
                ],
                false,
            ),
        );
        let place_type = TypeExpr::PlaceOf {
            relation,
            accepted_type: Box::new(referents.clone()),
            candidates: None,
        };
        let first = PredicateArgument::computed(
            place_type.clone(),
            vec![
                PlaceLabel::numbered(1),
                PlaceLabel::numbered(2),
                PlaceLabel::numbered(3),
            ],
            referents.clone(),
        );
        let second = PredicateArgument::computed(
            TypeExpr::PlaceOf {
                relation: signature.relation().clone(),
                accepted_type: Box::new(referents.clone()),
                candidates: Some(PlaceCandidates::new(vec![PlaceLabel::numbered(2)])),
            },
            vec![PlaceLabel::numbered(2)],
            referents,
        );
        assert!(signature.apply(&[first, second]).is_err());

        let omitted_subset = PredicateArgument::computed(
            TypeExpr::PlaceOf {
                relation: signature.relation().clone(),
                accepted_type: Box::new(TypeExpr::Referents(Box::new(TypeExpr::Atom(
                    TypeAtom::Entity,
                )))),
                candidates: None,
            },
            vec![PlaceLabel::numbered(1), PlaceLabel::numbered(2)],
            TypeExpr::Referents(Box::new(TypeExpr::Atom(TypeAtom::Entity))),
        );
        assert!(signature.apply(&[omitted_subset]).is_err());

        let explicit_event_type = TypeExpr::PlaceOf {
            relation: signature.relation().clone(),
            accepted_type: Box::new(TypeExpr::Referents(Box::new(TypeExpr::Atom(
                TypeAtom::Eventuality,
            )))),
            candidates: Some(PlaceCandidates::new(vec![PlaceLabel::Eventuality])),
        };
        let event_signature = PredicateSignature::new(
            signature.relation().clone(),
            Row::new(
                vec![RowSlot::new(
                    PlaceLabel::Eventuality,
                    TypeExpr::Referents(Box::new(TypeExpr::Atom(TypeAtom::Eventuality))),
                )],
                false,
            ),
        );
        assert!(
            event_signature
                .apply(&[PredicateArgument::computed(
                    explicit_event_type,
                    vec![PlaceLabel::Eventuality],
                    TypeExpr::Referents(Box::new(TypeExpr::Atom(TypeAtom::Eventuality))),
                )])
                .is_ok()
        );
        let derived_event_type = TypeExpr::PlaceOf {
            relation: event_signature.relation().clone(),
            accepted_type: Box::new(TypeExpr::Referents(Box::new(TypeExpr::Atom(
                TypeAtom::Eventuality,
            )))),
            candidates: None,
        };
        assert!(
            event_signature
                .apply(&[PredicateArgument::computed(
                    derived_event_type,
                    vec![PlaceLabel::Eventuality],
                    TypeExpr::Referents(Box::new(TypeExpr::Atom(TypeAtom::Eventuality))),
                )])
                .is_err()
        );
    }
}
