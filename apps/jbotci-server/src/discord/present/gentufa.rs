//! Gentufa presentation: source-aware brackets with Discord emphasis, a
//! bounded tree code block, diagnostics and the optional diagram.

#[allow(unused_imports)]
use bityzba::{ensures, invariant, new, requires};
use jbotci_web_core::{
    GentufaBlockRole, GentufaBracketFragment, GentufaSuccess, GentufaTreeRow, GentufaWebResult,
};

use super::RenderedResult;
use super::diagnostics::{render_diagnostics, summary};
use super::markdown::{code_block, escape, join_lines, subtext};
use crate::discord::operations::GentufaOutcome;
use crate::discord::request::{DiscordTool, GentufaRequest, GentufaTextView};

#[requires(true)]
#[ensures(ret.tool() == DiscordTool::Gentufa)]
pub(crate) fn render(outcome: &GentufaOutcome, request: &GentufaRequest) -> RenderedResult {
    let options = request.options;
    let view_label = match options.view {
        GentufaTextView::Brackets => "brackets",
        GentufaTextView::Tree => "tree",
    };
    let source = request.text.as_str();
    match &outcome.result {
        GentufaWebResult::Blank => {
            let mut rendered =
                RenderedResult::new(DiscordTool::Gentufa, "gentufa · empty".to_owned());
            rendered
                .body
                .push("Nothing to parse: the text is empty.".to_owned());
            rendered
        }
        GentufaWebResult::Error(error) => {
            let mut status = format!("gentufa · {view_label}");
            if let Some(counts) = summary(&error.diagnostics) {
                status.push_str(&format!(" · {counts}"));
            }
            let mut rendered = RenderedResult::new(DiscordTool::Gentufa, status);
            rendered
                .body
                .push(format!("**Could not parse:** {}", escape(&error.message)));
            rendered.set_diagnostics(render_diagnostics(source, &error.diagnostics));
            if options.include_diagram {
                rendered.notice = Some(subtext(
                    "No diagram: the text did not parse. The diagram setting stays on for the next successful parse.",
                ));
            }
            rendered
        }
        GentufaWebResult::Success(success) => {
            let recovered = success.diagnostics.iter().any(|diagnostic| {
                diagnostic.severity == jbotci_diagnostics::DiagnosticSeverity::Error
            });
            let mut status = format!("gentufa · {view_label}");
            if options.show_elided {
                status.push_str(" · elided shown");
            }
            if recovered {
                status.push_str(" · recovered parse");
            }
            if let Some(counts) = summary(&success.diagnostics) {
                status.push_str(&format!(" · {counts}"));
            }
            let mut rendered = RenderedResult::new(DiscordTool::Gentufa, status);
            let (body, full_text) = match options.view {
                GentufaTextView::Brackets => (
                    brackets_markdown(&success.bracket_fragments),
                    success.brackets_text.clone(),
                ),
                GentufaTextView::Tree => {
                    let tree = tree_text(&success.tree_rows);
                    (code_block(&tree), tree)
                }
            };
            rendered.body.push(body);
            rendered.set_full_text(full_text);
            rendered.set_diagnostics(render_diagnostics(source, &success.diagnostics));
            let mut notes = Vec::new();
            if recovered {
                notes.push(subtext(
                    "Recovered parse: underlined text was skipped by error recovery; the tree is not a complete parse.",
                ));
            }
            if options.include_diagram {
                match &outcome.diagram {
                    Some(image) => {
                        let mut legend = format!(
                            "Diagram {}×{} · compounds {} · glosses {}",
                            image.width,
                            image.height,
                            on_off(options.show_compounds),
                            on_off(options.show_glosses)
                        );
                        if options.show_elided {
                            legend.push_str(" · elided shown");
                        }
                        notes.push(subtext(&legend));
                    }
                    None => notes.push(subtext("No diagram was produced.")),
                }
            } else if options.show_glosses || !options.show_compounds {
                // The controls are image-only; say so when the image is off
                // so the setting is not mistaken for a text effect.
                notes.push(subtext(&format!(
                    "Compounds {} and glosses {} apply to the diagram, which is off.",
                    on_off(options.show_compounds),
                    on_off(options.show_glosses)
                )));
            }
            if !notes.is_empty() {
                rendered.notice = Some(join_lines(notes));
            }
            rendered.image = outcome.diagram.clone();
            rendered
        }
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn on_off(value: bool) -> &'static str {
    if value { "on" } else { "off" }
}

/// Brackets from the source-aware fragments: elided material struck
/// through, error regions underlined, everything else escaped as data.
#[requires(true)]
#[ensures(true)]
pub(crate) fn brackets_markdown(fragments: &[GentufaBracketFragment]) -> String {
    let mut output = String::new();
    append_fragments(fragments, &mut output);
    if output.trim().is_empty() {
        "*(empty text)*".to_owned()
    } else {
        output
    }
}

#[requires(true)]
#[ensures(true)]
fn append_fragments(fragments: &[GentufaBracketFragment], output: &mut String) {
    for fragment in fragments {
        match fragment {
            GentufaBracketFragment::Text { text, role } => match role {
                GentufaBlockRole::Normal => output.push_str(&escape(text)),
                GentufaBlockRole::Elided => {
                    let trimmed = text.trim();
                    if trimmed.is_empty() {
                        output.push_str(&escape(text));
                    } else {
                        let leading = &text[..text.len() - text.trim_start().len()];
                        let trailing = &text[text.trim_end().len()..];
                        output.push_str(leading);
                        output.push_str(&format!("~~{}~~", escape(trimmed)));
                        output.push_str(trailing);
                    }
                }
                GentufaBlockRole::Error => {
                    let trimmed = text.trim();
                    if trimmed.is_empty() {
                        output.push_str(&escape(text));
                    } else {
                        let leading = &text[..text.len() - text.trim_start().len()];
                        let trailing = &text[text.trim_end().len()..];
                        output.push_str(leading);
                        output.push_str(&format!("__{}__", escape(trimmed)));
                        output.push_str(trailing);
                    }
                }
            },
            GentufaBracketFragment::Span { children, .. } => append_fragments(children, output),
        }
    }
}

/// A plain-text tree from the shared tree rows: labels for constructs, word
/// text at the leaves, `⟨…⟩` around elided terminators and `✗` before error
/// regions.
#[requires(true)]
#[ensures(true)]
pub(crate) fn tree_text(rows: &[GentufaTreeRow]) -> String {
    if rows.is_empty() {
        return "(empty text)".to_owned();
    }
    let mut lines = Vec::with_capacity(rows.len() + 1);
    // Rows are in pre-order; a row is the last child of its parent when no
    // later row shares that parent.
    for (index, row) in rows.iter().enumerate() {
        let mut prefix = String::new();
        // Guides: for each ancestor depth, draw a vertical bar if that
        // ancestor still has children after this row.
        let mut ancestors = Vec::new();
        let mut current = row.parent_id;
        while let Some(parent_id) = current {
            ancestors.push(parent_id);
            current = rows
                .iter()
                .find(|candidate| candidate.node_id == parent_id)
                .and_then(|candidate| candidate.parent_id);
        }
        ancestors.reverse();
        for (depth, ancestor) in ancestors.iter().enumerate() {
            let is_last_at_depth = depth == ancestors.len() - 1;
            let ancestor_has_more = rows[index + 1..]
                .iter()
                .any(|later| later.parent_id == Some(*ancestor));
            if is_last_at_depth {
                prefix.push_str(if ancestor_has_more { "├ " } else { "└ " });
            } else {
                prefix.push_str(if ancestor_has_more { "│ " } else { "  " });
            }
        }
        let cell = row.cells.first();
        let text = match cell {
            Some(cell) if cell.is_word => match cell.role {
                GentufaBlockRole::Elided => format!("⟨{}⟩", cell.text),
                GentufaBlockRole::Error => format!("✗ {}", cell.text),
                GentufaBlockRole::Normal => cell.text.clone(),
            },
            _ => row.label.clone(),
        };
        let line = if row.has_children || cell.is_none_or(|cell| !cell.is_word) {
            format!("{prefix}{}", row.label)
        } else {
            format!("{prefix}{text}")
        };
        lines.push(line);
    }
    let has_elided = rows.iter().any(|row| {
        row.cells
            .iter()
            .any(|cell| cell.role == GentufaBlockRole::Elided)
    });
    let has_error = rows.iter().any(|row| {
        row.cells
            .iter()
            .any(|cell| cell.role == GentufaBlockRole::Error)
    });
    if has_elided || has_error {
        let mut legend = Vec::new();
        if has_elided {
            legend.push("⟨ ⟩ elided");
        }
        if has_error {
            legend.push("✗ skipped by recovery");
        }
        lines.push(legend.join(" · "));
    }
    lines.join("\n")
}

/// The brackets text of a success, for tests and the overflow attachment.
#[requires(true)]
#[ensures(true)]
pub(crate) fn brackets_plain(success: &GentufaSuccess) -> &str {
    &success.brackets_text
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discord::request::{GentufaOptions, SourceText};
    use jbotci_web_core::{GentufaWebOptions, GentufaWebRequest, parse_gentufa_for_web};

    #[requires(!text.is_empty())]
    #[ensures(true)]
    fn success(text: &str, show_elided: bool) -> GentufaSuccess {
        let GentufaWebResult::Success(success) = parse_gentufa_for_web(&GentufaWebRequest {
            text: text.to_owned(),
            options: GentufaWebOptions {
                show_elided,
                ..GentufaWebOptions::default()
            },
        }) else {
            panic!("{text} parses");
        };
        success
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn fragment_texts_concatenate_to_the_plain_brackets() {
        for (text, show_elided) in [
            ("mi klama lo zarci", false),
            ("mi klama lo zarci", true),
            ("mi klama .i do", true),
        ] {
            let success = success(text, show_elided);
            let mut plain = String::new();
            #[requires(true)]
            #[ensures(true)]
            fn collect(fragments: &[GentufaBracketFragment], out: &mut String) {
                for fragment in fragments {
                    match fragment {
                        GentufaBracketFragment::Text { text, .. } => out.push_str(text),
                        GentufaBracketFragment::Span { children, .. } => collect(children, out),
                    }
                }
            }
            collect(&success.bracket_fragments, &mut plain);
            assert_eq!(plain, success.brackets_text, "{text} elided={show_elided}");
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn brackets_mark_elided_terminators_and_escape_source() {
        let plain = success("mi klama lo zarci", false);
        let markdown = brackets_markdown(&plain.bracket_fragments);
        assert!(!markdown.contains("~~"), "{markdown}");
        assert!(markdown.contains("kláma"), "{markdown}");
        let elided = success("mi klama lo zarci", true);
        let markdown = brackets_markdown(&elided.bracket_fragments);
        assert!(
            markdown.contains("~~vau~~") || markdown.contains("~~ku~~"),
            "{markdown}"
        );
        // Markdown in the source is escaped, never interpreted.
        let starred = success("mi klama", false);
        let markdown = brackets_markdown(&starred.bracket_fragments);
        assert!(!markdown.contains("**"), "{markdown}");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn tree_text_follows_the_hierarchy_with_guides_and_marks_elided_leaves() {
        // The description branch is closed before its following sibling
        // terminator, which stays a marked leaf under the bridi tail.
        let tree = tree_text(&success("mi klama lo zarci", true).tree_rows);
        assert_eq!(
            tree,
            [
                "bridi",
                "├ mi",
                "└ bridi tail",
                "  ├ kláma",
                "  ├ description",
                "  │ ├ lo",
                "  │ ├ zárci",
                "  │ └ ⟨ku⟩",
                "  └ ⟨vau⟩",
                "⟨ ⟩ elided",
            ]
            .join("\n")
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn recovered_parse_is_labelled_and_keeps_diagnostics() {
        let request = GentufaRequest {
            text: SourceText::new("mi klama cu cu").expect("text"),
            dialect: None,
            options: GentufaOptions::default(),
        };
        let result = parse_gentufa_for_web(&GentufaWebRequest {
            text: request.text.as_str().to_owned(),
            options: GentufaWebOptions::default(),
        });
        let rendered = render(
            &GentufaOutcome {
                result,
                diagram: None,
            },
            &request,
        );
        assert!(
            rendered.status.text.contains("recovered parse"),
            "{}",
            rendered.status.text
        );
        assert!(rendered.diagnostics.is_some());
        assert!(
            rendered
                .notice
                .as_deref()
                .is_some_and(|notice| notice.contains("Recovered parse"))
        );
        assert!(rendered.image.is_none());
    }
}
