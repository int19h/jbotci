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
        EntryKind::Field => "Field",
        EntryKind::Variant => "Variant",
        EntryKind::DerivedFact => "DerivedFact",
    };
    let (disposition, detail, fallback_reason_id, expected_type_schema, minimum_raw_owner_type) =
        match entry.disposition.as_data() {
            data!(Disposition::DirectLowering) => ("DirectLowering", None, None, None, None),
            data!(Disposition::ProvenDesugaring) => ("ProvenDesugaring", None, None, None, None),
            data!(Disposition::NotationDefault(reason)) => {
                ("NotationDefault", Some(*reason), None, None, None)
            }
            data!(Disposition::ProvenanceSuppression(reason)) => {
                ("ProvenanceSuppression", Some(*reason), None, None, None)
            }
            data!(Disposition::DiagnosticCollection) => {
                ("DiagnosticCollection", None, None, None, None)
            }
            data!(Disposition::TypedFallback {
                reason,
                expected_type_schema,
                minimum_raw_owner_type,
                reason_id,
            }) => (
                "TypedFallback",
                Some(*reason),
                *reason_id,
                Some(*expected_type_schema),
                Some(*minimum_raw_owner_type),
            ),
        };
    DispositionSeed {
        owner: format!("{category}:{}:{kind}:{}", entry.surface.name, entry.field),
        model_member: match entry.variant_of {
            Some(variant) => format!("{kind}:{}@{variant}", entry.field),
            None => format!("{kind}:{}", entry.field),
        },
        disposition: disposition.to_owned(),
        detail: detail.map(str::to_owned),
        fallback_reason_id: fallback_reason_id.map(str::to_owned),
        expected_type_schema: expected_type_schema.map(str::to_owned),
        minimum_raw_owner_type: minimum_raw_owner_type.map(str::to_owned),
    }
}
