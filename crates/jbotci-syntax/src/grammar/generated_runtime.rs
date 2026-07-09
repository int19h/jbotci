//! Generic runtime primitives for declarative generated syntax parsers.

use bityzba::{contract_trait, invariant, new, requires};
use chumsky::{Parser, input::Input, primitive::custom};
use jbotci_diagnostics::{TraceEventKind, TraceLevel};
use jbotci_dialect::DialectFeature;
use jbotci_morphology::{Cmavo, Selmaho};
use std::cell::Cell;

use super::{
    BoxedParser, ParseExtra, ParserInput, Span, SyntaxFound, SyntaxFoundData, SyntaxParseError,
    tokens::{
        ExperimentalCmavoContext, cmevla_word, is_brivla_relation_word, is_cmevla_word,
        is_koha_argument, is_letter_word, is_relation_word, token_matching,
        token_matching_with_experimental_context,
    },
};
use crate::{
    ExperimentalConstruct, ParseOptions, SyntaxExpectedToken, SyntaxExpectedTokenData,
    SyntaxWordCategory, Token,
    tree::{SyntaxRecoveryItem, WithFreeModifiers},
};

#[invariant(!words.is_empty(), "vocative marker sequence cannot be empty")]
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct VocativeMarkerWordsSyntax {
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
    pub zantufa_adverbials_enabled: bool,
    pub zantufa_connectives_enabled: bool,
    pub zantufa_mex_enabled: bool,
    pub zantufa_quotes_enabled: bool,
    pub zantufa_tags_enabled: bool,
    pub zantufa_terms_enabled: bool,
}

impl SyntaxGrammarDialect {
    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn from_options(options: &ParseOptions) -> Self {
        let features = &options.dialect.features;
        Self {
            term_hierarchy_enabled: features.contains(&DialectFeature::TermHierarchy),
            cbm_enabled: features.contains(&DialectFeature::Cbm),
            zantufa_adverbials_enabled: features.contains(&DialectFeature::ZantufaAdverbials),
            zantufa_connectives_enabled: features.contains(&DialectFeature::ZantufaConnectives),
            zantufa_mex_enabled: features.contains(&DialectFeature::ZantufaMex),
            zantufa_quotes_enabled: features.contains(&DialectFeature::ZantufaQuotes),
            zantufa_tags_enabled: features.contains(&DialectFeature::ZantufaTags),
            zantufa_terms_enabled: features.contains(&DialectFeature::ZantufaTerms),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
#[allow(dead_code)]
pub(crate) enum SyntaxGrammarFeature {
    TermHierarchy,
    Cbm,
    ZantufaAdverbials,
    ZantufaConnectives,
    ZantufaMex,
    ZantufaQuotes,
    ZantufaTags,
    ZantufaTerms,
}

impl SyntaxGrammarFeature {
    #[requires(true)]
    #[ensures(true)]
    fn enabled(self, dialect: SyntaxGrammarDialect) -> bool {
        match self {
            Self::TermHierarchy => dialect.term_hierarchy_enabled,
            Self::Cbm => dialect.cbm_enabled,
            Self::ZantufaAdverbials => dialect.zantufa_adverbials_enabled,
            Self::ZantufaConnectives => dialect.zantufa_connectives_enabled,
            Self::ZantufaMex => dialect.zantufa_mex_enabled,
            Self::ZantufaQuotes => dialect.zantufa_quotes_enabled,
            Self::ZantufaTags => dialect.zantufa_tags_enabled,
            Self::ZantufaTerms => dialect.zantufa_terms_enabled,
        }
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn expected_name(self) -> &'static str {
        match self {
            Self::TermHierarchy => "TERM-HIERARCHY feature",
            Self::Cbm => "CBM feature",
            Self::ZantufaAdverbials => "ZANTUFA-ADVERBIALS feature",
            Self::ZantufaConnectives => "ZANTUFA-CONNECTIVES feature",
            Self::ZantufaMex => "ZANTUFA-MEX feature",
            Self::ZantufaQuotes => "ZANTUFA-QUOTES feature",
            Self::ZantufaTags => "ZANTUFA-TAGS feature",
            Self::ZantufaTerms => "ZANTUFA-TERMS feature",
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
#[requires(context.is_none_or(|construct| !construct.is_empty()))]
#[ensures(true)]
pub(crate) fn rule_wrapper<'tokens, O, P>(
    name: &'static str,
    context: Option<&'static str>,
    parser: P,
) -> BoxedParser<'tokens, O>
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
        if let Some(error) = input.state().syntax_memo_failure(name, start_location) {
            input.rewind(checkpoint);
            return Err(error);
        }
        if !input.state().enter_syntax_memo_rule(name, start_location) {
            input.rewind(checkpoint);
            return Err(expected_found_named_at_current(input, name.to_owned()));
        }
        let warning_start = input.state().warning_count();
        let start_byte = input.state().byte_offset_for_location(start_location);
        input.state().push_syntax_rule(name, start_byte);
        if let Some(construct) = context {
            if input
                .state()
                .trace_should_record(TraceLevel::Top, construct)
            {
                input
                    .state()
                    .trace_enter_construct(TraceLevel::Top, construct, 0, 0);
            }
            input.state().push_syntax_context(construct, start_byte);
        }
        let failure_span = Cell::new(None);
        let parse_result = if context.is_some() {
            let parser = parser
                .clone()
                .map_err_with_state(|error, span: Span, _state| {
                    failure_span.set(Some(span));
                    error
                });
            input.parse(parser)
        } else {
            input.parse(&parser)
        };
        match parse_result {
            Ok(output) => {
                if let Some(construct) = context {
                    let span = input.span_since(checkpoint.cursor());
                    trace_rule_exit(input, construct, TraceEventKind::ConstructSuccess, span);
                    input.state().pop_syntax_context();
                }
                let end_location = ParserInput::cursor_location(input.cursor().inner());
                let warnings = input.state().warnings_since(warning_start);
                input.state().store_syntax_memo_success(
                    name,
                    start_location,
                    end_location,
                    output.clone(),
                    warnings,
                );
                input.state().pop_syntax_rule();
                input.state().exit_syntax_memo_rule(name, start_location);
                Ok(output)
            }
            Err(error) => {
                let error = if let Some(construct) = context {
                    let failure_location = ParserInput::cursor_location(input.cursor().inner());
                    let span = failure_span.get().unwrap_or(*error.span());
                    trace_rule_exit(input, construct, TraceEventKind::ConstructFailure, span);
                    let error = error.with_rule_context_from_progress(
                        construct,
                        start_byte,
                        failure_location > start_location,
                    );
                    let error = error
                        .with_active_contexts(input.state().active_syntax_contexts())
                        .with_active_rule_contexts(input.state().active_syntax_rules());
                    input.state().pop_syntax_context();
                    error
                } else {
                    error
                };
                input.state().pop_syntax_rule();
                input.rewind(checkpoint);
                input
                    .state()
                    .store_syntax_memo_failure(name, start_location, error.clone());
                input.state().exit_syntax_memo_rule(name, start_location);
                Err(error)
            }
        }
    })
    .boxed()
}

#[requires(!construct.is_empty())]
#[ensures(true)]
fn trace_rule_exit<'tokens>(
    input: &mut chumsky::input::InputRef<'tokens, '_, ParserInput<'tokens>, ParseExtra<'tokens>>,
    construct: &'static str,
    kind: TraceEventKind,
    span: Span,
) {
    let byte_start = span.start.min(span.end);
    let byte_end = span.start.max(span.end);
    input.state().trace_exit_construct(
        TraceLevel::Top,
        kind,
        construct,
        byte_start,
        byte_end,
        || None,
    );
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
    syntax_gate(
        parser,
        move |env| feature.enabled(env.dialect),
        feature.expected_name(),
    )
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
    syntax_gate(
        parser,
        move |env| policy.enabled(env.policy),
        policy.expected_name(),
    )
}

#[requires(!expected.is_empty())]
#[ensures(true)]
fn syntax_gate<'tokens, O, P, E>(
    parser: P,
    enabled: E,
    expected: &'static str,
) -> BoxedParser<'tokens, O>
where
    O: 'tokens,
    E: Fn(SyntaxGrammarEnv) -> bool + Clone + 'tokens,
    P: Parser<'tokens, ParserInput<'tokens>, O, ParseExtra<'tokens>> + Clone + 'tokens,
{
    custom::<_, ParserInput<'tokens>, _, ParseExtra<'tokens>>(move |input| {
        let env = input.state().syntax_grammar_env();
        if enabled(env) {
            return input.parse(&parser);
        }

        Err(expected_found_named_at_current(input, expected.to_owned()))
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
    custom::<_, ParserInput<'tokens>, _, ParseExtra<'tokens>>(move |input| {
        let checkpoint = input.save();
        match input.parse(&parser) {
            Ok(output) => Ok(Some(output)),
            Err(error) => {
                input.rewind(checkpoint);
                input.state().record_diagnostic_candidate(error);
                Ok(None)
            }
        }
    })
    .boxed()
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn strict_greedy_many_parser<'tokens, O: 'tokens>(
    parser: BoxedParser<'tokens, O>,
) -> BoxedParser<'tokens, Vec<O>> {
    custom::<_, ParserInput<'tokens>, _, ParseExtra<'tokens>>(move |input| {
        let mut values = Vec::new();
        loop {
            let checkpoint = input.save();
            let start_location = ParserInput::cursor_location(checkpoint.cursor().inner());
            match input.parse(&parser) {
                Ok(output) => {
                    let end_location = ParserInput::cursor_location(input.cursor().inner());
                    if end_location == start_location {
                        debug_assert!(false, "generated repetition parser accepted empty input");
                        input.rewind(checkpoint);
                        break;
                    }
                    values.push(output);
                }
                Err(error) => {
                    input.rewind(checkpoint);
                    input.state().record_diagnostic_candidate(error);
                    break;
                }
            }
        }
        Ok(values)
    })
    .boxed()
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
    custom::<_, ParserInput<'tokens>, _, ParseExtra<'tokens>>(move |input| {
        let first_checkpoint = input.save();
        let first_start_location = ParserInput::cursor_location(first_checkpoint.cursor().inner());
        let first = match input.parse(&parser) {
            Ok(output) => {
                let first_end_location = ParserInput::cursor_location(input.cursor().inner());
                if first_end_location == first_start_location {
                    debug_assert!(
                        false,
                        "generated non-empty repetition parser accepted empty input"
                    );
                    input.rewind(first_checkpoint);
                    return Ok(Vec::new());
                }
                output
            }
            Err(error) => {
                input.rewind(first_checkpoint);
                return Err(error);
            }
        };

        let mut values = vec![first];
        loop {
            let checkpoint = input.save();
            let start_location = ParserInput::cursor_location(checkpoint.cursor().inner());
            match input.parse(&parser) {
                Ok(output) => {
                    let end_location = ParserInput::cursor_location(input.cursor().inner());
                    if end_location == start_location {
                        debug_assert!(
                            false,
                            "generated non-empty repetition parser accepted empty input"
                        );
                        input.rewind(checkpoint);
                        break;
                    }
                    values.push(output);
                }
                Err(error) => {
                    input.rewind(checkpoint);
                    input.state().record_diagnostic_candidate(error);
                    break;
                }
            }
        }
        Ok(values)
    })
    .boxed()
}

#[requires(!alternatives.is_empty())]
#[ensures(true)]
pub(crate) fn strict_ordered_choice_parsers<'tokens, O: 'tokens>(
    alternatives: Vec<BoxedParser<'tokens, O>>,
) -> BoxedParser<'tokens, O> {
    custom::<_, ParserInput<'tokens>, _, ParseExtra<'tokens>>(move |input| {
        let mut abandoned_error = None;
        for alternative in &alternatives {
            let checkpoint = input.save();
            match input.parse(alternative) {
                Ok(output) => {
                    if let Some(error) = abandoned_error {
                        input.state().record_diagnostic_candidate(error);
                    }
                    return Ok(output);
                }
                Err(error) => {
                    input.rewind(checkpoint);
                    abandoned_error = Some(match abandoned_error {
                        None => error,
                        Some(previous) => merge_choice_errors(previous, error),
                    });
                }
            }
        }
        Err(abandoned_error.expect("ordered choice has at least one alternative"))
    })
    .boxed()
}

#[requires(true)]
#[ensures(true)]
fn merge_choice_errors<'tokens>(
    previous: SyntaxParseError<'tokens>,
    error: SyntaxParseError<'tokens>,
) -> SyntaxParseError<'tokens> {
    match error.span().start.cmp(&previous.span().start) {
        std::cmp::Ordering::Greater => error,
        std::cmp::Ordering::Less => previous,
        std::cmp::Ordering::Equal if previous.same_report_content(&error) => previous,
        std::cmp::Ordering::Equal => previous.merge_for_parser(error),
    }
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn strict_free_modifier_list_parser<'tokens, F>(
    free_modifier: BoxedParser<'tokens, F>,
) -> BoxedParser<'tokens, Vec<F>>
where
    F: 'tokens,
{
    strict_greedy_many_parser_without_diagnostics(free_modifier)
}

#[contract_trait]
pub(crate) trait RecoveredSyntaxSlot: Sized {
    #[requires(true)]
    #[ensures(true)]
    fn from_recovery_item(item: SyntaxRecoveryItem) -> Self;

    #[requires(true)]
    #[ensures(true)]
    fn prepend_recovery_item(self, item: SyntaxRecoveryItem) -> Self;
}

#[contract_trait]
impl<T> RecoveredSyntaxSlot for jbotci_tree::Recovered<T, SyntaxRecoveryItem> {
    #[requires(true)]
    #[ensures(true)]
    fn from_recovery_item(item: SyntaxRecoveryItem) -> Self {
        Self::error(item)
    }

    #[requires(true)]
    #[ensures(true)]
    fn prepend_recovery_item(self, item: SyntaxRecoveryItem) -> Self {
        match self {
            Self::Valid(value) => Self::prefix_boxed(vec![item], value),
            Self::Prefix(jbotci_tree::RecoveredPrefix { errors, value }) => {
                let mut errors = errors.into_vec();
                errors.insert(0, item);
                Self::prefix_boxed(errors, value)
            }
            Self::Error(existing) => Self::error(existing),
        }
    }
}

#[contract_trait]
trait RecoveredSyntaxRequiredSlot: RecoveredSyntaxSlot {}

impl<T> RecoveredSyntaxRequiredSlot for jbotci_tree::Recovered<T, SyntaxRecoveryItem> {}

#[contract_trait]
impl<T> RecoveredSyntaxSlot for Vec<T>
where
    T: RecoveredSyntaxSlot,
{
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn from_recovery_item(item: SyntaxRecoveryItem) -> Self {
        vec![T::from_recovery_item(item)]
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn prepend_recovery_item(mut self, item: SyntaxRecoveryItem) -> Self {
        if let Some(first) = self.first_mut() {
            let placeholder = T::from_recovery_item(item.clone());
            let previous = std::mem::replace(first, placeholder);
            *first = previous.prepend_recovery_item(item);
        } else {
            self.push(T::from_recovery_item(item));
        }
        self
    }
}

impl<T> RecoveredSyntaxRequiredSlot for Vec<T> where T: RecoveredSyntaxSlot {}

#[contract_trait]
impl<T> RecoveredSyntaxSlot for vec1::Vec1<T>
where
    T: RecoveredSyntaxSlot,
{
    #[requires(true)]
    #[ensures(true)]
    fn from_recovery_item(item: SyntaxRecoveryItem) -> Self {
        vec1::Vec1::new(T::from_recovery_item(item))
    }

    #[requires(true)]
    #[ensures(true)]
    fn prepend_recovery_item(self, item: SyntaxRecoveryItem) -> Self {
        let mut values = self.into_vec();
        let first = values
            .first_mut()
            .expect("Vec1 contains at least one value");
        let placeholder = T::from_recovery_item(item.clone());
        let previous = std::mem::replace(first, placeholder);
        *first = previous.prepend_recovery_item(item);
        vec1::Vec1::try_from_vec(values).expect("Vec1 recovery prefix preserves non-empty vector")
    }
}

impl<T> RecoveredSyntaxRequiredSlot for vec1::Vec1<T> where T: RecoveredSyntaxSlot {}

#[contract_trait]
impl<A> RecoveredSyntaxSlot for smallvec::SmallVec<A>
where
    A: smallvec::Array,
    A::Item: RecoveredSyntaxSlot,
{
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn from_recovery_item(item: SyntaxRecoveryItem) -> Self {
        smallvec::SmallVec::from_vec(vec![A::Item::from_recovery_item(item)])
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn prepend_recovery_item(mut self, item: SyntaxRecoveryItem) -> Self {
        if let Some(first) = self.first_mut() {
            let placeholder = A::Item::from_recovery_item(item.clone());
            let previous = std::mem::replace(first, placeholder);
            *first = previous.prepend_recovery_item(item);
        } else {
            self.push(A::Item::from_recovery_item(item));
        }
        self
    }
}

impl<A> RecoveredSyntaxRequiredSlot for smallvec::SmallVec<A>
where
    A: smallvec::Array,
    A::Item: RecoveredSyntaxSlot,
{
}

#[contract_trait]
impl<A> RecoveredSyntaxSlot for vec1::smallvec_v1::SmallVec1<A>
where
    A: smallvec::Array,
    A::Item: RecoveredSyntaxSlot,
{
    #[requires(true)]
    #[ensures(true)]
    fn from_recovery_item(item: SyntaxRecoveryItem) -> Self {
        vec1::smallvec_v1::SmallVec1::try_from_vec(vec![A::Item::from_recovery_item(item)])
            .expect("one recovery item creates non-empty SmallVec1")
    }

    #[requires(true)]
    #[ensures(true)]
    fn prepend_recovery_item(self, item: SyntaxRecoveryItem) -> Self {
        let mut values = self.into_vec();
        let first = values
            .first_mut()
            .expect("SmallVec1 contains at least one value");
        let placeholder = A::Item::from_recovery_item(item.clone());
        let previous = std::mem::replace(first, placeholder);
        *first = previous.prepend_recovery_item(item);
        vec1::smallvec_v1::SmallVec1::try_from_vec(values)
            .expect("SmallVec1 recovery prefix preserves non-empty vector")
    }
}

impl<A> RecoveredSyntaxRequiredSlot for vec1::smallvec_v1::SmallVec1<A>
where
    A: smallvec::Array,
    A::Item: RecoveredSyntaxSlot,
{
}

#[contract_trait]
impl<T> RecoveredSyntaxSlot for Option<T>
where
    T: RecoveredSyntaxSlot,
{
    #[requires(true)]
    #[ensures(ret.is_some())]
    fn from_recovery_item(item: SyntaxRecoveryItem) -> Self {
        Some(T::from_recovery_item(item))
    }

    #[requires(true)]
    #[ensures(ret.is_some())]
    fn prepend_recovery_item(self, item: SyntaxRecoveryItem) -> Self {
        Some(match self {
            Some(value) => value.prepend_recovery_item(item),
            None => T::from_recovery_item(item),
        })
    }
}

#[contract_trait]
impl<T> RecoveredSyntaxSlot for Box<T>
where
    T: RecoveredSyntaxSlot,
{
    #[requires(true)]
    #[ensures(true)]
    fn from_recovery_item(item: SyntaxRecoveryItem) -> Self {
        Box::new(T::from_recovery_item(item))
    }

    #[requires(true)]
    #[ensures(true)]
    fn prepend_recovery_item(self, item: SyntaxRecoveryItem) -> Self {
        Box::new((*self).prepend_recovery_item(item))
    }
}

impl<T> RecoveredSyntaxRequiredSlot for Box<T> where T: RecoveredSyntaxSlot {}

#[contract_trait]
impl<T> RecoveredSyntaxSlot for std::sync::Arc<T>
where
    T: Clone + RecoveredSyntaxSlot,
{
    #[requires(true)]
    #[ensures(true)]
    fn from_recovery_item(item: SyntaxRecoveryItem) -> Self {
        std::sync::Arc::new(T::from_recovery_item(item))
    }

    #[requires(true)]
    #[ensures(true)]
    fn prepend_recovery_item(self, item: SyntaxRecoveryItem) -> Self {
        let value = std::sync::Arc::try_unwrap(self).unwrap_or_else(|value| (*value).clone());
        std::sync::Arc::new(value.prepend_recovery_item(item))
    }
}

impl<T> RecoveredSyntaxRequiredSlot for std::sync::Arc<T> where T: Clone + RecoveredSyntaxSlot {}

#[contract_trait]
impl<T, F> RecoveredSyntaxSlot for WithFreeModifiers<T, F>
where
    T: RecoveredSyntaxSlot,
{
    #[requires(true)]
    #[ensures(true)]
    fn from_recovery_item(item: SyntaxRecoveryItem) -> Self {
        Self {
            value: T::from_recovery_item(item),
            free_modifiers: Vec::new(),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn prepend_recovery_item(self, item: SyntaxRecoveryItem) -> Self {
        Self {
            value: self.value.prepend_recovery_item(item),
            free_modifiers: self.free_modifiers,
        }
    }
}

impl<T, F> RecoveredSyntaxRequiredSlot for WithFreeModifiers<T, F> where T: RecoveredSyntaxSlot {}

#[contract_trait]
impl<T> RecoveredSyntaxSlot for super::generated_model::recovered::WithFreeModifiers<T>
where
    T: RecoveredSyntaxSlot,
{
    #[requires(true)]
    #[ensures(true)]
    fn from_recovery_item(item: SyntaxRecoveryItem) -> Self {
        Self {
            value: T::from_recovery_item(item),
            free_modifiers: Vec::new(),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn prepend_recovery_item(self, item: SyntaxRecoveryItem) -> Self {
        Self {
            value: self.value.prepend_recovery_item(item),
            free_modifiers: self.free_modifiers,
        }
    }
}

impl<T> RecoveredSyntaxRequiredSlot for super::generated_model::recovered::WithFreeModifiers<T> where
    T: RecoveredSyntaxSlot
{
}

#[contract_trait]
impl<E, Links> RecoveredSyntaxSlot for jbotci_tree::Chain<E, Links>
where
    E: RecoveredSyntaxSlot,
    Links: Default,
{
    #[requires(true)]
    #[ensures(true)]
    fn from_recovery_item(item: SyntaxRecoveryItem) -> Self {
        Self {
            first: E::from_recovery_item(item),
            links: Links::default(),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn prepend_recovery_item(self, item: SyntaxRecoveryItem) -> Self {
        Self {
            first: self.first.prepend_recovery_item(item),
            links: self.links,
        }
    }
}

impl<E, Links> RecoveredSyntaxRequiredSlot for jbotci_tree::Chain<E, Links>
where
    E: RecoveredSyntaxSlot,
    Links: Default,
{
}

#[contract_trait]
impl<A, B> RecoveredSyntaxSlot for (Option<A>, B)
where
    A: RecoveredSyntaxSlot,
    B: RecoveredSyntaxSlot,
{
    #[requires(true)]
    #[ensures(true)]
    fn from_recovery_item(item: SyntaxRecoveryItem) -> Self {
        (None, B::from_recovery_item(item))
    }

    #[requires(true)]
    #[ensures(true)]
    fn prepend_recovery_item(self, item: SyntaxRecoveryItem) -> Self {
        match self.0 {
            Some(first) => (Some(first.prepend_recovery_item(item)), self.1),
            None => (None, self.1.prepend_recovery_item(item)),
        }
    }
}

#[contract_trait]
impl<A, B> RecoveredSyntaxSlot for (A, Option<B>)
where
    A: RecoveredSyntaxRequiredSlot,
    B: RecoveredSyntaxSlot,
{
    #[requires(true)]
    #[ensures(true)]
    fn from_recovery_item(item: SyntaxRecoveryItem) -> Self {
        (A::from_recovery_item(item), None)
    }

    #[requires(true)]
    #[ensures(true)]
    fn prepend_recovery_item(self, item: SyntaxRecoveryItem) -> Self {
        (self.0.prepend_recovery_item(item), self.1)
    }
}

#[requires(!rule.is_empty())]
#[ensures(true)]
pub(crate) fn recovered_field_parser<'tokens, O, P>(
    rule: &'static str,
    field_index: usize,
    parser: P,
) -> BoxedParser<'tokens, O>
where
    O: RecoveredSyntaxSlot + 'tokens,
    P: Parser<'tokens, ParserInput<'tokens>, O, ParseExtra<'tokens>> + Clone + 'tokens,
{
    custom::<_, ParserInput<'tokens>, _, ParseExtra<'tokens>>(move |input| {
        let checkpoint = input.save();
        let location = ParserInput::cursor_location(checkpoint.cursor().inner());
        let active_frame = input
            .state()
            .active_syntax_rules()
            .iter()
            .rev()
            .find(|frame| frame.rule() == rule);
        let instance_byte_start = match active_frame {
            Some(frame) => frame.byte_start(),
            None => input.state().byte_offset_for_location(location),
        };
        let directive = input.state().consume_recovery_directive(
            rule,
            instance_byte_start,
            field_index,
            location,
        );
        let item = directive.map(|directive| {
            advance_to_location(input, directive.resume_token_index);
            input.state().recovery_item_for_directive(&directive)
        });
        let value = input.parse(&parser)?;
        match item {
            Some(item) => Ok(value.prepend_recovery_item(item)),
            None => Ok(value),
        }
    })
    .boxed()
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn recovered_free_modifier_list_parser<'tokens, F>(
    free_modifier: BoxedParser<'tokens, F>,
) -> BoxedParser<'tokens, Vec<jbotci_tree::Recovered<F, crate::tree::SyntaxRecoveryItem>>>
where
    F: 'tokens,
{
    strict_free_modifier_list_parser(free_modifier)
        .map(|free_modifiers| {
            free_modifiers
                .into_iter()
                .map(jbotci_tree::Recovered::valid)
                .collect()
        })
        .boxed()
}

#[contract_trait]
pub(crate) trait SyntaxFirstWord {
    #[requires(true)]
    #[ensures(true)]
    fn first_word(&self) -> Option<&Token>;
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn strict_cll_prohibited_free_modifier_list_parser<'tokens, F>(
    free_modifier: BoxedParser<'tokens, F>,
) -> BoxedParser<'tokens, Vec<F>>
where
    F: SyntaxFirstWord + 'tokens,
{
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
pub(crate) fn recovered_cll_prohibited_free_modifier_list_parser<'tokens, F>(
    free_modifier: BoxedParser<'tokens, F>,
) -> BoxedParser<'tokens, Vec<jbotci_tree::Recovered<F, crate::tree::SyntaxRecoveryItem>>>
where
    F: SyntaxFirstWord + 'tokens,
{
    strict_cll_prohibited_free_modifier_list_parser(free_modifier)
        .map(|free_modifiers| {
            free_modifiers
                .into_iter()
                .map(jbotci_tree::Recovered::valid)
                .collect()
        })
        .boxed()
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn with_free_modifier_list<'tokens, O, F, P>(
    inner: P,
    free_modifier_list: BoxedParser<'tokens, Vec<F>>,
) -> BoxedParser<'tokens, WithFreeModifiers<O, F>>
where
    O: 'tokens,
    F: 'tokens,
    P: Parser<'tokens, ParserInput<'tokens>, O, ParseExtra<'tokens>> + Clone + 'tokens,
{
    custom::<_, ParserInput<'tokens>, _, ParseExtra<'tokens>>(move |input| {
        let value = input.parse(&inner)?;
        let free_modifiers = input.parse(&free_modifier_list)?;
        Ok(WithFreeModifiers::new(value, free_modifiers))
    })
    .boxed()
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn strict_empty_free_modifier_parser<'tokens, F>() -> BoxedParser<'tokens, F>
where
    F: 'tokens,
{
    custom::<_, ParserInput<'tokens>, _, ParseExtra<'tokens>>(move |input| {
        Err(expected_found_at_current(input, "free modifier"))
    })
    .boxed()
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn recovered_empty_free_modifier_parser<'tokens, F>() -> BoxedParser<'tokens, F>
where
    F: 'tokens,
{
    strict_empty_free_modifier_parser()
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn not_next_selmaho<'tokens>(selmaho: Selmaho) -> BoxedParser<'tokens, ()> {
    custom::<_, ParserInput<'tokens>, _, ParseExtra<'tokens>>(move |input| {
        let checkpoint = input.save();
        match input.next() {
            Some(token) if token.is_selmaho(selmaho) => {
                input.rewind(checkpoint);
                Err(expected_found_named_at_current(
                    input,
                    format!("not {}", selmaho.name()),
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

#[requires(!expected.is_empty())]
#[ensures(true)]
pub(crate) fn complete_statement_item<'tokens, O, P>(
    inner: P,
    expected: &'static str,
) -> BoxedParser<'tokens, O>
where
    O: 'tokens,
    P: Parser<'tokens, ParserInput<'tokens>, O, ParseExtra<'tokens>> + Clone + 'tokens,
{
    complete_before_boundary(inner, expected, |token| {
        token.is_none_or(|token| token.is_selmaho(Selmaho::I) || token.is_selmaho(Selmaho::Niho))
    })
}

#[requires(!expected.is_empty())]
#[ensures(true)]
pub(crate) fn complete_before_selmaho<'tokens, O, P>(
    inner: P,
    selmaho: Selmaho,
    expected: &'static str,
) -> BoxedParser<'tokens, O>
where
    O: 'tokens,
    P: Parser<'tokens, ParserInput<'tokens>, O, ParseExtra<'tokens>> + Clone + 'tokens,
{
    complete_before_boundary(inner, expected, move |token| {
        token.is_some_and(|token| token.is_selmaho(selmaho))
    })
}

#[requires(!expected.is_empty())]
#[ensures(true)]
fn complete_before_boundary<'tokens, O, P, B>(
    inner: P,
    expected: &'static str,
    is_boundary: B,
) -> BoxedParser<'tokens, O>
where
    O: 'tokens,
    B: Fn(Option<&Token>) -> bool + Clone + 'tokens,
    P: Parser<'tokens, ParserInput<'tokens>, O, ParseExtra<'tokens>> + Clone + 'tokens,
{
    custom::<_, ParserInput<'tokens>, _, ParseExtra<'tokens>>(move |input| {
        let before = input.save();
        let start_location = ParserInput::cursor_location(before.cursor().inner());
        let start_byte = input.state().byte_offset_for_location(start_location);
        let diagnostic_snapshot = input.state().diagnostic_candidates_snapshot();
        match input.parse(&inner) {
            Ok(value) => {
                let after_inner = input.save();
                let next = input.next();
                let at_boundary = is_boundary(next.as_ref());
                input.rewind(after_inner);
                if at_boundary {
                    Ok(value)
                } else {
                    input.rewind(before);
                    input
                        .state()
                        .restore_diagnostic_candidates(diagnostic_snapshot);
                    Err(expected_found_at_current(input, expected))
                }
            }
            Err(error) => {
                input.rewind(before);
                input
                    .state()
                    .restore_diagnostic_candidates_preserving_start(
                        diagnostic_snapshot,
                        start_byte,
                    );
                if error.span().start == start_byte {
                    Err(error)
                } else {
                    Err(expected_found_at_current(input, expected))
                }
            }
        }
    })
    .boxed()
}

#[requires(!expected.is_empty())]
#[ensures(true)]
fn expected_found_at_current<'tokens>(
    input: &mut chumsky::input::InputRef<'tokens, '_, ParserInput<'tokens>, ParseExtra<'tokens>>,
    expected: &'static str,
) -> SyntaxParseError<'tokens> {
    expected_found_named_at_current(input, expected.to_owned())
}

#[requires(!expected.is_empty())]
#[ensures(true)]
fn expected_found_named_at_current<'tokens>(
    input: &mut chumsky::input::InputRef<'tokens, '_, ParserInput<'tokens>, ParseExtra<'tokens>>,
    expected: String,
) -> SyntaxParseError<'tokens> {
    expected_found_tokens_at_current(input, vec![new!(SyntaxExpectedToken::Named(expected))])
}

#[requires(!expected.is_empty())]
#[ensures(true)]
fn expected_found_tokens_at_current<'tokens>(
    input: &mut chumsky::input::InputRef<'tokens, '_, ParserInput<'tokens>, ParseExtra<'tokens>>,
    expected: Vec<SyntaxExpectedToken>,
) -> SyntaxParseError<'tokens> {
    let checkpoint = input.save();
    let cursor = input.cursor();
    let found = input
        .next()
        .map(|token| new!(SyntaxFound::Token(token)))
        .unwrap_or_else(|| new!(SyntaxFound::EndOfInput));
    let span = input.span_since(&cursor);
    input.rewind(checkpoint);
    SyntaxParseError::expected_found(span, expected, found)
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
    token_matching_with_experimental_context(
        category.display_name(),
        category.display_name(),
        vec![new!(SyntaxExpectedToken::WordCategory(category))],
        word_category_experimental_context(category),
        move |token, _state| token_matches_word_category(token, category),
    )
}

#[requires(true)]
#[ensures(true)]
fn word_category_experimental_context(category: SyntaxWordCategory) -> ExperimentalCmavoContext {
    match category {
        SyntaxWordCategory::ProSumti => ExperimentalCmavoContext::Selmaho(Selmaho::Koha),
        SyntaxWordCategory::LetterWord => ExperimentalCmavoContext::Selmaho(Selmaho::By),
        _ => ExperimentalCmavoContext::Label(category.display_name()),
    }
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
pub(crate) fn word_not_cmavo<'tokens>(
    terminators: &'static [Cmavo],
) -> BoxedParser<'tokens, Token> {
    token_matching(
        "word other than terminator cmavo",
        "WORD",
        vec![new!(SyntaxExpectedToken::Named(
            "non-terminator word".to_owned()
        ))],
        move |token, _state| !token.is_one_of_cmavo(terminators),
    )
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
            input.rewind(checkpoint);
            return Err(expected_found_at_current(input, "non-CBM leading CMEVLA"));
        }

        match input.next() {
            Some(word) if is_cmevla_word(&word) => Ok(word),
            Some(_) | None => {
                input.rewind(checkpoint);
                Err(expected_found_tokens_at_current(
                    input,
                    vec![new!(SyntaxExpectedToken::WordCategory(
                        SyntaxWordCategory::Cmevla,
                    ))],
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
