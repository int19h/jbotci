//! The `smusni` renderer's completeness coverage, registered against the merged
//! inventory ([`crate::completeness`]).
//!
//! Phase-B step 2 authored a completeness *contract* (the declared `smusni`
//! design intent) with no renderer present. This module is the renderer side:
//! it declares, per inventory entry, the [`Disposition`] the `smusni` renderer in
//! [`super::render`] actually applies, and the tests below verify that coverage
//! (a) audits complete against [`render_field_inventory`] — every entry
//! dispositioned, no orphans — and (b) agrees, with zero [`disagreements`], with
//! the design-intent [`baseline_contract_for`].
//!
//! # The renderer's coverage policy (independently stated from the render code)
//!
//! The `smusni` renderer in [`super::render`] treats each surface as follows,
//! and this policy is authored from that code — not copied from
//! [`crate::completeness::baseline_disposition`]:
//!
//! * **Source provenance is config-dependent.** [`super::render::SmusniConfig`]
//!   defaults `provenance` off, so `render_source` renders nothing and every
//!   source-provenance coordinate is `ExcludedWithReason` — the profile the
//!   design-intent baseline models. With `provenance` ON, the coordinates the
//!   renderer actually emits ([`source_rendered_under_provenance`]: the 12
//!   object kinds it has renderers for, plus the traversed value structs
//!   `AssignedName`/`RelativeClause`/`ArgumentValue` per Amendment 2, plus the
//!   newly traversed `PlaceQuestionBinding`, plus the `SemanticSource`/
//!   `SourceByteSpan` fields those emit) become `Renders`; a source whose struct
//!   the renderer never renders (`RelationMetadata`, or untraversed value
//!   structs like `ModalArgument`)
//!   stays `ExcludedWithReason`. The source-link surface set comes from
//!   [`source_link_surfaces`], the model-type-graph authority the completeness
//!   crate cross-checks, so this side and the baseline share one source of truth.
//! * **One document-level NOT COMPUTED fact is declared.** The renderer emits
//!   `NOT COMPUTED { denotation-multiplicity; }`, so
//!   `document.not-computed:denotation-multiplicity` is `NotComputedDeclared`.
//! * **Everything else renders.** Content fields, enum variants, and the
//!   content-bearing derived facts are `Renders`. Absent optionals still count
//!   as `Renders` (they surface as `UNSPECIFIED`, never `NOT COMPUTED`); the
//!   absent-in-document vs not-computed distinction lives in
//!   [`crate::completeness::Presence`], not in the disposition.
//!   This includes the first-class QUESTION records and predication
//!   `PLACE QUESTIONS` bindings added by jbotci#622.
//!
//! # Known-consistent-pattern debt (documented, not a build failure)
//!
//! * **Runtime shape-markers.** For variant shapes not yet covered by a
//!   first-class notation design, the renderer emits an ad-hoc
//!   `NOT COMPUTED: <shape>(…)` marker at runtime (e.g. an `aspect`
//!   interval-modifier with no `contour`, a
//!   quotation with an unrecognised `mode`, a non-`integer` math literal). These
//!   are flag-don't-guess safety nets, NOT declared inventory facts. The
//!   discriminant-complete question witnesses added by jbotci#622 deliberately
//!   expose three adjacent pre-existing markers — connector parameters,
//!   quantity question parameters, and math-expression operator parameters —
//!   while the QUESTION and PLACE QUESTIONS surfaces themselves render fully.
//!   Promoting those adjacent shapes to first-class notation is separate
//!   follow-up work; it must not be conflated with a question-object gap.
//! * **Untraversed value structs (provenance).** Value structs the renderer
//!   never traverses (`AnchorMagnitude`, `ModalArgument`, `QuantifierBinding`,
//!   …) and the object kind with no per-kind renderer (`RelationMetadata`) have
//!   no rendered source under provenance; the coverage reflects this precisely
//!   (their `source` stays `ExcludedWithReason` even under provenance).
//!   `Question`/`PlaceQuestionBinding` are now traversed and their sources render
//!   when provenance is on; the remaining untraversed structs stay corpus-absent.
//!   (Argument-attached `RelativeClause`s ARE now rendered — Amendment 3 — so
//!   the round-2 deferral is resolved. `RelativeClause`/`ArgumentValue`
//!   traversal is total across the RENDERED parent paths — descriptors and
//!   predication arguments — and the provenance coverage is occurrence-accurate
//!   there; `ArgumentValue`s reachable only through the untraversed value
//!   structs above (`ModalArgument`, `ReciprocalExchange`) remain part of that
//!   acknowledged `NoCorpusWitness` debt, not covered by this claim.
//!   `PlaceQuestionBinding` is now traversed by the predication renderer, and
//!   its nested `ArgumentValue` participates in that rendered parent path.)

#[allow(unused_imports)]
use bityzba::{data, ensures, requires};

use crate::completeness::model::DispositionData;
use crate::completeness::{
    render_field_inventory, source_link_surfaces, source_provenance_reason, CompletenessContract,
    Disposition, EntryKind, InventoryEntry,
};
use super::render::SmusniConfig;

/// The one document-level NOT COMPUTED fact the `smusni` renderer declares.
const NOT_COMPUTED_FACT: &str = "not-computed:denotation-multiplicity";

/// The surfaces whose `source` the renderer's `render_source` is actually
/// reached on — the 12 object kinds it has per-kind renderers for (each calls
/// `render_source`), plus the four value structs it traverses and, under
/// Amendment 2, renders the nested source of: `AssignedName`, `RelativeClause`,
/// `ArgumentValue`, and jbotci#622's `PlaceQuestionBinding`. The object kind
/// with no per-kind renderer (`RelationMetadata`) falls to the `UNKNOWN` path
/// and never renders a source; the other source-bearing value structs
/// (`AnchorMagnitude`, `ModalArgument`, `QuantifierBinding`, ...) are not
/// traversed at all — so under provenance their `source` still does not surface,
/// and the coverage must not claim it does. (`RelativeClause` is now rendered
/// for BOTH descriptor-attached and argument-attached clauses — Amendment 3,
/// round-3 review — so its `source` is occurrence-accurate under provenance for
/// every clause on a rendered parent path; clauses on `ArgumentValue`s nested
/// inside the untraversed value structs above share those structs'
/// `NoCorpusWitness` debt.)
const SOURCE_RENDERED_SURFACES: &[&str] = &[
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
    "Question",
    "AssignedName",
    "RelativeClause",
    "ArgumentValue",
    "PlaceQuestionBinding",
];

/// True for a `SemanticSource`/`SourceByteSpan` value, or a `SemanticSource`
/// `source` link on any surface — the source-provenance coordinates.
#[requires(true)]
#[ensures(ret == (matches!(entry.surface.name, "SemanticSource" | "SourceByteSpan")
    || (entry.field == "source" && source_link_surfaces().contains(&entry.surface.name))))]
fn is_source_provenance(entry: &InventoryEntry) -> bool {
    matches!(entry.surface.name, "SemanticSource" | "SourceByteSpan")
        || (entry.field == "source" && source_link_surfaces().contains(&entry.surface.name))
}

/// True when, under `--provenance`, the renderer actually emits this
/// source-provenance coordinate: the source struct's own fields
/// (`SemanticSource`/`SourceByteSpan`) render whenever any source renders (top-
/// level object sources always do), and a `source` link renders iff its surface
/// is one the renderer reaches [`render_source`] on.
#[requires(is_source_provenance(entry))]
#[ensures(ret == (matches!(entry.surface.name, "SemanticSource" | "SourceByteSpan")
    || SOURCE_RENDERED_SURFACES.contains(&entry.surface.name)))]
fn source_rendered_under_provenance(entry: &InventoryEntry) -> bool {
    matches!(entry.surface.name, "SemanticSource" | "SourceByteSpan")
        || SOURCE_RENDERED_SURFACES.contains(&entry.surface.name)
}

/// True for the one document-level NOT COMPUTED fact the renderer declares.
#[requires(true)]
#[ensures(ret == (entry.kind == EntryKind::DerivedFact && entry.field == NOT_COMPUTED_FACT))]
fn renderer_declares_not_computed(entry: &InventoryEntry) -> bool {
    entry.kind == EntryKind::DerivedFact && entry.field == NOT_COMPUTED_FACT
}

/// True when the renderer excludes this entry from output — a source-provenance
/// coordinate the renderer does not emit: either provenance is off, or the
/// coordinate's struct is one the renderer never renders a source for.
#[requires(true)]
#[ensures(ret == (is_source_provenance(entry)
    && !(config.provenance && source_rendered_under_provenance(entry))))]
fn renderer_excludes(entry: &InventoryEntry, config: SmusniConfig) -> bool {
    is_source_provenance(entry) && !(config.provenance && source_rendered_under_provenance(entry))
}

/// The disposition the `smusni` renderer applies to one inventory entry under a
/// given [`SmusniConfig`], stated from the render code (see the module docs). The
/// coverage is genuinely *config-dependent* (round-1 review, kimi 8): with
/// provenance OFF (the default, which the design-intent baseline models) every
/// source-provenance coordinate is `ExcludedWithReason`; with provenance ON the
/// coordinates the renderer actually emits ([`source_rendered_under_provenance`])
/// become `Renders`, while a source whose struct the renderer never renders
/// stays `ExcludedWithReason` (round-2 review, Blocker 1 — the coverage must not
/// claim a nested source renders when it does not). The three postconditions pin
/// the exact policy for both configs so a regression changes a proof obligation.
#[requires(true)]
#[ensures(matches!(ret.as_data(), data!(Disposition::ExcludedWithReason(_)))
    == renderer_excludes(entry, config))]
#[ensures(matches!(ret.as_data(), data!(Disposition::NotComputedDeclared))
    == (!renderer_excludes(entry, config)
        && renderer_declares_not_computed(entry)))]
#[ensures(matches!(ret.as_data(), data!(Disposition::Renders))
    == (!renderer_excludes(entry, config)
        && !renderer_declares_not_computed(entry)))]
pub fn renderer_disposition(entry: &InventoryEntry, config: SmusniConfig) -> Disposition {
    if renderer_excludes(entry, config) {
        // Adopt the spec's own stated reason verbatim (the `Disposition` carries
        // the reason, so agreeing with the baseline means reusing it).
        return Disposition::excluded_with_reason(source_provenance_reason());
    }
    if renderer_declares_not_computed(entry) {
        return Disposition::not_computed_declared();
    }
    Disposition::renders()
}

/// The `smusni` renderer's completeness contract under `config`: every inventory
/// entry registered (via `try_register`, so a double-cover is rejected) with the
/// disposition the renderer applies. Pass [`SmusniConfig::default`] (provenance
/// off) for the profile the design-intent baseline models.
#[requires(true)]
#[ensures(ret.len() == render_field_inventory().len())]
pub fn smusni_coverage_contract(config: SmusniConfig) -> CompletenessContract {
    let inventory = render_field_inventory();
    let mut contract = CompletenessContract::new();
    for entry in inventory.entries() {
        contract
            .try_register(entry.key(), renderer_disposition(entry, config))
            .expect("inventory entries are unique by key, so registration never collides");
    }
    contract
}

/// Corpus-*presence* split of the inventory (round-1 kimi 6 / Codex 1; refined
/// round-2 Codex 5). A zero-`disagreements` coverage contract certifies
/// *declared* intent, not *observed* behaviour — and even this split certifies
/// only that a coordinate is **present** in some frozen corpus document, NOT
/// that the field affects output (the original silent drops were corpus-present
/// while parity still passed). It returns `(corpus_present, corpus_absent)` read
/// from the inventory's own [`Witness`](crate::completeness::Witness) data.
/// True read-grounding (proving each field changes the rendered bytes) is out of
/// scope for this PR; this is deliberately the weaker presence claim, named so
/// it cannot be misread as behavioral verification.
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

    /// (a) The renderer's coverage audits complete against the full inventory:
    /// every entry has a disposition, and no disposition is an orphan.
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn coverage_audits_complete() {
        let inventory = render_field_inventory();
        let contract = smusni_coverage_contract(SmusniConfig::default());
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

    /// (b) The renderer's actual coverage agrees with the declared `smusni`
    /// design-intent baseline — zero disagreements in either direction.
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn coverage_matches_design_intent_baseline() {
        let inventory = render_field_inventory();
        // The baseline models the DEFAULT profile (provenance off).
        let contract = smusni_coverage_contract(SmusniConfig::default());
        let baseline = baseline_contract_for(&inventory);
        let disagreements = contract.disagreements(&baseline);
        assert!(
            disagreements.is_empty(),
            "{} renderer dispositions disagree with the smusni design intent: {:?}",
            disagreements.len(),
            &disagreements[..disagreements.len().min(8)]
        );
        // Symmetric check: the baseline must not disagree with the renderer
        // either (disagreements compares only shared keys; audit above already
        // proved the key sets coincide, so this pins full agreement).
        assert!(baseline.disagreements(&contract).is_empty());
    }

    /// The coverage is config-dependent (kimi 8) AND accurate about which nested
    /// sources actually render (round-2 Blocker 1): turning provenance ON flips
    /// to `Renders` exactly the source coordinates the renderer emits
    /// ([`source_rendered_under_provenance`]) — no more (a source whose struct is
    /// never rendered stays `ExcludedWithReason`), no less, and nothing that
    /// isn't a source moves.
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn provenance_on_flips_exactly_the_rendered_source_entries() {
        let inventory = render_field_inventory();
        let with_provenance = smusni_coverage_contract(SmusniConfig { provenance: true });
        let baseline = baseline_contract_for(&inventory);
        let disagreements = with_provenance.disagreements(&baseline);
        let expected_flipped = inventory
            .entries()
            .iter()
            .filter(|entry| is_source_provenance(entry) && source_rendered_under_provenance(entry))
            .count();
        assert!(expected_flipped > 0, "some nested/object sources must render under provenance");
        assert_eq!(
            disagreements.len(),
            expected_flipped,
            "provenance-on must flip exactly the {expected_flipped} rendered source coordinates"
        );
        // Every flipped entry is a source coordinate whose struct is rendered.
        for key in &disagreements {
            let entry = inventory
                .entries()
                .iter()
                .find(|e| e.key() == *key)
                .expect("disagreement key is an inventory entry");
            assert!(is_source_provenance(entry) && source_rendered_under_provenance(entry));
        }
        // A non-rendered source struct (e.g. ModalArgument.source) does NOT flip.
        let unrendered = inventory.entries().iter().any(|entry| {
            is_source_provenance(entry) && !source_rendered_under_provenance(entry)
        });
        assert!(unrendered, "the corpus inventory includes a non-rendered source struct");
    }

    /// The corpus-presence split is surfaced and consistent (Codex 5): some
    /// entries are corpus-present (their coordinate appears in a frozen doc) and
    /// the bulk are corpus-absent — the two partition the whole inventory. This
    /// certifies presence, not read-grounding.
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn corpus_presence_partitions_the_inventory() {
        let (corpus_present, corpus_absent) = corpus_presence_coverage();
        assert_eq!(corpus_present + corpus_absent, render_field_inventory().len());
        assert!(corpus_present > 0, "some coordinates must be corpus-present");
        assert!(
            corpus_absent > 0,
            "the corpus-absent set must be surfaced, not hidden"
        );
    }
}
