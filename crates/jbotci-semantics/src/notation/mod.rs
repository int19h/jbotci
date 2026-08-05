//! Human-readable projections of the `lojban-semantics-json-1` graph.
//!
//! [`render_smusni`] is the experimental typed S-expression notation. It uses
//! compact semantic forms only when they are proved faithful and otherwise
//! preserves the graph through typed structural fallbacks. [`render_xml`] is
//! the independent canonical SFN-XML renderer.

pub mod coverage;
pub(crate) mod registry;
pub(crate) mod relation_expression;
pub mod sexpr;
pub(crate) mod typed_ir;
pub mod word_cards;
mod xml;
pub mod xml_coverage;
pub(crate) mod xml_words;

#[allow(unused_imports)]
use bityzba::{ensures, invariant, requires};

pub use sexpr::{DocumentMode, SmusniDiagnostic, SmusniRender, SmusniRenderStats};
pub use xml::{
    CompactIncompatibility, XML_DECLARED_WAIVERS, XmlOmission, XmlRender, XmlSurface,
    XmlWaiverFamily, analyze_compact_incompatibilities, render_xml, render_xml_value_for_tooling,
    render_xml_with_word_cards,
};

use self::word_cards::WordCard;
use crate::model::SemanticGraph;

/// A notation profile. The experimental smusni notation has one ordinary
/// profile; source provenance remains available through typed-graph fallback.
#[invariant(::Smusni => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotationProfile {
    Smusni,
}

/// Render a graph under the selected human-readable notation profile.
#[requires(graph.objects.contains_key(&graph.root))]
#[ensures(ret.ends_with('\n') && !ret.ends_with("\n\n"))]
pub fn render_notation(graph: &SemanticGraph, profile: NotationProfile) -> String {
    match profile {
        NotationProfile::Smusni => render_smusni(graph),
    }
}

/// Render one experimental typed S-expression document.
#[requires(graph.objects.contains_key(&graph.root))]
#[ensures(ret.ends_with('\n') && !ret.ends_with("\n\n"))]
pub fn render_smusni(graph: &SemanticGraph) -> String {
    render_smusni_detailed(graph, &[]).into_data().text
}

/// Render one typed S-expression document with structured word cards.
#[requires(graph.objects.contains_key(&graph.root))]
#[ensures(ret.ends_with('\n') && !ret.ends_with("\n\n"))]
pub fn render_smusni_with_word_cards(graph: &SemanticGraph, cards: &[WordCard]) -> String {
    render_smusni_detailed(graph, cards).into_data().text
}

/// Render a document and retain non-golden measurements for corpus auditing.
#[requires(graph.objects.contains_key(&graph.root))]
#[ensures(ret.text.ends_with('\n') && !ret.text.ends_with("\n\n"))]
pub fn render_smusni_detailed(graph: &SemanticGraph, cards: &[WordCard]) -> SmusniRender {
    let card_data = cards
        .iter()
        .filter_map(sexpr::word_card_datum)
        .collect::<Vec<_>>();
    sexpr::render_document(graph, &card_data)
}
