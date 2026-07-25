//! Profile-driven model-facing notation renderer for the
//! `lojban-semantics-json-1` graph (Phase-B steps 3–4 of the tersmu notation
//! program; research repo `DESIGN-RECORD.md` / `FREEZE-PHASE-B.md`).
//!
//! Today one profile is realised, [`NotationProfile::Lean3`] — the Phase-B
//! default-candidate `lean3` profile, a byte-parity port of the frozen Python
//! oracle (`experiments/notation-renderer-v0/render_v5.py` at commit
//! `cab176bcce`). The [`NotationProfile`] seam is the profile-driven extension
//! point: a future `dense` (or other) profile adds a variant and its own
//! render path without disturbing `lean3` or the existing `render.rs`
//! tree+proj renderer (which this module does not touch).
//!
//! [`coverage`] registers this renderer's field coverage against the merged
//! completeness contract ([`crate::completeness`]); the tests there verify the
//! coverage audits complete and agrees with the declared `lean3` design intent.

pub mod coverage;
mod render;
mod writer;

#[allow(unused_imports)]
use bityzba::{ensures, invariant, requires};

pub use render::Lean3Config;

use crate::model::SemanticGraph;

/// A notation profile. Only `lean3` exists today (the Phase-B default
/// candidate); the enum is the profile-driven seam future profiles extend.
// `#[invariant(::Lean3(_) => true)]`: an audited no-op — the wrapped
// `Lean3Config` validates its own (trivial) domain, so every `Lean3` value is a
// valid profile selection.
#[invariant(::Lean3(_) => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotationProfile {
    /// The frozen `lean3` profile (DESIGN-RECORD.md, Phase-A close), carrying
    /// its one runtime toggle (provenance on/off).
    Lean3(Lean3Config),
}

/// Render `graph` under `profile`, producing the model-facing notation text
/// (terminated by a single trailing newline). Requires a valid this-build
/// `SemanticGraph` (see [`render_lean3`]).
#[requires(graph.objects.contains_key(&graph.root))]
#[ensures(ret.ends_with('\n'))]
pub fn render_notation(graph: &SemanticGraph, profile: NotationProfile) -> String {
    match profile {
        NotationProfile::Lean3(config) => render::render_lean3(graph, config),
    }
}

/// Convenience entry for the `lean3` profile. Requires a valid this-build
/// `SemanticGraph`: its type invariants guarantee referential integrity and
/// required-field population, which the renderer relies on (failing loudly, not
/// degrading, if ever violated). See [`render::render_lean3`]'s contract.
#[requires(graph.objects.contains_key(&graph.root))]
#[ensures(ret.ends_with('\n'))]
pub fn render_lean3(graph: &SemanticGraph, config: Lean3Config) -> String {
    render::render_lean3(graph, config)
}
