//! Render-field completeness inventory and contract for the tersmu notation.
//!
//! Phase-B step 2 (research repo `DESIGN-RECORD.md`): before any notation
//! renderer is written, enumerate every serializable field, enum variant, and
//! derived/conditional fact of the `lojban-semantics-json-1` surface, and make
//! a checked contract that a future renderer registers coverage against. No
//! renderer lives here — only the inventory, the contract machinery, and (in
//! `tests/completeness.rs`) the corpus-witness and drift-guard tests.
//!
//! * [`model`] — the typed inventory/contract vocabulary.
//! * [`inventory::render_field_inventory`] — the authored entries.
//! * [`disposition::baseline_contract`] — the baseline `lean3` design contract.
//! * [`corpus`] — the frozen Phase-B corpus manifest.

pub mod corpus;
pub mod disposition;
pub mod inventory;
pub mod model;

pub use disposition::{baseline_contract, baseline_contract_for, baseline_disposition};
pub use inventory::render_field_inventory;
pub use model::{
    CompletenessContract, ContractAudit, Disposition, EntryKey, EntryKind, InventoryEntry,
    Presence, RenderFieldInventory, Surface, SurfaceCategory, Witness, WitnessExpect,
};

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use bityzba::{data, ensures, requires};

    use super::model::DispositionData;
    use super::*;

    /// The inventory builds (its unique-identity invariant holds) and is large
    /// enough to be the real surface, not a stub.
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn inventory_builds_and_is_unique() {
        let inventory = render_field_inventory();
        assert!(
            inventory.len() >= 500,
            "expected the full surface, got {} entries",
            inventory.len()
        );
        assert!(model::entries_have_unique_keys(inventory.entries()));
    }

    /// The baseline contract disposes of every inventory entry and has no
    /// orphan dispositions — the core completeness-contract guarantee.
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn baseline_contract_is_complete() {
        let inventory = render_field_inventory();
        let contract = baseline_contract_for(&inventory);
        let audit = contract.audit(&inventory);
        assert!(
            audit.missing.is_empty(),
            "{} inventory entries lack a disposition: {:?}",
            audit.missing.len(),
            &audit.missing[..audit.missing.len().min(8)]
        );
        assert!(
            audit.orphans.is_empty(),
            "{} dispositions name no inventory entry: {:?}",
            audit.orphans.len(),
            &audit.orphans[..audit.orphans.len().min(8)]
        );
        assert!(audit.is_complete());
    }

    /// A contract missing one entry is caught by the audit (the tripwire a
    /// future renderer must satisfy actually fires).
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn audit_detects_a_missing_disposition() {
        let inventory = render_field_inventory();
        let mut contract = CompletenessContract::new();
        // Register all but the first entry.
        for entry in inventory.entries().iter().skip(1) {
            contract.register(entry.key(), baseline_disposition(entry));
        }
        let audit = contract.audit(&inventory);
        assert_eq!(audit.missing.len(), 1);
        assert!(!audit.is_complete());
    }

    /// Source-provenance surfaces are excluded with a reason; content renders;
    /// the declared NOT-COMPUTED facts are declared, not spuriously excluded.
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn disposition_policy_is_evidence_grounded() {
        let inventory = render_field_inventory();
        let mut saw_excluded = false;
        let mut saw_not_computed = false;
        for entry in inventory.entries() {
            let disposition = baseline_disposition(entry);
            if entry.surface.name == "SemanticSource" {
                assert!(
                    matches!(disposition.as_data(), data!(Disposition::ExcludedWithReason(_))),
                    "source provenance must be excluded with a reason"
                );
                saw_excluded = true;
            }
            if entry.field == "not-computed:denotation-multiplicity" {
                assert!(matches!(
                    disposition.as_data(),
                    data!(Disposition::NotComputedDeclared)
                ));
                saw_not_computed = true;
            }
        }
        assert!(saw_excluded && saw_not_computed);
    }
}
