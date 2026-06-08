//! Lojban syntax model and parser facade.

pub mod tree;
pub use tree::{Token, WithIndicators, elidable_terminator_for_absent_field};

mod grammar;

extern crate self as jbotci_syntax;

use std::{cmp::Ordering, sync::Arc};

#[allow(unused_imports)]
use bityzba::{data, ensures, expensive_ensures, expensive_invariant, invariant, new, requires};
use jbotci_diagnostics::{
    Diagnostic, DiagnosticLabel, DiagnosticNoteMode, DiagnosticPhase, DiagnosticSeverity,
    DiagnosticStyledNote, DiagnosticTextRole, DiagnosticTextSegment, source_span_from_byte_offsets,
};
pub use jbotci_diagnostics::{TraceFilter, TraceLevel, TraceOptions, TracePhase, TraceReport};
use jbotci_dialect::DialectDefinition;
use jbotci_morphology::{Cmavo, Selmaho, Word, WordLike};
use jbotci_source::{SourceId, SourceSpan};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

pub mod ast {
    pub use crate::grammar::ast::*;
}
pub use ast::{Indicator, IndicatorData, TextSyntax};

type RecoveredSyntax<T> = tree::recovered::Recovered<T>;

pub const SYNTAX_TRACE_FILTERS: &[&str] = &[
    "text",
    "statement",
    "subbridi",
    "selbri",
    "term",
    "sumti",
    "free modifier",
    "token",
    "rewind",
];

impl TextSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_source_spans(&self, visitor: &mut impl FnMut(&jbotci_source::SourceSpan)) {
        self.visit_words(&mut |word| {
            for span in word.source_spans() {
                visitor(span);
            }
        });
    }
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn text_syntax_leaf_spans_match_words(
    words: &[WordLike],
    parse_tree: &TextSyntax,
) -> bool {
    let mut expected_refs = Vec::new();
    for word in words {
        word.source_spans_into(&mut expected_refs);
    }
    let expected: Vec<_> = expected_refs.into_iter().cloned().collect();
    let mut actual = Vec::new();
    parse_tree.visit_source_spans(&mut |span| actual.push(span.clone()));
    actual == expected
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn syntax_parse_leaf_spans_match_words(words: &[WordLike], parse: &SyntaxParse) -> bool {
    text_syntax_leaf_spans_match_words(words, &parse.parse_tree)
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

#[derive(Debug, Clone)]
#[invariant(true)]
pub struct RecoveredSyntaxParse {
    pub parse_tree: Box<tree::recovered::TextSyntax>,
    pub diagnostics: Vec<SyntaxError>,
    pub warnings: Vec<SyntaxWarning>,
}

#[derive(Debug, Clone)]
#[invariant(true)]
pub struct RecoveredSyntaxParseAttempt {
    pub result: RecoveredSyntaxParse,
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

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[invariant(true)]
#[invariant(::Parse => true)]
pub enum SyntaxError {
    #[error("syntax parsing is not implemented yet")]
    NotImplemented,
    #[error("syntax error at byte {byte_start}: {reason}")]
    Parse {
        kind: SyntaxErrorKind,
        byte_start: usize,
        byte_end: usize,
        reason: String,
        expected: Vec<String>,
        expectations: Vec<SyntaxExpectation>,
        context: Option<SyntaxConstructContext>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[invariant(true)]
pub enum SyntaxErrorKind {
    UnexpectedEnd,
    UnexpectedCmavo,
    UnexpectedBrivla,
    UnexpectedCmevla,
    UnexpectedQuote,
    UnexpectedLerfu,
    UnexpectedZeiCompound,
    UnexpectedWord,
    IncompleteText,
    IncompleteStatement,
    IncompleteBridi,
    IncompleteTerm,
    IncompleteSumti,
    IncompleteSelbri,
    IncompleteFreeModifier,
    IncompleteMekso,
    IncompleteQuote,
    IncompleteForethoughtConnection,
    InvalidBridiTailConnection,
    InvalidConstruct,
}

impl SyntaxErrorKind {
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn code(self) -> &'static str {
        match self {
            Self::UnexpectedEnd => "syntax.unexpected-end",
            Self::UnexpectedCmavo => "syntax.unexpected-cmavo",
            Self::UnexpectedBrivla => "syntax.unexpected-brivla",
            Self::UnexpectedCmevla => "syntax.unexpected-cmevla",
            Self::UnexpectedQuote => "syntax.unexpected-quote",
            Self::UnexpectedLerfu => "syntax.unexpected-lerfu",
            Self::UnexpectedZeiCompound => "syntax.unexpected-zei-compound",
            Self::UnexpectedWord => "syntax.unexpected-word",
            Self::IncompleteText => "syntax.incomplete-text",
            Self::IncompleteStatement => "syntax.incomplete-statement",
            Self::IncompleteBridi => "syntax.incomplete-bridi",
            Self::IncompleteTerm => "syntax.incomplete-term",
            Self::IncompleteSumti => "syntax.incomplete-sumti",
            Self::IncompleteSelbri => "syntax.incomplete-selbri",
            Self::IncompleteFreeModifier => "syntax.incomplete-free-modifier",
            Self::IncompleteMekso => "syntax.incomplete-mekso",
            Self::IncompleteQuote => "syntax.incomplete-quote",
            Self::IncompleteForethoughtConnection => "syntax.incomplete-forethought-connection",
            Self::InvalidBridiTailConnection => "syntax.invalid-bridi-tail-connection",
            Self::InvalidConstruct => "syntax.invalid-construct",
        }
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn message(self) -> &'static str {
        match self {
            Self::UnexpectedEnd => "unexpected end of input",
            Self::UnexpectedCmavo => "unexpected cmavo",
            Self::UnexpectedBrivla => "unexpected brivla",
            Self::UnexpectedCmevla => "unexpected cmevla",
            Self::UnexpectedQuote => "unexpected quote",
            Self::UnexpectedLerfu => "unexpected lerfu word",
            Self::UnexpectedZeiCompound => "unexpected ZEI compound",
            Self::UnexpectedWord => "unexpected word",
            Self::IncompleteText => "incomplete text",
            Self::IncompleteStatement => "incomplete statement",
            Self::IncompleteBridi => "incomplete bridi",
            Self::IncompleteTerm => "incomplete term",
            Self::IncompleteSumti => "incomplete sumti",
            Self::IncompleteSelbri => "incomplete selbri",
            Self::IncompleteFreeModifier => "incomplete free modifier",
            Self::IncompleteMekso => "incomplete mekso expression",
            Self::IncompleteQuote => "incomplete quote",
            Self::IncompleteForethoughtConnection => "incomplete forethought connection",
            Self::InvalidBridiTailConnection => "invalid bridi-tail connection",
            Self::InvalidConstruct => "invalid syntax construct",
        }
    }
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
    SelbriWord,
    ProSumti,
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
            Self::SelbriWord => "SELBRI WORD",
            Self::ProSumti => "PRO-SUMTI",
            Self::LetterWord => "LERFU",
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
enum SyntaxConstructWiring {
    Parser,
    Synthetic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
struct SyntaxConstructMetadata {
    name: &'static str,
    parent: Option<&'static str>,
    wiring: SyntaxConstructWiring,
}

const SYNTAX_CONSTRUCT_METADATA: &[SyntaxConstructMetadata] = &[
    SyntaxConstructMetadata {
        name: "bridi",
        parent: Some("statement"),
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "prenex",
        parent: Some("statement"),
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "text group",
        parent: Some("statement"),
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "statement",
        parent: Some("text"),
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "fragment",
        parent: Some("text"),
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "free modifier",
        parent: Some("text"),
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "terms",
        parent: Some("bridi"),
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "tail terms",
        parent: Some("bridi"),
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "forethought bridi connection",
        parent: Some("bridi"),
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "term",
        parent: Some("terms"),
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "termset",
        parent: Some("terms"),
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "sumti",
        parent: Some("term"),
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "tag",
        parent: Some("term"),
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "place tag",
        parent: Some("term"),
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "NA KU term",
        parent: Some("term"),
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "description",
        parent: Some("sumti"),
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "pro-sumti",
        parent: Some("sumti"),
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "name",
        parent: Some("sumti"),
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "quote",
        parent: Some("sumti"),
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "number sumti",
        parent: Some("sumti"),
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "lerfu string",
        parent: Some("sumti"),
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "converted sumti",
        parent: Some("sumti"),
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "bridi description",
        parent: Some("sumti"),
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "forethought sumti connection",
        parent: Some("sumti"),
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "relative clauses",
        parent: Some("sumti"),
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "descriptor",
        parent: Some("description"),
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "description tail",
        parent: Some("description"),
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "relative clause",
        parent: Some("relative clauses"),
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "relative bridi",
        parent: Some("relative clause"),
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "sumti association phrase",
        parent: Some("relative clause"),
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "mex",
        parent: Some("number sumti"),
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "operand",
        parent: Some("mex"),
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "operator",
        parent: Some("mex"),
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "forethought mex",
        parent: Some("mex"),
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "reverse Polish mex",
        parent: Some("mex"),
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "number",
        parent: Some("operand"),
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "parenthesized mex",
        parent: Some("operand"),
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "selbri operand",
        parent: Some("operand"),
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "sumti operand",
        parent: Some("operand"),
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "mekso array",
        parent: Some("operand"),
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "qualified operand",
        parent: Some("operand"),
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "VUhU operator",
        parent: Some("operator"),
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "operand-to-operator",
        parent: Some("operator"),
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "selbri-to-operator",
        parent: Some("operator"),
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "converted operator",
        parent: Some("operator"),
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "selbri",
        parent: Some("bridi"),
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "negated selbri",
        parent: Some("selbri"),
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "forethought selbri connection",
        parent: Some("selbri"),
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "tanru",
        parent: Some("selbri"),
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "tanru unit",
        parent: Some("tanru"),
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "abstraction",
        parent: Some("tanru unit"),
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "grouped tanru",
        parent: Some("tanru unit"),
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "sumti-to-selbri",
        parent: Some("tanru unit"),
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "operator-to-selbri",
        parent: Some("tanru unit"),
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "ordinal selbri",
        parent: Some("tanru unit"),
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "converted tanru unit",
        parent: Some("tanru unit"),
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "modal conversion",
        parent: Some("tanru unit"),
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "linked arguments",
        parent: Some("tanru unit"),
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "selbri relative phrase",
        parent: Some("tanru unit"),
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "subbridi",
        parent: Some("abstraction"),
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "quantifier",
        parent: Some("description"),
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "simple tense/modal",
        parent: Some("tag"),
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "FIhO modal",
        parent: Some("tag"),
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "connected tag",
        parent: Some("tag"),
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "modal tag",
        parent: Some("simple tense/modal"),
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "time tense",
        parent: Some("simple tense/modal"),
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "space tense",
        parent: Some("simple tense/modal"),
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "vocative phrase",
        parent: Some("free modifier"),
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "parenthetical text",
        parent: Some("free modifier"),
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "metalinguistic comment",
        parent: Some("free modifier"),
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "reciprocal",
        parent: Some("free modifier"),
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "subscript",
        parent: Some("free modifier"),
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "utterance ordinal",
        parent: Some("free modifier"),
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "replacement phrase",
        parent: Some("free modifier"),
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "word quote",
        parent: Some("quote"),
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "text quote",
        parent: Some("quote"),
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "word-sequence quote",
        parent: Some("quote"),
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "non-Lojban quote",
        parent: Some("quote"),
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "text",
        parent: None,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "parse_text",
        parent: None,
        wiring: SyntaxConstructWiring::Synthetic,
    },
    SyntaxConstructMetadata {
        name: "end of input",
        parent: None,
        wiring: SyntaxConstructWiring::Synthetic,
    },
    SyntaxConstructMetadata {
        name: "syntax construct",
        parent: None,
        wiring: SyntaxConstructWiring::Synthetic,
    },
];

#[requires(!construct.is_empty())]
#[ensures(ret.as_ref().is_none_or(|metadata| metadata.name == construct))]
fn syntax_construct_metadata(construct: &str) -> Option<&'static SyntaxConstructMetadata> {
    SYNTAX_CONSTRUCT_METADATA
        .iter()
        .find(|metadata| metadata.name == construct)
}

#[requires(!construct.is_empty())]
#[ensures(true)]
pub(crate) fn syntax_construct_parent(construct: &str) -> Option<&'static str> {
    syntax_construct_metadata(construct).and_then(|metadata| metadata.parent)
}

#[requires(!construct.is_empty())]
#[ensures(ret < SYNTAX_CONSTRUCT_METADATA.len())]
pub(crate) fn syntax_construct_depth(construct: &str) -> usize {
    if !syntax_construct_is_known(construct) {
        panic!("missing syntax diagnostic construct metadata for {construct:?}");
    }
    let mut depth = 0;
    let mut current = construct;
    while let Some(parent) = syntax_construct_parent(current) {
        depth += 1;
        current = parent;
    }
    depth
}

#[requires(!construct.is_empty())]
#[ensures(ret -> !construct.is_empty())]
pub(crate) fn syntax_construct_is_known(construct: &str) -> bool {
    syntax_construct_metadata(construct).is_some()
}

#[requires(!construct.is_empty())]
#[ensures(ret == matches!(construct, "text" | "parse_text"))]
pub(crate) fn syntax_construct_is_root(construct: &str) -> bool {
    if !syntax_construct_is_known(construct) {
        panic!("missing syntax diagnostic construct metadata for {construct:?}");
    }
    matches!(construct, "text" | "parse_text")
}

#[requires(!ancestor.is_empty())]
#[requires(!descendant.is_empty())]
#[ensures(ret.as_ref().is_none_or(|child| !child.is_empty()))]
pub(crate) fn syntax_immediate_child_under(ancestor: &str, descendant: &str) -> Option<String> {
    if ancestor == descendant || !syntax_construct_is_known(ancestor) {
        return None;
    }
    let mut child = descendant;
    let mut parent = syntax_construct_parent(child)?;
    while parent != ancestor {
        child = parent;
        parent = syntax_construct_parent(child)?;
    }
    Some(child.to_owned())
}

#[requires(!ancestor.is_empty())]
#[requires(!descendant.is_empty())]
#[ensures(ret -> syntax_construct_is_known(ancestor))]
pub(crate) fn syntax_construct_is_descendant_of(ancestor: &str, descendant: &str) -> bool {
    if ancestor == descendant || !syntax_construct_is_known(ancestor) {
        return false;
    }
    let mut current = descendant;
    while let Some(parent) = syntax_construct_parent(current) {
        if parent == ancestor {
            return true;
        }
        current = parent;
    }
    false
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
                kind,
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
                    kind.code().to_owned(),
                    kind.message().to_owned(),
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
#[ensures(ret.starts_with("expected: "))]
pub(crate) fn syntax_expectation_summary_message(
    expectations: &[SyntaxExpectation],
    scope: Option<&str>,
) -> String {
    let constructs = syntax_expectation_summary_constructs(expectations, scope);
    format!("expected: {}", prose_list_text(&constructs))
}

#[requires(!expectations.is_empty())]
#[ensures(!ret.is_empty())]
fn syntax_expectation_summary_constructs(
    expectations: &[SyntaxExpectation],
    scope: Option<&str>,
) -> Vec<String> {
    let mut constructs = Vec::new();
    for expectation in merge_expectations_by_reason(expectations) {
        let construct = syntax_expectation_summary_construct(expectation.reason.construct(), scope);
        if !constructs.contains(&construct) {
            constructs.push(construct);
        }
    }
    if let Some(scope) = scope
        && constructs.len() > 1
    {
        constructs.retain(|construct| construct != scope);
    }
    if let Some(scope) = scope {
        let has_scoped_construct = constructs
            .iter()
            .any(|construct| syntax_construct_is_relevant_to_summary_scope(scope, construct));
        if has_scoped_construct {
            constructs.retain(|construct| {
                syntax_construct_is_relevant_to_summary_scope(scope, construct)
                    || syntax_construct_is_free_modifier_summary(construct)
                    || construct == "end of input"
            });
        }
    }
    constructs
}

#[requires(!scope.is_empty())]
#[requires(!construct.is_empty())]
#[ensures(true)]
fn syntax_construct_is_relevant_to_summary_scope(scope: &str, construct: &str) -> bool {
    if construct == scope {
        return true;
    }
    if syntax_construct_is_descendant_of(scope, construct) {
        return true;
    }
    if let Some(parent) = syntax_construct_parent(scope)
        && scope.starts_with("forethought ")
        && (construct == parent || syntax_construct_is_descendant_of(parent, construct))
    {
        return true;
    }
    false
}

#[requires(!construct.is_empty())]
#[ensures(true)]
fn syntax_construct_is_free_modifier_summary(construct: &str) -> bool {
    construct == "free modifier" || syntax_construct_is_descendant_of("free modifier", construct)
}

#[requires(!construct.is_empty())]
#[ensures(!ret.is_empty())]
fn syntax_expectation_summary_construct(construct: &str, scope: Option<&str>) -> String {
    if let Some(scope) = scope {
        if construct == scope {
            return construct.to_owned();
        }
        if let Some(child) = syntax_immediate_child_under(scope, construct) {
            return child;
        }
    }
    if construct != "free modifier" && syntax_construct_is_descendant_of("free modifier", construct)
    {
        "free modifier".to_owned()
    } else {
        construct.to_owned()
    }
}

#[requires(!items.is_empty())]
#[ensures(!ret.is_empty())]
fn prose_list_text(items: &[String]) -> String {
    let mut text = String::new();
    for (index, item) in items.iter().enumerate() {
        if index > 0 {
            push_prose_list_separator_text(&mut text, index, items.len());
        }
        text.push_str(item);
    }
    text
}

#[requires(index > 0)]
#[requires(index < len)]
#[ensures(!text.is_empty())]
fn push_prose_list_separator_text(text: &mut String, index: usize, len: usize) {
    if index + 1 == len {
        if len > 2 {
            text.push_str(", or ");
        } else {
            text.push_str(" or ");
        }
    } else {
        text.push_str(", ");
    }
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
    push_token_list(&mut segments, &tokens);
    segments
}

#[requires(!expected.is_empty())]
#[ensures(!ret.is_empty())]
fn syntax_summary_segments_from_strings(expected: &[String]) -> Vec<DiagnosticTextSegment> {
    let mut segments = vec![plain_segment("expected one of: ")];
    for (index, item) in expected.iter().enumerate() {
        if index > 0 {
            push_prose_list_separator_segment(&mut segments, index, expected.len());
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
    let bucket_order =
        syntax_reason_sort_bucket(&left.reason).cmp(&syntax_reason_sort_bucket(&right.reason));
    if bucket_order != Ordering::Equal {
        return bucket_order;
    }

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
#[ensures(ret <= 1)]
fn syntax_reason_sort_bucket(reason: &SyntaxExpectationReason) -> u8 {
    let construct = syntax_reason_sort_construct(reason);
    if construct == "free modifier" || syntax_construct_is_descendant_of("free modifier", construct)
    {
        0
    } else {
        1
    }
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
            push_prose_list_separator_segment(segments, index, tokens.len());
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
            push_prose_list_separator_segment(segments, index, constructs.len());
        }
        segments.push(construct_segment(construct));
    }
}

#[requires(index > 0)]
#[requires(index < len)]
#[ensures(true)]
fn push_prose_list_separator_segment(
    segments: &mut Vec<DiagnosticTextSegment>,
    index: usize,
    len: usize,
) {
    if index + 1 == len {
        if len > 2 {
            segments.push(punctuation_segment(", or "));
        } else {
            segments.push(punctuation_segment(" or "));
        }
    } else {
        segments.push(punctuation_segment(", "));
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
#[expensive_ensures(ret.as_ref().map_or(true, |parse_tree| {
    text_syntax_leaf_spans_match_words(words, parse_tree)
}))]
pub fn parse_text(words: &[WordLike], options: &ParseOptions) -> Result<TextSyntax, SyntaxError> {
    grammar::parse_text(words, options)
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
    ExperimentalMehOiQuote,
    ExperimentalMehOiSelbriUnit,
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
    ExperimentalGohoiSelbriUnit,
    ExperimentalKeTermset,
    ExperimentalCuTermsSelbri,
    ExperimentalLaheNaheTermWrapper,
    ExperimentalForethoughtRelativeClauseConnective,
    ExperimentalBroadAConnective,
    ExperimentalVuhuConnective,
    ExperimentalNahuPredicateConnective,
    ExperimentalFaAsTag,
    ExperimentalFlattenedTag,
    ExperimentalCbmCmevlaSelbriWord,
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
    ExperimentalXohiTagSelbri,
    ExperimentalZantufaCmavo,
    ExperimentalZantufaForethoughtGihi,
    ExperimentalZantufaGek,
    ExperimentalZantufaPoihaBrigahi,
    ExperimentalZantufaJaiTagTerm,
    ExperimentalZantufaRecursiveTag,
    ExperimentalZantufaRahoiQuote,
    ExperimentalZantufaMuhoiSelbriUnit,
    ExperimentalZantufaLuheiSelbriUnit,
    CllProhibitedFreeModifierPlacement,
}

impl ExperimentalConstruct {
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub const fn code(self) -> &'static str {
        match self {
            Self::ExperimentalCmavo => "syntax.warning.experimental-cmavo",
            Self::ExperimentalZohOiQuote => "syntax.warning.experimental-zoh-oi-quote",
            Self::ExperimentalMehOiQuote => "syntax.warning.experimental-meh-oi-quote",
            Self::ExperimentalMehOiSelbriUnit => "syntax.warning.experimental-meh-oi-selbri-unit",
            Self::ExperimentalLohOiBridiDescription => {
                "syntax.warning.experimental-loh-oi-bridi-description"
            }
            Self::ExperimentalLohAiReplacementFree => {
                "syntax.warning.experimental-loh-ai-replacement-free"
            }
            Self::ExperimentalJacuPredicateTailConnective => {
                "syntax.warning.experimental-jacu-bridi-tail-connective"
            }
            Self::ExperimentalJeIStatementConnective => {
                "syntax.warning.experimental-je-i-statement-connective"
            }
            Self::ExperimentalMultipleNaFragment => {
                "syntax.warning.experimental-multiple-na-fragment"
            }
            Self::ExperimentalEmptyPrenex => "syntax.warning.experimental-empty-prenex",
            Self::ExperimentalBareCuPredicate => "syntax.warning.experimental-bare-cu-bridi",
            Self::ExperimentalNaheArgumentWithoutBo => {
                "syntax.warning.experimental-nahe-sumti-without-bo"
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
                "syntax.warning.experimental-explicit-cu-bridi-tail-starter"
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
                "syntax.warning.experimental-simpler-description-head-connective"
            }
            Self::ExperimentalJiAsJaConnective => "syntax.warning.experimental-ji-as-ja-connective",
            Self::ExperimentalGadganzuGadri => "syntax.warning.experimental-gadganzu-gadri",
            Self::ExperimentalIauReset => "syntax.warning.experimental-iau-reset",
            Self::ExperimentalGohoiSelbriUnit => "syntax.warning.experimental-gohoi-selbri-unit",
            Self::ExperimentalKeTermset => "syntax.warning.experimental-ke-termset",
            Self::ExperimentalCuTermsSelbri => "syntax.warning.experimental-cu-terms-selbri",
            Self::ExperimentalLaheNaheTermWrapper => {
                "syntax.warning.experimental-lahe-nahe-term-wrapper"
            }
            Self::ExperimentalForethoughtRelativeClauseConnective => {
                "syntax.warning.experimental-forethought-relative-clause-connective"
            }
            Self::ExperimentalBroadAConnective => "syntax.warning.experimental-broad-a-connective",
            Self::ExperimentalVuhuConnective => "syntax.warning.experimental-vuhu-connective",
            Self::ExperimentalNahuPredicateConnective => {
                "syntax.warning.experimental-nahu-bridi-connective"
            }
            Self::ExperimentalFaAsTag => "syntax.warning.experimental-fa-as-tag",
            Self::ExperimentalFlattenedTag => "syntax.warning.experimental-flattened-tag",
            Self::ExperimentalCbmCmevlaSelbriWord => {
                "syntax.warning.experimental-cbm-cmevla-selbri-word"
            }
            Self::ExperimentalCbmLaNameAsDescriptor => {
                "syntax.warning.experimental-cbm-la-name-as-description"
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
                "syntax.warning.experimental-broad-ke-bridi-continuation"
            }
            Self::ExperimentalTermHierarchyBoConnection => {
                "syntax.warning.experimental-term-hierarchy-bo-connection"
            }
            Self::ExperimentalBareNaTerm => "syntax.warning.experimental-bare-na-term",
            Self::ExperimentalXohiTagSelbri => "syntax.warning.experimental-xohi-tag-selbri",
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
            Self::ExperimentalZantufaRahoiQuote => {
                "syntax.warning.experimental-zantufa-rahoi-quote"
            }
            Self::ExperimentalZantufaMuhoiSelbriUnit => {
                "syntax.warning.experimental-zantufa-muhoi-selbri-unit"
            }
            Self::ExperimentalZantufaLuheiSelbriUnit => {
                "syntax.warning.experimental-zantufa-luhei-selbri-unit"
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
            Self::ExperimentalMehOiQuote => "MEhOI single-word quote",
            Self::ExperimentalMehOiSelbriUnit => "MEhOI stage-0 fu'ivla selbri unit",
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
                "JA connective used in an sumti connective slot"
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
                "JA connective used between description heads"
            }
            Self::ExperimentalJiAsJaConnective => "JI used as an experimental JA-family connective",
            Self::ExperimentalGadganzuGadri => "gadganzu article",
            Self::ExperimentalIauReset => "IhAU bridi-level reset",
            Self::ExperimentalGohoiSelbriUnit => "GOhOI pro-bridi word quote",
            Self::ExperimentalKeTermset => "KE/KEhE termset grouping",
            Self::ExperimentalCuTermsSelbri => "CU followed by terms before the main selbri",
            Self::ExperimentalLaheNaheTermWrapper => "LAhE/NAhE term wrapper",
            Self::ExperimentalForethoughtRelativeClauseConnective => {
                "forethought connective used between relative clauses"
            }
            Self::ExperimentalBroadAConnective => {
                "A-family connective used in a broader connective-family slot"
            }
            Self::ExperimentalVuhuConnective => "VUhU used as a non-MEX connective",
            Self::ExperimentalNahuPredicateConnective => "NAhU/ji'oi bridi-to-connective form",
            Self::ExperimentalFaAsTag => "FA place tag used as a tag/stag atom",
            Self::ExperimentalFlattenedTag => "experimental flattened tag form",
            Self::ExperimentalCbmCmevlaSelbriWord => "CBM cmevla used as a selbri word",
            Self::ExperimentalCbmLaNameAsDescriptor => "CBM LA name form parsed as a description",
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
            Self::ExperimentalFihoiAdverbial => "FIhOI bridi/subbridi adverbial term",
            Self::ExperimentalSoiAdverbial => "SOI/XOI bridi/subbridi adverbial term",
            Self::ExperimentalPreposedLinkargs => "BE linkargs before a selbri unit",
            Self::ExperimentalEmptyLinkargs => "empty BE/BEI linkarg slot",
            Self::ExperimentalBroadBoStatementConnective => {
                "broad connective with BO in a statement/subbridi continuation"
            }
            Self::ExperimentalBroadKePredicateContinuation => {
                "broad connective with KE/KEhE in a bridi/subbridi continuation"
            }
            Self::ExperimentalTermHierarchyBoConnection => {
                "experimental term-hierarchy BO connection"
            }
            Self::ExperimentalBareNaTerm => "bare NA term/adverbial without KU",
            Self::ExperimentalXohiTagSelbri => "XOhI tag-to-selbri conversion",
            Self::ExperimentalZantufaCmavo => "Zantufa experimental cmavo classification",
            Self::ExperimentalZantufaForethoughtGihi => "Zantufa GIhI forethought-chain terminator",
            Self::ExperimentalZantufaGek => "Zantufa forethought connective form",
            Self::ExperimentalZantufaPoihaBrigahi => {
                "Zantufa POIhA briga'i term with KU terminator"
            }
            Self::ExperimentalZantufaJaiTagTerm => "Zantufa JAI tag term",
            Self::ExperimentalZantufaRecursiveTag => "Zantufa recursive SE/NAhE tag prefix",
            Self::ExperimentalZantufaRahoiQuote => "Zantufa RAhOI rafsi quote",
            Self::ExperimentalZantufaMuhoiSelbriUnit => {
                "Zantufa MUhOI delimited foreign selbri unit"
            }
            Self::ExperimentalZantufaLuheiSelbriUnit => "Zantufa LUhEI/LIhAU text selbri unit",
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
    pub anchor: Token,
}

impl SyntaxWarning {
    #[requires(true)]
    #[ensures(true)]
    pub fn experimental_construct(
        construct: ExperimentalConstruct,
        anchor_index: usize,
        anchor: Token,
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
    words: &[Token],
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
    words: &[Token],
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
fn warning_context(words: &[Token], index: usize) -> String {
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
fn warning_word_text(word: &Token) -> String {
    format!("{word}")
}

#[requires(true)]
#[ensures(true)]
#[expensive_ensures(ret.as_ref().map_or(true, |parse| {
    syntax_parse_leaf_spans_match_words(words, parse)
}))]
pub fn parse_syntax_tree(words: &[WordLike]) -> Result<SyntaxParse, SyntaxError> {
    parse_syntax_tree_with_options(words, &ParseOptions::default())
}

#[requires(true)]
#[ensures(true)]
#[expensive_ensures(ret.as_ref().map_or(true, |parse| {
    syntax_parse_leaf_spans_match_words(words, parse)
}))]
pub fn parse_syntax_tree_with_options(
    words: &[WordLike],
    options: &ParseOptions,
) -> Result<SyntaxParse, SyntaxError> {
    grammar::parse_syntax_tree(words, options)
}

#[requires(true)]
#[ensures(true)]
#[expensive_ensures(ret.as_ref().map_or(true, |parse| {
    syntax_parse_leaf_spans_match_words(words, parse)
}))]
pub fn parse_syntax_tree_with_source_and_options(
    words: &[WordLike],
    source: &str,
    options: &ParseOptions,
) -> Result<SyntaxParse, SyntaxError> {
    grammar::parse_syntax_tree_with_source(words, Some(source), options)
}

#[requires(true)]
#[ensures(true)]
#[expensive_ensures(ret.result.as_ref().map_or(true, |parse| {
    syntax_parse_leaf_spans_match_words(words, parse)
}))]
pub fn parse_syntax_tree_with_source_and_options_attempt(
    words: &[WordLike],
    source: &str,
    options: &ParseOptions,
) -> SyntaxParseAttempt {
    grammar::parse_syntax_tree_with_source_attempt(words, Some(source), options)
}

#[requires(true)]
#[ensures(true)]
pub fn parse_syntax_tree_recovered(
    words: &[jbotci_morphology::tree::recovered::WordLike],
) -> RecoveredSyntaxParseAttempt {
    parse_syntax_tree_recovered_with_source_and_options(words, None, &ParseOptions::default())
}

#[requires(true)]
#[ensures(true)]
pub fn parse_syntax_tree_recovered_with_options(
    words: &[jbotci_morphology::tree::recovered::WordLike],
    options: &ParseOptions,
) -> RecoveredSyntaxParseAttempt {
    parse_syntax_tree_recovered_with_source_and_options(words, None, options)
}

#[requires(true)]
#[ensures(true)]
pub fn parse_syntax_tree_recovered_with_source_and_options(
    words: &[jbotci_morphology::tree::recovered::WordLike],
    source: Option<&str>,
    options: &ParseOptions,
) -> RecoveredSyntaxParseAttempt {
    let valid_words = words
        .iter()
        .cloned()
        .map(jbotci_morphology::tree::recovered::WordLike::try_into_valid)
        .collect::<Result<Vec<_>, _>>();
    let valid_words = match valid_words {
        Ok(valid_words) => valid_words,
        Err(error) => {
            let diagnostic = syntax_error_for_morphology_recovery(&error);
            let statement = recovered_statement_slot_from_morphology_recovery(error);
            return RecoveredSyntaxParseAttempt {
                result: RecoveredSyntaxParse {
                    parse_tree: Box::new(recovered_text_from_statement_slot(statement)),
                    diagnostics: vec![diagnostic],
                    warnings: Vec::new(),
                },
                trace: None,
            };
        }
    };

    let attempt = grammar::parse_syntax_tree_with_source_attempt(&valid_words, source, options);
    match attempt.result {
        Ok(parse) => {
            let data = parse.into_data();
            RecoveredSyntaxParseAttempt {
                result: RecoveredSyntaxParse {
                    parse_tree: Box::new(tree::recovered::TextSyntax::from_valid(*data.parse_tree)),
                    diagnostics: Vec::new(),
                    warnings: data.warnings,
                },
                trace: attempt.trace,
            }
        }
        Err(error) => {
            let tokens = grammar::syntax_tokens(&valid_words);
            RecoveredSyntaxParseAttempt {
                result: RecoveredSyntaxParse {
                    parse_tree: Box::new(recovered_text_from_tokens(&tokens, &error)),
                    diagnostics: vec![error],
                    warnings: Vec::new(),
                },
                trace: attempt.trace,
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn recovered_text_from_statement_slot(
    statement: RecoveredSyntax<tree::recovered::StatementSyntax>,
) -> tree::recovered::TextSyntax {
    tree::recovered::TextSyntax {
        leading_nai: Vec::new(),
        leading_cmevla: Vec::new(),
        leading_indicators: Vec::new(),
        leading_free_modifiers: Vec::new(),
        leading_connective: None,
        paragraphs: vec![RecoveredSyntax::valid(tree::recovered::ParagraphSyntax {
            i: None,
            niho: Vec::new(),
            free_modifiers: Vec::new(),
            statements: vec![RecoveredSyntax::valid(
                tree::recovered::ParagraphStatementSyntax {
                    i: None,
                    connective: None,
                    free_modifiers: Vec::new(),
                    statement: Some(Box::new(statement)),
                },
            )],
        })],
    }
}

#[requires(true)]
#[ensures(true)]
fn recovered_text_from_tokens(
    tokens: &[Token],
    error: &SyntaxError,
) -> tree::recovered::TextSyntax {
    let mut paragraphs = Vec::new();
    let mut paragraph_niho = Vec::new();
    let mut statements = Vec::new();
    let mut statement_i = None;
    let mut statement_tokens = Vec::new();

    for token in tokens {
        if token.is_selmaho(Selmaho::Niho) {
            push_recovered_statement(
                &mut statements,
                &mut statement_i,
                &mut statement_tokens,
                error,
            );
            push_recovered_paragraph(&mut paragraphs, &mut paragraph_niho, &mut statements);
            paragraph_niho.push(RecoveredSyntax::valid(token.clone()));
            continue;
        }
        if token.is_cmavo(Cmavo::I) {
            push_recovered_statement(
                &mut statements,
                &mut statement_i,
                &mut statement_tokens,
                error,
            );
            statement_i = Some(RecoveredSyntax::valid(token.clone()));
            continue;
        }
        statement_tokens.push(token.clone());
        if is_local_syntax_recovery_boundary(token) {
            push_recovered_statement(
                &mut statements,
                &mut statement_i,
                &mut statement_tokens,
                error,
            );
        }
    }
    push_recovered_statement(
        &mut statements,
        &mut statement_i,
        &mut statement_tokens,
        error,
    );
    push_recovered_paragraph(&mut paragraphs, &mut paragraph_niho, &mut statements);

    tree::recovered::TextSyntax {
        leading_nai: Vec::new(),
        leading_cmevla: Vec::new(),
        leading_indicators: Vec::new(),
        leading_free_modifiers: Vec::new(),
        leading_connective: None,
        paragraphs,
    }
}

#[requires(true)]
#[ensures(true)]
fn push_recovered_paragraph(
    paragraphs: &mut Vec<RecoveredSyntax<tree::recovered::ParagraphSyntax>>,
    niho: &mut Vec<RecoveredSyntax<Token>>,
    statements: &mut Vec<RecoveredSyntax<tree::recovered::ParagraphStatementSyntax>>,
) {
    if niho.is_empty() && statements.is_empty() {
        return;
    }
    paragraphs.push(RecoveredSyntax::valid(tree::recovered::ParagraphSyntax {
        i: None,
        niho: std::mem::take(niho),
        free_modifiers: Vec::new(),
        statements: std::mem::take(statements),
    }));
}

#[requires(true)]
#[ensures(true)]
fn push_recovered_statement(
    statements: &mut Vec<RecoveredSyntax<tree::recovered::ParagraphStatementSyntax>>,
    i: &mut Option<RecoveredSyntax<Token>>,
    tokens: &mut Vec<Token>,
    error: &SyntaxError,
) {
    if i.is_none() && tokens.is_empty() {
        return;
    }
    let statement = if tokens.is_empty() {
        Some(Box::new(RecoveredSyntax::missing(
            syntax_missing_item_at_error(error, "statement"),
        )))
    } else {
        Some(Box::new(RecoveredSyntax::invalid(
            syntax_invalid_item_for_tokens(tokens, error),
        )))
    };
    statements.push(RecoveredSyntax::valid(
        tree::recovered::ParagraphStatementSyntax {
            i: i.take(),
            connective: None,
            free_modifiers: Vec::new(),
            statement,
        },
    ));
    tokens.clear();
}

#[requires(true)]
#[ensures(true)]
fn is_local_syntax_recovery_boundary(token: &Token) -> bool {
    token.is_cmavo(Cmavo::Cu)
        || token.is_selmaho(Selmaho::Gi)
        || token.is_one_of_cmavo(&[
            Cmavo::Ku,
            Cmavo::Vau,
            Cmavo::Kehe,
            Cmavo::Kuho,
            Cmavo::Gehu,
            Cmavo::Nuhu,
            Cmavo::Luhu,
            Cmavo::Lehu,
            Cmavo::Tehu,
            Cmavo::Mehu,
            Cmavo::Fehu,
            Cmavo::Sehu,
            Cmavo::Dohu,
            Cmavo::Toi,
            Cmavo::Veho,
            Cmavo::Kuhe,
            Cmavo::Lihu,
            Cmavo::Beho,
        ])
}

#[requires(!tokens.is_empty())]
#[ensures(!ret.text.is_empty())]
fn syntax_invalid_item_for_tokens(tokens: &[Token], error: &SyntaxError) -> tree::InvalidTreeItem {
    let span = token_slice_span(tokens).unwrap_or_else(|| syntax_error_span(error));
    new!(tree::InvalidTreeItem {
        span: Arc::new(span),
        text: token_text(tokens),
        diagnostic_code: syntax_error_code(error).to_owned(),
    })
}

#[requires(true)]
#[ensures(!ret.expected.is_empty())]
fn syntax_missing_item_at_error(error: &SyntaxError, expected: &str) -> tree::MissingTreeItem {
    new!(tree::MissingTreeItem {
        span: Arc::new(syntax_error_span(error)),
        expected: vec![expected.to_owned()],
    })
}

#[requires(!tokens.is_empty())]
#[ensures(!ret.is_empty())]
fn token_text(tokens: &[Token]) -> String {
    tokens
        .iter()
        .map(recovered_token_text)
        .collect::<Vec<_>>()
        .join(" ")
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn recovered_token_text(token: &Token) -> String {
    token
        .core_word()
        .bare_word()
        .map(Word::canonical_phonemes)
        .unwrap_or_else(|| token.to_string())
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|span| span.byte_start <= span.byte_end))]
fn token_slice_span(tokens: &[Token]) -> Option<SourceSpan> {
    let mut spans = Vec::new();
    for token in tokens {
        token.source_spans_into(&mut spans);
    }
    let first = spans.first()?;
    let byte_start = spans.iter().map(|span| span.byte_start).min()?;
    let byte_end = spans.iter().map(|span| span.byte_end).max()?;
    let char_start = spans.iter().map(|span| span.char_start).min()?;
    let char_end = spans.iter().map(|span| span.char_end).max()?;
    SourceSpan::new(
        first.source_id.clone(),
        byte_start,
        byte_end,
        char_start,
        char_end,
    )
    .ok()
}

#[requires(true)]
#[ensures(ret.byte_start <= ret.byte_end)]
fn syntax_error_span(error: &SyntaxError) -> SourceSpan {
    match error {
        SyntaxError::Parse {
            byte_start,
            byte_end,
            ..
        } => SourceSpan::new(None, *byte_start, *byte_end, *byte_start, *byte_end)
            .expect("syntax error byte ranges are ordered"),
        SyntaxError::NotImplemented => {
            SourceSpan::new(None, 0, 0, 0, 0).expect("zero span is ordered")
        }
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn syntax_error_code(error: &SyntaxError) -> &'static str {
    match error {
        SyntaxError::Parse { kind, .. } => kind.code(),
        SyntaxError::NotImplemented => "syntax.not-implemented",
    }
}

#[requires(true)]
#[ensures(true)]
fn recovered_statement_slot_from_morphology_recovery(
    error: jbotci_tree::RecoveryError<
        jbotci_morphology::tree::InvalidTreeItem,
        jbotci_morphology::tree::MissingTreeItem,
    >,
) -> RecoveredSyntax<tree::recovered::StatementSyntax> {
    match error {
        jbotci_tree::RecoveryError::Invalid { item, .. } => {
            let data = item.into_data();
            RecoveredSyntax::invalid(new!(tree::InvalidTreeItem {
                span: data.span,
                text: data.text,
                diagnostic_code: data.diagnostic_code,
            }))
        }
        jbotci_tree::RecoveryError::Missing { item, .. } => {
            let data = item.into_data();
            RecoveredSyntax::missing(new!(tree::MissingTreeItem {
                span: data.span,
                expected: data.expected,
            }))
        }
    }
}

#[requires(true)]
#[ensures(matches!(ret, SyntaxError::Parse { .. }))]
fn syntax_error_for_morphology_recovery(
    error: &jbotci_tree::RecoveryError<
        jbotci_morphology::tree::InvalidTreeItem,
        jbotci_morphology::tree::MissingTreeItem,
    >,
) -> SyntaxError {
    match error {
        jbotci_tree::RecoveryError::Invalid { item, .. } => SyntaxError::Parse {
            kind: SyntaxErrorKind::UnexpectedWord,
            byte_start: item.span.byte_start,
            byte_end: item.span.byte_end,
            reason: "recovered morphology item is not a valid syntax token".to_owned(),
            expected: Vec::new(),
            expectations: Vec::new(),
            context: None,
        },
        jbotci_tree::RecoveryError::Missing { item, .. } => SyntaxError::Parse {
            kind: SyntaxErrorKind::UnexpectedEnd,
            byte_start: item.span.byte_start,
            byte_end: item.span.byte_end,
            reason: "missing morphology item cannot be parsed as syntax".to_owned(),
            expected: item.expected.clone(),
            expectations: Vec::new(),
            context: None,
        },
    }
}

#[requires(true)]
#[ensures(true)]
pub fn syntax_tree_eq_ignoring_spans(left: &TextSyntax, right: &TextSyntax) -> bool {
    let Ok(mut left) = serde_json::to_value(left) else {
        return false;
    };
    let Ok(mut right) = serde_json::to_value(right) else {
        return false;
    };
    remove_source_span_fields(&mut left);
    remove_source_span_fields(&mut right);
    left == right
}

#[requires(true)]
#[ensures(true)]
fn remove_source_span_fields(value: &mut Value) {
    match value {
        Value::Object(object) => {
            object.remove("span");
            for child in object.values_mut() {
                remove_source_span_fields(child);
            }
        }
        Value::Array(items) => {
            for child in items {
                remove_source_span_fields(child);
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
    }
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use bityzba::{data, ensures, new, requires};

    use super::*;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn raw_syntax_tree_paths_round_trip_on_real_parse() {
        use crate::ast::TreeNode as _;
        use jbotci_tree::{TreePath, TreePathStep};

        let words =
            jbotci_morphology::segment_words_with_modifiers("mi klama").expect("valid morphology");
        let parsed = parse_syntax_tree(&words).expect("valid syntax");
        let tree = parsed.parse_tree.as_ref();
        let paragraph = tree.paragraphs.first().expect("parse has a paragraph");
        let target = ast::NodeRef::ParagraphSyntax(paragraph);

        let path = tree.path_to_node(target).expect("paragraph is in tree");

        assert_eq!(
            path,
            TreePath::from_steps(vec![
                TreePathStep::field(Some("paragraphs"), 5),
                TreePathStep::sequence_index(0),
            ])
        );
        assert_eq!(tree.node_at_path(&path), Some(target));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn recovered_syntax_successful_parse_round_trips_to_strict_tree() {
        let morphology =
            jbotci_morphology::segment_words_with_modifiers_recovered("mi klama").into_data();
        let strict_words = morphology
            .result
            .iter()
            .cloned()
            .map(jbotci_morphology::tree::recovered::WordLike::try_into_valid)
            .collect::<Result<Vec<_>, _>>()
            .expect("morphology is valid");
        let strict = parse_syntax_tree(&strict_words).expect("strict syntax is valid");

        let recovered = parse_syntax_tree_recovered(&morphology.result).result;

        assert!(recovered.diagnostics.is_empty());
        let valid = recovered
            .parse_tree
            .as_ref()
            .clone()
            .try_into_valid()
            .expect("recovered syntax is valid");
        assert_eq!(valid, strict.parse_tree.as_ref().clone());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn recovered_syntax_splits_invalid_statements_at_i() {
        let morphology =
            jbotci_morphology::segment_words_with_modifiers_recovered("mi ku i do").into_data();

        let recovered = parse_syntax_tree_recovered(&morphology.result).result;

        assert_eq!(recovered.diagnostics.len(), 1);
        let statements = recovered_statement_slots(recovered.parse_tree.as_ref());
        assert_eq!(statements.len(), 2);
        assert_recovered_statement_invalid_text(&statements[0], "mi ku");
        assert_recovered_statement_invalid_text(&statements[1], "do");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn recovered_syntax_uses_cu_as_local_boundary() {
        let morphology =
            jbotci_morphology::segment_words_with_modifiers_recovered("mi cu ku").into_data();

        let recovered = parse_syntax_tree_recovered(&morphology.result).result;

        assert_eq!(recovered.diagnostics.len(), 1);
        let statements = recovered_statement_slots(recovered.parse_tree.as_ref());
        assert_eq!(statements.len(), 2);
        assert_recovered_statement_invalid_text(&statements[0], "mi cu");
        assert_recovered_statement_invalid_text(&statements[1], "ku");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn recovered_syntax_preserves_invalid_morphology_leaf() {
        let morphology =
            jbotci_morphology::segment_words_with_modifiers_recovered("mi @@@ do").into_data();

        let recovered = parse_syntax_tree_recovered(&morphology.result).result;

        assert!(matches!(
            recovered.diagnostics.first(),
            Some(SyntaxError::Parse {
                kind: SyntaxErrorKind::UnexpectedWord,
                ..
            })
        ));
        let statements = recovered_statement_slots(recovered.parse_tree.as_ref());
        assert_eq!(statements.len(), 1);
        assert_recovered_statement_invalid_text(&statements[0], "@@@");
    }

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
                    construct: "selbri".to_owned(),
                }),
            ),
            SyntaxExpectation::new(
                vec![new!(SyntaxExpectedToken::Cmavo(Cmavo::Lo))],
                new!(SyntaxExpectationReason::StartNested {
                    construct: "sumti".to_owned(),
                }),
            ),
            SyntaxExpectation::new(
                vec![new!(SyntaxExpectedToken::WordCategory(
                    SyntaxWordCategory::Brivla,
                ))],
                new!(SyntaxExpectationReason::ContinueCurrent {
                    construct: "sumti".to_owned(),
                }),
            ),
        ];

        let text = segment_text(&syntax_detailed_segments(&expectations));

        let continue_argument = text
            .find("- BRIVLA [continues sumti]")
            .expect("sumti continuation");
        let start_argument = text.find("- sumti (lo)").expect("sumti start");
        let continue_relation = text
            .find("- GA [continues selbri]")
            .expect("selbri continuation");
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
                construct: "sumti".to_owned(),
            }),
        )];

        let text = segment_text(&syntax_detailed_segments(&expectations));

        assert!(text.contains("- sumti (BRIVLA, GAhO, be, or lo)"));
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
                    construct: "selbri".to_owned(),
                }),
            ),
            SyntaxExpectation::new(
                tokens,
                new!(SyntaxExpectationReason::ContinueCurrent {
                    construct: "sumti".to_owned(),
                }),
            ),
        ];

        let text = segment_text(&syntax_detailed_segments(&expectations));

        assert!(text.contains("- BIhI or SE [continues sumti]"));
        assert!(!text.contains("[continues selbri]"));
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
                construct: "sumti".to_owned(),
            }),
        )];

        let text = segment_text(&syntax_summary_segments_from_expectations(&expectations));

        assert_eq!(text, "expected one of: BRIVLA, GAhO, or lo");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn significant_construct_tree_collapses_to_immediate_child() {
        assert_eq!(
            syntax_immediate_child_under("sumti", "mex"),
            Some("number sumti".to_owned())
        );
        assert_eq!(
            syntax_immediate_child_under("number sumti", "mex"),
            Some("mex".to_owned())
        );
        assert!(syntax_construct_is_descendant_of(
            "free modifier",
            "metalinguistic comment"
        ));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn syntax_expectation_summary_message_uses_constructs_and_oxford_comma() {
        let expectations = vec![
            SyntaxExpectation::new(
                vec![new!(SyntaxExpectedToken::Selmaho(Selmaho::Sei))],
                new!(SyntaxExpectationReason::StartNested {
                    construct: "free modifier".to_owned(),
                }),
            ),
            SyntaxExpectation::new(
                vec![new!(SyntaxExpectedToken::WordCategory(
                    SyntaxWordCategory::LetterWord
                ))],
                new!(SyntaxExpectationReason::StartNested {
                    construct: "mex".to_owned(),
                }),
            ),
            SyntaxExpectation::new(
                vec![new!(SyntaxExpectedToken::WordCategory(
                    SyntaxWordCategory::Quote
                ))],
                new!(SyntaxExpectationReason::StartNested {
                    construct: "quote".to_owned(),
                }),
            ),
        ];

        assert_eq!(
            syntax_expectation_summary_message(&expectations, None),
            "expected: free modifier, mex, or quote"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn syntax_expectation_summary_message_collapses_to_summary_scope() {
        let expectations = vec![
            SyntaxExpectation::new(
                vec![new!(SyntaxExpectedToken::Selmaho(Selmaho::Lahe))],
                new!(SyntaxExpectationReason::StartNested {
                    construct: "converted sumti".to_owned(),
                }),
            ),
            SyntaxExpectation::new(
                vec![new!(SyntaxExpectedToken::Selmaho(Selmaho::Le))],
                new!(SyntaxExpectationReason::StartNested {
                    construct: "description".to_owned(),
                }),
            ),
            SyntaxExpectation::new(
                vec![new!(SyntaxExpectedToken::WordCategory(
                    SyntaxWordCategory::Brivla
                ))],
                new!(SyntaxExpectationReason::ContinueCurrent {
                    construct: "selbri".to_owned(),
                }),
            ),
            SyntaxExpectation::new(
                vec![new!(SyntaxExpectedToken::Selmaho(Selmaho::Sei))],
                new!(SyntaxExpectationReason::StartNested {
                    construct: "metalinguistic comment".to_owned(),
                }),
            ),
            SyntaxExpectation::new(
                vec![new!(SyntaxExpectedToken::EndOfInput)],
                new!(SyntaxExpectationReason::EndThenStart {
                    starts: "end of input".to_owned(),
                    ends: vec!["selbri".to_owned(), "statement".to_owned()],
                }),
            ),
        ];

        assert_eq!(
            syntax_expectation_summary_message(&expectations, Some("text")),
            "expected: free modifier, statement, or end of input"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn syntax_expectation_summary_message_omits_current_scope_when_alternatives_exist() {
        let expectations = vec![
            SyntaxExpectation::new(
                vec![new!(SyntaxExpectedToken::Selmaho(Selmaho::Sei))],
                new!(SyntaxExpectationReason::StartNested {
                    construct: "free modifier".to_owned(),
                }),
            ),
            SyntaxExpectation::new(
                vec![new!(SyntaxExpectedToken::WordCategory(
                    SyntaxWordCategory::Brivla
                ))],
                new!(SyntaxExpectationReason::StartNested {
                    construct: "bridi".to_owned(),
                }),
            ),
            SyntaxExpectation::new(
                vec![new!(SyntaxExpectedToken::Selmaho(Selmaho::Ja))],
                new!(SyntaxExpectationReason::ContinueCurrent {
                    construct: "statement".to_owned(),
                }),
            ),
        ];

        assert_eq!(
            syntax_expectation_summary_message(&expectations, Some("statement")),
            "expected: free modifier or bridi"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn structured_expected_notes_drop_duplicate_summary_note() {
        let expectations = vec![SyntaxExpectation::new(
            vec![new!(SyntaxExpectedToken::WordCategory(
                SyntaxWordCategory::LetterWord
            ))],
            new!(SyntaxExpectationReason::StartNested {
                construct: "mex".to_owned(),
            }),
        )];

        let notes = syntax_expected_notes(&["LERFU".to_owned()], &expectations);

        assert_eq!(notes.len(), 1);
        assert!(matches!(notes[0].mode, DiagnosticNoteMode::Detailed));
        let text = segment_text(&notes[0].segments);
        assert!(text.starts_with("needs one of:"));
        assert!(!text.contains("expected one of:"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn li_nu_error_reports_number_sumti_and_mex() {
        let source = "li nu";
        let words = jbotci_morphology::segment_words_with_modifiers(source).expect("valid words");
        let error = parse_syntax_tree(&words).expect_err("li requires a mex");

        let SyntaxError::Parse {
            reason,
            expectations,
            context,
            ..
        } = &error
        else {
            panic!("expected syntax parse error");
        };

        assert_eq!(reason, "expected: free modifier or mex");
        assert_eq!(
            context.as_ref().map(|context| context.construct.as_str()),
            Some("number sumti")
        );
        assert!(expectations.iter().any(|expectation| matches!(
            expectation.reason.as_data(),
            data!(SyntaxExpectationReason::StartNested { construct }) if construct == "free modifier"
        )));
        assert!(expectations.iter().any(|expectation| matches!(
            expectation.reason.as_data(),
            data!(SyntaxExpectationReason::StartNested { construct }) if construct == "mex"
        )));

        let diagnostic = error.to_diagnostic(None, source);
        assert_eq!(
            diagnostic.primary_label().message,
            "expected: free modifier or mex"
        );
        assert_eq!(diagnostic.styled_notes.len(), 1);
        assert!(matches!(
            diagnostic.styled_notes[0].mode,
            DiagnosticNoteMode::Detailed
        ));
        let note_text = segment_text(&diagnostic.styled_notes[0].segments);
        assert!(note_text.contains("needs one of:"));
        assert!(note_text.contains("LERFU"));
        assert!(!note_text.contains("LETTER WORD"));
        assert!(!note_text.contains("expected one of:"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn parser_wires_all_parser_diagnostic_constructs() {
        let parser_source = include_str!("grammar/parser.rs");

        for metadata in SYNTAX_CONSTRUCT_METADATA {
            if metadata.wiring == SyntaxConstructWiring::Synthetic {
                continue;
            }
            assert!(
                parser_source_wires_construct(parser_source, metadata.name),
                "parser-wired diagnostic construct {:?} is missing a parser label/context",
                metadata.name,
            );
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn truncated_forethought_forms_report_committed_constructs() {
        assert_error_context("ga mi broda gi", "forethought bridi connection");
        assert_error_mentions_construct("ga lo mlatu gi", "forethought sumti connection");
        assert_error_context("mi gu'e broda gi", "forethought selbri connection");
        assert_error_context("li ga pa gi", "forethought mex");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn syntax_error_kinds_use_found_word_categories() {
        assert_error_kind("ku", SyntaxErrorKind::UnexpectedCmavo);
        assert_error_kind("mi djan.", SyntaxErrorKind::UnexpectedCmevla);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn syntax_error_kinds_use_incomplete_parser_contexts() {
        assert_error_kind("lo", SyntaxErrorKind::IncompleteSumti);
        assert_error_kind("mi cu", SyntaxErrorKind::IncompleteBridi);
        assert_error_kind("mi sei", SyntaxErrorKind::IncompleteFreeModifier);
        assert_error_kind("li vei pa su'i", SyntaxErrorKind::IncompleteMekso);
        assert_error_kind(
            "ga lo mlatu gi",
            SyntaxErrorKind::IncompleteForethoughtConnection,
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn representative_constructs_appear_in_structured_expectations() {
        assert_error_mentions_construct("nu'i", "termset");
        assert_error_mentions_construct("lo pa", "quantifier");
        assert_error_mentions_construct("li peho", "operator");
        assert_error_mentions_construct("lo vei", "number sumti");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn quote_subtype_branches_still_parse() {
        for source in [
            "zo coi",
            "lu mi klama li'u",
            "lo'u coi rodo le'u",
            "zoi gy hello gy",
        ] {
            let words =
                jbotci_morphology::segment_words_with_modifiers(source).expect("valid morphology");
            parse_syntax_tree(&words).unwrap_or_else(|error| {
                panic!("quote source {source:?} should parse, got {error:?}");
            });
        }
    }

    #[cfg(feature = "grammar-debug")]
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn grammar_debug_ebnf_contains_terminal_labels() {
        let output = syntax_grammar_ebnf(&ParseOptions::default());

        assert!(output.contains("sumti"));
        assert!(output.contains("BRIVLA"));
        assert!(output.contains("QUOTE"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn syntax_tree_span_equality_ignores_source_offsets_only() {
        let left = syntax_tree_for_source("mi klama");
        let same_tree_different_spans = syntax_tree_for_source("mi  klama");
        let different_tree = syntax_tree_for_source("mi tavla");

        assert!(syntax_tree_eq_ignoring_spans(
            &left,
            &same_tree_different_spans
        ));
        assert!(!syntax_tree_eq_ignoring_spans(&left, &different_tree));
    }

    #[cfg(feature = "grammar-debug")]
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn grammar_debug_dialect_changes_generated_grammar() {
        let default_output = syntax_grammar_ebnf(&ParseOptions::default());
        let dialect = jbotci_dialect::parse_dialect_definition("(+ZANTUFA-QUOTES)")
            .expect("valid dialect definition");
        let zantufa_options = ParseOptions::default().with_dialect_definition(&dialect);
        let zantufa_output = syntax_grammar_ebnf(&zantufa_options);

        assert_ne!(default_output, zantufa_output);
        assert!(zantufa_output.contains("mu'oi"));
    }

    #[requires(true)]
    #[ensures(true)]
    fn recovered_statement_slots(
        text: &tree::recovered::TextSyntax,
    ) -> Vec<&RecoveredSyntax<tree::recovered::StatementSyntax>> {
        let mut statements = Vec::new();
        for paragraph in &text.paragraphs {
            let RecoveredSyntax::Valid(paragraph) = paragraph else {
                continue;
            };
            for statement in &paragraph.statements {
                let RecoveredSyntax::Valid(statement) = statement else {
                    continue;
                };
                if let Some(statement) = &statement.statement {
                    statements.push(statement.as_ref());
                }
            }
        }
        statements
    }

    #[requires(!expected.is_empty())]
    #[ensures(true)]
    fn assert_recovered_statement_invalid_text(
        statement: &RecoveredSyntax<tree::recovered::StatementSyntax>,
        expected: &str,
    ) {
        let RecoveredSyntax::Invalid(item) = statement else {
            panic!("expected invalid recovered syntax statement");
        };
        assert_eq!(item.text, expected);
    }

    #[requires(true)]
    #[ensures(true)]
    fn segment_text(segments: &[DiagnosticTextSegment]) -> String {
        segments
            .iter()
            .map(|segment| segment.text.as_str())
            .collect::<String>()
    }

    #[requires(true)]
    #[ensures(true)]
    fn parser_source_wires_construct(parser_source: &str, construct: &str) -> bool {
        let normalized = parser_source
            .chars()
            .filter(|ch| !ch.is_whitespace())
            .collect::<String>();
        let normalized_construct = construct
            .chars()
            .filter(|ch| !ch.is_whitespace())
            .collect::<String>();
        [
            format!("syntax_context(\"{normalized_construct}\""),
            format!("syntax_label(\"{normalized_construct}\""),
            format!(".labelled(\"{normalized_construct}\""),
        ]
        .into_iter()
        .any(|pattern| normalized.contains(&pattern))
    }

    #[requires(!source.is_empty())]
    #[ensures(true)]
    fn assert_error_context(source: &str, construct: &str) {
        let error = syntax_error_for_source(source);
        let SyntaxError::Parse { context, .. } = error else {
            panic!("expected syntax parse error for {source:?}");
        };
        assert_eq!(
            context.as_ref().map(|context| context.construct.as_str()),
            Some(construct),
            "unexpected diagnostic context for {source:?}",
        );
    }

    #[requires(!source.is_empty())]
    #[ensures(true)]
    fn assert_error_mentions_construct(source: &str, construct: &str) {
        let error = syntax_error_for_source(source);
        assert!(
            syntax_error_mentions_construct(&error, construct),
            "syntax error for {source:?} did not mention construct {construct:?}: {error:?}",
        );
    }

    #[requires(!source.is_empty())]
    #[ensures(true)]
    fn assert_error_kind(source: &str, expected_kind: SyntaxErrorKind) {
        let error = syntax_error_for_source(source);
        let SyntaxError::Parse { kind, .. } = &error else {
            panic!("expected syntax parse error for {source:?}");
        };
        assert_eq!(*kind, expected_kind, "unexpected kind for {source:?}");

        let diagnostic = error.to_diagnostic(None, source);
        assert_eq!(diagnostic.code, expected_kind.code(), "{source:?}");
        assert_eq!(diagnostic.message, expected_kind.message(), "{source:?}");
    }

    #[requires(!source.is_empty())]
    #[ensures(true)]
    fn syntax_error_for_source(source: &str) -> SyntaxError {
        let words = jbotci_morphology::segment_words_with_modifiers(source).expect("valid words");
        parse_syntax_tree(&words).expect_err("source should have a syntax error")
    }

    #[requires(!source.is_empty())]
    #[ensures(true)]
    fn syntax_tree_for_source(source: &str) -> TextSyntax {
        let words = jbotci_morphology::segment_words_with_modifiers(source).expect("valid words");
        parse_syntax_tree_with_source_and_options(&words, source, &ParseOptions::default())
            .expect("valid syntax")
            .parse_tree
            .as_ref()
            .clone()
    }

    #[requires(!construct.is_empty())]
    #[ensures(true)]
    fn syntax_error_mentions_construct(error: &SyntaxError, construct: &str) -> bool {
        let SyntaxError::Parse {
            expectations,
            context,
            ..
        } = error
        else {
            return false;
        };
        context
            .as_ref()
            .is_some_and(|context| context.construct == construct)
            || expectations
                .iter()
                .any(|expectation| expectation.reason.construct() == construct)
    }
}
