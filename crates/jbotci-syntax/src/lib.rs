//! Lojban syntax model and parser facade.

pub mod tree;
pub use tree::WithIndicators;

mod grammar;

extern crate self as jbotci_syntax;

use std::{cmp::Ordering, fmt};

#[allow(unused_imports)]
use bityzba::{data, ensures, expensive_invariant, invariant, new, requires};
use jbotci_diagnostics::{
    Diagnostic, DiagnosticLabel, DiagnosticNoteMode, DiagnosticPhase, DiagnosticSeverity,
    DiagnosticStyledNote, DiagnosticTextRole, DiagnosticTextSegment, source_span_from_byte_offsets,
};
pub use jbotci_diagnostics::{TraceFilter, TraceLevel, TraceOptions, TracePhase, TraceReport};
use jbotci_dialect::DialectDefinition;
use jbotci_morphology::{Cmavo, Selmaho, Word, WordLike};
use jbotci_source::SourceId;
use jbotci_tree::TreeVisitor;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub mod ast {
    pub use crate::grammar::ast::*;
}
use ast::{
    AtomRef as SyntaxAtomRef, NodeRef as SyntaxNodeRef, TextSyntax, TreeNode as SyntaxAstTreeNode,
};
pub use ast::{Indicator, IndicatorData};

pub const SYNTAX_TRACE_FILTERS: &[&str] = &[
    "text",
    "statement",
    "subsentence",
    "relation",
    "term",
    "argument",
    "free modifier",
    "token",
    "rewind",
];

impl TextSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_source_spans(&self, visitor: &mut impl FnMut(&jbotci_source::SourceSpan)) {
        let mut visitor = SourceSpanVisitor { visitor };
        self.visit_in_order(&mut visitor);
    }
}

#[invariant(true)]
struct SourceSpanVisitor<'callback> {
    visitor: &'callback mut dyn FnMut(&jbotci_source::SourceSpan),
}

impl fmt::Debug for SourceSpanVisitor<'_> {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SourceSpanVisitor")
            .finish_non_exhaustive()
    }
}

impl<'tree> TreeVisitor<'tree> for SourceSpanVisitor<'_> {
    type Node = SyntaxNodeRef<'tree>;
    type Atom = SyntaxAtomRef<'tree>;

    #[requires(true)]
    #[ensures(true)]
    fn visit_atom(&mut self, atom: Self::Atom) {
        match atom {
            SyntaxAtomRef::WithIndicatorsWordLike(word) => {
                for span in word.source_spans() {
                    (self.visitor)(span);
                }
            }
            SyntaxAtomRef::Word(word) => (self.visitor)(word.span()),
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn is_indicator_word(word: &Word) -> bool {
    word.cmavo().is_some_and(|cmavo| {
        cmavo.is_selmaho(Selmaho::Ui) || cmavo.is_selmaho(Selmaho::Cai) || cmavo == Cmavo::Y
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[invariant(true)]
pub struct ParseOptions {
    pub trace: TraceOptions,
    pub dialect: DialectDefinition,
}

#[derive(Debug, Clone)]
#[invariant(true)]
pub struct SyntaxParseAttempt {
    pub result: Result<SyntaxParse, SyntaxError>,
    pub trace: Option<TraceReport>,
}

impl ParseOptions {
    #[requires(true)]
    #[ensures(ret.dialect == *definition)]
    pub fn with_dialect_definition(mut self, definition: &DialectDefinition) -> Self {
        self.dialect = definition.clone();
        self
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn with_trace_options(mut self, trace: TraceOptions) -> Self {
        self.trace = trace;
        self
    }
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LojbanText {
    pub leading_nai: Vec<WithIndicators<WordLike>>,
    pub leading_cmevla: Vec<WithIndicators<WordLike>>,
    pub leading_indicators: Vec<Indicator>,
    pub leading_free_modifiers: Vec<FreeModifier>,
    pub leading_connective: Option<Connective>,
    pub paragraphs: Vec<Paragraph>,
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Paragraph {
    pub i: Option<WithIndicators<WordLike>>,
    pub niho: Vec<WithIndicators<WordLike>>,
    pub free_modifiers: Vec<FreeModifier>,
    pub statements: Vec<ParagraphStatement>,
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParagraphStatement {
    pub i: Option<WithIndicators<WordLike>>,
    pub connective: Option<Connective>,
    pub free_modifiers: Vec<FreeModifier>,
    pub statement: Option<Statement>,
}

#[invariant(true)]
#[invariant(::Fragment(_) => true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "fragment", rename_all = "kebab-case")]
pub enum Statement {
    Fragment(Fragment),
    Placeholder,
}

impl Statement {
    #[requires(true)]
    #[ensures(true)]
    pub fn fragment(fragment: Fragment) -> Self {
        Statement::Fragment(fragment)
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn placeholder() -> Self {
        Statement::Placeholder
    }
}

#[invariant(true)]
#[invariant(::Other(_) => true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "words", rename_all = "kebab-case")]
pub enum Fragment {
    Other(Vec<WithIndicators<WordLike>>),
}

impl Fragment {
    #[requires(true)]
    #[ensures(true)]
    pub fn other(words: Vec<WithIndicators<WordLike>>) -> Self {
        Fragment::Other(words)
    }
}

#[invariant(true)]
#[invariant(::Words(_) => true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "words", rename_all = "kebab-case")]
pub enum FreeModifier {
    Words(Vec<WithIndicators<WordLike>>),
}

impl FreeModifier {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(words: Vec<WithIndicators<WordLike>>) -> Self {
        FreeModifier::Words(words)
    }
}

#[invariant(true)]
#[invariant(::Words(_) => true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "words", rename_all = "kebab-case")]
pub enum Connective {
    Words(Vec<WithIndicators<WordLike>>),
}

impl Connective {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(words: Vec<WithIndicators<WordLike>>) -> Self {
        Connective::Words(words)
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[invariant(true)]
#[invariant(::Parse => true)]
pub enum SyntaxError {
    #[error("syntax parsing is not implemented yet")]
    NotImplemented,
    #[error("syntax parse failed at byte {byte_start}: {reason}")]
    Parse {
        byte_start: usize,
        byte_end: usize,
        reason: String,
        expected: Vec<String>,
        expectations: Vec<SyntaxExpectation>,
        context: Option<SyntaxConstructContext>,
    },
}

#[invariant(::Cmavo(cmavo) => !cmavo.canonical_text().is_empty())]
#[invariant(::Selmaho(selmaho) => !selmaho.name().is_empty())]
#[invariant(::WordCategory(category) => !category.display_name().is_empty())]
#[invariant(::Named(name) => !name.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SyntaxExpectedToken {
    Cmavo(Cmavo),
    Selmaho(Selmaho),
    WordCategory(SyntaxWordCategory),
    EndOfInput,
    Named(String),
}

impl SyntaxExpectedToken {
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn summary_text(&self) -> String {
        match self.as_data() {
            data!(SyntaxExpectedToken::Cmavo(cmavo)) => cmavo.canonical_text().to_owned(),
            data!(SyntaxExpectedToken::Selmaho(selmaho)) => selmaho.name().to_owned(),
            data!(SyntaxExpectedToken::WordCategory(category)) => {
                category.display_name().to_owned()
            }
            data!(SyntaxExpectedToken::EndOfInput) => "end of input".to_owned(),
            data!(SyntaxExpectedToken::Named(name)) => name.clone(),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn role(&self) -> DiagnosticTextRole {
        match self.as_data() {
            data!(SyntaxExpectedToken::Cmavo(_)) => DiagnosticTextRole::SpecificWord,
            data!(SyntaxExpectedToken::Selmaho(_)) => DiagnosticTextRole::Selmaho,
            data!(SyntaxExpectedToken::WordCategory(_)) => DiagnosticTextRole::WordCategory,
            data!(SyntaxExpectedToken::EndOfInput) | data!(SyntaxExpectedToken::Named(_)) => {
                DiagnosticTextRole::Plain
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SyntaxWordCategory {
    Brivla,
    Cmevla,
    RelationWord,
    KohaArgument,
    LetterWord,
    ReplacementWord,
    Quote,
}

impl SyntaxWordCategory {
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn display_name(self) -> &'static str {
        match self {
            Self::Brivla => "BRIVLA",
            Self::Cmevla => "CMEVLA",
            Self::RelationWord => "RELATION WORD",
            Self::KohaArgument => "KOhA ARGUMENT",
            Self::LetterWord => "LETTER WORD",
            Self::ReplacementWord => "REPLACEMENT WORD",
            Self::Quote => "QUOTE",
        }
    }
}

#[invariant(!construct.is_empty())]
#[invariant(byte_start <= byte_end)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SyntaxConstructContext {
    pub construct: String,
    pub byte_start: usize,
    pub byte_end: usize,
}

impl SyntaxConstructContext {
    #[requires(!construct.is_empty())]
    #[requires(byte_start <= byte_end)]
    #[ensures(true)]
    pub fn new(construct: String, byte_start: usize, byte_end: usize) -> Self {
        new!(SyntaxConstructContext {
            construct,
            byte_start,
            byte_end,
        })
    }
}

#[requires(!construct.is_empty())]
#[ensures(ret <= 6)]
pub(crate) fn syntax_construct_depth(construct: &str) -> usize {
    match construct {
        "free modifier" => 6,
        "argument" => 5,
        "term" => 4,
        "relation" => 3,
        "subsentence" => 2,
        "statement" => 1,
        "text" | "parse_text" | "end of input" | "syntax construct" => 0,
        _ => panic!("missing syntax diagnostic construct metadata for {construct:?}"),
    }
}

#[requires(!construct.is_empty())]
#[ensures(ret -> !construct.is_empty())]
pub(crate) fn syntax_construct_is_known(construct: &str) -> bool {
    matches!(
        construct,
        "free modifier"
            | "argument"
            | "term"
            | "relation"
            | "subsentence"
            | "statement"
            | "text"
            | "parse_text"
            | "end of input"
            | "syntax construct"
    )
}

#[requires(!construct.is_empty())]
#[ensures(ret == matches!(construct, "text" | "parse_text"))]
pub(crate) fn syntax_construct_is_root(construct: &str) -> bool {
    match construct {
        "text" | "parse_text" => true,
        "free modifier" | "argument" | "term" | "relation" | "subsentence" | "statement"
        | "end of input" | "syntax construct" => false,
        _ => panic!("missing syntax diagnostic construct metadata for {construct:?}"),
    }
}

#[invariant(::ContinueCurrent { construct } => !construct.is_empty())]
#[invariant(::StartNested { construct } => !construct.is_empty())]
#[invariant(::EndThenStart { starts, ends } => !starts.is_empty() && ends.iter().all(|construct| !construct.is_empty()))]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SyntaxExpectationReason {
    ContinueCurrent { construct: String },
    StartNested { construct: String },
    EndThenStart { starts: String, ends: Vec<String> },
}

impl SyntaxExpectationReason {
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn construct(&self) -> &str {
        match self.as_data() {
            data!(SyntaxExpectationReason::ContinueCurrent { construct })
            | data!(SyntaxExpectationReason::StartNested { construct }) => construct,
            data!(SyntaxExpectationReason::EndThenStart { starts, .. }) => starts,
        }
    }
}

#[invariant(!tokens.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SyntaxExpectation {
    pub tokens: Vec<SyntaxExpectedToken>,
    pub reason: SyntaxExpectationReason,
}

impl SyntaxExpectation {
    #[requires(!tokens.is_empty())]
    #[ensures(true)]
    pub fn new(tokens: Vec<SyntaxExpectedToken>, reason: SyntaxExpectationReason) -> Self {
        new!(SyntaxExpectation { tokens, reason })
    }
}

impl SyntaxError {
    #[requires(true)]
    #[ensures(!ret.code.is_empty())]
    pub fn to_diagnostic(&self, source_id: Option<SourceId>, source: &str) -> Diagnostic {
        match self {
            Self::NotImplemented => {
                let span = source_span_from_byte_offsets(source_id, source, 0, 0)
                    .expect("the start of a source string is always a valid source span");
                Diagnostic::new(
                    DiagnosticSeverity::Error,
                    DiagnosticPhase::Syntax,
                    "syntax.not-implemented".to_owned(),
                    "syntax parsing is not implemented yet".to_owned(),
                    vec![DiagnosticLabel::new(
                        span,
                        "syntax parser is unavailable".to_owned(),
                        true,
                    )],
                    Vec::new(),
                    None,
                )
            }
            Self::Parse {
                byte_start,
                byte_end,
                reason,
                expected,
                expectations,
                context,
            } => {
                let span = source_span_from_byte_offsets(
                    source_id.clone(),
                    source,
                    *byte_start,
                    *byte_end,
                )
                .expect("syntax errors store offsets derived from the same source text");
                let mut labels = vec![DiagnosticLabel::new(span, reason.clone(), true)];
                if let Some(context) = context {
                    let context_span = source_span_from_byte_offsets(
                        source_id,
                        source,
                        context.byte_start,
                        context.byte_end,
                    )
                    .expect("syntax contexts store offsets derived from the same source text");
                    labels.push(DiagnosticLabel::new(
                        context_span,
                        format!("while parsing {}", context.construct),
                        false,
                    ));
                }
                Diagnostic::new(
                    DiagnosticSeverity::Error,
                    DiagnosticPhase::Syntax,
                    "syntax.parse".to_owned(),
                    "syntax parse failed".to_owned(),
                    labels,
                    Vec::new(),
                    None,
                )
                .with_styled_notes(syntax_expected_notes(expected, expectations))
            }
        }
    }
}

#[requires(true)]
#[ensures(ret.iter().all(|note| !note.segments.is_empty()))]
fn syntax_expected_notes(
    expected: &[String],
    expectations: &[SyntaxExpectation],
) -> Vec<DiagnosticStyledNote> {
    let mut notes = Vec::new();
    if !expectations.is_empty() {
        notes.push(DiagnosticStyledNote::new(
            DiagnosticNoteMode::Summary,
            syntax_summary_segments_from_expectations(expectations),
        ));
        notes.push(DiagnosticStyledNote::new(
            DiagnosticNoteMode::Detailed,
            syntax_detailed_segments(expectations),
        ));
    } else if !expected.is_empty() {
        notes.push(DiagnosticStyledNote::new(
            DiagnosticNoteMode::Summary,
            syntax_summary_segments_from_strings(expected),
        ));
    }
    notes
}

#[requires(!expectations.is_empty())]
#[ensures(!ret.is_empty())]
fn syntax_summary_segments_from_expectations(
    expectations: &[SyntaxExpectation],
) -> Vec<DiagnosticTextSegment> {
    let mut tokens = Vec::<SyntaxExpectedToken>::new();
    for expectation in expectations {
        for token in &expectation.tokens {
            if !tokens.contains(token) {
                tokens.push(token.clone());
            }
        }
    }
    sort_syntax_tokens(&mut tokens);
    let mut segments = vec![plain_segment("expected one of: ")];
    push_comma_token_list(&mut segments, &tokens);
    segments
}

#[requires(!expected.is_empty())]
#[ensures(!ret.is_empty())]
fn syntax_summary_segments_from_strings(expected: &[String]) -> Vec<DiagnosticTextSegment> {
    let mut segments = vec![plain_segment("expected one of: ")];
    for (index, item) in expected.iter().enumerate() {
        if index > 0 {
            segments.push(punctuation_segment(", "));
        }
        segments.push(plain_segment(item));
    }
    segments
}

#[requires(!expectations.is_empty())]
#[ensures(!ret.is_empty())]
fn syntax_detailed_segments(expectations: &[SyntaxExpectation]) -> Vec<DiagnosticTextSegment> {
    let mut segments = vec![plain_segment("needs one of:")];
    let deduped = merge_expectations_by_reason(expectations);
    for expectation in &deduped {
        segments.push(plain_segment("\n"));
        segments.push(punctuation_segment("- "));
        push_expectation_segments(&mut segments, expectation);
    }
    segments
}

#[requires(!expectations.is_empty())]
#[ensures(ret.iter().all(|expectation| !expectation.tokens.is_empty()))]
fn merge_expectations_by_reason(expectations: &[SyntaxExpectation]) -> Vec<SyntaxExpectation> {
    let mut merged = Vec::<SyntaxExpectation>::new();
    for expectation in expectations {
        if let Some(existing) = merged
            .iter_mut()
            .find(|existing| existing.reason == expectation.reason)
        {
            let mut tokens = existing.tokens.clone();
            for token in &expectation.tokens {
                if !tokens.contains(token) {
                    tokens.push(token.clone());
                }
            }
            if tokens.len() != existing.tokens.len() {
                *existing = existing.clone().with_data(data! { tokens: tokens });
            }
        } else {
            merged.push(expectation.clone());
        }
    }
    for expectation in &mut merged {
        let mut tokens = expectation.tokens.clone();
        sort_syntax_tokens(&mut tokens);
        if tokens != expectation.tokens {
            *expectation = expectation.clone().with_data(data! { tokens: tokens });
        }
    }
    retain_innermost_continue_expectations(&mut merged);
    merged.sort_by(compare_syntax_expectations);
    merged
}

#[requires(true)]
#[ensures(expectations.iter().all(|expectation| !expectation.tokens.is_empty()))]
fn retain_innermost_continue_expectations(expectations: &mut Vec<SyntaxExpectation>) {
    let keep = expectations
        .iter()
        .enumerate()
        .map(|(index, expectation)| {
            !has_deeper_continue_with_same_tokens(index, expectation, expectations)
        })
        .collect::<Vec<_>>();
    let mut index = 0;
    expectations.retain(|_| {
        let keep_current = keep[index];
        index += 1;
        keep_current
    });
}

#[requires(index < expectations.len())]
#[requires(!expectation.tokens.is_empty())]
#[ensures(true)]
fn has_deeper_continue_with_same_tokens(
    index: usize,
    expectation: &SyntaxExpectation,
    expectations: &[SyntaxExpectation],
) -> bool {
    let Some(construct) = continue_current_construct(&expectation.reason) else {
        return false;
    };
    let depth = syntax_construct_depth(construct);
    expectations.iter().enumerate().any(|(other_index, other)| {
        other_index != index
            && other.tokens == expectation.tokens
            && continue_current_construct(&other.reason)
                .is_some_and(|other_construct| syntax_construct_depth(other_construct) > depth)
    })
}

#[requires(true)]
#[ensures(ret.is_none_or(|construct| !construct.is_empty()))]
fn continue_current_construct(reason: &SyntaxExpectationReason) -> Option<&str> {
    match reason.as_data() {
        data!(SyntaxExpectationReason::ContinueCurrent { construct }) => Some(construct),
        _ => None,
    }
}

#[requires(!expectation.tokens.is_empty())]
#[ensures(true)]
fn push_expectation_segments(
    segments: &mut Vec<DiagnosticTextSegment>,
    expectation: &SyntaxExpectation,
) {
    match expectation.reason.as_data() {
        data!(SyntaxExpectationReason::ContinueCurrent { construct }) => {
            push_token_list(segments, &expectation.tokens);
            segments.push(punctuation_segment(" ["));
            segments.push(keyword_segment("continues"));
            segments.push(punctuation_segment(" "));
            segments.push(construct_segment(construct));
            segments.push(punctuation_segment("]"));
        }
        data!(SyntaxExpectationReason::StartNested { construct }) => {
            segments.push(construct_segment(construct));
            if !token_list_redundantly_names_construct(construct, &expectation.tokens) {
                segments.push(punctuation_segment(" ("));
                push_token_list(segments, &expectation.tokens);
                segments.push(punctuation_segment(")"));
            }
        }
        data!(SyntaxExpectationReason::EndThenStart { starts, ends }) => {
            segments.push(construct_segment(starts));
            if !token_list_redundantly_names_construct(starts, &expectation.tokens) {
                segments.push(punctuation_segment(" ("));
                push_token_list(segments, &expectation.tokens);
                segments.push(punctuation_segment(")"));
            }
            if !ends.is_empty() {
                segments.push(punctuation_segment(" ["));
                segments.push(keyword_segment("ends"));
                segments.push(punctuation_segment(" "));
                push_construct_list(segments, ends);
                segments.push(punctuation_segment("]"));
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn compare_syntax_expectations(left: &SyntaxExpectation, right: &SyntaxExpectation) -> Ordering {
    let depth_order =
        syntax_reason_sort_depth(&right.reason).cmp(&syntax_reason_sort_depth(&left.reason));
    if depth_order != Ordering::Equal {
        return depth_order;
    }

    let reason_order =
        syntax_reason_sort_order(&left.reason).cmp(&syntax_reason_sort_order(&right.reason));
    if reason_order != Ordering::Equal {
        return reason_order;
    }

    let construct_order =
        syntax_reason_sort_construct(&left.reason).cmp(syntax_reason_sort_construct(&right.reason));
    if construct_order != Ordering::Equal {
        return construct_order;
    }

    let end_order = syntax_reason_ends(&left.reason).cmp(syntax_reason_ends(&right.reason));
    if end_order != Ordering::Equal {
        return end_order;
    }

    compare_syntax_token_slices(&left.tokens, &right.tokens)
}

#[requires(true)]
#[ensures(true)]
fn syntax_reason_sort_depth(reason: &SyntaxExpectationReason) -> usize {
    syntax_construct_depth(syntax_reason_sort_construct(reason))
}

#[requires(true)]
#[ensures(ret <= 2)]
fn syntax_reason_sort_order(reason: &SyntaxExpectationReason) -> u8 {
    match reason.as_data() {
        data!(SyntaxExpectationReason::ContinueCurrent { .. }) => 0,
        data!(SyntaxExpectationReason::StartNested { .. }) => 1,
        data!(SyntaxExpectationReason::EndThenStart { .. }) => 2,
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn syntax_reason_sort_construct(reason: &SyntaxExpectationReason) -> &str {
    match reason.as_data() {
        data!(SyntaxExpectationReason::ContinueCurrent { construct })
        | data!(SyntaxExpectationReason::StartNested { construct }) => construct,
        data!(SyntaxExpectationReason::EndThenStart { starts, .. }) => starts,
    }
}

#[requires(true)]
#[ensures(true)]
fn syntax_reason_ends(reason: &SyntaxExpectationReason) -> &[String] {
    match reason.as_data() {
        data!(SyntaxExpectationReason::EndThenStart { ends, .. }) => ends,
        _ => &[],
    }
}

#[requires(true)]
#[ensures(true)]
fn compare_syntax_token_slices(
    left: &[SyntaxExpectedToken],
    right: &[SyntaxExpectedToken],
) -> Ordering {
    left.iter()
        .zip(right)
        .map(|(left, right)| compare_syntax_expected_tokens(left, right))
        .find(|order| *order != Ordering::Equal)
        .unwrap_or_else(|| left.len().cmp(&right.len()))
}

#[requires(true)]
#[ensures(true)]
fn sort_syntax_tokens(tokens: &mut [SyntaxExpectedToken]) {
    tokens.sort_by(compare_syntax_expected_tokens);
}

#[requires(true)]
#[ensures(true)]
fn compare_syntax_expected_tokens(
    left: &SyntaxExpectedToken,
    right: &SyntaxExpectedToken,
) -> Ordering {
    syntax_expected_token_sort_category(left)
        .cmp(&syntax_expected_token_sort_category(right))
        .then_with(|| {
            syntax_expected_token_sort_text(left).cmp(syntax_expected_token_sort_text(right))
        })
}

#[requires(true)]
#[ensures(ret <= 4)]
fn syntax_expected_token_sort_category(token: &SyntaxExpectedToken) -> u8 {
    match token.as_data() {
        data!(SyntaxExpectedToken::WordCategory(_)) => 0,
        data!(SyntaxExpectedToken::Selmaho(_)) => 1,
        data!(SyntaxExpectedToken::Cmavo(_)) => 2,
        data!(SyntaxExpectedToken::EndOfInput) => 3,
        data!(SyntaxExpectedToken::Named(_)) => 4,
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn syntax_expected_token_sort_text(token: &SyntaxExpectedToken) -> &str {
    match token.as_data() {
        data!(SyntaxExpectedToken::Cmavo(cmavo)) => cmavo.canonical_text(),
        data!(SyntaxExpectedToken::Selmaho(selmaho)) => selmaho.name(),
        data!(SyntaxExpectedToken::WordCategory(category)) => category.display_name(),
        data!(SyntaxExpectedToken::EndOfInput) => "end of input",
        data!(SyntaxExpectedToken::Named(name)) => name,
    }
}

#[requires(!construct.is_empty())]
#[requires(!tokens.is_empty())]
#[ensures(ret -> tokens.len() == 1)]
fn token_list_redundantly_names_construct(construct: &str, tokens: &[SyntaxExpectedToken]) -> bool {
    construct == "end of input"
        && matches!(
            tokens,
            [token] if matches!(token.as_data(), data!(SyntaxExpectedToken::EndOfInput))
        )
}

#[requires(!tokens.is_empty())]
#[ensures(true)]
fn push_token_list(segments: &mut Vec<DiagnosticTextSegment>, tokens: &[SyntaxExpectedToken]) {
    for (index, token) in tokens.iter().enumerate() {
        if index > 0 {
            if index + 1 == tokens.len() {
                segments.push(punctuation_segment(" or "));
            } else {
                segments.push(punctuation_segment(", "));
            }
        }
        segments.push(DiagnosticTextSegment::new(
            token.role(),
            token.summary_text(),
        ));
    }
}

#[requires(!tokens.is_empty())]
#[ensures(true)]
fn push_comma_token_list(
    segments: &mut Vec<DiagnosticTextSegment>,
    tokens: &[SyntaxExpectedToken],
) {
    for (index, token) in tokens.iter().enumerate() {
        if index > 0 {
            segments.push(punctuation_segment(", "));
        }
        segments.push(DiagnosticTextSegment::new(
            token.role(),
            token.summary_text(),
        ));
    }
}

#[requires(!constructs.is_empty())]
#[ensures(true)]
fn push_construct_list(segments: &mut Vec<DiagnosticTextSegment>, constructs: &[String]) {
    for (index, construct) in constructs.iter().enumerate() {
        if index > 0 {
            if index + 1 == constructs.len() {
                segments.push(punctuation_segment(" or "));
            } else {
                segments.push(punctuation_segment(", "));
            }
        }
        segments.push(construct_segment(construct));
    }
}

#[requires(!text.is_empty())]
#[ensures(ret.text == text)]
fn plain_segment(text: &str) -> DiagnosticTextSegment {
    DiagnosticTextSegment::new(DiagnosticTextRole::Plain, text.to_owned())
}

#[requires(!text.is_empty())]
#[ensures(ret.text == text)]
fn punctuation_segment(text: &str) -> DiagnosticTextSegment {
    DiagnosticTextSegment::new(DiagnosticTextRole::Punctuation, text.to_owned())
}

#[requires(!text.is_empty())]
#[ensures(ret.text == text)]
fn keyword_segment(text: &str) -> DiagnosticTextSegment {
    DiagnosticTextSegment::new(DiagnosticTextRole::Keyword, text.to_owned())
}

#[requires(!text.is_empty())]
#[ensures(ret.text == text)]
fn construct_segment(text: &str) -> DiagnosticTextSegment {
    DiagnosticTextSegment::new(DiagnosticTextRole::Construct, text.to_owned())
}

#[requires(true)]
#[ensures(true)]
pub fn parse_text(words: &[WordLike], options: &ParseOptions) -> Result<LojbanText, SyntaxError> {
    grammar::parse_text(words, options)
}

#[requires(true)]
#[ensures(true)]
pub fn parse_raw_text(
    words: &[WordLike],
    options: &ParseOptions,
) -> Result<ast::TextSyntax, SyntaxError> {
    grammar::parse_raw_text(words, options)
}

#[cfg(feature = "grammar-debug")]
#[requires(true)]
#[ensures(!ret.is_empty())]
pub fn syntax_grammar_ebnf(options: &ParseOptions) -> String {
    grammar::syntax_grammar_ebnf(options)
}

#[cfg(feature = "grammar-debug")]
#[requires(true)]
#[ensures(!ret.is_empty())]
pub fn syntax_grammar_svg(options: &ParseOptions) -> String {
    grammar::syntax_grammar_svg(options)
}

#[invariant(warnings.iter().all(|warning| !warning.anchor.source_spans().is_empty()))]
#[expensive_invariant({
    let mut last_end = None;
    let mut ordered = true;
    parse_tree.visit_source_spans(&mut |span| {
        if !ordered {
            return;
        }
        if last_end.is_some_and(|end| end > span.byte_start) {
            ordered = false;
            return;
        }
        last_end = Some(span.byte_end);
    });
    ordered
})]
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SyntaxParse {
    pub parse_tree: Box<TextSyntax>,
    #[serde(default)]
    pub warnings: Vec<SyntaxWarning>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ExperimentalConstruct {
    ExperimentalCmavo,
    ExperimentalZohOiQuote,
    ExperimentalMehOiRelationUnit,
    ExperimentalLohOiBridiDescription,
    ExperimentalLohAiReplacementFree,
    ExperimentalJacuPredicateTailConnective,
    ExperimentalJeIStatementConnective,
    ExperimentalMultipleNaFragment,
    ExperimentalEmptyPrenex,
    ExperimentalBareCuPredicate,
    ExperimentalNaheArgumentWithoutBo,
    ExperimentalVuhoScopedAttachment,
    ExperimentalNohoiSelbriRelativeClause,
    ExperimentalSimplerSumtiConnective,
    ExperimentalExplicitCuPredicateTailStarter,
    ExperimentalRelativeClauseConnective,
    ExperimentalSimplerForethoughtConnective,
    ExperimentalSimplerTermConnective,
    ExperimentalSimplerMexOperandConnective,
    ExperimentalSimplerDescriptorHeadConnective,
    ExperimentalJiAsJaConnective,
    ExperimentalGadganzuGadri,
    ExperimentalIauReset,
    ExperimentalGohoiRelationUnit,
    ExperimentalKeTermset,
    ExperimentalLaheNaheTermWrapper,
    ExperimentalForethoughtRelativeClauseConnective,
    ExperimentalBroadAConnective,
    ExperimentalVuhuConnective,
    ExperimentalNahuPredicateConnective,
    ExperimentalFaAsTag,
    ExperimentalFlattenedTag,
    ExperimentalCbmCmevlaRelationWord,
    ExperimentalCbmLaNameAsDescriptor,
    ExperimentalDictionaryDoiVocative,
    ExperimentalDictionaryCoiVocative,
    ExperimentalDictionarySeiFreeModifier,
    ExperimentalDictionaryPaNumber,
    ExperimentalDictionaryFahaTag,
    ExperimentalDictionaryUiIndicator,
    ExperimentalNoihaAdverbial,
    ExperimentalFihoiAdverbial,
    ExperimentalSoiAdverbial,
    ExperimentalPreposedLinkargs,
    ExperimentalEmptyLinkargs,
    ExperimentalBroadBoStatementConnective,
    ExperimentalBroadKePredicateContinuation,
    ExperimentalTermHierarchyBoConnection,
    ExperimentalBareNaTerm,
    ExperimentalXohiTagRelation,
    ExperimentalZantufaCmavo,
    ExperimentalZantufaForethoughtGihi,
    ExperimentalZantufaGek,
    ExperimentalZantufaPoihaBrigahi,
    ExperimentalZantufaJaiTagTerm,
    ExperimentalZantufaRecursiveTag,
    ExperimentalZantufaMuhoiRelationUnit,
    ExperimentalZantufaLuheiRelationUnit,
    CllProhibitedFreeModifierPlacement,
}

impl ExperimentalConstruct {
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub const fn code(self) -> &'static str {
        match self {
            Self::ExperimentalCmavo => "syntax.warning.experimental-cmavo",
            Self::ExperimentalZohOiQuote => "syntax.warning.experimental-zoh-oi-quote",
            Self::ExperimentalMehOiRelationUnit => {
                "syntax.warning.experimental-meh-oi-relation-unit"
            }
            Self::ExperimentalLohOiBridiDescription => {
                "syntax.warning.experimental-loh-oi-bridi-description"
            }
            Self::ExperimentalLohAiReplacementFree => {
                "syntax.warning.experimental-loh-ai-replacement-free"
            }
            Self::ExperimentalJacuPredicateTailConnective => {
                "syntax.warning.experimental-jacu-predicate-tail-connective"
            }
            Self::ExperimentalJeIStatementConnective => {
                "syntax.warning.experimental-je-i-statement-connective"
            }
            Self::ExperimentalMultipleNaFragment => {
                "syntax.warning.experimental-multiple-na-fragment"
            }
            Self::ExperimentalEmptyPrenex => "syntax.warning.experimental-empty-prenex",
            Self::ExperimentalBareCuPredicate => "syntax.warning.experimental-bare-cu-predicate",
            Self::ExperimentalNaheArgumentWithoutBo => {
                "syntax.warning.experimental-nahe-argument-without-bo"
            }
            Self::ExperimentalVuhoScopedAttachment => {
                "syntax.warning.experimental-vuho-scoped-attachment"
            }
            Self::ExperimentalNohoiSelbriRelativeClause => {
                "syntax.warning.experimental-nohoi-selbri-relative-clause"
            }
            Self::ExperimentalSimplerSumtiConnective => {
                "syntax.warning.experimental-simpler-sumti-connective"
            }
            Self::ExperimentalExplicitCuPredicateTailStarter => {
                "syntax.warning.experimental-explicit-cu-predicate-tail-starter"
            }
            Self::ExperimentalRelativeClauseConnective => {
                "syntax.warning.experimental-relative-clause-connective"
            }
            Self::ExperimentalSimplerForethoughtConnective => {
                "syntax.warning.experimental-simpler-forethought-connective"
            }
            Self::ExperimentalSimplerTermConnective => {
                "syntax.warning.experimental-simpler-term-connective"
            }
            Self::ExperimentalSimplerMexOperandConnective => {
                "syntax.warning.experimental-simpler-mex-operand-connective"
            }
            Self::ExperimentalSimplerDescriptorHeadConnective => {
                "syntax.warning.experimental-simpler-descriptor-head-connective"
            }
            Self::ExperimentalJiAsJaConnective => "syntax.warning.experimental-ji-as-ja-connective",
            Self::ExperimentalGadganzuGadri => "syntax.warning.experimental-gadganzu-gadri",
            Self::ExperimentalIauReset => "syntax.warning.experimental-iau-reset",
            Self::ExperimentalGohoiRelationUnit => {
                "syntax.warning.experimental-gohoi-relation-unit"
            }
            Self::ExperimentalKeTermset => "syntax.warning.experimental-ke-termset",
            Self::ExperimentalLaheNaheTermWrapper => {
                "syntax.warning.experimental-lahe-nahe-term-wrapper"
            }
            Self::ExperimentalForethoughtRelativeClauseConnective => {
                "syntax.warning.experimental-forethought-relative-clause-connective"
            }
            Self::ExperimentalBroadAConnective => "syntax.warning.experimental-broad-a-connective",
            Self::ExperimentalVuhuConnective => "syntax.warning.experimental-vuhu-connective",
            Self::ExperimentalNahuPredicateConnective => {
                "syntax.warning.experimental-nahu-predicate-connective"
            }
            Self::ExperimentalFaAsTag => "syntax.warning.experimental-fa-as-tag",
            Self::ExperimentalFlattenedTag => "syntax.warning.experimental-flattened-tag",
            Self::ExperimentalCbmCmevlaRelationWord => {
                "syntax.warning.experimental-cbm-cmevla-relation-word"
            }
            Self::ExperimentalCbmLaNameAsDescriptor => {
                "syntax.warning.experimental-cbm-la-name-as-descriptor"
            }
            Self::ExperimentalDictionaryDoiVocative => {
                "syntax.warning.experimental-dictionary-doi-vocative"
            }
            Self::ExperimentalDictionaryCoiVocative => {
                "syntax.warning.experimental-dictionary-coi-vocative"
            }
            Self::ExperimentalDictionarySeiFreeModifier => {
                "syntax.warning.experimental-dictionary-sei-free-modifier"
            }
            Self::ExperimentalDictionaryPaNumber => {
                "syntax.warning.experimental-dictionary-pa-number"
            }
            Self::ExperimentalDictionaryFahaTag => {
                "syntax.warning.experimental-dictionary-faha-tag"
            }
            Self::ExperimentalDictionaryUiIndicator => {
                "syntax.warning.experimental-dictionary-ui-indicator"
            }
            Self::ExperimentalNoihaAdverbial => "syntax.warning.experimental-noiha-adverbial",
            Self::ExperimentalFihoiAdverbial => "syntax.warning.experimental-fihoi-adverbial",
            Self::ExperimentalSoiAdverbial => "syntax.warning.experimental-soi-adverbial",
            Self::ExperimentalPreposedLinkargs => "syntax.warning.experimental-preposed-linkargs",
            Self::ExperimentalEmptyLinkargs => "syntax.warning.experimental-empty-linkargs",
            Self::ExperimentalBroadBoStatementConnective => {
                "syntax.warning.experimental-broad-bo-statement-connective"
            }
            Self::ExperimentalBroadKePredicateContinuation => {
                "syntax.warning.experimental-broad-ke-predicate-continuation"
            }
            Self::ExperimentalTermHierarchyBoConnection => {
                "syntax.warning.experimental-term-hierarchy-bo-connection"
            }
            Self::ExperimentalBareNaTerm => "syntax.warning.experimental-bare-na-term",
            Self::ExperimentalXohiTagRelation => "syntax.warning.experimental-xohi-tag-relation",
            Self::ExperimentalZantufaCmavo => "syntax.warning.experimental-zantufa-cmavo",
            Self::ExperimentalZantufaForethoughtGihi => {
                "syntax.warning.experimental-zantufa-forethought-gihi"
            }
            Self::ExperimentalZantufaGek => "syntax.warning.experimental-zantufa-gek",
            Self::ExperimentalZantufaPoihaBrigahi => {
                "syntax.warning.experimental-zantufa-poiha-brigahi"
            }
            Self::ExperimentalZantufaJaiTagTerm => {
                "syntax.warning.experimental-zantufa-jai-tag-term"
            }
            Self::ExperimentalZantufaRecursiveTag => {
                "syntax.warning.experimental-zantufa-recursive-tag"
            }
            Self::ExperimentalZantufaMuhoiRelationUnit => {
                "syntax.warning.experimental-zantufa-muhoi-relation-unit"
            }
            Self::ExperimentalZantufaLuheiRelationUnit => {
                "syntax.warning.experimental-zantufa-luhei-relation-unit"
            }
            Self::CllProhibitedFreeModifierPlacement => {
                "syntax.warning.cll-prohibited-free-modifier-placement"
            }
        }
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub const fn message(self) -> &'static str {
        match self {
            Self::ExperimentalCmavo => "experimental cmavo",
            Self::ExperimentalZohOiQuote => "ZOhOI single-word foreign quote",
            Self::ExperimentalMehOiRelationUnit => "MEhOI stage-0 fu'ivla relation unit",
            Self::ExperimentalLohOiBridiDescription => "LOhOI/KUhAU bridi description sumti",
            Self::ExperimentalLohAiReplacementFree => "LOhAI/LEhAI replacement free modifier",
            Self::ExperimentalJacuPredicateTailConnective => {
                "JA/JOI connective used in a bridi-tail connective slot"
            }
            Self::ExperimentalJeIStatementConnective => {
                "JA/JOI connective used before statement separator I"
            }
            Self::ExperimentalMultipleNaFragment => "multiple NA fragment sequence",
            Self::ExperimentalEmptyPrenex => "empty prenex",
            Self::ExperimentalBareCuPredicate => "bare CU before the main selbri",
            Self::ExperimentalNaheArgumentWithoutBo => "NAhE before sumti without BO",
            Self::ExperimentalVuhoScopedAttachment => "VUhO scoped attachment enhancement",
            Self::ExperimentalNohoiSelbriRelativeClause => "NOhOI/KUhOI selbri relative clause",
            Self::ExperimentalSimplerSumtiConnective => {
                "JA connective used in an argument connective slot"
            }
            Self::ExperimentalExplicitCuPredicateTailStarter => {
                "explicit CU before the right side of a bridi-tail connective"
            }
            Self::ExperimentalRelativeClauseConnective => {
                "JA/JOI connective used between relative clauses"
            }
            Self::ExperimentalSimplerForethoughtConnective => {
                "simpler binary forethought connective form"
            }
            Self::ExperimentalSimplerTermConnective => "JA connective used directly between terms",
            Self::ExperimentalSimplerMexOperandConnective => {
                "JA connective used between MEX operands"
            }
            Self::ExperimentalSimplerDescriptorHeadConnective => {
                "JA connective used between descriptor heads"
            }
            Self::ExperimentalJiAsJaConnective => "JI used as an experimental JA-family connective",
            Self::ExperimentalGadganzuGadri => "gadganzu article",
            Self::ExperimentalIauReset => "IhAU bridi-level reset",
            Self::ExperimentalGohoiRelationUnit => "GOhOI pro-bridi word quote",
            Self::ExperimentalKeTermset => "KE/KEhE termset grouping",
            Self::ExperimentalLaheNaheTermWrapper => "LAhE/NAhE term wrapper",
            Self::ExperimentalForethoughtRelativeClauseConnective => {
                "forethought connective used between relative clauses"
            }
            Self::ExperimentalBroadAConnective => {
                "A-family connective used in a broader connective-family slot"
            }
            Self::ExperimentalVuhuConnective => "VUhU used as a non-MEX connective",
            Self::ExperimentalNahuPredicateConnective => "NAhU/ji'oi predicate-to-connective form",
            Self::ExperimentalFaAsTag => "FA place tag used as a tag/stag atom",
            Self::ExperimentalFlattenedTag => "experimental flattened tag form",
            Self::ExperimentalCbmCmevlaRelationWord => "CBM cmevla used as a relation word",
            Self::ExperimentalCbmLaNameAsDescriptor => "CBM LA name form parsed as a descriptor",
            Self::ExperimentalDictionaryDoiVocative => {
                "dictionary-first DOI experimental vocative/attribution cmavo"
            }
            Self::ExperimentalDictionaryCoiVocative => {
                "dictionary-first COI experimental vocative cmavo"
            }
            Self::ExperimentalDictionarySeiFreeModifier => {
                "dictionary-first SEI-style experimental free modifier"
            }
            Self::ExperimentalDictionaryPaNumber => "dictionary-first PA experimental number word",
            Self::ExperimentalDictionaryFahaTag => "dictionary-first FAhA experimental spatial tag",
            Self::ExperimentalDictionaryUiIndicator => {
                "dictionary-first UI3a experimental indicator"
            }
            Self::ExperimentalNoihaAdverbial => "NOIhA adverbial relative-clause term",
            Self::ExperimentalFihoiAdverbial => "FIhOI bridi/subsentence adverbial term",
            Self::ExperimentalSoiAdverbial => "SOI/XOI bridi/subsentence adverbial term",
            Self::ExperimentalPreposedLinkargs => "BE linkargs before a relation unit",
            Self::ExperimentalEmptyLinkargs => "empty BE/BEI linkarg slot",
            Self::ExperimentalBroadBoStatementConnective => {
                "broad connective with BO in a statement/subsentence continuation"
            }
            Self::ExperimentalBroadKePredicateContinuation => {
                "broad connective with KE/KEhE in a predicate/subsentence continuation"
            }
            Self::ExperimentalTermHierarchyBoConnection => {
                "experimental term-hierarchy BO connection"
            }
            Self::ExperimentalBareNaTerm => "bare NA term/adverbial without KU",
            Self::ExperimentalXohiTagRelation => "XOhI tag-to-relation conversion",
            Self::ExperimentalZantufaCmavo => "Zantufa experimental cmavo classification",
            Self::ExperimentalZantufaForethoughtGihi => "Zantufa GIhI forethought-chain terminator",
            Self::ExperimentalZantufaGek => "Zantufa forethought connective form",
            Self::ExperimentalZantufaPoihaBrigahi => {
                "Zantufa POIhA briga'i term with KU terminator"
            }
            Self::ExperimentalZantufaJaiTagTerm => "Zantufa JAI tag term",
            Self::ExperimentalZantufaRecursiveTag => "Zantufa recursive SE/NAhE tag prefix",
            Self::ExperimentalZantufaMuhoiRelationUnit => {
                "Zantufa MUhOI delimited foreign relation unit"
            }
            Self::ExperimentalZantufaLuheiRelationUnit => "Zantufa LUhEI/LIhAU text relation unit",
            Self::CllProhibitedFreeModifierPlacement => {
                "free modifier placement prohibited by CLL grammar"
            }
        }
    }
}

#[invariant(!anchor.source_spans().is_empty())]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxWarning {
    pub kind: ExperimentalConstruct,
    pub anchor_index: usize,
    pub anchor: WithIndicators<WordLike>,
}

impl SyntaxWarning {
    #[requires(true)]
    #[ensures(true)]
    pub fn experimental_construct(
        construct: ExperimentalConstruct,
        anchor_index: usize,
        anchor: WithIndicators<WordLike>,
    ) -> Self {
        new!(SyntaxWarning {
            kind: construct,
            anchor_index: anchor_index,
            anchor: anchor,
        })
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn message(&self) -> &'static str {
        self.kind.message()
    }

    #[requires(true)]
    #[ensures(!ret.code.is_empty())]
    pub fn to_diagnostic(&self, source_id: Option<SourceId>, source: &str) -> Diagnostic {
        let (byte_start, byte_end) = warning_byte_selection(self);
        let span = source_span_from_byte_offsets(source_id, source, byte_start, byte_end)
            .expect("syntax warnings store offsets derived from the same source text");
        let message = warning_message(self);
        Diagnostic::new(
            DiagnosticSeverity::Warning,
            DiagnosticPhase::Syntax,
            self.kind.code().to_owned(),
            format!("experimental syntax: {message}"),
            vec![DiagnosticLabel::new(span, message, true)],
            Vec::new(),
            Some(self.anchor_index),
        )
    }
}

#[requires(true)]
#[ensures(ret.0 <= ret.1)]
fn warning_byte_selection(warning: &SyntaxWarning) -> (usize, usize) {
    let mut spans = warning.anchor.source_spans();
    spans.sort_by_key(|span| span.byte_start);
    let Some(first) = spans.first() else {
        return (0, 0);
    };
    let last = spans.last().expect("first span exists");
    (first.byte_start, last.byte_end)
}

#[invariant(!source_label.is_empty())]
#[invariant(!message.is_empty())]
#[invariant(*line > 0)]
#[invariant(*column > 0)]
#[invariant(!context.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxWarningDisplay {
    pub source_label: String,
    pub kind: ExperimentalConstruct,
    pub message: String,
    pub line: usize,
    pub column: usize,
    pub selection_start: usize,
    pub selection_length: usize,
    pub experimental_cmavo: Option<String>,
    pub context: String,
}

#[requires(!source_label.is_empty())]
#[ensures(ret.len() == warnings.len())]
pub fn syntax_warning_displays(
    source_label: &str,
    source: &str,
    words: &[WithIndicators<WordLike>],
    warnings: &[SyntaxWarning],
) -> Vec<SyntaxWarningDisplay> {
    warnings
        .iter()
        .map(|warning| syntax_warning_display(source_label, source, words, warning))
        .collect()
}

#[requires(!source_label.is_empty())]
#[ensures(!ret.source_label.is_empty())]
pub fn syntax_warning_display(
    source_label: &str,
    source: &str,
    words: &[WithIndicators<WordLike>],
    warning: &SyntaxWarning,
) -> SyntaxWarningDisplay {
    let (selection_start, selection_length) = warning_selection(warning);
    let (line, column) = char_offset_to_line_column(source, selection_start);
    let experimental_cmavo = experimental_cmavo_text(warning);
    let message = warning_message(warning);
    new!(SyntaxWarningDisplay {
        source_label: source_label.to_owned(),
        kind: warning.kind,
        message: message,
        line: line,
        column: column,
        selection_start: selection_start,
        selection_length: selection_length,
        experimental_cmavo: experimental_cmavo,
        context: warning_context(words, warning.anchor_index),
    })
}

#[requires(true)]
#[ensures(ret.0 >= 1 && ret.1 >= 1)]
fn char_offset_to_line_column(source: &str, char_offset: usize) -> (usize, usize) {
    let mut line = 1usize;
    let mut column = 1usize;
    for (index, ch) in source.chars().enumerate() {
        if index == char_offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            column = 1;
        } else {
            column += 1;
        }
    }
    (line, column)
}

#[requires(true)]
#[ensures(true)]
fn warning_selection(warning: &SyntaxWarning) -> (usize, usize) {
    let mut spans = warning.anchor.source_spans();
    spans.sort_by_key(|span| span.char_start);
    let Some(first) = spans.first() else {
        return (0, 0);
    };
    let last = spans.last().expect("first span exists");
    (
        first.char_start,
        last.char_end.saturating_sub(first.char_start),
    )
}

#[requires(true)]
#[ensures(true)]
fn experimental_cmavo_text(warning: &SyntaxWarning) -> Option<String> {
    if warning.kind == ExperimentalConstruct::ExperimentalCmavo {
        return warning
            .anchor
            .core_word()
            .bare_word()
            .map(jbotci_morphology::Word::canonical_phonemes)
            .filter(|text| !text.trim().is_empty());
    }
    None
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn warning_message(warning: &SyntaxWarning) -> String {
    experimental_cmavo_text(warning).map_or_else(
        || warning.message().to_owned(),
        |cmavo| format!("{}: {cmavo}", warning.message()),
    )
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn warning_context(words: &[WithIndicators<WordLike>], index: usize) -> String {
    let before_all = words.get(..index).unwrap_or(words);
    let before_count = before_all.len().min(3);
    let before = &before_all[before_all.len().saturating_sub(before_count)..];
    let after = if index + 1 < words.len() {
        &words[index + 1..words.len().min(index + 4)]
    } else {
        &[]
    };
    let mut parts = Vec::new();
    parts.extend(before.iter().map(warning_word_text));
    let current = words.get(index).map_or_else(
        || "👉<EOF>".to_owned(),
        |word| format!("👉{}", warning_word_text(word)),
    );
    parts.push(current);
    parts.extend(after.iter().map(warning_word_text));
    let prefix = if index > 3 { "… " } else { "" };
    let suffix = if words.len().saturating_sub(index + 1) > 3 {
        " …"
    } else {
        ""
    };
    format!("{prefix}@ {index}: {}{suffix}", parts.join(" "))
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn warning_word_text(word: &WithIndicators<WordLike>) -> String {
    format!("{word}")
}

#[requires(true)]
#[ensures(true)]
pub fn parse_syntax_tree(words: &[WordLike]) -> Result<SyntaxParse, SyntaxError> {
    parse_syntax_tree_with_options(words, &ParseOptions::default())
}

#[requires(true)]
#[ensures(true)]
pub fn parse_syntax_tree_with_options(
    words: &[WordLike],
    options: &ParseOptions,
) -> Result<SyntaxParse, SyntaxError> {
    grammar::parse_syntax_tree(words, options)
}

#[requires(true)]
#[ensures(true)]
pub fn parse_syntax_tree_with_source_and_options(
    words: &[WordLike],
    source: &str,
    options: &ParseOptions,
) -> Result<SyntaxParse, SyntaxError> {
    grammar::parse_syntax_tree_with_source(words, Some(source), options)
}

#[requires(true)]
#[ensures(true)]
pub fn parse_syntax_tree_with_source_and_options_attempt(
    words: &[WordLike],
    source: &str,
    options: &ParseOptions,
) -> SyntaxParseAttempt {
    grammar::parse_syntax_tree_with_source_attempt(words, Some(source), options)
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use bityzba::{data, ensures, new, requires};

    use super::*;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn syntax_expected_tokens_sort_by_category_then_text() {
        let mut tokens = vec![
            new!(SyntaxExpectedToken::Named("input".to_owned())),
            new!(SyntaxExpectedToken::Cmavo(Cmavo::Lo)),
            new!(SyntaxExpectedToken::EndOfInput),
            new!(SyntaxExpectedToken::Selmaho(Selmaho::Gaho)),
            new!(SyntaxExpectedToken::Cmavo(Cmavo::Be)),
            new!(SyntaxExpectedToken::WordCategory(
                SyntaxWordCategory::Brivla
            )),
        ];

        sort_syntax_tokens(&mut tokens);

        let texts = tokens
            .iter()
            .map(SyntaxExpectedToken::summary_text)
            .collect::<Vec<_>>();
        assert_eq!(
            texts,
            ["BRIVLA", "GAhO", "be", "lo", "end of input", "input"]
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn detailed_expectation_groups_sort_by_depth_and_reason() {
        let expectations = vec![
            SyntaxExpectation::new(
                vec![new!(SyntaxExpectedToken::EndOfInput)],
                new!(SyntaxExpectationReason::EndThenStart {
                    starts: "end of input".to_owned(),
                    ends: vec!["statement".to_owned()],
                }),
            ),
            SyntaxExpectation::new(
                vec![new!(SyntaxExpectedToken::Selmaho(Selmaho::Ga))],
                new!(SyntaxExpectationReason::ContinueCurrent {
                    construct: "relation".to_owned(),
                }),
            ),
            SyntaxExpectation::new(
                vec![new!(SyntaxExpectedToken::Cmavo(Cmavo::Lo))],
                new!(SyntaxExpectationReason::StartNested {
                    construct: "argument".to_owned(),
                }),
            ),
            SyntaxExpectation::new(
                vec![new!(SyntaxExpectedToken::WordCategory(
                    SyntaxWordCategory::Brivla,
                ))],
                new!(SyntaxExpectationReason::ContinueCurrent {
                    construct: "argument".to_owned(),
                }),
            ),
        ];

        let text = segment_text(&syntax_detailed_segments(&expectations));

        let continue_argument = text
            .find("- BRIVLA [continues argument]")
            .expect("argument continuation");
        let start_argument = text.find("- argument (lo)").expect("argument start");
        let continue_relation = text
            .find("- GA [continues relation]")
            .expect("relation continuation");
        let end_statement = text
            .find("- end of input [ends statement]")
            .expect("end-of-input expectation");
        assert!(continue_argument < start_argument);
        assert!(start_argument < continue_relation);
        assert!(continue_relation < end_statement);
        assert!(!text.contains("end of input (end of input)"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn detailed_group_tokens_are_sorted() {
        let expectations = vec![SyntaxExpectation::new(
            vec![
                new!(SyntaxExpectedToken::Cmavo(Cmavo::Lo)),
                new!(SyntaxExpectedToken::Selmaho(Selmaho::Gaho)),
                new!(SyntaxExpectedToken::Cmavo(Cmavo::Be)),
                new!(SyntaxExpectedToken::WordCategory(
                    SyntaxWordCategory::Brivla
                )),
            ],
            new!(SyntaxExpectationReason::StartNested {
                construct: "argument".to_owned(),
            }),
        )];

        let text = segment_text(&syntax_detailed_segments(&expectations));

        assert!(text.contains("- argument (BRIVLA, GAhO, be or lo)"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn duplicate_continue_groups_keep_innermost_construct() {
        let tokens = vec![
            new!(SyntaxExpectedToken::Selmaho(Selmaho::Se)),
            new!(SyntaxExpectedToken::Selmaho(Selmaho::Bihi)),
        ];
        let expectations = vec![
            SyntaxExpectation::new(
                tokens.clone(),
                new!(SyntaxExpectationReason::ContinueCurrent {
                    construct: "statement".to_owned(),
                }),
            ),
            SyntaxExpectation::new(
                tokens.clone(),
                new!(SyntaxExpectationReason::ContinueCurrent {
                    construct: "relation".to_owned(),
                }),
            ),
            SyntaxExpectation::new(
                tokens,
                new!(SyntaxExpectationReason::ContinueCurrent {
                    construct: "argument".to_owned(),
                }),
            ),
        ];

        let text = segment_text(&syntax_detailed_segments(&expectations));

        assert!(text.contains("- BIhI or SE [continues argument]"));
        assert!(!text.contains("[continues relation]"));
        assert!(!text.contains("[continues statement]"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn summary_tokens_are_sorted() {
        let expectations = vec![SyntaxExpectation::new(
            vec![
                new!(SyntaxExpectedToken::Cmavo(Cmavo::Lo)),
                new!(SyntaxExpectedToken::WordCategory(
                    SyntaxWordCategory::Brivla
                )),
                new!(SyntaxExpectedToken::Selmaho(Selmaho::Gaho)),
            ],
            new!(SyntaxExpectationReason::StartNested {
                construct: "argument".to_owned(),
            }),
        )];

        let text = segment_text(&syntax_summary_segments_from_expectations(&expectations));

        assert_eq!(text, "expected one of: BRIVLA, GAhO, lo");
    }

    #[cfg(feature = "grammar-debug")]
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn grammar_debug_ebnf_contains_terminal_labels() {
        let output = syntax_grammar_ebnf(&ParseOptions::default());

        assert!(output.contains("argument"));
        assert!(output.contains("BRIVLA"));
        assert!(output.contains("QUOTE"));
    }

    #[cfg(feature = "grammar-debug")]
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn grammar_debug_dialect_changes_generated_grammar() {
        let default_output = syntax_grammar_ebnf(&ParseOptions::default());
        let dialect = jbotci_dialect::parse_dialect_definition("(zantufa-quotes)")
            .expect("valid dialect definition");
        let zantufa_options = ParseOptions::default().with_dialect_definition(&dialect);
        let zantufa_output = syntax_grammar_ebnf(&zantufa_options);

        assert_ne!(default_output, zantufa_output);
        assert!(zantufa_output.contains("mu'oi"));
    }

    #[requires(true)]
    #[ensures(true)]
    fn segment_text(segments: &[DiagnosticTextSegment]) -> String {
        segments
            .iter()
            .map(|segment| segment.text.as_str())
            .collect::<String>()
    }
}
