//! Jvozba presentation: the built word and its ordered constituents.

#[allow(unused_imports)]
use bityzba::{ensures, invariant, new, requires};
use jbotci_jvozba::JvozbaInput;

use super::RenderedResult;
use super::markdown::{escape, inline_code, join_lines, subtext_markdown};
use crate::discord::operations::JvozbaOutcome;
use crate::discord::request::{DiscordTool, JvozbaRequest, JvozbaTarget};

#[requires(true)]
#[ensures(ret.tool() == DiscordTool::Jvozba)]
pub(crate) fn render(outcome: &JvozbaOutcome, _request: &JvozbaRequest) -> RenderedResult {
    let target = match outcome.target {
        JvozbaTarget::Lujvo => "lujvo",
        JvozbaTarget::Cmevla => "cmevla",
    };
    let inputs = outcome
        .inputs
        .iter()
        .map(|input| match input {
            JvozbaInput::Word(word) => inline_code(word),
            JvozbaInput::FixedRafsi(rafsi) => format!("{} (fixed rafsi)", inline_code(rafsi)),
        })
        .collect::<Vec<_>>()
        .join(" + ");
    match &outcome.result {
        Ok(built) => {
            let mut rendered =
                RenderedResult::new(DiscordTool::Jvozba, format!("jvozba · {target}"));
            let mut lines = vec![format!("**{}**", escape(&built.word))];
            if !inputs.is_empty() {
                lines.push(subtext_markdown(&format!("from {inputs}")));
            }
            let constituents = built
                .constituents
                .iter()
                .map(|constituent| {
                    if constituent.is_hyphen {
                        format!("{} — hyphen", inline_code(&constituent.text))
                    } else {
                        match &constituent.source {
                            Some(source) => format!(
                                "{} — rafsi of {}",
                                inline_code(&constituent.text),
                                inline_code(source)
                            ),
                            None => format!("{} — rafsi", inline_code(&constituent.text)),
                        }
                    }
                })
                .collect::<Vec<_>>();
            if !constituents.is_empty() {
                lines.push(constituents.join("\n"));
            }
            rendered.body.push(join_lines(lines));
            rendered
        }
        Err(error) => {
            let mut rendered = RenderedResult::new(
                DiscordTool::Jvozba,
                format!("jvozba · {target} · not built"),
            );
            let mut lines = vec![format!(
                "**Could not build a {target}:** {}",
                escape(&error.to_string())
            )];
            if !inputs.is_empty() {
                lines.push(subtext_markdown(&format!("inputs: {inputs}")));
            }
            rendered.body.push(join_lines(lines));
            rendered
        }
    }
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
