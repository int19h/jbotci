//! The baseline `lean3` disposition contract.
//!
//! Phase-B forbids a renderer in this PR, but the completeness contract must be
//! exercised end to end: every inventoried entry needs a disposition or the
//! audit fails. This module registers the *declared design intent* of the frozen
//! `lean3` profile (DESIGN-RECORD.md + FREEZE-PHASE-B.md + the frozen
//! `*.lean3.txt` sample outputs) as the baseline. It is the spec the future
//! renderer is held to, not the renderer itself; the byte-parity PR will verify
//! actual output against these dispositions.
//!
//! Policy (evidence-grounded where possible):
//! * Source-provenance surfaces (`SemanticSource`, `SourceByteSpan`, and the
//!   `SemanticObjectCommon.source` link) are `ExcludedWithReason`: they carry no
//!   coordinate in any frozen `lean3` sample output — `lean3` renders semantic
//!   content, not source spans.
//! * The three explicit `NOT COMPUTED` derived facts are `NotComputedDeclared`
//!   (evidence: the `NOT COMPUTED { denotation-multiplicity; }` block present in
//!   every frozen `lean3` sample).
//! * Everything else — content fields, enum variants, content-bearing derived
//!   facts — is `Renders`, per DESIGN-RECORD.md's "content-complete rendering".
//!   Absent optionals stay `Renders` (they surface as `UNSPECIFIED`, not as a
//!   `NOT COMPUTED`); the absent-in-document vs not-computed distinction lives in
//!   [`super::model::Presence`], never in the disposition.

#[allow(unused_imports)]
use bityzba::{data, ensures, new, requires};

use super::inventory::render_field_inventory;
use super::model::{
    CompletenessContract, Disposition, DispositionData, EntryKind, InventoryEntry,
    RenderFieldInventory,
};

const SOURCE_PROVENANCE_REASON: &str =
    "source provenance; lean3 renders semantic content, not source spans (absent from every frozen lean3 sample output)";

/// The three document-level `NOT COMPUTED` facts the frozen `lean3` renderer
/// declares rather than computes.
#[requires(true)]
#[ensures(true)]
fn is_not_computed_fact(entry: &InventoryEntry) -> bool {
    entry.kind == EntryKind::DerivedFact
        && (entry.field.starts_with("not-computed:")
            || entry.field == "fact:role-composition-all-hold"
            || entry.field == "fact:role-binding-role-for")
}

/// True for the source-provenance surfaces `lean3` deliberately omits.
#[requires(true)]
#[ensures(true)]
fn is_source_provenance(entry: &InventoryEntry) -> bool {
    matches!(entry.surface.name, "SemanticSource" | "SourceByteSpan")
        || (entry.surface.name == "SemanticObjectCommon" && entry.field == "source")
}

/// The baseline disposition for a single entry under the `lean3` design intent.
#[requires(true)]
#[ensures(matches!(ret.as_data(), data!(Disposition::NotComputedDeclared)) == is_not_computed_fact(entry))]
pub fn baseline_disposition(entry: &InventoryEntry) -> Disposition {
    if is_source_provenance(entry) {
        return new!(Disposition::ExcludedWithReason(SOURCE_PROVENANCE_REASON));
    }
    if is_not_computed_fact(entry) {
        return new!(Disposition::NotComputedDeclared);
    }
    // The document envelope and every content field/variant/fact renders.
    new!(Disposition::Renders)
}

/// Build the baseline contract: every inventory entry registered with its
/// [`baseline_disposition`]. The audit against [`render_field_inventory`] is
/// complete by construction (verified by the contract-completeness test).
#[requires(true)]
#[ensures(ret.len() == inventory.len())]
pub fn baseline_contract_for(inventory: &RenderFieldInventory) -> CompletenessContract {
    let mut contract = CompletenessContract::new();
    for entry in inventory.entries() {
        contract.register(entry.key(), baseline_disposition(entry));
    }
    contract
}

/// The baseline contract over the full authored inventory.
#[requires(true)]
#[ensures(!ret.is_empty())]
pub fn baseline_contract() -> CompletenessContract {
    baseline_contract_for(&render_field_inventory())
}
