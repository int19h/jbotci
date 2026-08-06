//! Mechanical projection of the one authored semantic-surface ledger.

#[allow(unused_imports)]
use bityzba::{data, ensures, requires};

use crate::smusni_v0_bundle::DispositionSeed;
#[allow(unused_imports)]
use crate::smusni_v0_completeness::model::{
    Disposition, DispositionData, EntryKind, InventoryEntry, SurfaceCategory,
};

/// Project every authored inventory coordinate without a catch-all default.
#[requires(true)]
#[ensures(!ret.is_empty())]
pub fn projected_dispositions() -> Vec<DispositionSeed> {
    crate::smusni_v0_completeness::inventory::render_field_inventory()
        .entries()
        .iter()
        .map(disposition_seed)
        .collect()
}

#[requires(true)]
#[ensures(!ret.owner.is_empty() && !ret.model_member.is_empty() && !ret.disposition.is_empty())]
fn disposition_seed(entry: &InventoryEntry) -> DispositionSeed {
    let category = match entry.surface.category {
        SurfaceCategory::Object => "Object",
        SurfaceCategory::ValueStruct => "ValueStruct",
        SurfaceCategory::Enum => "Enum",
        SurfaceCategory::Document => "Document",
    };
    let kind = match entry.kind {
        EntryKind::Constructor => "Constructor",
        EntryKind::Discriminator => "Discriminator",
        EntryKind::Field => "Field",
        EntryKind::EnumVariant => "EnumVariant",
        EntryKind::VariantField => "VariantField",
        EntryKind::DerivedFact => "DerivedFact",
    };
    let (
        disposition,
        target_contract,
        detail,
        fallback_reason_id,
        expected_type_schema,
        minimum_raw_owner_type,
    ) = match entry.disposition.as_data() {
        data!(Disposition::DirectLowering { target_contract }) => (
            "DirectLowering",
            Some(*target_contract),
            None,
            None,
            None,
            None,
        ),
        data!(Disposition::ProvenDesugaring { target_contract }) => (
            "ProvenDesugaring",
            Some(*target_contract),
            None,
            None,
            None,
            None,
        ),
        data!(Disposition::NotationDefault {
            target_contract,
            reason
        }) => (
            "NotationDefault",
            Some(*target_contract),
            Some(*reason),
            None,
            None,
            None,
        ),
        data!(Disposition::ProvenanceSuppression {
            target_contract,
            reason
        }) => (
            "ProvenanceSuppression",
            Some(*target_contract),
            Some(*reason),
            None,
            None,
            None,
        ),
        data!(Disposition::DiagnosticCollection { target_contract }) => (
            "DiagnosticCollection",
            Some(*target_contract),
            None,
            None,
            None,
            None,
        ),
        data!(Disposition::TypedFallback {
            reason,
            expected_type_schema,
            minimum_raw_owner_type,
            reason_id,
        }) => (
            "TypedFallback",
            None,
            Some(*reason),
            Some(*reason_id),
            Some(*expected_type_schema),
            Some(*minimum_raw_owner_type),
        ),
    };
    let member = entry.variant_of.map_or_else(
        || entry.field.to_owned(),
        |variant| format!("{}@{variant}", entry.field),
    );
    DispositionSeed {
        owner: format!("{category}:{}:{kind}:{member}", entry.surface.name),
        model_member: format!("{kind}:{member}"),
        disposition: disposition.to_owned(),
        target_contract: target_contract.map(str::to_owned),
        detail: detail.map(str::to_owned),
        fallback_reason_id: fallback_reason_id.map(str::to_owned),
        expected_type_schema: expected_type_schema.map(str::to_owned),
        minimum_raw_owner_type: minimum_raw_owner_type.map(str::to_owned),
    }
}
