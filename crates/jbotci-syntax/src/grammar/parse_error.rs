#[allow(unused_imports)]
use bityzba::{data, ensures, invariant, new, requires};
use chumsky::error::{Error, LabelError, Rich, RichPattern, RichReason};
use chumsky::input::Input;
use chumsky::util::MaybeRef;

use super::{Span, Token};
use crate::{
    SyntaxConstructContext, SyntaxExpectation, SyntaxExpectationReason,
    SyntaxExpectationReasonData, SyntaxExpectedToken, SyntaxExpectedTokenData,
    syntax_construct_is_descendant_of, syntax_construct_is_known, syntax_construct_is_root,
    syntax_immediate_child_under,
};

#[invariant(true)]
#[derive(Debug, Clone)]
pub(super) struct SyntaxParseError<'tokens> {
    inner: Rich<'tokens, Token, Span>,
    expected_groups: Vec<ExpectedTokenGroup>,
    context_paths: Vec<Vec<SyntaxConstructContext>>,
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
            context_paths: empty_context_paths(),
        }
    }

    #[requires(!tokens.is_empty())]
    #[ensures(ret.expected_groups.len() == 1)]
    pub(super) fn expected(span: Span, tokens: Vec<SyntaxExpectedToken>) -> Self {
        Self {
            inner: Rich::custom(span, "unexpected input".to_owned()),
            expected_groups: vec![ExpectedTokenGroup::new(tokens)],
            context_paths: empty_context_paths(),
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
        let contexts = merged_context_names(&self.context_paths);
        for group in &self.expected_groups {
            if !group.tokens.is_empty() {
                let reason = group
                    .reason
                    .clone()
                    .unwrap_or_else(|| expectation_reason(&group.tokens, &contexts));
                let reason = normalize_expectation_reason(reason, &self.context_paths);
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
                    normalize_expectation_reason(
                        expectation_reason(
                            &[new!(SyntaxExpectedToken::Named("input".to_owned()))],
                            &contexts,
                        ),
                        &self.context_paths,
                    ),
                ));
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
    pub(super) fn merge_for_report(mut self, other: Self) -> Self {
        append_unique_groups(&mut self.expected_groups, other.expected_groups);
        append_unique_context_paths(&mut self.context_paths, other.context_paths);
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
        append_unique_context_paths(&mut merged.context_paths, other.context_paths);
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
            context_paths: empty_context_paths(),
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
        self.context_paths = empty_context_paths();
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
        <Rich<'tokens, Token, Span> as LabelError<'tokens, I, L>>::in_context(
            &mut self.inner,
            label.clone(),
            span,
        );
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
    let construct = match pattern {
        RichPattern::Label(label) => label.to_string(),
        RichPattern::Identifier(identifier) => identifier.clone(),
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
#[ensures(!ret.is_empty())]
fn empty_context_paths() -> Vec<Vec<SyntaxConstructContext>> {
    vec![Vec::new()]
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
        <SyntaxParseError<'static> as LabelError<
            'static,
            ParserInput<'static>,
            &'static str,
        >>::label_with(error, label);
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
        <SyntaxParseError<'static> as LabelError<
            'static,
            ParserInput<'static>,
            &'static str,
        >>::in_context(error, label, Span::from(span));
    }
}
