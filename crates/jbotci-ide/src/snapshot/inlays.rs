use std::cell::RefCell;

#[allow(unused_imports)]
use bityzba::{data, ensures, invariant, new, requires};
use jbotci_morphology::{
    NodeRef as MorphologyNodeRef, TreeNode as MorphologyTreeNode, Word, fold_lojban_diacritic,
};
use jbotci_source::SourceSpan;
use jbotci_tree::TreeVisitor;

use super::{
    DocumentSnapshot, StructureInlayKind,
    structure_inlays::{DecorationProfile, anchor_is_in_range},
};
use crate::LineIndex;

/// Enabled inlay kinds and kind-specific rendering options.
#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InlayOptions {
    pub structure_brackets: StructureBracketInlayOptions,
    pub word_boundaries: bool,
    pub rafsi_boundaries: bool,
}

impl Default for InlayOptions {
    #[requires(true)]
    #[ensures(ret.structure_brackets.enabled)]
    #[ensures(!ret.word_boundaries && !ret.rafsi_boundaries)]
    fn default() -> Self {
        Self {
            // Preserve the structure-inlay behavior shipped before the
            // kind-keyed configuration surface. The new word-stream kinds are
            // opt-in so the schema migration does not change existing output.
            structure_brackets: StructureBracketInlayOptions::default(),
            word_boundaries: false,
            rafsi_boundaries: false,
        }
    }
}

/// Structure-bracket enablement plus its decoration-profile configuration.
#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructureBracketInlayOptions {
    pub enabled: bool,
    pub profile: DecorationProfile,
}

impl Default for StructureBracketInlayOptions {
    #[requires(true)]
    #[ensures(ret.enabled)]
    fn default() -> Self {
        Self {
            enabled: true,
            profile: DecorationProfile::default(),
        }
    }
}

/// The source feature that produced an inlay.
#[invariant(true)]
#[invariant(::Structure { .. } => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InlayKind {
    Structure { boundary: StructureInlayKind },
    WordBoundary,
    RafsiBoundary,
}

/// One transport-independent inlay anchored to a zero-width source span.
#[invariant(anchor.is_empty(), "inlays must have zero-width anchors")]
#[invariant(!label.is_empty(), "inlays must have visible labels")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Inlay {
    pub anchor: SourceSpan,
    pub label: String,
    pub kind: InlayKind,
}

impl DocumentSnapshot {
    /// Return enabled inlays whose anchor positions lie in `range`.
    ///
    /// Structure brackets use recovered syntax fragments. Word and rafsi
    /// boundaries use only the recovered morphology word stream, so sparse or
    /// degenerate syntax recovery cannot suppress them.
    #[requires(range.byte_end <= self.text.len())]
    #[requires(range.char_end <= self.line_index.char_len())]
    #[ensures(ret.windows(2).all(|inlays| inlays[0].anchor.byte_start <= inlays[1].anchor.byte_start))]
    #[ensures(ret.iter().all(|inlay| range.byte_start <= inlay.anchor.byte_start && inlay.anchor.byte_start < range.byte_end))]
    pub fn inlays(&self, options: &InlayOptions, range: &SourceSpan) -> Vec<Inlay> {
        let mut inlays = Vec::new();
        if options.structure_brackets.enabled {
            inlays.extend(
                self.structure_inlays(&options.structure_brackets.profile, range)
                    .into_iter()
                    .map(|inlay| {
                        let inlay = inlay.into_data();
                        new!(Inlay {
                            anchor: inlay.anchor,
                            label: inlay.label,
                            kind: InlayKind::Structure {
                                boundary: inlay.kind,
                            },
                        })
                    }),
            );
        }
        if options.word_boundaries {
            append_word_boundary_inlays(self, range, &mut inlays);
        }
        if options.rafsi_boundaries {
            append_rafsi_boundary_inlays(self, range, &mut inlays);
        }
        // Stable source ordering preserves the structure engine's deliberate
        // outer/inner ordering when multiple brackets share one anchor.
        inlays.sort_by_key(|inlay| inlay.anchor.byte_start);
        inlays
    }
}

#[requires(query_range.byte_end <= snapshot.text.len())]
#[ensures(inlays.len() >= old(inlays.len()))]
fn append_word_boundary_inlays(
    snapshot: &DocumentSnapshot,
    query_range: &SourceSpan,
    inlays: &mut Vec<Inlay>,
) {
    for words in snapshot.word_spans.windows(2) {
        let left = &words[0];
        let right = &words[1];
        // Span adjacency is the complete definition. In particular, do not
        // duplicate morphology here with a separator or character-class list.
        if left.char_end == right.char_start && anchor_is_in_range(left.byte_end, query_range) {
            inlays.push(inlay_at_byte(
                left.byte_end,
                "-",
                InlayKind::WordBoundary,
                &snapshot.line_index,
            ));
        }
    }
}

#[requires(query_range.byte_end <= snapshot.text.len())]
#[ensures(inlays.len() >= old(inlays.len()))]
fn append_rafsi_boundary_inlays(
    snapshot: &DocumentSnapshot,
    query_range: &SourceSpan,
    inlays: &mut Vec<Inlay>,
) {
    let mut collector = new!(RafsiBoundaryInlayCollector {
        source: snapshot.text.as_ref(),
        query_range,
        line_index: &snapshot.line_index,
        inlays: RefCell::new(inlays),
    });
    for word_like in &snapshot.words.words {
        word_like.visit_in_order(&mut collector);
    }
}

#[invariant(query_range.byte_end <= source.len())]
#[invariant(source.len() == line_index.byte_len())]
struct RafsiBoundaryInlayCollector<'snapshot, 'output> {
    source: &'snapshot str,
    query_range: &'snapshot SourceSpan,
    line_index: &'snapshot LineIndex,
    inlays: RefCell<&'output mut Vec<Inlay>>,
}

impl<'tree> TreeVisitor<'tree> for RafsiBoundaryInlayCollector<'_, '_> {
    type Node = MorphologyNodeRef<'tree>;
    type Atom = jbotci_morphology::AtomRef<'tree>;

    #[requires(true)]
    #[ensures(true)]
    fn enter_node(&mut self, node: Self::Node) {
        if let MorphologyNodeRef::WordLujvo(word) = node {
            let mut inlays = self.inlays.borrow_mut();
            append_lujvo_inlays(
                &mut **inlays,
                word,
                self.source,
                self.query_range,
                self.line_index,
            );
        }
    }
}

#[requires(word.span().byte_end <= source.len())]
#[requires(query_range.byte_end <= source.len())]
#[ensures(inlays.len() >= old(inlays.len()))]
fn append_lujvo_inlays(
    inlays: &mut Vec<Inlay>,
    word: &Word,
    source: &str,
    query_range: &SourceSpan,
    line_index: &LineIndex,
) {
    let Some(parts) = word.lujvo_parts() else {
        return;
    };
    if parts.len() < 2 {
        return;
    }
    let span = word.span();
    let surface = &source[span.byte_start..span.byte_end];
    let canonical_surface_len = surface
        .chars()
        .filter(|value| *value != ',')
        .filter_map(fold_lojban_diacritic)
        .count();
    let part_phoneme_len = parts
        .iter()
        .map(|part| part.phonemes().as_str().chars().count())
        .sum::<usize>();
    if canonical_surface_len != part_phoneme_len {
        // A dialect can map one source word to a differently sized canonical
        // lujvo. Without a source map there is no exact internal anchor, so do
        // not invent one by proportional placement or reparsing raw text.
        return;
    }

    let mut boundary_parts = parts.iter().take(parts.len() - 1);
    let mut next_boundary = boundary_parts
        .next()
        .map(|part| part.phonemes().as_str().chars().count());
    let mut canonical_offset = 0_usize;
    let mut char_offset = span.char_start;
    let mut pending_boundary = false;

    for (relative_byte, value) in surface.char_indices() {
        let contributes_phoneme = value != ',' && fold_lojban_diacritic(value).is_some();
        if contributes_phoneme {
            if pending_boundary {
                let byte_offset = span.byte_start + relative_byte;
                if anchor_is_in_range(byte_offset, query_range) {
                    inlays.push(inlay_at_byte(
                        byte_offset,
                        "·",
                        InlayKind::RafsiBoundary,
                        line_index,
                    ));
                }
                pending_boundary = false;
            }
            canonical_offset += 1;
            char_offset += 1;
            if next_boundary == Some(canonical_offset) {
                pending_boundary = true;
                next_boundary = boundary_parts
                    .next()
                    .map(|part| canonical_offset + part.phonemes().as_str().chars().count());
            }
        } else {
            char_offset += 1;
        }
    }

    assert_eq!(canonical_offset, canonical_surface_len);
    assert_eq!(char_offset, span.char_end);
    assert!(!pending_boundary);
    assert!(next_boundary.is_none());
}

#[requires(byte_offset <= line_index.byte_len())]
#[requires(!label.is_empty())]
#[ensures(ret.anchor.byte_start == byte_offset)]
fn inlay_at_byte(
    byte_offset: usize,
    label: &str,
    kind: InlayKind,
    line_index: &LineIndex,
) -> Inlay {
    let offsets = line_index.offsets_for_byte(byte_offset);
    assert_eq!(
        offsets.byte, byte_offset,
        "morphology-derived inlays must anchor on Unicode scalar boundaries",
    );
    let anchor = SourceSpan::new(None, byte_offset, byte_offset, offsets.char, offsets.char)
        .expect("equal source offsets are ordered");
    new!(Inlay {
        anchor,
        label: label.to_owned(),
        kind,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[allow(unused_imports)]
    use jbotci_syntax::{SyntaxRecoveryParse, SyntaxRecoveryParseData};

    #[requires(true)]
    #[ensures(ret.byte_end == snapshot.text.len())]
    fn whole_document_span(snapshot: &DocumentSnapshot) -> SourceSpan {
        SourceSpan::new(
            None,
            0,
            snapshot.text.len(),
            0,
            snapshot.line_index.char_len(),
        )
        .expect("whole-document offsets are ordered")
    }

    #[requires(true)]
    #[ensures(!ret.structure_brackets.enabled)]
    fn word_stream_options(word_boundaries: bool, rafsi_boundaries: bool) -> InlayOptions {
        InlayOptions {
            structure_brackets: StructureBracketInlayOptions {
                enabled: false,
                profile: DecorationProfile::default(),
            },
            word_boundaries,
            rafsi_boundaries,
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn solid_cmavo_run_has_only_zero_gap_word_boundaries() {
        let solid = DocumentSnapshot::new("uanaisaidai".to_owned(), 1);
        let inlays = solid.inlays(
            &word_stream_options(true, false),
            &whole_document_span(&solid),
        );
        assert_eq!(
            inlays
                .iter()
                .map(|inlay| (inlay.anchor.char_start, inlay.label.as_str()))
                .collect::<Vec<_>>(),
            vec![(2, "-"), (5, "-"), (8, "-")],
        );

        let spaced = DocumentSnapshot::new("ua nai sai dai".to_owned(), 1);
        assert!(
            spaced
                .inlays(
                    &word_stream_options(true, false),
                    &whole_document_span(&spaced),
                )
                .is_empty(),
            "non-adjacent morphology spans must not produce boundary hints",
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn lujvo_parts_include_hyphens_as_independent_boundaries() {
        let snapshot = DocumentSnapshot::new("lenkymipri".to_owned(), 1);
        let inlays = snapshot.inlays(
            &word_stream_options(false, true),
            &whole_document_span(&snapshot),
        );
        assert_eq!(
            inlays
                .iter()
                .map(|inlay| (inlay.anchor.char_start, inlay.label.as_str()))
                .collect::<Vec<_>>(),
            vec![(4, "·"), (5, "·")],
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn word_stream_kinds_toggle_and_scope_independently() {
        let snapshot = DocumentSnapshot::new("uanaisaidai lenkymipri".to_owned(), 1);
        let whole = whole_document_span(&snapshot);
        let word_only = snapshot.inlays(&word_stream_options(true, false), &whole);
        let rafsi_only = snapshot.inlays(&word_stream_options(false, true), &whole);
        let both = snapshot.inlays(&word_stream_options(true, true), &whole);
        assert_eq!(word_only.len(), 3);
        assert!(
            word_only
                .iter()
                .all(|inlay| inlay.kind == InlayKind::WordBoundary)
        );
        assert_eq!(rafsi_only.len(), 2);
        assert!(
            rafsi_only
                .iter()
                .all(|inlay| inlay.kind == InlayKind::RafsiBoundary)
        );
        assert_eq!(both.len(), word_only.len() + rafsi_only.len());

        let range = SourceSpan::new(
            None,
            5,
            snapshot.text.len(),
            5,
            snapshot.line_index.char_len(),
        )
        .expect("ordered subset");
        assert_eq!(
            snapshot
                .inlays(&word_stream_options(true, true), &range)
                .iter()
                .map(|inlay| inlay.anchor.char_start)
                .collect::<Vec<_>>(),
            vec![5, 8, 16, 17],
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn word_stream_inlays_survive_degenerate_syntax_recovery() {
        let snapshot = DocumentSnapshot::new("mi ku i uanaisaidai lenkymipri".to_owned(), 1);
        assert!(matches!(
            snapshot.parse.as_data(),
            data!(SyntaxRecoveryParse::Recovered { .. })
        ));
        let inlays = snapshot.inlays(
            &word_stream_options(true, true),
            &whole_document_span(&snapshot),
        );
        assert_eq!(
            inlays
                .iter()
                .filter(|inlay| inlay.kind == InlayKind::WordBoundary)
                .count(),
            3,
        );
        assert_eq!(
            inlays
                .iter()
                .filter(|inlay| inlay.kind == InlayKind::RafsiBoundary)
                .count(),
            2,
        );
    }
}
