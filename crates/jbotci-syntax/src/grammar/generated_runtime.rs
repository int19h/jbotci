//! Generic runtime primitives for declarative generated syntax parsers.

use bityzba::{invariant, new, requires};
use chumsky::{
    IterParser, Parser,
    input::Input,
    primitive::{choice, custom},
};
use jbotci_diagnostics::{TraceEventKind, TraceLevel};
use jbotci_dialect::DialectFeature;
use jbotci_morphology::{Cmavo, Selmaho};

use super::{
    BoxedParser, ParseExtra, ParserInput, Span, SyntaxFound, SyntaxFoundData, SyntaxParseError,
    tokens::{
        cmevla_word, is_brivla_relation_word, is_cmevla_word, is_koha_argument, is_letter_word,
        is_relation_word, token_matching,
    },
};
use crate::{
    ExperimentalConstruct, ParseOptions, SyntaxExpectedToken, SyntaxExpectedTokenData,
    SyntaxWordCategory, Token,
};

#[invariant(!words.is_empty(), "vocative marker sequence cannot be empty")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VocativeMarkerWordsSyntax {
    pub words: Vec<Token>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
pub(crate) struct SyntaxGrammarEnv {
    pub dialect: SyntaxGrammarDialect,
    pub policy: SyntaxGrammarPolicy,
}

impl SyntaxGrammarEnv {
    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn from_options(options: &ParseOptions) -> Self {
        Self {
            dialect: SyntaxGrammarDialect::from_options(options),
            policy: SyntaxGrammarPolicy::default(),
        }
    }
}

impl Default for SyntaxGrammarEnv {
    #[requires(true)]
    #[ensures(true)]
    fn default() -> Self {
        Self {
            dialect: SyntaxGrammarDialect::default(),
            policy: SyntaxGrammarPolicy::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[invariant(true)]
pub(crate) struct SyntaxGrammarDialect {
    pub term_hierarchy_enabled: bool,
    pub cbm_enabled: bool,
    pub zantufa_connectives_enabled: bool,
    pub zantufa_tags_enabled: bool,
}

impl SyntaxGrammarDialect {
    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn from_options(options: &ParseOptions) -> Self {
        let features = &options.dialect.features;
        Self {
            term_hierarchy_enabled: features.contains(&DialectFeature::TermHierarchy),
            cbm_enabled: features.contains(&DialectFeature::Cbm),
            zantufa_connectives_enabled: features.contains(&DialectFeature::ZantufaConnectives),
            zantufa_tags_enabled: features.contains(&DialectFeature::ZantufaTags),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
#[allow(dead_code)]
pub(crate) enum SyntaxGrammarFeature {
    TermHierarchy,
    Cbm,
    ZantufaConnectives,
    ZantufaTags,
}

impl SyntaxGrammarFeature {
    #[requires(true)]
    #[ensures(true)]
    fn enabled(self, dialect: SyntaxGrammarDialect) -> bool {
        match self {
            Self::TermHierarchy => dialect.term_hierarchy_enabled,
            Self::Cbm => dialect.cbm_enabled,
            Self::ZantufaConnectives => dialect.zantufa_connectives_enabled,
            Self::ZantufaTags => dialect.zantufa_tags_enabled,
        }
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn expected_name(self) -> &'static str {
        match self {
            Self::TermHierarchy => "TERM-HIERARCHY feature",
            Self::Cbm => "CBM feature",
            Self::ZantufaConnectives => "ZANTUFA-CONNECTIVES feature",
            Self::ZantufaTags => "ZANTUFA-TAGS feature",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
pub(crate) struct SyntaxGrammarPolicy {
    pub soi_adverbials_enabled: bool,
    pub zantufa_adverbials_enabled: bool,
    pub zantufa_quotes_enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
#[allow(dead_code)]
pub(crate) enum SyntaxGrammarPolicyFlag {
    SoiAdverbials,
    ZantufaAdverbials,
    ZantufaQuotes,
}

impl SyntaxGrammarPolicyFlag {
    #[requires(true)]
    #[ensures(true)]
    fn enabled(self, policy: SyntaxGrammarPolicy) -> bool {
        match self {
            Self::SoiAdverbials => policy.soi_adverbials_enabled,
            Self::ZantufaAdverbials => policy.zantufa_adverbials_enabled,
            Self::ZantufaQuotes => policy.zantufa_quotes_enabled,
        }
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn expected_name(self) -> &'static str {
        match self {
            Self::SoiAdverbials => "SOI adverbials policy",
            Self::ZantufaAdverbials => "Zantufa adverbials policy",
            Self::ZantufaQuotes => "Zantufa quotes policy",
        }
    }
}

impl Default for SyntaxGrammarPolicy {
    #[requires(true)]
    #[ensures(ret.soi_adverbials_enabled)]
    #[ensures(ret.zantufa_adverbials_enabled)]
    #[ensures(ret.zantufa_quotes_enabled)]
    fn default() -> Self {
        Self {
            soi_adverbials_enabled: true,
            zantufa_adverbials_enabled: true,
            zantufa_quotes_enabled: true,
        }
    }
}

#[requires(!name.is_empty())]
#[ensures(true)]
pub(crate) fn memoized_rule<'tokens, O, P>(name: &'static str, parser: P) -> BoxedParser<'tokens, O>
where
    O: Clone + 'static,
    P: Parser<'tokens, ParserInput<'tokens>, O, ParseExtra<'tokens>> + Clone + 'tokens,
{
    custom::<_, ParserInput<'tokens>, _, ParseExtra<'tokens>>(move |input| {
        let checkpoint = input.save();
        let start_location = ParserInput::cursor_location(checkpoint.cursor().inner());
        if let Some((output, end_location, warnings)) =
            input.state().syntax_memo_success::<O>(name, start_location)
        {
            advance_to_location(input, end_location);
            input.state().extend_warnings(&warnings);
            return Ok(output);
        }
        let warning_start = input.state().warning_count();
        match input.parse(&parser) {
            Ok(output) => {
                let end_location = ParserInput::cursor_location(input.cursor().inner());
                let warnings = input.state().warnings_since(warning_start);
                input.state().store_syntax_memo_success(
                    name,
                    start_location,
                    end_location,
                    output.clone(),
                    warnings,
                );
                Ok(output)
            }
            Err(error) => {
                input.rewind(checkpoint);
                Err(error)
            }
        }
    })
    .boxed()
}

#[requires(!construct.is_empty())]
#[ensures(true)]
pub(crate) fn syntax_context<'tokens, O: 'tokens>(
    construct: &'static str,
    parser: impl Parser<'tokens, ParserInput<'tokens>, O, ParseExtra<'tokens>> + 'tokens,
) -> BoxedParser<'tokens, O> {
    trace_enter(construct)
        .ignore_then(
            parser
                .labelled(construct)
                .as_context()
                .map_with(move |output, extra| {
                    let span: Span = extra.span();
                    let byte_start = span.start.min(span.end);
                    let byte_end = span.start.max(span.end);
                    extra.state().trace_exit_construct(
                        TraceLevel::Top,
                        TraceEventKind::ConstructSuccess,
                        construct,
                        byte_start,
                        byte_end,
                        || None,
                    );
                    output
                })
                .map_err_with_state(move |error, span: Span, state| {
                    let byte_start = span.start.min(span.end);
                    let byte_end = span.start.max(span.end);
                    state.trace_exit_construct(
                        TraceLevel::Top,
                        TraceEventKind::ConstructFailure,
                        construct,
                        byte_start,
                        byte_end,
                        || None,
                    );
                    error
                }),
        )
        .boxed()
}

#[requires(!construct.is_empty())]
#[ensures(true)]
fn trace_enter<'tokens>(construct: &'static str) -> BoxedParser<'tokens, ()> {
    custom::<_, ParserInput<'tokens>, (), ParseExtra<'tokens>>(move |input| {
        if input
            .state()
            .trace_should_record(TraceLevel::Top, construct)
        {
            input
                .state()
                .trace_enter_construct(TraceLevel::Top, construct, 0, 0);
        }
        Ok(())
    })
    .boxed()
}

#[requires(true)]
#[ensures(true)]
fn advance_to_location<'tokens>(
    input: &mut chumsky::input::InputRef<'tokens, '_, ParserInput<'tokens>, ParseExtra<'tokens>>,
    end_location: usize,
) {
    while ParserInput::cursor_location(input.cursor().inner()) < end_location {
        if input.next().is_none() {
            break;
        }
    }
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn empty<'tokens>() -> BoxedParser<'tokens, ()> {
    chumsky::primitive::empty().boxed()
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn eof<'tokens>() -> BoxedParser<'tokens, ()> {
    chumsky::primitive::end().boxed()
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn feature_gate<'tokens, O, P>(
    feature: SyntaxGrammarFeature,
    parser: P,
) -> BoxedParser<'tokens, O>
where
    O: 'tokens,
    P: Parser<'tokens, ParserInput<'tokens>, O, ParseExtra<'tokens>> + Clone + 'tokens,
{
    custom::<_, ParserInput<'tokens>, _, ParseExtra<'tokens>>(move |input| {
        let env = input.state().syntax_grammar_env();
        if feature.enabled(env.dialect) {
            return input.parse(&parser);
        }

        let checkpoint = input.save();
        let cursor = input.cursor();
        let found = input
            .next()
            .map(|token| new!(SyntaxFound::Token(token)))
            .unwrap_or_else(|| new!(SyntaxFound::EndOfInput));
        let span = input.span_since(&cursor);
        input.rewind(checkpoint);
        Err(SyntaxParseError::expected_found(
            span,
            vec![new!(SyntaxExpectedToken::Named(
                feature.expected_name().to_owned()
            ))],
            found,
        ))
    })
    .boxed()
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn policy_gate<'tokens, O, P>(
    policy: SyntaxGrammarPolicyFlag,
    parser: P,
) -> BoxedParser<'tokens, O>
where
    O: 'tokens,
    P: Parser<'tokens, ParserInput<'tokens>, O, ParseExtra<'tokens>> + Clone + 'tokens,
{
    custom::<_, ParserInput<'tokens>, _, ParseExtra<'tokens>>(move |input| {
        let env = input.state().syntax_grammar_env();
        if policy.enabled(env.policy) {
            return input.parse(&parser);
        }

        let checkpoint = input.save();
        let cursor = input.cursor();
        let found = input
            .next()
            .map(|token| new!(SyntaxFound::Token(token)))
            .unwrap_or_else(|| new!(SyntaxFound::EndOfInput));
        let span = input.span_since(&cursor);
        input.rewind(checkpoint);
        Err(SyntaxParseError::expected_found(
            span,
            vec![new!(SyntaxExpectedToken::Named(
                policy.expected_name().to_owned()
            ))],
            found,
        ))
    })
    .boxed()
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn strict_optional<'tokens, O, P>(parser: P) -> BoxedParser<'tokens, Option<O>>
where
    O: 'tokens,
    P: Parser<'tokens, ParserInput<'tokens>, O, ParseExtra<'tokens>> + 'tokens,
{
    parser.or_not().boxed()
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn strict_greedy_many_parser<'tokens, O: 'tokens>(
    parser: BoxedParser<'tokens, O>,
) -> BoxedParser<'tokens, Vec<O>> {
    parser.repeated().collect::<Vec<_>>().boxed()
}

#[requires(true)]
#[ensures(true)]
fn strict_greedy_many_parser_without_diagnostics<'tokens, O: 'tokens>(
    parser: BoxedParser<'tokens, O>,
) -> BoxedParser<'tokens, Vec<O>> {
    strict_greedy_many_parser(parser)
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn strict_greedy_many1_parser<'tokens, O: 'tokens>(
    parser: BoxedParser<'tokens, O>,
) -> BoxedParser<'tokens, Vec<O>> {
    parser
        .clone()
        .then(strict_greedy_many_parser(parser))
        .map(|(first, rest)| std::iter::once(first).chain(rest).collect())
        .boxed()
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn singleton<'tokens, O, P>(parser: P) -> BoxedParser<'tokens, Vec<O>>
where
    O: 'tokens,
    P: Parser<'tokens, ParserInput<'tokens>, O, ParseExtra<'tokens>> + 'tokens,
{
    parser.map(|item| vec![item]).boxed()
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn prepend<'tokens, O, H, T>(head: H, tail: T) -> BoxedParser<'tokens, Vec<O>>
where
    O: 'tokens,
    H: Parser<'tokens, ParserInput<'tokens>, O, ParseExtra<'tokens>> + 'tokens,
    T: Parser<'tokens, ParserInput<'tokens>, Vec<O>, ParseExtra<'tokens>> + 'tokens,
{
    head.then(tail)
        .map(|(head, tail)| std::iter::once(head).chain(tail).collect())
        .boxed()
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn concat<'tokens, O, H, T>(head: H, tail: T) -> BoxedParser<'tokens, Vec<O>>
where
    O: 'tokens,
    H: Parser<'tokens, ParserInput<'tokens>, Vec<O>, ParseExtra<'tokens>> + 'tokens,
    T: Parser<'tokens, ParserInput<'tokens>, Vec<Vec<O>>, ParseExtra<'tokens>> + 'tokens,
{
    head.then(tail)
        .map(|(mut head, tail)| {
            for mut segment in tail {
                head.append(&mut segment);
            }
            head
        })
        .boxed()
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn strict_ordered_choice_parsers<'tokens, O: 'tokens>(
    alternatives: Vec<BoxedParser<'tokens, O>>,
) -> BoxedParser<'tokens, O> {
    choice(alternatives).boxed()
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn strict_free_modifier_list_parser<'tokens>(
    free_modifier: BoxedParser<'tokens, crate::tree::FreeModifierSyntax>,
) -> BoxedParser<'tokens, Vec<crate::tree::FreeModifierSyntax>> {
    strict_greedy_many_parser_without_diagnostics(free_modifier)
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn strict_cll_prohibited_free_modifier_list_parser<'tokens>(
    free_modifier: BoxedParser<'tokens, crate::tree::FreeModifierSyntax>,
) -> BoxedParser<'tokens, Vec<crate::tree::FreeModifierSyntax>> {
    strict_greedy_many_parser_without_diagnostics(
        free_modifier
            .map_with(
                |free_modifier,
                 extra: &mut chumsky::input::MapExtra<
                    'tokens,
                    '_,
                    ParserInput<'tokens>,
                    ParseExtra<'tokens>,
                >| {
                    if let Some(anchor) = free_modifier.first_word() {
                        extra.state().warn(
                            ExperimentalConstruct::CllProhibitedFreeModifierPlacement,
                            anchor,
                        );
                    }
                    free_modifier
                },
            )
            .boxed(),
    )
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn strict_empty_free_modifier_parser<'tokens>()
-> BoxedParser<'tokens, crate::tree::FreeModifierSyntax> {
    custom::<_, ParserInput<'tokens>, _, ParseExtra<'tokens>>(move |input| {
        let checkpoint = input.save();
        let cursor = input.cursor();
        let found = input
            .next()
            .map(|token| new!(SyntaxFound::Token(token)))
            .unwrap_or_else(|| new!(SyntaxFound::EndOfInput));
        let span = input.span_since(&cursor);
        input.rewind(checkpoint);
        Err(SyntaxParseError::expected_found(
            span,
            vec![new!(SyntaxExpectedToken::Named("free modifier".to_owned()))],
            found,
        ))
    })
    .boxed()
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn not_next_selmaho<'tokens>(selmaho: Selmaho) -> BoxedParser<'tokens, ()> {
    custom::<_, ParserInput<'tokens>, _, ParseExtra<'tokens>>(move |input| {
        let checkpoint = input.save();
        let cursor = input.cursor();
        match input.next() {
            Some(token) if token.is_selmaho(selmaho) => {
                let span = input.span_since(&cursor);
                input.rewind(checkpoint);
                Err(SyntaxParseError::expected_found(
                    span,
                    vec![new!(SyntaxExpectedToken::Named(format!(
                        "not {}",
                        selmaho.name()
                    )))],
                    new!(SyntaxFound::Token(token)),
                ))
            }
            _ => {
                input.rewind(checkpoint);
                Ok(())
            }
        }
    })
    .boxed()
}

#[requires(!expected.is_empty())]
#[ensures(true)]
pub(crate) fn not_next_rule_after<'tokens, O, GO, G, P>(
    inner: P,
    guard: G,
    expected: &'static str,
) -> BoxedParser<'tokens, O>
where
    O: 'tokens,
    GO: 'tokens,
    G: Parser<'tokens, ParserInput<'tokens>, GO, ParseExtra<'tokens>> + Clone + 'tokens,
    P: Parser<'tokens, ParserInput<'tokens>, O, ParseExtra<'tokens>> + Clone + 'tokens,
{
    custom::<_, ParserInput<'tokens>, _, ParseExtra<'tokens>>(move |input| {
        let before = input.save();
        let value = match input.parse(&inner) {
            Ok(value) => value,
            Err(error) => {
                input.rewind(before);
                return Err(error);
            }
        };
        let after_inner = input.save();
        let cursor = input.cursor();
        match input.parse(&guard) {
            Ok(_) => {
                let span = input.span_since(&cursor);
                input.rewind(before);
                Err(SyntaxParseError::expected_found(
                    span,
                    vec![new!(SyntaxExpectedToken::Named(expected.to_owned()))],
                    new!(SyntaxFound::EndOfInput),
                ))
            }
            Err(_) => {
                input.rewind(after_inner);
                Ok(value)
            }
        }
    })
    .boxed()
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn followed_by<'tokens, O, GO, G, P>(inner: P, guard: G) -> BoxedParser<'tokens, O>
where
    O: 'tokens,
    GO: 'tokens,
    G: Parser<'tokens, ParserInput<'tokens>, GO, ParseExtra<'tokens>> + Clone + 'tokens,
    P: Parser<'tokens, ParserInput<'tokens>, O, ParseExtra<'tokens>> + Clone + 'tokens,
{
    custom::<_, ParserInput<'tokens>, _, ParseExtra<'tokens>>(move |input| {
        let before = input.save();
        let value = match input.parse(&inner) {
            Ok(value) => value,
            Err(error) => {
                input.rewind(before);
                return Err(error);
            }
        };
        let after_inner = input.save();
        match input.parse(&guard) {
            Ok(_) => {
                input.rewind(after_inner);
                Ok(value)
            }
            Err(error) => {
                input.rewind(before);
                Err(error)
            }
        }
    })
    .boxed()
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn lookahead<'tokens, O, P>(parser: P) -> BoxedParser<'tokens, O>
where
    O: 'tokens,
    P: Parser<'tokens, ParserInput<'tokens>, O, ParseExtra<'tokens>> + Clone + 'tokens,
{
    custom::<_, ParserInput<'tokens>, _, ParseExtra<'tokens>>(move |input| {
        let before = input.save();
        let value = match input.parse(&parser) {
            Ok(value) => value,
            Err(error) => {
                input.rewind(before);
                return Err(error);
            }
        };
        input.rewind(before);
        Ok(value)
    })
    .boxed()
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn not<'tokens, O, P>(parser: P) -> BoxedParser<'tokens, ()>
where
    O: 'tokens,
    P: Parser<'tokens, ParserInput<'tokens>, O, ParseExtra<'tokens>> + Clone + 'tokens,
{
    custom::<_, ParserInput<'tokens>, _, ParseExtra<'tokens>>(move |input| {
        let before = input.save();
        let cursor = input.cursor();
        match input.parse(&parser) {
            Ok(_) => {
                let span = input.span_since(&cursor);
                input.rewind(before);
                Err(SyntaxParseError::expected_found(
                    span,
                    vec![new!(SyntaxExpectedToken::Named(
                        "negative predicate".to_owned()
                    ))],
                    new!(SyntaxFound::EndOfInput),
                ))
            }
            Err(_) => {
                input.rewind(before);
                Ok(())
            }
        }
    })
    .boxed()
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn word_category<'tokens>(category: SyntaxWordCategory) -> BoxedParser<'tokens, Token> {
    token_matching(
        category.display_name(),
        category.display_name(),
        vec![new!(SyntaxExpectedToken::WordCategory(category))],
        move |token, _state| token_matches_word_category(token, category),
    )
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn exact_word_category<'tokens>(
    category: SyntaxWordCategory,
) -> BoxedParser<'tokens, Token> {
    word_category(category)
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn quote_marker<'tokens>(marker: Cmavo) -> BoxedParser<'tokens, Token> {
    token_matching(
        "quote marker",
        marker.canonical_text(),
        vec![new!(SyntaxExpectedToken::Cmavo(marker))],
        move |token, _state| token.quote_marker_cmavo() == Some(marker),
    )
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn delimited_quote_marker<'tokens>(marker: Cmavo) -> BoxedParser<'tokens, Token> {
    token_matching(
        "delimited quote marker",
        marker.canonical_text(),
        vec![new!(SyntaxExpectedToken::Cmavo(marker))],
        move |token, _state| {
            matches!(
                token.core_word().as_data(),
                bityzba::data!(jbotci_morphology::WordLike::DelimitedNonLojbanQuote {
                    zoi,
                    ..
                }) if zoi.is_cmavo(marker)
            )
        },
    )
}

#[requires(!terminators.is_empty())]
#[ensures(true)]
pub(crate) fn raw_words_until<'tokens>(
    terminators: &'static [Cmavo],
) -> BoxedParser<'tokens, Vec<Token>> {
    token_matching(
        "replacement word",
        "REPLACEMENT WORD",
        vec![new!(SyntaxExpectedToken::WordCategory(
            SyntaxWordCategory::ReplacementWord,
        ))],
        move |token, _state| !token.is_one_of_cmavo(terminators),
    )
    .repeated()
    .collect::<Vec<_>>()
    .boxed()
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn tanru_unit_relation_word<'tokens>() -> BoxedParser<'tokens, Token> {
    let brivla = token_matching(
        SyntaxWordCategory::SelbriWord.display_name(),
        SyntaxWordCategory::SelbriWord.display_name(),
        vec![new!(SyntaxExpectedToken::WordCategory(
            SyntaxWordCategory::SelbriWord,
        ))],
        move |token, _state| is_brivla_relation_word(token),
    );
    let cbm_cmevla = feature_gate(
        SyntaxGrammarFeature::Cbm,
        cmevla_word().map_with(
            |word,
             extra: &mut chumsky::input::MapExtra<
                'tokens,
                '_,
                ParserInput<'tokens>,
                ParseExtra<'tokens>,
            >| {
                extra.state().warn(
                    ExperimentalConstruct::ExperimentalCbmCmevlaSelbriWord,
                    &word,
                );
                word
            },
        ),
    );
    brivla.or(cbm_cmevla).boxed()
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn text_leading_cmevla_word<'tokens>() -> BoxedParser<'tokens, Token> {
    custom::<_, ParserInput<'tokens>, _, ParseExtra<'tokens>>(move |input| {
        let checkpoint = input.save();
        if input.state().syntax_grammar_env().dialect.cbm_enabled {
            let cursor = input.cursor();
            let found = input
                .next()
                .map(|word| new!(SyntaxFound::Token(word)))
                .unwrap_or_else(|| new!(SyntaxFound::EndOfInput));
            let span = input.span_since(&cursor);
            input.rewind(checkpoint);
            return Err(SyntaxParseError::expected_found(
                span,
                vec![new!(SyntaxExpectedToken::Named(
                    "non-CBM leading CMEVLA".to_owned()
                ))],
                found,
            ));
        }

        let cursor = input.cursor();
        match input.next() {
            Some(word) if is_cmevla_word(&word) => Ok(word),
            Some(word) => {
                let span = input.span_since(&cursor);
                input.rewind(checkpoint);
                Err(SyntaxParseError::expected_found(
                    span,
                    vec![new!(SyntaxExpectedToken::WordCategory(
                        SyntaxWordCategory::Cmevla,
                    ))],
                    new!(SyntaxFound::Token(word)),
                ))
            }
            None => {
                let span = input.span_since(&cursor);
                input.rewind(checkpoint);
                Err(SyntaxParseError::expected_found(
                    span,
                    vec![new!(SyntaxExpectedToken::WordCategory(
                        SyntaxWordCategory::Cmevla,
                    ))],
                    new!(SyntaxFound::EndOfInput),
                ))
            }
        }
    })
    .boxed()
}

#[requires(true)]
#[ensures(true)]
fn token_matches_word_category(token: &Token, category: SyntaxWordCategory) -> bool {
    match category {
        SyntaxWordCategory::Brivla => is_brivla_relation_word(token),
        SyntaxWordCategory::Cmevla => is_cmevla_word(token),
        SyntaxWordCategory::SelbriWord => is_relation_word(token),
        SyntaxWordCategory::ProSumti => is_koha_argument(token),
        SyntaxWordCategory::LetterWord => is_letter_word(token),
        SyntaxWordCategory::Quote => token_is_compound_quote(token),
        SyntaxWordCategory::ReplacementWord => false,
    }
}

#[requires(true)]
#[ensures(true)]
fn token_is_compound_quote(token: &Token) -> bool {
    matches!(
        token.core_word().as_data(),
        bityzba::data!(jbotci_morphology::WordLike::QuotedWord { .. })
            | bityzba::data!(jbotci_morphology::WordLike::DelimitedWordQuote { .. })
            | bityzba::data!(jbotci_morphology::WordLike::DelimitedNonLojbanQuote { .. })
            | bityzba::data!(jbotci_morphology::WordLike::QuotedWords { .. })
    )
}
