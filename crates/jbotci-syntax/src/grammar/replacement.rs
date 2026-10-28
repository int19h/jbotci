//! Projection of one morphology quote into the replacement syntax fields.

use std::sync::Arc;

use bityzba::{data, new, requires};
use jbotci_morphology::{Cmavo, Word, WordLike};

use super::{
    BoxedParser,
    generated_model::{self as model, recovered},
    parser_core::{MapExtra, Parser},
    tokens::token_matching,
};
use crate::{ExperimentalConstruct, SyntaxExpectedToken, Token, WithIndicators};

/// Consume the opaque quote without exposing its body to syntax terminals.
#[requires(true)]
#[ensures(true)]
fn quote_token<'tokens>() -> BoxedParser<'tokens, Token> {
    token_matching(
        "replacement quote",
        "LOhAI/LEhAI quote",
        vec![
            new!(SyntaxExpectedToken::Cmavo(Cmavo::Lohai)),
            new!(SyntaxExpectedToken::Cmavo(Cmavo::Sahai)),
            new!(SyntaxExpectedToken::Cmavo(Cmavo::Lehai)),
        ],
        |token, _| {
            matches!(
                token.core_word().as_data(),
                data!(WordLike::ReplacementQuote { .. })
            )
        },
    )
    .map_with(|token, extra: &mut MapExtra<'tokens, '_>| {
        let data!(WordLike::ReplacementQuote { runs, lehai }) = token.core_word().as_data() else {
            unreachable!("the terminal requires a replacement quote")
        };
        if let Some(first) = runs.first() {
            extra.state().warn_word(
                ExperimentalConstruct::ExperimentalLohAiReplacementFree,
                &token,
                &first.marker,
            );
            if first.marker.is_cmavo(Cmavo::Lohai)
                && let Some(second) = runs.get(1)
                && second.marker.is_cmavo(Cmavo::Sahai)
            {
                extra.state().warn_word(
                    ExperimentalConstruct::ExperimentalLohAiReplacementFree,
                    &token,
                    &second.marker,
                );
            }
        }
        extra.state().warn_word(
            ExperimentalConstruct::ExperimentalLohAiReplacementFree,
            &token,
            lehai,
        );
        token
    })
    .boxed()
}

#[requires(true)]
#[ensures(true)]
pub(super) fn strict<'tokens>(
    free: BoxedParser<'tokens, model::FreeModifierSyntax>,
) -> BoxedParser<'tokens, model::TextReplacementFreeModifierSyntax> {
    quote_token()
        .then(super::generated_runtime::strict_free_modifier_list_parser(
            free,
        ))
        .map(|(token, free)| {
            let mut node = project(&token);
            let free = free.into_iter().map(Arc::new).collect();
            match &mut node {
                model::TextReplacementFreeModifierSyntax::FullTextReplacementFreeModifier(
                    inner,
                ) => Arc::make_mut(inner).lehai.free_modifiers = free,
                model::TextReplacementFreeModifierSyntax::NewOnlyTextReplacementFreeModifier(
                    inner,
                ) => Arc::make_mut(inner).lehai.free_modifiers = free,
                model::TextReplacementFreeModifierSyntax::CloseOnlyTextReplacementFreeModifier(
                    inner,
                ) => Arc::make_mut(inner).0.free_modifiers = free,
            }
            node
        })
        .boxed()
}

#[requires(true)]
#[ensures(true)]
pub(super) fn recovered_parser<'tokens>(
    free: BoxedParser<'tokens, recovered::FreeModifierSyntax>,
) -> BoxedParser<'tokens, recovered::TextReplacementFreeModifierSyntax> {
    quote_token().then(super::generated_runtime::recovered_free_modifier_list_parser(free))
        .map(|(token, free)| {
            let mut node = recovered::TextReplacementFreeModifierSyntax::from_valid(project(&token));
            let free = free.into_iter().map(Arc::new).collect();
            match &mut node {
                recovered::TextReplacementFreeModifierSyntax::FullTextReplacementFreeModifier(inner) => {
                    let jbotci_tree::Recovered::Valid(inner) = Arc::make_mut(inner) else { unreachable!("projection is valid") };
                    inner.lehai.free_modifiers = free;
                }
                recovered::TextReplacementFreeModifierSyntax::NewOnlyTextReplacementFreeModifier(inner) => {
                    let jbotci_tree::Recovered::Valid(inner) = Arc::make_mut(inner) else { unreachable!("projection is valid") };
                    inner.lehai.free_modifiers = free;
                }
                recovered::TextReplacementFreeModifierSyntax::CloseOnlyTextReplacementFreeModifier(inner) => {
                    let jbotci_tree::Recovered::Valid(inner) = Arc::make_mut(inner) else { unreachable!("projection is valid") };
                    inner.0.free_modifiers = free;
                }
            }
            node
        }).boxed()
}

/// Keep emphasis on the opening marker and indicators on the closing marker.
#[requires(first.is_selmaho(jbotci_morphology::Selmaho::Lohai) || first.is_cmavo(Cmavo::Lehai))]
#[requires(close.is_cmavo(Cmavo::Lehai))]
#[requires(bare_close == first.is_cmavo(Cmavo::Lehai))]
#[requires(token.quote_marker_cmavo() == first.cmavo())]
#[ensures(true)]
fn markers(token: &Token, first: &Word, close: &Word, bare_close: bool) -> Markers {
    let mut opening = Token::bare(WordLike::bare(first.clone()));
    let mut closing = Token::bare(WordLike::bare(close.clone()));
    let mut layers = Vec::new();
    let mut indicators = token.as_indicators();
    loop {
        match indicators.as_data() {
            data!(WithIndicators::Plain(_)) => break,
            data!(WithIndicators::Emphasized {
                bahe,
                extra_bahe,
                ..
            }) => {
                opening = Token::from_indicators(WithIndicators::emphasized_with_extra_bahe(
                    bahe.clone(),
                    extra_bahe.clone(),
                    WordLike::bare(first.clone()),
                ));
                break;
            }
            data!(WithIndicators::WithIndicator { base, .. }) => {
                layers.push(indicators);
                indicators = base;
            }
        }
    }
    if bare_close {
        closing = opening.clone();
    }
    for layer in layers.into_iter().rev() {
        let data!(WithIndicators::WithIndicator {
            indicator_bahe,
            indicator,
            nai_bahe,
            nai,
            ..
        }) = layer.as_data()
        else {
            unreachable!("indicator layer")
        };
        closing = Token::with_indicator_with_modifiers(
            closing,
            indicator_bahe.clone(),
            indicator.clone(),
            nai_bahe.clone(),
            nai.clone(),
        );
    }
    new!(Markers { opening, closing })
}

#[bityzba::invariant(opening.is_one_of_cmavo(&[Cmavo::Lohai, Cmavo::Sahai, Cmavo::Lehai]) && closing.is_cmavo(Cmavo::Lehai))]
struct Markers {
    opening: Token,
    closing: Token,
}

#[requires(matches!(token.core_word().as_data(), data!(WordLike::ReplacementQuote { .. })))]
#[ensures(match &ret {
    model::TextReplacementFreeModifierSyntax::FullTextReplacementFreeModifier(node) => node.lohai.is_cmavo(Cmavo::Lohai) && node.lehai.value.is_cmavo(Cmavo::Lehai),
    model::TextReplacementFreeModifierSyntax::NewOnlyTextReplacementFreeModifier(node) => node.sahai.is_cmavo(Cmavo::Sahai) && node.lehai.value.is_cmavo(Cmavo::Lehai),
    model::TextReplacementFreeModifierSyntax::CloseOnlyTextReplacementFreeModifier(node) => node.0.value.is_cmavo(Cmavo::Lehai),
})]
fn project(token: &Token) -> model::TextReplacementFreeModifierSyntax {
    let data!(WordLike::ReplacementQuote { runs, lehai }) = token.core_word().as_data() else {
        unreachable!("replacement quote terminal")
    };
    let first = runs.first();
    let markers = markers(
        token,
        first.map_or(lehai.as_ref(), |run| &run.marker),
        lehai,
        first.is_none(),
    )
    .into_data();
    let lehai = crate::WithFreeModifiers::new(markers.closing, Vec::new());
    let raw = |word: &Word| Token::bare(WordLike::bare(word.clone()));
    match first {
        None => model::TextReplacementFreeModifierSyntax::CloseOnlyTextReplacementFreeModifier(
            Arc::new(model::CloseOnlyTextReplacementFreeModifierSyntax(lehai)),
        ),
        Some(first) if first.marker.is_cmavo(Cmavo::Lohai) => {
            let mut old_words: Vec<_> = first
                .words
                .iter()
                .map(|word| Token::bare(word.word.clone()))
                .collect();
            let mut sahai = None;
            let mut new_words = Vec::new();
            if let Some(second) = runs.get(1) {
                if second.marker.is_cmavo(Cmavo::Sahai) {
                    sahai = Some(raw(&second.marker));
                    new_words.extend(
                        second
                            .words
                            .iter()
                            .map(|word| Token::bare(word.word.clone())),
                    );
                } else {
                    old_words.push(raw(&second.marker));
                    old_words.extend(
                        second
                            .words
                            .iter()
                            .map(|word| Token::bare(word.word.clone())),
                    );
                }
            }
            model::TextReplacementFreeModifierSyntax::FullTextReplacementFreeModifier(Arc::new(
                model::FullTextReplacementFreeModifierSyntax {
                    lohai: markers.opening,
                    old_words,
                    sahai,
                    new_words,
                    lehai,
                },
            ))
        }
        Some(first) => {
            let mut new_words: Vec<_> = first
                .words
                .iter()
                .map(|word| Token::bare(word.word.clone()))
                .collect();
            if let Some(second) = runs.get(1) {
                new_words.push(raw(&second.marker));
                new_words.extend(
                    second
                        .words
                        .iter()
                        .map(|word| Token::bare(word.word.clone())),
                );
            }
            model::TextReplacementFreeModifierSyntax::NewOnlyTextReplacementFreeModifier(Arc::new(
                model::NewOnlyTextReplacementFreeModifierSyntax {
                    sahai: markers.opening,
                    new_words,
                    lehai,
                },
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn projection_retains_full_fields_and_marker_spans() {
        let mut words = jbotci_morphology::segment_words_with_modifiers("lo'ai mi sa'ai do le'ai")
            .expect("quote");
        let quote = Token::bare(words.remove(0));
        let model::TextReplacementFreeModifierSyntax::FullTextReplacementFreeModifier(node) =
            project(&quote)
        else {
            panic!("full replacement")
        };
        assert_eq!(node.lohai.core_word().byte_range(), Some(0..5));
        assert_eq!(node.old_words.len(), 1);
        assert!(node.old_words[0].is_cmavo(Cmavo::Mi));
        assert_eq!(
            node.sahai
                .as_ref()
                .and_then(|token| token.core_word().byte_range()),
            Some(9..14)
        );
        assert_eq!(node.new_words.len(), 1);
        assert!(node.new_words[0].is_cmavo(Cmavo::Do));
        assert_eq!(node.lehai.value.core_word().byte_range(), Some(18..23));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn projection_retains_opening_emphasis_and_closing_indicators() {
        let mut words = jbotci_morphology::segment_words_with_modifiers("ba'e lo'ai mi le'ai ui")
            .expect("quote")
            .into_iter();
        let bahe = words
            .next()
            .expect("BAhE")
            .bare_word()
            .expect("raw BAhE")
            .clone();
        let quote = words.next().expect("quote");
        let ui = words
            .next()
            .expect("UI")
            .bare_word()
            .expect("raw UI")
            .clone();
        let token = Token::with_indicator(Token::emphasized(bahe, quote), ui, None);
        let model::TextReplacementFreeModifierSyntax::FullTextReplacementFreeModifier(node) =
            project(&token)
        else {
            panic!("full replacement")
        };
        assert!(matches!(
            node.lohai.as_indicators().as_data(),
            data!(WithIndicators::Emphasized { .. })
        ));
        assert!(
            matches!(node.lehai.value.as_indicators().as_data(), data!(WithIndicators::WithIndicator { indicator, .. }) if indicator.is_cmavo(Cmavo::Ui))
        );
        assert_eq!(node.old_words.len(), 1);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn projection_retains_new_only_and_close_only_forms() {
        let words = jbotci_morphology::segment_words_with_modifiers("sa'ai mi le'ai le'ai")
            .expect("quotes");
        let model::TextReplacementFreeModifierSyntax::NewOnlyTextReplacementFreeModifier(node) =
            project(&Token::bare(words[0].clone()))
        else {
            panic!("new-only replacement")
        };
        assert!(node.sahai.is_cmavo(Cmavo::Sahai));
        assert_eq!(node.new_words.len(), 1);
        assert!(node.new_words[0].is_cmavo(Cmavo::Mi));
        let model::TextReplacementFreeModifierSyntax::CloseOnlyTextReplacementFreeModifier(node) =
            project(&Token::bare(words[1].clone()))
        else {
            panic!("close-only replacement")
        };
        assert_eq!(node.0.value.core_word().byte_range(), Some(15..20));
    }
}
