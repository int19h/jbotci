#[allow(unused_imports)]
use bityzba::{data, ensures, invariant, new, requires};
use chumsky::Boxed;
use chumsky::error::Rich;
use chumsky::input::MappedInput;
use chumsky::prelude::*;
use chumsky::span::{SimpleSpan, Spanned};
use jbotci_morphology::{Word, WordKind, WordLike, WordLikeData, canonicalize_text};

use crate::{
    Connective, Fragment, FreeModifier, LojbanText, Paragraph, ParagraphStatement, ParseOptions,
    Statement, SyntaxError, SyntaxParse, WordWithModifiers, WordWithModifiersData,
};

pub(crate) mod ast;
use ast::*;
mod parser;
mod tense;
pub(crate) mod tokens;

type Span = SimpleSpan;
type Token = WordWithModifiers;
type SpannedToken = Spanned<Token, Span>;
type ParserInput<'tokens> = MappedInput<'tokens, Token, Span, &'tokens [SpannedToken]>;
type ParseExtra<'tokens> = extra::Err<Rich<'tokens, Token, Span>>;
type BoxedParser<'tokens, O> =
    Boxed<'tokens, 'tokens, ParserInput<'tokens>, O, ParseExtra<'tokens>>;

#[requires(true)]
#[ensures(true)]
pub(crate) fn parse_syntax_tree(
    words: &[WordLike],
    options: &ParseOptions,
) -> Result<SyntaxParse, SyntaxError> {
    parse_syntax_tree_with_source(words, None, options)
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn parse_syntax_tree_with_source(
    words: &[WordLike],
    source: Option<&str>,
    options: &ParseOptions,
) -> Result<SyntaxParse, SyntaxError> {
    let tokens = syntax_tokens(words);
    let text = parser::parse_statement(&tokens, source, options)?;
    Ok(new!(SyntaxParse {
        parse_tree: text,
        warnings: Vec::new(),
    }))
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn parse_text(
    words: &[WordLike],
    options: &ParseOptions,
) -> Result<LojbanText, SyntaxError> {
    let tokens = syntax_tokens(words);
    let text = parser::parse_statement(&tokens, None, options)?;
    let paragraphs = text
        .paragraphs
        .into_iter()
        .map(public_paragraph)
        .collect::<Vec<_>>();
    Ok(LojbanText {
        leading_nai: text.leading_nai,
        leading_cmevla: text.leading_cmevla,
        leading_indicators: text.leading_indicators,
        leading_free_modifiers: text
            .leading_free_modifiers
            .into_iter()
            .map(public_free_modifier)
            .collect(),
        leading_connective: text.leading_connective.map(public_connective),
        paragraphs,
    })
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn parse_raw_text(
    words: &[WordLike],
    options: &ParseOptions,
) -> Result<TextSyntax, SyntaxError> {
    let tokens = syntax_tokens(words);
    parser::parse_statement(&tokens, None, options)
}

#[requires(true)]
#[ensures(true)]
fn syntax_tokens(words: &[WordLike]) -> Vec<WordWithModifiers> {
    attach_indicators(attach_bahe(
        words.iter().cloned().map(WordWithModifiers::bare).collect(),
    ))
}

#[requires(true)]
#[ensures(true)]
fn attach_bahe(words: Vec<WordWithModifiers>) -> Vec<WordWithModifiers> {
    let mut out = Vec::new();
    let mut iter = words.into_iter().peekable();
    while let Some(word) = iter.next() {
        if modifier_word(&word)
            .is_some_and(|word| word.is_cmavo_text("ba'e") || word.is_cmavo_text("za'e"))
        {
            if let Some(next) = iter.next()
                && let Some(bahe) = modifier_word(&word)
                && let Some(word_like) = next.word_like()
            {
                out.push(WordWithModifiers::emphasized(bahe, word_like.clone()));
                continue;
            }
        }
        out.push(word);
    }
    out
}

#[requires(true)]
#[ensures(true)]
fn attach_indicators(words: Vec<WordWithModifiers>) -> Vec<WordWithModifiers> {
    let mut out = Vec::new();
    let mut iter = words.into_iter().peekable();
    while let Some(word) = iter.next() {
        if modifier_word(&word).is_some_and(|word| is_indicator_word(&word)) {
            let indicator = modifier_word(&word);
            let nai = if iter
                .peek()
                .and_then(modifier_word)
                .is_some_and(|next| next.is_cmavo_text("nai"))
            {
                iter.next().and_then(|next| modifier_word(&next))
            } else {
                None
            };
            if let (Some(prev), Some(indicator)) = (out.pop(), indicator) {
                let prev_is_leading_indicator_nai = modifier_word(&prev)
                    .is_some_and(|word| word.is_cmavo_text("nai"))
                    && out
                        .last()
                        .and_then(modifier_word)
                        .is_some_and(|word| is_indicator_word(&word));
                if prev_is_leading_indicator_nai {
                    out.push(prev);
                    out.push(word);
                    if let Some(nai) = nai {
                        out.push(WordWithModifiers::bare(WordLike::bare(nai)));
                    }
                } else {
                    out.push(WordWithModifiers::with_indicator(prev, indicator, nai));
                }
            } else {
                out.push(word);
                if let Some(nai) = nai {
                    out.push(WordWithModifiers::bare(WordLike::bare(nai)));
                }
            }
        } else {
            out.push(word);
        }
    }
    out
}

#[requires(true)]
#[ensures(true)]
fn modifier_word(word: &WordWithModifiers) -> Option<Word> {
    match word.as_data() {
        data!(WordWithModifiers::Bare(word_like))
        | data!(WordWithModifiers::Emphasized { word_like, .. }) => match word_like.as_data() {
            data!(WordLike::Bare(word)) => Some((**word).clone()),
            _ => None,
        },
        data!(WordWithModifiers::WithIndicator { base, .. }) => modifier_word(base),
    }
}

#[requires(true)]
#[ensures(true)]
fn is_indicator_word(word: &Word) -> bool {
    let text = canonicalize_text(&word.phonemes);
    word.kind == WordKind::Cmavo
        && (tokens::UI_WORDS.contains(&text.as_str())
            || tokens::CAI_WORDS.contains(&text.as_str())
            || text == "y")
}

#[requires(true)]
#[ensures(true)]
fn public_paragraph(paragraph: ParagraphSyntax) -> Paragraph {
    Paragraph {
        i: paragraph.i,
        niho: paragraph.niho,
        free_modifiers: paragraph
            .free_modifiers
            .into_iter()
            .map(public_free_modifier)
            .collect(),
        statements: paragraph
            .statements
            .into_iter()
            .map(public_paragraph_statement)
            .collect(),
    }
}

#[requires(true)]
#[ensures(true)]
fn public_paragraph_statement(statement: ParagraphStatementSyntax) -> ParagraphStatement {
    ParagraphStatement {
        i: statement.i,
        connective: statement.connective.map(public_connective),
        free_modifiers: statement
            .free_modifiers
            .into_iter()
            .map(public_free_modifier)
            .collect(),
        statement: statement.statement.map(public_statement),
    }
}

#[requires(true)]
#[ensures(true)]
fn public_statement(statement: StatementSyntax) -> Statement {
    Statement::fragment(Fragment::other(statement.words()))
}

#[requires(true)]
#[ensures(true)]
fn public_free_modifier(free_modifier: FreeModifierSyntax) -> FreeModifier {
    FreeModifier::words(free_modifier.words())
}

#[requires(true)]
#[ensures(true)]
fn public_connective(connective: ConnectiveSyntax) -> Connective {
    Connective::words(connective.words())
}

#[cfg(test)]
mod tests {
    use bityzba::requires;
    use jbotci_morphology::segment_words_with_modifiers;

    use super::*;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn parses_basic_predicate_with_leading_and_tail_terms() {
        let words = segment_words_with_modifiers("do mamta mi").expect("valid morphology");

        let parsed = parse_syntax_tree(&words, &ParseOptions::default()).expect("valid syntax");

        assert_eq!(parsed.parse_tree.paragraphs.len(), 1);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn rejects_stray_cu() {
        let words = segment_words_with_modifiers("cu").expect("valid morphology");

        let error = parse_syntax_tree(&words, &ParseOptions::default()).expect_err("invalid");

        assert!(matches!(error, SyntaxError::Parse { .. }));
    }
}
