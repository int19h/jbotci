#[allow(unused_imports)]
use bityzba::{data, ensures, invariant, new, requires};
use chumsky::Boxed;
use chumsky::error::Rich;
use chumsky::input::MappedInput;
use chumsky::prelude::*;
use chumsky::span::{SimpleSpan, Spanned};
use jbotci_morphology::{
    WordKind, WordLike, WordLikeData, WordWithModifiers, WordWithModifiersData,
};

use crate::{
    Connective, Fragment, FreeModifier, LojbanText, Paragraph, ParagraphStatement, ParseOptions,
    Statement, SyntaxError, SyntaxField, SyntaxParse, SyntaxValue,
};

mod ast;
use ast::*;
mod parser;
mod render;
mod tense;
mod tokens;

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
    words: &[WordWithModifiers],
    options: &ParseOptions,
) -> Result<SyntaxParse, SyntaxError> {
    parse_syntax_tree_with_source(words, None, options)
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn parse_syntax_tree_with_source(
    words: &[WordWithModifiers],
    source: Option<&str>,
    options: &ParseOptions,
) -> Result<SyntaxParse, SyntaxError> {
    let text = parser::parse_statement(words, source, options)?;
    Ok(new!(SyntaxParse {
        parse_tree: render::lojban_text_tree(text),
        warnings: Vec::new(),
    }))
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn parse_text(
    words: &[WordWithModifiers],
    options: &ParseOptions,
) -> Result<LojbanText, SyntaxError> {
    let text = parser::parse_statement(words, None, options)?;
    let paragraphs = text
        .paragraphs
        .into_iter()
        .map(public_paragraph)
        .collect::<Vec<_>>();
    Ok(new!(LojbanText {
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
    }))
}

#[requires(true)]
#[ensures(true)]
fn public_paragraph(paragraph: ParagraphSyntax) -> Paragraph {
    new!(Paragraph {
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
    })
}

#[requires(true)]
#[ensures(true)]
fn public_paragraph_statement(statement: ParagraphStatementSyntax) -> ParagraphStatement {
    new!(ParagraphStatement {
        i: statement.i,
        connective: statement.connective.map(public_connective),
        free_modifiers: statement
            .free_modifiers
            .into_iter()
            .map(public_free_modifier)
            .collect(),
        statement: statement.statement.map(public_statement),
    })
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

    use crate::SyntaxValueData;

    use super::*;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn parses_basic_predicate_with_leading_and_tail_terms() {
        let words = segment_words_with_modifiers("do mamta mi").expect("valid morphology");

        let parsed = parse_syntax_tree(&words, &ParseOptions::default()).expect("valid syntax");

        let data!(SyntaxValue::Node { node }) = parsed.parse_tree.as_data() else {
            panic!("expected node");
        };
        assert_eq!(node.constructor, "LojbanText");
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
