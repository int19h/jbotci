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
#[invariant(::Plain(_) => true)]
#[invariant(::Emphasized => true)]
#[invariant(::WithIndicator => true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WithIndicators<T> {
    Plain(T),
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
        WithIndicators::Plain(word_like)
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
            WithIndicators::Plain(word_like) | WithIndicators::Emphasized { word_like, .. } => {
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
            WithIndicators::Plain(word_like) => word_like.source_spans_into(out),
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

#[contract_trait]
impl OptionalSyntaxCmavoExt for Option<Arc<WithFreeModifiers<Token>>> {
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
            WithIndicators::Plain(word_like) => write!(f, "{word_like}"),
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
pub type MeksoVec = Vec1<MeksoSyntax>;

#[invariant(indicator.core_word().bare_word().is_some_and(crate::is_indicator_word))]
#[invariant(nai.as_ref().is_none_or(|nai| nai.is_cmavo(Cmavo::Nai)))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Indicator {
    pub indicator: Token,
    pub nai: Option<Word>,
}

#[invariant(cu.is_absent_or_cmavo(Cmavo::Cu))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BridiSyntax {
    pub leading_terms: Vec<TermSyntax>,
    pub cu: Option<Arc<WithFreeModifiers<Token>>>,
    #[tree_child(primary)]
    pub bridi_tail: Box<BridiTailSyntax>,
    pub free_modifiers: Vec<FreeModifierSyntax>,
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BridiTailSyntax {
    #[tree_child(primary)]
    pub first: Box<AfterthoughtBridiTailSyntax>,
    pub ke_continuation: Option<Box<GroupedBridiTailConnectionSyntax>>,
}

#[invariant(ke.is_cmavo(Cmavo::Ke))]
#[invariant(kehe.is_absent_or_cmavo(Cmavo::Kehe))]
#[invariant(vau.is_absent_or_cmavo(Cmavo::Vau))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GroupedBridiTailConnectionSyntax {
    pub connective: ConnectiveSyntax,
    pub tense_modal: Option<Box<TenseModalSyntax>>,
    pub ke: WithFreeModifiers<Token>,
    #[tree_child(primary)]
    pub bridi_tail: Box<BridiTailSyntax>,
    pub kehe: Option<Arc<WithFreeModifiers<Token>>>,
    pub tail_terms: Vec<TermSyntax>,
    pub vau: Option<Arc<WithFreeModifiers<Token>>>,
    pub free_modifiers: Vec<FreeModifierSyntax>,
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AfterthoughtBridiTailSyntax {
    #[tree_child(primary)]
    pub first: Box<BoGroupedBridiTailSyntax>,
    pub continuations: Vec<BridiTailConnectionSyntax>,
}

#[invariant(cu.is_absent_or_cmavo(Cmavo::Cu))]
#[invariant(vau.is_absent_or_cmavo(Cmavo::Vau))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BridiTailConnectionSyntax {
    pub connective: ConnectiveSyntax,
    pub tense_modal: Option<Box<TenseModalSyntax>>,
    pub cu: Option<Arc<WithFreeModifiers<Token>>>,
    #[tree_child(primary)]
    pub bridi_tail: Box<BoGroupedBridiTailSyntax>,
    pub tail_terms: Vec<TermSyntax>,
    pub vau: Option<Arc<WithFreeModifiers<Token>>>,
    pub free_modifiers: Vec<FreeModifierSyntax>,
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BoGroupedBridiTailSyntax {
    #[tree_child(primary)]
    pub first: Box<SimpleBridiTailSyntax>,
    pub bo_continuation: Option<Box<BoundBridiTailConnectionSyntax>>,
}

#[invariant(bo.is_cmavo(Cmavo::Bo))]
#[invariant(cu.is_absent_or_cmavo(Cmavo::Cu))]
#[invariant(vau.is_absent_or_cmavo(Cmavo::Vau))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BoundBridiTailConnectionSyntax {
    pub connective: ConnectiveSyntax,
    pub tense_modal: Option<Box<TenseModalSyntax>>,
    pub bo: WithFreeModifiers<Token>,
    pub cu: Option<Arc<WithFreeModifiers<Token>>>,
    #[tree_child(primary)]
    pub bridi_tail: Box<BoGroupedBridiTailSyntax>,
    pub tail_terms: Vec<TermSyntax>,
    pub vau: Option<Arc<WithFreeModifiers<Token>>>,
    pub free_modifiers: Vec<FreeModifierSyntax>,
}

#[invariant(true)]
#[invariant(::SelbriBridiTail => vau.is_absent_or_cmavo(Cmavo::Vau))]
#[invariant(::ForethoughtBridiTailConnection(..) => true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum SimpleBridiTailSyntax {
    SelbriBridiTail {
        #[tree_child(primary)]
        selbri: Box<SelbriSyntax>,
        terms: Vec<TermSyntax>,
        vau: Option<Arc<WithFreeModifiers<Token>>>,
        free_modifiers: Vec<FreeModifierSyntax>,
    },
    ForethoughtBridiTailConnection(Box<ForethoughtBridiConnectionSyntax>),
}

#[invariant(true)]
#[invariant(::BridiConnection => gihi.is_absent_or_selmaho(Selmaho::Gihi) && vau.is_absent_or_cmavo(Cmavo::Vau))]
#[invariant(::GroupedBridiConnection => ke.is_cmavo(Cmavo::Ke) && kehe.is_absent_or_cmavo(Cmavo::Kehe))]
#[invariant(::NegatedBridiConnection => na.is_selmaho(Selmaho::Na))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum ForethoughtBridiConnectionSyntax {
    BridiConnection {
        gek: ConnectiveSyntax,
        first: Box<SubbridiSyntax>,
        gik: ConnectiveSyntax,
        second: Box<SubbridiSyntax>,
        gihi: Option<Token>,
        tail_terms: Vec<TermSyntax>,
        vau: Option<Arc<WithFreeModifiers<Token>>>,
        free_modifiers: Vec<FreeModifierSyntax>,
    },
    GroupedBridiConnection {
        tense_modal: Option<Box<TenseModalSyntax>>,
        ke: WithFreeModifiers<Token>,
        #[tree_child(primary)]
        inner: Box<ForethoughtBridiConnectionSyntax>,
        kehe: Option<Arc<WithFreeModifiers<Token>>>,
    },
    NegatedBridiConnection {
        na: WithFreeModifiers<Token>,
        #[tree_child(primary)]
        inner: Box<ForethoughtBridiConnectionSyntax>,
    },
}

#[invariant(true)]
#[invariant(::Bridi(..) => true)]
#[invariant(::Prenex => zohu.is_cmavo(Cmavo::Zohu))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum SubbridiSyntax {
    Bridi(Box<BridiSyntax>),
    Prenex {
        prenex_terms: Vec<TermSyntax>,
        zohu: WithFreeModifiers<Token>,
        #[tree_child(primary)]
        inner_subbridi: Box<SubbridiSyntax>,
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
#[invariant(::MetalinguisticBridi => sei.is_selmaho(Selmaho::Sei) && cu.is_absent_or_cmavo(Cmavo::Cu) && sehu.is_absent_or_cmavo(Cmavo::Sehu))]
#[invariant(::ParentheticalText => to.is_selmaho(Selmaho::To) && toi.is_absent_or_cmavo(Cmavo::Toi))]
#[invariant(::Subscript => xi.is_selmaho(Selmaho::Xi))]
#[invariant(::UtteranceOrdinal => is_word_run_number_or_letter(number) && mai.is_selmaho(Selmaho::Mai))]
#[invariant(::ReciprocalSumti => soi.is_selmaho(Selmaho::Soi) && sehu.is_absent_or_cmavo(Cmavo::Sehu))]
#[invariant(::Vocative => is_valid_vocative_marker_words(&vocative_markers.value) && dohu.is_absent_or_cmavo(Cmavo::Dohu))]
#[invariant(::TextReplacement => lohai.is_absent_or_cmavo(Cmavo::Lohai) && sahai.is_absent_or_cmavo(Cmavo::Sahai) && lehai.is_cmavo(Cmavo::Lehai))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum FreeModifierSyntax {
    MetalinguisticBridi {
        sei: WithFreeModifiers<Token>,
        terms: Vec<TermSyntax>,
        cu: Option<WithFreeModifiers<Token>>,
        selbri: Box<SelbriSyntax>,
        sehu: Option<WithFreeModifiers<Token>>,
    },
    ParentheticalText {
        to: WithFreeModifiers<Token>,
        #[tree_child(primary)]
        text: Box<TextSyntax>,
        toi: Option<WithFreeModifiers<Token>>,
    },
    Subscript {
        xi: WithFreeModifiers<Token>,
        #[tree_child(primary)]
        expression: Box<MeksoSyntax>,
    },
    UtteranceOrdinal {
        number: WordRun,
        mai: WithFreeModifiers<Token>,
    },
    ReciprocalSumti {
        soi: WithFreeModifiers<Token>,
        leading_sumti: Box<SumtiSyntax>,
        trailing_sumti: Option<Box<SumtiSyntax>>,
        sehu: Option<WithFreeModifiers<Token>>,
    },
    Vocative {
        vocative_markers: WithFreeModifiers<Vec<Token>>,
        sumti: Option<Box<SumtiSyntax>>,
        dohu: Option<WithFreeModifiers<Token>>,
    },
    TextReplacement {
        lohai: Option<Token>,
        old_words: Vec<Token>,
        sahai: Option<Token>,
        new_words: Vec<Token>,
        lehai: WithFreeModifiers<Token>,
    },
}

#[invariant(true)]
#[invariant(::TextGroup => tuhe.is_cmavo(Cmavo::Tuhe) && tuhu.is_absent_or_cmavo(Cmavo::Tuhu))]
#[invariant(::Prenex => zohu.is_cmavo(Cmavo::Zohu))]
#[invariant(::Bridi(..) => true)]
#[invariant(::StatementConnection => i.is_cmavo(Cmavo::I))]
#[invariant(::PreposedIStatementConnection => i.is_cmavo(Cmavo::I))]
#[invariant(::Iau => iau.is_cmavo(Cmavo::Ihau))]
#[invariant(::ExperimentalBridiContinuation => true)]
#[invariant(::Fragment(..) => true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum StatementSyntax {
    TextGroup {
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
    Bridi(Box<BridiSyntax>),
    StatementConnection {
        leading_statement: Box<StatementSyntax>,
        i: Token,
        connective: ConnectiveSyntax,
        trailing_statement: Box<StatementSyntax>,
    },
    PreposedIStatementConnection {
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
    ExperimentalBridiContinuation {
        leading_statement: Box<StatementSyntax>,
        continuation: BridiStatementContinuationSyntax,
    },
    Fragment(Box<FragmentSyntax>),
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BridiStatementContinuationSyntax {
    pub connective: ConnectiveSyntax,
    pub tense_modal: Option<Box<TenseModalSyntax>>,
    pub marker: BridiStatementContinuationMarkerSyntax,
    pub trailing_subbridi: Box<SubbridiSyntax>,
}

#[invariant(true)]
#[invariant(::BoGrouped(bo) => bo.is_cmavo(Cmavo::Bo))]
#[invariant(::KeGrouped => ke.is_cmavo(Cmavo::Ke) && kehe.is_absent_or_cmavo(Cmavo::Kehe))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum BridiStatementContinuationMarkerSyntax {
    BoGrouped(WithFreeModifiers<Token>),
    KeGrouped {
        ke: WithFreeModifiers<Token>,
        kehe: Option<WithFreeModifiers<Token>>,
    },
}

#[invariant(true)]
#[invariant(::Ek(..) => true)]
#[invariant(::BridiTailConnective(..) => true)]
#[invariant(::Other(words) => !words.value.is_empty())]
#[invariant(::BridiConnective => i.is_cmavo(Cmavo::I))]
#[invariant(::Prenex => zohu.is_cmavo(Cmavo::Zohu))]
#[invariant(::LinkedSumti => be.is_cmavo(Cmavo::Be) && fa.is_absent_or_selmaho(Selmaho::Fa) && (fa.is_none() || first_sumti.is_some()) && beho.is_absent_or_cmavo(Cmavo::Beho))]
#[invariant(::LinkedSumtiContinuation(bei_only_links) => !bei_only_links.is_empty())]
#[invariant(::RelativeClauses(relative_clauses) => !relative_clauses.is_empty())]
#[invariant(::Mekso(..) => true)]
#[invariant(::Terms => vau.is_absent_or_cmavo(Cmavo::Vau))]
#[invariant(::Selbri(..) => true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum FragmentSyntax {
    Ek(ConnectiveSyntax),
    BridiTailConnective(ConnectiveSyntax),
    Other(WithFreeModifiers<Vec<Token>>),
    BridiConnective {
        i: Token,
        connective: ConnectiveSyntax,
    },
    Prenex {
        terms: Vec<TermSyntax>,
        zohu: WithFreeModifiers<Token>,
    },
    LinkedSumti {
        be: WithFreeModifiers<Token>,
        fa: Option<WithFreeModifiers<Token>>,
        first_sumti: Option<Box<SumtiSyntax>>,
        bei_links: Vec<AdditionalLinkedSumtiSyntax>,
        beho: Option<WithFreeModifiers<Token>>,
    },
    LinkedSumtiContinuation(Vec<AdditionalLinkedSumtiSyntax>),
    RelativeClauses(Vec<RelativeClauseSyntax>),
    Mekso(Box<MeksoSyntax>),
    Terms {
        terms: Vec<TermSyntax>,
        vau: Option<WithFreeModifiers<Token>>,
    },
    Selbri(Box<SelbriSyntax>),
}

#[invariant(true)]
#[invariant(::Termset => nuhi.is_cmavo(Cmavo::Nuhi) && !termset.is_empty() && nuhu.is_absent_or_cmavo(Cmavo::Nuhu))]
#[invariant(::ForethoughtTermsetConnection => m_nuhi.as_ref().is_none_or(|nuhi| nuhi.is_cmavo(Cmavo::Nuhi)) && !terms.is_empty() && nuhu.is_absent_or_cmavo(Cmavo::Nuhu) && !gik_terms.is_empty() && gihi.is_absent_or_selmaho(Selmaho::Gihi) && gik_nuhu.is_absent_or_cmavo(Cmavo::Nuhu))]
#[invariant(::TermsetGroup => !leading_terms.is_empty() && cehe.is_cmavo(Cmavo::Cehe) && !trailing_terms.is_empty())]
#[invariant(::TermsetConnection => !leading_terms.is_empty() && pehe.is_cmavo(Cmavo::Pehe) && !trailing_terms.is_empty())]
#[invariant(::Sumti(..) => true)]
#[invariant(::PlaceTaggedSumti => fa.is_selmaho(Selmaho::Fa) && ku.is_absent_or_cmavo(Cmavo::Ku))]
#[invariant(::BridiNegation => na.is_selmaho(Selmaho::Na) && na_ku.is_cmavo(Cmavo::Ku))]
#[invariant(::BareNegation(na) => na.is_selmaho(Selmaho::Na))]
#[invariant(::RelativeAdverbialTerm => noiha.is_selmaho(Selmaho::Noiha) && fehu.is_absent_or_cmavo(Cmavo::Fehu))]
#[invariant(::BridiVariableAdverbialTerm => poiha.is_selmaho(Selmaho::Noiha) && brigahi_ku.is_cmavo(Cmavo::Ku))]
#[invariant(::AdHocBridiAdverbialTerm => fihoi.is_cmavo(Cmavo::Fihoi) && fihau.is_absent_or_cmavo(Cmavo::Fihau))]
#[invariant(::ReciprocalBridiAdverbialTerm => soi.is_selmaho(Selmaho::Soi) && sehu.is_absent_or_cmavo(Cmavo::Sehu))]
#[invariant(::JaiTaggedSumti => jai.is_cmavo(Cmavo::Jai))]
#[invariant(::TaggedSumti => tense_modal.is_some())]
#[invariant(::TermConnection => !leading_terms.is_empty() && !trailing_terms.is_empty())]
#[invariant(::BoundTermConnection => !leading_terms.is_empty() && bo.is_cmavo(Cmavo::Bo))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum TermSyntax {
    Termset {
        nuhi: WithFreeModifiers<Token>,
        termset: Vec<TermSyntax>,
        nuhu: Option<WithFreeModifiers<Token>>,
    },
    ForethoughtTermsetConnection {
        m_nuhi: Option<WithFreeModifiers<Token>>,
        gek: ConnectiveSyntax,
        terms: Vec<TermSyntax>,
        nuhu: Option<WithFreeModifiers<Token>>,
        gik: ConnectiveSyntax,
        gik_terms: Vec<TermSyntax>,
        gihi: Option<Token>,
        gik_nuhu: Option<WithFreeModifiers<Token>>,
    },
    TermsetGroup {
        leading_terms: Vec<TermSyntax>,
        cehe: WithFreeModifiers<Token>,
        trailing_terms: Vec<TermSyntax>,
    },
    TermsetConnection {
        leading_terms: Vec<TermSyntax>,
        pehe: WithFreeModifiers<Token>,
        connective: ConnectiveSyntax,
        trailing_terms: Vec<TermSyntax>,
    },
    Sumti(Box<SumtiSyntax>),
    PlaceTaggedSumti {
        fa: WithFreeModifiers<Token>,
        #[tree_child(primary)]
        sumti: Box<SumtiSyntax>,
        ku: Option<WithFreeModifiers<Token>>,
    },
    BridiNegation {
        na: Token,
        na_ku: WithFreeModifiers<Token>,
    },
    BareNegation(WithFreeModifiers<Token>),
    RelativeAdverbialTerm {
        noiha: WithFreeModifiers<Token>,
        tail_elements: Vec<DescriptionTailElementSyntax>,
        selbri: Option<Box<SelbriSyntax>>,
        relative_clauses: Vec<RelativeClauseSyntax>,
        fehu: Option<WithFreeModifiers<Token>>,
    },
    BridiVariableAdverbialTerm {
        poiha: WithFreeModifiers<Token>,
        tail_elements: Vec<DescriptionTailElementSyntax>,
        selbri: Option<Box<SelbriSyntax>>,
        relative_clauses: Vec<RelativeClauseSyntax>,
        brigahi_ku: WithFreeModifiers<Token>,
    },
    AdHocBridiAdverbialTerm {
        fihoi: WithFreeModifiers<Token>,
        #[tree_child(primary)]
        subbridi: Box<SubbridiSyntax>,
        fihau: Option<WithFreeModifiers<Token>>,
    },
    ReciprocalBridiAdverbialTerm {
        soi: WithFreeModifiers<Token>,
        #[tree_child(primary)]
        subbridi: Box<SubbridiSyntax>,
        sehu: Option<WithFreeModifiers<Token>>,
    },
    JaiTaggedSumti {
        jai: WithFreeModifiers<Token>,
        tag: Option<Box<TenseModalSyntax>>,
        #[tree_child(primary)]
        sumti: Box<SumtiSyntax>,
    },
    TaggedSumti {
        tense_modal: Option<Box<TenseModalSyntax>>,
        #[tree_child(primary)]
        sumti: Box<SumtiSyntax>,
    },
    TermConnection {
        leading_terms: Vec<TermSyntax>,
        connective: ConnectiveSyntax,
        trailing_terms: Vec<TermSyntax>,
    },
    BoundTermConnection {
        leading_terms: Vec<TermSyntax>,
        bo_connective: Option<Box<ConnectiveSyntax>>,
        tense_modal: Option<Box<TenseModalSyntax>>,
        bo: WithFreeModifiers<Token>,
        trailing_term: Box<TermSyntax>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum SumtiWrapperKindSyntax {
    Referent,
    ScalarNegationWithBo,
    ScalarNegation,
}

#[invariant(true)]
#[invariant(::TenseModal(..) => true)]
#[invariant(::PlaceTag(fa) => fa.is_selmaho(Selmaho::Fa))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum SumtiTagSyntax {
    TenseModal(Box<TenseModalSyntax>),
    PlaceTag(WithFreeModifiers<Token>),
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SumtiConnectionSyntax {
    pub connective: ConnectiveSyntax,
    #[tree_child(primary)]
    pub sumti: Box<SumtiSyntax>,
}

#[invariant(true)]
#[invariant(::QuotedSumti(..) => true)]
#[invariant(::NumberSumti => li.is_selmaho(Selmaho::Li) && loho.is_absent_or_cmavo(Cmavo::Loho))]
#[invariant(::LerfuStringSumti => is_word_run_number_or_letter(&letter.value) && boi.is_absent_or_cmavo(Cmavo::Boi))]
#[invariant(::QuantifiedSumti => true)]
#[invariant(::SumtiWithRelativeClauses => vuho.is_absent_or_cmavo(Cmavo::Vuho) && !relative_clauses.is_empty())]
#[invariant(::SumtiWithComplexRelativeClauses => vuho_marker.is_cmavo(Cmavo::Vuho) && (!relative_clauses.is_empty() || sumti_connection.is_some()))]
#[invariant(::BridiDescription => lohoi.is_selmaho(Selmaho::Lohoi) && kuhau.is_absent_or_cmavo(Cmavo::Kuhau))]
#[invariant(::NegatedSumti => na.is_selmaho(Selmaho::Na) && ku.is_cmavo(Cmavo::Ku))]
#[invariant(::TaggedSumti => true)]
#[invariant(::ScalarNegatedSumtiWithBo => nahe.is_selmaho(Selmaho::Nahe) && bo.is_cmavo(Cmavo::Bo) && luhu.is_absent_or_cmavo(Cmavo::Luhu))]
#[invariant(::ScalarNegatedSumti => nahe.is_selmaho(Selmaho::Nahe) && luhu.is_absent_or_cmavo(Cmavo::Luhu))]
#[invariant(::QualifiedTerm => match term_wrapper_kind {
    SumtiWrapperKindSyntax::Referent => wrapper.is_selmaho(Selmaho::Lahe) && wrapper_bo.is_none(),
    SumtiWrapperKindSyntax::ScalarNegationWithBo => wrapper.is_selmaho(Selmaho::Nahe)
        && wrapper_bo.as_ref().is_some_and(|bo| bo.is_cmavo(Cmavo::Bo)),
    SumtiWrapperKindSyntax::ScalarNegation => wrapper.is_selmaho(Selmaho::Nahe) && wrapper_bo.is_none(),
} && luhu.is_absent_or_cmavo(Cmavo::Luhu))]
#[invariant(::ProSumti(koha) => crate::grammar::tokens::is_koha_argument(&koha.value))]
#[invariant(::ElidedSumti => maybe_ku.is_absent_or_cmavo(Cmavo::Ku))]
#[invariant(::ReferentSumti => lahe.is_selmaho(Selmaho::Lahe) && luhu.is_absent_or_cmavo(Cmavo::Luhu))]
#[invariant(::SumtiConnection => true)]
#[invariant(::GroupedSumti => ke.is_cmavo(Cmavo::Ke) && kehe.is_absent_or_cmavo(Cmavo::Kehe))]
#[invariant(::BoundSumtiConnection => bo.is_cmavo(Cmavo::Bo))]
#[invariant(::ForethoughtSumtiConnection => gihi.is_absent_or_selmaho(Selmaho::Gihi))]
#[invariant(::Description(description) => description.description.as_ref().is_none_or(|marker| marker.is_one_of_selmaho(&[Selmaho::Le, Selmaho::La])) && description.ku.is_absent_or_cmavo(Cmavo::Ku) && (description.description.is_some() || (!description.tail_elements.is_empty() && description.selbri.is_some())))]
#[invariant(::DescriptionConnection(description) => description.leading_description_head.description.is_one_of_selmaho(&[Selmaho::Le, Selmaho::La]) && description.trailing_description_head.description.is_one_of_selmaho(&[Selmaho::Le, Selmaho::La]) && description.ku.is_absent_or_cmavo(Cmavo::Ku))]
#[invariant(::NameDescription => la.is_selmaho(Selmaho::La) && is_word_run_cmevla(&names.value))]
#[invariant(::NameWords(names) => is_word_run_cmevla(&names.value))]
#[invariant(::SelbriVocative => true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum SumtiSyntax {
    QuotedSumti(Box<QuoteSyntax>),
    NumberSumti {
        li: WithFreeModifiers<Token>,
        #[tree_child(primary)]
        expression: Box<MeksoSyntax>,
        loho: Option<WithFreeModifiers<Token>>,
    },
    LerfuStringSumti {
        letter: WithFreeModifiers<WordRun>,
        boi: Option<WithFreeModifiers<Token>>,
    },
    QuantifiedSumti {
        quantifier: QuantifierSyntax,
        #[tree_child(primary)]
        inner_sumti: Box<SumtiSyntax>,
    },
    SumtiWithRelativeClauses {
        base_sumti: Box<SumtiSyntax>,
        vuho: Option<WithFreeModifiers<Token>>,
        relative_clauses: Vec<RelativeClauseSyntax>,
    },
    SumtiWithComplexRelativeClauses {
        base_sumti: Box<SumtiSyntax>,
        vuho_marker: WithFreeModifiers<Token>,
        relative_clauses: Vec<RelativeClauseSyntax>,
        sumti_connection: Option<Box<SumtiConnectionSyntax>>,
    },
    BridiDescription {
        lohoi: WithFreeModifiers<Token>,
        #[tree_child(primary)]
        subbridi: Box<SubbridiSyntax>,
        kuhau: Option<WithFreeModifiers<Token>>,
    },
    NegatedSumti {
        na: Token,
        ku: WithFreeModifiers<Token>,
    },
    TaggedSumti {
        tag: SumtiTagSyntax,
        #[tree_child(primary)]
        inner_sumti: Box<SumtiSyntax>,
    },
    ScalarNegatedSumtiWithBo {
        nahe: Token,
        bo: WithFreeModifiers<Token>,
        #[tree_child(primary)]
        inner_sumti: Box<SumtiSyntax>,
        luhu: Option<WithFreeModifiers<Token>>,
    },
    ScalarNegatedSumti {
        nahe: WithFreeModifiers<Token>,
        #[tree_child(primary)]
        inner_sumti: Box<SumtiSyntax>,
        luhu: Option<WithFreeModifiers<Token>>,
    },
    QualifiedTerm {
        term_wrapper_kind: SumtiWrapperKindSyntax,
        wrapper: WithFreeModifiers<Token>,
        wrapper_bo: Option<WithFreeModifiers<Token>>,
        #[tree_child(primary)]
        inner_term: Box<TermSyntax>,
        luhu: Option<WithFreeModifiers<Token>>,
    },
    ProSumti(WithFreeModifiers<Token>),
    ElidedSumti {
        tag: Option<Box<SumtiTagSyntax>>,
        maybe_ku: Option<WithFreeModifiers<Token>>,
        free_modifiers: Vec<FreeModifierSyntax>,
    },
    ReferentSumti {
        lahe: WithFreeModifiers<Token>,
        relative_clauses: Vec<RelativeClauseSyntax>,
        #[tree_child(primary)]
        inner_sumti: Box<SumtiSyntax>,
        luhu: Option<WithFreeModifiers<Token>>,
    },
    SumtiConnection {
        leading_sumti: Box<SumtiSyntax>,
        connective: ConnectiveSyntax,
        trailing_sumti: Box<SumtiSyntax>,
    },
    GroupedSumti {
        ke: WithFreeModifiers<Token>,
        #[tree_child(primary)]
        inner_sumti: Box<SumtiSyntax>,
        kehe: Option<WithFreeModifiers<Token>>,
    },
    BoundSumtiConnection {
        leading_sumti: Box<SumtiSyntax>,
        bo_connective: Option<Box<ConnectiveSyntax>>,
        bo_tense_modal: Option<Box<TenseModalSyntax>>,
        bo: WithFreeModifiers<Token>,
        trailing_sumti: Box<SumtiSyntax>,
    },
    ForethoughtSumtiConnection {
        gek: ConnectiveSyntax,
        leading_sumti: Box<SumtiSyntax>,
        gik: ConnectiveSyntax,
        trailing_sumti: Box<SumtiSyntax>,
        gihi: Option<Token>,
    },
    Description(Box<DescriptionSyntax>),
    DescriptionConnection(Box<DescriptionConnectionSyntax>),
    NameDescription {
        la: WithFreeModifiers<Token>,
        names: WithFreeModifiers<WordRun>,
    },
    NameWords(WithFreeModifiers<WordRun>),
    SelbriVocative {
        leading_relative_clauses: Vec<RelativeClauseSyntax>,
        selbri: Box<SelbriSyntax>,
        trailing_relative_clauses: Vec<RelativeClauseSyntax>,
    },
}

#[invariant(true)]
#[invariant(::SumtiAssociationPhrase(phrase) => phrase.association_marker.is_selmaho(Selmaho::Goi) && phrase.gehu.is_absent_or_cmavo(Cmavo::Gehu))]
#[invariant(::IncidentalRelativeBridi => noi.is_one_of_cmavo(NONRESTRICTIVE_RELATIVE_CLAUSE_CMAVO) && kuho.is_absent_or_cmavo(Cmavo::Kuho))]
#[invariant(::RestrictiveRelativeBridi => poi.is_one_of_cmavo(RESTRICTIVE_RELATIVE_CLAUSE_CMAVO) && kuho.is_absent_or_cmavo(Cmavo::Kuho))]
#[invariant(::JoinedRelativeClauses => zihe.is_cmavo(Cmavo::Zihe))]
#[invariant(::RelativeClauseConnection => true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum RelativeClauseSyntax {
    SumtiAssociationPhrase(Box<SumtiAssociationPhraseSyntax>),
    IncidentalRelativeBridi {
        noi: WithFreeModifiers<Token>,
        #[tree_child(primary)]
        subbridi: Box<SubbridiSyntax>,
        kuho: Option<WithFreeModifiers<Token>>,
    },
    RestrictiveRelativeBridi {
        poi: WithFreeModifiers<Token>,
        #[tree_child(primary)]
        subbridi: Box<SubbridiSyntax>,
        kuho: Option<WithFreeModifiers<Token>>,
    },
    JoinedRelativeClauses {
        zihe: WithFreeModifiers<Token>,
        #[tree_child(primary)]
        inner: Box<RelativeClauseSyntax>,
    },
    RelativeClauseConnection {
        connective: ConnectiveSyntax,
        #[tree_child(primary)]
        inner: Box<RelativeClauseSyntax>,
    },
}

#[invariant(association_marker.is_selmaho(Selmaho::Goi))]
#[invariant(gehu.is_absent_or_cmavo(Cmavo::Gehu))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SumtiAssociationPhraseSyntax {
    pub association_marker: WithFreeModifiers<Token>,
    #[tree_child(primary)]
    pub sumti: Box<SumtiSyntax>,
    pub gehu: Option<WithFreeModifiers<Token>>,
}

#[invariant(nohoi.is_cmavo(Cmavo::Nohoi))]
#[invariant(kuhoi.is_absent_or_cmavo(Cmavo::Kuhoi))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SelbriRelativePhraseSyntax {
    pub nohoi: WithFreeModifiers<Token>,
    #[tree_child(primary)]
    pub selbri: Box<SelbriSyntax>,
    pub kuhoi: Option<WithFreeModifiers<Token>>,
}

#[invariant(true)]
#[invariant(::TextQuote => lu.is_cmavo(Cmavo::Lu) && lihu.is_absent_or_cmavo(Cmavo::Lihu))]
#[invariant(::WordQuote(zo) => zo.is_quote_marker_cmavo(Cmavo::Zo))]
#[invariant(::DelimitedWordQuote(zohoi) => zohoi.quote_marker_cmavo().is_some_and(|cmavo| [Cmavo::Zohoi, Cmavo::Lahoi, Cmavo::Rahoi, Cmavo::Mehoi, Cmavo::Gohoi].contains(&cmavo)))]
#[invariant(::DelimitedNonLojbanQuote(zoi) => zoi.quote_marker_cmavo().is_some_and(|cmavo| Selmaho::Zoi.contains(cmavo)))]
#[invariant(::WordsQuote(lohu) => lohu.is_quote_marker_cmavo(Cmavo::Lohu))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum QuoteSyntax {
    TextQuote {
        lu: WithFreeModifiers<Token>,
        #[tree_child(primary)]
        text: Box<TextSyntax>,
        lihu: Option<WithFreeModifiers<Token>>,
    },
    WordQuote(WithFreeModifiers<Token>),
    DelimitedWordQuote(WithFreeModifiers<Token>),
    DelimitedNonLojbanQuote(WithFreeModifiers<Token>),
    WordsQuote(WithFreeModifiers<Token>),
}

#[invariant(description.as_ref().is_none_or(|description| description.is_one_of_selmaho(&[Selmaho::Le, Selmaho::La])))]
#[invariant(ku.is_absent_or_cmavo(Cmavo::Ku))]
#[invariant(description.is_some() || (!tail_elements.is_empty() && selbri.is_some()))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DescriptionSyntax {
    pub outer_quantifier: Option<Box<QuantifierSyntax>>,
    pub description: Option<WithFreeModifiers<Token>>,
    pub tail_elements: Vec<DescriptionTailElementSyntax>,
    pub selbri: Option<Box<SelbriSyntax>>,
    pub relative_clauses: Vec<RelativeClauseSyntax>,
    pub ku: Option<WithFreeModifiers<Token>>,
}

#[invariant(description.is_one_of_selmaho(&[Selmaho::Le, Selmaho::La]))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DescriptionHeadSyntax {
    pub description: WithFreeModifiers<Token>,
}

#[invariant(ku.is_absent_or_cmavo(Cmavo::Ku))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DescriptionConnectionSyntax {
    pub leading_description_head: Box<DescriptionHeadSyntax>,
    pub connective: ConnectiveSyntax,
    pub trailing_description_head: Box<DescriptionHeadSyntax>,
    pub tail_elements: Vec<DescriptionTailElementSyntax>,
    pub selbri: Option<Box<SelbriSyntax>>,
    pub relative_clauses: Vec<RelativeClauseSyntax>,
    pub ku: Option<WithFreeModifiers<Token>>,
}

#[invariant(true)]
#[invariant(::Afterthought => is_valid_connective_parts(se, nahe, na, cmavo, nai))]
#[invariant(::Selbri => is_valid_connective_parts(se, nahe, na, cmavo, nai))]
#[invariant(::BridiTail => is_valid_connective_parts(se, nahe, na, cmavo, nai))]
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
        cmavo: Arc<WithFreeModifiers<Vec<Token>>>,
        nai: Option<Arc<WithFreeModifiers<Token>>>,
    },
    Selbri {
        se: Option<Token>,
        nahe: Option<Token>,
        na: Option<Token>,
        #[tree_child(primary)]
        cmavo: Arc<WithFreeModifiers<Vec<Token>>>,
        nai: Option<Arc<WithFreeModifiers<Token>>>,
    },
    BridiTail {
        se: Option<Token>,
        nahe: Option<Token>,
        na: Option<Token>,
        #[tree_child(primary)]
        cmavo: Arc<WithFreeModifiers<Vec<Token>>>,
        nai: Option<Arc<WithFreeModifiers<Token>>>,
    },
    Forethought {
        se: Option<Token>,
        nahe: Option<Token>,
        na: Option<Token>,
        #[tree_child(primary)]
        cmavo: Arc<WithFreeModifiers<Vec<Token>>>,
        nai: Option<Arc<WithFreeModifiers<Token>>>,
    },
    NonLogical {
        se: Option<Token>,
        nahe: Option<Token>,
        na: Option<Token>,
        #[tree_child(primary)]
        cmavo: Arc<WithFreeModifiers<Vec<Token>>>,
        nai: Option<Arc<WithFreeModifiers<Token>>>,
    },
    Interval {
        se: Option<Token>,
        nahe: Option<Token>,
        na: Option<Token>,
        #[tree_child(primary)]
        cmavo: Arc<WithFreeModifiers<Vec<Token>>>,
        nai: Option<Arc<WithFreeModifiers<Token>>>,
    },
}

#[invariant(bei.is_cmavo(Cmavo::Bei))]
#[invariant(fa.is_none() || sumti.is_some(), "lifted FA link tags must have an sumti")]
#[invariant(fa.is_absent_or_selmaho(Selmaho::Fa))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AdditionalLinkedSumtiSyntax {
    pub bei: WithFreeModifiers<Token>,
    pub fa: Option<WithFreeModifiers<Token>>,
    pub sumti: Option<Box<SumtiSyntax>>,
}

#[invariant(fa.is_none() || sumti.is_some(), "lifted FA link tags must have an sumti")]
#[invariant(fa.is_absent_or_selmaho(Selmaho::Fa))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LinkedSumtiSyntax {
    pub fa: Option<WithFreeModifiers<Token>>,
    pub sumti: Option<Box<SumtiSyntax>>,
}

#[invariant(be.is_cmavo(Cmavo::Be))]
#[invariant(fa.is_none() || first_sumti.is_some(), "lifted FA link tags must have an sumti")]
#[invariant(fa.is_absent_or_selmaho(Selmaho::Fa))]
#[invariant(beho.is_absent_or_cmavo(Cmavo::Beho))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LinkedSumtiListSyntax {
    pub be: WithFreeModifiers<Token>,
    pub fa: Option<WithFreeModifiers<Token>>,
    pub first_sumti: Option<Box<SumtiSyntax>>,
    pub bei_links: Vec<AdditionalLinkedSumtiSyntax>,
    pub beho: Option<WithFreeModifiers<Token>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum ConnectiveKind {
    Afterthought,
    Selbri,
    BridiTail,
    Forethought,
    NonLogical,
    Interval,
}

#[invariant(true)]
#[invariant(::DescriptionTailSumti(..) => true)]
#[invariant(::DescriptionTailRelativeClauses(relative_clauses) => !relative_clauses.is_empty())]
#[invariant(::DescriptionTailQuantifier(..) => true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum DescriptionTailElementSyntax {
    DescriptionTailSumti(Box<SumtiSyntax>),
    DescriptionTailRelativeClauses(Vec<RelativeClauseSyntax>),
    DescriptionTailQuantifier(QuantifierSyntax),
}

#[invariant(true)]
#[invariant(::NumberQuantifier => is_word_run_number_or_letter(&number.value) && boi.is_absent_or_cmavo(Cmavo::Boi))]
#[invariant(::MeksoQuantifier => vei.is_cmavo(Cmavo::Vei) && veho.is_absent_or_cmavo(Cmavo::Veho))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum QuantifierSyntax {
    NumberQuantifier {
        #[tree_child(primary)]
        number: WithFreeModifiers<WordRun>,
        boi: Option<WithFreeModifiers<Token>>,
    },
    MeksoQuantifier {
        vei: WithFreeModifiers<Token>,
        #[tree_child(primary)]
        mekso: Box<MeksoSyntax>,
        veho: Option<WithFreeModifiers<Token>>,
    },
}

#[invariant(true)]
#[invariant(::NumberMekso(..) => true)]
#[invariant(::LerfuStringMekso => is_word_run_number_or_letter(&letter.value) && boi.is_absent_or_cmavo(Cmavo::Boi))]
#[invariant(::ParenthesizedMekso => vei.is_cmavo(Cmavo::Vei) && veho.is_absent_or_cmavo(Cmavo::Veho))]
#[invariant(::ForethoughtMeksoConnection => true)]
#[invariant(::ForethoughtCall => peho.as_ref().is_none_or(|peho| peho.is_cmavo(Cmavo::Peho)) && !operands.is_empty() && kuhe.is_absent_or_cmavo(Cmavo::Kuhe))]
#[invariant(::ReversePolish => fuha.is_cmavo(Cmavo::Fuha) && !operands.is_empty())]
#[invariant(::SelbriOperand => nihe.is_cmavo(Cmavo::Nihe) && tehu.is_absent_or_cmavo(Cmavo::Tehu))]
#[invariant(::SumtiOperand => mohe.is_cmavo(Cmavo::Mohe) && tehu.is_absent_or_cmavo(Cmavo::Tehu))]
#[invariant(::MeksoArray => johi.is_cmavo(Cmavo::Johi) && tehu.is_absent_or_cmavo(Cmavo::Tehu))]
#[invariant(::QualifiedOperand => matches!(markers.value.as_slice(), [nahe, bo] if nahe.is_selmaho(Selmaho::Nahe) && bo.is_cmavo(Cmavo::Bo)) && luhu.is_absent_or_cmavo(Cmavo::Luhu))]
#[invariant(::MeksoConnection => true)]
#[invariant(::Infix => true)]
#[invariant(::PrecedenceInfix => bihe.is_cmavo(Cmavo::Bihe))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum MeksoSyntax {
    NumberMekso(Box<QuantifierSyntax>),
    LerfuStringMekso {
        #[tree_child(primary)]
        letter: WithFreeModifiers<WordRun>,
        boi: Option<WithFreeModifiers<Token>>,
    },
    ParenthesizedMekso {
        vei: WithFreeModifiers<Token>,
        #[tree_child(primary)]
        inner_expression: Box<MeksoSyntax>,
        veho: Option<WithFreeModifiers<Token>>,
    },
    ForethoughtMeksoConnection {
        gek: ConnectiveSyntax,
        left_expression: Box<MeksoSyntax>,
        gik: ConnectiveSyntax,
        right_expression: Box<MeksoSyntax>,
    },
    ForethoughtCall {
        peho: Option<WithFreeModifiers<Token>>,
        operator: Box<MeksoOperatorSyntax>,
        operands: Vec<MeksoSyntax>,
        kuhe: Option<WithFreeModifiers<Token>>,
    },
    ReversePolish {
        fuha: WithFreeModifiers<Token>,
        operands: Vec<MeksoSyntax>,
        operators: Vec<MeksoOperatorSyntax>,
    },
    SelbriOperand {
        nihe: WithFreeModifiers<Token>,
        #[tree_child(primary)]
        selbri: Box<SelbriSyntax>,
        tehu: Option<WithFreeModifiers<Token>>,
    },
    SumtiOperand {
        mohe: WithFreeModifiers<Token>,
        #[tree_child(primary)]
        sumti: Box<SumtiSyntax>,
        tehu: Option<WithFreeModifiers<Token>>,
    },
    MeksoArray {
        johi: WithFreeModifiers<Token>,
        #[tree_child(primary)]
        expressions: MeksoVec,
        tehu: Option<WithFreeModifiers<Token>>,
    },
    QualifiedOperand {
        markers: WithFreeModifiers<Vec<Token>>,
        #[tree_child(primary)]
        inner_expression: Box<MeksoSyntax>,
        luhu: Option<WithFreeModifiers<Token>>,
    },
    MeksoConnection {
        left_expression: Box<MeksoSyntax>,
        connective: ConnectiveSyntax,
        right_expression: Box<MeksoSyntax>,
    },
    Infix {
        left_expression: Box<MeksoSyntax>,
        operator: Box<MeksoOperatorSyntax>,
        right_expression: Box<MeksoSyntax>,
    },
    PrecedenceInfix {
        left_expression: Box<MeksoSyntax>,
        bihe: WithFreeModifiers<Token>,
        operator: Box<MeksoOperatorSyntax>,
        right_expression: Box<MeksoSyntax>,
    },
}

#[invariant(true)]
#[invariant(::Primitive(vuhu) => vuhu.is_selmaho(Selmaho::Vuhu))]
#[invariant(::OperandAsOperator => maho.is_cmavo(Cmavo::Maho) && tehu.is_absent_or_cmavo(Cmavo::Tehu))]
#[invariant(::Converted => se.is_selmaho(Selmaho::Se))]
#[invariant(::ScalarNegated => nahe.is_selmaho(Selmaho::Nahe))]
#[invariant(::SelbriAsOperator => nahu.is_cmavo(Cmavo::Nahu) && tehu.is_absent_or_cmavo(Cmavo::Tehu))]
#[invariant(::GroupedOperator => ke.is_cmavo(Cmavo::Ke) && kehe.is_absent_or_cmavo(Cmavo::Kehe))]
#[invariant(::BoundOperatorConnection => bo.is_cmavo(Cmavo::Bo))]
#[invariant(::OperatorConnection => true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum MeksoOperatorSyntax {
    Primitive(WithFreeModifiers<Token>),
    OperandAsOperator {
        maho: WithFreeModifiers<Token>,
        #[tree_child(primary)]
        mekso: Box<MeksoSyntax>,
        tehu: Option<WithFreeModifiers<Token>>,
    },
    Converted {
        se: WithFreeModifiers<Token>,
        #[tree_child(primary)]
        inner_operator: Box<MeksoOperatorSyntax>,
    },
    ScalarNegated {
        nahe: WithFreeModifiers<Token>,
        #[tree_child(primary)]
        inner_operator: Box<MeksoOperatorSyntax>,
    },
    SelbriAsOperator {
        nahu: WithFreeModifiers<Token>,
        #[tree_child(primary)]
        selbri: Box<SelbriSyntax>,
        tehu: Option<WithFreeModifiers<Token>>,
    },
    GroupedOperator {
        ke: WithFreeModifiers<Token>,
        #[tree_child(primary)]
        inner_operator: Box<MeksoOperatorSyntax>,
        kehe: Option<WithFreeModifiers<Token>>,
    },
    BoundOperatorConnection {
        left_operator: Box<MeksoOperatorSyntax>,
        connective: ConnectiveSyntax,
        bo: WithFreeModifiers<Token>,
        right_operator: Box<MeksoOperatorSyntax>,
    },
    OperatorConnection {
        left_operator: Box<MeksoOperatorSyntax>,
        connective: ConnectiveSyntax,
        right_operator: Box<MeksoOperatorSyntax>,
    },
}

#[invariant(true)]
#[invariant(::SelbriConnection => true)]
#[invariant(::InvertedTanru => co.is_cmavo(Cmavo::Co))]
#[invariant(::BoundSelbriConnection => bo.is_cmavo(Cmavo::Bo))]
#[invariant(::Negated => na.is_selmaho(Selmaho::Na))]
#[invariant(::SelbriWord(word) => crate::grammar::tokens::is_relation_word(word) || crate::grammar::tokens::is_cmevla_word(word))]
#[invariant(::ConvertedSelbri => se.is_selmaho(Selmaho::Se))]
#[invariant(::GroupedSelbri => ke.is_cmavo(Cmavo::Ke) && kehe.is_absent_or_cmavo(Cmavo::Kehe))]
#[invariant(::TaggedSelbri => true)]
#[invariant(::ForethoughtSelbriConnection => gihi.is_absent_or_selmaho(Selmaho::Gihi))]
#[invariant(::Abstraction(abstraction) => abstraction.nu.is_selmaho(Selmaho::Nu) && abstraction.nai.is_absent_or_cmavo(Cmavo::Nai) && abstraction.abstractor_connections.iter().all(|connected_abstractor| connected_abstractor.nu.is_selmaho(Selmaho::Nu) && connected_abstractor.nai.is_absent_or_cmavo(Cmavo::Nai)) && abstraction.kei.is_absent_or_cmavo(Cmavo::Kei))]
#[invariant(::Tanru(..) => true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum SelbriSyntax {
    SelbriConnection {
        leading_selbri: Box<SelbriSyntax>,
        connective: ConnectiveSyntax,
        trailing_selbri: Box<SelbriSyntax>,
    },
    InvertedTanru {
        leading_selbri: Box<SelbriSyntax>,
        co: WithFreeModifiers<Token>,
        trailing_selbri: Box<SelbriSyntax>,
    },
    BoundSelbriConnection {
        leading_selbri: Box<SelbriSyntax>,
        bo_connective: Option<Box<ConnectiveSyntax>>,
        bo_tense_modal: Option<Box<TenseModalSyntax>>,
        bo: WithFreeModifiers<Token>,
        trailing_selbri: Box<SelbriSyntax>,
    },
    Negated {
        na: WithFreeModifiers<Token>,
        #[tree_child(primary)]
        inner_selbri: Box<SelbriSyntax>,
    },
    SelbriWord(Token),
    ConvertedSelbri {
        se: WithFreeModifiers<Token>,
        #[tree_child(primary)]
        inner_selbri: Box<SelbriSyntax>,
    },
    GroupedSelbri {
        ke_tense_modal: Option<Box<TenseModalSyntax>>,
        ke: WithFreeModifiers<Token>,
        #[tree_child(primary)]
        selbri: Box<SelbriSyntax>,
        kehe: Option<WithFreeModifiers<Token>>,
    },
    TaggedSelbri {
        tense_modal: Box<TenseModalSyntax>,
        #[tree_child(primary)]
        inner_selbri: Box<SelbriSyntax>,
    },
    ForethoughtSelbriConnection {
        guhek: ConnectiveSyntax,
        leading_bridi: Box<BridiSyntax>,
        gik: ConnectiveSyntax,
        trailing_bridi: Box<BridiSyntax>,
        gihi: Option<Token>,
    },
    Abstraction(Box<AbstractionSyntax>),
    Tanru(Box<TanruUnitVec>),
}

pub type TanruUnitVec = Vec1<TanruUnitSyntax>;

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
pub struct AdHocModalSyntax {
    pub nahe: Option<Token>,
    pub fiho: WithFreeModifiers<Token>,
    pub selbri: Box<SelbriSyntax>,
    pub fehu: Option<WithFreeModifiers<Token>>,
}

#[invariant(true)]
#[invariant(::Cmavo(word) => is_valid_tense_modal_word(word) || word.is_one_of_selmaho(&[Selmaho::Na, Selmaho::Ja, Selmaho::Joi, Selmaho::Bihi, Selmaho::Gaho]))]
#[invariant(::AdHocModal(fiho) => fiho.nahe.is_absent_or_selmaho(Selmaho::Nahe) && fiho.fiho.is_cmavo(Cmavo::Fiho) && fiho.fehu.is_absent_or_cmavo(Cmavo::Fehu))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum CompositeTenseModalPartSyntax {
    Cmavo(Token),
    AdHocModal(Box<AdHocModalSyntax>),
}

#[invariant(true)]
#[invariant(::Composite => !parts.value.is_empty())]
#[invariant(::TimeDirection(pu) => pu.is_selmaho(Selmaho::Pu))]
#[invariant(::TimeDirectionDistance => pu.is_selmaho(Selmaho::Pu) && distance.is_selmaho(Selmaho::Zi))]
#[invariant(::TimeInterval(interval) => interval.is_selmaho(Selmaho::Zeha))]
#[invariant(::TimeDirectionActuality => pu.is_selmaho(Selmaho::Pu) && caha.is_selmaho(Selmaho::Caha))]
#[invariant(::SpaceDistance(distance) => distance.is_selmaho(Selmaho::Va))]
#[invariant(::SpaceDirection(direction) => direction.is_selmaho(Selmaho::Faha))]
#[invariant(::SpaceMovement => mohi.is_cmavo(Cmavo::Mohi) && direction.is_selmaho(Selmaho::Faha) && distance.is_absent_or_selmaho(Selmaho::Va))]
#[invariant(::Modal => nahe.as_ref().is_none_or(|nahe| nahe.is_selmaho(Selmaho::Nahe)) && se.as_ref().is_none_or(|se| se.is_selmaho(Selmaho::Se)) && bai.is_selmaho(Selmaho::Bai) && nai.is_absent_or_cmavo(Cmavo::Nai) && ki.is_absent_or_cmavo(Cmavo::Ki))]
#[invariant(::Sticky(ki) => ki.is_cmavo(Cmavo::Ki))]
#[invariant(::AdHocModal => fiho.is_cmavo(Cmavo::Fiho) && fehu.is_absent_or_cmavo(Cmavo::Fehu))]
#[invariant(::Actuality(caha) => caha.is_selmaho(Selmaho::Caha))]
#[invariant(::EventContour(zaho) => zaho.value.iter().all(|word| word.is_selmaho(Selmaho::Zaho)))]
#[invariant(::IntervalProperty => number.as_ref().is_none_or(is_word_run_number_or_letter) && roi_or_tahe.is_one_of_selmaho(&[Selmaho::Roi, Selmaho::Tahe]) && nai.is_absent_or_cmavo(Cmavo::Nai))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum TenseModalSyntax {
    Composite {
        #[tree_child(primary)]
        parts: WithFreeModifiers<Vec<CompositeTenseModalPartSyntax>>,
    },
    TimeDirection(WithFreeModifiers<Token>),
    TimeDirectionDistance {
        pu: Token,
        distance: WithFreeModifiers<Token>,
    },
    TimeInterval(WithFreeModifiers<Token>),
    TimeDirectionActuality {
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
    Modal {
        nahe: Option<WithFreeModifiers<Token>>,
        se: Option<WithFreeModifiers<Token>>,
        bai: WithFreeModifiers<Token>,
        nai: Option<WithFreeModifiers<Token>>,
        ki: Option<WithFreeModifiers<Token>>,
    },
    Sticky(WithFreeModifiers<Token>),
    AdHocModal {
        fiho: WithFreeModifiers<Token>,
        #[tree_child(primary)]
        selbri: Box<SelbriSyntax>,
        fehu: Option<WithFreeModifiers<Token>>,
    },
    Actuality(WithFreeModifiers<Token>),
    EventContour(WithFreeModifiers<Vec<Token>>),
    IntervalProperty {
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
    pub abstractor_connections: Vec<AbstractorConnectionSyntax>,
    #[tree_child(primary)]
    pub subbridi: Box<SubbridiSyntax>,
    pub kei: Option<WithFreeModifiers<Token>>,
}

#[invariant(nu.is_selmaho(Selmaho::Nu))]
#[invariant(nai.is_absent_or_cmavo(Cmavo::Nai))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AbstractorConnectionSyntax {
    pub connective: ConnectiveSyntax,
    pub nu: WithFreeModifiers<Token>,
    pub nai: Option<WithFreeModifiers<Token>>,
}

#[invariant(true)]
#[invariant(::TanruUnitWord(word) => crate::grammar::tokens::is_relation_word(&word.value) || crate::grammar::tokens::is_cmevla_word(&word.value))]
#[invariant(::ProBridi => goha.is_selmaho(Selmaho::Goha) && raho.is_absent_or_cmavo(Cmavo::Raho))]
#[invariant(::ConvertedTanruUnit => se.is_selmaho(Selmaho::Se))]
#[invariant(::GroupedTanruUnit => ke.is_cmavo(Cmavo::Ke) && kehe.is_absent_or_cmavo(Cmavo::Kehe))]
#[invariant(::ScalarNegatedTanruUnit => nahe.is_selmaho(Selmaho::Nahe))]
#[invariant(::BoundTanruUnitConnection => bo.is_cmavo(Cmavo::Bo))]
#[invariant(::TanruUnitConnection => true)]
#[invariant(::RelativeClauses => !selbri_relative_clauses.is_empty())]
#[invariant(::SelbriGroupTanruUnit(..) => true)]
#[invariant(::ModalConversion => jai.is_cmavo(Cmavo::Jai))]
#[invariant(::LinkedSumtiTanruUnit => be.is_cmavo(Cmavo::Be) && fa.is_absent_or_selmaho(Selmaho::Fa) && (fa.is_none() || first_sumti.is_some()) && beho.is_absent_or_cmavo(Cmavo::Beho))]
#[invariant(::PreposedLinkedSumtiTanruUnit => be.is_cmavo(Cmavo::Be) && fa.is_absent_or_selmaho(Selmaho::Fa) && (fa.is_none() || first_sumti.is_some()) && beho.is_absent_or_cmavo(Cmavo::Beho))]
#[invariant(::Abstraction(abstraction) => abstraction.nu.is_selmaho(Selmaho::Nu) && abstraction.nai.is_absent_or_cmavo(Cmavo::Nai) && abstraction.abstractor_connections.iter().all(|connected_abstractor| connected_abstractor.nu.is_selmaho(Selmaho::Nu) && connected_abstractor.nai.is_absent_or_cmavo(Cmavo::Nai)) && abstraction.kei.is_absent_or_cmavo(Cmavo::Kei))]
#[invariant(::SumtiSelbri => me.is_cmavo(Cmavo::Me) && mehu.is_absent_or_cmavo(Cmavo::Mehu) && moi_marker.is_absent_or_selmaho(Selmaho::Moi))]
#[invariant(::QuotedWordSelbri(mehoi) => mehoi.is_quote_marker_cmavo(Cmavo::Mehoi))]
#[invariant(::QuotedBridiSelbri(gohoi) => gohoi.is_quote_marker_cmavo(Cmavo::Gohoi))]
#[invariant(::QuotedTextSelbri(muhoi) => muhoi.is_quote_marker_cmavo(Cmavo::Muhoi))]
#[invariant(::TextSelbri => luhei.is_cmavo(Cmavo::Luhei) && liau.is_absent_or_cmavo(Cmavo::Lihau))]
#[invariant(::OrdinalSelbri => is_word_run_number_or_letter(number) && moi.is_selmaho(Selmaho::Moi))]
#[invariant(::OperatorSelbri => nuha.is_cmavo(Cmavo::Nuha))]
#[invariant(::TagSelbri => xohi.is_cmavo(Cmavo::Xohi))]
#[invariant(::AssignedProBridi => !assignments.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum TanruUnitSyntax {
    TanruUnitWord(WithFreeModifiers<Token>),
    ProBridi {
        goha: WithFreeModifiers<Token>,
        raho: Option<WithFreeModifiers<Token>>,
    },
    ConvertedTanruUnit {
        se: WithFreeModifiers<Token>,
        #[tree_child(primary)]
        inner_unit: Box<TanruUnitSyntax>,
    },
    GroupedTanruUnit {
        ke_tense_modal: Option<Box<TenseModalSyntax>>,
        ke: WithFreeModifiers<Token>,
        #[tree_child(primary)]
        selbri: Box<SelbriSyntax>,
        kehe: Option<WithFreeModifiers<Token>>,
    },
    ScalarNegatedTanruUnit {
        nahe: WithFreeModifiers<Token>,
        #[tree_child(primary)]
        inner_unit: Box<TanruUnitSyntax>,
    },
    BoundTanruUnitConnection {
        leading_unit: Box<TanruUnitSyntax>,
        bo_connective: Option<Box<ConnectiveSyntax>>,
        bo_tense_modal: Option<Box<TenseModalSyntax>>,
        bo: WithFreeModifiers<Token>,
        trailing_unit: Box<TanruUnitSyntax>,
    },
    TanruUnitConnection {
        leading_unit: Box<TanruUnitSyntax>,
        connective: ConnectiveSyntax,
        trailing_unit: Box<TanruUnitSyntax>,
    },
    RelativeClauses {
        #[tree_child(primary)]
        base: Box<TanruUnitSyntax>,
        selbri_relative_clauses: Vec<SelbriRelativePhraseSyntax>,
    },
    SelbriGroupTanruUnit(Box<SelbriSyntax>),
    ModalConversion {
        jai: WithFreeModifiers<Token>,
        tense_modal: Option<Box<TenseModalSyntax>>,
        #[tree_child(primary)]
        inner_unit: Box<TanruUnitSyntax>,
    },
    LinkedSumtiTanruUnit {
        #[tree_child(primary)]
        base: Box<TanruUnitSyntax>,
        be: WithFreeModifiers<Token>,
        fa: Option<WithFreeModifiers<Token>>,
        first_sumti: Option<Box<SumtiSyntax>>,
        bei_links: Vec<AdditionalLinkedSumtiSyntax>,
        beho: Option<WithFreeModifiers<Token>>,
    },
    PreposedLinkedSumtiTanruUnit {
        be: WithFreeModifiers<Token>,
        fa: Option<WithFreeModifiers<Token>>,
        first_sumti: Option<Box<SumtiSyntax>>,
        bei_links: Vec<AdditionalLinkedSumtiSyntax>,
        beho: Option<WithFreeModifiers<Token>>,
        #[tree_child(primary)]
        base: Box<TanruUnitSyntax>,
    },
    Abstraction(Box<AbstractionSyntax>),
    SumtiSelbri {
        me: WithFreeModifiers<Token>,
        #[tree_child(primary)]
        sumti: Box<SumtiSyntax>,
        mehu: Option<WithFreeModifiers<Token>>,
        moi_marker: Option<WithFreeModifiers<Token>>,
    },
    QuotedWordSelbri(WithFreeModifiers<Token>),
    QuotedBridiSelbri(WithFreeModifiers<Token>),
    QuotedTextSelbri(WithFreeModifiers<Token>),
    TextSelbri {
        luhei: WithFreeModifiers<Token>,
        #[tree_child(primary)]
        text: Box<TextSyntax>,
        liau: Option<WithFreeModifiers<Token>>,
    },
    OrdinalSelbri {
        number: WordRun,
        moi: WithFreeModifiers<Token>,
    },
    OperatorSelbri {
        nuha: WithFreeModifiers<Token>,
        #[tree_child(primary)]
        mekso_operator: Box<MeksoOperatorSyntax>,
    },
    TagSelbri {
        xohi: WithFreeModifiers<Token>,
        #[tree_child(primary)]
        tag: Box<TenseModalSyntax>,
    },
    AssignedProBridi {
        #[tree_child(primary)]
        base: Box<TanruUnitSyntax>,
        assignments: Vec<ProBridiAssignmentSyntax>,
    },
}

#[invariant(cei.is_cmavo(Cmavo::Cei))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProBridiAssignmentSyntax {
    pub cei: WithFreeModifiers<Token>,
    #[tree_child(primary)]
    pub tanru_unit: Box<TanruUnitSyntax>,
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
    nai: &Option<Arc<WithFreeModifiers<Token>>>,
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
