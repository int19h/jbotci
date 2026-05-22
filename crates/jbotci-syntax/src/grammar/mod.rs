#[allow(unused_imports)]
use bityzba::{data, ensures, invariant, new, requires};
use std::collections::VecDeque;

use chumsky::Boxed;
use chumsky::error::Rich;
use chumsky::input::MappedInput;
use chumsky::input::{Checkpoint, Cursor};
use chumsky::inspector::Inspector;
use chumsky::prelude::*;
use chumsky::span::{SimpleSpan, Spanned};
use jbotci_dialect::DialectFeature;
use jbotci_morphology::{Word, WordKind, WordLike, WordLikeData, canonicalize_text};

use crate::{
    Connective, ExperimentalConstruct, Fragment, FreeModifier, LojbanText, Paragraph,
    ParagraphStatement, ParseOptions, Statement, SyntaxError, SyntaxParse, SyntaxWarning,
    WithIndicators,
};

pub(crate) mod ast;
use ast::*;
mod parser;
mod tense;
pub(crate) mod tokens;

type Span = SimpleSpan;
type Token = WithIndicators<WordLike>;
type SpannedToken = Spanned<Token, Span>;
type ParserInput<'tokens> = MappedInput<'tokens, Token, Span, &'tokens [SpannedToken]>;
type ParseExtra<'tokens> = extra::Full<Rich<'tokens, Token, Span>, ParserState, ()>;
type BoxedParser<'tokens, O> =
    Boxed<'tokens, 'tokens, ParserInput<'tokens>, O, ParseExtra<'tokens>>;

#[derive(Debug, Clone)]
#[invariant(true)]
pub(super) struct ParsedStatement {
    pub text: TextSyntax,
    pub warnings: Vec<SyntaxWarning>,
}

#[derive(Debug, Clone, Default)]
#[invariant(true)]
pub(super) struct ParserState {
    anchor_byte_starts: Vec<Option<usize>>,
    warnings: Vec<SyntaxWarning>,
    dialect_features: std::collections::BTreeSet<DialectFeature>,
}

impl ParserState {
    #[requires(true)]
    #[ensures(ret.anchor_byte_starts.len() == words.len())]
    pub(super) fn new(words: &[WithIndicators<WordLike>], options: &ParseOptions) -> Self {
        Self {
            anchor_byte_starts: words.iter().map(word_anchor_byte_start).collect(),
            warnings: Vec::new(),
            dialect_features: options.dialect.features.clone(),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn feature_enabled(&self, feature: DialectFeature) -> bool {
        self.dialect_features.contains(&feature)
    }

    #[requires(true)]
    #[ensures(self.warnings.len() == old(self.warnings.len()) + 1)]
    pub(super) fn warn(
        &mut self,
        construct: ExperimentalConstruct,
        anchor: &WithIndicators<WordLike>,
    ) {
        let anchor_index = self.anchor_index(anchor);
        self.warnings.push(SyntaxWarning::experimental_construct(
            construct,
            anchor_index,
            anchor.clone(),
        ));
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn finish_warnings(self) -> Vec<SyntaxWarning> {
        let mut deduped = Vec::new();
        for warning in self.warnings {
            if !deduped.contains(&warning) {
                deduped.push(warning);
            }
        }
        deduped
    }

    #[requires(true)]
    #[ensures(ret < self.anchor_byte_starts.len() || self.anchor_byte_starts.is_empty())]
    fn anchor_index(&self, anchor: &WithIndicators<WordLike>) -> usize {
        if let Some(anchor_start) = word_anchor_byte_start(anchor)
            && let Some(index) = self
                .anchor_byte_starts
                .iter()
                .position(|candidate| *candidate == Some(anchor_start))
        {
            return index;
        }
        0
    }
}

impl<'tokens> Inspector<'tokens, ParserInput<'tokens>> for ParserState {
    type Checkpoint = usize;

    #[requires(true)]
    #[ensures(true)]
    fn on_token(&mut self, _token: &Token) {}

    #[requires(true)]
    #[ensures(ret == self.warnings.len())]
    fn on_save<'parse>(&self, _cursor: &Cursor<'tokens, 'parse, ParserInput<'tokens>>) -> usize {
        self.warnings.len()
    }

    #[requires(true)]
    #[ensures(self.warnings.len() <= old(self.warnings.len()))]
    fn on_rewind<'parse>(
        &mut self,
        marker: &Checkpoint<'tokens, 'parse, ParserInput<'tokens>, usize>,
    ) {
        self.warnings.truncate(*marker.inspector());
    }
}

#[requires(true)]
#[ensures(true)]
fn word_anchor_byte_start(word: &WithIndicators<WordLike>) -> Option<usize> {
    word.visible_word().map(|word| word.span.byte_start)
}

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
    let parsed = parser::parse_statement(&tokens, source, options)?;
    Ok(new!(SyntaxParse {
        parse_tree: parsed.text,
        warnings: parsed.warnings,
    }))
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn parse_text(
    words: &[WordLike],
    options: &ParseOptions,
) -> Result<LojbanText, SyntaxError> {
    let tokens = syntax_tokens(words);
    let text = parser::parse_statement(&tokens, None, options)?.text;
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
    Ok(parser::parse_statement(&tokens, None, options)?.text)
}

#[requires(true)]
#[ensures(true)]
fn syntax_tokens(words: &[WordLike]) -> Vec<WithIndicators<WordLike>> {
    attach_indicators(attach_bahe(
        words.iter().cloned().map(WithIndicators::bare).collect(),
    ))
}

#[requires(true)]
#[ensures(true)]
fn attach_bahe(words: Vec<WithIndicators<WordLike>>) -> Vec<WithIndicators<WordLike>> {
    let mut reversed: VecDeque<_> = words.into_iter().rev().collect();
    let mut out = Vec::new();
    while let Some(word) = reversed.pop_front() {
        if reversed.front().is_some_and(is_bahe_word)
            && let Some(bahe_token) = reversed.pop_front()
            && let Some(bahe) = modifier_word(&bahe_token)
            && let Some(word_like) = word.word_like()
        {
            reversed.push_front(WithIndicators::emphasized(bahe, word_like.clone()));
        } else {
            out.push(word);
        }
    }
    out.reverse();
    out
}

#[requires(true)]
#[ensures(true)]
fn is_bahe_word(word: &WithIndicators<WordLike>) -> bool {
    modifier_word(word).is_some_and(|word| word.is_cmavo_text("ba'e") || word.is_cmavo_text("za'e"))
}

#[requires(true)]
#[ensures(true)]
fn attach_indicators(words: Vec<WithIndicators<WordLike>>) -> Vec<WithIndicators<WordLike>> {
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
                        out.push(WithIndicators::bare(WordLike::bare(nai)));
                    }
                } else {
                    out.push(WithIndicators::with_indicator(prev, indicator, nai));
                }
            } else {
                out.push(word);
                if let Some(nai) = nai {
                    out.push(WithIndicators::bare(WordLike::bare(nai)));
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
fn modifier_word(word: &WithIndicators<WordLike>) -> Option<Word> {
    match word {
        WithIndicators::Bare(word_like) | WithIndicators::Emphasized { word_like, .. } => {
            match word_like.as_data() {
                data!(WordLike::Bare(word)) => Some((**word).clone()),
                _ => None,
            }
        }
        WithIndicators::WithIndicator { base, .. } => modifier_word(base),
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
    use jbotci_dialect::parse_dialect_definition;
    use jbotci_morphology::segment_words_with_modifiers;

    use super::*;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn parses_basic_predicate_with_leading_and_tail_terms() {
        run_on_large_stack(|| {
            let words = segment_words_with_modifiers("do mamta mi").expect("valid morphology");

            let parsed = parse_syntax_tree(&words, &ParseOptions::default()).expect("valid syntax");

            assert_eq!(parsed.parse_tree.paragraphs.len(), 1);
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn rejects_stray_cu() {
        run_on_large_stack(|| {
            let words = segment_words_with_modifiers("cu").expect("valid morphology");

            let error = parse_syntax_tree(&words, &ParseOptions::default()).expect_err("invalid");

            assert!(matches!(error, SyntaxError::Parse { .. }));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn parses_grouped_math_operator() {
        run_on_large_stack(|| {
            let words = segment_words_with_modifiers("li re ke su'i ke'e ci du li mu")
                .expect("valid morphology");

            let parsed = parse_syntax_tree(&words, &ParseOptions::default()).expect("valid syntax");

            assert!(format!("{:#?}", parsed.parse_tree).contains("Ke"));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn parses_bo_connected_math_operator() {
        run_on_large_stack(|| {
            let words = segment_words_with_modifiers("li re su'i je bo vu'u ci du li mu")
                .expect("valid morphology");

            let parsed = parse_syntax_tree(&words, &ParseOptions::default()).expect("valid syntax");

            assert!(format!("{:#?}", parsed.parse_tree).contains("Bo"));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gates_zantufa_quote_relation_units() {
        run_on_large_stack(|| {
            let words =
                segment_words_with_modifiers("lu'ei mi klama li'au").expect("valid morphology");

            assert!(parse_syntax_tree(&words, &ParseOptions::default()).is_err());

            let dialect =
                parse_dialect_definition("(+ZANTUFA-QUOTES)").expect("valid dialect definition");
            let options = ParseOptions::default().with_dialect_definition(&dialect);
            let parsed = parse_syntax_tree(&words, &options).expect("valid zantufa quote syntax");

            assert!(parsed.warnings.iter().any(|warning| {
                warning.kind == ExperimentalConstruct::ExperimentalZantufaLuheiRelationUnit
            }));

            let words =
                segment_words_with_modifiers("mi cu mu'oi gy foo gy").expect("valid morphology");

            assert!(parse_syntax_tree(&words, &ParseOptions::default()).is_err());

            let parsed = parse_syntax_tree(&words, &options).expect("valid zantufa MUhOI syntax");

            assert!(parsed.warnings.iter().any(|warning| {
                warning.kind == ExperimentalConstruct::ExperimentalZantufaMuhoiRelationUnit
            }));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gates_zantufa_jai_tag_terms() {
        run_on_large_stack(|| {
            let words =
                segment_words_with_modifiers("jai pu mi cu klama").expect("valid morphology");

            assert!(parse_syntax_tree(&words, &ParseOptions::default()).is_err());

            let dialect =
                parse_dialect_definition("(+ZANTUFA-TAGS)").expect("valid dialect definition");
            let options = ParseOptions::default().with_dialect_definition(&dialect);
            let parsed = parse_syntax_tree(&words, &options).expect("valid zantufa JAI tag term");

            assert!(parsed.warnings.iter().any(|warning| {
                warning.kind == ExperimentalConstruct::ExperimentalZantufaJaiTagTerm
            }));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gates_zantufa_poiha_brigahi_ku() {
        run_on_large_stack(|| {
            let words = segment_words_with_modifiers("noi'a klama ku mi cu broda")
                .expect("valid morphology");

            assert!(parse_syntax_tree(&words, &ParseOptions::default()).is_err());

            let dialect = parse_dialect_definition("(+ZANTUFA-ADVERBIALS)")
                .expect("valid dialect definition");
            let options = ParseOptions::default().with_dialect_definition(&dialect);
            let parsed = parse_syntax_tree(&words, &options).expect("valid Zantufa POIhA briga'i");

            assert!(parsed.warnings.iter().any(|warning| {
                warning.kind == ExperimentalConstruct::ExperimentalZantufaPoihaBrigahi
            }));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gates_zantufa_cmavo_table_entries() {
        run_on_large_stack(|| {
            let words = segment_words_with_modifiers("mi bo'ei do").expect("valid morphology");

            assert!(parse_syntax_tree(&words, &ParseOptions::default()).is_err());

            let dialect =
                parse_dialect_definition("(+ZANTUFA-CMAVO)").expect("valid dialect definition");
            let options = ParseOptions::default().with_dialect_definition(&dialect);
            let parsed = parse_syntax_tree(&words, &options).expect("valid Zantufa cmavo syntax");

            assert!(parsed.warnings.iter().any(|warning| {
                warning.kind == ExperimentalConstruct::ExperimentalZantufaCmavo
            }));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gates_zantufa_initial_gi_gek() {
        run_on_large_stack(|| {
            let words = segment_words_with_modifiers("gi je mi klama gi do klama")
                .expect("valid morphology");

            assert!(parse_syntax_tree(&words, &ParseOptions::default()).is_err());

            let dialect = parse_dialect_definition("(+ZANTUFA-CONNECTIVES)")
                .expect("valid dialect definition");
            let options = ParseOptions::default().with_dialect_definition(&dialect);
            let parsed = parse_syntax_tree(&words, &options).expect("valid Zantufa GI GEK");

            assert!(
                parsed
                    .warnings
                    .iter()
                    .any(|warning| warning.kind == ExperimentalConstruct::ExperimentalZantufaGek)
            );
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gates_zantufa_gihi_forethought_terminator() {
        run_on_large_stack(|| {
            let words = segment_words_with_modifiers("ge mi klama gi do klama gi'i")
                .expect("valid morphology");

            assert!(parse_syntax_tree(&words, &ParseOptions::default()).is_err());

            let dialect = parse_dialect_definition("(+ZANTUFA-CONNECTIVES)")
                .expect("valid dialect definition");
            let options = ParseOptions::default().with_dialect_definition(&dialect);
            let parsed = parse_syntax_tree(&words, &options).expect("valid Zantufa GIhI");

            assert!(parsed.warnings.iter().any(|warning| {
                warning.kind == ExperimentalConstruct::ExperimentalZantufaForethoughtGihi
            }));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn warns_for_flat_tag_forms() {
        run_on_large_stack(|| {
            let words =
                segment_words_with_modifiers("na'e fa mi cu klama").expect("valid morphology");

            let parsed = parse_syntax_tree(&words, &ParseOptions::default())
                .expect("valid flattened FA tag");

            assert!(
                parsed
                    .warnings
                    .iter()
                    .any(|warning| warning.kind == ExperimentalConstruct::ExperimentalFlattenedTag)
            );
            assert!(
                parsed
                    .warnings
                    .iter()
                    .any(|warning| warning.kind == ExperimentalConstruct::ExperimentalFaAsTag)
            );
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gates_zantufa_recursive_tags() {
        run_on_large_stack(|| {
            let words = segment_words_with_modifiers("na'e se na'e se fa mi cu klama")
                .expect("valid morphology");

            assert!(parse_syntax_tree(&words, &ParseOptions::default()).is_err());

            let dialect =
                parse_dialect_definition("(+ZANTUFA-TAGS)").expect("valid dialect definition");
            let options = ParseOptions::default().with_dialect_definition(&dialect);
            let parsed = parse_syntax_tree(&words, &options).expect("valid recursive tag");

            assert!(parsed.warnings.iter().any(|warning| {
                warning.kind == ExperimentalConstruct::ExperimentalZantufaRecursiveTag
            }));
        });
    }

    #[requires(true)]
    #[ensures(true)]
    fn run_on_large_stack(test: impl FnOnce() + Send + 'static) {
        std::thread::Builder::new()
            .stack_size(32 * 1024 * 1024)
            .spawn(test)
            .expect("spawn large-stack syntax test")
            .join()
            .expect("large-stack syntax test thread");
    }
}
