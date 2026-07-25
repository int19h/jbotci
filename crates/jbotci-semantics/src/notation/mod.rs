//! Profile-driven model-facing notation renderer for the
//! `lojban-semantics-json-1` graph (Phase-B steps 3–4 of the tersmu notation
//! program; research repo `DESIGN-RECORD.md` / `FREEZE-PHASE-B.md`).
//!
//! Today one profile is realised, [`NotationProfile::Smusni`] — the Phase-B
//! default-candidate `smusni` profile, a byte-parity port of the frozen Python
//! oracle (`experiments/notation-renderer-v0/render_v5.py` at commit
//! `28c7d5f`). The research repo's internal profile name for this rendering
//! is `lean3` (a historical experiment label); `smusni` is its product name.
//! The [`NotationProfile`] seam is the profile-driven extension
//! point: a future `dense` (or other) profile adds a variant and its own
//! render path without disturbing `smusni` or the existing `render.rs`
//! tree+proj renderer (which this module does not touch).
//!
//! [`coverage`] registers this renderer's field coverage against the merged
//! completeness contract ([`crate::completeness`]); the tests there verify the
//! coverage audits complete and agrees with the declared `smusni` design intent.

pub mod coverage;
mod render;
mod writer;

#[allow(unused_imports)]
use bityzba::{ensures, invariant, requires};

pub use render::SmusniConfig;

use crate::model::SemanticGraph;

/// A notation profile. Only `smusni` exists today (the Phase-B default
/// candidate); the enum is the profile-driven seam future profiles extend.
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
