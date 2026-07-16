use std::sync::Arc;

#[allow(unused_imports)]
use bityzba::{ensures, expensive_invariant, invariant, new, requires};
use jbotci_diagnostics::{Diagnostic, DiagnosticLabel};
use jbotci_morphology::{RecoveredMorphologySegmentation, WordLike};
use jbotci_source::SourceSpan;
use jbotci_syntax::SyntaxRecoveryParse;
use jbotci_web_core::{GentufaWebOptions, analyze_gentufa_source};

use crate::{LineIndex, PositionEncoding, PositionRange};

mod hover;

pub use hover::HoverContent;

/// Immutable recovery-capable analysis of one document version.
#[invariant(words.words.len() == word_spans.len(), "every segmented word has one query span")]
#[invariant(text.len() == line_index.byte_len(), "text and line index byte lengths must agree")]
#[expensive_invariant(text.as_ref() == line_index.text(), "text and line index content must agree")]
#[derive(Debug, Clone)]
pub struct DocumentSnapshot {
    pub text: Arc<str>,
    pub version: i64,
    pub line_index: LineIndex,
    pub words: RecoveredMorphologySegmentation,
    pub parse: SyntaxRecoveryParse,
    pub diagnostics: Vec<Diagnostic>,
    word_spans: Vec<SourceSpan>,
}

impl DocumentSnapshot {
    /// Analyze a complete immutable document with the default Gentufa dialect options.
    #[requires(true)]
    #[ensures(ret.version == version)]
    #[ensures(ret.text.as_ref() == ret.line_index.text())]
    pub fn new(text: String, version: i64) -> Self {
        let analysis = analyze_gentufa_source(&text, &GentufaWebOptions::default())
            .expect("the built-in default dialect must always compile");
        let analysis = analysis.into_data();
        let words = analysis.morphology;
        let parse = analysis.parse;
        let diagnostics = analysis.diagnostics;
        let word_spans = word_spans(&words.words);
        let text: Arc<str> = Arc::from(text);
        let line_index = LineIndex::new(Arc::clone(&text));
        new!(DocumentSnapshot {
            text,
            version,
            line_index,
            words,
            parse,
            diagnostics,
            word_spans,
        })
    }

    /// Iterate diagnostics and lazily resolved labels without per-query allocation.
    #[requires(true)]
    #[ensures(ret.len() == self.diagnostics.len())]
    pub fn diagnostics(
        &self,
        encoding: PositionEncoding,
    ) -> impl ExactSizeIterator<Item = ResolvedDiagnostic<'_>> + '_ {
        self.diagnostics.iter().map(move |diagnostic| {
            new!(ResolvedDiagnostic {
                diagnostic,
                line_index: &self.line_index,
                encoding,
            })
        })
    }

    /// Find the recovered morphology item whose full char span contains `offset`.
    #[requires(true)]
    #[ensures(ret.as_ref().is_none_or(|word| word.span.char_start <= offset && offset < word.span.char_end))]
    pub fn word_at(&self, offset: usize) -> Option<WordAt<'_>> {
        let index = self
            .word_spans
            .partition_point(|span| span.char_end <= offset);
        let span = self.word_spans.get(index)?;
        if offset < span.char_start || span.char_end <= offset {
            return None;
        }
        Some(new!(WordAt {
            word: &self.words.words[index],
            span,
            index,
        }))
    }
}

/// A diagnostic reference whose labels can be viewed in one position encoding.
#[invariant(!diagnostic.labels.is_empty())]
#[derive(Debug, Clone, Copy)]
pub struct ResolvedDiagnostic<'snapshot> {
    pub diagnostic: &'snapshot Diagnostic,
    line_index: &'snapshot LineIndex,
    encoding: PositionEncoding,
}

impl<'snapshot> ResolvedDiagnostic<'snapshot> {
    #[requires(true)]
    #[ensures(ret.len() == self.diagnostic.labels.len())]
    pub fn labels(&self) -> impl ExactSizeIterator<Item = ResolvedLabel<'snapshot>> + '_ {
        self.diagnostic.labels.iter().map(|label| {
            new!(ResolvedLabel {
                label,
                span: &label.span,
                positions: self
                    .line_index
                    .positions_for_span(&label.span, self.encoding),
            })
        })
    }
}

/// One primary or secondary diagnostic label resolved to editor positions.
#[invariant(positions.start <= positions.end)]
#[derive(Debug, Clone, Copy)]
pub struct ResolvedLabel<'snapshot> {
    pub label: &'snapshot DiagnosticLabel,
    pub span: &'snapshot SourceSpan,
    pub positions: PositionRange,
}

/// A recovered morphology item and its full half-open source span.
#[invariant(span.byte_start < span.byte_end && span.char_start < span.char_end)]
#[derive(Debug, Clone, Copy)]
pub struct WordAt<'snapshot> {
    pub word: &'snapshot WordLike,
    pub span: &'snapshot SourceSpan,
    index: usize,
}

#[requires(true)]
#[ensures(ret.len() == words.len())]
fn word_spans(words: &[WordLike]) -> Vec<SourceSpan> {
    let mut result = Vec::with_capacity(words.len());
    let mut component_spans = Vec::new();
    for word in words {
        component_spans.clear();
        word.source_spans_into(&mut component_spans);
        let first = component_spans
            .first()
            .expect("every morphology word-like has at least one source component");
        let last = component_spans
            .last()
            .expect("every morphology word-like has at least one source component");
        result.push(
            SourceSpan::new(
                first.source_id.clone(),
                first.byte_start,
                last.byte_end,
                first.char_start,
                last.char_end,
            )
            .expect("ordered word components must produce an ordered full span"),
        );
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[allow(unused_imports)]
    use bityzba::{ensures, requires};
    use jbotci_web_core::{GentufaWebRequest, GentufaWebResult, parse_gentufa_for_web};

    #[requires(true)]
    #[ensures(true)]
    fn web_diagnostics(source: &str) -> Vec<Diagnostic> {
        let result = parse_gentufa_for_web(&GentufaWebRequest {
            text: source.to_owned(),
            options: GentufaWebOptions::default(),
        });
        match result {
            GentufaWebResult::Blank => Vec::new(),
            GentufaWebResult::Success(success) => success.diagnostics,
            GentufaWebResult::Error(error) => error.diagnostics,
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn snapshot_diagnostics_equal_web_orchestration() {
        for source in ["mi klama", "mi ku i do", "mi @ do"] {
            let snapshot = DocumentSnapshot::new(source.to_owned(), 17);
            assert_eq!(snapshot.diagnostics, web_diagnostics(source), "{source:?}");
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn diagnostics_resolve_primary_and_secondary_labels() {
        let snapshot = DocumentSnapshot::new("mi ku i do".to_owned(), 1);
        let resolved = snapshot
            .diagnostics(PositionEncoding::Utf16)
            .next()
            .expect("recovered syntax must report a diagnostic");
        let labels = resolved.labels().collect::<Vec<_>>();
        assert_eq!(labels.len(), resolved.diagnostic.labels.len());
        assert!(labels.iter().any(|label| label.label.primary));
        assert!(
            labels
                .iter()
                .all(|label| label.positions.start <= label.positions.end)
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn word_at_uses_adjacent_morphology_spans_without_text_scanning() {
        let snapshot = DocumentSnapshot::new("lenu mi klama".to_owned(), -4);
        let le = snapshot.word_at(0).expect("le must cover the first char");
        assert_eq!((le.span.char_start, le.span.char_end), (0, 2));
        assert_eq!(snapshot.word_at(1).map(|word| word.span), Some(le.span));
        let nu = snapshot
            .word_at(2)
            .expect("nu must begin without whitespace");
        assert_eq!((nu.span.char_start, nu.span.char_end), (2, 4));
        assert!(
            snapshot.word_at(4).is_none(),
            "inter-word whitespace is not a word"
        );
        assert_eq!(
            snapshot.version, -4,
            "document versions are opaque signed values"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gismu_hover_snapshots_dictionary_markdown_and_full_span() {
        let hover = DocumentSnapshot::new("klama".to_owned(), 1)
            .hover(0)
            .expect("klama has hover documentation");

        assert_eq!((hover.span.char_start, hover.span.char_end), (0, 5));
        assert_eq!(
            hover.markdown,
            concat!(
                "### `klama`\n\n",
                "*gismu*\n\n",
                "`x1` comes/goes to destination `x2` from origin `x3` via route `x4` ",
                "using means/vehicle `x5`.\n\n",
                "**Glosses:** `come`\n\n",
                "**Rafsi:** `kla`",
            ),
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn lujvo_hover_snapshots_morphology_decomposition_and_component_cards() {
        let hover = DocumentSnapshot::new("sutykla".to_owned(), 1)
            .hover(0)
            .expect("sutykla has hover documentation");

        assert_eq!((hover.span.char_start, hover.span.char_end), (0, 7));
        assert_eq!(
            hover.markdown,
            concat!(
                "**Word type:** lujvo\n\n",
                "**Decomposition:** `sut`·`y`·`kla` → `sutra` + `klama`\n\n",
                "**Component definitions**\n\n---\n\n",
                "### `sutra`\n\n",
                "*gismu*\n\n",
                "`x1` is fast/swift/quick/hastes/rapid at doing/being/bringing about `x2` ",
                "(event/state).\n\n",
                "**Glosses:** `fast`, `quick (fast)`, `rapid`\n\n",
                "**Rafsi:** `sut`\n\n---\n\n",
                "### `klama`\n\n",
                "*gismu*\n\n",
                "`x1` comes/goes to destination `x2` from origin `x3` via route `x4` ",
                "using means/vehicle `x5`.\n\n",
                "**Glosses:** `come`\n\n",
                "**Rafsi:** `kla`",
            ),
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn dictionary_lujvo_hover_prefers_its_own_card() {
        let hover = DocumentSnapshot::new("blari'o".to_owned(), 1)
            .hover(0)
            .expect("blari'o has hover documentation");

        assert!(hover.markdown.starts_with(
            "**Word type:** lujvo\n\n**Decomposition:** `bla`·`ri'o` → `blanu` + `crino`"
        ));
        assert!(hover.markdown.contains("### `blari'o`"));
        assert!(!hover.markdown.contains("**Component definitions**"));
        assert!(!hover.markdown.contains("### `blanu`"));
        assert!(!hover.markdown.contains("### `crino`"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn compact_cmavo_hover_leads_with_constituent_and_appends_compound_card() {
        let snapshot = DocumentSnapshot::new("a'acu'i".to_owned(), 1);
        let first = snapshot.hover(0).expect("a'a has hover documentation");
        let second = snapshot.hover(3).expect("cu'i has hover documentation");

        assert_eq!((first.span.char_start, first.span.char_end), (0, 3));
        assert_eq!((second.span.char_start, second.span.char_end), (3, 7));
        assert_eq!(
            first.markdown,
            concat!(
                "### `a'a`\n\n*cmavo* · **UI1**\n\n",
                "attitudinal: attentive - inattentive - avoiding.\n\n",
                "**Glosses:** `attentive`\n\n---\n\n",
                "## Compact cmavo compound `a'acu'i`\n\n",
                "### `a'acu'i`\n\n*cmavo-compound* · **UI\\*1**\n\n",
                "attitudinal: attentive - inattentive - avoiding.\n\n",
                "**Glosses:** `inattentive`",
            ),
        );
        assert_eq!(
            second.markdown,
            concat!(
                "### `cu'i`\n\n*cmavo* · **CAI**\n\n",
                "attitudinal: neutral scalar attitude modifier.\n\n",
                "**Glosses:** `neutral emotion`\n\n---\n\n",
                "## Compact cmavo compound `a'acu'i`\n\n",
                "### `a'acu'i`\n\n*cmavo-compound* · **UI\\*1**\n\n",
                "attitudinal: attentive - inattentive - avoiding.\n\n",
                "**Glosses:** `inattentive`",
            ),
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn fuhivla_and_cmevla_hovers_follow_dictionary_boundary() {
        let fuhivla = DocumentSnapshot::new("djarspageti".to_owned(), 1)
            .hover(0)
            .expect("dictionary fu'ivla has hover documentation");
        assert_eq!(
            fuhivla.markdown,
            concat!(
                "### `djarspageti`\n\n*fu'ivla*\n\n",
                "`x1` is a quantity of spaghetti (long, thin cylindrical pasta).\n\n",
                "**Glosses:** `spaghetti`",
            ),
        );

        let cmevla = DocumentSnapshot::new(".alis.".to_owned(), 1)
            .hover(1)
            .expect("name word has morphology hover documentation");
        assert_eq!(cmevla.markdown, "**Word type:** name word (cmevla)");
        assert_eq!((cmevla.span.char_start, cmevla.span.char_end), (1, 5));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn unknown_dictionary_word_returns_morphological_classification() {
        let hover = DocumentSnapshot::new("brapu".to_owned(), 1)
            .hover(0)
            .expect("valid unknown gismu still has hover documentation");

        assert_eq!(hover.markdown, "**Word type:** gismu");
        assert_eq!((hover.span.char_start, hover.span.char_end), (0, 5));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn adjacent_lenu_hovers_stay_on_the_segmented_constituent() {
        let snapshot = DocumentSnapshot::new("lenu".to_owned(), 1);
        let le = snapshot.hover(0).expect("le has hover documentation");
        let nu = snapshot.hover(2).expect("nu has hover documentation");

        assert_eq!((le.span.char_start, le.span.char_end), (0, 2));
        assert_eq!((nu.span.char_start, nu.span.char_end), (2, 4));
        assert!(le.markdown.starts_with("### `le`\n"));
        assert!(nu.markdown.starts_with("### `nu`\n"));
        assert!(!le.markdown.contains("Compact cmavo compound"));
        assert!(!nu.markdown.contains("Compact cmavo compound"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn quote_payloads_enforce_semantic_hover_boundaries() {
        for (source, payload_offset) in [("zoi gy klama gy", 8), ("la'o gy klama gy", 9)] {
            let snapshot = DocumentSnapshot::new(source.to_owned(), 1);
            assert!(
                snapshot.word_at(payload_offset).is_some(),
                "the outer morphology quote contains its payload in {source:?}",
            );
            assert!(
                snapshot.hover(payload_offset).is_none(),
                "non-Lojban quote payload has no hover in {source:?}",
            );
        }

        let lohu = DocumentSnapshot::new("lo'u klama le'u".to_owned(), 1)
            .hover(5)
            .expect("LOhU payload has morphology-only hover");
        assert_eq!(lohu.markdown, "**Word type:** gismu");
        assert_eq!((lohu.span.char_start, lohu.span.char_end), (5, 10));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn whitespace_pause_and_erased_words_have_no_hover() {
        let spaced = DocumentSnapshot::new("mi klama".to_owned(), 1);
        assert!(spaced.hover(2).is_none());

        let cmevla = DocumentSnapshot::new(".alis.".to_owned(), 1);
        assert!(cmevla.hover(0).is_none());
        assert!(cmevla.hover(5).is_none());

        let erased = DocumentSnapshot::new("mi si do".to_owned(), 1);
        assert!(erased.hover(0).is_none(), "SI erased its preceding word");
        assert!(erased.hover(3).is_none(), "the SI eraser is not hoverable");
    }
}
