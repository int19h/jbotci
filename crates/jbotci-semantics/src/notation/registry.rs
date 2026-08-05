//! Read-only runtime view of the current smusni-v0 candidate registry.
//!
//! The checked-in generated source contains data only. This module supplies
//! the closed vocabulary, parses every expected type through the v0 kernel,
//! and constructs fallback boundaries only by joining the exact disposition
//! owner to its one registered reason row.

#![cfg_attr(not(test), allow(dead_code))]

use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

#[allow(unused_imports)]
use bityzba::{data, ensures, invariant, new, requires};

use super::sexpr::datum::parse_document;
use super::sexpr::type_system::TypeExpr;
use super::typed_ir::{DynamicValueFamily, ScopePolicy};

/// Closed semantic-surface owner category.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum CoordinateCategory {
    Object,
    ValueStruct,
    Enum,
    Document,
}

/// Closed semantic-coordinate role.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum CoordinateKind {
    Constructor,
    Discriminator,
    Field,
    EnumVariant,
    VariantField,
    DerivedFact,
}

/// One coordinate which is known to occur in the generated registry.
#[invariant(!surface.is_empty() && !member.is_empty())]
#[invariant(qualifier.is_none_or(|value| !value.is_empty()))]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct DispositionCoordinate {
    category: CoordinateCategory,
    surface: &'static str,
    kind: CoordinateKind,
    member: &'static str,
    qualifier: Option<&'static str>,
}

impl DispositionCoordinate {
    #[requires(!surface.is_empty() && !member.is_empty())]
    #[requires(qualifier.is_none_or(|value| !value.is_empty()))]
    #[ensures(ret.surface == surface && ret.member == member)]
    fn from_generated(
        category: CoordinateCategory,
        surface: &'static str,
        kind: CoordinateKind,
        member: &'static str,
        qualifier: Option<&'static str>,
    ) -> Self {
        new!(DispositionCoordinate {
            category,
            surface,
            kind,
            member,
            qualifier,
        })
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub(crate) fn owner(&self) -> String {
        let member = self.qualifier.map_or_else(
            || self.member.to_owned(),
            |qualifier| format!("{}@{qualifier}", self.member),
        );
        format!(
            "{}:{}:{}:{member}",
            category_name(self.category),
            self.surface,
            kind_name(self.kind)
        )
    }
}

/// Closed disposition taxonomy from specification section 14.4.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DispositionKind {
    DirectLowering,
    ProvenDesugaring,
    NotationDefault,
    ProvenanceSuppression,
    DiagnosticCollection,
    TypedFallback,
}

/// A generated nonfallback target contract. There is deliberately no public
/// free-string constructor.
#[invariant(!text.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TargetContract {
    text: &'static str,
}

impl TargetContract {
    #[requires(!text.is_empty())]
    #[ensures(ret.as_str() == text)]
    fn from_generated(text: &'static str) -> Self {
        new!(TargetContract { text })
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub(crate) fn as_str(&self) -> &'static str {
        self.text
    }
}

/// A bundled fallback reason identity. Values can only come from the generated
/// reason table.
#[invariant(!text.is_empty() && text.starts_with("smusni.fallback."))]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct FallbackReasonId {
    text: &'static str,
}

impl FallbackReasonId {
    #[requires(!text.is_empty() && text.starts_with("smusni.fallback."))]
    #[ensures(ret.as_str() == text)]
    fn from_generated(text: &'static str) -> Self {
        new!(FallbackReasonId { text })
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub(crate) fn as_str(&self) -> &'static str {
        self.text
    }
}

/// Closed raw model owners allowed at a fallback boundary.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MinimumRawOwner {
    SemanticGraph,
    Referent,
    Eventuality,
    Quantity,
    MathExpression,
}

impl MinimumRawOwner {
    #[requires(true)]
    #[ensures(ret.is_some() == matches!(text, "SemanticGraph" | "Referent" | "Eventuality" | "Quantity" | "MathExpression"))]
    fn parse_generated(text: &str) -> Option<Self> {
        match text {
            "SemanticGraph" => Some(Self::SemanticGraph),
            "Referent" => Some(Self::Referent),
            "Eventuality" => Some(Self::Eventuality),
            "Quantity" => Some(Self::Quantity),
            "MathExpression" => Some(Self::MathExpression),
            _ => None,
        }
    }
}

/// The exact validated disposition/reason join for one fallback.
#[invariant(disposition_owner.owner().starts_with("Object:") || disposition_owner.owner().starts_with("ValueStruct:") || disposition_owner.owner().starts_with("Enum:") || disposition_owner.owner().starts_with("Document:"))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FallbackBoundary {
    disposition_owner: DispositionCoordinate,
    reason_id: FallbackReasonId,
    expected_type: TypeExpr,
    minimum_raw_owner: MinimumRawOwner,
}

impl FallbackBoundary {
    #[requires(true)]
    #[ensures(ret == &self.disposition_owner)]
    pub(crate) fn disposition_owner(&self) -> &DispositionCoordinate {
        &self.disposition_owner
    }

    #[requires(true)]
    #[ensures(ret == &self.reason_id)]
    pub(crate) fn reason_id(&self) -> &FallbackReasonId {
        &self.reason_id
    }

    #[requires(true)]
    #[ensures(ret == &self.expected_type)]
    pub(crate) fn expected_type(&self) -> &TypeExpr {
        &self.expected_type
    }

    #[requires(true)]
    #[ensures(ret == self.minimum_raw_owner)]
    pub(crate) fn minimum_raw_owner(&self) -> MinimumRawOwner {
        self.minimum_raw_owner
    }
}

/// One fully joined runtime registry row.
#[invariant(::NonFallback { kind, .. } => *kind != DispositionKind::TypedFallback)]
#[invariant(::TypedFallback { .. } => true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RegisteredDisposition {
    NonFallback {
        coordinate: DispositionCoordinate,
        kind: DispositionKind,
        target_contract: TargetContract,
    },
    TypedFallback {
        boundary: FallbackBoundary,
    },
}

impl RegisteredDisposition {
    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn coordinate(&self) -> &DispositionCoordinate {
        match self.as_data() {
            data!(RegisteredDisposition::NonFallback { coordinate, .. }) => coordinate,
            data!(RegisteredDisposition::TypedFallback { boundary }) => {
                boundary.disposition_owner()
            }
        }
    }

    #[requires(true)]
    #[ensures(ret.is_some() == matches!(self.as_data(), data!(RegisteredDisposition::NonFallback { .. })))]
    pub(crate) fn target_contract(&self) -> Option<&TargetContract> {
        match self.as_data() {
            data!(RegisteredDisposition::NonFallback {
                target_contract,
                ..
            }) => Some(target_contract),
            data!(RegisteredDisposition::TypedFallback { .. }) => None,
        }
    }

    #[requires(true)]
    #[ensures(ret.is_some() == matches!(self.as_data(), data!(RegisteredDisposition::TypedFallback { .. })))]
    pub(crate) fn fallback_boundary(&self) -> Option<&FallbackBoundary> {
        match self.as_data() {
            data!(RegisteredDisposition::NonFallback { .. }) => None,
            data!(RegisteredDisposition::TypedFallback { boundary }) => Some(boundary),
        }
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RegistryBuildError {
    DuplicateCoordinate,
    DuplicateReason,
    MissingReason,
    OrphanReason,
    WrongReasonOwner,
    InvalidExpectedType,
    UnknownMinimumRawOwner,
    InvalidDispositionShape,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RegistryLookupError {
    UnknownCoordinate,
}

/// Complete deterministic runtime table.
#[invariant(rows_have_unique_coordinates(rows))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DispositionRegistry {
    rows: Vec<RegisteredDisposition>,
}

impl DispositionRegistry {
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|registry| registry.rows.len() == dispositions.len()) || ret.is_err())]
    fn try_from_generated(
        dispositions: &[GeneratedDispositionRow],
        reasons: &[GeneratedFallbackReasonRow],
    ) -> Result<Self, RegistryBuildError> {
        let mut reason_by_id = BTreeMap::new();
        for reason in reasons {
            if reason_by_id.insert(reason.reason_id, reason).is_some() {
                return Err(RegistryBuildError::DuplicateReason);
            }
        }
        let mut used_reasons = BTreeSet::new();
        let mut seen_coordinates = BTreeSet::new();
        let mut rows = Vec::with_capacity(dispositions.len());
        for row in dispositions {
            let coordinate = DispositionCoordinate::from_generated(
                row.category,
                row.surface,
                row.kind,
                row.member,
                row.qualifier,
            );
            if !seen_coordinates.insert(coordinate.clone()) {
                return Err(RegistryBuildError::DuplicateCoordinate);
            }
            let registered = if row.disposition == DispositionKind::TypedFallback {
                let reason_id = row
                    .fallback_reason_id
                    .ok_or(RegistryBuildError::MissingReason)?;
                if row.target_contract.is_some() {
                    return Err(RegistryBuildError::InvalidDispositionShape);
                }
                let reason = reason_by_id
                    .get(reason_id)
                    .ok_or(RegistryBuildError::MissingReason)?;
                if reason.disposition_owner != coordinate.owner() {
                    return Err(RegistryBuildError::WrongReasonOwner);
                }
                let expected_type = parse_document(reason.expected_type_schema)
                    .ok()
                    .and_then(|datum| TypeExpr::parse(&datum).ok())
                    .ok_or(RegistryBuildError::InvalidExpectedType)?;
                let minimum_raw_owner =
                    MinimumRawOwner::parse_generated(reason.minimum_raw_owner_type)
                        .ok_or(RegistryBuildError::UnknownMinimumRawOwner)?;
                used_reasons.insert(reason_id);
                new!(RegisteredDisposition::TypedFallback {
                    boundary: new!(FallbackBoundary {
                        disposition_owner: coordinate,
                        reason_id: FallbackReasonId::from_generated(reason_id),
                        expected_type,
                        minimum_raw_owner,
                    }),
                })
            } else {
                if row.fallback_reason_id.is_some() {
                    return Err(RegistryBuildError::InvalidDispositionShape);
                }
                let target_contract = row
                    .target_contract
                    .filter(|target| !target.is_empty())
                    .ok_or(RegistryBuildError::InvalidDispositionShape)?;
                new!(RegisteredDisposition::NonFallback {
                    coordinate,
                    kind: row.disposition,
                    target_contract: TargetContract::from_generated(target_contract),
                })
            };
            rows.push(registered);
        }
        if used_reasons.len() != reasons.len() {
            return Err(RegistryBuildError::OrphanReason);
        }
        Ok(new!(DispositionRegistry { rows }))
    }

    #[requires(true)]
    #[ensures(ret.len() == self.rows.len())]
    pub(crate) fn iter(&self) -> impl ExactSizeIterator<Item = &RegisteredDisposition> {
        self.rows.iter()
    }

    #[requires(!surface.is_empty() && !member.is_empty())]
    #[requires(qualifier.is_none_or(|value| !value.is_empty()))]
    #[ensures(ret.as_ref().is_ok_and(|row| row.coordinate().surface == surface && row.coordinate().member == member) || ret.is_err())]
    pub(crate) fn lookup(
        &self,
        category: CoordinateCategory,
        surface: &str,
        kind: CoordinateKind,
        member: &str,
        qualifier: Option<&str>,
    ) -> Result<&RegisteredDisposition, RegistryLookupError> {
        self.rows
            .iter()
            .find(|row| {
                let coordinate = row.coordinate();
                coordinate.category == category
                    && coordinate.surface == surface
                    && coordinate.kind == kind
                    && coordinate.member == member
                    && coordinate.qualifier == qualifier
            })
            .ok_or(RegistryLookupError::UnknownCoordinate)
    }
}

/// Return the one lazily validated candidate registry.
#[requires(true)]
#[ensures(ret.iter().len() == GENERATED_DISPOSITION_ROWS.len())]
pub(crate) fn disposition_registry() -> &'static DispositionRegistry {
    static REGISTRY: OnceLock<DispositionRegistry> = OnceLock::new();
    REGISTRY.get_or_init(|| {
        DispositionRegistry::try_from_generated(
            GENERATED_DISPOSITION_ROWS,
            GENERATED_FALLBACK_REASON_ROWS,
        )
        .expect("the generated smusni-v0 registry was validated before compilation")
    })
}

#[requires(true)]
#[ensures(true)]
fn rows_have_unique_coordinates(rows: &[RegisteredDisposition]) -> bool {
    let mut seen = BTreeSet::new();
    rows.iter().all(|row| seen.insert(row.coordinate().clone()))
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn category_name(category: CoordinateCategory) -> &'static str {
    match category {
        CoordinateCategory::Object => "Object",
        CoordinateCategory::ValueStruct => "ValueStruct",
        CoordinateCategory::Enum => "Enum",
        CoordinateCategory::Document => "Document",
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn kind_name(kind: CoordinateKind) -> &'static str {
    match kind {
        CoordinateKind::Constructor => "Constructor",
        CoordinateKind::Discriminator => "Discriminator",
        CoordinateKind::Field => "Field",
        CoordinateKind::EnumVariant => "EnumVariant",
        CoordinateKind::VariantField => "VariantField",
        CoordinateKind::DerivedFact => "DerivedFact",
    }
}

/// Private generated lexical-policy record used by the CP2 typed foundation.
// These private records are the unchecked serialization shape emitted by the
// generator. `DispositionRegistry::try_from_generated` is the single boundary
// that validates them into the invariant-bearing runtime model.
#[invariant(true)]
#[derive(Debug, Clone, Copy)]
pub(super) struct GeneratedLexicalPolicyRow {
    pub(super) relation: &'static str,
    pub(super) original_place: usize,
    pub(super) attested_arity: usize,
    pub(super) accepted_family: DynamicValueFamily,
    pub(super) policy: ScopePolicy,
}

/// Private generated disposition record. It is validated before becoming a
/// public-to-the-crate registry value.
#[invariant(true)]
#[derive(Debug, Clone, Copy)]
struct GeneratedDispositionRow {
    category: CoordinateCategory,
    surface: &'static str,
    kind: CoordinateKind,
    member: &'static str,
    qualifier: Option<&'static str>,
    disposition: DispositionKind,
    target_contract: Option<&'static str>,
    fallback_reason_id: Option<&'static str>,
}

/// Private generated reason row used only for the validated join.
#[invariant(true)]
#[derive(Debug, Clone, Copy)]
struct GeneratedFallbackReasonRow {
    reason_id: &'static str,
    expected_type_schema: &'static str,
    minimum_raw_owner_type: &'static str,
    disposition_owner: &'static str,
}

include!(concat!(env!("OUT_DIR"), "/lexical_scope_policies.rs"));

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn generated_registry_round_trips_every_disposition_and_boundary() {
        let registry = disposition_registry();
        assert_eq!(registry.iter().len(), GENERATED_DISPOSITION_ROWS.len());
        assert_eq!(registry.iter().len(), 882);
        assert_eq!(
            registry
                .iter()
                .filter(|row| row.fallback_boundary().is_some())
                .count(),
            60
        );
        for row in registry.iter() {
            let coordinate = row.coordinate();
            let looked_up = registry
                .lookup(
                    coordinate.category,
                    coordinate.surface,
                    coordinate.kind,
                    coordinate.member,
                    coordinate.qualifier,
                )
                .expect("every iterated coordinate round-trips");
            assert_eq!(looked_up, row);
            assert_ne!(
                row.target_contract().is_some(),
                row.fallback_boundary().is_some()
            );
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn exact_lookup_does_not_borrow_a_sibling_variant() {
        let registry = disposition_registry();
        assert!(
            registry
                .lookup(
                    CoordinateCategory::Object,
                    "Formula",
                    CoordinateKind::VariantField,
                    "body",
                    Some("FormulaNode::Quantified"),
                )
                .is_ok()
        );
        assert_eq!(
            registry.lookup(
                CoordinateCategory::Object,
                "Formula",
                CoordinateKind::VariantField,
                "body",
                Some("FormulaNode::Atom"),
            ),
            Err(RegistryLookupError::UnknownCoordinate)
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn generated_join_rejects_every_malformed_boundary_class() {
        let fallback_index = GENERATED_DISPOSITION_ROWS
            .iter()
            .position(|row| row.disposition == DispositionKind::TypedFallback)
            .expect("generated fallback");
        let reason_id = GENERATED_DISPOSITION_ROWS[fallback_index]
            .fallback_reason_id
            .expect("fallback reason");
        let reason_index = GENERATED_FALLBACK_REASON_ROWS
            .iter()
            .position(|row| row.reason_id == reason_id)
            .expect("joined reason");

        let mut dispositions = GENERATED_DISPOSITION_ROWS.to_vec();
        dispositions.push(dispositions[0]);
        assert_eq!(
            DispositionRegistry::try_from_generated(&dispositions, GENERATED_FALLBACK_REASON_ROWS),
            Err(RegistryBuildError::DuplicateCoordinate)
        );

        let mut reasons = GENERATED_FALLBACK_REASON_ROWS.to_vec();
        reasons.push(reasons[0]);
        assert_eq!(
            DispositionRegistry::try_from_generated(GENERATED_DISPOSITION_ROWS, &reasons),
            Err(RegistryBuildError::DuplicateReason)
        );

        let mut dispositions = GENERATED_DISPOSITION_ROWS.to_vec();
        dispositions[fallback_index].fallback_reason_id = Some("smusni.fallback.absent");
        assert_eq!(
            DispositionRegistry::try_from_generated(&dispositions, GENERATED_FALLBACK_REASON_ROWS),
            Err(RegistryBuildError::MissingReason)
        );

        let mut reasons = GENERATED_FALLBACK_REASON_ROWS.to_vec();
        reasons[reason_index].disposition_owner = "Object:Wrong:Field:owner";
        assert_eq!(
            DispositionRegistry::try_from_generated(GENERATED_DISPOSITION_ROWS, &reasons),
            Err(RegistryBuildError::WrongReasonOwner)
        );

        let mut reasons = GENERATED_FALLBACK_REASON_ROWS.to_vec();
        reasons[reason_index].expected_type_schema = "(NotAType)";
        assert_eq!(
            DispositionRegistry::try_from_generated(GENERATED_DISPOSITION_ROWS, &reasons),
            Err(RegistryBuildError::InvalidExpectedType)
        );

        let mut reasons = GENERATED_FALLBACK_REASON_ROWS.to_vec();
        reasons[reason_index].minimum_raw_owner_type = "InventedOwner";
        assert_eq!(
            DispositionRegistry::try_from_generated(GENERATED_DISPOSITION_ROWS, &reasons),
            Err(RegistryBuildError::UnknownMinimumRawOwner)
        );

        let mut reasons = GENERATED_FALLBACK_REASON_ROWS.to_vec();
        reasons.remove(reason_index);
        assert_eq!(
            DispositionRegistry::try_from_generated(GENERATED_DISPOSITION_ROWS, &reasons),
            Err(RegistryBuildError::MissingReason)
        );
    }
}
