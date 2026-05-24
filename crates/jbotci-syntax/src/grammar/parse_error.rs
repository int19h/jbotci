#[allow(unused_imports)]
use bityzba::{data, ensures, invariant, new, requires};
use chumsky::error::{Error, LabelError, Rich, RichPattern, RichReason};
use chumsky::input::Input;
use chumsky::util::MaybeRef;

use super::{Span, Token};
use crate::{
    SyntaxExpectation, SyntaxExpectationReason, SyntaxExpectationReasonData, SyntaxExpectedToken,
    SyntaxExpectedTokenData,
};

#[invariant(true)]
#[derive(Debug, Clone)]
pub(super) struct SyntaxParseError<'tokens> {
    inner: Rich<'tokens, Token, Span>,
    expected_groups: Vec<ExpectedTokenGroup>,
    contexts: Vec<String>,
}

#[invariant(!tokens.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
struct ExpectedTokenGroup {
    tokens: Vec<SyntaxExpectedToken>,
    reason: Option<SyntaxExpectationReason>,
}

impl ExpectedTokenGroup {
    #[requires(!tokens.is_empty())]
    #[ensures(!ret.tokens.is_empty())]
    fn new(tokens: Vec<SyntaxExpectedToken>) -> Self {
        new!(ExpectedTokenGroup {
            tokens,
            reason: None,
        })
    }

    #[requires(!tokens.is_empty())]
    #[ensures(!ret.tokens.is_empty())]
    fn with_optional_reason(
        tokens: Vec<SyntaxExpectedToken>,
        reason: Option<SyntaxExpectationReason>,
    ) -> Self {
        new!(ExpectedTokenGroup { tokens, reason })
    }
}

impl<'tokens> SyntaxParseError<'tokens> {
    #[requires(!message.is_empty())]
    #[ensures(ret.expected_groups.is_empty())]
    pub(super) fn custom(span: Span, message: String) -> Self {
        Self {
            inner: Rich::custom(span, message),
            expected_groups: Vec::new(),
            contexts: Vec::new(),
        }
    }

    #[requires(!tokens.is_empty())]
    #[ensures(ret.expected_groups.len() == 1)]
    pub(super) fn expected(span: Span, tokens: Vec<SyntaxExpectedToken>) -> Self {
        Self {
            inner: Rich::custom(span, "unexpected input".to_owned()),
            expected_groups: vec![ExpectedTokenGroup::new(tokens)],
            contexts: Vec::new(),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn span(&self) -> &Span {
        self.inner.span()
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn reason(&self) -> &RichReason<'tokens, Token> {
        self.inner.reason()
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn expected_strings(&self) -> Vec<String> {
        self.inner
            .expected()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
    }

    #[requires(true)]
    #[ensures(ret.iter().all(|expectation| !expectation.tokens.is_empty()))]
    pub(super) fn expectations(&self) -> Vec<SyntaxExpectation> {
        let mut expectations = Vec::new();
        for group in &self.expected_groups {
            if !group.tokens.is_empty() {
                let reason = group
                    .reason
                    .clone()
                    .unwrap_or_else(|| expectation_reason(&group.tokens, &self.contexts));
                expectations.push(SyntaxExpectation::new(group.tokens.clone(), reason));
            }
        }
        if expectations.is_empty() {
            for token in self
                .inner
                .expected()
                .filter_map(syntax_expected_token_from_rich_pattern)
            {
                expectations.push(SyntaxExpectation::new(
                    vec![token],
                    expectation_reason(
                        &[new!(SyntaxExpectedToken::Named("input".to_owned()))],
                        &self.contexts,
                    ),
                ));
            }
        }
        expectations
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn merge_for_report(mut self, other: Self) -> Self {
        append_unique_groups(&mut self.expected_groups, other.expected_groups);
        append_unique_strings(&mut self.contexts, other.contexts);
        self
    }
}

impl<'tokens, I> Error<'tokens, I> for SyntaxParseError<'tokens>
where
    I: Input<'tokens, Token = Token, Span = Span>,
    Token: PartialEq,
{
    #[requires(true)]
    #[ensures(true)]
    fn merge(self, other: Self) -> Self {
        let mut merged = self;
        merged.inner =
            <Rich<'tokens, Token, Span> as Error<'tokens, I>>::merge(merged.inner, other.inner);
        append_unique_groups(&mut merged.expected_groups, other.expected_groups);
        append_unique_strings(&mut merged.contexts, other.contexts);
        merged
    }
}

impl<'tokens, I, L> LabelError<'tokens, I, L> for SyntaxParseError<'tokens>
where
    I: Input<'tokens, Token = Token, Span = Span>,
    Token: PartialEq,
    L: TryInto<RichPattern<'tokens, Token>> + Clone,
{
    #[requires(true)]
    #[ensures(true)]
    fn expected_found<E: IntoIterator<Item = L>>(
        expected: E,
        found: Option<MaybeRef<'tokens, I::Token>>,
        span: I::Span,
    ) -> Self {
        let expected = expected.into_iter().collect::<Vec<_>>();
        let inner = <Rich<'tokens, Token, Span> as LabelError<'tokens, I, L>>::expected_found(
            expected.clone(),
            found,
            span,
        );
        let expected_groups = expected_token_groups_from_labels(expected);
        Self {
            inner,
            expected_groups,
            contexts: Vec::new(),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn merge_expected_found<E: IntoIterator<Item = L>>(
        mut self,
        expected: E,
        found: Option<MaybeRef<'tokens, I::Token>>,
        span: I::Span,
    ) -> Self
    where
        Self: Error<'tokens, I>,
    {
        let expected = expected.into_iter().collect::<Vec<_>>();
        append_unique_groups(
            &mut self.expected_groups,
            expected_token_groups_from_labels(expected.clone()),
        );
        self.inner =
            <Rich<'tokens, Token, Span> as LabelError<'tokens, I, L>>::merge_expected_found(
                self.inner, expected, found, span,
            );
        self
    }

    #[requires(true)]
    #[ensures(true)]
    fn replace_expected_found<E: IntoIterator<Item = L>>(
        mut self,
        expected: E,
        found: Option<MaybeRef<'tokens, I::Token>>,
        span: I::Span,
    ) -> Self {
        let expected = expected.into_iter().collect::<Vec<_>>();
        self.expected_groups = expected_token_groups_from_labels(expected.clone());
        self.inner =
            <Rich<'tokens, Token, Span> as LabelError<'tokens, I, L>>::replace_expected_found(
                self.inner, expected, found, span,
            );
        self.contexts.clear();
        self
    }

    #[requires(true)]
    #[ensures(true)]
    fn label_with(&mut self, label: L) {
        <Rich<'tokens, Token, Span> as LabelError<'tokens, I, L>>::label_with(
            &mut self.inner,
            label.clone(),
        );
        let Some(pattern) = label.try_into().ok() else {
            return;
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
                    vec![token],
                    reason,
                ));
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn in_context(&mut self, label: L, span: I::Span) {
        <Rich<'tokens, Token, Span> as LabelError<'tokens, I, L>>::in_context(
            &mut self.inner,
            label.clone(),
            span,
        );
        if let Some(context) = label
            .try_into()
            .ok()
            .and_then(|pattern| context_from_rich_pattern(&pattern))
            && !self.contexts.contains(&context)
        {
            for group in &mut self.expected_groups {
                apply_context_to_group(group, &context);
            }
            self.contexts.push(context);
        }
    }
}

#[requires(true)]
#[ensures(ret.iter().all(|group| !group.tokens.is_empty()))]
fn expected_token_groups_from_labels<'tokens, L>(labels: Vec<L>) -> Vec<ExpectedTokenGroup>
where
    L: TryInto<RichPattern<'tokens, Token>>,
{
    labels
        .into_iter()
        .filter_map(|label| {
            label
                .try_into()
                .ok()
                .and_then(|pattern| syntax_expected_token_from_rich_pattern(&pattern))
                .map(|token| ExpectedTokenGroup::new(vec![token]))
        })
        .collect()
}

#[requires(true)]
#[ensures(true)]
fn syntax_expected_token_from_rich_pattern(
    pattern: &RichPattern<'_, Token>,
) -> Option<SyntaxExpectedToken> {
    match pattern {
        RichPattern::Token(token) => token
            .core_word()
            .cmavo()
            .map(|cmavo| new!(SyntaxExpectedToken::Cmavo(cmavo))),
        RichPattern::Label(label) => Some(new!(SyntaxExpectedToken::Named(label.to_string()))),
        RichPattern::Identifier(identifier) => {
            Some(new!(SyntaxExpectedToken::Named(identifier.clone())))
        }
        RichPattern::Any => Some(new!(SyntaxExpectedToken::Named("input".to_owned()))),
        RichPattern::SomethingElse => {
            Some(new!(SyntaxExpectedToken::Named("other input".to_owned())))
        }
        RichPattern::EndOfInput => Some(new!(SyntaxExpectedToken::EndOfInput)),
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn context_from_rich_pattern(pattern: &RichPattern<'_, Token>) -> Option<String> {
    match pattern {
        RichPattern::Label(label) => Some(label.to_string()),
        RichPattern::Identifier(identifier) => Some(identifier.clone()),
        _ => None,
    }
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
fn append_unique_strings(target: &mut Vec<String>, source: Vec<String>) {
    for item in source {
        if !target.contains(&item) {
            target.push(item);
        }
    }
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use bityzba::{ensures, requires};

    use crate::grammar::ParserInput;

    use super::*;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn labelled_error_records_start_nested_reason() {
        let mut error = SyntaxParseError::expected(Span::from(4..6), vec![named_token("lo")]);
        label_with(&mut error, "argument");

        let expectations = error.expectations();
        assert_eq!(expectations.len(), 1);
        match expectations[0].reason.as_data() {
            data!(SyntaxExpectationReason::StartNested { construct }) => {
                assert_eq!(construct, "argument");
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
        in_context(&mut error, "relation");
        in_context(&mut error, "text");

        let expectations = error.expectations();
        assert_eq!(expectations.len(), 1);
        match expectations[0].reason.as_data() {
            data!(SyntaxExpectationReason::EndThenStart { starts, ends }) => {
                assert_eq!(starts, "end of input");
                assert_eq!(ends, &["relation".to_owned(), "text".to_owned()]);
            }
            other => panic!("expected end-then-start reason, got {other:?}"),
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn merge_for_report_preserves_branch_expectation_reasons() {
        let mut relation = SyntaxParseError::expected(Span::from(4..6), vec![named_token("be")]);
        in_context(&mut relation, "relation");
        let mut argument = SyntaxParseError::expected(Span::from(4..6), vec![named_token("lo")]);
        label_with(&mut argument, "argument");

        let merged = relation.merge_for_report(argument);
        let expectations = merged.expectations();

        assert_eq!(expectations.len(), 2);
        assert!(expectations.iter().any(|expectation| matches!(
            expectation.reason.as_data(),
            data!(SyntaxExpectationReason::ContinueCurrent { construct }) if construct == "relation"
        )));
        assert!(expectations.iter().any(|expectation| matches!(
            expectation.reason.as_data(),
            data!(SyntaxExpectationReason::StartNested { construct }) if construct == "argument"
        )));
    }

    #[requires(!text.is_empty())]
    #[ensures(true)]
    fn named_token(text: &str) -> SyntaxExpectedToken {
        new!(SyntaxExpectedToken::Named(text.to_owned()))
    }

    #[requires(!label.is_empty())]
    #[ensures(true)]
    fn label_with(error: &mut SyntaxParseError<'static>, label: &'static str) {
        <SyntaxParseError<'static> as LabelError<
            'static,
            ParserInput<'static>,
            &'static str,
        >>::label_with(error, label);
    }

    #[requires(!label.is_empty())]
    #[ensures(true)]
    fn in_context(error: &mut SyntaxParseError<'static>, label: &'static str) {
        <SyntaxParseError<'static> as LabelError<
            'static,
            ParserInput<'static>,
            &'static str,
        >>::in_context(error, label, Span::from(0..0));
    }
}
