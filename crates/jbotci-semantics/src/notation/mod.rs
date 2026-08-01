//! Profile-driven model-facing notation renderer for the
//! `lojban-semantics-json-1` graph (Phase-B steps 3–4 of the tersmu notation
//! program; research repo `DESIGN-RECORD.md` / `FREEZE-PHASE-B.md`).
//!
//! Two product renderings are realised. [`NotationProfile::Smusni`] is the
//! `smusni` profile, a byte-parity port of the frozen Python oracle
//! (`experiments/notation-renderer-v0/render_v5.py` at commit `28c7d5f`).
//! `smusni` is now the default tersmu output format everywhere (CLI and MCP);
//! the legacy `tree`/`tree+proj` renderers it replaced have been removed. The
//! research repo's internal profile name for this rendering is `lean3` (a
//! historical experiment label); `smusni` is its product name.
//! [`render_xml`] is the canonical SFN-XML renderer adopted from
//! `render_xml.py` at research commit `e25eeaf`, the original adoption point;
//! [`xml`] and the corpus provenance pin the current paired prototype oracle.
//! Its separate result type also
//! returns occurrence-level omissions. [`NotationProfile`] remains the
//! string-only profile seam, while XML uses its richer explicit entry point.
//!
//! [`coverage`] and [`xml_coverage`] register both renderers against the merged
//! completeness inventory ([`crate::completeness`]).
//!
//! ## QUESTION record design (#622)
//!
//! - A question object is declared as `QUESTION quN { ... }`, following the
//!   existing uppercase-keyword + typed-short-ID + braces declaration shape.
//!   `qu` is the shortest question-derived prefix that does not collide with the
//!   existing `q=quantity` prefix, and it is added to `ID PREFIXES` from the same
//!   prefix table that assigns IDs.
//! - Fields follow the graph order `BODY`, `KIND`, `MODE`, `ASKER`, `RESPONDENT`,
//!   `DOMAIN`, `SLOTS`, then optional `FOCUS` and `PRESUPPOSED ANSWER`. Graph
//!   edges use typed-short IDs; `KIND`, `MODE`, and slot `ROLE`/`KIND` use the
//!   established uppercase enum vocabulary. `DOMAIN` uses the title-cased
//!   semantic-sort vocabulary already used by `PARAMETER SORT`.
//! - `SLOTS` is an ordered sequence whose one-based entries use `[N]:`, matching
//!   `ARGS`. Each entry is a field-labelled record: `PARAMETER` when present,
//!   `ROLE`, and, for heterogeneous slots, `KIND` plus `DOMAIN`. This preserves
//!   slot order and renders every `QuestionSlot` shape without inventing a new
//!   wording class.
//! - A predication's `PLACE QUESTIONS` is likewise an ordered `[N]:` sequence.
//!   Each binding renders `PARAMETER`, its `ARGUMENT` using the existing
//!   operand-site `VALUE`/`REFERENCE DENOTATION` convention, and ordered
//!   `CANDIDATE PLACES` entries written as bracketed place keys. This replaces
//!   the former `NOT COMPUTED: place-questions;` marker.
//! - QUESTION declarations use the existing dense-declaration rule unchanged:
//!   at most four direct fields collapse to one line; larger records remain
//!   multiline. Optional provenance continues to use the existing nested
//!   `PROVENANCE` block.
//!
//! Open for PM review: `qu` is the first multi-letter typed-short prefix, chosen
//! by the existing shortest-unambiguous rule; `DOMAIN` is title-cased rather
//! than uppercased because it is a semantic sort, not a closed enum wording.

pub mod coverage;
mod render;
pub mod word_cards;
mod writer;
mod xml;
pub mod xml_coverage;

#[allow(unused_imports)]
use bityzba::{ensures, invariant, requires};

pub use render::SmusniConfig;
pub use xml::{
    XML_DECLARED_WAIVERS, XmlOmission, XmlRender, XmlSurface, XmlWaiverFamily, render_xml,
};

use crate::model::SemanticGraph;

/// A notation profile. Only `smusni` exists today (the default tersmu output
/// format); the enum is the profile-driven seam future profiles extend.
// `#[invariant(::Smusni(_) => true)]`: an audited no-op — the wrapped
// `SmusniConfig` validates its own (trivial) domain, so every `Smusni` value is a
// valid profile selection.
#[invariant(::Smusni(_) => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotationProfile {
    /// The frozen `smusni` profile (DESIGN-RECORD.md, Phase-A close), carrying
    /// its one runtime toggle (provenance on/off).
    Smusni(SmusniConfig),
}

/// Render `graph` under `profile`, producing the model-facing notation text
/// (terminated by a single trailing newline). Requires a valid this-build
/// `SemanticGraph` (see [`render_smusni`]).
#[requires(graph.objects.contains_key(&graph.root))]
#[ensures(ret.ends_with('\n'))]
pub fn render_notation(graph: &SemanticGraph, profile: NotationProfile) -> String {
    match profile {
        NotationProfile::Smusni(config) => render::render_smusni(graph, config),
    }
}

/// Convenience entry for the `smusni` profile. Requires a valid this-build
/// `SemanticGraph`: its type invariants guarantee referential integrity and
/// required-field population, which the renderer relies on (failing loudly, not
/// degrading, if ever violated). See [`render::render_smusni`]'s contract.
#[requires(graph.objects.contains_key(&graph.root))]
#[ensures(ret.ends_with('\n'))]
pub fn render_smusni(graph: &SemanticGraph, config: SmusniConfig) -> String {
    render::render_smusni(graph, config)
}
