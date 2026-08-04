//! Ordinary-profile disposition contract for typed smusni S-expressions.
//!
//! Every authored semantic field and variant carries its Draft-9 disposition
//! directly on the inventory row. There is no predicate or wildcard that can
//! assign a new generated field a permissive default.

#[allow(unused_imports)]
use bityzba::{ensures, requires};

use super::inventory::render_field_inventory;
use super::model::{CompletenessContract, Disposition, InventoryEntry, RenderFieldInventory};

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
#[ensures(ret == entry.disposition)]
pub fn baseline_disposition(entry: &InventoryEntry) -> Disposition {
    entry.disposition
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
