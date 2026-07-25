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
use super::render::Lean3Config;

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

/// The disposition the `lean3` renderer applies to one inventory entry under a
/// given [`Lean3Config`], stated from the render code (see the module docs). The
/// coverage is genuinely *config-dependent* (round-1 review, kimi 8): with
/// provenance OFF (the default, which the design-intent baseline models) every
/// source-provenance coordinate is `ExcludedWithReason`; with provenance ON
/// `render_source` emits them, so they become `Renders`. The postconditions pin
/// the exact policy for both configs so a regression changes a proof obligation,
/// not just a number.
#[requires(true)]
#[ensures(matches!(ret.as_data(), data!(Disposition::ExcludedWithReason(_)))
    == (renderer_excludes_as_provenance(entry) && !config.provenance))]
#[ensures(matches!(ret.as_data(), data!(Disposition::NotComputedDeclared))
    == (renderer_declares_not_computed(entry)
        && !(renderer_excludes_as_provenance(entry) && !config.provenance)))]
pub fn renderer_disposition(entry: &InventoryEntry, config: Lean3Config) -> Disposition {
    if renderer_excludes_as_provenance(entry) {
        if config.provenance {
            // Provenance on: `render_source` renders the span/text/construct.
            return Disposition::renders();
        }
        // Provenance off (default): the renderer excludes exactly these
        // coordinates, adopting the spec's own stated reason verbatim (the
        // `Disposition` carries the reason, so agreeing with the baseline means
        // reusing it) — realised by `render_source` returning before emitting.
        return Disposition::excluded_with_reason(source_provenance_reason());
    }
    if renderer_declares_not_computed(entry) {
        return Disposition::not_computed_declared();
    }
    Disposition::renders()
}

/// The `lean3` renderer's completeness contract under `config`: every inventory
/// entry registered (via `try_register`, so a double-cover is rejected) with the
/// disposition the renderer applies. Pass [`Lean3Config::default`] (provenance
/// off) for the profile the design-intent baseline models.
#[requires(true)]
#[ensures(ret.len() == render_field_inventory().len())]
pub fn lean3_coverage_contract(config: Lean3Config) -> CompletenessContract {
    let inventory = render_field_inventory();
    let mut contract = CompletenessContract::new();
    for entry in inventory.entries() {
        contract
            .try_register(entry.key(), renderer_disposition(entry, config))
            .expect("inventory entries are unique by key, so registration never collides");
    }
    contract
}

/// Behavioral-verification split of the inventory (round-1 review, kimi 6 /
/// Codex 1): a zero-`disagreements` coverage contract certifies *declared*
/// intent, not *observed* behaviour. This partitions entries into those a frozen
/// corpus document actually exercises (`witnessed` — their rendering is
/// byte-checked by `tests/lean3_parity.rs`) and those with no corpus witness
/// (`declared_only` — behaviorally unverified, future test-fixture debt). The
/// distinction is read from the inventory's own [`Witness`](crate::completeness::Witness)
/// data, so "zero disagreements" can never be misread as "fully verified".
#[requires(true)]
#[ensures(ret.0 + ret.1 == render_field_inventory().len())]
pub fn behavioral_coverage() -> (usize, usize) {
    let inventory = render_field_inventory();
    let witnessed = inventory.witnessed_count();
    (witnessed, inventory.len() - witnessed)
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
        let contract = lean3_coverage_contract(Lean3Config::default());
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
        // The baseline models the DEFAULT profile (provenance off).
        let contract = lean3_coverage_contract(Lean3Config::default());
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

    /// The coverage is config-dependent (kimi 8): turning provenance ON
    /// reclassifies exactly the source-provenance entries (and only those) from
    /// `ExcludedWithReason` to `Renders`, so it disagrees with the
    /// provenance-off baseline on precisely that set — nothing else moves.
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn provenance_on_flips_exactly_the_source_entries() {
        let inventory = render_field_inventory();
        let with_provenance = lean3_coverage_contract(Lean3Config { provenance: true });
        let baseline = baseline_contract_for(&inventory);
        let disagreements = with_provenance.disagreements(&baseline);
        let source_entries = inventory
            .entries()
            .iter()
            .filter(|entry| renderer_excludes_as_provenance(entry))
            .count();
        assert_eq!(
            disagreements.len(),
            source_entries,
            "provenance-on must flip exactly the {source_entries} source-provenance entries"
        );
        assert!(disagreements
            .iter()
            .all(|key| key.field == "source"
                || matches!(key.surface, "SemanticSource" | "SourceByteSpan")));
    }

    /// The behavioral-verification split is surfaced and consistent (kimi 6):
    /// some entries are corpus-exercised (their rendering is byte-checked) and
    /// the bulk are declared-only (behaviorally unverified) — the two partition
    /// the whole inventory.
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn behavioral_coverage_partitions_the_inventory() {
        let (witnessed, declared_only) = behavioral_coverage();
        assert_eq!(witnessed + declared_only, render_field_inventory().len());
        assert!(witnessed > 0, "some entries must be corpus-exercised");
        assert!(
            declared_only > 0,
            "the declared-only (behaviorally unverified) set must be surfaced, not hidden"
        );
    }
}
