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

use super::kernel::types::TypeExpr;
use super::sexpr::datum::parse_document;
use super::sexpr::type_syntax::parse_type;
use super::typed_ir::{DynamicValueFamily, ScopePolicy};
use crate::completeness::model::{FailureClass, WHOLE_GRAPH_RAW_ROOT_TYPE};

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
    Failure,
}

/// A generated route target contract. There is deliberately no public
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

/// A bundled projection-failure reason identity. Values can only come from the
/// generated reason table.
#[invariant(!text.is_empty() && text.starts_with("smusni.projection."))]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct ProjectionFailureReasonId {
    text: &'static str,
}

impl ProjectionFailureReasonId {
    #[requires(!text.is_empty() && text.starts_with("smusni.projection."))]
    #[ensures(ret.as_str() == text)]
    fn from_generated(text: &'static str) -> Self {
        new!(ProjectionFailureReasonId { text })
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

/// Where a registered failure is attributed in the raw model.
///
/// A `TypedPosition` site carries the parsed expected v0 type and the smallest
/// model owner. A `WholeGraph` site establishes no smusni type at all, so it
/// carries neither; its raw root is always `SemanticGraph`.
// A trivial marker invariant: each shape's own fields already carry everything
// this type asserts, so it keeps plain construction and matching.
#[invariant(::TypedPosition { .. } => true)]
#[invariant(::WholeGraph => true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RegisteredFailureSite {
    TypedPosition {
        expected_type: TypeExpr,
        minimum_raw_owner: MinimumRawOwner,
    },
    WholeGraph,
}

/// The exact validated disposition/reason join for one failure route.
#[invariant(disposition_owner.owner().starts_with("Object:") || disposition_owner.owner().starts_with("ValueStruct:") || disposition_owner.owner().starts_with("Enum:") || disposition_owner.owner().starts_with("Document:"))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FailureBoundary {
    disposition_owner: DispositionCoordinate,
    reason_id: ProjectionFailureReasonId,
    site: RegisteredFailureSite,
    class: FailureClass,
}

impl FailureBoundary {
    #[requires(true)]
    #[ensures(ret == &self.disposition_owner)]
    pub(crate) fn disposition_owner(&self) -> &DispositionCoordinate {
        &self.disposition_owner
    }

    #[requires(true)]
    #[ensures(ret == &self.reason_id)]
    pub(crate) fn reason_id(&self) -> &ProjectionFailureReasonId {
        &self.reason_id
    }

    #[requires(true)]
    #[ensures(ret == &self.site)]
    pub(crate) fn site(&self) -> &RegisteredFailureSite {
        &self.site
    }

    #[requires(true)]
    #[ensures(ret == self.class)]
    pub(crate) fn class(&self) -> FailureClass {
        self.class
    }

    #[requires(true)]
    #[ensures(ret.is_some() == matches!(self.site, RegisteredFailureSite::TypedPosition { .. }))]
    pub(crate) fn expected_type(&self) -> Option<&TypeExpr> {
        match &self.site {
            RegisteredFailureSite::TypedPosition { expected_type, .. } => Some(expected_type),
            RegisteredFailureSite::WholeGraph => None,
        }
    }

    #[requires(true)]
    #[ensures(ret.is_some() == matches!(self.site, RegisteredFailureSite::TypedPosition { .. }))]
    pub(crate) fn minimum_raw_owner(&self) -> Option<MinimumRawOwner> {
        match &self.site {
            RegisteredFailureSite::TypedPosition {
                minimum_raw_owner, ..
            } => Some(*minimum_raw_owner),
            RegisteredFailureSite::WholeGraph => None,
        }
    }
}

/// One fully joined runtime registry row.
#[invariant(::Route { kind, .. } => *kind != DispositionKind::Failure)]
#[invariant(::Failure { .. } => true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RegisteredDisposition {
    Route {
        coordinate: DispositionCoordinate,
        kind: DispositionKind,
        target_contract: TargetContract,
    },
    Failure {
        boundary: FailureBoundary,
    },
}

impl RegisteredDisposition {
    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn coordinate(&self) -> &DispositionCoordinate {
        match self.as_data() {
            data!(RegisteredDisposition::Route { coordinate, .. }) => coordinate,
            data!(RegisteredDisposition::Failure { boundary }) => boundary.disposition_owner(),
        }
    }

    #[requires(true)]
    #[ensures(ret.is_some() == matches!(self.as_data(), data!(RegisteredDisposition::Route { .. })))]
    pub(crate) fn target_contract(&self) -> Option<&TargetContract> {
        match self.as_data() {
            data!(RegisteredDisposition::Route {
                target_contract,
                ..
            }) => Some(target_contract),
            data!(RegisteredDisposition::Failure { .. }) => None,
        }
    }

    #[requires(true)]
    #[ensures(ret.is_some() == matches!(self.as_data(), data!(RegisteredDisposition::Failure { .. })))]
    pub(crate) fn failure_boundary(&self) -> Option<&FailureBoundary> {
        match self.as_data() {
            data!(RegisteredDisposition::Route { .. }) => None,
            data!(RegisteredDisposition::Failure { boundary }) => Some(boundary),
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
        reasons: &[GeneratedProjectionFailureReasonRow],
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
            let registered = if row.disposition == DispositionKind::Failure {
                let reason_id = row
                    .failure_reason_id
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
                let site = match reason.site {
                    GeneratedFailureSite::TypedPosition {
                        expected_type_schema,
                        minimum_raw_owner_type,
                    } => {
                        let expected_type = parse_document(expected_type_schema)
                            .ok()
                            .and_then(|datum| parse_type(&datum).ok())
                            .ok_or(RegistryBuildError::InvalidExpectedType)?;
                        let minimum_raw_owner =
                            MinimumRawOwner::parse_generated(minimum_raw_owner_type)
                                .ok_or(RegistryBuildError::UnknownMinimumRawOwner)?;
                        RegisteredFailureSite::TypedPosition {
                            expected_type,
                            minimum_raw_owner,
                        }
                    }
                    GeneratedFailureSite::WholeGraph { raw_root_type } => {
                        if raw_root_type != WHOLE_GRAPH_RAW_ROOT_TYPE {
                            return Err(RegistryBuildError::UnknownMinimumRawOwner);
                        }
                        RegisteredFailureSite::WholeGraph
                    }
                };
                used_reasons.insert(reason_id);
                new!(RegisteredDisposition::Failure {
                    boundary: new!(FailureBoundary {
                        disposition_owner: coordinate,
                        reason_id: ProjectionFailureReasonId::from_generated(reason_id),
                        site,
                        class: reason.failure_class,
                    }),
                })
            } else {
                if row.failure_reason_id.is_some() {
                    return Err(RegistryBuildError::InvalidDispositionShape);
                }
                let target_contract = row
                    .target_contract
                    .filter(|target| !target.is_empty())
                    .ok_or(RegistryBuildError::InvalidDispositionShape)?;
                new!(RegisteredDisposition::Route {
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
            GENERATED_PROJECTION_FAILURE_REASON_ROWS,
        )
        .expect("the generated smusni-v0 registry was validated before compilation")
    })
}

/// The registered section-16.2 class of one emitted projection failure.
///
/// The class is registry data, never a per-site judgment: a renderer that
/// emits `reason_id` reports exactly the class its reason row declares. An
/// unregistered id is executable/registry drift, which is itself the
/// `ImplementationInvariant` class rather than a silent default.
#[requires(!reason_id.is_empty())]
#[ensures(true)]
pub(crate) fn registered_failure_class(reason_id: &str) -> FailureClass {
    GENERATED_PROJECTION_FAILURE_REASON_ROWS
        .iter()
        .find(|row| row.reason_id == reason_id)
        .map_or(FailureClass::ImplementationInvariant, |row| {
            row.failure_class
        })
}

/// Whether the generated reason table registers this id at all.
#[requires(true)]
#[ensures(true)]
pub(crate) fn reason_id_is_registered(reason_id: &str) -> bool {
    GENERATED_PROJECTION_FAILURE_REASON_ROWS
        .iter()
        .any(|row| row.reason_id == reason_id)
}

/// Whether the registered site of this reason is the whole graph.
#[requires(true)]
#[ensures(true)]
pub(crate) fn reason_id_is_whole_graph(reason_id: &str) -> bool {
    GENERATED_PROJECTION_FAILURE_REASON_ROWS.iter().any(|row| {
        row.reason_id == reason_id && matches!(row.site, GeneratedFailureSite::WholeGraph { .. })
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
    failure_reason_id: Option<&'static str>,
}

/// Private generated failure-site tag used only for the validated join.
#[invariant(::TypedPosition { .. } => true)]
#[invariant(::WholeGraph { .. } => true)]
#[derive(Debug, Clone, Copy)]
enum GeneratedFailureSite {
    TypedPosition {
        expected_type_schema: &'static str,
        minimum_raw_owner_type: &'static str,
    },
    WholeGraph {
        raw_root_type: &'static str,
    },
}

/// Private generated reason row used only for the validated join.
#[invariant(true)]
#[derive(Debug, Clone, Copy)]
struct GeneratedProjectionFailureReasonRow {
    reason_id: &'static str,
    site: GeneratedFailureSite,
    failure_class: FailureClass,
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
        assert_eq!(registry.iter().len(), 944);
        assert_eq!(
            registry
                .iter()
                .filter(|row| row.failure_boundary().is_some())
                .count(),
            61
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
                row.failure_boundary().is_some()
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
        let failure_index = GENERATED_DISPOSITION_ROWS
            .iter()
            .position(|row| {
                row.disposition == DispositionKind::Failure
                    && row.failure_reason_id.is_some_and(|reason_id| {
                        GENERATED_PROJECTION_FAILURE_REASON_ROWS
                            .iter()
                            .any(|reason| {
                                reason.reason_id == reason_id
                                    && matches!(
                                        reason.site,
                                        GeneratedFailureSite::TypedPosition { .. }
                                    )
                            })
                    })
            })
            .expect("generated typed-position failure");
        let reason_id = GENERATED_DISPOSITION_ROWS[failure_index]
            .failure_reason_id
            .expect("failure reason");
        let reason_index = GENERATED_PROJECTION_FAILURE_REASON_ROWS
            .iter()
            .position(|row| row.reason_id == reason_id)
            .expect("joined reason");

        let mut dispositions = GENERATED_DISPOSITION_ROWS.to_vec();
        dispositions.push(dispositions[0]);
        assert_eq!(
            DispositionRegistry::try_from_generated(
                &dispositions,
                GENERATED_PROJECTION_FAILURE_REASON_ROWS
            ),
            Err(RegistryBuildError::DuplicateCoordinate)
        );

        let mut reasons = GENERATED_PROJECTION_FAILURE_REASON_ROWS.to_vec();
        reasons.push(reasons[0]);
        assert_eq!(
            DispositionRegistry::try_from_generated(GENERATED_DISPOSITION_ROWS, &reasons),
            Err(RegistryBuildError::DuplicateReason)
        );

        let mut dispositions = GENERATED_DISPOSITION_ROWS.to_vec();
        dispositions[failure_index].failure_reason_id = Some("smusni.projection.absent");
        assert_eq!(
            DispositionRegistry::try_from_generated(
                &dispositions,
                GENERATED_PROJECTION_FAILURE_REASON_ROWS
            ),
            Err(RegistryBuildError::MissingReason)
        );

        let mut reasons = GENERATED_PROJECTION_FAILURE_REASON_ROWS.to_vec();
        reasons[reason_index].disposition_owner = "Object:Wrong:Field:owner";
        assert_eq!(
            DispositionRegistry::try_from_generated(GENERATED_DISPOSITION_ROWS, &reasons),
            Err(RegistryBuildError::WrongReasonOwner)
        );

        let mut reasons = GENERATED_PROJECTION_FAILURE_REASON_ROWS.to_vec();
        reasons[reason_index].site = GeneratedFailureSite::TypedPosition {
            expected_type_schema: "(NotAType)",
            minimum_raw_owner_type: "SemanticGraph",
        };
        assert_eq!(
            DispositionRegistry::try_from_generated(GENERATED_DISPOSITION_ROWS, &reasons),
            Err(RegistryBuildError::InvalidExpectedType)
        );

        let mut reasons = GENERATED_PROJECTION_FAILURE_REASON_ROWS.to_vec();
        reasons[reason_index].site = GeneratedFailureSite::TypedPosition {
            expected_type_schema: "Performable",
            minimum_raw_owner_type: "InventedOwner",
        };
        assert_eq!(
            DispositionRegistry::try_from_generated(GENERATED_DISPOSITION_ROWS, &reasons),
            Err(RegistryBuildError::UnknownMinimumRawOwner)
        );

        let mut reasons = GENERATED_PROJECTION_FAILURE_REASON_ROWS.to_vec();
        reasons.remove(reason_index);
        assert_eq!(
            DispositionRegistry::try_from_generated(GENERATED_DISPOSITION_ROWS, &reasons),
            Err(RegistryBuildError::MissingReason)
        );
    }
}
