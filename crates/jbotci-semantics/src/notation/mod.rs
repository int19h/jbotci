//! Human-readable projections of the `lojban-semantics-json-1` graph.
//!
//! [`render_smusni`] is the experimental typed S-expression notation. It uses
//! compact semantic forms only when they are proved faithful and otherwise
//! preserves the graph through typed structural fallbacks. [`render_xml`] is
//! the independent canonical SFN-XML renderer.

pub mod coverage;
pub mod kernel;
pub(crate) mod lexical_edge;
pub(crate) mod registry;
pub(crate) mod relation_expression;
pub mod sexpr;
pub mod word_cards;
mod xml;
pub mod xml_coverage;
pub(crate) mod xml_words;

#[allow(unused_imports)]
use bityzba::{ensures, invariant, requires};

pub use sexpr::{
    FailureSpanSource, SmusniDiagnostic, SmusniProjection, SmusniProjectionFailed,
    SmusniProjectionFailure, SmusniRender, SmusniRenderStats,
};
pub use xml::{
    CompactIncompatibility, XML_DECLARED_WAIVERS, XmlOmission, XmlRender, XmlSurface,
    XmlWaiverFamily, analyze_compact_incompatibilities, render_xml, render_xml_value_for_tooling,
    render_xml_with_word_cards,
};

use self::word_cards::WordCard;
use crate::model::SemanticGraph;

/// A notation profile. The experimental smusni notation has one ordinary
/// profile.
#[invariant(::Smusni => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotationProfile {
    Smusni,
}

/// Project a graph under the selected human-readable notation profile.
#[requires(graph.objects.contains_key(&graph.root))]
#[ensures(ret.as_ref().is_ok_and(|render| render.text.ends_with('\n')) || ret.is_err())]
pub fn render_notation(graph: &SemanticGraph, profile: NotationProfile) -> SmusniProjection {
    match profile {
        NotationProfile::Smusni => render_smusni(graph),
    }
}

/// Project one experimental typed S-expression document.
#[requires(graph.objects.contains_key(&graph.root))]
#[ensures(ret.as_ref().is_ok_and(|render| render.text.ends_with('\n')) || ret.is_err())]
pub fn render_smusni(graph: &SemanticGraph) -> SmusniProjection {
    render_smusni_with_word_cards(graph, &[])
}

/// Project one typed S-expression document with structured word cards.
///
/// The cards reach the document only on the success path, so a failed
/// projection never carries a `Words` section.
#[requires(graph.objects.contains_key(&graph.root))]
#[ensures(ret.as_ref().is_ok_and(|render| render.text.ends_with('\n')) || ret.is_err())]
pub fn render_smusni_with_word_cards(
    graph: &SemanticGraph,
    cards: &[WordCard],
) -> SmusniProjection {
    let card_data = cards
        .iter()
        .filter_map(sexpr::word_card_datum)
        .collect::<Vec<_>>();
    sexpr::render_document(graph, &card_data)
}
