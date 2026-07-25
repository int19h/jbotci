//! The baseline `smusni` disposition contract.
//!
//! Phase-B forbids a renderer in this PR, but the completeness contract must be
//! exercised end to end: every inventoried entry needs a disposition or the
//! audit fails. This module registers the *declared design intent* of the frozen
//! `smusni` profile (DESIGN-RECORD.md + FREEZE-PHASE-B.md + the frozen
//! `*.smusni.txt` sample outputs) as the baseline. It is the spec the future
//! renderer is held to, not the renderer itself; the byte-parity PR will verify
//! actual output against these dispositions.
//!
//! Policy (evidence-grounded where possible):
//! * A field whose *type* is `SemanticSource`/`SourceByteSpan` (every `.source`
//!   link, wherever it nests, plus the two provenance structs' own fields) is
//!   `ExcludedWithReason`: source provenance carries no coordinate in any frozen
//!   `smusni` sample. Excluding by type — not by field name — keeps lexical
//!   `source` fields such as `Connector.source` (a word, not a `SemanticSource`)
//!   rendering.
//! * Exactly one document-level `NOT COMPUTED` fact
//!   (`not-computed:denotation-multiplicity`) is `NotComputedDeclared`
//!   (evidence: the `NOT COMPUTED { denotation-multiplicity; }` block present in
//!   every frozen `smusni` sample). The tested-winner role wordings ALL HOLD /
//!   ROLE FOR are *rendered* wordings (FREEZE-PHASE-B.md (d)), so they RENDER —
//!   they are unexercised by the corpus, not uncomputed.
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
    "source provenance; smusni renders semantic content, not source spans (absent from every frozen smusni sample output)";

/// The single document-level `NOT COMPUTED` fact `smusni` declares rather than
/// computes. (ALL HOLD / ROLE FOR are rendered wordings, not NOT COMPUTED.)
const NOT_COMPUTED_FACT: &str = "not-computed:denotation-multiplicity";

/// Surfaces whose `source` field is a `SemanticSource` provenance link (the 13
/// object kinds carry it via `SemanticObjectCommon`; the rest are value structs).
/// Derived from the model type graph and pinned by the `provenance_is_type_based`
/// test, which fails if the set drifts from the `SemanticSource`-typed fields.
/// `Connector.source` is deliberately absent — its type is `String` (a lexical
/// word), so it renders.
const SOURCE_LINK_SURFACES: &[&str] = &[
    // object kinds (source via SemanticObjectCommon)
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
    // value structs
    "AnchorMagnitude",
    "ArgumentValue",
    "AssignedName",
    "DisplayedContentModifier",
    "ModalArgument",
    "OrdinalLabel",
    "PlaceQuestionBinding",
    "QuantifierBinding",
    "ReciprocalExchange",
    "Recurrence",
    "RelativeClause",
    "Subscript",
];

/// The surfaces whose `source` field is a `SemanticSource` provenance link.
/// Exposed so `provenance_is_type_based` can cross-check it against the model
/// type graph (fails if this hard-coded set drifts from the typed reality).
#[requires(true)]
#[ensures(true)]
pub fn source_link_surfaces() -> &'static [&'static str] {
    SOURCE_LINK_SURFACES
}

/// The exact reason a source-provenance entry is `ExcludedWithReason` under the
/// `smusni` design intent. Exposed so a renderer registering coverage adopts the
/// spec's own stated reason verbatim (the `Disposition` carries the reason, so a
/// coverage contract can only agree with the baseline by reusing this string),
/// mirroring how [`source_link_surfaces`] exposes the exclusion set.
#[requires(true)]
#[ensures(!ret.is_empty())]
pub fn source_provenance_reason() -> &'static str {
    SOURCE_PROVENANCE_REASON
}

/// True for a `SemanticSource`/`SourceByteSpan` value or a `SemanticSource`-typed
/// `source` link on any surface.
#[requires(true)]
#[ensures(ret == (matches!(entry.surface.name, "SemanticSource" | "SourceByteSpan")
    || (entry.field == "source" && SOURCE_LINK_SURFACES.contains(&entry.surface.name))))]
fn is_source_provenance(entry: &InventoryEntry) -> bool {
    matches!(entry.surface.name, "SemanticSource" | "SourceByteSpan")
        || (entry.field == "source" && SOURCE_LINK_SURFACES.contains(&entry.surface.name))
}

/// True for the one declared document-level NOT COMPUTED fact.
#[requires(true)]
#[ensures(ret == (entry.kind == EntryKind::DerivedFact && entry.field == NOT_COMPUTED_FACT))]
fn is_not_computed_fact(entry: &InventoryEntry) -> bool {
    entry.kind == EntryKind::DerivedFact && entry.field == NOT_COMPUTED_FACT
}

/// The baseline disposition for a single entry under the `smusni` design intent.
///
/// The postconditions pin the exact policy so a predicate regression (e.g.
/// re-classifying a rendered wording as NOT COMPUTED, or missing a nested
/// provenance link) is caught mechanically.
#[requires(true)]
#[ensures(matches!(ret.as_data(), data!(Disposition::ExcludedWithReason(_))) == is_source_provenance(entry))]
#[ensures(matches!(ret.as_data(), data!(Disposition::NotComputedDeclared))
    == (is_not_computed_fact(entry) && !is_source_provenance(entry)))]
#[ensures(matches!(ret.as_data(), data!(Disposition::Renders))
    == (!is_source_provenance(entry) && !is_not_computed_fact(entry)))]
pub fn baseline_disposition(entry: &InventoryEntry) -> Disposition {
    if is_source_provenance(entry) {
        return new!(Disposition::ExcludedWithReason(SOURCE_PROVENANCE_REASON));
    }
    if is_not_computed_fact(entry) {
        return new!(Disposition::NotComputedDeclared);
    }
    new!(Disposition::Renders)
}

/// Build the baseline contract: every inventory entry registered with its
/// [`baseline_disposition`]. Complete by construction (verified by the
/// contract-completeness test).
#[requires(true)]
#[ensures(ret.len() == inventory.len())]
pub fn baseline_contract_for(inventory: &RenderFieldInventory) -> CompletenessContract {
    let mut contract = CompletenessContract::new();
    for entry in inventory.entries() {
        // The inventory is unique-by-key, so try_register never fails here; using
        // it (over `register`) documents that the baseline builds each key once.
        contract
            .try_register(entry.key(), baseline_disposition(entry))
            .expect("inventory entries are unique by key");
    }
    contract
}

/// The baseline contract over the full authored inventory.
#[requires(true)]
#[ensures(!ret.is_empty())]
pub fn baseline_contract() -> CompletenessContract {
    baseline_contract_for(&render_field_inventory())
}
