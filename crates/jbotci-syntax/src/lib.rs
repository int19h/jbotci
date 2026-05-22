//! Lojban syntax model and parser facade.

mod grammar;
pub mod source_tree;
pub mod tree;

extern crate self as jbotci_syntax;

use std::fmt;

#[allow(unused_imports)]
use bityzba::{data, ensures, expensive_invariant, invariant, new, requires};
use jbotci_dialect::DialectDefinition;
use jbotci_morphology::{Word, WordLike};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub mod ast {
    pub use crate::grammar::ast::*;
}
use ast::TextSyntax;
pub use jbotci_syntax_macros::{SourceTree, SyntaxTree};

#[invariant(indicator_data_is_valid(self.as_data()))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Indicator {
    pub indicator: Box<WithIndicators<WordLike>>,
    pub nai: Option<Box<Word>>,
}

impl Indicator {
    #[requires(true)]
    #[ensures(true)]
    pub fn new(indicator: WithIndicators<WordLike>, nai: Option<Word>) -> Self {
        new!(Indicator {
            indicator: Box::new(indicator),
            nai: nai.map(Box::new),
        })
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn words(&self) -> Vec<WithIndicators<WordLike>> {
        let mut words = vec![(*self.indicator).clone()];
        if let Some(nai) = &self.nai {
            words.push(WithIndicators::bare(WordLike::bare((**nai).clone())));
        }
        words
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&WithIndicators<WordLike>)) {
        visitor(&self.indicator);
        if let Some(nai) = &self.nai {
            let nai = WithIndicators::bare(WordLike::bare((**nai).clone()));
            visitor(&nai);
        }
    }

    #[requires(true)]
    #[ensures(ret >= 1)]
    pub fn word_count(&self) -> usize {
        1 + usize::from(self.nai.is_some())
    }
}

#[requires(true)]
#[ensures(true)]
fn indicator_data_is_valid(indicator: &IndicatorData) -> bool {
    indicator
        .indicator
        .visible_word()
        .is_some_and(is_indicator_word)
        && indicator
            .nai
            .as_deref()
            .is_none_or(|nai| nai.is_cmavo_text("nai"))
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WithIndicators<T> {
    Bare(Box<T>),
    Emphasized {
        bahe: Box<Word>,
        word_like: Box<T>,
    },
    WithIndicator {
        base: Box<WithIndicators<T>>,
        indicator: Box<Word>,
        nai: Option<Box<Word>>,
    },
}

impl<T> WithIndicators<T> {
    #[requires(true)]
    #[ensures(true)]
    pub fn bare(word_like: T) -> Self {
        WithIndicators::Bare(Box::new(word_like))
    }

    #[requires(bahe.selmaho() == Some("BAhE"))]
    #[ensures(true)]
    pub fn emphasized(bahe: Word, word_like: T) -> Self {
        WithIndicators::Emphasized {
            bahe: Box::new(bahe),
            word_like: Box::new(word_like),
        }
    }

    #[requires(is_indicator_word(&indicator))]
    #[requires(nai.as_ref().is_none_or(|nai| nai.is_cmavo_text("nai")))]
    #[ensures(true)]
    pub fn with_indicator(base: WithIndicators<T>, indicator: Word, nai: Option<Word>) -> Self {
        WithIndicators::WithIndicator {
            base: Box::new(base),
            indicator: Box::new(indicator),
            nai: nai.map(Box::new),
        }
    }
}

impl WithIndicators<WordLike> {
    #[requires(true)]
    #[ensures(true)]
    pub fn word_like(&self) -> Option<&WordLike> {
        match self {
            WithIndicators::Bare(word_like) | WithIndicators::Emphasized { word_like, .. } => {
                Some(word_like)
            }
            WithIndicators::WithIndicator { base, .. } => base.word_like(),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn visible_word(&self) -> Option<&Word> {
        self.word_like().and_then(WordLike::visible_base_word)
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn source_spans(&self) -> Vec<&jbotci_source::SourceSpan> {
        let mut spans = Vec::new();
        self.source_spans_into(&mut spans);
        spans
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn source_spans_into<'a>(&'a self, out: &mut Vec<&'a jbotci_source::SourceSpan>) {
        match self {
            WithIndicators::Bare(word_like) => word_like.source_spans_into(out),
            WithIndicators::Emphasized { bahe, word_like } => {
                out.push(&bahe.span);
                word_like.source_spans_into(out);
            }
            WithIndicators::WithIndicator {
                base,
                indicator,
                nai,
            } => {
                base.source_spans_into(out);
                out.push(&indicator.span);
                if let Some(nai) = nai {
                    out.push(&nai.span);
                }
            }
        }
    }
}

impl<T: fmt::Display> fmt::Display for WithIndicators<T> {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WithIndicators::Bare(word_like) => write!(f, "{word_like}"),
            WithIndicators::Emphasized { bahe, word_like } => {
                write!(f, "{bahe}-{word_like}")
            }
            WithIndicators::WithIndicator {
                base,
                indicator,
                nai,
            } => {
                write!(f, "{base}-{indicator}")?;
                if let Some(nai) = nai {
                    write!(f, "-{nai}")?;
                }
                Ok(())
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn is_indicator_word(word: &Word) -> bool {
    let canonical = word.canonical_phonemes();
    word.kind == jbotci_morphology::WordKind::Cmavo
        && (crate::grammar::tokens::UI_WORDS.contains(&canonical.as_str())
            || crate::grammar::tokens::CAI_WORDS.contains(&canonical.as_str())
            || canonical == "y")
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[invariant(true)]
pub struct TraceOptions {
    pub level: u8,
    pub filter: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[invariant(true)]
pub struct ParseOptions {
    pub trace: TraceOptions,
    pub dialect: DialectDefinition,
}

impl ParseOptions {
    #[requires(true)]
    #[ensures(ret.dialect == *definition)]
    pub fn with_dialect_definition(mut self, definition: &DialectDefinition) -> Self {
        self.dialect = definition.clone();
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
pub enum SyntaxError {
    #[error("syntax parsing is not implemented yet")]
    NotImplemented,
    #[error("syntax parse failed at byte {byte_offset}: {reason}")]
    Parse { byte_offset: usize, reason: String },
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

#[invariant(syntax_parse_data_is_valid(self.as_data()))]
#[expensive_invariant(syntax_parse_source_spans_are_ordered(self.as_data()))]
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SyntaxParse {
    pub parse_tree: TextSyntax,
    #[serde(default)]
    pub warnings: Vec<SyntaxWarning>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[invariant(true)]
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

#[invariant(syntax_warning_data_is_valid(self.as_data()))]
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
}

#[invariant(syntax_warning_display_data_is_valid(self.as_data()))]
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
    let message = experimental_cmavo.as_ref().map_or_else(
        || warning.message().to_owned(),
        |cmavo| format!("{}: {cmavo}", warning.message()),
    );
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
            .visible_word()
            .map(jbotci_morphology::Word::canonical_phonemes)
            .filter(|text| !text.trim().is_empty());
    }
    None
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
fn syntax_parse_data_is_valid(data: &SyntaxParseData) -> bool {
    data.warnings
        .iter()
        .all(|warning| syntax_warning_data_is_valid(warning.as_data()))
}

#[requires(true)]
#[ensures(true)]
fn syntax_parse_source_spans_are_ordered(data: &SyntaxParseData) -> bool {
    let data!(SyntaxParse { parse_tree, .. }) = data;
    let mut last_end = None;
    let mut ordered = true;
    parse_tree.visit_words(&mut |word| {
        if !ordered {
            return;
        }
        for span in word.source_spans() {
            if last_end.is_some_and(|end| end > span.byte_start) {
                ordered = false;
                return;
            }
            last_end = Some(span.byte_end);
        }
    });
    ordered
}

#[requires(true)]
#[ensures(true)]
fn syntax_warning_data_is_valid(data: &SyntaxWarningData) -> bool {
    let data!(SyntaxWarning { anchor, .. }) = data;
    !anchor.source_spans().is_empty()
}

#[requires(true)]
#[ensures(true)]
fn syntax_warning_display_data_is_valid(data: &SyntaxWarningDisplayData) -> bool {
    let data!(SyntaxWarningDisplay {
        source_label,
        message,
        line,
        column,
        context,
        ..
    }) = data;
    !source_label.is_empty()
        && !message.is_empty()
        && *line > 0
        && *column > 0
        && !context.is_empty()
}
