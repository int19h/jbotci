//! Jvozba presentation: the built word and its ordered constituents.

#[allow(unused_imports)]
use bityzba::{ensures, invariant, new, requires};
use jbotci_jvozba::JvozbaInput;

use super::RenderedResult;
use super::markdown::{
    escape_in_scope, inline_code_in_scope, join_lines_in_scope, subtext_markdown_in_scope,
};
use crate::discord::operations::JvozbaOutcome;
use crate::discord::request::{DiscordTool, JvozbaRequest, JvozbaTarget};

#[requires(true)]
#[ensures(text_scope.is_exceeded() || (ret.tool() == DiscordTool::Jvozba))]
pub(crate) fn render_in_scope(
    outcome: &JvozbaOutcome,
    _request: &JvozbaRequest,
    text_scope: &jbotci_output::budget::TextScope,
) -> RenderedResult {
    let target = match outcome.target {
        JvozbaTarget::Lujvo => "lujvo",
        JvozbaTarget::Cmevla => "cmevla",
    };
    let inputs = text_scope.join(
        outcome.inputs.iter().map(|input| match input {
            JvozbaInput::Word(word) => inline_code_in_scope(word, text_scope),
            JvozbaInput::FixedRafsi(rafsi) => text_scope.format(format_args!(
                "{} (fixed rafsi)",
                inline_code_in_scope(rafsi, text_scope)
            )),
        }),
        " + ",
    );
    match &outcome.result {
        Ok(built) => {
            let mut rendered = RenderedResult::new_in_scope(
                DiscordTool::Jvozba,
                text_scope.format(format_args!("jvozba · {target}")),
                text_scope,
            );
            let mut lines = vec![text_scope.format(format_args!(
                "**{}**",
                escape_in_scope(&built.word, text_scope)
            ))];
            if !inputs.is_empty() {
                lines.push(subtext_markdown_in_scope(
                    &text_scope.format(format_args!("from {inputs}")),
                    text_scope,
                ));
            }
            let constituents = built
                .constituents
                .iter()
                .map(|constituent| {
                    if constituent.is_hyphen {
                        text_scope.format(format_args!(
                            "{} — hyphen",
                            inline_code_in_scope(&constituent.text, text_scope)
                        ))
                    } else {
                        match &constituent.source {
                            Some(source) => text_scope.format(format_args!(
                                "{} — rafsi of {}",
                                inline_code_in_scope(&constituent.text, text_scope),
                                inline_code_in_scope(source, text_scope)
                            )),
                            None => text_scope.format(format_args!(
                                "{} — rafsi",
                                inline_code_in_scope(&constituent.text, text_scope)
                            )),
                        }
                    }
                })
                .collect::<Vec<_>>();
            if !constituents.is_empty() {
                lines.push(text_scope.join(&constituents, "\n"));
            }
            rendered.body.push(join_lines_in_scope(lines, text_scope));
            rendered
        }
        Err(error) => {
            let mut rendered = RenderedResult::new_in_scope(
                DiscordTool::Jvozba,
                text_scope.format(format_args!("jvozba · {target} · not built")),
                text_scope,
            );
            let mut lines =
                vec![text_scope.format(format_args!(
                        "**Could not build a {target}:** {}",
                        escape_in_scope(
                            &match error {
                                jbotci_jvozba::JvozbaError::TooMuchWork { .. } =>
                                    text_scope.copy(
                                        crate::error::ServerErrorCode::OperationLimit.message()
                                    ),
                                _ => text_scope.format(format_args!("{error}")),
                            },
                            text_scope
                        )
                    ))];
            if !inputs.is_empty() {
                lines.push(subtext_markdown_in_scope(
                    &text_scope.format(format_args!("inputs: {inputs}")),
                    text_scope,
                ));
            }
            rendered.body.push(join_lines_in_scope(lines, text_scope));
            rendered
        }
    }
}

#[cfg(test)]
#[requires(true)]
#[ensures(true)]
pub(crate) fn render(outcome: &JvozbaOutcome, _request: &JvozbaRequest) -> RenderedResult {
    let text_scope = jbotci_output::budget::TextScope::new(
        jbotci_output::budget::ByteLimit::new(usize::MAX).expect("positive unlimited limit"),
    );
    let result = render_in_scope(outcome, _request, &text_scope);
    text_scope
        .finish(String::new())
        .expect("unrestricted Discord rendering");
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::discord::operations::{JvozbaBuilt, JvozbaConstituent};
    use crate::discord::request::{JvozbaOptions, SourceText};
    use jbotci_jvozba::JvozbaError;

    #[requires(true)]
    #[ensures(true)]
    fn request() -> JvozbaRequest {
        JvozbaRequest {
            parts: SourceText::new("klama bajra").expect("text"),
            options: JvozbaOptions::default(),
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn built_word_lists_constituents_with_sources() {
        let outcome = JvozbaOutcome {
            inputs: vec![
                JvozbaInput::Word("klama".to_owned()),
                JvozbaInput::FixedRafsi("bar".to_owned()),
            ],
            target: JvozbaTarget::Lujvo,
            result: Ok(new!(JvozbaBuilt {
                word: "klabar".to_owned(),
                constituents: vec![
                    new!(JvozbaConstituent {
                        text: "kla".to_owned(),
                        is_hyphen: false,
                        source: Some("klama".to_owned())
                    }),
                    new!(JvozbaConstituent {
                        text: "bar".to_owned(),
                        is_hyphen: false,
                        source: None
                    }),
                ],
            })),
        };
        let rendered = render(&outcome, &request());
        assert_eq!(rendered.status.text, "jvozba · lujvo");
        let body = rendered.body.join("\n");
        assert!(body.contains("**klabar**"), "{body}");
        assert!(body.contains("`kla` — rafsi of `klama`"), "{body}");
        assert!(body.contains("`bar` (fixed rafsi)"), "{body}");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn build_errors_are_typed_messages_not_usage_dumps() {
        let outcome = JvozbaOutcome {
            inputs: vec![JvozbaInput::Word("klama".to_owned())],
            target: JvozbaTarget::Cmevla,
            result: Err(JvozbaError::RequiresAtLeastTwoInputs),
        };
        let rendered = render(&outcome, &request());
        assert_eq!(rendered.status.text, "jvozba · cmevla · not built");
        assert!(rendered.body[0].contains("at least two rafsi-producing inputs"));
        assert!(!rendered.body[0].contains("Usage:"));
    }
}
