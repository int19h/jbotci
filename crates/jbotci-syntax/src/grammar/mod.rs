#[allow(unused_imports)]
use bityzba::{data, ensures, expensive_ensures, invariant, new, requires};
use std::collections::VecDeque;

use chumsky::Boxed;
use chumsky::input::MappedInput;
use chumsky::input::{Checkpoint, Cursor};
use chumsky::inspector::Inspector;
use chumsky::prelude::*;
use chumsky::span::{SimpleSpan, Spanned};
use jbotci_diagnostics::{
    TraceEventKind, TraceFailureSummary, TraceLevel, TracePhase, TraceRecorder, TraceReport,
};
use jbotci_morphology::{Cmavo, Selmaho, Word, WordLike, WordLikeData};

use crate::{
    ExperimentalConstruct, ParseOptions, SyntaxError, SyntaxExpectedToken, SyntaxParse,
    SyntaxParseAttempt, SyntaxWarning, SyntaxWordCategory, Token,
};

pub(crate) mod ast;
use ast::*;
mod parse_error;
mod parser;
mod tense;
pub(crate) mod tokens;
use parse_error::{SyntaxFound, SyntaxFoundData, SyntaxParseCustomKind, SyntaxParseError};

type Span = SimpleSpan;
type SpannedToken = Spanned<Token, Span>;
type ParserInput<'tokens> = MappedInput<'tokens, Token, Span, &'tokens [SpannedToken]>;
type ParseExtra<'tokens> = extra::Full<SyntaxParseError<'tokens>, ParserState, ()>;
type BoxedParser<'tokens, O> =
    Boxed<'tokens, 'tokens, ParserInput<'tokens>, O, ParseExtra<'tokens>>;

#[derive(Debug, Clone)]
#[invariant(true)]
pub(super) struct ParsedStatement {
    pub text: TextSyntax,
    pub warnings: Vec<SyntaxWarning>,
}

#[derive(Debug, Clone)]
#[invariant(true)]
pub(super) struct ParsedStatementAttempt {
    pub result: Result<ParsedStatement, SyntaxError>,
    pub trace: Option<TraceReport>,
}

#[derive(Debug, Clone)]
#[invariant(true)]
pub(super) struct ParserStateFinish {
    pub warnings: Vec<SyntaxWarning>,
    pub trace: Option<TraceReport>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
pub(crate) struct ParserCheckpoint {
    warning_count: usize,
    trace_save: bool,
}

#[derive(Debug, Clone, Default)]
#[invariant(true)]
pub(super) struct ParserState {
    anchor_byte_starts: Vec<Option<usize>>,
    warnings: Vec<SyntaxWarning>,
    trace: TraceRecorder,
}

impl ParserState {
    #[requires(true)]
    #[ensures(ret.anchor_byte_starts.len() == words.len())]
    pub(super) fn new(words: &[Token], options: &ParseOptions) -> Self {
        Self {
            anchor_byte_starts: words.iter().map(word_anchor_byte_start).collect(),
            warnings: Vec::new(),
            trace: TraceRecorder::new(options.trace.clone(), TracePhase::Syntax),
        }
    }

    #[requires(true)]
    #[ensures(self.warnings.len() == old(self.warnings.len()) + 1)]
    pub(super) fn warn(&mut self, construct: ExperimentalConstruct, anchor: &Token) {
        let anchor_index = self.anchor_index(anchor);
        let anchor = Token::bare(anchor.core_word().clone());
        self.warnings.push(SyntaxWarning::experimental_construct(
            construct,
            anchor_index,
            anchor,
        ));
    }

    #[requires(true)]
    #[ensures(self.warnings.len() == old(self.warnings.len()) + 1)]
    pub(super) fn warn_word(
        &mut self,
        construct: ExperimentalConstruct,
        context: &Token,
        anchor: &Word,
    ) {
        let anchor_index = self.anchor_index(context);
        self.warnings.push(SyntaxWarning::experimental_construct(
            construct,
            anchor_index,
            Token::bare(WordLike::bare(anchor.clone())),
        ));
    }

    #[requires(true)]
    #[ensures(ret.trace.as_ref().is_none_or(|report| report.phase == TracePhase::Syntax))]
    pub(super) fn finish(self) -> ParserStateFinish {
        let mut deduped = Vec::new();
        for warning in self.warnings {
            if !deduped.contains(&warning) {
                deduped.push(warning);
            }
        }
        ParserStateFinish {
            warnings: deduped,
            trace: self.trace.finish(),
        }
    }

    #[requires(true)]
    #[ensures(matches!(self.trace, TraceRecorder::Disabled) -> !ret)]
    pub(super) fn trace_enabled(&self) -> bool {
        self.trace.is_enabled()
    }

    #[requires(true)]
    #[ensures(matches!(self.trace, TraceRecorder::Disabled) -> !ret)]
    pub(super) fn trace_should_record(&self, level: TraceLevel, label: &str) -> bool {
        self.trace.should_record(level, label)
    }

    #[requires(byte_start <= byte_end)]
    #[ensures(true)]
    pub(super) fn trace_event(
        &mut self,
        level: TraceLevel,
        kind: TraceEventKind,
        label: &str,
        byte_start: usize,
        byte_end: usize,
        detail: impl FnOnce() -> Option<String>,
    ) {
        self.trace
            .record_with_detail(level, kind, label, byte_start, byte_end, detail);
    }

    #[requires(byte_start <= byte_end)]
    #[ensures(true)]
    pub(super) fn trace_enter_construct(
        &mut self,
        level: TraceLevel,
        label: &str,
        byte_start: usize,
        byte_end: usize,
    ) {
        self.trace
            .enter_construct(level, label, byte_start, byte_end);
    }

    #[requires(byte_start <= byte_end)]
    #[ensures(true)]
    pub(super) fn trace_exit_construct(
        &mut self,
        level: TraceLevel,
        kind: TraceEventKind,
        label: &str,
        byte_start: usize,
        byte_end: usize,
        detail: impl FnOnce() -> Option<String>,
    ) {
        self.trace
            .exit_construct(level, kind, label, byte_start, byte_end, detail);
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn trace_failure_summary(&mut self, failure: TraceFailureSummary) {
        self.trace.set_failure(failure);
    }

    #[requires(true)]
    #[ensures(ret < self.anchor_byte_starts.len() || self.anchor_byte_starts.is_empty())]
    fn anchor_index(&self, anchor: &Token) -> usize {
        if let Some(anchor_start) = word_anchor_byte_start(anchor)
            && let Some(index) = self
                .anchor_byte_starts
                .iter()
                .position(|candidate| *candidate == Some(anchor_start))
        {
            return index;
        }
        0
    }
}

impl<'tokens> Inspector<'tokens, ParserInput<'tokens>> for ParserState {
    type Checkpoint = ParserCheckpoint;

    #[requires(true)]
    #[ensures(true)]
    fn on_token(&mut self, token: &Token) {
        if !self.trace_should_record(TraceLevel::Primitives, "token") {
            return;
        }
        let span = token
            .core_word()
            .source_spans()
            .into_iter()
            .next()
            .map(|span| span.byte_start..span.byte_end)
            .unwrap_or(0..0);
        self.trace_event(
            TraceLevel::Primitives,
            TraceEventKind::Token,
            "token",
            span.start,
            span.end,
            || Some(trace_word_label(token)),
        );
    }

    #[requires(true)]
    #[ensures(ret.warning_count == self.warnings.len())]
    fn on_save<'parse>(
        &self,
        _cursor: &Cursor<'tokens, 'parse, ParserInput<'tokens>>,
    ) -> ParserCheckpoint {
        ParserCheckpoint {
            warning_count: self.warnings.len(),
            trace_save: self.trace_should_record(TraceLevel::Primitives, "save"),
        }
    }

    #[requires(true)]
    #[ensures(self.warnings.len() <= old(self.warnings.len()))]
    fn on_rewind<'parse>(
        &mut self,
        marker: &Checkpoint<'tokens, 'parse, ParserInput<'tokens>, ParserCheckpoint>,
    ) {
        if marker.inspector().trace_save {
            self.trace_event(
                TraceLevel::Primitives,
                TraceEventKind::Save,
                "save",
                0,
                0,
                || None,
            );
        }
        self.trace_event(
            TraceLevel::Primitives,
            TraceEventKind::Rewind,
            "rewind",
            0,
            0,
            || None,
        );
        self.warnings.truncate(marker.inspector().warning_count);
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn trace_word_label(token: &Token) -> String {
    token.core_word().to_string()
}

#[requires(true)]
#[ensures(true)]
fn word_anchor_byte_start(word: &Token) -> Option<usize> {
    word.core_word()
        .source_spans()
        .into_iter()
        .map(|span| span.byte_start)
        .min()
}

#[requires(true)]
#[ensures(true)]
#[expensive_ensures(ret.as_ref().map_or(true, |parse| {
    crate::syntax_parse_leaf_spans_match_words(words, parse)
}))]
pub(crate) fn parse_syntax_tree(
    words: &[WordLike],
    options: &ParseOptions,
) -> Result<SyntaxParse, SyntaxError> {
    parse_syntax_tree_with_source(words, None, options)
}

#[requires(true)]
#[ensures(true)]
#[expensive_ensures(ret.as_ref().map_or(true, |parse| {
    crate::syntax_parse_leaf_spans_match_words(words, parse)
}))]
pub(crate) fn parse_syntax_tree_with_source(
    words: &[WordLike],
    source: Option<&str>,
    options: &ParseOptions,
) -> Result<SyntaxParse, SyntaxError> {
    parse_syntax_tree_with_source_attempt(words, source, options).result
}

#[requires(true)]
#[ensures(true)]
#[expensive_ensures(ret.result.as_ref().map_or(true, |parse| {
    crate::syntax_parse_leaf_spans_match_words(words, parse)
}))]
pub(crate) fn parse_syntax_tree_with_source_attempt(
    words: &[WordLike],
    source: Option<&str>,
    options: &ParseOptions,
) -> SyntaxParseAttempt {
    let tokens = syntax_tokens(words);
    let parsed = parser::parse_statement_attempt(&tokens, source, options);
    let result = parsed.result.map(|parsed| {
        new!(SyntaxParse {
            parse_tree: Box::new(parsed.text),
            warnings: parsed.warnings,
        })
    });
    SyntaxParseAttempt {
        result,
        trace: parsed.trace,
    }
}

#[requires(true)]
#[ensures(true)]
#[expensive_ensures(ret.as_ref().map_or(true, |parse_tree| {
    crate::text_syntax_leaf_spans_match_words(words, parse_tree)
}))]
pub(crate) fn parse_text(
    words: &[WordLike],
    options: &ParseOptions,
) -> Result<TextSyntax, SyntaxError> {
    let tokens = syntax_tokens(words);
    Ok(parser::parse_statement(&tokens, None, options)?.text)
}

#[cfg(feature = "grammar-debug")]
#[requires(true)]
#[ensures(!ret.is_empty())]
pub(crate) fn syntax_grammar_ebnf(options: &ParseOptions) -> String {
    parser::syntax_grammar_ebnf(options)
}

#[cfg(feature = "grammar-debug")]
#[requires(true)]
#[ensures(!ret.is_empty())]
pub(crate) fn syntax_grammar_svg(options: &ParseOptions) -> String {
    parser::syntax_grammar_svg(options)
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn syntax_tokens(words: &[WordLike]) -> Vec<Token> {
    attach_indicators(attach_bahe(
        words.iter().cloned().map(Token::bare).collect(),
    ))
}

#[requires(true)]
#[ensures(true)]
fn attach_bahe(words: Vec<Token>) -> Vec<Token> {
    let mut reversed: VecDeque<_> = words.into_iter().rev().collect();
    let mut out = Vec::new();
    while let Some(word) = reversed.pop_front() {
        if reversed.front().is_some_and(is_bahe_word)
            && let Some(bahe_token) = reversed.pop_front()
            && let Some(bahe) = modifier_word(&bahe_token)
        {
            reversed.push_front(Token::emphasized(bahe, word.core_word().clone()));
        } else {
            out.push(word);
        }
    }
    out.reverse();
    out
}

#[requires(true)]
#[ensures(true)]
fn is_bahe_word(word: &Token) -> bool {
    modifier_word(word).is_some_and(|word| word.is_one_of_cmavo(&[Cmavo::Bahe, Cmavo::Zahe]))
}

#[requires(true)]
#[ensures(true)]
fn attach_indicators(words: Vec<Token>) -> Vec<Token> {
    let mut out = Vec::new();
    let mut iter = words.into_iter().peekable();
    while let Some(word) = iter.next() {
        if modifier_word(&word).is_some_and(|word| is_indicator_word(&word)) {
            let indicator = modifier_word(&word);
            let nai = if iter
                .peek()
                .and_then(modifier_word)
                .is_some_and(|next| next.is_cmavo(Cmavo::Nai))
            {
                iter.next().and_then(|next| modifier_word(&next))
            } else {
                None
            };
            if let (Some(prev), Some(indicator)) = (out.pop(), indicator) {
                let prev_is_leading_indicator_nai = modifier_word(&prev)
                    .is_some_and(|word| word.is_cmavo(Cmavo::Nai))
                    && out
                        .last()
                        .and_then(modifier_word)
                        .is_some_and(|word| is_indicator_word(&word));
                if prev_is_leading_indicator_nai || !should_attach_indicator(&prev, &indicator) {
                    out.push(prev);
                    out.push(word);
                    if let Some(nai) = nai {
                        out.push(Token::bare(WordLike::bare(nai)));
                    }
                } else {
                    out.push(Token::with_indicator(prev, indicator, nai));
                }
            } else {
                out.push(word);
                if let Some(nai) = nai {
                    out.push(Token::bare(WordLike::bare(nai)));
                }
            }
        } else {
            out.push(word);
        }
    }
    out
}

#[requires(true)]
#[ensures(true)]
fn modifier_word(word: &Token) -> Option<Word> {
    word.core_word().bare_word().cloned()
}

#[requires(true)]
#[ensures(true)]
fn is_indicator_word(word: &Word) -> bool {
    word.cmavo().is_some_and(|cmavo| {
        cmavo.is_selmaho(Selmaho::Ui) || cmavo.is_selmaho(Selmaho::Cai) || cmavo == Cmavo::Y
    })
}

#[requires(true)]
#[ensures(true)]
fn should_attach_indicator(prev: &Token, indicator: &Word) -> bool {
    !(indicator.is_selmaho(Selmaho::Roi)
        && modifier_word(prev).is_some_and(|prev| prev.is_selmaho(Selmaho::Pa)))
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use bityzba::{data, new, requires, try_new};
    use jbotci_dialect::parse_dialect_definition;
    use jbotci_morphology::segment_words_with_modifiers;

    use super::*;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn parses_basic_predicate_with_leading_and_tail_terms() {
        run_on_normal_stack(|| {
            let words = segment_words_with_modifiers("do mamta mi").expect("valid morphology");

            let parsed = parse_syntax_tree(&words, &ParseOptions::default()).expect("valid syntax");

            assert_eq!(parsed.parse_tree.paragraphs.len(), 1);
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn rejects_stray_cu() {
        run_on_normal_stack(|| {
            let words = segment_words_with_modifiers("cu").expect("valid morphology");

            let error = parse_syntax_tree(&words, &ParseOptions::default()).expect_err("invalid");

            assert!(matches!(error, SyntaxError::Parse { .. }));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn parses_grouped_math_operator() {
        run_on_normal_stack(|| {
            let words = segment_words_with_modifiers("li re ke su'i ke'e ci du li mu")
                .expect("valid morphology");

            let parsed = parse_syntax_tree(&words, &ParseOptions::default()).expect("valid syntax");

            assert!(format!("{:#?}", parsed.parse_tree).contains("GroupedOperator"));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn parses_bo_connected_math_operator() {
        run_on_normal_stack(|| {
            let words = segment_words_with_modifiers("li re su'i je bo vu'u ci du li mu")
                .expect("valid morphology");

            let parsed = parse_syntax_tree(&words, &ParseOptions::default()).expect("valid syntax");

            assert!(format!("{:#?}", parsed.parse_tree).contains("Bo"));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn parses_pehe_termset_with_cehe_connectives_under_contracts() {
        run_on_normal_stack(|| {
            let words = segment_words_with_modifiers(
                "mi klama le zarci ce'e le briju pe'e je le zdani ce'e le ckule",
            )
            .expect("valid morphology");

            let parsed = parse_syntax_tree(&words, &ParseOptions::default()).expect("valid syntax");
            let raw = format!("{:?}", parsed.parse_tree);

            assert!(raw.contains("TermsetConnection"));
            assert!(raw.contains("NonLogical"));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn parses_emphasized_goha_relation_under_contracts() {
        run_on_normal_stack(|| {
            let words = segment_words_with_modifiers("le lojbo cu ba'e du le loglo")
                .expect("valid morphology");

            let parsed = parse_syntax_tree(&words, &ParseOptions::default()).expect("valid syntax");
            let raw = format!("{:?}", parsed.parse_tree);

            assert!(raw.contains("Emphasized"));
            assert!(raw.contains("du"));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn parses_statement_connective_with_flattened_fiho_relation_under_contracts() {
        run_on_normal_stack(|| {
            let words = segment_words_with_modifiers("i fi'o ke broda brode bo mi klama")
                .expect("valid morphology");

            let parsed = parse_syntax_tree(&words, &ParseOptions::default()).expect("valid syntax");
            let raw = format!("{:?}", parsed.parse_tree);

            assert!(raw.contains("connective: Some(Selbri"));
            assert!(raw.contains("fi'o"));
            assert!(raw.contains("bróda"));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn keeps_i_connectives_out_of_tail_terms() {
        run_on_normal_stack(|| {
            let raw = parse_tree_debug("mi ca pilno .ije ca'o nelci", &ParseOptions::default());

            assert!(raw.contains("StatementConnection"));
            assert!(raw.contains("leading_statement"));
            assert!(raw.contains("trailing_statement"));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn classifies_mohi_as_spatial_movement_not_koha() {
        run_on_normal_stack(|| {
            let raw = parse_tree_debug(
                "le verba mo'i ri'u cadzu le bisli",
                &ParseOptions::default(),
            );

            assert!(raw.contains("TaggedSelbri"));
            assert!(raw.contains("mo'i"));
            assert!(!raw.contains("ProSumti(WithFreeModifiers { value: Plain(PlainWord(Cmavo { phonemes: Phonemes { text: \"mo'i\" }"));

            let words = segment_words_with_modifiers("da poi palci vimo'i selklama")
                .expect("valid morphology");
            assert!(parse_syntax_tree(&words, &ParseOptions::default()).is_err());
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn parses_v0_joik_and_cehe_argument_connective_cases() {
        run_on_normal_stack(|| {
            for source in [
                "la djeimyz. cebo la djordj. bruna remei",
                "mi joibo do cu broda",
                "ju'a nai cy pa ka ce'u ce ke do ke'e simxu cy no kei",
                "ce'e di",
            ] {
                parse_source(source, &ParseOptions::default());
            }
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn parses_nested_descriptor_tail_on_fixture_worker_stack() {
        run_on_fixture_worker_stack(|| {
            let source = "mi pensi ledu'u mi ba stidi fi la nitcion. fe le pu selsnu be mi joi do poi ckini lei bifce poi pu xabju le mi zdani kei";
            let words = segment_words_with_modifiers(source).expect("valid morphology");

            parse_syntax_tree(&words, &ParseOptions::default()).expect("valid syntax");
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn parses_modal_abstraction_tail_on_fixture_worker_stack() {
        run_on_fixture_worker_stack(|| {
            let source = ".ino'iji'a pa makcu nixli cu pleji fi mi lenu kelci ki'u lenu te cusku fe lesedu'u mi xamgu to malglico toi kelci";
            let words = segment_words_with_modifiers(source).expect("valid morphology");

            parse_syntax_tree(&words, &ParseOptions::default()).expect("valid syntax");
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn parses_grouped_argument_recursion_on_fixture_worker_stack() {
        run_on_fixture_worker_stack(|| {
            let source = concat!(
                " i abu zi ba le nu facki le du'u makau drani tadji le nu kurji cy ",
                "to no'u le nu tongau cy ja'e lo jgena gi'e tagji jgari le cy pritu ",
                "kerlo ku joi le cy zunle jamfu ja'e le nu rivbi le nu cy sezytolplo ",
                "toi cu bevri cy le bartu vacri i lu lei du romu'ei le du'u mi na ",
                "lebna le vi cifnu sei la alis pensi cu ba catra cy za lo djedi be ",
                "li ji'ire i xu na zekri fa le nu cliva cy li'u i abu cladu cusku ",
                "lei romoi valsi i le cmalu cu spuda cmoni to cy ca ba'o senci toi ",
                "i lu ko na cmoni sei la alis cusku i nasai drani tadji le nu cusku li'u ",
            );
            let words = segment_words_with_modifiers(source).expect("valid morphology");

            parse_syntax_tree(&words, &ParseOptions::default()).expect("valid syntax");
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn bare_vowel_cmavo_are_not_implicit_letters() {
        run_on_normal_stack(|| {
            let words = segment_words_with_modifiers("a cmene").expect("valid morphology");
            assert!(parse_syntax_tree(&words, &ParseOptions::default()).is_err());

            let raw = parse_tree_debug("a bu cmene", &ParseOptions::default());
            assert!(raw.contains("LerfuWord"));

            let raw = parse_tree_debug("abu cmene", &ParseOptions::default());
            assert!(raw.contains("LerfuWord"));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn core_word_strips_syntax_wrappers_but_preserves_word_like_unit() {
        run_on_normal_stack(|| {
            let mut words = segment_words_with_modifiers("zo coi").expect("valid morphology");
            let quote = words.remove(0);
            let wrapped = WithFreeModifiers::new(
                Token::with_indicator(
                    Token::emphasized(single_bare_word("ba'e"), quote.clone()),
                    single_bare_word("ui"),
                    None,
                ),
                Vec::new(),
            );

            assert_eq!(wrapped.core_word(), &quote);
            assert_eq!(wrapped.quote_marker_cmavo(), Some(Cmavo::Zo));
            assert!(!wrapped.is_cmavo(Cmavo::Zo));
            assert!(!wrapped.is_selmaho(Selmaho::Zo));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn quote_warning_anchor_covers_whole_core_word_like() {
        run_on_normal_stack(|| {
            let parsed = parse_source("mi tavla zo'oi broda", &ParseOptions::default());
            let quote_warning = parsed
                .warnings
                .iter()
                .find(|warning| warning.kind == ExperimentalConstruct::ExperimentalZohOiQuote)
                .expect("ZOhOI warning");

            assert_eq!(warning_span(quote_warning), [9, 20]);
            assert!(matches!(
                quote_warning.anchor.core_word().as_data(),
                data!(WordLike::DelimitedWordQuote { .. })
            ));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn mehoi_quote_warning_is_distinct_from_selbri_unit_warning() {
        run_on_normal_stack(|| {
            let parsed = parse_source("mi me'oi broda", &ParseOptions::default());

            assert!(has_warning_kind(
                &parsed,
                ExperimentalConstruct::ExperimentalMehOiQuote
            ));
            assert!(!has_warning_kind(
                &parsed,
                ExperimentalConstruct::ExperimentalMehOiSelbriUnit
            ));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn non_lu_quotes_do_not_warn_for_quoted_experimental_cmavo() {
        run_on_normal_stack(|| {
            for source in [
                "mi tavla zo li'oi",
                "mi tavla zo'oi li'oi",
                "mi tavla lo'u li'oi le'u",
            ] {
                let parsed = parse_source(source, &ParseOptions::default());
                assert!(
                    !has_warning_kind(
                        &parsed,
                        ExperimentalConstruct::ExperimentalDictionaryUiIndicator
                    ),
                    "{source}"
                );
            }
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn lu_quote_warns_for_inner_experimental_cmavo() {
        run_on_normal_stack(|| {
            let parsed = parse_source("mi cusku lu li'oi li'u", &ParseOptions::default());
            assert!(has_warning_kind(
                &parsed,
                ExperimentalConstruct::ExperimentalDictionaryUiIndicator
            ));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn experimental_indicator_warning_anchors_indicator_word() {
        run_on_normal_stack(|| {
            let parsed = parse_source("mi li'oi klama", &ParseOptions::default());
            let warning = parsed
                .warnings
                .iter()
                .find(|warning| {
                    warning.kind == ExperimentalConstruct::ExperimentalDictionaryUiIndicator
                })
                .expect("experimental UI warning");

            assert_eq!(warning.anchor_index, 0);
            assert_eq!(warning_span(warning), [3, 8]);
            assert!(warning.anchor.is_cmavo(Cmavo::Lihoi));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn parses_experimental_muhei_roi_tense_with_warning() {
        run_on_normal_stack(|| {
            let parsed = parse_source("mi so'emu'ei spuda", &ParseOptions::default());

            assert!(format!("{:?}", parsed.parse_tree).contains("Composite"));
            assert!(has_warning_kind(
                &parsed,
                ExperimentalConstruct::ExperimentalCmavo
            ));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn accepts_additive_zantufa_quote_relation_units_by_default() {
        run_on_normal_stack(|| {
            let words =
                segment_words_with_modifiers("lu'ei mi klama li'au").expect("valid morphology");

            let parsed = parse_syntax_tree(&words, &ParseOptions::default())
                .expect("valid zantufa quote syntax");

            assert!(parsed.warnings.iter().any(|warning| {
                warning.kind == ExperimentalConstruct::ExperimentalZantufaLuheiSelbriUnit
            }));

            let words =
                segment_words_with_modifiers("mi cu mu'oi gy foo gy").expect("valid morphology");

            let parsed = parse_syntax_tree(&words, &ParseOptions::default())
                .expect("valid zantufa MUhOI syntax");

            assert!(parsed.warnings.iter().any(|warning| {
                warning.kind == ExperimentalConstruct::ExperimentalZantufaMuhoiSelbriUnit
            }));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gates_zantufa_jai_tag_terms() {
        run_on_normal_stack(|| {
            let words =
                segment_words_with_modifiers("jai pu mi cu klama").expect("valid morphology");

            assert!(parse_syntax_tree(&words, &ParseOptions::default()).is_err());

            let dialect =
                parse_dialect_definition("(+ZANTUFA-TAGS)").expect("valid dialect definition");
            let options = ParseOptions::default().with_dialect_definition(&dialect);
            let parsed = parse_syntax_tree(&words, &options).expect("valid zantufa JAI tag term");

            assert!(parsed.warnings.iter().any(|warning| {
                warning.kind == ExperimentalConstruct::ExperimentalZantufaJaiTagTerm
            }));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn accepts_additive_zantufa_poiha_brigahi_ku_by_default() {
        run_on_normal_stack(|| {
            let words = segment_words_with_modifiers("noi'a klama ku mi cu broda")
                .expect("valid morphology");

            let parsed = parse_syntax_tree(&words, &ParseOptions::default())
                .expect("valid Zantufa POIhA briga'i");

            assert!(parsed.warnings.iter().any(|warning| {
                warning.kind == ExperimentalConstruct::ExperimentalZantufaPoihaBrigahi
            }));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn accepts_zantufa_cmavo_table_entries_with_warning() {
        run_on_normal_stack(|| {
            let words = segment_words_with_modifiers("mi cu xe'u").expect("valid morphology");

            let parsed = parse_syntax_tree(&words, &ParseOptions::default())
                .expect("valid Zantufa cmavo syntax");

            assert!(parsed.warnings.iter().any(|warning| {
                warning.kind == ExperimentalConstruct::ExperimentalZantufaCmavo
            }));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn parses_zantufa_1_17_gohoi_markers_as_word_quotes() {
        run_on_normal_stack(|| {
            for marker in ["go'oi", "ze'oi", "ta'ai", "bo'ei"] {
                let source = format!("mi cu {marker} coi");
                let words = segment_words_with_modifiers(&source).expect("valid morphology");
                let parsed = parse_syntax_tree(&words, &ParseOptions::default())
                    .expect("valid GOhOI word quote selbri");
                let debug_tree = format!("{:?}", parsed.parse_tree);

                assert!(debug_tree.contains("QuotedBridiSelbri"));
                assert!(parsed.warnings.iter().any(|warning| {
                    warning.kind == ExperimentalConstruct::ExperimentalGohoiSelbriUnit
                }));
                assert!(!parsed.warnings.iter().any(|warning| {
                    warning.kind == ExperimentalConstruct::ExperimentalZantufaCmavo
                }));
            }
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn parses_zantufa_1_17_lohoi_bridi_descriptions() {
        run_on_normal_stack(|| {
            for lohoi in ["lo'oi", "xu'u", "xau'a", "mau'a"] {
                let source = format!("{lohoi} mi cu broda ku'au");
                let parsed = parse_source(&source, &ParseOptions::default());

                assert!(format!("{:?}", parsed.parse_tree).contains("BridiDescription"));
                assert!(has_warning_kind(
                    &parsed,
                    ExperimentalConstruct::ExperimentalLohOiBridiDescription
                ));
            }

            let ui_parse = parse_source("xau'a mi cu broda", &ParseOptions::default());
            assert!(!format!("{:?}", ui_parse.parse_tree).contains("BridiDescription"));
            assert!(!has_warning_kind(
                &ui_parse,
                ExperimentalConstruct::ExperimentalLohOiBridiDescription
            ));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn parses_zantufa_1_17_rahoi_quote_warning() {
        run_on_normal_stack(|| {
            let parsed = parse_source("ra'oi broda cu brode", &ParseOptions::default());

            assert!(format!("{:?}", parsed.parse_tree).contains("DelimitedWordQuote"));
            assert!(has_warning_kind(
                &parsed,
                ExperimentalConstruct::ExperimentalZantufaRahoiQuote
            ));
            assert!(!has_warning_kind(
                &parsed,
                ExperimentalConstruct::ExperimentalZohOiQuote
            ));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn parses_zantufa_1_17_xoi_as_adverbial_term() {
        run_on_normal_stack(|| {
            let parsed = parse_source("xoi mi broda", &ParseOptions::default());

            assert!(format!("{:?}", parsed.parse_tree).contains("ReciprocalBridiAdverbialTerm"));
            assert!(has_warning_kind(
                &parsed,
                ExperimentalConstruct::ExperimentalSoiAdverbial
            ));
            assert!(!has_warning_kind(
                &parsed,
                ExperimentalConstruct::ExperimentalDictionarySeiFreeModifier
            ));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gates_zantufa_initial_gi_gek() {
        run_on_normal_stack(|| {
            let words = segment_words_with_modifiers("gi je mi klama gi do klama")
                .expect("valid morphology");

            assert!(parse_syntax_tree(&words, &ParseOptions::default()).is_err());

            let dialect = parse_dialect_definition("(+ZANTUFA-CONNECTIVES)")
                .expect("valid dialect definition");
            let options = ParseOptions::default().with_dialect_definition(&dialect);
            let parsed = parse_syntax_tree(&words, &options).expect("valid Zantufa GI GEK");

            assert!(
                parsed
                    .warnings
                    .iter()
                    .any(|warning| warning.kind == ExperimentalConstruct::ExperimentalZantufaGek)
            );
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gates_zantufa_gihi_forethought_terminator() {
        run_on_normal_stack(|| {
            let words = segment_words_with_modifiers("ge mi klama gi do klama gi'i")
                .expect("valid morphology");

            assert!(parse_syntax_tree(&words, &ParseOptions::default()).is_err());

            let dialect = parse_dialect_definition("(+ZANTUFA-CONNECTIVES)")
                .expect("valid dialect definition");
            let options = ParseOptions::default().with_dialect_definition(&dialect);
            let parsed = parse_syntax_tree(&words, &options).expect("valid Zantufa GIhI");

            assert!(parsed.warnings.iter().any(|warning| {
                warning.kind == ExperimentalConstruct::ExperimentalZantufaForethoughtGihi
            }));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn warns_for_jek_gek_and_bo_gek_extensions() {
        run_on_normal_stack(|| {
            let words = segment_words_with_modifiers("je gi mi klama gi do klama")
                .expect("valid morphology");
            let parsed =
                parse_syntax_tree(&words, &ParseOptions::default()).expect("valid jek GEK");
            assert!(
                parsed
                    .warnings
                    .iter()
                    .any(|warning| warning.kind == ExperimentalConstruct::ExperimentalZantufaGek)
            );

            let words = segment_words_with_modifiers("joi gi bo mi klama gi do klama")
                .expect("valid morphology");
            let parsed = parse_syntax_tree(&words, &ParseOptions::default()).expect("valid BO GEK");
            assert!(
                parsed
                    .warnings
                    .iter()
                    .any(|warning| warning.kind == ExperimentalConstruct::ExperimentalZantufaGek)
            );
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn warns_for_flat_tag_forms() {
        run_on_normal_stack(|| {
            let words =
                segment_words_with_modifiers("na'e fa mi cu klama").expect("valid morphology");

            let parsed = parse_syntax_tree(&words, &ParseOptions::default())
                .expect("valid flattened FA tag");

            assert!(
                parsed
                    .warnings
                    .iter()
                    .any(|warning| warning.kind == ExperimentalConstruct::ExperimentalFlattenedTag)
            );
            assert!(
                parsed
                    .warnings
                    .iter()
                    .any(|warning| warning.kind == ExperimentalConstruct::ExperimentalFaAsTag)
            );
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gates_zantufa_recursive_tags() {
        run_on_normal_stack(|| {
            let words = segment_words_with_modifiers("na'e se na'e se fa mi cu klama")
                .expect("valid morphology");

            assert!(parse_syntax_tree(&words, &ParseOptions::default()).is_err());

            let dialect =
                parse_dialect_definition("(+ZANTUFA-TAGS)").expect("valid dialect definition");
            let options = ParseOptions::default().with_dialect_definition(&dialect);
            let parsed = parse_syntax_tree(&words, &options).expect("valid recursive tag");

            assert!(parsed.warnings.iter().any(|warning| {
                warning.kind == ExperimentalConstruct::ExperimentalZantufaRecursiveTag
            }));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn classifies_v0_dictionary_first_cases_by_dictionary_selmaho() {
        run_on_normal_stack(|| {
            let cases = [
                (
                    "a'oi do klama",
                    ExperimentalConstruct::ExperimentalDictionaryCoiVocative,
                ),
                (
                    "o'ai do klama",
                    ExperimentalConstruct::ExperimentalDictionaryCoiVocative,
                ),
                (
                    "xe'e lo gerku cu klama",
                    ExperimentalConstruct::ExperimentalDictionaryPaNumber,
                ),
                (
                    "su'ai lo gerku cu klama",
                    ExperimentalConstruct::ExperimentalDictionaryPaNumber,
                ),
                (
                    "xei'e lo kibro mi klama",
                    ExperimentalConstruct::ExperimentalDictionaryFahaTag,
                ),
                (
                    "li'oi mi klama",
                    ExperimentalConstruct::ExperimentalDictionaryUiIndicator,
                ),
            ];

            for (source, expected) in cases {
                assert_warning_kind(source, &ParseOptions::default(), expected);
            }

            let xoi = parse_source("mi klama xoi mutce", &ParseOptions::default());
            assert!(has_warning_kind(
                &xoi,
                ExperimentalConstruct::ExperimentalSoiAdverbial
            ));
            assert!(!has_warning_kind(
                &xoi,
                ExperimentalConstruct::ExperimentalDictionarySeiFreeModifier
            ));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn cbm_accepts_cmevla_relation_in_descriptor_arguments() {
        run_on_normal_stack(|| {
            let source = "lo .alis. broda cu melbi";
            let baseline_words = segment_words_with_modifiers(source).expect("valid morphology");
            assert!(parse_syntax_tree(&baseline_words, &ParseOptions::default()).is_err());

            let dialect = parse_dialect_definition("(+CBM)").expect("valid dialect definition");
            let options = ParseOptions::default().with_dialect_definition(&dialect);
            let cbm = parse_tree_debug(source, &options);
            assert!(cbm.contains("Sumti("));
            assert!(cbm.contains("Description("));
            assert!(cbm.contains("Cmevla {"));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn cbm_warns_for_cmevla_relation_words() {
        run_on_normal_stack(|| {
            let dialect = parse_dialect_definition("(+CBM)").expect("valid dialect definition");
            let options = ParseOptions::default().with_dialect_definition(&dialect);

            assert_warning_kind(
                "lo .alis. broda cu melbi",
                &options,
                ExperimentalConstruct::ExperimentalCbmCmevlaSelbriWord,
            );
            assert_warning_kind(
                ".alis. broda",
                &options,
                ExperimentalConstruct::ExperimentalCbmCmevlaSelbriWord,
            );
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn rejects_wrong_enum_variant_cmavo_markers() {
        run_on_normal_stack(|| {
            let subbridi = sample_subbridi();

            assert!(
                try_new!(SumtiSyntax::BridiDescription {
                    lohoi: free_word("le"),
                    subbridi: Box::new(subbridi.clone()),
                    kuhau: None,
                })
                .is_err()
            );
            assert!(
                try_new!(SumtiSyntax::BridiDescription {
                    lohoi: free_word("lo'oi"),
                    subbridi: Box::new(subbridi),
                    kuhau: Some(free_word("ku'o")),
                })
                .is_err()
            );
            assert!(try_new!(TanruUnitSyntax::QuotedWordSelbri(free_word("go'oi broda"))).is_err());
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn rejects_wrong_struct_cmavo_markers() {
        run_on_normal_stack(|| {
            let sumti = sample_argument();
            let selbri = sample_relation();
            let subbridi = sample_subbridi();
            let bridi_tail = sample_predicate_tail();
            let predicate_tail2 = sample_predicate_tail2();
            let connective = sample_connective();

            assert!(
                try_new!(SumtiAssociationPhraseSyntax {
                    association_marker: free_word("le"),
                    sumti: Box::new(sumti.clone()),
                    gehu: None,
                })
                .is_err()
            );
            assert!(
                try_new!(SelbriRelativePhraseSyntax {
                    nohoi: free_word("no'oi"),
                    selbri: Box::new(selbri.clone()),
                    kuhoi: Some(free_word("ku'o")),
                })
                .is_err()
            );
            assert!(
                try_new!(DescriptionHeadSyntax {
                    description: free_word("mi"),
                })
                .is_err()
            );
            assert!(
                try_new!(DescriptionSyntax {
                    outer_quantifier: None,
                    description: Some(free_word("lo")),
                    tail_elements: Vec::new(),
                    selbri: None,
                    relative_clauses: Vec::new(),
                    ku: Some(free_word("ku'o")),
                })
                .is_err()
            );
            assert!(
                try_new!(AdditionalLinkedSumtiSyntax {
                    bei: free_word("be"),
                    fa: Some(free_word("fa")),
                    sumti: None,
                })
                .is_err()
            );
            assert!(
                try_new!(BridiSyntax {
                    leading_terms: Vec::new(),
                    cu: Some(std::sync::Arc::new(free_word("ku"))),
                    bridi_tail: Box::new(bridi_tail.clone()),
                    free_modifiers: Vec::new(),
                })
                .is_err()
            );
            assert!(
                try_new!(GroupedBridiTailConnectionSyntax {
                    connective: connective.clone(),
                    tense_modal: None,
                    ke: free_word("ke"),
                    bridi_tail: Box::new(bridi_tail.clone()),
                    kehe: Some(std::sync::Arc::new(free_word("ku"))),
                    tail_terms: Vec::new(),
                    vau: None,
                    free_modifiers: Vec::new(),
                })
                .is_err()
            );
            assert!(
                try_new!(BoundBridiTailConnectionSyntax {
                    connective: connective.clone(),
                    tense_modal: None,
                    bo: free_word("boi"),
                    cu: None,
                    bridi_tail: Box::new(predicate_tail2),
                    tail_terms: Vec::new(),
                    vau: None,
                    free_modifiers: Vec::new(),
                })
                .is_err()
            );
            assert!(
                try_new!(TextSyntax {
                    leading_nai: vec![indicated_word("i")],
                    leading_cmevla: Vec::new(),
                    leading_indicators: Vec::new(),
                    leading_free_modifiers: Vec::new(),
                    leading_connective: None,
                    paragraphs: Vec::new(),
                })
                .is_err()
            );
            assert!(
                try_new!(ParagraphSyntax {
                    i: Some(indicated_word("u'i")),
                    niho: Vec::new(),
                    free_modifiers: Vec::new(),
                    statements: Vec::new(),
                })
                .is_err()
            );
            assert!(
                try_new!(AdHocModalSyntax {
                    nahe: None,
                    fiho: free_word("fe'u"),
                    selbri: Box::new(selbri.clone()),
                    fehu: None,
                })
                .is_err()
            );
            assert!(
                try_new!(AbstractionSyntax {
                    nu: free_word("nu"),
                    nai: None,
                    abstractor_connections: Vec::new(),
                    subbridi: Box::new(subbridi),
                    kei: Some(free_word("ku")),
                })
                .is_err()
            );
            assert!(
                try_new!(AbstractorConnectionSyntax {
                    connective,
                    nu: free_word("ka'e"),
                    nai: None,
                })
                .is_err()
            );
            assert!(
                try_new!(ProBridiAssignmentSyntax {
                    cei: free_word("bei"),
                    tanru_unit: Box::new(new!(TanruUnitSyntax::TanruUnitWord(free_word("klama")))),
                })
                .is_err()
            );
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn rejects_empty_repeated_enum_payloads() {
        assert!(try_new!(FragmentSyntax::LinkedSumtiContinuation(Vec::new())).is_err());
        assert!(try_new!(FragmentSyntax::RelativeClauses(Vec::new())).is_err());
        assert!(
            try_new!(DescriptionTailElementSyntax::DescriptionTailRelativeClauses(Vec::new()))
                .is_err()
        );
    }

    #[requires(!source.is_empty())]
    #[ensures(true)]
    fn assert_warning_kind(source: &str, options: &ParseOptions, expected: ExperimentalConstruct) {
        let parsed = parse_source(source, options);
        assert!(has_warning_kind(&parsed, expected), "{source}");
    }

    #[requires(true)]
    #[ensures(true)]
    fn has_warning_kind(parsed: &SyntaxParse, expected: ExperimentalConstruct) -> bool {
        parsed
            .warnings
            .iter()
            .any(|warning| warning.kind == expected)
    }

    #[requires(!source.is_empty())]
    #[ensures(true)]
    fn parse_tree_debug(source: &str, options: &ParseOptions) -> String {
        format!("{:?}", parse_source(source, options).parse_tree)
    }

    #[requires(!source.is_empty())]
    #[ensures(true)]
    fn parse_source(source: &str, options: &ParseOptions) -> SyntaxParse {
        let words = segment_words_with_modifiers(source).expect("valid morphology");
        parse_syntax_tree(&words, options).expect("valid syntax")
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn chrestomathy_cu_terms_selbri_fallback_parses_alice_naku() {
        let parsed = parse_source("mi cu naku naku klama", &ParseOptions::default());
        assert!(has_warning_kind(
            &parsed,
            ExperimentalConstruct::ExperimentalCuTermsSelbri
        ));
        assert!(format!("{:?}", parsed.parse_tree).contains("TermPrefixedBridiTail"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn chrestomathy_cu_terms_selbri_fallback_preserves_existing_cu_parses() {
        for source in [
            "mi cu pu klama",
            "mi cu na klama",
            "mi cu fa klama",
            "cu klama",
            "cu fa klama",
        ] {
            let parsed = parse_source(source, &ParseOptions::default());
            let raw = format!("{:?}", parsed.parse_tree);
            assert!(
                !raw.contains("TermPrefixedBridiTail"),
                "{source} should keep its existing bridi-tail parse"
            );
            assert!(
                !has_warning_kind(&parsed, ExperimentalConstruct::ExperimentalCuTermsSelbri),
                "{source} should not use the CU TERMS fallback"
            );
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn chrestomathy_statement_i_stag_bo_accepts_free_modifier() {
        parse_source(
            "do tavla .i ca bo sei mi cusku mi klama",
            &ParseOptions::default(),
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn chrestomathy_ke_termset_parses_alice_table_row() {
        let parsed = parse_source(
            "la .alis. cu penmi le cmalu jubme .i cpana le jubme fa ke po'o le cmacma ke solji ckiku",
            &ParseOptions::default(),
        );
        assert!(has_warning_kind(
            &parsed,
            ExperimentalConstruct::ExperimentalKeTermset
        ));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn chrestomathy_repeated_cehe_termset_group_parses_forest_row() {
        let parsed = parse_source(
            ".i ko klama doi cilce je ricfoi ninmu .i ko klama .i mi prami do .i .au mi skicu fi le prenu noi ke'a fi do co'u morji ce'e fe le nu do ca'o renvi gi'e ca'o melbi ce'e fe le nu le risna be do ca'o ka'e prami ce'e fe le nu do badri gi'e se betri",
            &ParseOptions::default(),
        );
        let raw = format!("{:?}", parsed.parse_tree);
        assert!(raw.matches("TermsetGroup").count() >= 3);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn chrestomathy_forest_split_quote_rows_parse_when_combined() {
        parse_source(
            ".i fe lu .e'o sai doi do'u .e'o .e'o doi le ricfoi ninmu do'u .e'o mi catlu do cu pikci cusku fa mi .i ba bo go'i lu pu ki ca le po'o nai nu mi zvati le ckana be fi lo'e cifnu cu skicu fi mi fe lo zabna ranmi be do fe la'e lo se sanga poi jufra do .i je mi manci gi'e audji lo ka co'a zgana do .i mi ca le nu mi verba kei so'i roi ku ca lo nicte cu senva tu'a do fe lo nu do sanga fi mi fe lo jai se manci gi'e punji fi le stedu be mi fe lo xrula noi ja'e jadni ri\n.i ca le nu mi cilce verba be pu zi ku do ca'o raktu mi lo ka senva ma kau gi'e jai se senva mi fai lo nu do fagri gi'e kavbu gi'e jgari mi le ka se xance lo milxe glare kei tai lo nu do ralci gi'e milxe satre gi'e se panci lo ricfoi xrula gi'e vindu ja'e lo nu de'a sanji .i mi pu ta'e senva lo nu mi jersi do ije le risna be mi pu ku audji tu'a do gi'e prami do .i pu ta'e ku ca lo nicte mi di'a cikna tai lo da'i nu mi tirna lo nicte se sanga be do gi'e viska lo nu do vofli ni'a lei cizra tsani .i ku'i do .i do pu zvati ma ja'e lo nu mi tu'a do na ku ka'e ku viska gi'a tirna .i ba'e nau ku mi ta'e catlu le ricfoi gi'e zgana ri fau lo nu mi pacna gi'e djica lo nu mi cliva le cladu tcadu te zu'e lo nu mi klama gi'e penmi do li'u",
            &ParseOptions::default(),
        );
        parse_source(
            "lu .ia nai .i mi ba'o xlura ke ricfoi crida .i mi'a ba'o simxu lo ka kansa fi lo ka vofli bu'u lo ricfoi .i mi'a ba'o zukte lo ka gleki jinru lo ve'i rirxe .i mi'a ba'o cilce kelci ca lo nu le lunra cu te gusni .i mi'a ca cu spofu gi'e badri .i do'o pu lebna tu'a le citno dalgidva pe loi cmana zi'e noi se prami mi'a gi'e na'e dunku gi'e zifre .i le zgike poi sance lo flani pe le dalgidva pu je ca nai se minra fo le se stuzi be lo jbini be lo'i su'o cmana .i je le sance be le nu le dalgidva cu cinmo vasxu cu pu je ca nai se bevri ni'a le klina tsani ca lo nicte .i ba'o ku le dalgidva cu klaku fi tu'a mi'a gi'a senva tu'a mi'a gi'a zenba lo ka kandi ri'a tu'a mi'a\n.i do'o ne le za'u tcadu cu gasnu le cnino nabmi e le daspo be ge mi'a gi le dalgidva .i le dalgidva cu canci gi'e canci fau le nu ri te prina fi no da kei gi'e me le na'e cando virnu noi klama fo lu'i le foldi e le cmana fu lo ka se marce lo cilce xirma zi'e noi gasnu lo banli zi'e noi ta'e ku su'o me ke'a co'a morsi gi'a jinga .i nauku so'u roi ku su'o remna cu klama fo lu'i le klaji pe le ricfoi .i ro go'i cu ruble gi'e dunku gi'e du'e va'e pensi gi'e na'e cinmo gi'e to'e ckire gi'e badri .i le'e remna mo'u cliva mi'a gi'e na'e gleki fau le nu le nei na kansa mi'a .i le banli tcadu ku voi cpana le terdi cu cpana le spofu risna be lo remna .i le nurma tcadu cu simsa lo'e muzga be lo morsi .i bu'u le do'o banli malsi ba'o ku su'o da pikci .i mi pu prami le pa citno pe le cmana .i je ku'i ba bo le se go'i co'u prami mi gi'e cliva .i mi badri gi'e spofu .i ca le'e nicte e le'e donri mi klama fo lu'i le za'u ricfoi gi'e lausku le cmene be ra .i ku'i fliba .i le lastu flani be ra no roi se sance to'o su'o da li'u",
            &ParseOptions::default(),
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn chrestomathy_kubla_split_poem_rows_parse_when_combined() {
        parse_source(
            "la .alf. noi censa rirxe lei\nnoi so'i mei vau kevna fo",
            &ParseOptions::default(),
        );
        parse_source(
            ".uo li re pi'i mu se minli\nlei ferti dertu joi lei noi cinla\nvau korcu flecu joi lei purdi",
            &ParseOptions::default(),
        );
    }

    #[requires(!text.is_empty())]
    #[ensures(true)]
    fn free_word(text: &str) -> WithFreeModifiers<Token> {
        WithFreeModifiers::new(indicated_word(text), Vec::new())
    }

    #[requires(!text.is_empty())]
    #[ensures(true)]
    fn indicated_word(text: &str) -> Token {
        let mut words = segment_words_with_modifiers(text).expect("valid morphology");
        assert_eq!(words.len(), 1, "test helper expects one word");
        Token::bare(words.remove(0))
    }

    #[requires(!text.is_empty())]
    #[ensures(true)]
    fn single_bare_word(text: &str) -> Word {
        let mut words = segment_words_with_modifiers(text).expect("valid morphology");
        assert_eq!(words.len(), 1, "test helper expects one word");
        words
            .remove(0)
            .bare_word()
            .expect("test helper expects a bare word")
            .clone()
    }

    #[requires(true)]
    #[ensures(ret[0] <= ret[1])]
    fn warning_span(warning: &SyntaxWarning) -> [usize; 2] {
        let mut spans = warning.anchor.source_spans();
        spans.sort_by_key(|span| span.byte_start);
        let first = spans.first().expect("warning has source spans");
        let last = spans.last().expect("warning has source spans");
        [first.byte_start, last.byte_end]
    }

    #[requires(true)]
    #[ensures(true)]
    fn sample_subbridi() -> SubbridiSyntax {
        let words = segment_words_with_modifiers("mi klama").expect("valid morphology");
        let parsed = parse_syntax_tree(&words, &ParseOptions::default()).expect("valid syntax");
        let statement = parsed.parse_tree.paragraphs[0].statements[0]
            .statement
            .as_ref()
            .expect("parsed statement");
        let bridi = match statement.as_data() {
            data!(StatementSyntax::Bridi(bridi)) => *bridi.clone(),
            _ => panic!("test helper expected a bridi statement"),
        };
        new!(SubbridiSyntax::Bridi(Box::new(bridi)))
    }

    #[requires(true)]
    #[ensures(true)]
    fn sample_argument() -> SumtiSyntax {
        new!(SumtiSyntax::ProSumti(free_word("mi")))
    }

    #[requires(true)]
    #[ensures(true)]
    fn sample_relation() -> SelbriSyntax {
        new!(SelbriSyntax::SelbriWord(indicated_word("klama")))
    }

    #[requires(true)]
    #[ensures(true)]
    fn sample_predicate() -> BridiSyntax {
        let data!(SubbridiSyntax::Bridi(bridi)) = sample_subbridi().into_data() else {
            panic!("test helper expected a bridi subbridi");
        };
        *bridi
    }

    #[requires(true)]
    #[ensures(true)]
    fn sample_predicate_tail() -> BridiTailSyntax {
        let data!(BridiSyntax { bridi_tail, .. }) = sample_predicate().into_data();
        *bridi_tail
    }

    #[requires(true)]
    #[ensures(true)]
    fn sample_predicate_tail2() -> BoGroupedBridiTailSyntax {
        let bridi_tail = sample_predicate_tail();
        *bridi_tail.first.first
    }

    #[requires(true)]
    #[ensures(true)]
    fn sample_connective() -> ConnectiveSyntax {
        ConnectiveSyntax::new(
            ConnectiveKind::Afterthought,
            None,
            None,
            None,
            WithFreeModifiers::new(vec![indicated_word("je")], Vec::new()),
            None,
        )
    }

    #[requires(true)]
    #[ensures(true)]
    fn run_on_normal_stack(test: impl FnOnce()) {
        test();
    }

    #[requires(true)]
    #[ensures(true)]
    fn run_on_fixture_worker_stack(test: impl FnOnce() + Send + 'static) {
        let handle = std::thread::Builder::new()
            .stack_size(2 * 1024 * 1024)
            .spawn(test)
            .expect("fixture worker stack test thread should spawn");
        if let Err(panic) = handle.join() {
            std::panic::resume_unwind(panic);
        }
    }
}
