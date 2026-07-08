//! Lojban syntax model and parser facade.

pub mod tree;
pub use tree::{
    Token, WithIndicators, WithIndicatorsData, elidable_terminator_for_absent_field_ref,
};

mod grammar;

extern crate self as jbotci_syntax;

use std::cmp::Ordering;

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
use jbotci_tree::TreeVisitor;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

#[doc(hidden)]
pub mod generated_model {
    pub use crate::grammar::generated_model::*;
}
pub use generated_model::TextSyntax;

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

impl generated_model::TextSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_source_spans(&self, visitor: &mut impl FnMut(&SourceSpan)) {
        let mut span_visitor = GeneratedModelSourceSpanVisitor { visitor };
        generated_model::TreeNode::visit_in_order(self, &mut span_visitor);
    }
}

#[invariant(true)]
struct GeneratedModelSourceSpanVisitor<'a, F>
where
    F: FnMut(&SourceSpan),
{
    visitor: &'a mut F,
}

impl<F> GeneratedModelSourceSpanVisitor<'_, F>
where
    F: FnMut(&SourceSpan),
{
    #[requires(true)]
    #[ensures(true)]
    fn visit_token(&mut self, token: &Token) {
        for span in token.source_spans() {
            (self.visitor)(span);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_generated_tree<T>(&mut self, value: &T)
    where
        T: generated_model::TreeNode,
    {
        value.visit_in_order(self);
    }
}

impl<'tree, F> TreeVisitor<'tree> for GeneratedModelSourceSpanVisitor<'_, F>
where
    F: FnMut(&SourceSpan),
{
    type Node = generated_model::NodeRef<'tree>;
    type Atom = generated_model::AtomRef<'tree>;

    #[requires(true)]
    #[ensures(true)]
    fn visit_atom(&mut self, atom: Self::Atom) {
        match atom {
            generated_model::AtomRef::Token(token) => self.visit_token(token),
        }
    }
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn text_syntax_leaf_spans_match_words(
    words: &[WordLike],
    parse_tree: &TextSyntax,
) -> bool {
    generated_model_text_syntax_leaf_spans_match_words(words, parse_tree)
}

#[requires(true)]
#[ensures(true)]
pub fn generated_model_text_syntax_leaf_spans_match_words(
    words: &[WordLike],
    parse_tree: &generated_model::TextSyntax,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[invariant(true)]
pub struct ParseOptions {
    pub trace: TraceOptions,
    pub dialect: DialectDefinition,
    pub error_context_depth: usize,
}

impl Default for ParseOptions {
    #[requires(true)]
    #[ensures(true)]
    fn default() -> Self {
        Self {
            trace: TraceOptions::default(),
            dialect: DialectDefinition::default(),
            error_context_depth: 1,
        }
    }
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

    #[requires(true)]
    #[ensures(ret.error_context_depth == depth)]
    pub fn with_error_context_depth(mut self, depth: usize) -> Self {
        self.error_context_depth = depth;
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
        contexts: Vec<SyntaxConstructContext>,
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
enum SyntaxConstructIncompleteAttribution {
    Direct,
    GenericConnectiveParent,
    GenericConnectiveContext,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
struct SyntaxConstructMetadata {
    name: &'static str,
    parent: Option<&'static str>,
    incomplete_attribution: SyntaxConstructIncompleteAttribution,
    wiring: SyntaxConstructWiring,
}

const SYNTAX_CONSTRUCT_METADATA: &[SyntaxConstructMetadata] = &[
    SyntaxConstructMetadata {
        name: "bridi",
        parent: Some("statement"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "prenex",
        parent: Some("statement"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "text group",
        parent: Some("statement"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "statement",
        parent: Some("text"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "fragment",
        parent: Some("text"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "free modifier",
        parent: Some("text"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "terms",
        parent: Some("bridi"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "tail terms",
        parent: Some("bridi"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Synthetic,
    },
    SyntaxConstructMetadata {
        name: "forethought bridi connection",
        parent: Some("bridi"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "term",
        parent: Some("terms"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "termset",
        parent: Some("terms"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "sumti",
        parent: Some("term"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "tag",
        parent: Some("term"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "place tag",
        parent: Some("term"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "NA KU term",
        parent: Some("term"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "description",
        parent: Some("sumti"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "pro-sumti",
        parent: Some("sumti"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Synthetic,
    },
    SyntaxConstructMetadata {
        name: "name",
        parent: Some("sumti"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "quote",
        parent: Some("sumti"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "number sumti",
        parent: Some("sumti"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "lerfu string",
        parent: Some("sumti"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "converted sumti",
        parent: Some("sumti"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "bridi description",
        parent: Some("sumti"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "forethought sumti connection",
        parent: Some("sumti"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "relative clauses",
        parent: Some("sumti"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "descriptor",
        parent: Some("description"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "description tail",
        parent: Some("description"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "relative clause",
        parent: Some("relative clauses"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "relative bridi",
        parent: Some("relative clause"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "sumti association phrase",
        parent: Some("relative clause"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "mex",
        parent: Some("number sumti"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "operand",
        parent: Some("mex"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "operator",
        parent: Some("mex"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "forethought mex",
        parent: Some("mex"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "reverse Polish mex",
        parent: Some("mex"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "number",
        parent: Some("operand"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "parenthesized mex",
        parent: Some("operand"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "selbri operand",
        parent: Some("operand"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "sumti operand",
        parent: Some("operand"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "mekso array",
        parent: Some("operand"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "qualified operand",
        parent: Some("operand"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "VUhU operator",
        parent: Some("operator"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "operand-to-operator",
        parent: Some("operator"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "selbri-to-operator",
        parent: Some("operator"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "converted operator",
        parent: Some("operator"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "selbri",
        parent: Some("bridi"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "negated selbri",
        parent: Some("selbri"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "forethought selbri connection",
        parent: Some("selbri"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "tanru",
        parent: Some("selbri"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "tanru unit",
        parent: Some("tanru"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "abstraction",
        parent: Some("tanru unit"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "grouped tanru",
        parent: Some("tanru unit"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "sumti-to-selbri",
        parent: Some("tanru unit"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "operator-to-selbri",
        parent: Some("tanru unit"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "ordinal selbri",
        parent: Some("tanru unit"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "converted tanru unit",
        parent: Some("tanru unit"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "modal conversion",
        parent: Some("tanru unit"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "linked arguments",
        parent: Some("tanru unit"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "selbri relative phrase",
        parent: Some("tanru unit"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Synthetic,
    },
    SyntaxConstructMetadata {
        name: "subbridi",
        parent: Some("abstraction"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "quantifier",
        parent: Some("description"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "simple tense/modal",
        parent: Some("tag"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Synthetic,
    },
    SyntaxConstructMetadata {
        name: "FIhO modal",
        parent: Some("tag"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "connected tag",
        parent: Some("tag"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "modal tag",
        parent: Some("simple tense/modal"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "time tense",
        parent: Some("simple tense/modal"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "space tense",
        parent: Some("simple tense/modal"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "vocative phrase",
        parent: Some("free modifier"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "parenthetical text",
        parent: Some("free modifier"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "metalinguistic comment",
        parent: Some("free modifier"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "reciprocal",
        parent: Some("free modifier"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "subscript",
        parent: Some("free modifier"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "utterance ordinal",
        parent: Some("free modifier"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "replacement phrase",
        parent: Some("free modifier"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "word quote",
        parent: Some("quote"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Synthetic,
    },
    SyntaxConstructMetadata {
        name: "text quote",
        parent: Some("quote"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "word-sequence quote",
        parent: Some("quote"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Synthetic,
    },
    SyntaxConstructMetadata {
        name: "non-Lojban quote",
        parent: Some("quote"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Synthetic,
    },
    SyntaxConstructMetadata {
        name: "paragraphs",
        parent: Some("text"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "paragraph",
        parent: Some("paragraphs"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "paragraph statement sequence",
        parent: Some("paragraph"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "paragraph statement",
        parent: Some("paragraph statement sequence"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "statement connection",
        parent: Some("statement"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "statement connective",
        parent: Some("statement connection"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "statement branch",
        parent: Some("statement"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "bridi continuation",
        parent: Some("statement"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "text connective",
        parent: Some("text"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "leading indicator",
        parent: Some("text"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "bridi tail",
        parent: Some("bridi"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "bridi tail connective",
        parent: Some("bridi tail"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "forethought bridi branch",
        parent: Some("forethought bridi connection"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "term connection",
        parent: Some("term"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "term connection continuation",
        parent: Some("term connection"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "term connective",
        parent: Some("term connection"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "termset connection",
        parent: Some("termset"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "termset connection continuation",
        parent: Some("termset connection"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "termset connective",
        parent: Some("termset connection"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "termset continuation",
        parent: Some("termset"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "NOIhA adverbial",
        parent: Some("term"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "FIhOI adverbial",
        parent: Some("term"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "SOI adverbial",
        parent: Some("term"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "NA term",
        parent: Some("term"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "converted term",
        parent: Some("sumti"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "scalar-negated term",
        parent: Some("sumti"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "elided sumti",
        parent: Some("sumti"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "sumti connection",
        parent: Some("sumti"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "sumti connective",
        parent: Some("sumti connection"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "sumti relative phrase",
        parent: Some("sumti"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "quantified sumti",
        parent: Some("sumti"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "scalar-negated sumti",
        parent: Some("sumti"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "tagged sumti",
        parent: Some("sumti association phrase"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "descriptor connective",
        parent: Some("description"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "relative clause connective",
        parent: Some("relative clauses"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "number mex",
        parent: Some("number"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "number continuation",
        parent: Some("number"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "number or lerfu string",
        parent: Some("operand"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "lerfu word",
        parent: Some("lerfu string"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "lerfu string continuation",
        parent: Some("lerfu string"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "grouped mex",
        parent: Some("mex"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "mex precedence tail",
        parent: Some("mex"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "mex continuation",
        parent: Some("mex"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "reverse Polish mex tail",
        parent: Some("reverse Polish mex"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "operand connective",
        parent: Some("operand"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "operand continuation",
        parent: Some("operand connective"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "scalar-negated operand",
        parent: Some("operand"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "operator continuation",
        parent: Some("operator"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "grouped operator",
        parent: Some("operator"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "sumti-to-operator",
        parent: Some("operator"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "connective operator",
        parent: Some("operator"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "selbri connection",
        parent: Some("selbri"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "selbri connection continuation",
        parent: Some("selbri connection"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "selbri connective",
        parent: Some("selbri connection"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "tagged selbri",
        parent: Some("selbri"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "tanru unit continuation",
        parent: Some("tanru"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "BO-grouped tanru unit",
        parent: Some("tanru unit"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "pro-bridi assignment",
        parent: Some("tanru unit"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "pro-bridi",
        parent: Some("tanru unit"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "scalar-negated tanru unit",
        parent: Some("tanru unit"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "quoted bridi selbri",
        parent: Some("tanru unit"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "text selbri",
        parent: Some("tanru unit"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "quoted text selbri",
        parent: Some("tanru unit"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "tag selbri",
        parent: Some("tanru unit"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "mex selbri",
        parent: Some("tanru unit"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "sumti selbri",
        parent: Some("tanru unit"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "abstractor connection",
        parent: Some("abstraction"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    // `ek` is shared by sumti, termset, and operand connections; the sumti
    // connective branch is the canonical diagnostic parent because it is the
    // closest user-facing EK connection class and keeps EK failures in the
    // sumti/term family instead of the broader operand/mekso family.
    SyntaxConstructMetadata {
        name: "ek",
        parent: Some("sumti connective"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::GenericConnectiveParent,
        wiring: SyntaxConstructWiring::Parser,
    },
    // `jek` is reused by statement, selbri, and operator connections. The
    // selbri connective parent matches the existing connective hierarchy and
    // gives the most useful incomplete-selbri attribution for bare JA/JAI
    // connection failures.
    SyntaxConstructMetadata {
        name: "jek",
        parent: Some("selbri connective"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::GenericConnectiveParent,
        wiring: SyntaxConstructWiring::Parser,
    },
    // `joik` spans sumti, term, selbri, bridi-tail, statement, and operator
    // connections. The selbri connective parent is the canonical midpoint used
    // by neighboring connective entries; more specific parser contexts can
    // still refine to sumti, term, or mekso constructs when available.
    SyntaxConstructMetadata {
        name: "joik",
        parent: Some("selbri connective"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::GenericConnectiveParent,
        wiring: SyntaxConstructWiring::Parser,
    },
    // `interval` is the BIhI/GAhO branch inside `joik`, so its canonical
    // parent is `joik` even though joik itself is reused by several connection
    // families.
    SyntaxConstructMetadata {
        name: "interval",
        parent: Some("joik"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::GenericConnectiveParent,
        wiring: SyntaxConstructWiring::Parser,
    },
    // This is the grammar-level non-logical connective class around JOI/BIhI.
    // It follows `joik` to the selbri connective branch so diagnostics stay
    // consistent with neighboring logical/non-logical connective entries.
    SyntaxConstructMetadata {
        name: "non-logical connective",
        parent: Some("selbri connective"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::GenericConnectiveParent,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "gihek",
        parent: Some("bridi tail connective"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "forethought selbri connective",
        parent: Some("forethought selbri connection"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    // The unqualified forethought connective label is emitted for the generic
    // GA/GI connective shape. `forethought bridi connection` is the canonical
    // parent; parser contexts with known sumti, selbri, or mex ancestry use the
    // specialized forethought construct entries instead.
    SyntaxConstructMetadata {
        name: "forethought connective",
        parent: Some("forethought bridi connection"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::GenericConnectiveContext,
        wiring: SyntaxConstructWiring::Parser,
    },
    // Tag connectives occur in both tense/modal tags and simple tags. The
    // connected-tag parent keeps them under the adverbial/tag diagnostic branch
    // rather than forcing a time/space-specific parent too early.
    SyntaxConstructMetadata {
        name: "tag connective",
        parent: Some("connected tag"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::GenericConnectiveParent,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "connected tag continuation",
        parent: Some("connected tag"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "interval property",
        parent: Some("simple tense/modal"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "time interval",
        parent: Some("time tense"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "space interval",
        parent: Some("space tense"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "space interval property",
        parent: Some("space interval"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "vocative marker",
        parent: Some("vocative phrase"),
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "text",
        parent: None,
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Parser,
    },
    SyntaxConstructMetadata {
        name: "parse_text",
        parent: None,
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Synthetic,
    },
    SyntaxConstructMetadata {
        name: "end of input",
        parent: None,
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
        wiring: SyntaxConstructWiring::Synthetic,
    },
    SyntaxConstructMetadata {
        name: "syntax construct",
        parent: None,
        incomplete_attribution: SyntaxConstructIncompleteAttribution::Direct,
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
#[requires(syntax_construct_is_known(construct))]
#[ensures(ret < SYNTAX_CONSTRUCT_METADATA.len())]
pub(crate) fn syntax_construct_depth(construct: &str) -> usize {
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
#[ensures(ret -> syntax_construct_is_known(construct))]
pub(crate) fn syntax_construct_uses_generic_incomplete_attribution(construct: &str) -> bool {
    syntax_construct_metadata(construct).is_some_and(|metadata| {
        matches!(
            metadata.incomplete_attribution,
            SyntaxConstructIncompleteAttribution::GenericConnectiveParent
                | SyntaxConstructIncompleteAttribution::GenericConnectiveContext
        )
    })
}

#[requires(!construct.is_empty())]
#[ensures(ret.as_ref().is_none_or(|parent| !parent.is_empty()))]
pub(crate) fn syntax_construct_generic_incomplete_parent(construct: &str) -> Option<&'static str> {
    let metadata = syntax_construct_metadata(construct)?;
    if metadata.incomplete_attribution
        != SyntaxConstructIncompleteAttribution::GenericConnectiveParent
    {
        return None;
    }
    metadata.parent
}

#[requires(!construct.is_empty())]
#[requires(syntax_construct_is_known(construct))]
#[ensures(ret == matches!(construct, "text" | "parse_text"))]
pub(crate) fn syntax_construct_is_root(construct: &str) -> bool {
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
                contexts,
            } => {
                let span = source_span_from_byte_offsets(
                    source_id.clone(),
                    source,
                    *byte_start,
                    *byte_end,
                )
                .expect("syntax errors store offsets derived from the same source text");
                let mut labels = vec![DiagnosticLabel::new(span, reason.clone(), true)];
                for context in contexts {
                    let context_span = source_span_from_byte_offsets(
                        source_id.clone(),
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

#[requires(!expected.is_empty())]
#[ensures(!ret.is_empty())]
fn syntax_summary_segments_from_strings(expected: &[String]) -> Vec<DiagnosticTextSegment> {
    let mut segments = vec![
        keyword_segment("expected one of"),
        punctuation_segment(": "),
    ];
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
    let mut segments = vec![keyword_segment("needs one of"), punctuation_segment(":")];
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
    grammar::parse_generated_model_syntax_tree_with_source(words, None, options)
        .map(|parse_tree| *parse_tree)
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
    ExperimentalZantufaNaryForethought,
    ExperimentalZantufaGek,
    ExperimentalZantufaPoihaBrigahi,
    ExperimentalZantufaJaiTagTerm,
    ExperimentalZantufaRecursiveTag,
    ExperimentalZantufaGroupedBridiTail,
    ExperimentalZantufaStatementTerms,
    ExperimentalZantufaStatementRelativeClause,
    ExperimentalZantufaStatementFreeModifier,
    ExperimentalZantufaStatementAbstraction,
    ExperimentalZantufaMex,
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
            Self::ExperimentalZantufaNaryForethought => {
                "syntax.warning.experimental-zantufa-nary-forethought"
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
            Self::ExperimentalZantufaGroupedBridiTail => {
                "syntax.warning.experimental-zantufa-grouped-bridi-tail"
            }
            Self::ExperimentalZantufaStatementTerms => {
                "syntax.warning.experimental-zantufa-statement-terms"
            }
            Self::ExperimentalZantufaStatementRelativeClause => {
                "syntax.warning.experimental-zantufa-statement-relative-clause"
            }
            Self::ExperimentalZantufaStatementFreeModifier => {
                "syntax.warning.experimental-zantufa-statement-free-modifier"
            }
            Self::ExperimentalZantufaStatementAbstraction => {
                "syntax.warning.experimental-zantufa-statement-abstraction"
            }
            Self::ExperimentalZantufaMex => "syntax.warning.experimental-zantufa-mex",
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
            Self::ExperimentalZantufaNaryForethought => "Zantufa n-ary forethought branch",
            Self::ExperimentalZantufaGek => "Zantufa forethought connective form",
            Self::ExperimentalZantufaPoihaBrigahi => {
                "Zantufa POIhA briga'i term with KU terminator"
            }
            Self::ExperimentalZantufaJaiTagTerm => "Zantufa JAI tag term",
            Self::ExperimentalZantufaRecursiveTag => "Zantufa recursive SE/NAhE tag prefix",
            Self::ExperimentalZantufaGroupedBridiTail => "Zantufa KE bridi-tail grouping",
            Self::ExperimentalZantufaStatementTerms => "Zantufa statement-level trailing terms",
            Self::ExperimentalZantufaStatementRelativeClause => {
                "Zantufa statement payload in relative clause"
            }
            Self::ExperimentalZantufaStatementFreeModifier => {
                "Zantufa statement payload in SEI free modifier"
            }
            Self::ExperimentalZantufaStatementAbstraction => {
                "Zantufa statement payload in abstraction"
            }
            Self::ExperimentalZantufaMex => "Zantufa mex grammar form",
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
    parse_syntax_tree_with_source_and_options_attempt_inner(words, None, options).result
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
    parse_syntax_tree_with_source_and_options_attempt_inner(words, Some(source), options).result
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
    parse_syntax_tree_with_source_and_options_attempt_inner(words, Some(source), options)
}

#[requires(true)]
#[ensures(true)]
#[expensive_ensures(ret.result.as_ref().map_or(true, |parse| {
    syntax_parse_leaf_spans_match_words(words, parse)
}))]
fn parse_syntax_tree_with_source_and_options_attempt_inner(
    words: &[WordLike],
    source: Option<&str>,
    options: &ParseOptions,
) -> SyntaxParseAttempt {
    grammar::parse_generated_model_syntax_tree_with_source_attempt(words, source, options)
}

#[doc(hidden)]
#[requires(true)]
#[ensures(true)]
pub fn parse_syntax_tree_generated_model_with_source_and_options(
    words: &[WordLike],
    source: &str,
    options: &ParseOptions,
) -> Result<Box<generated_model::TextSyntax>, SyntaxError> {
    grammar::parse_generated_model_syntax_tree_with_source(words, Some(source), options)
}

#[doc(hidden)]
#[requires(true)]
#[ensures(true)]
#[expensive_ensures(ret.result.as_ref().map_or(true, |parse| {
    generated_model_text_syntax_leaf_spans_match_words(words, &parse.parse_tree)
}))]
pub fn parse_syntax_tree_generated_model_with_source_and_options_attempt(
    words: &[WordLike],
    source: &str,
    options: &ParseOptions,
) -> SyntaxParseAttempt {
    grammar::parse_generated_model_syntax_tree_with_source_attempt(words, Some(source), options)
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
    use std::collections::BTreeSet;

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
    fn li_nu_error_reports_mex_expectation() {
        let source = "li nu";
        let words = jbotci_morphology::segment_words_with_modifiers(source).expect("valid words");
        let error = parse_syntax_tree(&words).expect_err("li requires a mex");

        let SyntaxError::Parse {
            reason,
            expectations,
            contexts: _,
            ..
        } = &error
        else {
            panic!("expected syntax parse error");
        };

        assert!(reason.contains("free modifier"), "{reason}");
        assert!(reason.contains("mex"), "{reason}");
        assert!(expectations.iter().any(|expectation| matches!(
            expectation.reason.as_data(),
            data!(SyntaxExpectationReason::StartNested { construct }) if construct.contains("mex")
        )));

        let diagnostic = error.to_diagnostic(None, source);
        assert_eq!(diagnostic.primary_label().message, reason.as_str());
        assert_eq!(diagnostic.styled_notes.len(), 1);
        assert!(matches!(
            diagnostic.styled_notes[0].mode,
            DiagnosticNoteMode::Detailed
        ));
        let note_text = segment_text(&diagnostic.styled_notes[0].segments);
        assert!(note_text.contains("needs one of:"));
        assert!(!note_text.contains("expected one of:"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn parser_wires_all_parser_diagnostic_constructs() {
        let parser_source = include_str!("grammar/generated.rs");

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
    fn generated_parser_contexts_all_have_diagnostic_metadata_or_frozen_debt() {
        let generated_contexts = generated_model::GENERATED_MODEL_CONSTRUCTOR_LABELS
            .iter()
            .map(|(_, construct)| *construct)
            .collect::<BTreeSet<_>>();

        let missing = generated_contexts
            .iter()
            .copied()
            .filter(|construct| !syntax_construct_is_known(construct))
            .collect::<Vec<_>>();
        assert!(
            missing.is_empty(),
            "generated parser contexts missing syntax construct metadata: {missing:?}",
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn generated_elidable_terminators_drive_absent_field_lookup() {
        let mut seen = BTreeSet::new();
        for terminator in generated_model::GENERATED_MODEL_ELIDABLE_TERMINATORS {
            assert!(
                seen.insert(terminator.field),
                "duplicate elidable terminator field {:?}",
                terminator.field
            );
            assert_eq!(
                elidable_terminator_for_absent_field_ref(jbotci_tree::FieldRef::new(
                    Some(terminator.field),
                    0,
                    false,
                )),
                Some(terminator.cmavo)
            );
        }

        assert_eq!(
            elidable_terminator_for_absent_field_ref(jbotci_tree::FieldRef::new(
                Some("lihau"),
                0,
                false,
            )),
            Some(Cmavo::Lihau)
        );
        assert_eq!(
            elidable_terminator_for_absent_field_ref(jbotci_tree::FieldRef::new(
                Some("liau"),
                0,
                false,
            )),
            None
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn truncated_forethought_forms_report_structured_expectations() {
        assert_error_mentions_construct("ga mi broda gi", "forethought bridi branch");
        assert_error_mentions_construct("ga lo mlatu gi", "forethought sumti connection");
        assert_error_mentions_construct("mi gu'e broda gi", "forethought selbri connection");
        assert_error_mentions_construct("li ga pa gi", "forethought mex");
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
    fn syntax_error_kinds_cover_generated_contexts() {
        assert_error_kind("lo", SyntaxErrorKind::IncompleteSumti);
        assert_error_kind("se", SyntaxErrorKind::IncompleteSumti);
        assert_error_kind("te", SyntaxErrorKind::IncompleteSumti);
        assert_error_kind("nu", SyntaxErrorKind::IncompleteSelbri);
        assert_error_kind("ga'oga'i ki'a", SyntaxErrorKind::IncompleteSelbri);
        assert_error_kind("because", SyntaxErrorKind::IncompleteSelbri);
        assert_error_kind("xi", SyntaxErrorKind::IncompleteFreeModifier);
        assert_error_kind("li peho suhi", SyntaxErrorKind::IncompleteMekso);
        assert_error_kind(
            "ga lo mlatu gi",
            SyntaxErrorKind::IncompleteForethoughtConnection,
        );
        assert_error_kind(
            "ga mi broda gi",
            SyntaxErrorKind::IncompleteForethoughtConnection,
        );
        assert_error_kind("po li ce", SyntaxErrorKind::IncompleteMekso);
        assert_error_kind("voi ce", SyntaxErrorKind::IncompleteSumti);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn syntax_error_context_depth_controls_secondary_labels() {
        run_on_fixture_worker_stack(|| {
            let source = "cadga fa lo nu ro lo prenu goi ko'a cu troci lo nu ko'a tarti lo ku ce'u xendo je cnikansa ro lo jmive ta'i lo racli";
            let words =
                jbotci_morphology::segment_words_with_modifiers(source).expect("valid words");

            let no_context_error = parse_syntax_tree_with_source_and_options(
                &words,
                source,
                &ParseOptions::default().with_error_context_depth(0),
            )
            .expect_err("source should have a syntax error");
            let SyntaxError::Parse { contexts, .. } = &no_context_error else {
                panic!("expected syntax parse error");
            };
            assert!(contexts.is_empty());
            assert_eq!(no_context_error.to_diagnostic(None, source).labels.len(), 1);

            let nested_context_error = parse_syntax_tree_with_source_and_options(
                &words,
                source,
                &ParseOptions::default().with_error_context_depth(2),
            )
            .expect_err("source should have a syntax error");
            let SyntaxError::Parse {
                byte_end, contexts, ..
            } = &nested_context_error
            else {
                panic!("expected syntax parse error");
            };
            assert_eq!(contexts.len(), 2);
            assert_eq!(
                nested_context_error
                    .to_diagnostic(None, source)
                    .labels
                    .len(),
                3
            );
            assert_eq!(contexts[0].byte_end, *byte_end);
            assert_eq!(contexts[1].byte_end, *byte_end);
            assert!(contexts[1].byte_start <= contexts[0].byte_start);
            assert_ne!(contexts[0].byte_start, *byte_end);
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn representative_constructs_appear_in_structured_expectations() {
        assert_error_mentions_construct("nu'i", "termset");
        assert_error_mentions_construct("vei", "quantifier");
        assert_error_mentions_construct("li peho suhi", "operator");
        assert_error_mentions_construct("li ga pa gi", "forethought mex");
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

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn parse_tree_wrapper_accepts_baseline_quantified_fragments() {
        for source in ["iso'i melbi nixli", "vei ny. lo prenu", "reroi"] {
            let words =
                jbotci_morphology::segment_words_with_modifiers(source).expect("valid morphology");
            parse_syntax_tree_generated_model_with_source_and_options(
                &words,
                source,
                &ParseOptions::default(),
            )
            .unwrap_or_else(|error| {
                panic!("{source:?} should parse as generated model, got {error}")
            });
            parse_syntax_tree_with_source_and_options(&words, source, &ParseOptions::default())
                .unwrap_or_else(|error| panic!("{source:?} should parse, got {error}"));
        }
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
    fn run_on_fixture_worker_stack(test: impl FnOnce() + Send + 'static) {
        let handle = std::thread::Builder::new()
            .stack_size(8 * 1024 * 1024)
            .spawn(test)
            .expect("fixture worker stack test thread should spawn");
        if let Err(panic) = handle.join() {
            std::panic::resume_unwind(panic);
        }
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
            format!("rule\"{normalized_construct}\""),
            format!("alias\"{normalized_construct}\""),
            format!("context\"{normalized_construct}\""),
            format!("syntax_context(\"{normalized_construct}\""),
            format!("Some(\"{normalized_construct}\")"),
            format!("syntax_label(\"{normalized_construct}\""),
            format!(".labelled(\"{normalized_construct}\""),
        ]
        .into_iter()
        .any(|pattern| normalized.contains(&pattern))
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
            contexts,
            ..
        } = error
        else {
            return false;
        };
        contexts
            .iter()
            .any(|context| context.construct == construct)
            || expectations
                .iter()
                .any(|expectation| expectation.reason.construct() == construct)
    }
}
