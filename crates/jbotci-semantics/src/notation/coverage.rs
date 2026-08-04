//! Field-level completeness registration for typed smusni S-expressions.
//!
//! Compact recognizers account for every field they consume. The executable
//! inventory states whether each coordinate lowers directly, desugars, belongs
//! to notation/provenance/diagnostics, or requires typed fallback.

#[allow(unused_imports)]
use bityzba::{ensures, requires};

use crate::completeness::{
    CompletenessContract, Disposition, InventoryEntry, render_field_inventory,
};

/// The ordinary-profile disposition of one inventoried surface.
#[requires(true)]
#[ensures(ret == entry.disposition)]
pub fn renderer_disposition(entry: &InventoryEntry) -> Disposition {
    entry.disposition
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
