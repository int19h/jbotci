//! Field-level completeness registration for typed smusni S-expressions.
//!
//! Compact recognizers account for every field they consume. Anything they do
//! not prove equivalent is emitted through the mechanically complete typed
//! structural fallback. The ordinary profile suppresses only source provenance
//! and the surface spelling of an adjunct introducer.

#[allow(unused_imports)]
use bityzba::{data, ensures, requires};

use crate::completeness::model::DispositionData;
use crate::completeness::{
    CompletenessContract, Disposition, InventoryEntry, adjunct_introducer_provenance_reason,
    render_field_inventory, source_link_surfaces, source_provenance_reason,
};

/// Whether an inventory coordinate is ordinary-profile provenance.
#[requires(true)]
#[ensures(ret == (matches!(entry.surface.name, "SemanticSource" | "SourceByteSpan")
    || (entry.field == "source" && source_link_surfaces().contains(&entry.surface.name))
    || (entry.surface.name == "Adjunct" && entry.field == "introducedBy")))]
fn is_suppressed_provenance(entry: &InventoryEntry) -> bool {
    matches!(entry.surface.name, "SemanticSource" | "SourceByteSpan")
        || (entry.field == "source" && source_link_surfaces().contains(&entry.surface.name))
        || (entry.surface.name == "Adjunct" && entry.field == "introducedBy")
}

/// The ordinary-profile disposition of one inventoried surface.
#[requires(true)]
#[ensures(matches!(ret.as_data(), data!(Disposition::ExcludedWithReason(_)))
    == is_suppressed_provenance(entry))]
#[ensures(matches!(ret.as_data(), data!(Disposition::Renders))
    == !is_suppressed_provenance(entry))]
pub fn renderer_disposition(entry: &InventoryEntry) -> Disposition {
    if is_suppressed_provenance(entry) {
        let reason = if entry.surface.name == "Adjunct" && entry.field == "introducedBy" {
            adjunct_introducer_provenance_reason()
        } else {
            source_provenance_reason()
        };
        return Disposition::excluded_with_reason(reason);
    }
    Disposition::renders()
}

/// Complete field registry for the one ordinary smusni profile.
#[requires(true)]
#[ensures(ret.len() == render_field_inventory().len())]
pub fn smusni_coverage_contract() -> CompletenessContract {
    let inventory = render_field_inventory();
    let mut contract = CompletenessContract::new();
    for entry in inventory.entries() {
        contract
            .try_register(entry.key(), renderer_disposition(entry))
            .expect("inventory keys are unique");
    }
    contract
}

/// Partition the registry by whether the frozen corpus witnesses each field.
#[requires(true)]
#[ensures(ret.0 + ret.1 == render_field_inventory().len())]
pub fn corpus_presence_coverage() -> (usize, usize) {
    let inventory = render_field_inventory();
    let corpus_present = inventory.witnessed_count();
    (corpus_present, inventory.len() - corpus_present)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::completeness::baseline_contract_for;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn registry_is_complete_and_matches_the_ordinary_profile() {
        let inventory = render_field_inventory();
        let contract = smusni_coverage_contract();
        let audit = contract.audit(&inventory);
        assert!(
            audit.is_complete(),
            "missing={:?}; orphans={:?}",
            audit.missing,
            audit.orphans
        );
        assert!(
            contract
                .disagreements(&baseline_contract_for(&inventory))
                .is_empty()
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn corpus_presence_partitions_inventory() {
        let (present, absent) = corpus_presence_coverage();
        assert_eq!(present + absent, render_field_inventory().len());
        assert!(present > 0);
        assert!(absent > 0);
    }
}
