//! Source-backed syntax AST model and generated tree traversal.

// The syntax AST mirrors the source grammar and v0 constructors.
// Boxing only for enum-size symmetry would obscure that shape during the port.
#![allow(clippy::large_enum_variant)]

use std::{fmt, sync::Arc};

#[allow(unused_imports)]
use bityzba::{contract_trait, ensures, invariant, new, requires};
use jbotci_morphology::{Cmavo, Selmaho, Word, WordLike};
use jbotci_tree::FieldRef;
use serde::{Deserialize, Serialize};
use vec1::Vec1;

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WithFreeModifiers<T> {
    pub value: T,
    pub free_modifiers: Vec<FreeModifierSyntax>,
}

#[invariant(true)]
#[invariant(::Bare(_) => true)]
#[invariant(::Emphasized => true)]
#[invariant(::WithIndicator => true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WithIndicators<T> {
    Bare(T),
    Emphasized {
        bahe: Word,
        word_like: T,
    },
    WithIndicator {
        base: Arc<WithIndicators<T>>,
        indicator: Word,
        nai: Option<Word>,
    },
}

impl<T> WithIndicators<T> {
    #[requires(true)]
    #[ensures(true)]
    pub fn bare(word_like: T) -> Self {
        WithIndicators::Bare(word_like)
    }

    #[requires(bahe.is_selmaho(Selmaho::Bahe))]
    #[ensures(true)]
    pub fn emphasized(bahe: Word, word_like: T) -> Self {
        WithIndicators::Emphasized { bahe, word_like }
    }

    #[requires(crate::is_indicator_word(&indicator))]
    #[requires(nai.as_ref().is_none_or(|nai| nai.is_cmavo(Cmavo::Nai)))]
    #[ensures(true)]
    pub fn with_indicator(base: WithIndicators<T>, indicator: Word, nai: Option<Word>) -> Self {
        WithIndicators::WithIndicator {
            base: Arc::new(base),
            indicator,
            nai,
        }
    }
}

#[invariant(self.core_word().byte_range().is_some(), "syntax tokens must cover source bytes")]
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Token(Arc<WithIndicators<WordLike>>);

impl Token {
    #[requires(true)]
    #[ensures(true)]
    pub fn from_indicators(indicators: WithIndicators<WordLike>) -> Self {
        new!(Token(Arc::new(indicators)))
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn bare(word_like: WordLike) -> Self {
        Self::from_indicators(WithIndicators::bare(word_like))
    }

    #[requires(bahe.is_selmaho(Selmaho::Bahe))]
    #[ensures(true)]
    pub fn emphasized(bahe: Word, word_like: WordLike) -> Self {
        Self::from_indicators(WithIndicators::emphasized(bahe, word_like))
    }

    #[requires(crate::is_indicator_word(&indicator))]
    #[requires(nai.as_ref().is_none_or(|nai| nai.is_cmavo(Cmavo::Nai)))]
    #[ensures(true)]
    pub fn with_indicator(base: Token, indicator: Word, nai: Option<Word>) -> Self {
        new!(Token(Arc::new(WithIndicators::WithIndicator {
            base: Arc::clone(base.as_data()),
            indicator,
            nai,
        })))
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn as_indicators(&self) -> &WithIndicators<WordLike> {
        self.as_data().as_ref()
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn ptr_eq(left: &Self, right: &Self) -> bool {
        Arc::ptr_eq(left.as_data(), right.as_data())
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn core_word(&self) -> &WordLike {
        self.as_indicators().core_word()
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn quote_marker_cmavo(&self) -> Option<Cmavo> {
        self.as_indicators().quote_marker_cmavo()
    }

    #[requires(true)]
    #[ensures(ret == (self.cmavo() == Some(cmavo)))]
    pub fn is_cmavo(&self, cmavo: Cmavo) -> bool {
        self.cmavo() == Some(cmavo)
    }

    #[requires(!cmavo.is_empty())]
    #[ensures(ret == self.cmavo().is_some_and(|actual| cmavo.contains(&actual)))]
    pub fn is_one_of_cmavo(&self, cmavo: &[Cmavo]) -> bool {
        self.cmavo().is_some_and(|actual| cmavo.contains(&actual))
    }

    #[requires(true)]
    #[ensures(ret == self.cmavo().is_some_and(|cmavo| selmaho.contains(cmavo)))]
    pub fn is_selmaho(&self, selmaho: Selmaho) -> bool {
        self.cmavo().is_some_and(|cmavo| selmaho.contains(cmavo))
    }

    #[requires(!selmaho.is_empty())]
    #[ensures(ret == self.cmavo().is_some_and(|cmavo| selmaho.iter().any(|selmaho| selmaho.contains(cmavo))))]
    pub fn is_one_of_selmaho(&self, selmaho: &[Selmaho]) -> bool {
        self.cmavo()
            .is_some_and(|cmavo| selmaho.iter().any(|selmaho| selmaho.contains(cmavo)))
    }

    #[requires(true)]
    #[ensures(ret == (self.quote_marker_cmavo() == Some(cmavo)))]
    pub fn is_quote_marker_cmavo(&self, cmavo: Cmavo) -> bool {
        self.quote_marker_cmavo() == Some(cmavo)
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn cmavo(&self) -> Option<Cmavo> {
        self.as_indicators().cmavo()
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn source_spans(&self) -> Vec<&jbotci_source::SourceSpan> {
        self.as_indicators().source_spans()
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn source_spans_into<'a>(&'a self, out: &mut Vec<&'a jbotci_source::SourceSpan>) {
        self.as_indicators().source_spans_into(out);
    }
}

impl fmt::Debug for Token {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_indicators().fmt(formatter)
    }
}

impl fmt::Display for Token {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_indicators().fmt(formatter)
    }
}

impl AsRef<WithIndicators<WordLike>> for Token {
    #[requires(true)]
    #[ensures(true)]
    fn as_ref(&self) -> &WithIndicators<WordLike> {
        self.as_indicators()
    }
}

impl WithIndicators<WordLike> {
    #[requires(true)]
    #[ensures(true)]
    pub fn core_word(&self) -> &WordLike {
        match self {
            WithIndicators::Bare(word_like) | WithIndicators::Emphasized { word_like, .. } => {
                word_like
            }
            WithIndicators::WithIndicator { base, .. } => base.core_word(),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn quote_marker_cmavo(&self) -> Option<Cmavo> {
        self.core_word().quote_marker_cmavo()
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn cmavo(&self) -> Option<Cmavo> {
        self.core_word().cmavo()
    }

    #[requires(true)]
    #[ensures(ret == (self.cmavo() == Some(cmavo)))]
    pub fn is_cmavo(&self, cmavo: Cmavo) -> bool {
        self.cmavo() == Some(cmavo)
    }

    #[requires(!cmavo.is_empty())]
    #[ensures(ret == self.cmavo().is_some_and(|actual| cmavo.contains(&actual)))]
    pub fn is_one_of_cmavo(&self, cmavo: &[Cmavo]) -> bool {
        self.cmavo().is_some_and(|actual| cmavo.contains(&actual))
    }

    #[requires(true)]
    #[ensures(ret == self.cmavo().is_some_and(|cmavo| selmaho.contains(cmavo)))]
    pub fn is_selmaho(&self, selmaho: Selmaho) -> bool {
        self.cmavo().is_some_and(|cmavo| selmaho.contains(cmavo))
    }

    #[requires(!selmaho.is_empty())]
    #[ensures(ret == self.cmavo().is_some_and(|cmavo| selmaho.iter().any(|selmaho| selmaho.contains(cmavo))))]
    pub fn is_one_of_selmaho(&self, selmaho: &[Selmaho]) -> bool {
        self.cmavo()
            .is_some_and(|cmavo| selmaho.iter().any(|selmaho| selmaho.contains(cmavo)))
    }

    #[requires(true)]
    #[ensures(ret == (self.quote_marker_cmavo() == Some(cmavo)))]
    pub fn is_quote_marker_cmavo(&self, cmavo: Cmavo) -> bool {
        self.quote_marker_cmavo() == Some(cmavo)
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
                out.push(bahe.span());
                word_like.source_spans_into(out);
            }
            WithIndicators::WithIndicator {
                base,
                indicator,
                nai,
            } => {
                base.source_spans_into(out);
                out.push(indicator.span());
                if let Some(nai) = nai {
                    out.push(nai.span());
                }
            }
        }
    }
}

impl WithFreeModifiers<Token> {
    #[requires(true)]
    #[ensures(true)]
    pub fn core_word(&self) -> &WordLike {
        self.value.core_word()
    }

    #[requires(true)]
    #[ensures(ret == self.value.quote_marker_cmavo())]
    pub fn quote_marker_cmavo(&self) -> Option<Cmavo> {
        self.value.quote_marker_cmavo()
    }

    #[requires(true)]
    #[ensures(ret == self.value.cmavo())]
    pub fn cmavo(&self) -> Option<Cmavo> {
        self.value.cmavo()
    }

    #[requires(true)]
    #[ensures(ret == self.value.is_cmavo(cmavo))]
    pub fn is_cmavo(&self, cmavo: Cmavo) -> bool {
        self.value.is_cmavo(cmavo)
    }

    #[requires(!cmavo.is_empty())]
    #[ensures(ret == self.value.is_one_of_cmavo(cmavo))]
    pub fn is_one_of_cmavo(&self, cmavo: &[Cmavo]) -> bool {
        self.value.is_one_of_cmavo(cmavo)
    }

    #[requires(true)]
    #[ensures(ret == self.value.is_selmaho(selmaho))]
    pub fn is_selmaho(&self, selmaho: Selmaho) -> bool {
        self.value.is_selmaho(selmaho)
    }

    #[requires(!selmaho.is_empty())]
    #[ensures(ret == self.value.is_one_of_selmaho(selmaho))]
    pub fn is_one_of_selmaho(&self, selmaho: &[Selmaho]) -> bool {
        self.value.is_one_of_selmaho(selmaho)
    }

    #[requires(true)]
    #[ensures(ret == self.value.is_quote_marker_cmavo(cmavo))]
    pub fn is_quote_marker_cmavo(&self, cmavo: Cmavo) -> bool {
        self.value.is_quote_marker_cmavo(cmavo)
    }
}

#[contract_trait]
pub(crate) trait OptionalSyntaxCmavoExt {
    #[requires(true)]
    #[ensures(true)]
    fn is_absent_or_cmavo(&self, cmavo: Cmavo) -> bool;

    #[requires(!cmavo.is_empty())]
    #[ensures(true)]
    fn is_absent_or_one_of_cmavo(&self, cmavo: &[Cmavo]) -> bool;

    #[requires(true)]
    #[ensures(true)]
    fn is_absent_or_selmaho(&self, selmaho: Selmaho) -> bool;

    #[requires(!selmaho.is_empty())]
    #[ensures(true)]
    fn is_absent_or_one_of_selmaho(&self, selmaho: &[Selmaho]) -> bool;
}

#[contract_trait]
impl OptionalSyntaxCmavoExt for Option<Token> {
    #[requires(true)]
    #[ensures(true)]
    fn is_absent_or_cmavo(&self, cmavo: Cmavo) -> bool {
        self.as_ref().is_none_or(|word| word.is_cmavo(cmavo))
    }

    #[requires(!cmavo.is_empty())]
    #[ensures(true)]
    fn is_absent_or_one_of_cmavo(&self, cmavo: &[Cmavo]) -> bool {
        self.as_ref().is_none_or(|word| word.is_one_of_cmavo(cmavo))
    }

    #[requires(true)]
    #[ensures(true)]
    fn is_absent_or_selmaho(&self, selmaho: Selmaho) -> bool {
        self.as_ref().is_none_or(|word| word.is_selmaho(selmaho))
    }

    #[requires(!selmaho.is_empty())]
    #[ensures(true)]
    fn is_absent_or_one_of_selmaho(&self, selmaho: &[Selmaho]) -> bool {
        self.as_ref()
            .is_none_or(|word| word.is_one_of_selmaho(selmaho))
    }
}

#[contract_trait]
impl OptionalSyntaxCmavoExt for Option<WithFreeModifiers<Token>> {
    #[requires(true)]
    #[ensures(true)]
    fn is_absent_or_cmavo(&self, cmavo: Cmavo) -> bool {
        self.as_ref().is_none_or(|word| word.is_cmavo(cmavo))
    }

    #[requires(!cmavo.is_empty())]
    #[ensures(true)]
    fn is_absent_or_one_of_cmavo(&self, cmavo: &[Cmavo]) -> bool {
        self.as_ref().is_none_or(|word| word.is_one_of_cmavo(cmavo))
    }

    #[requires(true)]
    #[ensures(true)]
    fn is_absent_or_selmaho(&self, selmaho: Selmaho) -> bool {
        self.as_ref().is_none_or(|word| word.is_selmaho(selmaho))
    }

    #[requires(!selmaho.is_empty())]
    #[ensures(true)]
    fn is_absent_or_one_of_selmaho(&self, selmaho: &[Selmaho]) -> bool {
        self.as_ref()
            .is_none_or(|word| word.is_one_of_selmaho(selmaho))
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
pub fn elidable_terminator_for_absent_field(_node: NodeRef<'_>, field: FieldRef) -> Option<Cmavo> {
    match field.name {
        Some("beho") => Some(Cmavo::Beho),
        Some("boi") => Some(Cmavo::Boi),
        Some("dohu") => Some(Cmavo::Dohu),
        Some("fehu") => Some(Cmavo::Fehu),
        Some("fihau") => Some(Cmavo::Fihau),
        Some("gehu") => Some(Cmavo::Gehu),
        Some("gihi") => Some(Cmavo::Gihi),
        Some("gik_nuhu") | Some("nuhu") => Some(Cmavo::Nuhu),
        Some("kehe") => Some(Cmavo::Kehe),
        Some("kei") => Some(Cmavo::Kei),
        Some("ku") | Some("maybe_ku") => Some(Cmavo::Ku),
        Some("kuhau") => Some(Cmavo::Kuhau),
        Some("kuhe") => Some(Cmavo::Kuhe),
        Some("kuho") => Some(Cmavo::Kuho),
        Some("kuhoi") => Some(Cmavo::Kuhoi),
        Some("liau") => Some(Cmavo::Lihau),
        Some("lihu") => Some(Cmavo::Lihu),
        Some("loho") => Some(Cmavo::Loho),
        Some("luhu") => Some(Cmavo::Luhu),
        Some("mehu") => Some(Cmavo::Mehu),
        Some("sehu") => Some(Cmavo::Sehu),
        Some("tehu") => Some(Cmavo::Tehu),
        Some("toi") => Some(Cmavo::Toi),
        Some("tuhu") => Some(Cmavo::Tuhu),
        Some("vau") => Some(Cmavo::Vau),
        Some("veho") => Some(Cmavo::Veho),
        _ => None,
    }
}

jbotci_tree::tree_model! {
pub type WordRun = Vec1<Token>;
pub type MathExpressionVec = Vec1<MathExpressionSyntax>;

#[invariant(indicator.core_word().bare_word().is_some_and(crate::is_indicator_word))]
#[invariant(nai.as_ref().is_none_or(|nai| nai.is_cmavo(Cmavo::Nai)))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Indicator {
    pub indicator: Token,
    pub nai: Option<Word>,
}

#[invariant(cu.is_absent_or_cmavo(Cmavo::Cu))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PredicateSyntax {
    pub leading_terms: Vec<TermSyntax>,
    pub cu: Option<WithFreeModifiers<Token>>,
    #[tree_child(primary)]
    pub predicate_tail: Box<PredicateTailSyntax>,
    pub free_modifiers: Vec<FreeModifierSyntax>,
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PredicateTailSyntax {
    #[tree_child(primary)]
    pub first: Box<PredicateTail1Syntax>,
    pub ke_continuation: Option<Box<KePredicateTailSyntax>>,
}

#[invariant(ke.is_cmavo(Cmavo::Ke))]
#[invariant(kehe.is_absent_or_cmavo(Cmavo::Kehe))]
#[invariant(vau.is_absent_or_cmavo(Cmavo::Vau))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct KePredicateTailSyntax {
    pub connective: ConnectiveSyntax,
    pub tense_modal: Option<Box<TenseModalSyntax>>,
    pub ke: WithFreeModifiers<Token>,
    #[tree_child(primary)]
    pub predicate_tail: Box<PredicateTailSyntax>,
    pub kehe: Option<WithFreeModifiers<Token>>,
    pub tail_terms: Vec<TermSyntax>,
    pub vau: Option<WithFreeModifiers<Token>>,
    pub free_modifiers: Vec<FreeModifierSyntax>,
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PredicateTail1Syntax {
    #[tree_child(primary)]
    pub first: Box<PredicateTail2Syntax>,
    pub continuations: Vec<PredicateTailContinuationSyntax>,
}

#[invariant(cu.is_absent_or_cmavo(Cmavo::Cu))]
#[invariant(vau.is_absent_or_cmavo(Cmavo::Vau))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PredicateTailContinuationSyntax {
    pub connective: ConnectiveSyntax,
    pub tense_modal: Option<Box<TenseModalSyntax>>,
    pub cu: Option<WithFreeModifiers<Token>>,
    #[tree_child(primary)]
    pub predicate_tail: Box<PredicateTail2Syntax>,
    pub tail_terms: Vec<TermSyntax>,
    pub vau: Option<WithFreeModifiers<Token>>,
    pub free_modifiers: Vec<FreeModifierSyntax>,
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PredicateTail2Syntax {
    #[tree_child(primary)]
    pub first: Box<PredicateTail3Syntax>,
    pub bo_continuation: Option<Box<BoPredicateTailSyntax>>,
}

#[invariant(bo.is_cmavo(Cmavo::Bo))]
#[invariant(cu.is_absent_or_cmavo(Cmavo::Cu))]
#[invariant(vau.is_absent_or_cmavo(Cmavo::Vau))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BoPredicateTailSyntax {
    pub connective: ConnectiveSyntax,
    pub tense_modal: Option<Box<TenseModalSyntax>>,
    pub bo: WithFreeModifiers<Token>,
    pub cu: Option<WithFreeModifiers<Token>>,
    #[tree_child(primary)]
    pub predicate_tail: Box<PredicateTail2Syntax>,
    pub tail_terms: Vec<TermSyntax>,
    pub vau: Option<WithFreeModifiers<Token>>,
    pub free_modifiers: Vec<FreeModifierSyntax>,
}

#[invariant(true)]
#[invariant(::Relation => vau.is_absent_or_cmavo(Cmavo::Vau))]
#[invariant(::GekSentence(..) => true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum PredicateTail3Syntax {
    Relation {
        #[tree_child(primary)]
        relation: Box<RelationSyntax>,
        terms: Vec<TermSyntax>,
        vau: Option<WithFreeModifiers<Token>>,
        free_modifiers: Vec<FreeModifierSyntax>,
    },
    GekSentence(Box<GekSentenceSyntax>),
}

#[invariant(true)]
#[invariant(::Pair => gihi.is_absent_or_selmaho(Selmaho::Gihi) && vau.is_absent_or_cmavo(Cmavo::Vau))]
#[invariant(::Ke => ke.is_cmavo(Cmavo::Ke) && kehe.is_absent_or_cmavo(Cmavo::Kehe))]
#[invariant(::Na => na.is_selmaho(Selmaho::Na))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum GekSentenceSyntax {
    Pair {
        gek: ConnectiveSyntax,
        first: Box<SubsentenceSyntax>,
        gik: ConnectiveSyntax,
        second: Box<SubsentenceSyntax>,
        gihi: Option<Token>,
        tail_terms: Vec<TermSyntax>,
        vau: Option<WithFreeModifiers<Token>>,
        free_modifiers: Vec<FreeModifierSyntax>,
    },
    Ke {
        tense_modal: Option<Box<TenseModalSyntax>>,
        ke: WithFreeModifiers<Token>,
        #[tree_child(primary)]
        inner: Box<GekSentenceSyntax>,
        kehe: Option<WithFreeModifiers<Token>>,
    },
    Na {
        na: WithFreeModifiers<Token>,
        #[tree_child(primary)]
        inner: Box<GekSentenceSyntax>,
    },
}

#[invariant(true)]
#[invariant(::Plain(..) => true)]
#[invariant(::Prenex => zohu.is_cmavo(Cmavo::Zohu))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum SubsentenceSyntax {
    Plain(Box<PredicateSyntax>),
    Prenex {
        prenex_terms: Vec<TermSyntax>,
        zohu: WithFreeModifiers<Token>,
        #[tree_child(primary)]
        inner_subsentence: Box<SubsentenceSyntax>,
    },
}

#[invariant(leading_nai.iter().all(|nai| nai.is_cmavo(Cmavo::Nai)))]
#[invariant(leading_cmevla.iter().all(crate::grammar::tokens::is_cmevla_word))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TextSyntax {
    pub leading_nai: Vec<Token>,
    pub leading_cmevla: Vec<Token>,
    pub leading_indicators: Vec<Indicator>,
    pub leading_free_modifiers: Vec<FreeModifierSyntax>,
    pub leading_connective: Option<Box<ConnectiveSyntax>>,
    #[tree_child(primary)]
    pub paragraphs: Vec<ParagraphSyntax>,
}

#[invariant(i.is_absent_or_cmavo(Cmavo::I))]
#[invariant(niho.iter().all(|niho| niho.is_selmaho(Selmaho::Niho)))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ParagraphSyntax {
    pub i: Option<Token>,
    pub niho: Vec<Token>,
    pub free_modifiers: Vec<FreeModifierSyntax>,
    #[tree_child(primary)]
    pub statements: Vec<ParagraphStatementSyntax>,
}

#[invariant(i.is_absent_or_cmavo(Cmavo::I))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ParagraphStatementSyntax {
    pub i: Option<Token>,
    pub connective: Option<Box<ConnectiveSyntax>>,
    pub free_modifiers: Vec<FreeModifierSyntax>,
    #[tree_child(primary)]
    pub statement: Option<Box<StatementSyntax>>,
}

#[invariant(true)]
#[invariant(::Sei => sei.is_selmaho(Selmaho::Sei) && cu.is_absent_or_cmavo(Cmavo::Cu) && sehu.is_absent_or_cmavo(Cmavo::Sehu))]
#[invariant(::To => to.is_selmaho(Selmaho::To) && toi.is_absent_or_cmavo(Cmavo::Toi))]
#[invariant(::Xi => xi.is_selmaho(Selmaho::Xi))]
#[invariant(::Mai => is_word_run_number_or_letter(number) && mai.is_selmaho(Selmaho::Mai))]
#[invariant(::Soi => soi.is_selmaho(Selmaho::Soi) && sehu.is_absent_or_cmavo(Cmavo::Sehu))]
#[invariant(::Vocative => is_valid_vocative_marker_words(&vocative_markers.value) && dohu.is_absent_or_cmavo(Cmavo::Dohu))]
#[invariant(::Replacement => lohai.is_absent_or_cmavo(Cmavo::Lohai) && sahai.is_absent_or_cmavo(Cmavo::Sahai) && lehai.is_cmavo(Cmavo::Lehai))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum FreeModifierSyntax {
    Sei {
        sei: WithFreeModifiers<Token>,
        terms: Vec<TermSyntax>,
        cu: Option<WithFreeModifiers<Token>>,
        relation: Box<RelationSyntax>,
        sehu: Option<WithFreeModifiers<Token>>,
    },
    To {
        to: WithFreeModifiers<Token>,
        #[tree_child(primary)]
        text: Box<TextSyntax>,
        toi: Option<WithFreeModifiers<Token>>,
    },
    Xi {
        xi: WithFreeModifiers<Token>,
        #[tree_child(primary)]
        expression: Box<MathExpressionSyntax>,
    },
    Mai {
        number: WordRun,
        mai: WithFreeModifiers<Token>,
    },
    Soi {
        soi: WithFreeModifiers<Token>,
        leading_argument: Box<ArgumentSyntax>,
        trailing_argument: Option<Box<ArgumentSyntax>>,
        sehu: Option<WithFreeModifiers<Token>>,
    },
    Vocative {
        vocative_markers: WithFreeModifiers<Vec<Token>>,
        argument: Option<Box<ArgumentSyntax>>,
        dohu: Option<WithFreeModifiers<Token>>,
    },
    Replacement {
        lohai: Option<Token>,
        old_words: Vec<Token>,
        sahai: Option<Token>,
        new_words: Vec<Token>,
        lehai: WithFreeModifiers<Token>,
    },
}

#[invariant(true)]
#[invariant(::Tuhe => tuhe.is_cmavo(Cmavo::Tuhe) && tuhu.is_absent_or_cmavo(Cmavo::Tuhu))]
#[invariant(::Prenex => zohu.is_cmavo(Cmavo::Zohu))]
#[invariant(::Predicate(..) => true)]
#[invariant(::Connected => i.is_cmavo(Cmavo::I))]
#[invariant(::PreIConnected => i.is_cmavo(Cmavo::I))]
#[invariant(::Iau => iau.is_cmavo(Cmavo::Ihau))]
#[invariant(::ExperimentalPredicateContinuation => true)]
#[invariant(::Fragment(..) => true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum StatementSyntax {
    Tuhe {
        tense_modal: Option<Box<TenseModalSyntax>>,
        tuhe: WithFreeModifiers<Token>,
        #[tree_child(primary)]
        text: Box<TextSyntax>,
        tuhu: Option<WithFreeModifiers<Token>>,
    },
    Prenex {
        prenex_terms: Vec<TermSyntax>,
        zohu: WithFreeModifiers<Token>,
        #[tree_child(primary)]
        inner_statement: Box<StatementSyntax>,
    },
    Predicate(Box<PredicateSyntax>),
    Connected {
        leading_statement: Box<StatementSyntax>,
        i: Token,
        connective: ConnectiveSyntax,
        trailing_statement: Box<StatementSyntax>,
    },
    PreIConnected {
        leading_statement: Box<StatementSyntax>,
        connective: ConnectiveSyntax,
        i: Token,
        trailing_statement: Box<StatementSyntax>,
    },
    Iau {
        #[tree_child(primary)]
        inner_statement: Box<StatementSyntax>,
        iau: WithFreeModifiers<Token>,
        reset_terms: Vec<TermSyntax>,
    },
    ExperimentalPredicateContinuation {
        leading_statement: Box<StatementSyntax>,
        continuation: PredicateStatementContinuationSyntax,
    },
    Fragment(Box<FragmentSyntax>),
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PredicateStatementContinuationSyntax {
    pub connective: ConnectiveSyntax,
    pub tense_modal: Option<Box<TenseModalSyntax>>,
    pub marker: PredicateStatementContinuationMarkerSyntax,
    pub trailing_subsentence: Box<SubsentenceSyntax>,
}

#[invariant(true)]
#[invariant(::Bo(bo) => bo.is_cmavo(Cmavo::Bo))]
#[invariant(::Ke => ke.is_cmavo(Cmavo::Ke) && kehe.is_absent_or_cmavo(Cmavo::Kehe))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum PredicateStatementContinuationMarkerSyntax {
    Bo(WithFreeModifiers<Token>),
    Ke {
        ke: WithFreeModifiers<Token>,
        kehe: Option<WithFreeModifiers<Token>>,
    },
}

#[invariant(true)]
#[invariant(::Ek(..) => true)]
#[invariant(::Gihek(..) => true)]
#[invariant(::Other(words) => !words.value.is_empty())]
#[invariant(::Ijek => i.is_cmavo(Cmavo::I))]
#[invariant(::Prenex => zohu.is_cmavo(Cmavo::Zohu))]
#[invariant(::BeLink => be.is_cmavo(Cmavo::Be) && fa.is_absent_or_selmaho(Selmaho::Fa) && (fa.is_none() || first_argument.is_some()) && beho.is_absent_or_cmavo(Cmavo::Beho))]
#[invariant(::BeiLink(bei_only_links) => !bei_only_links.is_empty())]
#[invariant(::RelativeClause(relative_clauses) => !relative_clauses.is_empty())]
#[invariant(::MathExpression(..) => true)]
#[invariant(::Term => vau.is_absent_or_cmavo(Cmavo::Vau))]
#[invariant(::Relation(..) => true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum FragmentSyntax {
    Ek(ConnectiveSyntax),
    Gihek(ConnectiveSyntax),
    Other(WithFreeModifiers<Vec<Token>>),
    Ijek {
        i: Token,
        connective: ConnectiveSyntax,
    },
    Prenex {
        terms: Vec<TermSyntax>,
        zohu: WithFreeModifiers<Token>,
    },
    BeLink {
        be: WithFreeModifiers<Token>,
        fa: Option<WithFreeModifiers<Token>>,
        first_argument: Option<Box<ArgumentSyntax>>,
        bei_links: Vec<BeiLinkSyntax>,
        beho: Option<WithFreeModifiers<Token>>,
    },
    BeiLink(Vec<BeiLinkSyntax>),
    RelativeClause(Vec<RelativeClauseSyntax>),
    MathExpression(Box<MathExpressionSyntax>),
    Term {
        terms: Vec<TermSyntax>,
        vau: Option<WithFreeModifiers<Token>>,
    },
    Relation(Box<RelationSyntax>),
}

#[invariant(true)]
#[invariant(::NuhiTermset => nuhi.is_cmavo(Cmavo::Nuhi) && !termset.is_empty() && nuhu.is_absent_or_cmavo(Cmavo::Nuhu))]
#[invariant(::GekNuhiTermset => m_nuhi.as_ref().is_none_or(|nuhi| nuhi.is_cmavo(Cmavo::Nuhi)) && !terms.is_empty() && nuhu.is_absent_or_cmavo(Cmavo::Nuhu) && !gik_terms.is_empty() && gihi.is_absent_or_selmaho(Selmaho::Gihi) && gik_nuhu.is_absent_or_cmavo(Cmavo::Nuhu))]
#[invariant(::Cehe => !leading_terms.is_empty() && cehe.is_cmavo(Cmavo::Cehe) && !trailing_terms.is_empty())]
#[invariant(::Pehe => !leading_terms.is_empty() && pehe.is_cmavo(Cmavo::Pehe) && !trailing_terms.is_empty())]
#[invariant(::Argument(..) => true)]
#[invariant(::Fa => fa.is_selmaho(Selmaho::Fa) && ku.is_absent_or_cmavo(Cmavo::Ku))]
#[invariant(::NaKu => na.is_selmaho(Selmaho::Na) && na_ku.is_cmavo(Cmavo::Ku))]
#[invariant(::BareNa(na) => na.is_selmaho(Selmaho::Na))]
#[invariant(::NoihaAdverbial => noiha.is_selmaho(Selmaho::Noiha) && fehu.is_absent_or_cmavo(Cmavo::Fehu))]
#[invariant(::PoihaBrigahi => poiha.is_selmaho(Selmaho::Noiha) && brigahi_ku.is_cmavo(Cmavo::Ku))]
#[invariant(::FihoiAdverbial => fihoi.is_cmavo(Cmavo::Fihoi) && fihau.is_absent_or_cmavo(Cmavo::Fihau))]
#[invariant(::SoiAdverbial => soi.is_selmaho(Selmaho::Soi) && sehu.is_absent_or_cmavo(Cmavo::Sehu))]
#[invariant(::JaiTagged => jai.is_cmavo(Cmavo::Jai))]
#[invariant(::Tagged => tense_modal.is_some())]
#[invariant(::Connected => !leading_terms.is_empty() && !trailing_terms.is_empty())]
#[invariant(::BoConnected => !leading_terms.is_empty() && bo.is_cmavo(Cmavo::Bo))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum TermSyntax {
    NuhiTermset {
        nuhi: WithFreeModifiers<Token>,
        termset: Vec<TermSyntax>,
        nuhu: Option<WithFreeModifiers<Token>>,
    },
    GekNuhiTermset {
        m_nuhi: Option<WithFreeModifiers<Token>>,
        gek: ConnectiveSyntax,
        terms: Vec<TermSyntax>,
        nuhu: Option<WithFreeModifiers<Token>>,
        gik: ConnectiveSyntax,
        gik_terms: Vec<TermSyntax>,
        gihi: Option<Token>,
        gik_nuhu: Option<WithFreeModifiers<Token>>,
    },
    Cehe {
        leading_terms: Vec<TermSyntax>,
        cehe: WithFreeModifiers<Token>,
        trailing_terms: Vec<TermSyntax>,
    },
    Pehe {
        leading_terms: Vec<TermSyntax>,
        pehe: WithFreeModifiers<Token>,
        connective: ConnectiveSyntax,
        trailing_terms: Vec<TermSyntax>,
    },
    Argument(Box<ArgumentSyntax>),
    Fa {
        fa: WithFreeModifiers<Token>,
        #[tree_child(primary)]
        argument: Box<ArgumentSyntax>,
        ku: Option<WithFreeModifiers<Token>>,
    },
    NaKu {
        na: Token,
        na_ku: WithFreeModifiers<Token>,
    },
    BareNa(WithFreeModifiers<Token>),
    NoihaAdverbial {
        noiha: WithFreeModifiers<Token>,
        tail_elements: Vec<ArgumentTailElementSyntax>,
        relation: Option<Box<RelationSyntax>>,
        relative_clauses: Vec<RelativeClauseSyntax>,
        fehu: Option<WithFreeModifiers<Token>>,
    },
    PoihaBrigahi {
        poiha: WithFreeModifiers<Token>,
        tail_elements: Vec<ArgumentTailElementSyntax>,
        relation: Option<Box<RelationSyntax>>,
        relative_clauses: Vec<RelativeClauseSyntax>,
        brigahi_ku: WithFreeModifiers<Token>,
    },
    FihoiAdverbial {
        fihoi: WithFreeModifiers<Token>,
        #[tree_child(primary)]
        subsentence: Box<SubsentenceSyntax>,
        fihau: Option<WithFreeModifiers<Token>>,
    },
    SoiAdverbial {
        soi: WithFreeModifiers<Token>,
        #[tree_child(primary)]
        subsentence: Box<SubsentenceSyntax>,
        sehu: Option<WithFreeModifiers<Token>>,
    },
    JaiTagged {
        jai: WithFreeModifiers<Token>,
        tag: Option<Box<TenseModalSyntax>>,
        #[tree_child(primary)]
        argument: Box<ArgumentSyntax>,
    },
    Tagged {
        tense_modal: Option<Box<TenseModalSyntax>>,
        #[tree_child(primary)]
        argument: Box<ArgumentSyntax>,
    },
    Connected {
        leading_terms: Vec<TermSyntax>,
        connective: ConnectiveSyntax,
        trailing_terms: Vec<TermSyntax>,
    },
    BoConnected {
        leading_terms: Vec<TermSyntax>,
        bo_connective: Option<Box<ConnectiveSyntax>>,
        tense_modal: Option<Box<TenseModalSyntax>>,
        bo: WithFreeModifiers<Token>,
        trailing_term: Box<TermSyntax>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum TermWrapperKindSyntax {
    Lahe,
    NaheBo,
    Nahe,
}

#[invariant(true)]
#[invariant(::TenseModal(..) => true)]
#[invariant(::Fa(fa) => fa.is_selmaho(Selmaho::Fa))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum ArgumentTagSyntax {
    TenseModal(Box<TenseModalSyntax>),
    Fa(WithFreeModifiers<Token>),
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ArgumentConnectionSyntax {
    pub connective: ConnectiveSyntax,
    #[tree_child(primary)]
    pub argument: Box<ArgumentSyntax>,
}

#[invariant(true)]
#[invariant(::Quote(..) => true)]
#[invariant(::MathExpression => li.is_selmaho(Selmaho::Li) && loho.is_absent_or_cmavo(Cmavo::Loho))]
#[invariant(::Letter => is_word_run_number_or_letter(&letter.value) && boi.is_absent_or_cmavo(Cmavo::Boi))]
#[invariant(::Quantified => true)]
#[invariant(::RelativeClause => vuho.is_absent_or_cmavo(Cmavo::Vuho) && !relative_clauses.is_empty())]
#[invariant(::Vuho => vuho_marker.is_cmavo(Cmavo::Vuho) && (!relative_clauses.is_empty() || connected_argument.is_some()))]
#[invariant(::BridiDescription => lohoi.is_selmaho(Selmaho::Lohoi) && kuhau.is_absent_or_cmavo(Cmavo::Kuhau))]
#[invariant(::NaKu => na.is_selmaho(Selmaho::Na) && ku.is_cmavo(Cmavo::Ku))]
#[invariant(::Tagged => true)]
#[invariant(::NaheBo => nahe.is_selmaho(Selmaho::Nahe) && bo.is_cmavo(Cmavo::Bo) && luhu.is_absent_or_cmavo(Cmavo::Luhu))]
#[invariant(::Nahe => nahe.is_selmaho(Selmaho::Nahe) && luhu.is_absent_or_cmavo(Cmavo::Luhu))]
#[invariant(::TermWrapped => match term_wrapper_kind {
    TermWrapperKindSyntax::Lahe => wrapper.is_selmaho(Selmaho::Lahe) && wrapper_bo.is_none(),
    TermWrapperKindSyntax::NaheBo => wrapper.is_selmaho(Selmaho::Nahe)
        && wrapper_bo.as_ref().is_some_and(|bo| bo.is_cmavo(Cmavo::Bo)),
    TermWrapperKindSyntax::Nahe => wrapper.is_selmaho(Selmaho::Nahe) && wrapper_bo.is_none(),
} && luhu.is_absent_or_cmavo(Cmavo::Luhu))]
#[invariant(::Koha(koha) => crate::grammar::tokens::is_koha_argument(&koha.value))]
#[invariant(::Zohe => maybe_ku.is_absent_or_cmavo(Cmavo::Ku))]
#[invariant(::Lahe => lahe.is_selmaho(Selmaho::Lahe) && luhu.is_absent_or_cmavo(Cmavo::Luhu))]
#[invariant(::Connected => true)]
#[invariant(::Ke => ke.is_cmavo(Cmavo::Ke) && kehe.is_absent_or_cmavo(Cmavo::Kehe))]
#[invariant(::Bo => bo.is_cmavo(Cmavo::Bo))]
#[invariant(::Gek => gihi.is_absent_or_selmaho(Selmaho::Gihi))]
#[invariant(::Descriptor(descriptor) => descriptor.descriptor.as_ref().is_none_or(|marker| marker.is_one_of_selmaho(&[Selmaho::Le, Selmaho::La])) && descriptor.ku.is_absent_or_cmavo(Cmavo::Ku) && (descriptor.descriptor.is_some() || (!descriptor.tail_elements.is_empty() && descriptor.relation.is_some())))]
#[invariant(::ConnectedDescriptor(descriptor) => descriptor.leading_descriptor_head.descriptor.is_one_of_selmaho(&[Selmaho::Le, Selmaho::La]) && descriptor.trailing_descriptor_head.descriptor.is_one_of_selmaho(&[Selmaho::Le, Selmaho::La]) && descriptor.ku.is_absent_or_cmavo(Cmavo::Ku))]
#[invariant(::Name => la.is_selmaho(Selmaho::La) && is_word_run_cmevla(&names.value))]
#[invariant(::Cmevla(names) => is_word_run_cmevla(&names.value))]
#[invariant(::RelationVocative => true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum ArgumentSyntax {
    Quote(Box<QuoteSyntax>),
    MathExpression {
        li: WithFreeModifiers<Token>,
        #[tree_child(primary)]
        expression: Box<MathExpressionSyntax>,
        loho: Option<WithFreeModifiers<Token>>,
    },
    Letter {
        letter: WithFreeModifiers<WordRun>,
        boi: Option<WithFreeModifiers<Token>>,
    },
    Quantified {
        quantifier: QuantifierSyntax,
        #[tree_child(primary)]
        inner_argument: Box<ArgumentSyntax>,
    },
    RelativeClause {
        base_argument: Box<ArgumentSyntax>,
        vuho: Option<WithFreeModifiers<Token>>,
        relative_clauses: Vec<RelativeClauseSyntax>,
    },
    Vuho {
        base_argument: Box<ArgumentSyntax>,
        vuho_marker: WithFreeModifiers<Token>,
        relative_clauses: Vec<RelativeClauseSyntax>,
        connected_argument: Option<Box<ArgumentConnectionSyntax>>,
    },
    BridiDescription {
        lohoi: WithFreeModifiers<Token>,
        #[tree_child(primary)]
        subsentence: Box<SubsentenceSyntax>,
        kuhau: Option<WithFreeModifiers<Token>>,
    },
    NaKu {
        na: Token,
        ku: WithFreeModifiers<Token>,
    },
    Tagged {
        tag: ArgumentTagSyntax,
        #[tree_child(primary)]
        inner_argument: Box<ArgumentSyntax>,
    },
    NaheBo {
        nahe: Token,
        bo: WithFreeModifiers<Token>,
        #[tree_child(primary)]
        inner_argument: Box<ArgumentSyntax>,
        luhu: Option<WithFreeModifiers<Token>>,
    },
    Nahe {
        nahe: WithFreeModifiers<Token>,
        #[tree_child(primary)]
        inner_argument: Box<ArgumentSyntax>,
        luhu: Option<WithFreeModifiers<Token>>,
    },
    TermWrapped {
        term_wrapper_kind: TermWrapperKindSyntax,
        wrapper: WithFreeModifiers<Token>,
        wrapper_bo: Option<WithFreeModifiers<Token>>,
        #[tree_child(primary)]
        inner_term: Box<TermSyntax>,
        luhu: Option<WithFreeModifiers<Token>>,
    },
    Koha(WithFreeModifiers<Token>),
    Zohe {
        tag: Option<Box<ArgumentTagSyntax>>,
        maybe_ku: Option<WithFreeModifiers<Token>>,
        free_modifiers: Vec<FreeModifierSyntax>,
    },
    Lahe {
        lahe: WithFreeModifiers<Token>,
        relative_clauses: Vec<RelativeClauseSyntax>,
        #[tree_child(primary)]
        inner_argument: Box<ArgumentSyntax>,
        luhu: Option<WithFreeModifiers<Token>>,
    },
    Connected {
        leading_argument: Box<ArgumentSyntax>,
        connective: ConnectiveSyntax,
        trailing_argument: Box<ArgumentSyntax>,
    },
    Ke {
        ke: WithFreeModifiers<Token>,
        #[tree_child(primary)]
        inner_argument: Box<ArgumentSyntax>,
        kehe: Option<WithFreeModifiers<Token>>,
    },
    Bo {
        leading_argument: Box<ArgumentSyntax>,
        bo_connective: Option<Box<ConnectiveSyntax>>,
        bo_tense_modal: Option<Box<TenseModalSyntax>>,
        bo: WithFreeModifiers<Token>,
        trailing_argument: Box<ArgumentSyntax>,
    },
    Gek {
        gek: ConnectiveSyntax,
        leading_argument: Box<ArgumentSyntax>,
        gik: ConnectiveSyntax,
        trailing_argument: Box<ArgumentSyntax>,
        gihi: Option<Token>,
    },
    Descriptor(Box<DescriptorSyntax>),
    ConnectedDescriptor(Box<ConnectedDescriptorSyntax>),
    Name {
        la: WithFreeModifiers<Token>,
        names: WithFreeModifiers<WordRun>,
    },
    Cmevla(WithFreeModifiers<WordRun>),
    RelationVocative {
        leading_relative_clauses: Vec<RelativeClauseSyntax>,
        relation: Box<RelationSyntax>,
        trailing_relative_clauses: Vec<RelativeClauseSyntax>,
    },
}

#[invariant(true)]
#[invariant(::Goi(goi) => goi.goi.is_selmaho(Selmaho::Goi) && goi.gehu.is_absent_or_cmavo(Cmavo::Gehu))]
#[invariant(::Noi => noi.is_one_of_cmavo(NONRESTRICTIVE_RELATIVE_CLAUSE_CMAVO) && kuho.is_absent_or_cmavo(Cmavo::Kuho))]
#[invariant(::Poi => poi.is_one_of_cmavo(RESTRICTIVE_RELATIVE_CLAUSE_CMAVO) && kuho.is_absent_or_cmavo(Cmavo::Kuho))]
#[invariant(::Zihe => zihe.is_cmavo(Cmavo::Zihe))]
#[invariant(::Connected => true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum RelativeClauseSyntax {
    Goi(Box<GoiRelativeClauseSyntax>),
    Noi {
        noi: WithFreeModifiers<Token>,
        #[tree_child(primary)]
        subsentence: Box<SubsentenceSyntax>,
        kuho: Option<WithFreeModifiers<Token>>,
    },
    Poi {
        poi: WithFreeModifiers<Token>,
        #[tree_child(primary)]
        subsentence: Box<SubsentenceSyntax>,
        kuho: Option<WithFreeModifiers<Token>>,
    },
    Zihe {
        zihe: WithFreeModifiers<Token>,
        #[tree_child(primary)]
        inner: Box<RelativeClauseSyntax>,
    },
    Connected {
        connective: ConnectiveSyntax,
        #[tree_child(primary)]
        inner: Box<RelativeClauseSyntax>,
    },
}

#[invariant(goi.is_selmaho(Selmaho::Goi))]
#[invariant(gehu.is_absent_or_cmavo(Cmavo::Gehu))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GoiRelativeClauseSyntax {
    pub goi: WithFreeModifiers<Token>,
    #[tree_child(primary)]
    pub argument: Box<ArgumentSyntax>,
    pub gehu: Option<WithFreeModifiers<Token>>,
}

#[invariant(nohoi.is_cmavo(Cmavo::Nohoi))]
#[invariant(kuhoi.is_absent_or_cmavo(Cmavo::Kuhoi))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SelbriRelativeClauseSyntax {
    pub nohoi: WithFreeModifiers<Token>,
    #[tree_child(primary)]
    pub relation: Box<RelationSyntax>,
    pub kuhoi: Option<WithFreeModifiers<Token>>,
}

#[invariant(true)]
#[invariant(::Lu => lu.is_cmavo(Cmavo::Lu) && lihu.is_absent_or_cmavo(Cmavo::Lihu))]
#[invariant(::Zo(zo) => zo.is_quote_marker_cmavo(Cmavo::Zo))]
#[invariant(::ZohOi(zohoi) => zohoi.quote_marker_cmavo().is_some_and(|cmavo| [Cmavo::Zohoi, Cmavo::Lahoi, Cmavo::Rahoi, Cmavo::Mehoi, Cmavo::Gohoi].contains(&cmavo)))]
#[invariant(::Zoi(zoi) => zoi.quote_marker_cmavo().is_some_and(|cmavo| Selmaho::Zoi.contains(cmavo)))]
#[invariant(::Lohu(lohu) => lohu.is_quote_marker_cmavo(Cmavo::Lohu))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum QuoteSyntax {
    Lu {
        lu: WithFreeModifiers<Token>,
        #[tree_child(primary)]
        text: Box<TextSyntax>,
        lihu: Option<WithFreeModifiers<Token>>,
    },
    Zo(WithFreeModifiers<Token>),
    ZohOi(WithFreeModifiers<Token>),
    Zoi(WithFreeModifiers<Token>),
    Lohu(WithFreeModifiers<Token>),
}

#[invariant(descriptor.as_ref().is_none_or(|descriptor| descriptor.is_one_of_selmaho(&[Selmaho::Le, Selmaho::La])))]
#[invariant(ku.is_absent_or_cmavo(Cmavo::Ku))]
#[invariant(descriptor.is_some() || (!tail_elements.is_empty() && relation.is_some()))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DescriptorSyntax {
    pub outer_quantifier: Option<Box<QuantifierSyntax>>,
    pub descriptor: Option<WithFreeModifiers<Token>>,
    pub tail_elements: Vec<ArgumentTailElementSyntax>,
    pub relation: Option<Box<RelationSyntax>>,
    pub relative_clauses: Vec<RelativeClauseSyntax>,
    pub ku: Option<WithFreeModifiers<Token>>,
}

#[invariant(descriptor.is_one_of_selmaho(&[Selmaho::Le, Selmaho::La]))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DescriptorHeadSyntax {
    pub descriptor: WithFreeModifiers<Token>,
}

#[invariant(ku.is_absent_or_cmavo(Cmavo::Ku))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ConnectedDescriptorSyntax {
    pub leading_descriptor_head: Box<DescriptorHeadSyntax>,
    pub connective: ConnectiveSyntax,
    pub trailing_descriptor_head: Box<DescriptorHeadSyntax>,
    pub tail_elements: Vec<ArgumentTailElementSyntax>,
    pub relation: Option<Box<RelationSyntax>>,
    pub relative_clauses: Vec<RelativeClauseSyntax>,
    pub ku: Option<WithFreeModifiers<Token>>,
}

#[invariant(true)]
#[invariant(::Afterthought => is_valid_connective_parts(se, nahe, na, cmavo, nai))]
#[invariant(::Relation => is_valid_connective_parts(se, nahe, na, cmavo, nai))]
#[invariant(::PredicateTail => is_valid_connective_parts(se, nahe, na, cmavo, nai))]
#[invariant(::Forethought => is_valid_connective_parts(se, nahe, na, cmavo, nai))]
#[invariant(::NonLogical => is_valid_connective_parts(se, nahe, na, cmavo, nai))]
#[invariant(::Interval => is_valid_connective_parts(se, nahe, na, cmavo, nai))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum ConnectiveSyntax {
    Afterthought {
        se: Option<Token>,
        nahe: Option<Token>,
        na: Option<Token>,
        #[tree_child(primary)]
        cmavo: WithFreeModifiers<Vec<Token>>,
        nai: Option<WithFreeModifiers<Token>>,
    },
    Relation {
        se: Option<Token>,
        nahe: Option<Token>,
        na: Option<Token>,
        #[tree_child(primary)]
        cmavo: WithFreeModifiers<Vec<Token>>,
        nai: Option<WithFreeModifiers<Token>>,
    },
    PredicateTail {
        se: Option<Token>,
        nahe: Option<Token>,
        na: Option<Token>,
        #[tree_child(primary)]
        cmavo: WithFreeModifiers<Vec<Token>>,
        nai: Option<WithFreeModifiers<Token>>,
    },
    Forethought {
        se: Option<Token>,
        nahe: Option<Token>,
        na: Option<Token>,
        #[tree_child(primary)]
        cmavo: WithFreeModifiers<Vec<Token>>,
        nai: Option<WithFreeModifiers<Token>>,
    },
    NonLogical {
        se: Option<Token>,
        nahe: Option<Token>,
        na: Option<Token>,
        #[tree_child(primary)]
        cmavo: WithFreeModifiers<Vec<Token>>,
        nai: Option<WithFreeModifiers<Token>>,
    },
    Interval {
        se: Option<Token>,
        nahe: Option<Token>,
        na: Option<Token>,
        #[tree_child(primary)]
        cmavo: WithFreeModifiers<Vec<Token>>,
        nai: Option<WithFreeModifiers<Token>>,
    },
}

#[invariant(bei.is_cmavo(Cmavo::Bei))]
#[invariant(fa.is_none() || argument.is_some(), "lifted FA link tags must have an argument")]
#[invariant(fa.is_absent_or_selmaho(Selmaho::Fa))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BeiLinkSyntax {
    pub bei: WithFreeModifiers<Token>,
    pub fa: Option<WithFreeModifiers<Token>>,
    pub argument: Option<Box<ArgumentSyntax>>,
}

#[invariant(fa.is_none() || argument.is_some(), "lifted FA link tags must have an argument")]
#[invariant(fa.is_absent_or_selmaho(Selmaho::Fa))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LinkArgumentSyntax {
    pub fa: Option<WithFreeModifiers<Token>>,
    pub argument: Option<Box<ArgumentSyntax>>,
}

#[invariant(be.is_cmavo(Cmavo::Be))]
#[invariant(fa.is_none() || first_argument.is_some(), "lifted FA link tags must have an argument")]
#[invariant(fa.is_absent_or_selmaho(Selmaho::Fa))]
#[invariant(beho.is_absent_or_cmavo(Cmavo::Beho))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BeLinkSyntax {
    pub be: WithFreeModifiers<Token>,
    pub fa: Option<WithFreeModifiers<Token>>,
    pub first_argument: Option<Box<ArgumentSyntax>>,
    pub bei_links: Vec<BeiLinkSyntax>,
    pub beho: Option<WithFreeModifiers<Token>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum ConnectiveKind {
    Afterthought,
    Relation,
    PredicateTail,
    Forethought,
    NonLogical,
    Interval,
}

#[invariant(true)]
#[invariant(::Argument(..) => true)]
#[invariant(::RelativeClauses(relative_clauses) => !relative_clauses.is_empty())]
#[invariant(::Quantifier(..) => true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum ArgumentTailElementSyntax {
    Argument(Box<ArgumentSyntax>),
    RelativeClauses(Vec<RelativeClauseSyntax>),
    Quantifier(QuantifierSyntax),
}

#[invariant(true)]
#[invariant(::Number => is_word_run_number_or_letter(&number.value) && boi.is_absent_or_cmavo(Cmavo::Boi))]
#[invariant(::Vei => vei.is_cmavo(Cmavo::Vei) && veho.is_absent_or_cmavo(Cmavo::Veho))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum QuantifierSyntax {
    Number {
        #[tree_child(primary)]
        number: WithFreeModifiers<WordRun>,
        boi: Option<WithFreeModifiers<Token>>,
    },
    Vei {
        vei: WithFreeModifiers<Token>,
        #[tree_child(primary)]
        math_expression: Box<MathExpressionSyntax>,
        veho: Option<WithFreeModifiers<Token>>,
    },
}

#[invariant(true)]
#[invariant(::Number(..) => true)]
#[invariant(::Letter => is_word_run_number_or_letter(&letter.value) && boi.is_absent_or_cmavo(Cmavo::Boi))]
#[invariant(::Vei => vei.is_cmavo(Cmavo::Vei) && veho.is_absent_or_cmavo(Cmavo::Veho))]
#[invariant(::Gek => true)]
#[invariant(::Forethought => peho.as_ref().is_none_or(|peho| peho.is_cmavo(Cmavo::Peho)) && !operands.is_empty() && kuhe.is_absent_or_cmavo(Cmavo::Kuhe))]
#[invariant(::ReversePolish => fuha.is_cmavo(Cmavo::Fuha) && !operands.is_empty())]
#[invariant(::Nihe => nihe.is_cmavo(Cmavo::Nihe) && tehu.is_absent_or_cmavo(Cmavo::Tehu))]
#[invariant(::Mohe => mohe.is_cmavo(Cmavo::Mohe) && tehu.is_absent_or_cmavo(Cmavo::Tehu))]
#[invariant(::Johi => johi.is_cmavo(Cmavo::Johi) && tehu.is_absent_or_cmavo(Cmavo::Tehu))]
#[invariant(::Lahe => matches!(markers.value.as_slice(), [nahe, bo] if nahe.is_selmaho(Selmaho::Nahe) && bo.is_cmavo(Cmavo::Bo)) && luhu.is_absent_or_cmavo(Cmavo::Luhu))]
#[invariant(::Connected => true)]
#[invariant(::Binary => true)]
#[invariant(::Bihe => bihe.is_cmavo(Cmavo::Bihe))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum MathExpressionSyntax {
    Number(Box<QuantifierSyntax>),
    Letter {
        #[tree_child(primary)]
        letter: WithFreeModifiers<WordRun>,
        boi: Option<WithFreeModifiers<Token>>,
    },
    Vei {
        vei: WithFreeModifiers<Token>,
        #[tree_child(primary)]
        inner_expression: Box<MathExpressionSyntax>,
        veho: Option<WithFreeModifiers<Token>>,
    },
    Gek {
        gek: ConnectiveSyntax,
        left_expression: Box<MathExpressionSyntax>,
        gik: ConnectiveSyntax,
        right_expression: Box<MathExpressionSyntax>,
    },
    Forethought {
        peho: Option<WithFreeModifiers<Token>>,
        operator: Box<MathOperatorSyntax>,
        operands: Vec<MathExpressionSyntax>,
        kuhe: Option<WithFreeModifiers<Token>>,
    },
    ReversePolish {
        fuha: WithFreeModifiers<Token>,
        operands: Vec<MathExpressionSyntax>,
        operators: Vec<MathOperatorSyntax>,
    },
    Nihe {
        nihe: WithFreeModifiers<Token>,
        #[tree_child(primary)]
        relation: Box<RelationSyntax>,
        tehu: Option<WithFreeModifiers<Token>>,
    },
    Mohe {
        mohe: WithFreeModifiers<Token>,
        #[tree_child(primary)]
        argument: Box<ArgumentSyntax>,
        tehu: Option<WithFreeModifiers<Token>>,
    },
    Johi {
        johi: WithFreeModifiers<Token>,
        #[tree_child(primary)]
        expressions: MathExpressionVec,
        tehu: Option<WithFreeModifiers<Token>>,
    },
    Lahe {
        markers: WithFreeModifiers<Vec<Token>>,
        #[tree_child(primary)]
        inner_expression: Box<MathExpressionSyntax>,
        luhu: Option<WithFreeModifiers<Token>>,
    },
    Connected {
        left_expression: Box<MathExpressionSyntax>,
        connective: ConnectiveSyntax,
        right_expression: Box<MathExpressionSyntax>,
    },
    Binary {
        left_expression: Box<MathExpressionSyntax>,
        operator: Box<MathOperatorSyntax>,
        right_expression: Box<MathExpressionSyntax>,
    },
    Bihe {
        left_expression: Box<MathExpressionSyntax>,
        bihe: WithFreeModifiers<Token>,
        operator: Box<MathOperatorSyntax>,
        right_expression: Box<MathExpressionSyntax>,
    },
}

#[invariant(true)]
#[invariant(::Vuhu(vuhu) => vuhu.is_selmaho(Selmaho::Vuhu))]
#[invariant(::Maho => maho.is_cmavo(Cmavo::Maho) && tehu.is_absent_or_cmavo(Cmavo::Tehu))]
#[invariant(::Se => se.is_selmaho(Selmaho::Se))]
#[invariant(::Nahe => nahe.is_selmaho(Selmaho::Nahe))]
#[invariant(::Nahu => nahu.is_cmavo(Cmavo::Nahu) && tehu.is_absent_or_cmavo(Cmavo::Tehu))]
#[invariant(::Ke => ke.is_cmavo(Cmavo::Ke) && kehe.is_absent_or_cmavo(Cmavo::Kehe))]
#[invariant(::Bo => bo.is_cmavo(Cmavo::Bo))]
#[invariant(::Connected => true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum MathOperatorSyntax {
    Vuhu(WithFreeModifiers<Token>),
    Maho {
        maho: WithFreeModifiers<Token>,
        #[tree_child(primary)]
        math_expression: Box<MathExpressionSyntax>,
        tehu: Option<WithFreeModifiers<Token>>,
    },
    Se {
        se: WithFreeModifiers<Token>,
        #[tree_child(primary)]
        inner_operator: Box<MathOperatorSyntax>,
    },
    Nahe {
        nahe: WithFreeModifiers<Token>,
        #[tree_child(primary)]
        inner_operator: Box<MathOperatorSyntax>,
    },
    Nahu {
        nahu: WithFreeModifiers<Token>,
        #[tree_child(primary)]
        relation: Box<RelationSyntax>,
        tehu: Option<WithFreeModifiers<Token>>,
    },
    Ke {
        ke: WithFreeModifiers<Token>,
        #[tree_child(primary)]
        inner_operator: Box<MathOperatorSyntax>,
        kehe: Option<WithFreeModifiers<Token>>,
    },
    Bo {
        left_operator: Box<MathOperatorSyntax>,
        connective: ConnectiveSyntax,
        bo: WithFreeModifiers<Token>,
        right_operator: Box<MathOperatorSyntax>,
    },
    Connected {
        left_operator: Box<MathOperatorSyntax>,
        connective: ConnectiveSyntax,
        right_operator: Box<MathOperatorSyntax>,
    },
}

#[invariant(true)]
#[invariant(::Connected => true)]
#[invariant(::Co => co.is_cmavo(Cmavo::Co))]
#[invariant(::Bo => bo.is_cmavo(Cmavo::Bo))]
#[invariant(::Na => na.is_selmaho(Selmaho::Na))]
#[invariant(::Base(word) => crate::grammar::tokens::is_relation_word(word) || crate::grammar::tokens::is_cmevla_word(word))]
#[invariant(::Se => se.is_selmaho(Selmaho::Se))]
#[invariant(::Ke => ke.is_cmavo(Cmavo::Ke) && kehe.is_absent_or_cmavo(Cmavo::Kehe))]
#[invariant(::TenseModal => true)]
#[invariant(::Guha => gihi.is_absent_or_selmaho(Selmaho::Gihi))]
#[invariant(::Abstraction(abstraction) => abstraction.nu.is_selmaho(Selmaho::Nu) && abstraction.nai.is_absent_or_cmavo(Cmavo::Nai) && abstraction.additional_nu.iter().all(|additional_nu| additional_nu.nu.is_selmaho(Selmaho::Nu) && additional_nu.nai.is_absent_or_cmavo(Cmavo::Nai)) && abstraction.kei.is_absent_or_cmavo(Cmavo::Kei))]
#[invariant(::Compound(..) => true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum RelationSyntax {
    Connected {
        leading_relation: Box<RelationSyntax>,
        connective: ConnectiveSyntax,
        trailing_relation: Box<RelationSyntax>,
    },
    Co {
        leading_relation: Box<RelationSyntax>,
        co: WithFreeModifiers<Token>,
        trailing_relation: Box<RelationSyntax>,
    },
    Bo {
        leading_relation: Box<RelationSyntax>,
        bo_connective: Option<Box<ConnectiveSyntax>>,
        bo_tense_modal: Option<Box<TenseModalSyntax>>,
        bo: WithFreeModifiers<Token>,
        trailing_relation: Box<RelationSyntax>,
    },
    Na {
        na: WithFreeModifiers<Token>,
        #[tree_child(primary)]
        inner_relation: Box<RelationSyntax>,
    },
    Base(Token),
    Se {
        se: WithFreeModifiers<Token>,
        #[tree_child(primary)]
        inner_relation: Box<RelationSyntax>,
    },
    Ke {
        ke_tense_modal: Option<Box<TenseModalSyntax>>,
        ke: WithFreeModifiers<Token>,
        #[tree_child(primary)]
        relation: Box<RelationSyntax>,
        kehe: Option<WithFreeModifiers<Token>>,
    },
    TenseModal {
        tense_modal: Box<TenseModalSyntax>,
        #[tree_child(primary)]
        inner_relation: Box<RelationSyntax>,
    },
    Guha {
        guhek: ConnectiveSyntax,
        leading_predicate: Box<PredicateSyntax>,
        gik: ConnectiveSyntax,
        trailing_predicate: Box<PredicateSyntax>,
        gihi: Option<Token>,
    },
    Abstraction(Box<AbstractionSyntax>),
    Compound(Box<RelationUnitVec>),
}

pub type RelationUnitVec = Vec1<RelationUnitSyntax>;

#[invariant(direction.iter().all(|direction| direction.is_selmaho(Selmaho::Pu)))]
#[invariant(distance.as_ref().is_none_or(|distance| distance.is_selmaho(Selmaho::Zi)))]
#[invariant(interval.as_ref().is_none_or(|interval| interval.is_selmaho(Selmaho::Zeha)))]
#[invariant(nai.is_absent_or_cmavo(Cmavo::Nai))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TimeTenseSyntax {
    pub direction: Vec<Token>,
    pub distance: Option<Token>,
    pub interval: Option<Token>,
    pub nai: Option<Token>,
}

#[invariant(direction.iter().all(|direction| direction.is_selmaho(Selmaho::Faha)))]
#[invariant(distance.iter().all(|distance| distance.is_selmaho(Selmaho::Va)))]
#[invariant(interval.iter().all(|interval| interval.is_selmaho(Selmaho::Veha)))]
#[invariant(dimensions.iter().all(|dimension| dimension.is_selmaho(Selmaho::Viha)))]
#[invariant(mohi.is_absent_or_cmavo(Cmavo::Mohi))]
#[invariant(fehe.is_absent_or_cmavo(Cmavo::Fehe))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SpaceTenseSyntax {
    pub direction: Vec<Token>,
    pub distance: Vec<Token>,
    pub interval: Vec<Token>,
    pub dimensions: Vec<Token>,
    pub mohi: Option<Token>,
    pub fehe: Option<Token>,
}

#[invariant(number.as_ref().is_none_or(is_word_run_number_or_letter))]
#[invariant(roi_or_tahe.is_selmaho(Selmaho::Roi)
    || roi_or_tahe.is_selmaho(Selmaho::Tahe))]
#[invariant(nai.is_absent_or_cmavo(Cmavo::Nai))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IntervalTenseSyntax {
    pub number: Option<WordRun>,
    pub roi_or_tahe: Token,
    pub nai: Option<Token>,
}

#[invariant(nahe.as_ref().is_none_or(|nahe| nahe.is_selmaho(Selmaho::Nahe)))]
#[invariant(se.as_ref().is_none_or(|se| se.is_selmaho(Selmaho::Se)))]
#[invariant(bai.as_ref().is_none_or(|bai| bai.is_selmaho(Selmaho::Bai)))]
#[invariant(nai.is_absent_or_cmavo(Cmavo::Nai))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SimpleTenseModalSyntax {
    pub nahe: Option<Token>,
    pub se: Option<Token>,
    pub bai: Option<Token>,
    pub nai: Option<Token>,
}

#[invariant(nahe.as_ref().is_none_or(|nahe| nahe.is_selmaho(Selmaho::Nahe)))]
#[invariant(fiho.is_cmavo(Cmavo::Fiho))]
#[invariant(fehu.is_absent_or_cmavo(Cmavo::Fehu))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FihoModalSyntax {
    pub nahe: Option<Token>,
    pub fiho: WithFreeModifiers<Token>,
    pub relation: Box<RelationSyntax>,
    pub fehu: Option<WithFreeModifiers<Token>>,
}

#[invariant(true)]
#[invariant(::Word(word) => is_valid_tense_modal_word(word) || word.is_one_of_selmaho(&[Selmaho::Na, Selmaho::Ja, Selmaho::Joi, Selmaho::Bihi, Selmaho::Gaho]))]
#[invariant(::Fiho(fiho) => fiho.nahe.is_absent_or_selmaho(Selmaho::Nahe) && fiho.fiho.is_cmavo(Cmavo::Fiho) && fiho.fehu.is_absent_or_cmavo(Cmavo::Fehu))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum CompositeTenseModalPartSyntax {
    Word(Token),
    Fiho(Box<FihoModalSyntax>),
}

#[invariant(true)]
#[invariant(::Composite => !parts.value.is_empty())]
#[invariant(::Pu(pu) => pu.is_selmaho(Selmaho::Pu))]
#[invariant(::PuDistance => pu.is_selmaho(Selmaho::Pu) && distance.is_selmaho(Selmaho::Zi))]
#[invariant(::TimeInterval(interval) => interval.is_selmaho(Selmaho::Zeha))]
#[invariant(::PuCaha => pu.is_selmaho(Selmaho::Pu) && caha.is_selmaho(Selmaho::Caha))]
#[invariant(::SpaceDistance(distance) => distance.is_selmaho(Selmaho::Va))]
#[invariant(::SpaceDirection(direction) => direction.is_selmaho(Selmaho::Faha))]
#[invariant(::SpaceMovement => mohi.is_cmavo(Cmavo::Mohi) && direction.is_selmaho(Selmaho::Faha) && distance.is_absent_or_selmaho(Selmaho::Va))]
#[invariant(::Simple => nahe.as_ref().is_none_or(|nahe| nahe.is_selmaho(Selmaho::Nahe)) && se.as_ref().is_none_or(|se| se.is_selmaho(Selmaho::Se)) && bai.is_selmaho(Selmaho::Bai) && nai.is_absent_or_cmavo(Cmavo::Nai) && ki.is_absent_or_cmavo(Cmavo::Ki))]
#[invariant(::Ki(ki) => ki.is_cmavo(Cmavo::Ki))]
#[invariant(::Fiho => fiho.is_cmavo(Cmavo::Fiho) && fehu.is_absent_or_cmavo(Cmavo::Fehu))]
#[invariant(::Caha(caha) => caha.is_selmaho(Selmaho::Caha))]
#[invariant(::Zaho(zaho) => zaho.value.iter().all(|word| word.is_selmaho(Selmaho::Zaho)))]
#[invariant(::Interval => number.as_ref().is_none_or(is_word_run_number_or_letter) && roi_or_tahe.is_one_of_selmaho(&[Selmaho::Roi, Selmaho::Tahe]) && nai.is_absent_or_cmavo(Cmavo::Nai))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum TenseModalSyntax {
    Composite {
        #[tree_child(primary)]
        parts: WithFreeModifiers<Vec<CompositeTenseModalPartSyntax>>,
    },
    Pu(WithFreeModifiers<Token>),
    PuDistance {
        pu: Token,
        distance: WithFreeModifiers<Token>,
    },
    TimeInterval(WithFreeModifiers<Token>),
    PuCaha {
        pu: Token,
        caha: WithFreeModifiers<Token>,
    },
    SpaceDistance(WithFreeModifiers<Token>),
    SpaceDirection(WithFreeModifiers<Token>),
    SpaceMovement {
        mohi: Token,
        direction: WithFreeModifiers<Token>,
        distance: Option<WithFreeModifiers<Token>>,
    },
    Simple {
        nahe: Option<WithFreeModifiers<Token>>,
        se: Option<WithFreeModifiers<Token>>,
        bai: WithFreeModifiers<Token>,
        nai: Option<WithFreeModifiers<Token>>,
        ki: Option<WithFreeModifiers<Token>>,
    },
    Ki(WithFreeModifiers<Token>),
    Fiho {
        fiho: WithFreeModifiers<Token>,
        #[tree_child(primary)]
        relation: Box<RelationSyntax>,
        fehu: Option<WithFreeModifiers<Token>>,
    },
    Caha(WithFreeModifiers<Token>),
    Zaho(WithFreeModifiers<Vec<Token>>),
    Interval {
        number: Option<WordRun>,
        roi_or_tahe: WithFreeModifiers<Token>,
        nai: Option<WithFreeModifiers<Token>>,
    },
}

#[invariant(nu.is_selmaho(Selmaho::Nu))]
#[invariant(nai.is_absent_or_cmavo(Cmavo::Nai))]
#[invariant(kei.is_absent_or_cmavo(Cmavo::Kei))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AbstractionSyntax {
    pub nu: WithFreeModifiers<Token>,
    pub nai: Option<WithFreeModifiers<Token>>,
    pub additional_nu: Vec<AdditionalNuSyntax>,
    #[tree_child(primary)]
    pub subsentence: Box<SubsentenceSyntax>,
    pub kei: Option<WithFreeModifiers<Token>>,
}

#[invariant(nu.is_selmaho(Selmaho::Nu))]
#[invariant(nai.is_absent_or_cmavo(Cmavo::Nai))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AdditionalNuSyntax {
    pub connective: ConnectiveSyntax,
    pub nu: WithFreeModifiers<Token>,
    pub nai: Option<WithFreeModifiers<Token>>,
}

#[invariant(true)]
#[invariant(::Word(word) => crate::grammar::tokens::is_relation_word(&word.value) || crate::grammar::tokens::is_cmevla_word(&word.value))]
#[invariant(::Goha => goha.is_selmaho(Selmaho::Goha) && raho.is_absent_or_cmavo(Cmavo::Raho))]
#[invariant(::Se => se.is_selmaho(Selmaho::Se))]
#[invariant(::Ke => ke.is_cmavo(Cmavo::Ke) && kehe.is_absent_or_cmavo(Cmavo::Kehe))]
#[invariant(::Nahe => nahe.is_selmaho(Selmaho::Nahe))]
#[invariant(::Bo => bo.is_cmavo(Cmavo::Bo))]
#[invariant(::Connected => true)]
#[invariant(::SelbriRelativeClause => !selbri_relative_clauses.is_empty())]
#[invariant(::Wrapped(..) => true)]
#[invariant(::Jai => jai.is_cmavo(Cmavo::Jai))]
#[invariant(::Be => be.is_cmavo(Cmavo::Be) && fa.is_absent_or_selmaho(Selmaho::Fa) && (fa.is_none() || first_argument.is_some()) && beho.is_absent_or_cmavo(Cmavo::Beho))]
#[invariant(::PreposedBe => be.is_cmavo(Cmavo::Be) && fa.is_absent_or_selmaho(Selmaho::Fa) && (fa.is_none() || first_argument.is_some()) && beho.is_absent_or_cmavo(Cmavo::Beho))]
#[invariant(::Abstraction(abstraction) => abstraction.nu.is_selmaho(Selmaho::Nu) && abstraction.nai.is_absent_or_cmavo(Cmavo::Nai) && abstraction.additional_nu.iter().all(|additional_nu| additional_nu.nu.is_selmaho(Selmaho::Nu) && additional_nu.nai.is_absent_or_cmavo(Cmavo::Nai)) && abstraction.kei.is_absent_or_cmavo(Cmavo::Kei))]
#[invariant(::Me => me.is_cmavo(Cmavo::Me) && mehu.is_absent_or_cmavo(Cmavo::Mehu) && moi_marker.is_absent_or_selmaho(Selmaho::Moi))]
#[invariant(::Mehoi(mehoi) => mehoi.is_quote_marker_cmavo(Cmavo::Mehoi))]
#[invariant(::Gohoi(gohoi) => gohoi.is_quote_marker_cmavo(Cmavo::Gohoi))]
#[invariant(::Muhoi(muhoi) => muhoi.is_quote_marker_cmavo(Cmavo::Muhoi))]
#[invariant(::Luhei => luhei.is_cmavo(Cmavo::Luhei) && liau.is_absent_or_cmavo(Cmavo::Lihau))]
#[invariant(::Moi => is_word_run_number_or_letter(number) && moi.is_selmaho(Selmaho::Moi))]
#[invariant(::Nuha => nuha.is_cmavo(Cmavo::Nuha))]
#[invariant(::Xohi => xohi.is_cmavo(Cmavo::Xohi))]
#[invariant(::Cei => !assignments.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum RelationUnitSyntax {
    Word(WithFreeModifiers<Token>),
    Goha {
        goha: WithFreeModifiers<Token>,
        raho: Option<WithFreeModifiers<Token>>,
    },
    Se {
        se: WithFreeModifiers<Token>,
        #[tree_child(primary)]
        inner_unit: Box<RelationUnitSyntax>,
    },
    Ke {
        ke_tense_modal: Option<Box<TenseModalSyntax>>,
        ke: WithFreeModifiers<Token>,
        #[tree_child(primary)]
        relation: Box<RelationSyntax>,
        kehe: Option<WithFreeModifiers<Token>>,
    },
    Nahe {
        nahe: WithFreeModifiers<Token>,
        #[tree_child(primary)]
        inner_unit: Box<RelationUnitSyntax>,
    },
    Bo {
        leading_unit: Box<RelationUnitSyntax>,
        bo_connective: Option<Box<ConnectiveSyntax>>,
        bo_tense_modal: Option<Box<TenseModalSyntax>>,
        bo: WithFreeModifiers<Token>,
        trailing_unit: Box<RelationUnitSyntax>,
    },
    Connected {
        leading_unit: Box<RelationUnitSyntax>,
        connective: ConnectiveSyntax,
        trailing_unit: Box<RelationUnitSyntax>,
    },
    SelbriRelativeClause {
        #[tree_child(primary)]
        base: Box<RelationUnitSyntax>,
        selbri_relative_clauses: Vec<SelbriRelativeClauseSyntax>,
    },
    Wrapped(Box<RelationSyntax>),
    Jai {
        jai: WithFreeModifiers<Token>,
        tense_modal: Option<Box<TenseModalSyntax>>,
        #[tree_child(primary)]
        inner_unit: Box<RelationUnitSyntax>,
    },
    Be {
        #[tree_child(primary)]
        base: Box<RelationUnitSyntax>,
        be: WithFreeModifiers<Token>,
        fa: Option<WithFreeModifiers<Token>>,
        first_argument: Option<Box<ArgumentSyntax>>,
        bei_links: Vec<BeiLinkSyntax>,
        beho: Option<WithFreeModifiers<Token>>,
    },
    PreposedBe {
        be: WithFreeModifiers<Token>,
        fa: Option<WithFreeModifiers<Token>>,
        first_argument: Option<Box<ArgumentSyntax>>,
        bei_links: Vec<BeiLinkSyntax>,
        beho: Option<WithFreeModifiers<Token>>,
        #[tree_child(primary)]
        base: Box<RelationUnitSyntax>,
    },
    Abstraction(Box<AbstractionSyntax>),
    Me {
        me: WithFreeModifiers<Token>,
        #[tree_child(primary)]
        argument: Box<ArgumentSyntax>,
        mehu: Option<WithFreeModifiers<Token>>,
        moi_marker: Option<WithFreeModifiers<Token>>,
    },
    Mehoi(WithFreeModifiers<Token>),
    Gohoi(WithFreeModifiers<Token>),
    Muhoi(WithFreeModifiers<Token>),
    Luhei {
        luhei: WithFreeModifiers<Token>,
        #[tree_child(primary)]
        text: Box<TextSyntax>,
        liau: Option<WithFreeModifiers<Token>>,
    },
    Moi {
        number: WordRun,
        moi: WithFreeModifiers<Token>,
    },
    Nuha {
        nuha: WithFreeModifiers<Token>,
        #[tree_child(primary)]
        math_operator: Box<MathOperatorSyntax>,
    },
    Xohi {
        xohi: WithFreeModifiers<Token>,
        #[tree_child(primary)]
        tag: Box<TenseModalSyntax>,
    },
    Cei {
        #[tree_child(primary)]
        base: Box<RelationUnitSyntax>,
        assignments: Vec<CeiAssignmentSyntax>,
    },
}

#[invariant(cei.is_cmavo(Cmavo::Cei))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CeiAssignmentSyntax {
    pub cei: WithFreeModifiers<Token>,
    #[tree_child(primary)]
    pub relation_unit: Box<RelationUnitSyntax>,
}

}

pub(crate) const RESTRICTIVE_RELATIVE_CLAUSE_CMAVO: &[Cmavo] = &[Cmavo::Poi, Cmavo::Pohoi];
pub(crate) const NONRESTRICTIVE_RELATIVE_CLAUSE_CMAVO: &[Cmavo] =
    &[Cmavo::Noi, Cmavo::Nohoi, Cmavo::Voi, Cmavo::Voihi];

#[requires(true)]
#[ensures(true)]
pub(crate) fn is_word_run_number_or_letter(words: &WordRun) -> bool {
    words.iter().all(|word| {
        word.is_selmaho(Selmaho::Pa)
            || word.is_selmaho(Selmaho::Lau)
            || word.is_one_of_cmavo(&[Cmavo::Tei, Cmavo::Foi])
            || crate::grammar::tokens::is_letter_word(word)
    })
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn is_word_run_cmevla(words: &WordRun) -> bool {
    words.iter().all(crate::grammar::tokens::is_cmevla_word)
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn is_valid_vocative_marker_words(markers: &[Token]) -> bool {
    if markers.is_empty() {
        return false;
    }

    let mut may_take_nai = false;
    for (index, word) in markers.iter().enumerate() {
        if word.is_selmaho(Selmaho::Coi) {
            may_take_nai = true;
        } else if (may_take_nai && word.is_cmavo(Cmavo::Nai))
            || (word.is_cmavo(Cmavo::Doi) && index + 1 == markers.len())
        {
            may_take_nai = false;
        } else {
            return false;
        }
    }
    true
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn is_valid_connective_parts(
    se: &Option<Token>,
    nahe: &Option<Token>,
    na: &Option<Token>,
    cmavo: &WithFreeModifiers<Vec<Token>>,
    nai: &Option<WithFreeModifiers<Token>>,
) -> bool {
    se.is_absent_or_selmaho(Selmaho::Se)
        && nahe.is_absent_or_selmaho(Selmaho::Nahe)
        && na.is_absent_or_selmaho(Selmaho::Na)
        && !cmavo.value.is_empty()
        && is_valid_connective_words(&cmavo.value)
        && nai.is_absent_or_cmavo(Cmavo::Nai)
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn is_valid_connective_words(words: &[Token]) -> bool {
    let mut in_fiho_modal = false;
    let mut fiho_modal_has_relation_word = false;
    let mut segment_has_word = false;
    let mut last_was_i_separator = false;

    for (index, word) in words.iter().enumerate() {
        if in_fiho_modal {
            if word.is_cmavo(Cmavo::Fehu) {
                if !fiho_modal_has_relation_word {
                    return false;
                }
                in_fiho_modal = false;
                fiho_modal_has_relation_word = false;
                segment_has_word = true;
                last_was_i_separator = false;
                continue;
            } else if is_valid_fiho_modal_relation_word(word) {
                fiho_modal_has_relation_word |= crate::grammar::tokens::is_relation_word(word);
                segment_has_word = true;
                last_was_i_separator = false;
                continue;
            } else if fiho_modal_has_relation_word {
                in_fiho_modal = false;
                fiho_modal_has_relation_word = false;
            } else {
                return false;
            }
        }

        if word.is_cmavo(Cmavo::I) {
            if !segment_has_word || last_was_i_separator || index + 1 == words.len() {
                return false;
            }
            segment_has_word = false;
            last_was_i_separator = true;
        } else if word.is_cmavo(Cmavo::Fiho) {
            in_fiho_modal = true;
            fiho_modal_has_relation_word = false;
            segment_has_word = true;
            last_was_i_separator = false;
        } else if !is_valid_connective_word(word) {
            return false;
        } else {
            segment_has_word = true;
            last_was_i_separator = false;
        }
    }

    (!in_fiho_modal || fiho_modal_has_relation_word) && segment_has_word
}

#[requires(true)]
#[ensures(true)]
fn is_valid_connective_word(word: &Token) -> bool {
    word.is_one_of_selmaho(&[
        Selmaho::A,
        Selmaho::Cehe,
        Selmaho::Ja,
        Selmaho::Joi,
        Selmaho::Bihi,
        Selmaho::Gaho,
        Selmaho::Giha,
        Selmaho::Ga,
        Selmaho::Guha,
        Selmaho::Vuhu,
    ]) || word.is_one_of_cmavo(&[Cmavo::Gi, Cmavo::Bo])
        || is_valid_tense_modal_word(word)
}

#[requires(true)]
#[ensures(true)]
fn is_valid_fiho_modal_relation_word(word: &Token) -> bool {
    crate::grammar::tokens::is_relation_word(word)
        || word.is_selmaho(Selmaho::Se)
        || word.is_one_of_cmavo(&[Cmavo::Ke, Cmavo::Kehe, Cmavo::Bo])
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn is_valid_tense_modal_word(word: &Token) -> bool {
    word.is_one_of_selmaho(&[
        Selmaho::Pu,
        Selmaho::Zi,
        Selmaho::Va,
        Selmaho::Zeha,
        Selmaho::Faha,
        Selmaho::Veha,
        Selmaho::Viha,
        Selmaho::Caha,
        Selmaho::Zaho,
        Selmaho::Roi,
        Selmaho::Tahe,
        Selmaho::Bai,
        Selmaho::Nahe,
        Selmaho::Se,
        Selmaho::Pa,
        Selmaho::Fa,
    ]) || word.is_one_of_cmavo(&[
        Cmavo::Ki,
        Cmavo::Cuhe,
        Cmavo::Nau,
        Cmavo::Fehe,
        Cmavo::Mohi,
        Cmavo::Nai,
    ]) || crate::grammar::tokens::is_letter_word(word)
}

impl<T: TreeNode> TreeNode for WithFreeModifiers<T> {
    #[requires(true)]
    #[ensures(true)]
    fn visit_in_order<'tree, V>(&'tree self, visitor: &mut V)
    where
        V: jbotci_tree::TreeVisitor<'tree, Node = NodeRef<'tree>, Atom = AtomRef<'tree>>,
    {
        self.value.visit_in_order(visitor);
        if !self.free_modifiers.is_empty() {
            let field_ref = jbotci_tree::FieldRef::new(Some("free_modifiers"), 1, false);
            visitor.enter_field(field_ref);
            self.free_modifiers.visit_in_order(visitor);
            visitor.exit_field(field_ref);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn path_to_node_from<'tree>(
        &'tree self,
        target: NodeRef<'tree>,
        path: &mut jbotci_tree::TreePath,
    ) -> bool {
        if self.value.path_to_node_from(target, path) {
            return true;
        }
        if !self.free_modifiers.is_empty() {
            path.push(jbotci_tree::TreePathStep::field(Some("free_modifiers"), 1));
            if self.free_modifiers.path_to_node_from(target, path) {
                return true;
            }
            path.pop();
        }
        false
    }

    #[requires(true)]
    #[ensures(true)]
    fn node_at_path_steps<'tree>(
        &'tree self,
        steps: &[jbotci_tree::TreePathStep],
    ) -> Option<NodeRef<'tree>> {
        if let Some(node) = self.value.node_at_path_steps(steps) {
            return Some(node);
        }
        if let Some((step, rest)) = steps.split_first()
            && step.is_field(Some("free_modifiers"), 1)
        {
            return self.free_modifiers.node_at_path_steps(rest);
        }
        None
    }
}

impl Indicator {
    #[requires(true)]
    #[ensures(true)]
    pub fn new(indicator: Token, nai: Option<Word>) -> Self {
        new!(Indicator {
            indicator: indicator,
            nai: nai,
        })
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn words(&self) -> Vec<Token> {
        let mut words = vec![self.indicator.clone()];
        if let Some(nai) = &self.nai {
            words.push(Token::bare(WordLike::bare(nai.clone())));
        }
        words
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&Token)) {
        visitor(&self.indicator);
        if let Some(nai) = &self.nai {
            let nai = Token::bare(WordLike::bare(nai.clone()));
            visitor(&nai);
        }
    }

    #[requires(true)]
    #[ensures(ret >= 1)]
    pub fn word_count(&self) -> usize {
        1 + usize::from(self.nai.is_some())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[allow(unused_imports)]
    use bityzba::{ensures, invariant, requires};
    use jbotci_tree::FieldRef;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn absent_optional_fields_map_to_cll_terminators() {
        let node = parsed_text_node("mi klama");
        for (field, cmavo) in [
            ("tuhu", Cmavo::Tuhu),
            ("vau", Cmavo::Vau),
            ("kehe", Cmavo::Kehe),
            ("ku", Cmavo::Ku),
            ("luhu", Cmavo::Luhu),
            ("loho", Cmavo::Loho),
            ("lihu", Cmavo::Lihu),
            ("gehu", Cmavo::Gehu),
            ("kuho", Cmavo::Kuho),
            ("beho", Cmavo::Beho),
            ("nuhu", Cmavo::Nuhu),
            ("boi", Cmavo::Boi),
            ("veho", Cmavo::Veho),
            ("kuhe", Cmavo::Kuhe),
            ("tehu", Cmavo::Tehu),
            ("mehu", Cmavo::Mehu),
            ("kei", Cmavo::Kei),
            ("fehu", Cmavo::Fehu),
            ("sehu", Cmavo::Sehu),
            ("dohu", Cmavo::Dohu),
            ("toi", Cmavo::Toi),
        ] {
            assert_eq!(
                elidable_terminator_for_absent_field(node, FieldRef::new(Some(field), 0, false),),
                Some(cmavo),
                "{field}"
            );
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn absent_optional_fields_map_to_experimental_terminators() {
        let node = parsed_text_node("mi klama");
        for (field, cmavo) in [
            ("gihi", Cmavo::Gihi),
            ("fihau", Cmavo::Fihau),
            ("kuhau", Cmavo::Kuhau),
            ("kuhoi", Cmavo::Kuhoi),
            ("liau", Cmavo::Lihau),
        ] {
            assert_eq!(
                elidable_terminator_for_absent_field(node, FieldRef::new(Some(field), 0, false),),
                Some(cmavo),
                "{field}"
            );
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn absent_optional_field_mapping_rejects_non_terminators() {
        let node = parsed_text_node("mi klama");
        assert_eq!(
            elidable_terminator_for_absent_field(
                node,
                FieldRef::new(Some("leading_terms"), 0, false),
            ),
            None
        );
    }

    #[requires(!source.is_empty())]
    #[ensures(true)]
    fn parsed_text_node(source: &str) -> NodeRef<'static> {
        let words = jbotci_morphology::segment_words_with_modifiers(source).expect("morphology");
        let parsed = crate::parse_syntax_tree(&words).expect("syntax");
        let tree = Box::leak(parsed.parse_tree.clone());
        NodeRef::TextSyntax(tree)
    }
}
