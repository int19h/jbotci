//! Generic runtime primitives for declarative generated syntax parsers.

use bityzba::{contract_trait, invariant, new, requires};
use jbotci_diagnostics::{TraceEventKind, TraceLevel};
use jbotci_dialect::DialectFeature;
use jbotci_morphology::{Cmavo, Selmaho};
use std::{any::Any, cell::Cell, rc::Rc};

pub(crate) use super::parser_core::SharedSyntaxOutput;
use super::{
    BoxedParser, ParserInput, RecoveryCheckpointKind, Span, SyntaxFound, SyntaxFoundData,
    SyntaxParseError,
    parser_core::{InputRef, MapExtra, Parser, custom, empty as parser_empty, end as parser_end},
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
    pub unrestricted_free_enabled: bool,
    pub zantufa_adverbials_enabled: bool,
    pub zantufa_connectives_enabled: bool,
    pub zantufa_mex_enabled: bool,
    pub zantufa_mex_reinterpretation_enabled: bool,
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
            unrestricted_free_enabled: features.contains(&DialectFeature::UnrestrictedFree),
            zantufa_adverbials_enabled: features.contains(&DialectFeature::ZantufaAdverbials),
            zantufa_connectives_enabled: features.contains(&DialectFeature::ZantufaConnectives),
            zantufa_mex_enabled: features.contains(&DialectFeature::ZantufaMex),
            zantufa_mex_reinterpretation_enabled: features
                .contains(&DialectFeature::ZantufaMexReinterpretation),
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
    UnrestrictedFree,
    ZantufaAdverbials,
    ZantufaConnectives,
    ZantufaMex,
    ZantufaMexReinterpretation,
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
            Self::UnrestrictedFree => dialect.unrestricted_free_enabled,
            Self::ZantufaAdverbials => dialect.zantufa_adverbials_enabled,
            Self::ZantufaConnectives => dialect.zantufa_connectives_enabled,
            Self::ZantufaMex => dialect.zantufa_mex_enabled,
            Self::ZantufaMexReinterpretation => dialect.zantufa_mex_reinterpretation_enabled,
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
            Self::UnrestrictedFree => "UNRESTRICTED-FREE feature",
            Self::ZantufaAdverbials => "ZANTUFA-ADVERBIALS feature",
            Self::ZantufaConnectives => "ZANTUFA-CONNECTIVES feature",
            Self::ZantufaMex => "ZANTUFA-MEX feature",
            Self::ZantufaMexReinterpretation => "ZANTUFA-MEX-REINTERPRETATION feature",
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

#[invariant(!rule.is_empty())]
#[derive(Clone)]
struct RecoveryRuleParser<RF, R, P, O> {
    rule: &'static str,
    recovered_factory: RF,
    plain_parser: P,
    parser_types: std::marker::PhantomData<fn() -> (R, O)>,
}

#[requires(!rule.is_empty())]
#[ensures(true)]
pub(crate) fn recovery_rule_parser<'tokens, O, R, P, RF>(
    rule: &'static str,
    recovered_factory: RF,
    plain_parser: P,
) -> impl Parser<'tokens, O> + Clone
where
    O: Clone,
    R: Parser<'tokens, O> + Clone,
    P: Parser<'tokens, O> + Clone,
    RF: Fn() -> R + Clone,
{
    new!(RecoveryRuleParser {
        rule,
        recovered_factory,
        plain_parser,
        parser_types: std::marker::PhantomData,
    })
}

#[contract_trait]
impl<'tokens, O, R, P, RF> Parser<'tokens, O> for RecoveryRuleParser<RF, R, P, O>
where
    R: Parser<'tokens, O>,
    P: Parser<'tokens, O>,
    RF: Fn() -> R,
{
    #[inline(always)]
    fn drive_emit(&self, input: &mut InputRef<'tokens, '_>) -> Result<O, ()> {
        if recovery_rule_evaluation_enabled(input, self.rule) {
            mark_recovered_rule_path_cold();
            (self.recovered_factory)().drive_emit(input)
        } else {
            self.plain_parser.drive_emit(input)
        }
    }

    #[inline(always)]
    fn drive_check(&self, input: &mut InputRef<'tokens, '_>) -> Result<(), ()> {
        if recovery_rule_evaluation_enabled(input, self.rule) {
            mark_recovered_rule_path_cold();
            (self.recovered_factory)().drive_check(input)
        } else {
            self.plain_parser.drive_check(input)
        }
    }
}

#[requires(true)]
#[ensures(true)]
#[cold]
#[inline(never)]
fn mark_recovered_rule_path_cold() {}

#[requires(!rule.is_empty())]
#[ensures(true)]
// Keep the selector out of the generated rule monomorphizations. It returns
// before either concrete parser descends, so this does not add to the recursive
// parser stack; `RecoveryRuleParser::drive_*` stays always-inlined to preserve
// static dispatch into the selected parser body.
#[inline(never)]
fn recovery_rule_evaluation_enabled(input: &mut InputRef<'_, '_>, rule: &'static str) -> bool {
    if let Some(frame) = input
        .state()
        .active_syntax_rules()
        .last()
        .filter(|frame| frame.rule() == rule)
    {
        return frame.recovery_enabled();
    }
    let location = ParserInput::cursor_location(input.cursor().inner());
    let byte_start = input.state().byte_offset_for_location(location);
    input.state().recovery_rule_parser_enabled(rule, byte_start)
}

#[requires(!name.is_empty())]
#[requires(context.is_none_or(|construct| !construct.is_empty()))]
#[ensures(true)]
pub(crate) fn rule_wrapper<'tokens, O, P>(
    name: &'static str,
    context: Option<&'static str>,
    parser: P,
) -> BoxedParser<'tokens, SharedSyntaxOutput<O>>
where
    O: Clone + 'static,
    P: Parser<'tokens, O> + Clone + 'tokens,
{
    custom::<_, _>(move |input: &mut InputRef<'tokens, '_>| {
        let checkpoint = input.save();
        let start_location = ParserInput::cursor_location(checkpoint.cursor().inner());
        let memo_context = input.state().syntax_memo_context();
        input.state().begin_syntax_memo_rule_frame();
        if input.state().trace_enabled() {
            input.state().mark_syntax_memo_rule_recovery_sensitive();
        }
        let replay_hit = input
            .state()
            .syntax_memo_success(name, start_location, memo_context);
        if let Some(hit) = replay_hit
            && let Ok(value) = hit.value().downcast::<O>()
        {
            let replay = input.state().apply_syntax_memo_success(hit);
            advance_to_location(input, replay.end_location);
            input
                .state()
                .replay_syntax_memo_side_effects(&replay.side_effects);
            input.state().finish_syntax_memo_rule_frame();
            return Ok(SharedSyntaxOutput::from_shared(value));
        }
        let failure = input
            .state()
            .syntax_memo_failure(name, start_location, memo_context);
        if let Some(failure) = failure {
            input.rewind(checkpoint);
            input
                .state()
                .replay_syntax_diagnostic_observations(failure.diagnostic_observations.as_ref());
            input.state().finish_syntax_memo_rule_frame();
            return Err(failure.into_error());
        }
        if !input
            .state()
            .enter_syntax_memo_rule(name, start_location, memo_context)
        {
            input.rewind(checkpoint);
            input.state().finish_syntax_memo_rule_frame();
            return Err(expected_found_named_at_current(input, name.to_owned()));
        }
        let warning_start = input.state().warning_count();
        let start_byte = input.state().byte_offset_for_location(start_location);
        input.state().observe_syntax_rule(name, start_byte);
        let track_recovery_branches = input.state().recovery_branch_tracking_enabled();
        if track_recovery_branches {
            input.state().push_syntax_rule(name, start_byte);
        }
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
                let output = SharedSyntaxOutput::new(output);
                let memo_value: Rc<dyn Any> = output.clone().into_shared();
                input.state().store_syntax_memo_success(
                    name,
                    start_location,
                    memo_context,
                    end_location,
                    super::SyntaxMemoValue::from_shared(memo_value),
                    warnings,
                );
                if track_recovery_branches {
                    input.state().pop_syntax_rule();
                }
                input
                    .state()
                    .exit_syntax_memo_rule(name, start_location, memo_context);
                input.state().finish_syntax_memo_rule_frame();
                Ok(output)
            }
            Err(error) => {
                let failure_location = ParserInput::cursor_location(input.cursor().inner());
                let error = if let Some(construct) = context {
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
                input.state().record_continuation_rule_failure(
                    start_location,
                    failure_location,
                    &error,
                );
                if track_recovery_branches {
                    input.state().pop_syntax_rule();
                }
                input.rewind(checkpoint);
                input.state().store_syntax_memo_failure(
                    name,
                    start_location,
                    memo_context,
                    error.clone(),
                );
                input
                    .state()
                    .exit_syntax_memo_rule(name, start_location, memo_context);
                input.state().finish_syntax_memo_rule_frame();
                Err(error)
            }
        }
    })
    .boxed()
}

#[requires(!construct.is_empty())]
#[ensures(true)]
fn trace_rule_exit<'tokens>(
    input: &mut InputRef<'tokens, '_>,
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
fn advance_to_location<'tokens>(input: &mut InputRef<'tokens, '_>, end_location: usize) {
    while ParserInput::cursor_location(input.cursor().inner()) < end_location {
        if input.next().is_none() {
            break;
        }
    }
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn empty<'tokens>() -> BoxedParser<'tokens, ()> {
    parser_empty().boxed()
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn eof<'tokens>() -> BoxedParser<'tokens, ()> {
    parser_end().boxed()
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn feature_gate<'tokens, O, P>(
    feature: SyntaxGrammarFeature,
    parser: P,
) -> BoxedParser<'tokens, O>
where
    O: 'tokens,
    P: Parser<'tokens, O> + Clone + 'tokens,
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
    P: Parser<'tokens, O> + Clone + 'tokens,
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
    P: Parser<'tokens, O> + Clone + 'tokens,
{
    custom::<_, _>(move |input| {
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
    P: Parser<'tokens, O> + 'tokens,
{
    custom::<_, _>(move |input| {
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
    custom::<_, _>(move |input| {
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
    custom::<_, _>(move |input| {
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

#[requires(!rule.is_empty())]
#[ensures(true)]
pub(crate) fn recovery_checkpoint_field_parser<'tokens, O, P>(
    rule: &'static str,
    field_index: usize,
    parser: P,
) -> BoxedParser<'tokens, O>
where
    O: 'tokens,
    P: Parser<'tokens, O> + Clone + 'tokens,
{
    custom::<_, _>(move |input| {
        let start_location = ParserInput::cursor_location(input.cursor().inner());
        let instance_byte_start = input
            .state()
            .recovery_rule_instance_byte_start(rule, start_location);
        input.state().record_recovery_checkpoint(
            rule,
            instance_byte_start,
            start_location,
            field_index,
            RecoveryCheckpointKind::FieldStart,
        );
        let value = input.parse(&parser)?;
        let end_location = ParserInput::cursor_location(input.cursor().inner());
        input.state().record_recovery_checkpoint(
            rule,
            instance_byte_start,
            end_location,
            field_index,
            RecoveryCheckpointKind::Trailing,
        );
        Ok(value)
    })
    .boxed()
}

#[requires(!rule.is_empty())]
#[requires(min_count <= 1)]
#[ensures(true)]
pub(crate) fn recovery_checkpoint_greedy_many_field_parser<'tokens, O, P>(
    rule: &'static str,
    field_index: usize,
    min_count: usize,
    parser: P,
) -> BoxedParser<'tokens, Vec<O>>
where
    O: 'tokens,
    P: Parser<'tokens, O> + Clone + 'tokens,
{
    custom::<_, _>(move |input| {
        let mut values = Vec::new();
        loop {
            let checkpoint = input.save();
            let start_location = ParserInput::cursor_location(checkpoint.cursor().inner());
            let instance_byte_start = input
                .state()
                .recovery_rule_instance_byte_start(rule, start_location);
            input.state().record_recovery_checkpoint(
                rule,
                instance_byte_start,
                start_location,
                field_index,
                RecoveryCheckpointKind::FieldStart,
            );
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
                    if values.len() < min_count {
                        return Err(error);
                    }
                    input.state().record_diagnostic_candidate(error);
                    break;
                }
            }
        }
        Ok(values)
    })
    .boxed()
}

#[requires(!rule.is_empty())]
#[requires(min_count <= 1)]
#[ensures(true)]
pub(crate) fn recovery_checkpoint_recovered_greedy_many_field_parser<'tokens, O, P>(
    rule: &'static str,
    field_index: usize,
    min_count: usize,
    parser: P,
) -> BoxedParser<'tokens, Vec<O>>
where
    O: 'tokens,
    P: Parser<'tokens, O> + Clone + 'tokens,
{
    custom::<_, _>(move |input| {
        let mut values = Vec::new();
        loop {
            let checkpoint = input.save();
            let start_location = ParserInput::cursor_location(checkpoint.cursor().inner());
            let instance_byte_start = input
                .state()
                .recovery_rule_instance_byte_start(rule, start_location);
            input.state().record_recovery_checkpoint(
                rule,
                instance_byte_start,
                start_location,
                field_index,
                RecoveryCheckpointKind::FieldStart,
            );
            match input.parse(&parser) {
                Ok(output) => {
                    let end_location = ParserInput::cursor_location(input.cursor().inner());
                    let end_checkpoint = input.save();
                    if end_location == start_location
                        && !end_checkpoint.recovery_state_changed_since(&checkpoint)
                    {
                        input.rewind(checkpoint);
                        if values.len() < min_count {
                            return Err(expected_found_at_current(
                                input,
                                "non-empty recovered repetition item",
                            ));
                        }
                        break;
                    }
                    values.push(output);
                }
                Err(error) => {
                    input.rewind(checkpoint);
                    if values.len() < min_count {
                        return Err(error);
                    }
                    input.state().record_diagnostic_candidate(error);
                    break;
                }
            }
        }
        Ok(values)
    })
    .boxed()
}

/// Repeat a recovered parser while either input or recovery-directive state
/// advances. A recovered required field may synthesize an item without
/// consuming a token, so input position alone cannot establish progress.
#[requires(true)]
#[ensures(true)]
pub(crate) fn recovered_greedy_many_parser<'tokens, O: 'tokens>(
    parser: BoxedParser<'tokens, O>,
) -> BoxedParser<'tokens, Vec<O>> {
    custom::<_, _>(move |input| {
        let mut values = Vec::new();
        loop {
            let checkpoint = input.save();
            let start_location = ParserInput::cursor_location(checkpoint.cursor().inner());
            match input.parse(&parser) {
                Ok(output) => {
                    let end_location = ParserInput::cursor_location(input.cursor().inner());
                    let end_checkpoint = input.save();
                    if end_location == start_location
                        && !end_checkpoint.recovery_state_changed_since(&checkpoint)
                    {
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

/// Repeat a recovered parser at least once, counting recovery-directive state
/// advancement as progress when it produces a zero-width missing item.
#[requires(true)]
#[ensures(true)]
pub(crate) fn recovered_greedy_many1_parser<'tokens, O: 'tokens>(
    parser: BoxedParser<'tokens, O>,
) -> BoxedParser<'tokens, Vec<O>> {
    custom::<_, _>(move |input| {
        let first_checkpoint = input.save();
        let first_start_location = ParserInput::cursor_location(first_checkpoint.cursor().inner());
        let first = match input.parse(&parser) {
            Ok(output) => {
                let first_end_location = ParserInput::cursor_location(input.cursor().inner());
                let first_end_checkpoint = input.save();
                if first_end_location == first_start_location
                    && !first_end_checkpoint.recovery_state_changed_since(&first_checkpoint)
                {
                    input.rewind(first_checkpoint);
                    return Err(expected_found_at_current(
                        input,
                        "non-empty recovered repetition item",
                    ));
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
                    let end_checkpoint = input.save();
                    if end_location == start_location
                        && !end_checkpoint.recovery_state_changed_since(&checkpoint)
                    {
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
    custom::<_, _>(move |input| {
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

/// Parses the supplied free-modifier list only when `feature` is enabled.
///
/// The disabled branch succeeds without touching the input, so callers can retain a
/// `WithFreeModifiers` model field while making the corresponding grammar slot absent.
#[requires(true)]
#[ensures(true)]
pub(crate) fn feature_free_modifier_list_parser<'tokens, F>(
    feature: SyntaxGrammarFeature,
    enabled_parser: BoxedParser<'tokens, Vec<F>>,
) -> BoxedParser<'tokens, Vec<F>>
where
    F: 'tokens,
{
    custom::<_, _>(move |input| {
        let env = input.state().syntax_grammar_env();
        if feature.enabled(env.dialect) {
            input.parse(&enabled_parser)
        } else {
            Ok(Vec::new())
        }
    })
    .boxed()
}

#[contract_trait]
pub(crate) trait RecoveredSyntaxSlot: Sized {
    #[requires(true)]
    #[ensures(true)]
    fn empty_recovery_slot() -> Option<Self> {
        None
    }

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
    #[ensures(ret.as_ref().is_some_and(Vec::is_empty))]
    fn empty_recovery_slot() -> Option<Self> {
        Some(Vec::new())
    }

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
    #[ensures(ret.as_ref().is_some_and(smallvec::SmallVec::is_empty))]
    fn empty_recovery_slot() -> Option<Self> {
        Some(smallvec::SmallVec::new())
    }

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
    #[ensures(ret.as_ref().is_some_and(Option::is_none))]
    fn empty_recovery_slot() -> Option<Self> {
        Some(None)
    }

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
    P: Parser<'tokens, O> + Clone + 'tokens,
{
    custom::<_, _>(move |input| {
        let checkpoint = input.save();
        let location = ParserInput::cursor_location(checkpoint.cursor().inner());
        let instance_byte_start = input
            .state()
            .recovery_rule_instance_byte_start(rule, location);
        let empty_slot = O::empty_recovery_slot();
        input.state().record_recovery_checkpoint(
            rule,
            instance_byte_start,
            location,
            field_index,
            RecoveryCheckpointKind::FieldStart,
        );
        let action = input.state().recovery_field_action(
            rule,
            instance_byte_start,
            field_index,
            location,
            empty_slot.is_some(),
        );
        let item = match action.map(super::RecoveryFieldAction::into_parts) {
            Some((super::RecoveryFieldActionKind::Abandon, item, _resume_token_index)) => {
                if let Some(value) = empty_slot {
                    return Ok(value);
                }
                return Ok(O::from_recovery_item(item.expect(
                    "required abandoned recovery fields carry a recovery item",
                )));
            }
            Some((super::RecoveryFieldActionKind::BoundaryResync, _, _)) => {
                unreachable!("boundary resync actions are produced only after a field failure")
            }
            Some((super::RecoveryFieldActionKind::Resume, item, Some(resume_token_index))) => {
                advance_to_location(input, resume_token_index);
                item
            }
            Some((super::RecoveryFieldActionKind::Resume, _, None)) => {
                unreachable!("resume recovery actions carry a resume token index")
            }
            None => None,
        };
        let parse_checkpoint = input.save();
        let parse_start_location = ParserInput::cursor_location(parse_checkpoint.cursor().inner());
        let mut value = match input.parse(&parser) {
            Ok(value) => value,
            Err(error) => {
                if !input.state().boundary_resync_catches_field_failure(
                    rule,
                    instance_byte_start,
                    field_index,
                    parse_start_location,
                ) {
                    return Err(error);
                }
                input.rewind(parse_checkpoint);
                let action = input
                    .state()
                    .boundary_resync_field_action_after_failure(
                        rule,
                        instance_byte_start,
                        field_index,
                        parse_start_location,
                    )
                    .into_parts();
                match action {
                    (
                        super::RecoveryFieldActionKind::BoundaryResync,
                        Some(item),
                        Some(resume_token_index),
                    ) => {
                        advance_to_location(input, resume_token_index);
                        return Ok(O::from_recovery_item(item));
                    }
                    (super::RecoveryFieldActionKind::Resume, item, Some(resume_token_index)) => {
                        advance_to_location(input, resume_token_index);
                        let mut value = input.parse(&parser)?;
                        if let Some(item) = item {
                            value = value.prepend_recovery_item(item);
                        }
                        value
                    }
                    (super::RecoveryFieldActionKind::BoundaryResync, None, _) => {
                        unreachable!("boundary resync of a failed field carries a skipped item")
                    }
                    (super::RecoveryFieldActionKind::Abandon, _, _) => {
                        unreachable!("failed-field boundary recovery never returns local abandon")
                    }
                    (super::RecoveryFieldActionKind::BoundaryResync, _, None) => {
                        unreachable!("boundary resync actions carry a resume token index")
                    }
                    (super::RecoveryFieldActionKind::Resume, _, None) => {
                        unreachable!("resume recovery actions carry a resume token index")
                    }
                }
            }
        };
        if let Some(item) = item {
            value = value.prepend_recovery_item(item);
        }
        let location = ParserInput::cursor_location(input.cursor().inner());
        input.state().record_recovery_checkpoint(
            rule,
            instance_byte_start,
            location,
            field_index,
            RecoveryCheckpointKind::Trailing,
        );
        if let Some((item, resume_token_index)) = input.state().trailing_recovery_field_action(
            rule,
            instance_byte_start,
            field_index,
            location,
        ) {
            advance_to_location(input, resume_token_index);
            value = value.prepend_recovery_item(item);
        }
        Ok(value)
    })
    .boxed()
}

#[requires(!rule.is_empty())]
#[requires(min_count <= 1)]
#[ensures(true)]
pub(crate) fn recovered_greedy_many_field_parser<'tokens, O, P>(
    rule: &'static str,
    field_index: usize,
    min_count: usize,
    recovery_boundary: bool,
    parser: P,
) -> BoxedParser<'tokens, Vec<O>>
where
    O: RecoveredSyntaxSlot + 'tokens,
    P: Parser<'tokens, O> + Clone + 'tokens,
{
    custom::<_, _>(move |input| {
        let mut values = Vec::new();
        loop {
            let checkpoint = input.save();
            let start_location = ParserInput::cursor_location(checkpoint.cursor().inner());
            let instance_byte_start = input
                .state()
                .recovery_rule_instance_byte_start(rule, start_location);
            input.state().record_recovery_checkpoint(
                rule,
                instance_byte_start,
                start_location,
                field_index,
                RecoveryCheckpointKind::FieldStart,
            );
            let action = input.state().recovery_field_action(
                rule,
                instance_byte_start,
                field_index,
                start_location,
                values.len() >= min_count,
            );
            match action.map(super::RecoveryFieldAction::into_parts) {
                Some((super::RecoveryFieldActionKind::Abandon, item, _resume_token_index)) => {
                    if let Some(item) = item {
                        values.push(O::from_recovery_item(item));
                    }
                    if values.len() >= min_count {
                        break;
                    }
                }
                Some((super::RecoveryFieldActionKind::BoundaryResync, _, _)) => {
                    unreachable!(
                        "boundary resync actions are produced only after a repetition item failure"
                    )
                }
                Some((super::RecoveryFieldActionKind::Resume, item, Some(resume_token_index))) => {
                    advance_to_location(input, resume_token_index);
                    if let Some(item) = item {
                        values.push(O::from_recovery_item(item));
                    }
                    continue;
                }
                Some((super::RecoveryFieldActionKind::Resume, _, None)) => {
                    unreachable!("resume recovery actions carry a resume token index")
                }
                None => {}
            }

            match input.parse(&parser) {
                Ok(output) => {
                    let end_location = ParserInput::cursor_location(input.cursor().inner());
                    if end_location == start_location {
                        debug_assert!(false, "generated repetition parser accepted empty input");
                        input.rewind(checkpoint);
                        break;
                    }
                    if recovery_boundary {
                        input
                            .state()
                            .record_completed_recovery_boundary(start_location);
                    }
                    values.push(output);
                }
                Err(error) => {
                    input.rewind(checkpoint);
                    if input.state().boundary_resync_catches_field_failure(
                        rule,
                        instance_byte_start,
                        field_index,
                        start_location,
                    ) {
                        let action = input
                            .state()
                            .boundary_resync_field_action_after_failure(
                                rule,
                                instance_byte_start,
                                field_index,
                                start_location,
                            )
                            .into_parts();
                        match action {
                            (
                                super::RecoveryFieldActionKind::BoundaryResync,
                                Some(item),
                                Some(resume_token_index),
                            ) => {
                                advance_to_location(input, resume_token_index);
                                values.push(O::from_recovery_item(item));
                                break;
                            }
                            (
                                super::RecoveryFieldActionKind::Resume,
                                Some(item),
                                Some(resume_token_index),
                            ) => {
                                advance_to_location(input, resume_token_index);
                                values.push(O::from_recovery_item(item));
                                continue;
                            }
                            (
                                super::RecoveryFieldActionKind::BoundaryResync
                                | super::RecoveryFieldActionKind::Resume,
                                None,
                                _,
                            ) => {
                                unreachable!(
                                    "boundary resync of a failed repetition item carries a skipped item"
                                )
                            }
                            (super::RecoveryFieldActionKind::Abandon, _, _) => {
                                unreachable!(
                                    "failed repetition boundary recovery never returns local abandon"
                                )
                            }
                            (super::RecoveryFieldActionKind::BoundaryResync, _, None) => {
                                unreachable!(
                                    "boundary resync actions carry a resume token index"
                                )
                            }
                            (super::RecoveryFieldActionKind::Resume, _, None) => {
                                unreachable!("resume recovery actions carry a resume token index")
                            }
                        }
                    }
                    let action = input.state().recovery_field_action_at_natural_stop(
                        rule,
                        instance_byte_start,
                        field_index,
                        start_location,
                        values.len() >= min_count,
                    );
                    match action.map(super::RecoveryFieldAction::into_parts) {
                        Some((super::RecoveryFieldActionKind::Abandon, item, _)) => {
                            if let Some(item) = item {
                                values.push(O::from_recovery_item(item));
                            }
                            if values.len() >= min_count {
                                break;
                            }
                        }
                        Some((
                            super::RecoveryFieldActionKind::Resume,
                            item,
                            Some(resume_token_index),
                        )) => {
                            advance_to_location(input, resume_token_index);
                            if let Some(item) = item {
                                values.push(O::from_recovery_item(item));
                            }
                            continue;
                        }
                        Some((super::RecoveryFieldActionKind::BoundaryResync, _, _)) => {
                            unreachable!(
                                "boundary resync actions are produced only after a repetition item failure"
                            )
                        }
                        Some((super::RecoveryFieldActionKind::Resume, _, None)) => {
                            unreachable!("resume recovery actions carry a resume token index")
                        }
                        None => {}
                    }
                    if values.len() >= min_count {
                        input.state().record_diagnostic_candidate(error);
                        break;
                    }
                    return Err(error);
                }
            }
        }
        if values.len() < min_count {
            return Err(expected_found_named_at_current(input, rule.to_owned()));
        }
        Ok(values)
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
            .map_with(|free_modifier, extra: &mut MapExtra<'tokens, '_>| {
                if let Some(anchor) = free_modifier.first_word() {
                    extra.state().warn(
                        ExperimentalConstruct::CllProhibitedFreeModifierPlacement,
                        anchor,
                    );
                }
                free_modifier
            })
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
    P: Parser<'tokens, O> + Clone + 'tokens,
{
    custom::<_, _>(move |input| {
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
    custom::<_, _>(move |input| Err(expected_found_at_current(input, "free modifier"))).boxed()
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
    custom::<_, _>(move |input| {
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
    G: Parser<'tokens, GO> + Clone + 'tokens,
    P: Parser<'tokens, O> + Clone + 'tokens,
{
    custom::<_, _>(move |input| {
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
    G: Parser<'tokens, GO> + Clone + 'tokens,
    P: Parser<'tokens, O> + Clone + 'tokens,
{
    custom::<_, _>(move |input| {
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
    P: Parser<'tokens, O> + Clone + 'tokens,
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
    P: Parser<'tokens, O> + Clone + 'tokens,
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
    P: Parser<'tokens, O> + Clone + 'tokens,
{
    custom::<_, _>(move |input| {
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
    input: &mut InputRef<'tokens, '_>,
    expected: &'static str,
) -> SyntaxParseError<'tokens> {
    expected_found_named_at_current(input, expected.to_owned())
}

#[requires(!expected.is_empty())]
#[ensures(true)]
fn expected_found_named_at_current<'tokens>(
    input: &mut InputRef<'tokens, '_>,
    expected: String,
) -> SyntaxParseError<'tokens> {
    expected_found_tokens_at_current(input, vec![new!(SyntaxExpectedToken::Named(expected))])
}

#[requires(!expected.is_empty())]
#[ensures(true)]
fn expected_found_tokens_at_current<'tokens>(
    input: &mut InputRef<'tokens, '_>,
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
    P: Parser<'tokens, O> + Clone + 'tokens,
{
    custom::<_, _>(move |input| {
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

/// A typed refinement that can reject an otherwise successful parser output.
#[contract_trait]
pub(crate) trait OutputRejection<O> {
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn rejected_name(&self) -> &'static str;

    #[requires(true)]
    #[ensures(true)]
    fn rejects(&self, value: &O) -> bool;
}

/// Rejects a completed typed match and rewinds all state to the route's start.
#[requires(true)]
#[ensures(true)]
pub(crate) fn reject_output<'tokens, O, P, R>(inner: P, rejection: R) -> BoxedParser<'tokens, O>
where
    O: 'tokens,
    P: Parser<'tokens, O> + Clone + 'tokens,
    R: OutputRejection<O> + Clone + 'tokens,
{
    custom::<_, _>(move |input| {
        let before = input.save();
        let diagnostic_snapshot = input.state().diagnostic_candidates_snapshot();
        let value = match input.parse(&inner) {
            Ok(value) => value,
            Err(error) => {
                input.rewind(before);
                return Err(error);
            }
        };
        if !rejection.rejects(&value) {
            return Ok(value);
        }
        input.rewind(before);
        input
            .state()
            .restore_diagnostic_candidates(diagnostic_snapshot);
        Err(expected_found_named_at_current(
            input,
            format!("not {}", rejection.rejected_name()),
        ))
    })
    .boxed()
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn not<'tokens, O, P>(parser: P) -> BoxedParser<'tokens, ()>
where
    O: 'tokens,
    P: Parser<'tokens, O> + Clone + 'tokens,
{
    custom::<_, _>(move |input| {
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
        cmevla_word().map_with(|word, extra: &mut MapExtra<'tokens, '_>| {
            extra.state().warn(
                ExperimentalConstruct::ExperimentalCbmCmevlaSelbriWord,
                &word,
            );
            word
        }),
    );
    brivla.or(cbm_cmevla).boxed()
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn text_leading_cmevla_word<'tokens>() -> BoxedParser<'tokens, Token> {
    custom::<_, _>(move |input| {
        let checkpoint = input.save();
        if input.state().syntax_grammar_env().dialect.cbm_enabled {
            input.rewind(checkpoint);
            return Err(expected_found_at_current(input, "non-CBM leading CMEVLA"));
        }

        let is_continuation_sentinel = input.next_is_continuation_sentinel();
        match input.next() {
            Some(word) if !is_continuation_sentinel && is_cmevla_word(&word) => Ok(word),
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
            | bityzba::data!(jbotci_morphology::WordLike::SelmahoQuotedWord { .. })
            | bityzba::data!(jbotci_morphology::WordLike::DelimitedWordQuote { .. })
            | bityzba::data!(jbotci_morphology::WordLike::DelimitedNonLojbanQuote { .. })
            | bityzba::data!(jbotci_morphology::WordLike::QuotedWords { .. })
    )
}
