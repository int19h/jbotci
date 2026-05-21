//! Lojban syntax model and parser facade.

mod grammar;

use std::fmt;

use bityzba::{data, invariant, new, requires};
use jbotci_dialect::DialectDefinition;
use jbotci_morphology::{Word, WordLike};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub mod ast {
    pub use crate::grammar::ast::*;
}
use ast::TextSyntax;

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
fn syntax_warning_data_is_valid(data: &SyntaxWarningData) -> bool {
    let data!(SyntaxWarning { .. }) = data;
    true
}
