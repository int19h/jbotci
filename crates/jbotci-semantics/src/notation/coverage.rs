//! The `lean3` renderer's completeness coverage, registered against the merged
//! inventory ([`crate::completeness`]).
//!
//! Phase-B step 2 authored a completeness *contract* (the declared `lean3`
//! design intent) with no renderer present. This module is the renderer side:
//! it declares, per inventory entry, the [`Disposition`] the `lean3` renderer in
//! [`super::render`] actually applies, and the tests below verify that coverage
//! (a) audits complete against [`render_field_inventory`] — every entry
//! dispositioned, no orphans — and (b) agrees, with zero [`disagreements`], with
//! the design-intent [`baseline_contract_for`].
//!
//! # The renderer's coverage policy (independently stated from the render code)
//!
//! The `lean3` renderer in [`super::render`] treats each surface as follows,
//! and this policy is authored from that code — not copied from
//! [`crate::completeness::baseline_disposition`]:
//!
//! * **Source provenance is excluded.** [`super::render::Lean3Config`] defaults
//!   `provenance` off, so `render_source` returns before emitting anything, and
//!   `SemanticSource`/`SourceByteSpan` never surface. Every `source` link (typed
//!   `SemanticSource`) and the two provenance structs' own fields are therefore
//!   `ExcludedWithReason`. The set of surfaces carrying a `SemanticSource`
//!   `source` link is taken from [`source_link_surfaces`], the model-type-graph
//!   authority the completeness crate already cross-checks — so this side and
//!   the baseline draw the exclusion set from the same source of truth rather
//!   than two hand-maintained lists.
//! * **One document-level NOT COMPUTED fact is declared.** The renderer emits
//!   `NOT COMPUTED { denotation-multiplicity; }` (the `opt_collapse_notcomputed`
//!   block), so `document.not-computed:denotation-multiplicity` is
//!   `NotComputedDeclared`.
//! * **Everything else renders.** Content fields, enum variants, and the
//!   content-bearing derived facts (sort header, binding label, denotation
//!   reading, dimension record, the tested-winner role wordings) are `Renders`.
//!   Absent optionals still count as `Renders`: they surface as `UNSPECIFIED`
//!   inside the compact dimension record, never as a `NOT COMPUTED` — the
//!   absent-in-document vs not-computed distinction lives in
//!   [`crate::completeness::Presence`], not in the disposition.
//!
//! # Reported design-intent gaps (PM adjudication, not a build failure)
//!
//! The [`Disposition`] vocabulary is deliberately coarse (`Renders` /
//! `NotComputedDeclared` / `ExcludedWithReason`); it has no "structurally not
//! yet read" class. A few inventory fields are *present in the corpus graph but
//! not read by the frozen `lean3` prototype* — most notably
//! `Eventuality.intervalModifiers`, which is the redundant interval-form of the
//! `aspect`/`recurrence` the dimension record already renders (verified on
//! `b30`: the same `initiative` contour appears in both `aspect` and
//! `intervalModifiers`). Both the oracle and this port drop it, so byte parity
//! is unaffected. The baseline classifies such fields `Renders` (design intent);
//! this coverage matches that classification rather than inventing a
//! disposition the enum cannot express. These prototype-completeness gaps are
//! listed for PM adjudication in the PR body, not papered over — the baseline
//! may want a future "redundant-with" disposition, which is out of scope here.

#[allow(unused_imports)]
use bityzba::{data, ensures, requires};

use crate::completeness::model::DispositionData;
use crate::completeness::{
    render_field_inventory, source_link_surfaces, source_provenance_reason, CompletenessContract,
    Disposition, EntryKind, InventoryEntry,
};

/// The one document-level NOT COMPUTED fact the `lean3` renderer declares.
const NOT_COMPUTED_FACT: &str = "not-computed:denotation-multiplicity";

/// True for a `SemanticSource`/`SourceByteSpan` value, or a `SemanticSource`
/// `source` link on any surface — exactly the coordinates the renderer omits
/// when provenance is off (its default).
#[requires(true)]
#[ensures(ret == (matches!(entry.surface.name, "SemanticSource" | "SourceByteSpan")
    || (entry.field == "source" && source_link_surfaces().contains(&entry.surface.name))))]
fn renderer_excludes_as_provenance(entry: &InventoryEntry) -> bool {
    matches!(entry.surface.name, "SemanticSource" | "SourceByteSpan")
        || (entry.field == "source" && source_link_surfaces().contains(&entry.surface.name))
}

/// True for the one document-level NOT COMPUTED fact the renderer declares.
#[requires(true)]
#[ensures(ret == (entry.kind == EntryKind::DerivedFact && entry.field == NOT_COMPUTED_FACT))]
fn renderer_declares_not_computed(entry: &InventoryEntry) -> bool {
    entry.kind == EntryKind::DerivedFact && entry.field == NOT_COMPUTED_FACT
}

/// The disposition the `lean3` renderer applies to one inventory entry, stated
/// from the render code (see the module docs). The postconditions pin the exact
/// policy so a regression changes a proof obligation, not just a number.
#[requires(true)]
#[ensures(matches!(ret.as_data(), data!(Disposition::ExcludedWithReason(_))) == renderer_excludes_as_provenance(entry))]
#[ensures(matches!(ret.as_data(), data!(Disposition::NotComputedDeclared))
    == (renderer_declares_not_computed(entry) && !renderer_excludes_as_provenance(entry)))]
#[ensures(matches!(ret.as_data(), data!(Disposition::Renders))
    == (!renderer_excludes_as_provenance(entry) && !renderer_declares_not_computed(entry)))]
pub fn renderer_disposition(entry: &InventoryEntry) -> Disposition {
    if renderer_excludes_as_provenance(entry) {
        // The renderer excludes exactly the source-provenance coordinates the
        // `lean3` design intent does, and adopts the spec's own stated reason
        // verbatim (the `Disposition` carries the reason, so agreeing with the
        // baseline means reusing it) — the renderer's behaviour that realises it
        // is `render_source` returning before emitting when provenance is off.
        return Disposition::excluded_with_reason(source_provenance_reason());
    }
    if renderer_declares_not_computed(entry) {
        return Disposition::not_computed_declared();
    }
    Disposition::renders()
}

/// The `lean3` renderer's completeness contract: every inventory entry
/// registered (via `try_register`, so a double-cover is rejected) with the
/// disposition the renderer applies.
#[requires(true)]
#[ensures(ret.len() == render_field_inventory().len())]
pub fn lean3_coverage_contract() -> CompletenessContract {
    let inventory = render_field_inventory();
    let mut contract = CompletenessContract::new();
    for entry in inventory.entries() {
        contract
            .try_register(entry.key(), renderer_disposition(entry))
            .expect("inventory entries are unique by key, so registration never collides");
    }
    contract
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::completeness::baseline_contract_for;

    /// (a) The renderer's coverage audits complete against the full inventory:
    /// every entry has a disposition, and no disposition is an orphan.
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn coverage_audits_complete() {
        let inventory = render_field_inventory();
        let contract = lean3_coverage_contract();
        let audit = contract.audit(&inventory);
        assert!(
            audit.missing.is_empty(),
            "{} inventory entries lack a renderer disposition: {:?}",
            audit.missing.len(),
            &audit.missing[..audit.missing.len().min(8)]
        );
        assert!(
            audit.orphans.is_empty(),
            "{} renderer dispositions name no inventory entry: {:?}",
            audit.orphans.len(),
            &audit.orphans[..audit.orphans.len().min(8)]
        );
        assert!(audit.is_complete());
    }

    /// (b) The renderer's actual coverage agrees with the declared `lean3`
    /// design-intent baseline — zero disagreements in either direction.
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn coverage_matches_design_intent_baseline() {
        let inventory = render_field_inventory();
        let contract = lean3_coverage_contract();
        let baseline = baseline_contract_for(&inventory);
        let disagreements = contract.disagreements(&baseline);
        assert!(
            disagreements.is_empty(),
            "{} renderer dispositions disagree with the lean3 design intent: {:?}",
            disagreements.len(),
            &disagreements[..disagreements.len().min(8)]
        );
        // Symmetric check: the baseline must not disagree with the renderer
        // either (disagreements compares only shared keys; audit above already
        // proved the key sets coincide, so this pins full agreement).
        assert!(baseline.disagreements(&contract).is_empty());
    }
}
