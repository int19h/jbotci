//! Source-backed syntax AST model and generated tree traversal.

// The syntax AST mirrors the source grammar and v0 constructors.
// Boxing only for enum-size symmetry would obscure that shape during the port.
#![allow(clippy::large_enum_variant)]

use std::{fmt, sync::Arc};

#[allow(unused_imports)]
use bityzba::{contract_trait, data, ensures, invariant, new, requires};
use jbotci_morphology::{Cmavo, Selmaho, Word, WordLike};
use jbotci_tree::FieldRef;
use serde::ser::{SerializeSeq, Serializer};
use serde::{Deserialize, Serialize};

#[invariant(::Missing => span.char_len() == 0
    && !expected.is_empty()
    && !diagnostic_code.is_empty())]
#[invariant(::Invalid => span.char_len() > 0
    && !text.is_empty()
    && !expected.is_empty()
    && !diagnostic_code.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum RecoveryTreeItem {
    Missing {
        span: Arc<jbotci_source::SourceSpan>,
        expected: Vec<String>,
        diagnostic_code: String,
    },
    Invalid {
        span: Arc<jbotci_source::SourceSpan>,
        text: String,
        expected: Vec<String>,
        diagnostic_code: String,
    },
}

#[contract_trait]
impl jbotci_tree::RecoveryItemState for RecoveryTreeItem {
    #[requires(true)]
    #[ensures(true)]
    fn recovery_item_kind(&self) -> jbotci_tree::RecoveryItemKind {
        match self.as_data() {
            data!(RecoveryTreeItem::Missing { .. }) => jbotci_tree::RecoveryItemKind::Missing,
            data!(RecoveryTreeItem::Invalid { .. }) => jbotci_tree::RecoveryItemKind::Invalid,
        }
    }
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WithFreeModifiers<T, F> {
    pub value: T,
    pub free_modifiers: Vec<F>,
}

impl<T, F> WithFreeModifiers<T, F> {
    #[requires(true)]
    #[ensures(true)]
    pub fn new(value: T, free_modifiers: Vec<F>) -> Self {
        Self {
            value,
            free_modifiers,
        }
    }
}

impl<T: Serialize, F: Serialize> Serialize for WithFreeModifiers<T, F> {
    #[requires(true)]
    #[ensures(true)]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if self.free_modifiers.is_empty() {
            return self.value.serialize(serializer);
        }
        let mut seq = serializer.serialize_seq(Some(1 + self.free_modifiers.len()))?;
        seq.serialize_element(&self.value)?;
        for free_modifier in &self.free_modifiers {
            seq.serialize_element(free_modifier)?;
        }
        seq.end()
    }
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
        extra_bahe: Vec<Word>,
        word_like: T,
    },
    WithIndicator {
        base: Arc<WithIndicators<T>>,
        indicator_bahe: Vec<Word>,
        indicator: Word,
        nai_bahe: Vec<Word>,
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
        WithIndicators::Emphasized {
            bahe,
            extra_bahe: Vec::new(),
            word_like,
        }
    }

    #[requires(bahe.is_selmaho(Selmaho::Bahe))]
    #[requires(extra_bahe.iter().all(|bahe| bahe.is_selmaho(Selmaho::Bahe)))]
    #[ensures(true)]
    pub fn emphasized_with_extra_bahe(bahe: Word, extra_bahe: Vec<Word>, word_like: T) -> Self {
        WithIndicators::Emphasized {
            bahe,
            extra_bahe,
            word_like,
        }
    }

    #[requires(crate::is_indicator_word(&indicator))]
    #[requires(nai.as_ref().is_none_or(|nai| nai.is_cmavo(Cmavo::Nai)))]
    #[ensures(true)]
    pub fn with_indicator(base: WithIndicators<T>, indicator: Word, nai: Option<Word>) -> Self {
        Self::with_indicator_with_modifiers(base, Vec::new(), indicator, Vec::new(), nai)
    }

    #[requires(indicator_bahe.iter().all(|bahe| bahe.is_one_of_cmavo(&[Cmavo::Bahe, Cmavo::Zahe])))]
    #[requires(crate::is_indicator_word(&indicator))]
    #[requires(nai_bahe.iter().all(|bahe| bahe.is_one_of_cmavo(&[Cmavo::Bahe, Cmavo::Zahe])))]
    #[requires(nai.as_ref().is_none_or(|nai| nai.is_cmavo(Cmavo::Nai)))]
    #[ensures(true)]
    pub fn with_indicator_with_modifiers(
        base: WithIndicators<T>,
        indicator_bahe: Vec<Word>,
        indicator: Word,
        nai_bahe: Vec<Word>,
        nai: Option<Word>,
    ) -> Self {
        WithIndicators::WithIndicator {
            base: Arc::new(base),
            indicator_bahe,
            indicator,
            nai_bahe,
            nai,
        }
    }
}

impl<T: Clone> WithIndicators<T> {
    #[requires(bahe.is_selmaho(Selmaho::Bahe))]
    #[ensures(true)]
    pub fn with_prepended_bahe(&self, bahe: Word) -> Self {
        match self {
            WithIndicators::Plain(word_like) => Self::emphasized(bahe, word_like.clone()),
            WithIndicators::Emphasized {
                bahe: first_bahe,
                extra_bahe,
                word_like,
            } => {
                let mut new_extra = Vec::with_capacity(extra_bahe.len() + 1);
                new_extra.push(first_bahe.clone());
                new_extra.extend(extra_bahe.iter().cloned());
                Self::emphasized_with_extra_bahe(bahe, new_extra, word_like.clone())
            }
            WithIndicators::WithIndicator {
                base,
                indicator_bahe,
                indicator,
                nai_bahe,
                nai,
            } => WithIndicators::WithIndicator {
                base: Arc::new(base.with_prepended_bahe(bahe)),
                indicator_bahe: indicator_bahe.clone(),
                indicator: indicator.clone(),
                nai_bahe: nai_bahe.clone(),
                nai: nai.clone(),
            },
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

    #[requires(bahe.is_selmaho(Selmaho::Bahe))]
    #[ensures(true)]
    pub fn with_prepended_bahe(&self, bahe: Word) -> Self {
        Self::from_indicators(self.as_indicators().with_prepended_bahe(bahe))
    }

    #[requires(crate::is_indicator_word(&indicator))]
    #[requires(nai.as_ref().is_none_or(|nai| nai.is_cmavo(Cmavo::Nai)))]
    #[ensures(true)]
    pub fn with_indicator(base: Token, indicator: Word, nai: Option<Word>) -> Self {
        Self::with_indicator_with_modifiers(base, Vec::new(), indicator, Vec::new(), nai)
    }

    #[requires(indicator_bahe.iter().all(|bahe| bahe.is_one_of_cmavo(&[Cmavo::Bahe, Cmavo::Zahe])))]
    #[requires(crate::is_indicator_word(&indicator))]
    #[requires(nai_bahe.iter().all(|bahe| bahe.is_one_of_cmavo(&[Cmavo::Bahe, Cmavo::Zahe])))]
    #[requires(nai.as_ref().is_none_or(|nai| nai.is_cmavo(Cmavo::Nai)))]
    #[ensures(true)]
    pub fn with_indicator_with_modifiers(
        base: Token,
        indicator_bahe: Vec<Word>,
        indicator: Word,
        nai_bahe: Vec<Word>,
        nai: Option<Word>,
    ) -> Self {
        new!(Token(Arc::new(WithIndicators::WithIndicator {
            base: Arc::clone(base.as_data()),
            indicator_bahe,
            indicator,
            nai_bahe,
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
            WithIndicators::Emphasized {
                bahe,
                extra_bahe,
                word_like,
            } => {
                out.push(bahe.span());
                for bahe in extra_bahe {
                    out.push(bahe.span());
                }
                word_like.source_spans_into(out);
            }
            WithIndicators::WithIndicator {
                base,
                indicator_bahe,
                indicator,
                nai_bahe,
                nai,
            } => {
                base.source_spans_into(out);
                for bahe in indicator_bahe {
                    out.push(bahe.span());
                }
                out.push(indicator.span());
                for bahe in nai_bahe {
                    out.push(bahe.span());
                }
                if let Some(nai) = nai {
                    out.push(nai.span());
                }
            }
        }
    }
}

impl<F> WithFreeModifiers<Token, F> {
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

impl<T: fmt::Display> fmt::Display for WithIndicators<T> {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WithIndicators::Plain(word_like) => write!(f, "{word_like}"),
            WithIndicators::Emphasized {
                bahe,
                extra_bahe,
                word_like,
            } => {
                write!(f, "{bahe}")?;
                for bahe in extra_bahe {
                    write!(f, "-{bahe}")?;
                }
                write!(f, "-{word_like}")
            }
            WithIndicators::WithIndicator {
                base,
                indicator_bahe,
                indicator,
                nai_bahe,
                nai,
            } => {
                write!(f, "{base}")?;
                for bahe in indicator_bahe {
                    write!(f, "-{bahe}")?;
                }
                write!(f, "-{indicator}")?;
                if let Some(nai) = nai {
                    for bahe in nai_bahe {
                        write!(f, "-{bahe}")?;
                    }
                    write!(f, "-{nai}")?;
                }
                Ok(())
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
pub fn elidable_terminator_for_absent_field<Node>(_node: Node, field: FieldRef) -> Option<Cmavo> {
    elidable_terminator_for_absent_field_ref(field)
}

#[requires(true)]
#[ensures(true)]
pub fn elidable_terminator_for_absent_field_ref(field: FieldRef) -> Option<Cmavo> {
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
