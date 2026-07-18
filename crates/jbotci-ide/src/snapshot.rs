use std::sync::Arc;

#[allow(unused_imports)]
use bityzba::{ensures, expensive_invariant, invariant, new, requires};
use jbotci_diagnostics::{Diagnostic, DiagnosticLabel};
use jbotci_morphology::{RecoveredMorphologySegmentation, WordLike};
use jbotci_source::SourceSpan;
use jbotci_syntax::SyntaxRecoveryParse;
use jbotci_web_core::{
    GentufaSourceAnalysis, GentufaWebOptions, analyze_gentufa_morphology_source,
    complete_gentufa_source_analysis,
};

use crate::{LineIndex, PositionEncoding, PositionRange};

mod completion;
mod hover;
mod incremental_diagnostics;
mod semantic_tokens;
mod structure_inlays;

pub use completion::{
    CompletionCancellationToken, CompletionDocumentationHandle, CompletionInterpretation,
    CompletionItem, CompletionKind, CompletionProvenance, completion_documentation_markdown,
};
pub use hover::HoverContent;
pub use incremental_diagnostics::{
    DiagnosticSnapshot, IncrementalAnalysisTimings, IncrementalDiagnosticGate,
    PreparedDocumentAnalysis,
};
pub use semantic_tokens::{SemanticToken, SemanticTokenKind};
use structure_inlays::{DecorationFragment, build_decoration_fragments};
pub use structure_inlays::{
    DecorationProfile, RawBracketsOptions, StructureConstructFilter, StructureInlay,
    StructureInlayKind,
};

/// Immutable recovery-capable analysis of one document version.
#[invariant(words.words.len() == word_spans.len(), "every segmented word has one query span")]
#[invariant(text.len() == line_index.byte_len(), "text and line index byte lengths must agree")]
#[expensive_invariant(text.as_ref() == line_index.text(), "text and line index content must agree")]
#[expensive_invariant(semantic_tokens.windows(2).all(|tokens| tokens[0].span.char_end <= tokens[1].span.char_start), "semantic tokens must be in source order and non-overlapping")]
#[expensive_invariant(structure_fragments.iter().all(|fragment| fragment.ranges_are_within(text.len())), "structure fragments must stay within the snapshot source")]
#[derive(Debug, Clone)]
pub struct DocumentSnapshot {
    pub text: Arc<str>,
    pub version: i64,
    pub line_index: LineIndex,
    pub words: RecoveredMorphologySegmentation,
    pub parse: SyntaxRecoveryParse,
    pub diagnostics: Vec<Diagnostic>,
    word_spans: Vec<SourceSpan>,
    semantic_tokens: Vec<SemanticToken>,
    structure_fragments: Vec<DecorationFragment>,
}

impl DocumentSnapshot {
    /// Analyze a complete immutable document with the default Gentufa dialect options.
    #[requires(true)]
    #[ensures(ret.version == version)]
    #[ensures(ret.text.as_ref() == ret.line_index.text())]
    pub fn new(text: String, version: i64) -> Self {
        let options = GentufaWebOptions::default();
        let morphology = analyze_gentufa_morphology_source(&text, &options)
            .expect("the built-in default dialect must always compile");
        let analysis = complete_gentufa_source_analysis(&text, &options, morphology);
        Self::from_analysis(Arc::from(text), version, analysis)
    }

    #[requires(true)]
    #[ensures(ret.version == version)]
    #[ensures(ret.text.as_ref() == ret.line_index.text())]
    pub(super) fn from_analysis(
        text: Arc<str>,
        version: i64,
        analysis: GentufaSourceAnalysis,
    ) -> Self {
        let analysis = analysis.into_data();
        let words = analysis.morphology;
        let parse = analysis.parse;
        let diagnostics = analysis.diagnostics;
        let word_spans = word_spans(&words.words);
        let semantic_tokens = semantic_tokens::build_semantic_tokens(&words.words, &word_spans);
        let structure_fragments = build_decoration_fragments(&parse, &text);
        let line_index = LineIndex::new(Arc::clone(&text));
        new!(DocumentSnapshot {
            text,
            version,
            line_index,
            words,
            parse,
            diagnostics,
            word_spans,
            semantic_tokens,
            structure_fragments,
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

    /// Iterate morphology-derived semantic tokens in source order.
    ///
    /// Token spans stay in source coordinates so transport adapters can resolve
    /// them through [`LineIndex`] using their negotiated position encoding.
    #[requires(true)]
    #[ensures(ret.len() == self.semantic_tokens.len())]
    pub fn semantic_tokens(&self) -> impl ExactSizeIterator<Item = &SemanticToken> {
        self.semantic_tokens.iter()
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
    fn semantic_token_fixtures_cover_the_complete_legend() {
        let fixtures: &[(&str, &[SemanticTokenKind])] = &[
            ("klama", &[SemanticTokenKind::Gismu]),
            ("lojybau", &[SemanticTokenKind::Lujvo]),
            ("djarspageti", &[SemanticTokenKind::Fuhivla]),
            (".alis.", &[SemanticTokenKind::Cmevla]),
            ("mi", &[SemanticTokenKind::SumtiWord]),
            ("go'i", &[SemanticTokenKind::SelbriWord]),
            ("je", &[SemanticTokenKind::Connective]),
            ("ku", &[SemanticTokenKind::Terminator]),
            (
                "zo klama",
                &[
                    SemanticTokenKind::QuotationMarker,
                    SemanticTokenKind::String,
                ],
            ),
            ("pa", &[SemanticTokenKind::Number]),
            ("by", &[SemanticTokenKind::Letteral]),
            ("ui", &[SemanticTokenKind::Attitudinal]),
            ("pu", &[SemanticTokenKind::TenseModal]),
            ("cu", &[SemanticTokenKind::Cmavo]),
            (
                "zoi gy non-Lojban text gy",
                &[
                    SemanticTokenKind::QuotationMarker,
                    SemanticTokenKind::QuotationMarker,
                    SemanticTokenKind::String,
                    SemanticTokenKind::QuotationMarker,
                ],
            ),
        ];

        let mut covered = Vec::new();
        for (source, expected) in fixtures {
            let snapshot = DocumentSnapshot::new((*source).to_owned(), 1);
            let actual = snapshot
                .semantic_tokens()
                .map(|token| token.kind)
                .collect::<Vec<_>>();
            assert_eq!(actual, *expected, "{source:?}");
            for kind in actual {
                if !covered.contains(&kind) {
                    covered.push(kind);
                }
            }
        }
        assert!(
            SemanticTokenKind::ALL
                .iter()
                .all(|kind| covered.contains(kind)),
            "fixtures must exercise every advertised token kind: {covered:?}",
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn semantic_tokens_preserve_spaceless_boundaries_and_skip_erased_words() {
        let snapshot = DocumentSnapshot::new("lenu".to_owned(), 1);
        let tokens = snapshot.semantic_tokens().collect::<Vec<_>>();
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].kind, SemanticTokenKind::SumtiWord);
        assert_eq!((tokens[0].span.char_start, tokens[0].span.char_end), (0, 2),);
        assert_eq!(tokens[1].kind, SemanticTokenKind::SelbriWord);
        assert_eq!((tokens[1].span.char_start, tokens[1].span.char_end), (2, 4),);

        let erased = DocumentSnapshot::new("mi si do".to_owned(), 1);
        let tokens = erased.semantic_tokens().collect::<Vec<_>>();
        assert_eq!(tokens.len(), 1, "SI and the word it erases stay unstyled");
        assert_eq!(tokens[0].kind, SemanticTokenKind::SumtiWord);
        assert_eq!((tokens[0].span.char_start, tokens[0].span.char_end), (6, 8),);
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
                "### `klama` — *gismu*\n\n",
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
                "### `sutykla` — *lujvo*\n\n",
                "**Decomposition:** `sut`·`y`·`kla` → `sutra` + `klama`\n\n",
                "---\n\n",
                "### `sutra` — *gismu*\n\n",
                "`x1` is fast/swift/quick/hastes/rapid at doing/being/bringing about `x2` ",
                "(event/state).\n\n",
                "**Glosses:** `fast`, `quick (fast)`, `rapid`\n\n",
                "**Rafsi:** `sut`\n\n---\n\n",
                "### `klama` — *gismu*\n\n",
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
            "### `blari'o` — *lujvo*\n\n**Decomposition:** `bla`·`ri'o` → `blanu` + `crino`"
        ));
        assert!(!hover.markdown.contains("**Component definitions**"));
        assert!(!hover.markdown.contains("### `blanu`"));
        assert!(!hover.markdown.contains("### `crino`"));
        assert!(!hover.markdown.contains("**Word type:**"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn cmavo_sequence_hover_replaces_constituents_and_uses_the_full_span() {
        let snapshot = DocumentSnapshot::new("a'acu'i".to_owned(), 1);
        let first = snapshot.hover(0).expect("a'a has hover documentation");
        let second = snapshot.hover(3).expect("cu'i has hover documentation");

        assert_eq!((first.span.char_start, first.span.char_end), (0, 7));
        assert_eq!((second.span.char_start, second.span.char_end), (0, 7));
        assert_eq!(
            first.markdown,
            concat!(
                "### `a'acu'i` — *cmavo sequence* · **UI\\*1**\n\n",
                "attitudinal: attentive - inattentive - avoiding.\n\n",
                "**Glosses:** `inattentive`",
            ),
        );
        assert_eq!(second.markdown, first.markdown);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn unattested_zei_compound_hover_stacks_component_cards() {
        let snapshot = DocumentSnapshot::new("gleki zei py.".to_owned(), 1);
        let expected = concat!(
            "### `gleki zei py` — *ZEI compound*\n\n",
            "---\n\n",
            "### `gleki` — *gismu*\n\n",
            "`x1` is happy/merry/glad/gleeful about `x2` (event/state).\n\n",
            "**Glosses:** `happy`\n\n",
            "**Rafsi:** `gek`, `gei`\n\n",
            "---\n\n",
            "### `py` — *cmavo* · **BY2**\n\n",
            "letteral for p.\n\n",
            "**Glosses:** `p`",
        );

        for offset in [0, 7, 11] {
            let hover = snapshot
                .hover(offset)
                .expect("every ZEI compound component has hover documentation");
            assert_eq!((hover.span.char_start, hover.span.char_end), (0, 12));
            assert_eq!(hover.markdown, expected);
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn attested_zei_compound_hover_prefers_the_compound_card() {
        let snapshot = DocumentSnapshot::new("a bu zei sance".to_owned(), 1);
        let hover = snapshot
            .hover(6)
            .expect("the attested ZEI compound has hover documentation");

        assert_eq!((hover.span.char_start, hover.span.char_end), (0, 14));
        assert_eq!(
            hover.markdown,
            concat!(
                "### `abu zei sance` — *ZEI compound*\n\n",
                "`x1` is an open/low central unrounded vowel sound produced by `x2`.\n\n",
                "**Glosses:** `A sound (sound of the letter A in Lojban and many other languages)`, ",
                "`low central unrounded vowel (phone)`, `open central unrounded vowel (phone)`, ",
                "`open/low central unrounded vowel sound`",
            ),
        );
        assert!(!hover.markdown.contains("### `abu`"));
        assert!(!hover.markdown.contains("### `sance`"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn ziholafti_excerpts_distinguish_plain_and_zei_compound_gleki() {
        const PLAIN_EXCERPT: &str = ".i pe'a lo gleki djedi zo'u";
        const ZEI_EXCERPT: &str = "lo gleki zei py. ku zifre cilce";

        let plain_start = PLAIN_EXCERPT.find("gleki").expect("plain excerpt");
        let plain = DocumentSnapshot::new(PLAIN_EXCERPT.to_owned(), 1)
            .hover(plain_start)
            .expect("plain gleki has hover documentation");
        assert_eq!(
            (plain.span.char_start, plain.span.char_end),
            (plain_start, plain_start + "gleki".len()),
        );
        assert!(plain.markdown.starts_with("### `gleki` — *gismu*"));
        assert!(!plain.markdown.contains("ZEI compound"));

        let zei_start = ZEI_EXCERPT.find("gleki").expect("ZEI excerpt");
        let zei = DocumentSnapshot::new(ZEI_EXCERPT.to_owned(), 1)
            .hover(zei_start)
            .expect("ZEI compound gleki has hover documentation");
        assert_eq!(
            (zei.span.char_start, zei.span.char_end),
            (zei_start, zei_start + "gleki zei py".len()),
        );
        assert!(
            zei.markdown
                .starts_with("### `gleki zei py` — *ZEI compound*")
        );
        assert!(zei.markdown.contains("### `gleki` — *gismu*"));
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
                "### `djarspageti` — *fu'ivla*\n\n",
                "`x1` is a quantity of spaghetti (long, thin cylindrical pasta).\n\n",
                "**Glosses:** `spaghetti`",
            ),
        );

        let cmevla = DocumentSnapshot::new(".alis.".to_owned(), 1)
            .hover(1)
            .expect("name word has morphology hover documentation");
        assert_eq!(cmevla.markdown, "### `alis` — *name word (cmevla)*");
        assert_eq!((cmevla.span.char_start, cmevla.span.char_end), (1, 5));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn unknown_dictionary_word_returns_morphological_classification() {
        let hover = DocumentSnapshot::new("brapu".to_owned(), 1)
            .hover(0)
            .expect("valid unknown gismu still has hover documentation");

        assert_eq!(hover.markdown, "### `brapu` — *gismu*");
        assert_eq!((hover.span.char_start, hover.span.char_end), (0, 5));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn morphology_invalid_cursor_text_has_no_hover() {
        let snapshot = DocumentSnapshot::new("pruxrpóltyrgaiste".to_owned(), 1);

        assert!(snapshot.hover(0).is_none());
        assert!(snapshot.hover(8).is_none());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn lenu_hovers_show_only_the_attested_sequence_card_and_full_span() {
        let snapshot = DocumentSnapshot::new("lenu".to_owned(), 1);
        let le = snapshot.hover(0).expect("le has hover documentation");
        let nu = snapshot.hover(2).expect("nu has hover documentation");

        assert_eq!((le.span.char_start, le.span.char_end), (0, 4));
        assert_eq!((nu.span.char_start, nu.span.char_end), (0, 4));
        assert_eq!(le.markdown, nu.markdown);
        assert_eq!(
            le.markdown,
            concat!(
                "### `lenu` — *cmavo sequence* · **LE\\***\n\n",
                "specific event descriptor: contraction of {le nu} and identical in meaning.\n\n",
                "**Glosses:** `the specific event of`",
            ),
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn minu_hovers_have_no_unattested_sequence_section() {
        let snapshot = DocumentSnapshot::new("minu".to_owned(), 1);
        let mi = snapshot.hover(0).expect("mi has hover documentation");
        let nu = snapshot.hover(2).expect("nu has hover documentation");

        assert_eq!((mi.span.char_start, mi.span.char_end), (0, 2));
        assert_eq!((nu.span.char_start, nu.span.char_end), (2, 4));
        assert_eq!(
            mi.markdown,
            concat!(
                "### `mi` — *cmavo* · **KOhA3**\n\n",
                "pro-sumti: me/we the speaker(s)/author(s); identified by self-vocative.\n\n",
                "**Glosses:** `me`, `I`\n\n",
                "**Rafsi:** `mib`",
            ),
        );
        assert_eq!(
            nu.markdown,
            concat!(
                "### `nu` — *cmavo* · **NU**\n\n",
                "abstractor: generalized event abstractor; `x1` is ",
                "state/process/achievement/activity of \\[bridi\\].\n\n",
                "**Glosses:** `event abstract`\n\n",
                "**Rafsi:** `nun`",
            ),
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn spaced_and_solid_ije_have_identical_je_hover_content() {
        let solid = DocumentSnapshot::new("ije".to_owned(), 1)
            .hover(1)
            .expect("solid je has hover documentation");
        let spaced = DocumentSnapshot::new("i je".to_owned(), 1)
            .hover(2)
            .expect("spaced je has hover documentation");

        assert_eq!((solid.span.char_start, solid.span.char_end), (0, 3));
        assert_eq!((spaced.span.char_start, spaced.span.char_end), (0, 4));
        assert_eq!(solid.markdown, spaced.markdown);
        assert_eq!(
            solid.markdown,
            concat!(
                "### `ije` — *cmavo sequence* · **JA\\***\n\n",
                "logical connective: sentence afterthought and.\n\n",
                "**Glosses:** `sentence and`",
            ),
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn ije_hover_contains_no_constituent_card() {
        let hover = DocumentSnapshot::new("ije".to_owned(), 1)
            .hover(1)
            .expect("je has hover documentation");

        assert!(hover.markdown.starts_with("### `ije` — *cmavo sequence*"));
        assert!(
            hover
                .markdown
                .contains("logical connective: sentence afterthought and.")
        );
        assert!(!hover.markdown.contains("### `je`"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn containing_cmavo_sequences_render_only_the_longest_attested_match() {
        let hover = DocumentSnapshot::new("binonovo".to_owned(), 1)
            .hover(0)
            .expect("bi has hover documentation");

        assert_eq!((hover.span.char_start, hover.span.char_end), (0, 8));
        assert!(
            hover
                .markdown
                .starts_with("### `binonovo` — *cmavo sequence*")
        );
        assert!(!hover.markdown.contains("### `binono`"));
        assert!(!hover.markdown.contains("### `bino`"));
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
        assert_eq!(lohu.markdown, "### `klama` — *gismu*");
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
