#[allow(unused_imports)]
use bityzba::{data, ensures, invariant, new, requires};
use std::{
    borrow::Cow,
    ops::{Deref, DerefMut},
    rc::Rc,
    sync::Arc,
};

use super::parser_core::{Error, LabelError, MaybeRef, RichPattern, RichReason};
use super::{Span, SyntaxContextFrame, SyntaxRuleFrame, Token};
use crate::{
    SyntaxConstructContext, SyntaxExpectation, SyntaxExpectationReason,
    SyntaxExpectationReasonData, SyntaxExpectedToken, SyntaxExpectedTokenData,
    syntax_construct_depth, syntax_construct_is_descendant_of, syntax_construct_is_known,
    syntax_construct_is_root, syntax_construct_parent, syntax_immediate_child_under,
};

type SyntaxRichReason<'tokens> = RichReason<'tokens, Token, Cow<'static, str>>;

#[invariant(true)]
#[derive(Debug, Clone)]
pub(super) struct SyntaxParseError<'tokens> {
    data: Rc<SyntaxParseErrorData<'tokens>>,
}

#[invariant(true)]
#[derive(Debug, Clone)]
pub(super) struct SyntaxParseErrorData<'tokens> {
    span: Span,
    reason: SyntaxRichReason<'tokens>,
    expected_groups: Vec<ExpectedTokenGroup>,
    context_paths: Vec<Vec<SyntaxConstructContext>>,
    found: Option<SyntaxFound>,
    custom_kind: Option<SyntaxParseCustomKind>,
    active_contexts: Vec<SyntaxContextFrame>,
    active_rule_contexts: Vec<SyntaxRuleFrame>,
    preferred_context_hint: Option<SyntaxConstructContext>,
    same_position_branches: Vec<Arc<SyntaxParseError<'tokens>>>,
}

impl<'tokens> Deref for SyntaxParseError<'tokens> {
    type Target = SyntaxParseErrorData<'tokens>;

    #[requires(true)]
    #[ensures(true)]
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<'tokens> DerefMut for SyntaxParseError<'tokens> {
    #[requires(true)]
    #[ensures(true)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        Rc::make_mut(&mut self.data)
    }
}

#[invariant(true)]
#[invariant(::Token(token) => token.core_word().byte_range().is_some(), "found syntax token must cover source bytes")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum SyntaxFound {
    Token(Token),
    EndOfInput,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
pub(super) enum SyntaxParseCustomKind {
    BridiTailKeContinuationConflict,
}

#[invariant(!tokens.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
struct ExpectedTokenGroup {
    tokens: Arc<[SyntaxExpectedToken]>,
    reason: Option<SyntaxExpectationReason>,
}

impl ExpectedTokenGroup {
    #[requires(!tokens.is_empty())]
    #[ensures(!ret.tokens.is_empty())]
    fn new(tokens: Arc<[SyntaxExpectedToken]>) -> Self {
        new!(ExpectedTokenGroup {
            tokens,
            reason: None,
        })
    }

    #[requires(!tokens.is_empty())]
    #[ensures(!ret.tokens.is_empty())]
    fn from_vec(tokens: Vec<SyntaxExpectedToken>) -> Self {
        Self::new(Arc::from(tokens))
    }

    #[requires(!tokens.is_empty())]
    #[ensures(!ret.tokens.is_empty())]
    fn with_optional_reason(
        tokens: Arc<[SyntaxExpectedToken]>,
        reason: Option<SyntaxExpectationReason>,
    ) -> Self {
        new!(ExpectedTokenGroup { tokens, reason })
    }
}

#[requires(true)]
#[ensures(matches!(ret, RichReason::Custom(ref message) if message == "unexpected input"))]
fn unexpected_input_error<'tokens>() -> SyntaxRichReason<'tokens> {
    RichReason::Custom(Cow::Borrowed("unexpected input"))
}

impl<'tokens> SyntaxParseError<'tokens> {
    #[requires(true)]
    #[ensures(Rc::strong_count(&ret.data) == 1)]
    fn from_data(data: SyntaxParseErrorData<'tokens>) -> Self {
        Self {
            data: Rc::new(data),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn into_data(self) -> SyntaxParseErrorData<'tokens> {
        Rc::try_unwrap(self.data).unwrap_or_else(|data| (*data).clone())
    }

    #[requires(!message.is_empty())]
    #[ensures(ret.expected_groups.is_empty())]
    pub(super) fn custom(span: Span, message: String) -> Self {
        Self::from_data(SyntaxParseErrorData {
            span,
            reason: RichReason::Custom(Cow::Owned(message)),
            expected_groups: Vec::new(),
            context_paths: empty_context_paths(),
            found: None,
            custom_kind: None,
            active_contexts: Vec::new(),
            active_rule_contexts: Vec::new(),
            preferred_context_hint: None,
            same_position_branches: Vec::new(),
        })
    }

    #[requires(!message.is_empty())]
    #[ensures(ret.expected_groups.is_empty())]
    pub(super) fn custom_with_kind(
        span: Span,
        message: String,
        custom_kind: SyntaxParseCustomKind,
    ) -> Self {
        Self::from_data(SyntaxParseErrorData {
            span,
            reason: RichReason::Custom(Cow::Owned(message)),
            expected_groups: Vec::new(),
            context_paths: empty_context_paths(),
            found: None,
            custom_kind: Some(custom_kind),
            active_contexts: Vec::new(),
            active_rule_contexts: Vec::new(),
            preferred_context_hint: None,
            same_position_branches: Vec::new(),
        })
    }

    #[requires(!tokens.is_empty())]
    #[ensures(ret.expected_groups.len() == 1)]
    pub(super) fn expected(span: Span, tokens: Vec<SyntaxExpectedToken>) -> Self {
        Self::expected_shared(span, Arc::from(tokens))
    }

    #[requires(!tokens.is_empty())]
    #[ensures(ret.expected_groups.len() == 1)]
    pub(super) fn expected_shared(span: Span, tokens: Arc<[SyntaxExpectedToken]>) -> Self {
        Self::from_data(SyntaxParseErrorData {
            span,
            reason: unexpected_input_error(),
            expected_groups: vec![ExpectedTokenGroup::new(tokens)],
            context_paths: empty_context_paths(),
            found: None,
            custom_kind: None,
            active_contexts: Vec::new(),
            active_rule_contexts: Vec::new(),
            preferred_context_hint: None,
            same_position_branches: Vec::new(),
        })
    }

    #[requires(!tokens.is_empty())]
    #[ensures(ret.expected_groups.len() == 1)]
    pub(super) fn expected_found(
        span: Span,
        tokens: Vec<SyntaxExpectedToken>,
        found: SyntaxFound,
    ) -> Self {
        Self::expected_found_shared(span, Arc::from(tokens), found)
    }

    #[requires(!tokens.is_empty())]
    #[ensures(ret.expected_groups.len() == 1)]
    pub(super) fn expected_found_shared(
        span: Span,
        tokens: Arc<[SyntaxExpectedToken]>,
        found: SyntaxFound,
    ) -> Self {
        Self::from_data(SyntaxParseErrorData {
            span,
            reason: unexpected_input_error(),
            expected_groups: vec![ExpectedTokenGroup::new(tokens)],
            context_paths: empty_context_paths(),
            found: Some(found),
            custom_kind: None,
            active_contexts: Vec::new(),
            active_rule_contexts: Vec::new(),
            preferred_context_hint: None,
            same_position_branches: Vec::new(),
        })
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn span(&self) -> &Span {
        &self.span
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn reason(&self) -> &RichReason<'tokens, Token, Cow<'static, str>> {
        &self.reason
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn expected_strings(&self) -> Vec<String> {
        match &self.reason {
            RichReason::ExpectedFound { expected, .. } => {
                expected.iter().map(ToString::to_string).collect()
            }
            RichReason::Custom(_) => Vec::new(),
        }
    }

    #[requires(true)]
    #[ensures(ret.iter().all(|expectation| !expectation.tokens.is_empty()))]
    pub(super) fn expectations(&self) -> Vec<SyntaxExpectation> {
        let mut expectations = Vec::new();
        let contexts = merged_context_names(&self.context_paths);
        for group in &self.expected_groups {
            if !group.tokens.is_empty() {
                let reason = group
                    .reason
                    .clone()
                    .unwrap_or_else(|| expectation_reason(&group.tokens, &contexts));
                let reason = normalize_expectation_reason(reason, &self.context_paths);
                expectations.push(SyntaxExpectation::new(group.tokens.to_vec(), reason));
            }
        }
        if expectations.is_empty() {
            let expected = match &self.reason {
                RichReason::ExpectedFound { expected, .. } => expected.as_slice(),
                RichReason::Custom(_) => &[],
            };
            for token in expected
                .iter()
                .filter_map(syntax_expected_token_from_rich_pattern)
            {
                let reason = normalize_expectation_reason(
                    expectation_reason(
                        &[new!(SyntaxExpectedToken::Named("input".to_owned()))],
                        &contexts,
                    ),
                    &self.context_paths,
                );
                expectations.push(SyntaxExpectation::new(vec![token], reason));
            }
        }
        expectations
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_none_or(|context| !context.construct.is_empty()))]
    pub(super) fn current_context(&self) -> Option<SyntaxConstructContext> {
        select_current_context(&self.context_paths)
    }

    #[requires(true)]
    #[ensures(ret.len() <= limit)]
    pub(super) fn report_contexts(&self, limit: usize) -> Vec<SyntaxConstructContext> {
        if limit == 0 {
            return Vec::new();
        }
        let mut contexts = select_report_contexts(&self.context_paths, usize::MAX);
        if contexts.is_empty() {
            self.current_context()
                .or_else(|| self.preferred_context_hint.clone())
                .into_iter()
                .for_each(|context| contexts.push(context));
        }
        append_active_contexts_to_report_contexts(&mut contexts, &self.active_contexts, self.span);
        let contexts = normalize_report_contexts(contexts, limit);
        stretch_report_contexts_to_error(contexts, self.span)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_none_or(|context| !context.construct.is_empty()))]
    pub(super) fn preferred_context(&self) -> Option<SyntaxConstructContext> {
        if self.same_position_branches.is_empty() {
            return self
                .current_context()
                .or_else(|| self.preferred_context_hint.clone());
        }
        self.preferred_context_hint.clone()
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_none_or(|context| !context.construct.is_empty()))]
    pub(super) fn summary_context(&self) -> Option<SyntaxConstructContext> {
        select_current_context(&self.context_paths)
            .or_else(|| select_outer_common_context_including_roots(&self.context_paths))
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn context_paths(&self) -> &[Vec<SyntaxConstructContext>] {
        &self.context_paths
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn found(&self) -> Option<&SyntaxFound> {
        self.found.as_ref()
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn custom_kind(&self) -> Option<SyntaxParseCustomKind> {
        self.custom_kind
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn active_rule_contexts(&self) -> &[SyntaxRuleFrame] {
        &self.active_rule_contexts
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn merge_for_report(self, other: Self) -> Self {
        let preferred_context_hint =
            deeper_preferred_context(self.preferred_context(), other.preferred_context());
        let mut merged = self.into_report_error();
        let other = other.into_report_error().into_data();
        append_unique_groups(&mut merged.expected_groups, other.expected_groups);
        append_unique_context_paths(&mut merged.context_paths, other.context_paths);
        let merged_found = std::mem::take(&mut merged.found);
        merged.found = merge_optional_equal(merged_found, other.found);
        merged.custom_kind = merge_optional_equal(merged.custom_kind, other.custom_kind);
        merged.preferred_context_hint = preferred_context_hint;
        merged
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn merge_for_parser(self, other: Self) -> Self {
        select_parser_error(self, other)
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn with_active_contexts(mut self, contexts: &[SyntaxContextFrame]) -> Self {
        if self.active_contexts.len() > contexts.len() {
            return self;
        }
        self.active_contexts = contexts.to_vec();
        if self.preferred_context_hint.is_none() {
            self.preferred_context_hint =
                preferred_context_from_branches(&self.same_position_branches);
        }
        self
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn with_active_rule_contexts(mut self, contexts: &[SyntaxRuleFrame]) -> Self {
        if self.active_rule_contexts.len() <= contexts.len() {
            self.active_rule_contexts = contexts.to_vec();
        }
        self
    }

    #[requires(!construct.is_empty())]
    #[ensures(true)]
    pub(super) fn with_rule_start_label(mut self, construct: &'static str) -> Self {
        <Self as LabelError<'tokens, &'static str>>::label_with(&mut self, construct);
        self
    }

    #[requires(!construct.is_empty())]
    #[ensures(true)]
    pub(super) fn with_rule_context(mut self, construct: &'static str, span: Span) -> Self {
        <Self as LabelError<'tokens, &'static str>>::in_context(&mut self, construct, span);
        self
    }

    #[requires(!construct.is_empty())]
    #[ensures(true)]
    pub(super) fn with_rule_context_from_progress(
        self,
        construct: &'static str,
        start_byte: usize,
        advanced: bool,
    ) -> Self {
        let error_start = self.span.start;
        if advanced && error_start >= start_byte {
            self.with_rule_context(construct, Span::from(start_byte..error_start))
        } else {
            self.with_rule_start_label(construct)
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn same_report_content(&self, other: &Self) -> bool {
        if !self.same_position_branches.is_empty() || !other.same_position_branches.is_empty() {
            return false;
        }
        self.span == other.span
            && self.expected_groups == other.expected_groups
            && self.context_paths == other.context_paths
            && self.found == other.found
            && self.custom_kind == other.custom_kind
            && self.active_contexts == other.active_contexts
    }

    #[requires(true)]
    #[ensures(ret.same_position_branches.is_empty())]
    pub(super) fn into_report_error(self) -> Self {
        if self.same_position_branches.is_empty() {
            return self;
        }
        let mut error = self;
        let branches = std::mem::take(&mut error.same_position_branches);
        let mut merged = None;
        for branch in branches {
            let branch = arc_into_inner_or_clone(branch).into_report_error();
            merged = Some(match merged {
                None => branch,
                Some(previous) => merge_report_errors(previous, branch),
            });
        }
        merged.unwrap_or(error)
    }
}

#[bityzba::contract_trait]
impl<'tokens> Error<'tokens> for SyntaxParseError<'tokens> {
    #[requires(true)]
    #[ensures(true)]
    fn merge(self, other: Self) -> Self {
        select_parser_error(self, other)
    }
}

#[bityzba::contract_trait]
impl<'tokens, L> LabelError<'tokens, L> for SyntaxParseError<'tokens>
where
    L: TryInto<RichPattern<'tokens>> + Clone,
{
    #[requires(true)]
    #[ensures(true)]
    fn expected_found<E: IntoIterator<Item = L>>(
        expected: E,
        found: Option<MaybeRef<'tokens, Token>>,
        span: Span,
    ) -> Self {
        let expected = expected.into_iter().collect::<Vec<_>>();
        let syntax_found = syntax_found_from_maybe(found.clone());
        let reason = RichReason::ExpectedFound {
            expected: expected
                .iter()
                .cloned()
                .filter_map(|expected| expected.try_into().ok())
                .collect(),
            found,
        };
        let expected_groups = expected_token_groups_from_labels(expected);
        Self::from_data(SyntaxParseErrorData {
            span,
            reason,
            expected_groups,
            context_paths: empty_context_paths(),
            found: Some(syntax_found),
            custom_kind: None,
            active_contexts: Vec::new(),
            active_rule_contexts: Vec::new(),
            preferred_context_hint: None,
            same_position_branches: Vec::new(),
        })
    }

    #[requires(true)]
    #[ensures(true)]
    fn merge_expected_found<E: IntoIterator<Item = L>>(
        mut self,
        expected: E,
        found: Option<MaybeRef<'tokens, Token>>,
        _span: Span,
    ) -> Self
    where
        Self: Error<'tokens>,
    {
        if !self.same_position_branches.is_empty() {
            self = self.into_report_error();
        }
        let expected = expected.into_iter().collect::<Vec<_>>();
        append_unique_groups(
            &mut self.expected_groups,
            expected_token_groups_from_labels(expected.clone()),
        );
        let syntax_found = syntax_found_from_maybe(found.clone());
        if let RichReason::ExpectedFound {
            expected: current,
            found: current_found,
        } = &mut self.reason
        {
            for expected in expected {
                if let Ok(expected) = expected.try_into()
                    && !current.contains(&expected)
                {
                    current.push(expected);
                }
            }
            *current_found = current_found.take().or(found);
        }
        let current_found = std::mem::take(&mut self.found);
        self.found = merge_optional_equal(current_found, Some(syntax_found));
        self.custom_kind = None;
        self
    }

    #[requires(true)]
    #[ensures(true)]
    fn replace_expected_found<E: IntoIterator<Item = L>>(
        mut self,
        expected: E,
        found: Option<MaybeRef<'tokens, Token>>,
        span: Span,
    ) -> Self {
        if !self.same_position_branches.is_empty() {
            self = self.into_report_error();
        }
        let expected = expected.into_iter().collect::<Vec<_>>();
        self.expected_groups = expected_token_groups_from_labels(expected.clone());
        let syntax_found = syntax_found_from_maybe(found.clone());
        self.reason = RichReason::ExpectedFound {
            expected: expected
                .into_iter()
                .filter_map(|expected| expected.try_into().ok())
                .collect(),
            found,
        };
        self.span = span;
        self.context_paths = empty_context_paths();
        self.found = Some(syntax_found);
        self.custom_kind = None;
        self.active_contexts = Vec::new();
        self.active_rule_contexts = Vec::new();
        self.preferred_context_hint = None;
        self
    }

    #[requires(true)]
    #[ensures(true)]
    fn label_with(&mut self, label: L) {
        if !self.same_position_branches.is_empty() {
            for branch in &mut self.same_position_branches {
                <SyntaxParseError<'tokens> as LabelError<'tokens, L>>::label_with(
                    Arc::make_mut(branch),
                    label.clone(),
                );
            }
            return;
        }
        let Some(pattern) = label.clone().try_into().ok() else {
            return;
        };
        let found = match &mut self.reason {
            RichReason::ExpectedFound { found, .. } => found.take(),
            RichReason::Custom(_) => None,
        };
        self.reason = RichReason::ExpectedFound {
            expected: vec![pattern.clone()],
            found,
        };
        if !self.expected_groups.is_empty() {
            if let Some(construct) = context_from_rich_pattern(&pattern) {
                for group in &mut self.expected_groups {
                    if group.reason.is_none() {
                        *group = group.clone().with_data(data! {
                            reason: Some(start_nested_reason(&construct)),
                        });
                    }
                }
            }
        } else if let Some(token) = syntax_expected_token_from_rich_pattern(&pattern) {
            let reason = context_from_rich_pattern(&pattern)
                .map(|construct| start_nested_reason(&construct));
            self.expected_groups
                .push(ExpectedTokenGroup::with_optional_reason(
                    Arc::from(vec![token]),
                    reason,
                ));
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn in_context(&mut self, label: L, span: Span) {
        if !self.same_position_branches.is_empty() {
            for branch in &mut self.same_position_branches {
                <SyntaxParseError<'tokens> as LabelError<'tokens, L>>::in_context(
                    Arc::make_mut(branch),
                    label.clone(),
                    span,
                );
            }
            self.preferred_context_hint =
                preferred_context_from_branches(&self.same_position_branches);
            return;
        }
        let context = label
            .clone()
            .try_into()
            .ok()
            .and_then(|pattern| context_from_rich_pattern(&pattern))
            .map(|construct| {
                SyntaxConstructContext::new(
                    construct,
                    span.start.min(span.end),
                    span.start.max(span.end),
                )
            });
        if let Some(context) = context {
            for group in &mut self.expected_groups {
                apply_context_to_group(group, &context.construct);
            }
            push_context_to_paths(&mut self.context_paths, context);
        }
    }
}

#[requires(true)]
#[ensures(ret.iter().all(|group| !group.tokens.is_empty()))]
fn expected_token_groups_from_labels<'tokens, L>(labels: Vec<L>) -> Vec<ExpectedTokenGroup>
where
    L: TryInto<RichPattern<'tokens>>,
{
    labels
        .into_iter()
        .filter_map(|label| {
            label
                .try_into()
                .ok()
                .and_then(|pattern| syntax_expected_token_from_rich_pattern(&pattern))
                .map(|token| ExpectedTokenGroup::from_vec(vec![token]))
        })
        .collect()
}

#[requires(true)]
#[ensures(true)]
fn syntax_expected_token_from_rich_pattern(
    pattern: &RichPattern<'_>,
) -> Option<SyntaxExpectedToken> {
    match pattern {
        RichPattern::Label(label) => Some(new!(SyntaxExpectedToken::Named(label.to_string()))),
        RichPattern::EndOfInput => Some(new!(SyntaxExpectedToken::EndOfInput)),
    }
}

#[requires(true)]
#[ensures(true)]
fn context_from_rich_pattern(pattern: &RichPattern<'_>) -> Option<String> {
    let construct = match pattern {
        RichPattern::Label(label) => label.to_string(),
        _ => return None,
    };
    syntax_construct_is_known(&construct).then_some(construct)
}

#[requires(!construct.is_empty())]
#[ensures(ret.construct() == construct)]
fn start_nested_reason(construct: &str) -> SyntaxExpectationReason {
    new!(SyntaxExpectationReason::StartNested {
        construct: construct.to_owned(),
    })
}

#[requires(!context.is_empty())]
#[ensures(true)]
fn apply_context_to_group(group: &mut ExpectedTokenGroup, context: &str) {
    let reason = match &group.reason {
        Some(reason) => match reason.as_data() {
            data!(SyntaxExpectationReason::EndThenStart { starts, ends })
                if !ends.iter().any(|end| end == context) =>
            {
                let mut ends = ends.clone();
                ends.push(context.to_owned());
                Some(new!(SyntaxExpectationReason::EndThenStart {
                    starts: starts.clone(),
                    ends,
                }))
            }
            _ => None,
        },
        None if group.tokens.iter().any(is_end_of_input_token) => {
            Some(new!(SyntaxExpectationReason::EndThenStart {
                starts: "end of input".to_owned(),
                ends: vec![context.to_owned()],
            }))
        }
        None => Some(new!(SyntaxExpectationReason::ContinueCurrent {
            construct: context.to_owned(),
        })),
    };
    if let Some(reason) = reason {
        *group = group.clone().with_data(data! {
            reason: Some(reason),
        });
    }
}

#[requires(true)]
#[ensures(true)]
fn is_end_of_input_token(token: &SyntaxExpectedToken) -> bool {
    matches!(token.as_data(), data!(SyntaxExpectedToken::EndOfInput))
}

#[requires(!tokens.is_empty())]
#[ensures(!ret.construct().is_empty())]
fn expectation_reason(
    tokens: &[SyntaxExpectedToken],
    contexts: &[String],
) -> SyntaxExpectationReason {
    if tokens
        .iter()
        .any(|token| matches!(token.as_data(), data!(SyntaxExpectedToken::EndOfInput)))
    {
        return new!(SyntaxExpectationReason::EndThenStart {
            starts: "end of input".to_owned(),
            ends: contexts.to_vec(),
        });
    }
    let construct = contexts
        .first()
        .cloned()
        .unwrap_or_else(|| "syntax construct".to_owned());
    if contexts.len() > 1 {
        new!(SyntaxExpectationReason::StartNested { construct })
    } else {
        new!(SyntaxExpectationReason::ContinueCurrent { construct })
    }
}

#[requires(true)]
#[ensures(!ret.construct().is_empty())]
fn normalize_expectation_reason(
    reason: SyntaxExpectationReason,
    paths: &[Vec<SyntaxConstructContext>],
) -> SyntaxExpectationReason {
    let Some(current_context) = select_current_context(paths) else {
        return reason;
    };
    match reason.into_data() {
        data!(SyntaxExpectationReason::ContinueCurrent { construct }) => {
            if construct == current_context.construct {
                new!(SyntaxExpectationReason::ContinueCurrent { construct })
            } else if let Some(child) =
                immediate_child_under_current(&current_context.construct, &construct, paths)
            {
                new!(SyntaxExpectationReason::StartNested { construct: child })
            } else if let Some(construct) =
                external_start_construct(&current_context.construct, &construct)
            {
                new!(SyntaxExpectationReason::StartNested { construct })
            } else {
                new!(SyntaxExpectationReason::ContinueCurrent { construct })
            }
        }
        data!(SyntaxExpectationReason::StartNested { construct }) => {
            let construct =
                immediate_child_under_current(&current_context.construct, &construct, paths)
                    .or_else(|| external_start_construct(&current_context.construct, &construct))
                    .unwrap_or(construct);
            new!(SyntaxExpectationReason::StartNested { construct })
        }
        data!(SyntaxExpectationReason::EndThenStart { starts, ends }) => {
            let starts = immediate_child_under_current(&current_context.construct, &starts, paths)
                .or_else(|| external_start_construct(&current_context.construct, &starts))
                .unwrap_or(starts);
            new!(SyntaxExpectationReason::EndThenStart { starts, ends })
        }
    }
}

#[requires(!current.is_empty())]
#[requires(!construct.is_empty())]
#[ensures(ret.as_ref().is_none_or(|construct| !construct.is_empty()))]
fn external_start_construct(current: &str, construct: &str) -> Option<String> {
    if current != "free modifier"
        && (construct == "free modifier"
            || syntax_construct_is_descendant_of("free modifier", construct))
    {
        Some("free modifier".to_owned())
    } else {
        None
    }
}

#[requires(!current.is_empty())]
#[requires(!descendant.is_empty())]
#[ensures(ret.as_ref().is_none_or(|child| !child.is_empty()))]
fn immediate_child_under_current(
    current: &str,
    descendant: &str,
    paths: &[Vec<SyntaxConstructContext>],
) -> Option<String> {
    if current == descendant {
        return None;
    }
    immediate_child_from_context_paths(current, descendant, paths)
        .or_else(|| syntax_immediate_child_under(current, descendant))
        .or_else(|| immediate_child_under_forethought_parent(current, descendant))
}

#[requires(!current.is_empty())]
#[requires(!descendant.is_empty())]
#[ensures(ret.as_ref().is_none_or(|child| !child.is_empty()))]
fn immediate_child_under_forethought_parent(current: &str, descendant: &str) -> Option<String> {
    if !current.starts_with("forethought ") {
        return None;
    }
    let parent = syntax_construct_parent(current)?;
    syntax_immediate_child_under(parent, descendant)
}

#[requires(!current.is_empty())]
#[requires(!descendant.is_empty())]
#[ensures(ret.as_ref().is_none_or(|child| !child.is_empty()))]
fn immediate_child_from_context_paths(
    current: &str,
    descendant: &str,
    paths: &[Vec<SyntaxConstructContext>],
) -> Option<String> {
    for path in paths {
        let Some(current_index) = path.iter().position(|context| context.construct == current)
        else {
            continue;
        };
        let Some(descendant_index) = path
            .iter()
            .position(|context| context.construct == descendant)
        else {
            continue;
        };
        if descendant_index < current_index && current_index > 0 {
            return Some(path[current_index - 1].construct.clone());
        }
    }
    None
}

#[requires(true)]
#[ensures(ret.is_empty())]
fn empty_context_paths() -> Vec<Vec<SyntaxConstructContext>> {
    Vec::new()
}

#[requires(true)]
#[ensures(true)]
fn select_parser_error<'tokens>(
    left: SyntaxParseError<'tokens>,
    right: SyntaxParseError<'tokens>,
) -> SyntaxParseError<'tokens> {
    match right.span.start.cmp(&left.span.start) {
        std::cmp::Ordering::Greater => right,
        std::cmp::Ordering::Less => left,
        std::cmp::Ordering::Equal if left.same_report_content(&right) => left,
        std::cmp::Ordering::Equal => {
            match parser_error_context_depth(&right).cmp(&parser_error_context_depth(&left)) {
                std::cmp::Ordering::Greater => right,
                _ => left,
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn parser_error_context_depth(error: &SyntaxParseError<'_>) -> usize {
    error
        .preferred_context()
        .map(|context| syntax_construct_depth(&context.construct))
        .unwrap_or(0)
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|context| !context.construct.is_empty()))]
fn deeper_preferred_context(
    left: Option<SyntaxConstructContext>,
    right: Option<SyntaxConstructContext>,
) -> Option<SyntaxConstructContext> {
    match (left, right) {
        (None, None) => None,
        (Some(context), None) | (None, Some(context)) => Some(context),
        (Some(left), Some(right))
            if syntax_construct_depth(&right.construct)
                > syntax_construct_depth(&left.construct) =>
        {
            Some(right)
        }
        (Some(left), Some(_right)) => Some(left),
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|context| !context.construct.is_empty()))]
fn preferred_context_from_branches(
    branches: &[Arc<SyntaxParseError<'_>>],
) -> Option<SyntaxConstructContext> {
    let mut selected = None;
    for branch in branches {
        selected = deeper_preferred_context(selected, branch.preferred_context());
    }
    selected
}

#[requires(!frame.construct().is_empty())]
#[ensures(!ret.construct.is_empty())]
#[ensures(ret.byte_start <= ret.byte_end)]
fn syntax_context_from_frame(frame: &SyntaxContextFrame, span: Span) -> SyntaxConstructContext {
    SyntaxConstructContext::new(
        frame.construct().to_owned(),
        frame.byte_start().min(span.end),
        span.start.max(span.end),
    )
}

#[requires(true)]
#[ensures(true)]
fn append_active_contexts_to_report_contexts(
    contexts: &mut Vec<SyntaxConstructContext>,
    active_contexts: &[SyntaxContextFrame],
    span: Span,
) {
    let mut reached_report_context = contexts.is_empty();
    for frame in active_contexts.iter().rev() {
        let construct = frame.construct();
        if !syntax_construct_is_known(construct) {
            continue;
        }
        if !reached_report_context {
            reached_report_context = contexts.iter().any(|context| {
                context.construct == construct
                    || syntax_construct_is_descendant_of(&context.construct, construct)
            });
            if !reached_report_context {
                continue;
            }
        }
        let context = syntax_context_from_frame(frame, span);
        if !contexts
            .iter()
            .any(|existing| existing.construct == context.construct)
        {
            contexts.push(context);
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn stretch_report_contexts_to_error(
    contexts: Vec<SyntaxConstructContext>,
    span: Span,
) -> Vec<SyntaxConstructContext> {
    let byte_end = span.start.max(span.end);
    contexts
        .into_iter()
        .map(|context| {
            SyntaxConstructContext::new(
                context.construct.clone(),
                context.byte_start.min(byte_end),
                byte_end,
            )
        })
        .collect()
}

#[requires(true)]
#[ensures(ret.same_position_branches.is_empty())]
fn merge_report_errors<'tokens>(
    left: SyntaxParseError<'tokens>,
    right: SyntaxParseError<'tokens>,
) -> SyntaxParseError<'tokens> {
    let preferred_context_hint =
        deeper_preferred_context(left.preferred_context(), right.preferred_context());
    let mut left = left.into_report_error().into_data();
    let right = right.into_report_error().into_data();
    left.active_contexts = deeper_active_context_stack(left.active_contexts, right.active_contexts);
    append_unique_groups(&mut left.expected_groups, right.expected_groups);
    append_unique_context_paths(&mut left.context_paths, right.context_paths);
    left.found = merge_optional_equal(left.found, right.found);
    left.custom_kind = merge_optional_equal(left.custom_kind, right.custom_kind);
    left.preferred_context_hint = preferred_context_hint;
    left.same_position_branches = Vec::new();
    SyntaxParseError::from_data(left)
}

#[requires(true)]
#[ensures(true)]
fn deeper_active_context_stack(
    left: Vec<SyntaxContextFrame>,
    right: Vec<SyntaxContextFrame>,
) -> Vec<SyntaxContextFrame> {
    if right.len() > left.len() {
        right
    } else {
        left
    }
}

#[requires(true)]
#[ensures(true)]
fn arc_into_inner_or_clone<T: Clone>(value: Arc<T>) -> T {
    Arc::try_unwrap(value).unwrap_or_else(|value| (*value).clone())
}

#[requires(true)]
#[ensures(true)]
fn push_context_to_paths(
    paths: &mut Vec<Vec<SyntaxConstructContext>>,
    context: SyntaxConstructContext,
) {
    if paths.is_empty() {
        paths.push(Vec::new());
    }
    for path in paths {
        path.push(context.clone());
    }
}

#[requires(true)]
#[ensures(true)]
fn append_unique_context_paths(
    target: &mut Vec<Vec<SyntaxConstructContext>>,
    source: Vec<Vec<SyntaxConstructContext>>,
) {
    if source.is_empty() {
        if !target.is_empty() && !target.iter().any(Vec::is_empty) {
            target.push(Vec::new());
        }
        return;
    }
    if target.is_empty() {
        target.push(Vec::new());
    }
    for path in source {
        if !target.contains(&path) {
            target.push(path);
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn merged_context_names(paths: &[Vec<SyntaxConstructContext>]) -> Vec<String> {
    let mut names = Vec::new();
    for path in paths {
        for context in path {
            if !names.contains(&context.construct) {
                names.push(context.construct.clone());
            }
        }
    }
    names
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|context| !context.construct.is_empty()))]
fn select_current_context(paths: &[Vec<SyntaxConstructContext>]) -> Option<SyntaxConstructContext> {
    select_shared_innermost_context(paths).or_else(|| select_outer_common_context(paths))
}

#[requires(true)]
#[ensures(ret.len() <= limit)]
fn select_report_contexts(
    paths: &[Vec<SyntaxConstructContext>],
    limit: usize,
) -> Vec<SyntaxConstructContext> {
    if limit == 0 || paths.is_empty() {
        return Vec::new();
    }
    if let Some(current_context) = select_current_context(paths)
        && let Some(path) = report_path_from_current_context(paths, &current_context)
    {
        return normalize_report_contexts(path, limit);
    }
    let prefix_len = common_innermost_prefix_len(paths);
    let selected = if prefix_len > 0 {
        paths[0][..prefix_len].to_vec()
    } else {
        common_outer_suffix(paths)
    };
    normalize_report_contexts(selected, limit)
}

#[requires(true)]
#[ensures(true)]
fn report_path_from_current_context(
    paths: &[Vec<SyntaxConstructContext>],
    current_context: &SyntaxConstructContext,
) -> Option<Vec<SyntaxConstructContext>> {
    let mut selected = None::<Vec<SyntaxConstructContext>>;
    for path in paths {
        let Some(index) = path
            .iter()
            .position(|context| context == current_context)
            .or_else(|| {
                path.iter()
                    .position(|context| context.construct == current_context.construct)
            })
        else {
            continue;
        };
        let suffix = path[index..].to_vec();
        if selected
            .as_ref()
            .is_none_or(|previous| suffix.len() > previous.len())
        {
            selected = Some(suffix);
        }
    }
    selected
}

#[requires(true)]
#[ensures(ret.len() <= limit)]
fn normalize_report_contexts(
    contexts: Vec<SyntaxConstructContext>,
    limit: usize,
) -> Vec<SyntaxConstructContext> {
    contexts
        .into_iter()
        .filter(|context| !syntax_construct_is_root(&context.construct))
        .fold(Vec::new(), |mut contexts, context| {
            if !contexts.contains(&context) && contexts.len() < limit {
                contexts.push(context);
            }
            contexts
        })
}

#[requires(!paths.is_empty())]
#[ensures(ret <= paths.iter().map(Vec::len).min().unwrap_or(0))]
fn common_innermost_prefix_len(paths: &[Vec<SyntaxConstructContext>]) -> usize {
    let shortest_path_len = paths.iter().map(Vec::len).min().unwrap_or(0);
    let mut len = 0;
    for index in 0..shortest_path_len {
        let candidate = &paths[0][index];
        if paths.iter().all(|path| path.get(index) == Some(candidate)) {
            len += 1;
        } else {
            break;
        }
    }
    len
}

#[requires(!paths.is_empty())]
#[ensures(true)]
fn common_outer_suffix(paths: &[Vec<SyntaxConstructContext>]) -> Vec<SyntaxConstructContext> {
    let shortest_path_len = paths.iter().map(Vec::len).min().unwrap_or(0);
    let mut contexts = Vec::new();
    for outer_index in 0..shortest_path_len {
        let candidate = &paths[0][paths[0].len() - 1 - outer_index];
        if paths
            .iter()
            .all(|path| path.get(path.len() - 1 - outer_index) == Some(candidate))
        {
            contexts.push(candidate.clone());
        } else {
            break;
        }
    }
    contexts.reverse();
    contexts
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|context| !context.construct.is_empty()))]
fn select_shared_innermost_context(
    paths: &[Vec<SyntaxConstructContext>],
) -> Option<SyntaxConstructContext> {
    let selected = paths.first()?.first()?;
    if syntax_construct_is_root(&selected.construct) {
        return None;
    }
    if paths.iter().all(|path| path.first() == Some(selected)) {
        Some(selected.clone())
    } else {
        None
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|context| !context.construct.is_empty()))]
fn select_outer_common_context(
    paths: &[Vec<SyntaxConstructContext>],
) -> Option<SyntaxConstructContext> {
    let selected = select_outer_common_context_including_roots(paths)?;
    if syntax_construct_is_root(&selected.construct) {
        None
    } else {
        Some(selected)
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|context| !context.construct.is_empty()))]
fn select_outer_common_context_including_roots(
    paths: &[Vec<SyntaxConstructContext>],
) -> Option<SyntaxConstructContext> {
    let shortest_path_len = paths.iter().map(Vec::len).min()?;
    let mut selected = None;
    for outer_index in 0..shortest_path_len {
        let candidate = &paths[0][paths[0].len() - 1 - outer_index];
        if paths
            .iter()
            .all(|path| path.get(path.len() - 1 - outer_index) == Some(candidate))
        {
            selected = Some(candidate);
        } else {
            break;
        }
    }
    selected.cloned()
}

#[requires(true)]
#[ensures(true)]
fn append_unique_groups(target: &mut Vec<ExpectedTokenGroup>, source: Vec<ExpectedTokenGroup>) {
    for group in source {
        if !group.tokens.is_empty() && !target.contains(&group) {
            target.push(group);
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn syntax_found_from_maybe(found: Option<MaybeRef<'_, Token>>) -> SyntaxFound {
    found
        .map(|found| new!(SyntaxFound::Token(found.into_inner())))
        .unwrap_or_else(|| new!(SyntaxFound::EndOfInput))
}

#[requires(true)]
#[ensures(true)]
fn merge_optional_equal<T: PartialEq>(left: Option<T>, right: Option<T>) -> Option<T> {
    match (left, right) {
        (Some(left), Some(right)) if left == right => Some(left),
        (Some(_), Some(_)) => None,
        (Some(value), None) | (None, Some(value)) => Some(value),
        (None, None) => None,
    }
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use bityzba::{ensures, requires};

    use super::*;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn labelled_error_records_start_nested_reason() {
        let mut error = SyntaxParseError::expected(Span::from(4..6), vec![named_token("lo")]);
        label_with(&mut error, "sumti");

        let expectations = error.expectations();
        assert_eq!(expectations.len(), 1);
        match expectations[0].reason.as_data() {
            data!(SyntaxExpectationReason::StartNested { construct }) => {
                assert_eq!(construct, "sumti");
            }
            other => panic!("expected start-nested reason, got {other:?}"),
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn contextual_eof_records_end_then_start_reason() {
        let mut error = SyntaxParseError::expected(
            Span::from(4..4),
            vec![new!(SyntaxExpectedToken::EndOfInput)],
        );
        in_context(&mut error, "selbri");
        in_context(&mut error, "text");

        let expectations = error.expectations();
        assert_eq!(expectations.len(), 1);
        match expectations[0].reason.as_data() {
            data!(SyntaxExpectationReason::EndThenStart { starts, ends }) => {
                assert_eq!(starts, "end of input");
                assert_eq!(ends, &["selbri".to_owned(), "text".to_owned()]);
            }
            other => panic!("expected end-then-start reason, got {other:?}"),
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn merge_for_report_preserves_branch_expectation_reasons() {
        let mut selbri = SyntaxParseError::expected(Span::from(4..6), vec![named_token("be")]);
        in_context(&mut selbri, "selbri");
        let mut sumti = SyntaxParseError::expected(Span::from(4..6), vec![named_token("lo")]);
        label_with(&mut sumti, "sumti");

        let merged = selbri.merge_for_report(sumti);
        let expectations = merged.expectations();

        assert_eq!(expectations.len(), 2);
        assert!(expectations.iter().any(|expectation| matches!(
            expectation.reason.as_data(),
            data!(SyntaxExpectationReason::ContinueCurrent { construct }) if construct == "selbri"
        )));
        assert!(expectations.iter().any(|expectation| matches!(
            expectation.reason.as_data(),
            data!(SyntaxExpectationReason::StartNested { construct }) if construct == "sumti"
        )));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn parser_merge_selects_deeper_context_without_branch_bundle() {
        let mut shallow = SyntaxParseError::expected(Span::from(4..6), vec![named_token("lo")]);
        in_context(&mut shallow, "text");
        let mut deep = SyntaxParseError::expected(Span::from(4..6), vec![named_token("le")]);
        in_context(&mut deep, "sumti");

        let merged = shallow.merge_for_parser(deep);

        assert!(merged.same_position_branches.is_empty());
        assert_eq!(
            merged
                .preferred_context()
                .map(|context| context.construct.clone()),
            Some("sumti".to_owned())
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn cloned_errors_share_payload_until_context_is_changed() {
        let stored = SyntaxParseError::expected(Span::from(4..6), vec![named_token("lo")]);
        let mut replayed = stored.clone();

        assert!(Rc::ptr_eq(&stored.data, &replayed.data));
        in_context(&mut replayed, "sumti");
        assert!(!Rc::ptr_eq(&stored.data, &replayed.data));
        assert!(stored.current_context().is_none());
        assert_eq!(
            replayed
                .current_context()
                .map(|context| context.construct.clone()),
            Some("sumti".to_owned())
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn current_context_uses_single_branch_innermost_context() {
        let mut error = SyntaxParseError::expected(Span::from(8..10), vec![named_token("lo")]);
        in_context_span(&mut error, "selbri", 0..8);
        in_context_span(&mut error, "statement", 0..8);
        in_context_span(&mut error, "text", 0..8);

        let context = error.current_context().expect("selected context");

        assert_eq!(context.construct, "selbri");
        assert_eq!([context.byte_start, context.byte_end], [0, 8]);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn current_context_peels_to_common_parent_across_branches() {
        let mut sumti = SyntaxParseError::expected(Span::from(8..10), vec![named_token("lo")]);
        in_context_span(&mut sumti, "sumti", 4..8);
        in_context_span(&mut sumti, "selbri", 0..8);
        in_context_span(&mut sumti, "statement", 0..8);
        in_context_span(&mut sumti, "text", 0..8);
        let mut term = SyntaxParseError::expected(Span::from(8..10), vec![named_token("fa")]);
        in_context_span(&mut term, "term", 4..8);
        in_context_span(&mut term, "selbri", 0..8);
        in_context_span(&mut term, "statement", 0..8);
        in_context_span(&mut term, "text", 0..8);

        let context = sumti
            .merge_for_report(term)
            .current_context()
            .expect("selected context");

        assert_eq!(context.construct, "selbri");
        assert_eq!([context.byte_start, context.byte_end], [0, 8]);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn current_context_prefers_shared_innermost_context_across_divergent_routes() {
        let mut via_relation =
            SyntaxParseError::expected(Span::from(8..10), vec![named_token("lo")]);
        in_context_span(&mut via_relation, "sumti", 4..8);
        in_context_span(&mut via_relation, "term", 4..8);
        in_context_span(&mut via_relation, "selbri", 0..8);
        in_context_span(&mut via_relation, "statement", 0..8);
        in_context_span(&mut via_relation, "text", 0..8);
        let mut via_free = SyntaxParseError::expected(Span::from(8..10), vec![named_token("le")]);
        in_context_span(&mut via_free, "sumti", 4..8);
        in_context_span(&mut via_free, "term", 4..8);
        in_context_span(&mut via_free, "free modifier", 2..8);
        in_context_span(&mut via_free, "statement", 0..8);
        in_context_span(&mut via_free, "text", 0..8);

        let context = via_relation
            .merge_for_report(via_free)
            .current_context()
            .expect("selected context");

        assert_eq!(context.construct, "sumti");
        assert_eq!([context.byte_start, context.byte_end], [4, 8]);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn current_context_omits_root_only_ambiguity() {
        let mut sumti = SyntaxParseError::expected(Span::from(8..10), vec![named_token("lo")]);
        in_context_span(&mut sumti, "sumti", 0..8);
        in_context_span(&mut sumti, "text", 0..8);
        let mut selbri = SyntaxParseError::expected(Span::from(8..10), vec![named_token("ga")]);
        in_context_span(&mut selbri, "selbri", 0..8);
        in_context_span(&mut selbri, "text", 0..8);

        assert!(sumti.merge_for_report(selbri).current_context().is_none());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn current_context_treats_matching_construct_with_different_span_as_ambiguous() {
        let mut first = SyntaxParseError::expected(Span::from(8..10), vec![named_token("lo")]);
        in_context_span(&mut first, "sumti", 0..8);
        in_context_span(&mut first, "statement", 0..8);
        in_context_span(&mut first, "text", 0..8);
        let mut second = SyntaxParseError::expected(Span::from(8..10), vec![named_token("le")]);
        in_context_span(&mut second, "sumti", 3..8);
        in_context_span(&mut second, "statement", 0..8);
        in_context_span(&mut second, "text", 0..8);

        let context = first
            .merge_for_report(second)
            .current_context()
            .expect("selected context");

        assert_eq!(context.construct, "statement");
        assert_eq!([context.byte_start, context.byte_end], [0, 8]);
    }

    #[requires(!text.is_empty())]
    #[ensures(true)]
    fn named_token(text: &str) -> SyntaxExpectedToken {
        new!(SyntaxExpectedToken::Named(text.to_owned()))
    }

    #[requires(!label.is_empty())]
    #[ensures(true)]
    fn label_with(error: &mut SyntaxParseError<'static>, label: &'static str) {
        <SyntaxParseError<'static> as LabelError<'static, &'static str>>::label_with(error, label);
    }

    #[requires(!label.is_empty())]
    #[ensures(true)]
    fn in_context(error: &mut SyntaxParseError<'static>, label: &'static str) {
        in_context_span(error, label, 0..0);
    }

    #[requires(!label.is_empty())]
    #[ensures(true)]
    fn in_context_span(
        error: &mut SyntaxParseError<'static>,
        label: &'static str,
        span: std::ops::Range<usize>,
    ) {
        <SyntaxParseError<'static> as LabelError<'static, &'static str>>::in_context(
            error,
            label,
            Span::from(span),
        );
    }
}
