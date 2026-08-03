//! Ordinary-profile disposition contract for typed smusni S-expressions.
//!
//! Every semantic field and variant renders directly or through typed fallback.
//! Only source provenance and the surface spelling of an adjunct introducer are
//! suppressed by the named concise-profile rules.

#[allow(unused_imports)]
use bityzba::{data, ensures, new, requires};

use super::inventory::render_field_inventory;
use super::model::{
    CompletenessContract, Disposition, DispositionData, InventoryEntry, RenderFieldInventory,
};

const SOURCE_PROVENANCE_REASON: &str =
    "ordinary-profile source provenance suppression; retained by TypedGraph fallback";
const ADJUNCT_INTRODUCER_PROVENANCE_REASON: &str =
    "surface adjunct introducer; modal semantics use the actual predicate/place map";

const SOURCE_LINK_SURFACES: &[&str] = &[
    "Utterance",
    "Sequence",
    "Eventuality",
    "Referent",
    "Sign",
    "Parameter",
    "Predication",
    "Formula",
    "DisplayedContent",
    "MathExpression",
    "Quantity",
    "RelationMetadata",
    "Question",
    "AnchorMagnitude",
    "ArgumentValue",
    "AssignedName",
    "DisplayedContentModifier",
    "Adjunct",
    "OrdinalLabel",
    "PlaceQuestionBinding",
    "QuantifierBinding",
    "ReciprocalExchange",
    "Recurrence",
    "RelativeClause",
    "Subscript",
];

#[requires(true)]
#[ensures(ret == SOURCE_LINK_SURFACES)]
pub fn source_link_surfaces() -> &'static [&'static str] {
    SOURCE_LINK_SURFACES
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub fn source_provenance_reason() -> &'static str {
    SOURCE_PROVENANCE_REASON
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub fn adjunct_introducer_provenance_reason() -> &'static str {
    ADJUNCT_INTRODUCER_PROVENANCE_REASON
}

#[requires(true)]
#[ensures(ret == (matches!(entry.surface.name, "SemanticSource" | "SourceByteSpan")
    || (entry.field == "source" && SOURCE_LINK_SURFACES.contains(&entry.surface.name))))]
fn is_source_provenance(entry: &InventoryEntry) -> bool {
    matches!(entry.surface.name, "SemanticSource" | "SourceByteSpan")
        || (entry.field == "source" && SOURCE_LINK_SURFACES.contains(&entry.surface.name))
}

#[requires(true)]
#[ensures(ret == (entry.surface.name == "Adjunct" && entry.field == "introducedBy"))]
fn is_adjunct_introducer_provenance(entry: &InventoryEntry) -> bool {
    entry.surface.name == "Adjunct" && entry.field == "introducedBy"
}

#[requires(true)]
#[ensures(matches!(ret.as_data(), data!(Disposition::ExcludedWithReason(_)))
    == (is_source_provenance(entry) || is_adjunct_introducer_provenance(entry)))]
#[ensures(matches!(ret.as_data(), data!(Disposition::Renders))
    == !(is_source_provenance(entry) || is_adjunct_introducer_provenance(entry)))]
pub fn baseline_disposition(entry: &InventoryEntry) -> Disposition {
    if is_source_provenance(entry) {
        return new!(Disposition::ExcludedWithReason(SOURCE_PROVENANCE_REASON));
    }
    if is_adjunct_introducer_provenance(entry) {
        return new!(Disposition::ExcludedWithReason(
            ADJUNCT_INTRODUCER_PROVENANCE_REASON
        ));
    }
    new!(Disposition::Renders)
}

#[requires(true)]
#[ensures(ret.len() == inventory.len())]
pub fn baseline_contract_for(inventory: &RenderFieldInventory) -> CompletenessContract {
    let mut contract = CompletenessContract::new();
    for entry in inventory.entries() {
        contract
            .try_register(entry.key(), baseline_disposition(entry))
            .expect("inventory entries are unique by key");
    }
    contract
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub fn baseline_contract() -> CompletenessContract {
    baseline_contract_for(&render_field_inventory())
}
