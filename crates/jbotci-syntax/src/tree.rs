//! Source-backed syntax AST model and generated tree traversal.

// The syntax AST mirrors the source grammar and v0 constructors.
// Boxing only for enum-size symmetry would obscure that shape during the port.
#![allow(clippy::large_enum_variant)]

use std::{
    fmt,
    hash::{Hash, Hasher},
    sync::Arc,
};

#[allow(unused_imports)]
use bityzba::{contract_trait, data, ensures, invariant, new, requires};
use jbotci_morphology::{Cmavo, Selmaho, Word, WordLike};
use jbotci_source::SourceSpan;
use jbotci_tree::FieldRef;
use serde::ser::{SerializeSeq, Serializer};
use serde::{Deserialize, Serialize};
use vec1::Vec1;

#[invariant(::SkippedTokens => syntax_recovery_tokens_have_ordered_source_attribution(tokens))]
#[invariant(::MissingRequiredField => span.is_empty() && !expected.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum SyntaxRecoveryItem {
    /// Tokens skipped while recovering, in parser-stream order.
    ///
    /// Adjacent plain tokens may have exactly the same source span because a
    /// dialect expansion maps every replacement back to its one source word.
    /// This is ordered attribution to one source region, not an overlap: only
    /// an identical single-span attribution is admitted by the invariant.
    SkippedTokens {
        error_index: usize,
        tokens: Vec1<Token>,
    },
    MissingRequiredField {
        error_index: usize,
        span: Arc<jbotci_source::SourceSpan>,
        expected: String,
    },
}

#[contract_trait]
impl jbotci_tree::RecoveryItemState for SyntaxRecoveryItem {
    #[requires(true)]
    #[ensures(true)]
    fn recovery_item_kind(&self) -> jbotci_tree::RecoveryItemKind {
        match self.as_data() {
            data!(SyntaxRecoveryItem::SkippedTokens { .. }) => {
                jbotci_tree::RecoveryItemKind::Invalid
            }
            data!(SyntaxRecoveryItem::MissingRequiredField { .. }) => {
                jbotci_tree::RecoveryItemKind::Missing
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_source_spans(&self, visitor: &mut dyn FnMut(&jbotci_source::SourceSpan)) {
        match self.as_data() {
            data!(SyntaxRecoveryItem::SkippedTokens { tokens, .. }) => {
                for token in tokens {
                    for span in token.source_spans() {
                        visitor(span);
                    }
                }
            }
            data!(SyntaxRecoveryItem::MissingRequiredField { span, .. }) => visitor(span),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn recovery_error_index(&self) -> Option<usize> {
        match self.as_data() {
            data!(SyntaxRecoveryItem::SkippedTokens { error_index, .. }) => Some(*error_index),
            data!(SyntaxRecoveryItem::MissingRequiredField { error_index, .. }) => {
                Some(*error_index)
            }
        }
    }
}

impl SyntaxRecoveryItem {
    /// Return the syntax tokens retained by a skipped-token recovery item.
    #[requires(true)]
    #[ensures(ret.is_some() == matches!(self.as_data(), data!(SyntaxRecoveryItem::SkippedTokens { .. })))]
    pub fn skipped_tokens(&self) -> Option<&[Token]> {
        match self.as_data() {
            data!(SyntaxRecoveryItem::SkippedTokens { tokens, .. }) => Some(tokens.as_slice()),
            data!(SyntaxRecoveryItem::MissingRequiredField { .. }) => None,
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn syntax_recovery_tokens_have_ordered_source_attribution(tokens: &Vec1<Token>) -> bool {
    let mut order = TokenSourceAttributionOrder::new();
    for token in tokens {
        order.observe_token(token);
    }
    order.is_ordered()
}

/// Incremental checker for parser-stream source attribution.
///
/// Spans within one token must never overlap. Across token boundaries, the
/// only admitted overlap is an adjacent pair of single-span tokens with the
/// exact same [`SourceSpan`], which is how dialect expansion siblings retain
/// attribution to their shared source word.
#[invariant(::Empty => true)]
#[invariant(::Ordered { previous_byte_end, previous_single_span } =>
    previous_single_span.is_none_or(|span| span.byte_end == *previous_byte_end))]
#[invariant(::Invalid => true)]
#[derive(Debug)]
pub(crate) enum TokenSourceAttributionOrder<'tokens> {
    Empty,
    Ordered {
        previous_byte_end: usize,
        previous_single_span: Option<&'tokens SourceSpan>,
    },
    Invalid,
}

impl<'tokens> TokenSourceAttributionOrder<'tokens> {
    #[requires(true)]
    #[ensures(matches!(ret.as_data(), data!(TokenSourceAttributionOrder::Empty)))]
    pub(crate) fn new() -> Self {
        new!(TokenSourceAttributionOrder::Empty)
    }

    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn observe_token(&mut self, token: &'tokens Token) {
        let (previous_byte_end, previous_single_span) = match self.as_data() {
            data!(TokenSourceAttributionOrder::Empty) => (None, None),
            data!(TokenSourceAttributionOrder::Ordered {
                previous_byte_end,
                previous_single_span,
            }) => (Some(*previous_byte_end), *previous_single_span),
            data!(TokenSourceAttributionOrder::Invalid) => return,
        };

        let spans = token.source_spans();
        let Some(first) = spans.first() else {
            *self = new!(TokenSourceAttributionOrder::Invalid);
            return;
        };
        let repeats_previous_source_word = match spans.as_slice() {
            [span] => previous_single_span == Some(*span),
            _ => false,
        };

        let mut token_previous_byte_end = None;
        for span in &spans {
            if token_previous_byte_end.is_some_and(|byte_end| span.byte_start < byte_end) {
                *self = new!(TokenSourceAttributionOrder::Invalid);
                return;
            }
            token_previous_byte_end = Some(span.byte_end);
        }
        if !repeats_previous_source_word
            && previous_byte_end.is_some_and(|byte_end| first.byte_start < byte_end)
        {
            *self = new!(TokenSourceAttributionOrder::Invalid);
            return;
        }

        let previous_single_span = match spans.as_slice() {
            [span] => Some(*span),
            _ => None,
        };
        *self = new!(TokenSourceAttributionOrder::Ordered {
            previous_byte_end: token_previous_byte_end.expect("non-empty token spans"),
            previous_single_span,
        });
    }

    #[requires(true)]
    #[ensures(ret == matches!(self.as_data(), data!(TokenSourceAttributionOrder::Ordered { .. })))]
    pub(crate) fn is_ordered(&self) -> bool {
        matches!(
            self.as_data(),
            data!(TokenSourceAttributionOrder::Ordered { .. })
        )
    }
}

#[cfg(test)]
mod source_attribution_tests {
    use jbotci_morphology::{Phonemes, WordKind};
    use jbotci_source::SourceId;

    use super::*;

    #[requires(!phonemes.is_empty())]
    #[requires(byte_start < byte_end)]
    #[ensures(ret.source_spans().len() == 1)]
    fn token_with_span(
        phonemes: &str,
        source_id: Option<&str>,
        byte_start: usize,
        byte_end: usize,
    ) -> Token {
        let span = SourceSpan::new(
            source_id.map(|source_id| SourceId(source_id.to_owned())),
            byte_start,
            byte_end,
            byte_start,
            byte_end,
        )
        .expect("ordered test span");
        let phonemes = Phonemes::from_canonical(phonemes.to_owned()).expect("canonical phonemes");
        Token::bare(WordLike::bare(Word::from_kind(
            WordKind::Cmavo,
            phonemes,
            span,
        )))
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn exact_adjacent_single_span_attribution_is_ordered() {
        let tokens = Vec1::try_from_vec(vec![
            token_with_span("lo", None, 2, 4),
            token_with_span("su'u", None, 2, 4),
            token_with_span("do", None, 5, 7),
        ])
        .expect("non-empty tokens");

        assert!(syntax_recovery_tokens_have_ordered_source_attribution(
            &tokens
        ));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn nonidentical_overlapping_attribution_is_rejected() {
        let tokens = Vec1::try_from_vec(vec![
            token_with_span("lo", None, 2, 5),
            token_with_span("do", None, 4, 6),
        ])
        .expect("non-empty tokens");

        assert!(!syntax_recovery_tokens_have_ordered_source_attribution(
            &tokens
        ));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn matching_offsets_from_different_sources_are_rejected() {
        let tokens = Vec1::try_from_vec(vec![
            token_with_span("lo", Some("left"), 2, 4),
            token_with_span("do", Some("right"), 2, 4),
        ])
        .expect("non-empty tokens");

        assert!(!syntax_recovery_tokens_have_ordered_source_attribution(
            &tokens
        ));
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

#[invariant(::Plain(_) => true)]
#[invariant(::Emphasized { bahe, extra_bahe, .. } =>
    is_bahe_modifier_word(bahe)
        && extra_bahe.iter().all(is_bahe_modifier_word))]
#[invariant(::WithIndicator {
    indicator_bahe,
    indicator,
    nai_bahe,
    nai,
    ..
} =>
    indicator_bahe.iter().all(is_bahe_modifier_word)
        && crate::is_indicator_word(indicator)
        && nai_bahe.iter().all(is_bahe_modifier_word)
        && nai.as_ref().is_none_or(|nai| nai.is_cmavo(Cmavo::Nai)))]
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
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
        new!(WithIndicators::Plain(word_like))
    }

    #[requires(is_bahe_modifier_word(&bahe))]
    #[ensures(true)]
    pub fn emphasized(bahe: Word, word_like: T) -> Self {
        new!(WithIndicators::Emphasized {
            bahe,
            extra_bahe: Vec::new(),
            word_like,
        })
    }

    #[requires(is_bahe_modifier_word(&bahe))]
    #[requires(extra_bahe.iter().all(is_bahe_modifier_word))]
    #[ensures(true)]
    pub fn emphasized_with_extra_bahe(bahe: Word, extra_bahe: Vec<Word>, word_like: T) -> Self {
        new!(WithIndicators::Emphasized {
            bahe,
            extra_bahe,
            word_like,
        })
    }

    #[requires(crate::is_indicator_word(&indicator))]
    #[requires(nai.as_ref().is_none_or(|nai| nai.is_cmavo(Cmavo::Nai)))]
    #[ensures(true)]
    pub fn with_indicator(base: WithIndicators<T>, indicator: Word, nai: Option<Word>) -> Self {
        Self::with_indicator_with_modifiers(base, Vec::new(), indicator, Vec::new(), nai)
    }

    #[requires(indicator_bahe.iter().all(is_bahe_modifier_word))]
    #[requires(crate::is_indicator_word(&indicator))]
    #[requires(nai_bahe.iter().all(is_bahe_modifier_word))]
    #[requires(nai.as_ref().is_none_or(|nai| nai.is_cmavo(Cmavo::Nai)))]
    #[ensures(true)]
    pub fn with_indicator_with_modifiers(
        base: WithIndicators<T>,
        indicator_bahe: Vec<Word>,
        indicator: Word,
        nai_bahe: Vec<Word>,
        nai: Option<Word>,
    ) -> Self {
        new!(WithIndicators::WithIndicator {
            base: Arc::new(base),
            indicator_bahe,
            indicator,
            nai_bahe,
            nai,
        })
    }
}

impl<T: fmt::Debug> fmt::Debug for WithIndicators<T> {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.as_data() {
            data!(WithIndicators::Plain(word_like)) => {
                formatter.debug_tuple("Plain").field(word_like).finish()
            }
            data!(WithIndicators::Emphasized {
                bahe,
                extra_bahe,
                word_like,
            }) => formatter
                .debug_struct("Emphasized")
                .field("bahe", bahe)
                .field("extra_bahe", extra_bahe)
                .field("word_like", word_like)
                .finish(),
            data!(WithIndicators::WithIndicator {
                base,
                indicator_bahe,
                indicator,
                nai_bahe,
                nai,
            }) => formatter
                .debug_struct("WithIndicator")
                .field("base", base)
                .field("indicator_bahe", indicator_bahe)
                .field("indicator", indicator)
                .field("nai_bahe", nai_bahe)
                .field("nai", nai)
                .finish(),
        }
    }
}

impl<T: Clone> WithIndicators<T> {
    #[requires(is_bahe_modifier_word(&bahe))]
    #[ensures(true)]
    pub fn with_prepended_bahe(&self, bahe: Word) -> Self {
        match self.as_data() {
            data!(WithIndicators::Plain(word_like)) => Self::emphasized(bahe, word_like.clone()),
            data!(WithIndicators::Emphasized {
                bahe: first_bahe,
                extra_bahe,
                word_like,
            }) => {
                let mut new_extra = Vec::with_capacity(extra_bahe.len() + 1);
                new_extra.push(first_bahe.clone());
                new_extra.extend(extra_bahe.iter().cloned());
                Self::emphasized_with_extra_bahe(bahe, new_extra, word_like.clone())
            }
            data!(WithIndicators::WithIndicator {
                base,
                indicator_bahe,
                indicator,
                nai_bahe,
                nai,
            }) => new!(WithIndicators::WithIndicator {
                base: Arc::new(base.with_prepended_bahe(bahe)),
                indicator_bahe: indicator_bahe.clone(),
                indicator: indicator.clone(),
                nai_bahe: nai_bahe.clone(),
                nai: nai.clone(),
            }),
        }
    }
}

#[requires(true)]
#[ensures(ret == word.is_one_of_cmavo(&[Cmavo::Bahe, Cmavo::Zahe]))]
fn is_bahe_modifier_word(word: &Word) -> bool {
    word.is_one_of_cmavo(&[Cmavo::Bahe, Cmavo::Zahe])
}

#[invariant(self.core_word().byte_range().is_some(), "syntax tokens must cover source bytes")]
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Token(Arc<WithIndicators<WordLike>>);

/// Parser-local identity for a token allocation.
///
/// This owns an `Arc` so its pointer cannot be reused while it is a cache key.
/// Equality and hashing deliberately use allocation identity, not the token's
/// structural contents or source attribution.
#[invariant(true)]
#[derive(Clone)]
pub(crate) struct TokenIdentity(Arc<WithIndicators<WordLike>>);

impl PartialEq for TokenIdentity {
    #[requires(true)]
    #[ensures(ret == Arc::ptr_eq(&self.0, &other.0))]
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for TokenIdentity {}

impl Hash for TokenIdentity {
    #[requires(true)]
    #[ensures(true)]
    fn hash<H: Hasher>(&self, state: &mut H) {
        Arc::as_ptr(&self.0).hash(state);
    }
}

impl fmt::Debug for TokenIdentity {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("TokenIdentity")
            .field(&Arc::as_ptr(&self.0))
            .finish()
    }
}

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

    #[requires(is_bahe_modifier_word(&bahe))]
    #[ensures(true)]
    pub fn emphasized(bahe: Word, word_like: WordLike) -> Self {
        Self::from_indicators(WithIndicators::emphasized(bahe, word_like))
    }

    #[requires(is_bahe_modifier_word(&bahe))]
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

    #[requires(indicator_bahe.iter().all(is_bahe_modifier_word))]
    #[requires(crate::is_indicator_word(&indicator))]
    #[requires(nai_bahe.iter().all(is_bahe_modifier_word))]
    #[requires(nai.as_ref().is_none_or(|nai| nai.is_cmavo(Cmavo::Nai)))]
    #[ensures(true)]
    pub fn with_indicator_with_modifiers(
        base: Token,
        indicator_bahe: Vec<Word>,
        indicator: Word,
        nai_bahe: Vec<Word>,
        nai: Option<Word>,
    ) -> Self {
        let indicators = new!(WithIndicators::WithIndicator {
            base: Arc::clone(base.as_data()),
            indicator_bahe,
            indicator,
            nai_bahe,
            nai,
        });
        new!(Token(Arc::new(indicators)))
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

    /// Return the stable identity of this token's shared backing allocation.
    ///
    /// Source ranges are attribution rather than identity: one dialect source
    /// word can expand into several distinct tokens with the same range. Token
    /// clones retain this identity through their shared `Arc`, so parser-local
    /// caches can distinguish expansion siblings without retaining extra data.
    #[requires(true)]
    #[ensures(Arc::ptr_eq(&ret.0, self.as_data()))]
    pub(crate) fn identity(&self) -> TokenIdentity {
        TokenIdentity(Arc::clone(self.as_data()))
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
        match self.as_data() {
            data!(WithIndicators::Plain(word_like))
            | data!(WithIndicators::Emphasized { word_like, .. }) => word_like,
            data!(WithIndicators::WithIndicator { base, .. }) => base.core_word(),
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
        match self.as_data() {
            data!(WithIndicators::Plain(word_like)) => word_like.source_spans_into(out),
            data!(WithIndicators::Emphasized {
                bahe,
                extra_bahe,
                word_like,
            }) => {
                out.push(bahe.span());
                for bahe in extra_bahe {
                    out.push(bahe.span());
                }
                word_like.source_spans_into(out);
            }
            data!(WithIndicators::WithIndicator {
                base,
                indicator_bahe,
                indicator,
                nai_bahe,
                nai,
            }) => {
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
        match self.as_data() {
            data!(WithIndicators::Plain(word_like)) => write!(f, "{word_like}"),
            data!(WithIndicators::Emphasized {
                bahe,
                extra_bahe,
                word_like,
            }) => {
                write!(f, "{bahe}")?;
                for bahe in extra_bahe {
                    write!(f, "-{bahe}")?;
                }
                write!(f, "-{word_like}")
            }
            data!(WithIndicators::WithIndicator {
                base,
                indicator_bahe,
                indicator,
                nai_bahe,
                nai,
            }) => {
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
pub fn elidable_terminator_for_absent_field_ref(field: FieldRef) -> Option<Cmavo> {
    let name = field.name?;
    crate::generated_model::GENERATED_MODEL_ELIDABLE_TERMINATORS
        .iter()
        .find(|terminator| terminator.field == name)
        .map(|terminator| terminator.cmavo)
}
