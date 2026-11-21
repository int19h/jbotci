//! Markdown rendering of the book model in two dialects.
//!
//! GitHub-flavoured Markdown is what the CLI and the MCP tool emit: pipe
//! tables, `$…$` math, sub- and superscript markers and the book's text
//! verbatim. A joined run of cmavo parts is not book prose. That run uses one
//! GitHub escaper so markup in the run stays text. Discord's dialect has none
//! of those, escapes ordinary text so the
//! client cannot misread it as formatting, and folds tabular content into code
//! blocks. Everything that is book semantics rather than syntax (which link
//! kinds survive a route-free rendering, how a rule-status note is labelled,
//! the example line kinds) is shared between the two.

use std::borrow::Cow;

use super::*;

/// Appends `text` as inert content in the dialect.
#[requires(true)]
#[ensures(true)]
fn push_text(output: &mut jbotci_budget::TextBuffer, text: &str, dialect: CllMarkdownDialect) {
    let scope = output.scope();
    match dialect {
        CllMarkdownDialect::GitHub => output.push_str(text),
        CllMarkdownDialect::Discord => {
            output.push_str(&escape_discord_markdown_in_scope(text, &scope))
        }
    }
}

/// `text` as inert content in the dialect.
#[requires(true)]
#[ensures(true)]
pub(crate) fn dialect_text_in_scope<'text>(
    text: &'text str,
    dialect: CllMarkdownDialect,
    scope: &jbotci_budget::TextScope,
) -> Cow<'text, str> {
    match dialect {
        CllMarkdownDialect::GitHub => Cow::Borrowed(text),
        CllMarkdownDialect::Discord => Cow::Owned(escape_discord_markdown_in_scope(text, &scope)),
    }
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn render_block_markdown_to(
    site: &CllSite,
    block: &CllBlock,
    output: &mut jbotci_budget::TextBuffer,
    depth: usize,
    link_mode: CllLinkRenderMode,
    dialect: CllMarkdownDialect,
) {
    let scope = output.scope();
    match block {
        CllBlock::Paragraph {
            role,
            inlines,
            text,
            ..
        } => {
            let body = if inlines.is_empty() {
                dialect_text_in_scope(text, dialect, &scope)
            } else {
                Cow::Owned(render_inlines_markdown(
                    site, inlines, link_mode, dialect, &scope,
                ))
            };
            if role.as_ref().is_some_and(CllParagraphRole::is_status_note) {
                output.push_str("> **");
                output.push_str(CLL_STATUS_NOTE_LABEL);
                output.push_str(".** ");
                for (index, line) in body.lines().enumerate() {
                    if index != 0 {
                        output.push_str("\n> ");
                    }
                    output.push_str(line);
                }
            } else {
                output.push_str(&body);
            }
            output.push_str("\n\n");
        }
        CllBlock::List { ordered, items } => {
            for (index, item) in items.iter().enumerate() {
                let marker = if *ordered {
                    scope.format(format_args!("{}.", index + 1))
                } else {
                    "-".to_owned()
                };
                output.push_str(&scope.repeat("  ", depth));
                output.push_str(&marker);
                output.push(' ');
                let mut item_text = scope.buffer();
                for block in item {
                    render_block_markdown_to(
                        site,
                        block,
                        &mut item_text,
                        depth + 1,
                        link_mode,
                        dialect,
                    );
                }
                output.push_str(item_text.trim());
                output.push('\n');
            }
            output.push('\n');
        }
        CllBlock::Example { example_id } => {
            if let Some(example) = cll_lookup_example(site, example_id) {
                render_example_markdown_to(site, example, output, link_mode, dialect);
            }
        }
        CllBlock::Table {
            caption,
            header_rows,
            body_rows,
            ..
        } => match dialect {
            CllMarkdownDialect::GitHub => render_table_markdown(
                site,
                caption.as_deref(),
                header_rows,
                body_rows,
                output,
                link_mode,
            ),
            CllMarkdownDialect::Discord => {
                if let Some(caption) = caption {
                    output.push_str("**");
                    output.push_str(&render_inlines_markdown(
                        site, caption, link_mode, dialect, &scope,
                    ));
                    output.push_str("**\n\n");
                }
                push_discord_rows(
                    header_rows.iter().chain(body_rows.iter()).map(|row| {
                        row.iter()
                            .map(|cell| {
                                single_line(
                                    &blocks_plain_text_in_scope(site, &cell.blocks, &scope),
                                    &scope,
                                )
                            })
                            .collect::<Vec<_>>()
                    }),
                    output,
                );
            }
        },
        CllBlock::SimpleListTable { rows, .. } => match dialect {
            CllMarkdownDialect::GitHub => {
                render_simple_list_table_markdown(site, rows, output, link_mode);
            }
            CllMarkdownDialect::Discord => push_discord_rows(
                rows.iter().map(|row| {
                    row.iter()
                        .map(|cell| {
                            cell.as_deref()
                                .map(|inlines| {
                                    single_line(
                                        &inline_plain_text_in_scope(inlines, &scope),
                                        &scope,
                                    )
                                })
                                .unwrap_or_default()
                        })
                        .collect::<Vec<_>>()
                }),
                output,
            ),
        },
        CllBlock::VariableList { entries, .. } => {
            for entry in entries {
                output.push_str("**");
                output.push_str(&render_inlines_markdown(
                    site,
                    &entry.term,
                    link_mode,
                    dialect,
                    &scope,
                ));
                output.push_str("**\n\n");
                for block in &entry.blocks {
                    render_block_markdown_to(site, block, output, depth, link_mode, dialect);
                }
            }
        }
        CllBlock::Media {
            title, src, alt, ..
        } => {
            // Discord does not render Markdown images, so its dialect always
            // carries the description.
            if link_mode == CllLinkRenderMode::Web && dialect == CllMarkdownDialect::GitHub {
                output.push_str(&scope.format(format_args!("![{}]({})\n\n", alt, src)));
            } else {
                push_text(output, alt, dialect);
                output.push_str("\n\n");
            }
            if let Some(title) = title {
                output.push_str(&render_inlines_markdown(
                    site, title, link_mode, dialect, &scope,
                ));
                output.push_str("\n\n");
            }
        }
        CllBlock::Rule { term, body, .. } => {
            output.push_str("**");
            push_text(output, term, dialect);
            output.push_str("**\n\n");
            for block in body {
                render_block_markdown_to(site, block, output, depth, link_mode, dialect);
            }
        }
        CllBlock::Code { text, .. } => {
            push_code_block(output, text, dialect);
        }
        CllBlock::DisplayMath { text, latex, .. } => match dialect {
            CllMarkdownDialect::GitHub => {
                output.push_str("$$\n");
                output.push_str(latex);
                output.push_str("\n$$\n\n");
            }
            // Discord renders no math; the readable text form is shown, or
            // the LaTeX source when the book carries no text form.
            CllMarkdownDialect::Discord => {
                push_code_block(output, math_text(text, latex), dialect);
            }
        },
        CllBlock::Heading { level, inlines, .. } => {
            let level = match dialect {
                CllMarkdownDialect::GitHub => usize::from(*level),
                // Discord supports three heading levels.
                CllMarkdownDialect::Discord => usize::from(*level).clamp(1, 3),
            };
            output.push_str(&scope.repeat("#", level));
            output.push(' ');
            output.push_str(&render_inlines_markdown(
                site, inlines, link_mode, dialect, &scope,
            ));
            output.push_str("\n\n");
        }
        CllBlock::BlockQuote { blocks, .. } => {
            let mut inner = scope.buffer();
            for block in blocks {
                render_block_markdown_to(site, block, &mut inner, depth, link_mode, dialect);
            }
            for line in inner.trim().lines() {
                output.push_str("> ");
                output.push_str(line);
                output.push('\n');
            }
            output.push('\n');
        }
        CllBlock::Definition { body, .. } | CllBlock::GrammarTemplate { body, .. } => {
            output.push_str(&render_inlines_markdown(
                site, body, link_mode, dialect, &scope,
            ));
            output.push_str("\n\n");
        }
        CllBlock::InterlinearGloss {
            aligned,
            parse_href,
            rows,
            natlang,
            comments,
            ..
        } => match dialect {
            CllMarkdownDialect::GitHub => render_interlinear_markdown(
                site,
                *aligned,
                parse_href.as_deref(),
                rows,
                natlang,
                comments,
                output,
                link_mode,
            ),
            CllMarkdownDialect::Discord => {
                render_interlinear_discord(site, rows, natlang, comments, output, link_mode);
            }
        },
        CllBlock::CmavoList {
            titles,
            headers,
            rows,
            ..
        } => match dialect {
            CllMarkdownDialect::GitHub => {
                render_cmavo_list_markdown(site, titles, headers, rows, output, link_mode);
            }
            CllMarkdownDialect::Discord => {
                render_cmavo_list_discord(site, titles, headers, rows, output, link_mode);
            }
        },
        CllBlock::Lojbanization { lines, .. } => match dialect {
            CllMarkdownDialect::GitHub => {
                render_lojbanization_markdown(site, lines, output, link_mode);
            }
            CllMarkdownDialect::Discord => {
                for line in lines {
                    output.push_str("- **");
                    output.push_str(line.kind.as_str());
                    output.push_str("**: ");
                    output.push_str(&render_inlines_markdown(
                        site, &line.body, link_mode, dialect, &scope,
                    ));
                    if let Some(comment) = &line.comment {
                        output.push_str(" — ");
                        output.push_str(&render_inlines_markdown(
                            site, comment, link_mode, dialect, &scope,
                        ));
                    }
                    output.push('\n');
                }
                output.push('\n');
            }
        },
        CllBlock::LujvoMaking { parts, .. } => {
            for part in parts {
                output.push_str("- **");
                output.push_str(part.kind.as_str());
                output.push_str("**: ");
                output.push_str(&render_inlines_markdown(
                    site, &part.body, link_mode, dialect, &scope,
                ));
                output.push('\n');
            }
            output.push('\n');
        }
        CllBlock::Ebnf { entries, .. } => match dialect {
            CllMarkdownDialect::GitHub => render_ebnf_markdown(site, entries, output, link_mode),
            CllMarkdownDialect::Discord => render_ebnf_discord(entries, output),
        },
    }
}

/// An example: its label, then its content blocks, or its legacy line list
/// when the book carries no blocks for it.
#[requires(true)]
#[ensures(output.scope().is_exceeded() || output.contains(example.label.as_str()) || dialect == CllMarkdownDialect::Discord)]
pub(crate) fn render_example_markdown_to(
    site: &CllSite,
    example: &CllExample,
    output: &mut jbotci_budget::TextBuffer,
    link_mode: CllLinkRenderMode,
    dialect: CllMarkdownDialect,
) {
    let scope = output.scope();
    match dialect {
        CllMarkdownDialect::GitHub => {
            output.push_str(&scope.format(format_args!("### {}", example.label)));
            if link_mode == CllLinkRenderMode::Web
                && let Some(parse_href) = &example.parse_href
            {
                output.push_str(&scope.format(format_args!(" [Parse]({parse_href})")));
            }
            output.push_str("\n\n");
        }
        // Discord headings are large; a bold line keeps a section with many
        // examples readable, and the example title becomes subtext.
        CllMarkdownDialect::Discord => {
            output.push_str("**");
            push_text(output, &example.label, dialect);
            output.push_str("**\n");
            if let Some(title) = &example.title {
                output.push_str("-# ");
                push_text(output, title, dialect);
                output.push('\n');
            }
        }
    }
    for block in &example.blocks {
        render_block_markdown_to(site, block, output, 0, link_mode, dialect);
    }
    if example.blocks.is_empty() {
        for line in &example.lines {
            match dialect {
                CllMarkdownDialect::GitHub => {
                    if line.kind == CllExampleLineKind::Text {
                        output.push_str(&line.text);
                    } else {
                        output.push_str(&scope.format(format_args!(
                            "{}: {}",
                            line.kind.as_str(),
                            line.text
                        )));
                    }
                }
                CllMarkdownDialect::Discord => {
                    output.push_str("- ");
                    if line.kind != CllExampleLineKind::Text {
                        output.push_str("**");
                        output.push_str(line.kind.as_str());
                        output.push_str("**: ");
                    }
                    push_text(output, &line.text, dialect);
                }
            }
            output.push('\n');
        }
        output.push('\n');
    }
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn render_inlines_markdown(
    site: &CllSite,
    inlines: &[CllInline],
    link_mode: CllLinkRenderMode,
    dialect: CllMarkdownDialect,
    scope: &jbotci_budget::TextScope,
) -> String {
    let mut output = scope.buffer();
    for inline in inlines {
        match inline {
            CllInline::Text(text) => push_text(&mut output, text, dialect),
            CllInline::Emphasis { inlines, .. } => {
                output.push('*');
                output.push_str(&render_inlines_markdown(
                    site, inlines, link_mode, dialect, &scope,
                ));
                output.push('*');
            }
            CllInline::Quote { inlines, .. } => {
                output.push('"');
                output.push_str(&render_inlines_markdown(
                    site, inlines, link_mode, dialect, &scope,
                ));
                output.push('"');
            }
            CllInline::LanguageSpan { inlines, .. } | CllInline::CiteTitle { inlines } => {
                match dialect {
                    CllMarkdownDialect::GitHub => {
                        output.push_str(&render_inlines_markdown(
                            site, inlines, link_mode, dialect, &scope,
                        ));
                    }
                    // The web reader sets Lojban and foreign phrases and cited
                    // titles in italics; Discord can do the same.
                    CllMarkdownDialect::Discord => {
                        output.push('*');
                        output.push_str(&render_inlines_markdown(
                            site, inlines, link_mode, dialect, &scope,
                        ));
                        output.push('*');
                    }
                }
            }
            CllInline::Subscript { inlines } => match dialect {
                CllMarkdownDialect::GitHub => {
                    output.push('~');
                    output.push_str(&render_inlines_markdown(
                        site, inlines, link_mode, dialect, &scope,
                    ));
                    output.push('~');
                }
                CllMarkdownDialect::Discord => {
                    output.push_str(&render_inlines_markdown(
                        site, inlines, link_mode, dialect, &scope,
                    ));
                }
            },
            CllInline::Superscript { inlines } => match dialect {
                CllMarkdownDialect::GitHub => {
                    output.push('^');
                    output.push_str(&render_inlines_markdown(
                        site, inlines, link_mode, dialect, &scope,
                    ));
                    output.push('^');
                }
                CllMarkdownDialect::Discord => {
                    output.push_str(&render_inlines_markdown(
                        site, inlines, link_mode, dialect, &scope,
                    ));
                }
            },
            CllInline::Link {
                target,
                inlines,
                kind,
            } => {
                let text = render_inlines_markdown(site, inlines, link_mode, dialect, &scope);
                let text = if text.is_empty() {
                    dialect_text_in_scope(target, dialect, &scope)
                } else {
                    Cow::Owned(text)
                };
                match link_mode {
                    CllLinkRenderMode::Web => output.push_str(&scope.format(format_args!(
                        "[{}]({})",
                        markdown_link_label_text(&text, &scope),
                        cll_link_href(site, *kind, target)
                    ))),
                    CllLinkRenderMode::Plain => match kind.plain_disposition() {
                        CllPlainLinkDisposition::KeepContent => output.push_str(&text),
                        CllPlainLinkDisposition::Drop => {}
                    },
                }
            }
            CllInline::Code(text) => match dialect {
                CllMarkdownDialect::GitHub => {
                    output.push_str(&scope.format(format_args!("`{text}`")))
                }
                CllMarkdownDialect::Discord => {
                    output.push_str(&discord_inline_code_in_scope(text, &scope))
                }
            },
            CllInline::Elidable { shown, inlines, .. } => {
                let text = render_inlines_markdown(site, inlines, link_mode, dialect, &scope);
                output.push('[');
                if text.is_empty() {
                    push_text(&mut output, shown, dialect);
                } else {
                    output.push_str(&text);
                }
                output.push(']');
            }
            CllInline::InlineMath { text, latex, .. } => match dialect {
                CllMarkdownDialect::GitHub => {
                    output.push('$');
                    output.push_str(latex);
                    output.push('$');
                }
                CllMarkdownDialect::Discord => {
                    output.push_str(&discord_inline_code_in_scope(
                        math_text(text, latex),
                        &scope,
                    ));
                }
            },
            CllInline::Anchor { .. } => {}
        }
    }
    output.into_piece()
}

/// The readable form of a formula, or its LaTeX when the book has none.
#[requires(true)]
#[ensures(true)]
fn math_text<'a>(text: &'a str, latex: &'a str) -> &'a str {
    if text.trim().is_empty() { latex } else { text }
}

/// A cell's text on one line.
#[requires(true)]
#[ensures(!ret.contains('\n'))]
fn single_line(text: &str, scope: &jbotci_budget::TextScope) -> String {
    scope.join(text.split_whitespace(), " ")
}

#[requires(true)]
#[ensures(true)]
fn push_code_block(
    output: &mut jbotci_budget::TextBuffer,
    text: &str,
    dialect: CllMarkdownDialect,
) {
    let scope = output.scope();
    match dialect {
        CllMarkdownDialect::GitHub => {
            output.push_str("```\n");
            output.push_str(text);
            output.push_str("\n```\n\n");
        }
        CllMarkdownDialect::Discord => {
            output.push_str(&discord_code_block_in_scope(text, &scope));
            output.push_str("\n\n");
        }
    }
}

/// Discord has no tables: rows become ` | `-separated lines in a code block,
/// which keeps them legible without relying on column alignment.
#[requires(true)]
#[ensures(true)]
fn push_discord_rows<I>(rows: I, output: &mut jbotci_budget::TextBuffer)
where
    I: IntoIterator<Item = Vec<String>>,
{
    let scope = output.scope();
    let lines = scope.join(
        rows.into_iter()
            .map(|row| scope.join(row, " | "))
            .filter(|line| !line.trim().is_empty()),
        "\n",
    );
    if lines.is_empty() {
        return;
    }
    output.push_str(&discord_code_block_in_scope(&lines, &scope));
    output.push_str("\n\n");
}

#[requires(true)]
#[ensures(true)]
fn render_table_markdown(
    site: &CllSite,
    caption: Option<&[CllInline]>,
    header_rows: &[Vec<CllTableCell>],
    body_rows: &[Vec<CllTableCell>],
    output: &mut jbotci_budget::TextBuffer,
    link_mode: CllLinkRenderMode,
) {
    let scope = output.scope();
    if let Some(caption) = caption {
        output.push_str("**");
        output.push_str(&render_inlines_markdown(
            site,
            caption,
            link_mode,
            CllMarkdownDialect::GitHub,
            &scope,
        ));
        output.push_str("**\n\n");
    }
    let width = header_rows
        .iter()
        .chain(body_rows.iter())
        .map(Vec::len)
        .max()
        .unwrap_or(0);
    render_markdown_table_rows(
        header_rows.iter().chain(body_rows.iter()).map(|row| {
            row.iter()
                .map(|cell| table_cell_markdown_text(site, cell, link_mode, &scope))
                .collect::<Vec<_>>()
        }),
        output,
        width,
    );
}

#[requires(true)]
#[ensures(true)]
fn render_simple_list_table_markdown(
    site: &CllSite,
    rows: &[Vec<Option<Vec<CllInline>>>],
    output: &mut jbotci_budget::TextBuffer,
    link_mode: CllLinkRenderMode,
) {
    let scope = output.scope();
    let width = rows.iter().map(Vec::len).max().unwrap_or(0);
    render_markdown_table_rows(
        rows.iter().map(|row| {
            row.iter()
                .map(|cell| {
                    cell.as_deref()
                        .map(|inlines| {
                            markdown_table_cell_text(
                                &render_inlines_markdown(
                                    site,
                                    inlines,
                                    link_mode,
                                    CllMarkdownDialect::GitHub,
                                    &scope,
                                ),
                                &scope,
                            )
                        })
                        .unwrap_or_default()
                })
                .collect::<Vec<_>>()
        }),
        output,
        width,
    );
}

#[requires(true)]
#[ensures(true)]
fn render_markdown_table_rows<I>(rows: I, output: &mut jbotci_budget::TextBuffer, width: usize)
where
    I: IntoIterator<Item = Vec<String>>,
{
    let mut rows = rows.into_iter().peekable();
    if width == 0 || rows.peek().is_none() {
        return;
    }
    for (row_index, row) in rows.enumerate() {
        output.push('|');
        for cell_index in 0..width {
            output.push(' ');
            output.push_str(row.get(cell_index).map(String::as_str).unwrap_or_default());
            output.push_str(" |");
        }
        output.push('\n');
        if row_index == 0 {
            output.push('|');
            for _ in 0..width {
                output.push_str(" --- |");
            }
            output.push('\n');
        }
    }
    output.push('\n');
}

#[requires(true)]
#[ensures(true)]
fn markdown_table_cell_text(text: &str, scope: &jbotci_budget::TextScope) -> String {
    let mut output = scope.buffer();
    for character in text.chars() {
        match character {
            '|' => output.push_str("\\|"),
            '\n' => output.push_str("<br>"),
            character => output.push(character),
        }
    }
    output.into_piece()
}

#[requires(true)]
#[ensures(true)]
fn table_cell_markdown_text(
    site: &CllSite,
    cell: &CllTableCell,
    link_mode: CllLinkRenderMode,
    scope: &jbotci_budget::TextScope,
) -> String {
    let mut text = scope.buffer_with(&markdown_table_cell_text(
        &blocks_plain_text_in_scope(site, &cell.blocks, &scope),
        &scope,
    ));
    if link_mode == CllLinkRenderMode::Web
        && let Some(parse_href) = &cell.parse_href
    {
        if !text.is_empty() {
            text.push(' ');
        }
        text.push_str(&scope.format(format_args!("[Parse]({parse_href})")));
    }
    text.into_piece()
}

#[requires(true)]
#[ensures(true)]
fn markdown_link_label_text(text: &str, scope: &jbotci_budget::TextScope) -> String {
    let mut output = scope.buffer();
    for character in text.chars() {
        match character {
            '[' => output.push_str("\\["),
            ']' => output.push_str("\\]"),
            character => output.push(character),
        }
    }
    output.into_piece()
}

#[requires(true)]
#[ensures(true)]
fn render_interlinear_markdown(
    site: &CllSite,
    aligned: bool,
    parse_href: Option<&str>,
    rows: &[CllInterlinearRow],
    natlang: &[Vec<CllInline>],
    comments: &[Vec<CllInline>],
    output: &mut jbotci_budget::TextBuffer,
    link_mode: CllLinkRenderMode,
) {
    let scope = output.scope();
    let dialect = CllMarkdownDialect::GitHub;
    if link_mode == CllLinkRenderMode::Web
        && let Some(parse_href) = parse_href
    {
        output.push_str("[Parse](");
        output.push_str(parse_href);
        output.push_str(")\n\n");
    }
    if !aligned {
        for row in rows {
            let body = scope.join(
                row.cells
                    .iter()
                    .map(|cell| render_inlines_markdown(site, cell, link_mode, dialect, &scope))
                    .filter(|cell| !cell.is_empty()),
                " ",
            );
            if !body.is_empty() {
                output.push_str(row.kind.as_str());
                output.push_str(": ");
                output.push_str(&body);
                output.push('\n');
            }
        }
        for line in comments {
            output.push_str("comment: ");
            output.push_str(&render_inlines_markdown(
                site, line, link_mode, dialect, &scope,
            ));
            output.push('\n');
        }
        for line in natlang {
            output.push_str("natlang: ");
            output.push_str(&render_inlines_markdown(
                site, line, link_mode, dialect, &scope,
            ));
            output.push('\n');
        }
        output.push('\n');
        return;
    }

    let width = rows.iter().map(|row| row.cells.len()).max().unwrap_or(0);
    let table_rows = rows.iter().map(|row| {
        row.cells
            .iter()
            .map(|cell| {
                markdown_table_cell_text(
                    &render_inlines_markdown(site, cell, link_mode, dialect, &scope),
                    &scope,
                )
            })
            .collect::<Vec<_>>()
    });
    render_markdown_table_rows(table_rows, output, width);
    for line in comments {
        output.push_str("_");
        output.push_str(&render_inlines_markdown(
            site, line, link_mode, dialect, &scope,
        ));
        output.push_str("_\n\n");
    }
    for line in natlang {
        output.push_str("> ");
        output.push_str(&render_inlines_markdown(
            site, line, link_mode, dialect, &scope,
        ));
        output.push_str("\n\n");
    }
}

/// Discord keeps every gloss as one labelled list line, aligned or not: the
/// client wraps table-like content on narrow screens, so word-by-word column
/// alignment cannot be relied on there.
#[requires(true)]
#[ensures(true)]
fn render_interlinear_discord(
    site: &CllSite,
    rows: &[CllInterlinearRow],
    natlang: &[Vec<CllInline>],
    comments: &[Vec<CllInline>],
    output: &mut jbotci_budget::TextBuffer,
    link_mode: CllLinkRenderMode,
) {
    let scope = output.scope();
    let dialect = CllMarkdownDialect::Discord;
    for row in rows {
        let body = scope.join(
            row.cells
                .iter()
                .map(|cell| render_inlines_markdown(site, cell, link_mode, dialect, &scope))
                .filter(|cell| !cell.is_empty()),
            " ",
        );
        if !body.is_empty() {
            output.push_str("- **");
            output.push_str(row.kind.as_str());
            output.push_str("**: ");
            output.push_str(&body);
            output.push('\n');
        }
    }
    for line in comments {
        output.push_str("- **comment**: ");
        output.push_str(&render_inlines_markdown(
            site, line, link_mode, dialect, &scope,
        ));
        output.push('\n');
    }
    for line in natlang {
        output.push_str("- **natlang**: ");
        output.push_str(&render_inlines_markdown(
            site, line, link_mode, dialect, &scope,
        ));
        output.push('\n');
    }
    output.push('\n');
}

#[requires(true)]
#[ensures(true)]
fn render_cmavo_list_markdown(
    site: &CllSite,
    titles: &[Vec<CllInline>],
    headers: &[Vec<CllInline>],
    rows: &[Vec<CllCmavoListCell>],
    output: &mut jbotci_budget::TextBuffer,
    link_mode: CllLinkRenderMode,
) {
    let scope = output.scope();
    let dialect = CllMarkdownDialect::GitHub;
    for title in titles {
        output.push_str("**");
        output.push_str(&render_inlines_markdown(
            site, title, link_mode, dialect, &scope,
        ));
        output.push_str("**\n\n");
    }
    if headers.is_empty() {
        for row in rows {
            let mut rendered_cells = Vec::new();
            let mut at_line_start = true;
            for cell in row {
                let rendered = if let Some(parts) = cell.word_parts() {
                    render_github_cmavo_word_parts(site, &parts, link_mode, &scope, at_line_start)
                } else {
                    render_inlines_markdown(site, cell.inlines(), link_mode, dialect, &scope)
                        .trim()
                        .to_owned()
                };
                if rendered.is_empty() {
                    continue;
                }
                rendered_cells.push(rendered);
                at_line_start = false;
            }
            if rendered_cells.is_empty() {
                continue;
            }
            output.push_str(&scope.join(&rendered_cells, " | "));
            output.push_str("\n\n");
        }
        return;
    }
    let header = headers
        .iter()
        .map(|cell| {
            markdown_table_cell_text(
                &render_inlines_markdown(site, cell, link_mode, dialect, &scope),
                &scope,
            )
        })
        .collect::<Vec<_>>();
    let rendered_rows = rows.iter().map(|row| {
        row.iter()
            .map(|cell| {
                if let Some(parts) = cell.word_parts() {
                    // The cell sits after `| `, so it does not start a line.
                    // The escaper already treats `|` as text. The table helper
                    // would escape that pipe a second time.
                    render_github_cmavo_word_parts(site, &parts, link_mode, &scope, false)
                } else {
                    markdown_table_cell_text(
                        &render_inlines_markdown(site, cell.inlines(), link_mode, dialect, &scope),
                        &scope,
                    )
                }
            })
            .collect::<Vec<_>>()
    });
    render_markdown_table_rows(
        std::iter::once(header).chain(rendered_rows),
        output,
        headers
            .len()
            .max(rows.iter().map(Vec::len).max().unwrap_or(0)),
    );
}

/// True when `character` starts GitHub inline markup at any position.
///
/// A pause dot is not in this set. `.ui` stays `.ui`. A dot is escaped only
/// when it completes `www.` or an ordered-list marker at the start of a line.
#[requires(true)]
#[ensures(true)]
fn github_inline_escape(character: char) -> bool {
    matches!(
        character,
        '\\' | '`'
            | '*'
            | '_'
            | '['
            | ']'
            | '('
            | ')'
            | '<'
            | '>'
            | '~'
            | '$'
            | '^'
            | '&'
            | '|'
            | '!'
            | '@'
    )
}

/// Bytes of a heading, bullet, or ordered-list marker at the start of `tail`.
///
/// The caller has already skipped up to three leading spaces. A heading is one
/// to six `#` characters and then a space or the end of the line. A bullet is
/// `-` or `+` and then a space or the end of the line. An ordered list is one
/// to nine digits, a `.`, and then a space or the end of the line.
#[requires(true)]
#[ensures(ret.is_none_or(|skip| skip > 0 && skip <= tail.len()))]
fn github_line_marker_bytes(tail: &str) -> Option<usize> {
    let bytes = tail.as_bytes();
    if bytes.is_empty() {
        return None;
    }
    let hashes = bytes.iter().take_while(|byte| **byte == b'#').count();
    if (1..=6).contains(&hashes) {
        let after = bytes.get(hashes).copied();
        if after.is_none_or(|byte| byte == b' ' || byte == b'\t') {
            return Some(1);
        }
    }
    if bytes[0] == b'-' || bytes[0] == b'+' {
        let after = bytes.get(1).copied();
        if after.is_none_or(|byte| byte == b' ' || byte == b'\t') {
            return Some(1);
        }
    }
    let digits = bytes
        .iter()
        .take_while(|byte| byte.is_ascii_digit())
        .count();
    if (1..=9).contains(&digits) && bytes.get(digits) == Some(&b'.') {
        let after = bytes.get(digits + 1).copied();
        if after.is_none_or(|byte| byte == b' ' || byte == b'\t') {
            return Some(digits + 1);
        }
    }
    None
}

/// Writes `marker` so GitHub does not treat it as a heading, a bullet, or a list.
#[requires(!marker.is_empty())]
#[ensures(true)]
fn push_github_marker(output: &mut jbotci_budget::TextBuffer, marker: &str) {
    if let Some(digits) = marker.strip_suffix('.') {
        output.push_str(digits);
        output.push('\\');
        output.push('.');
        return;
    }
    output.push('\\');
    output.push(marker.chars().next().expect("marker is not empty"));
}

/// Writes inline GitHub text. `www.`, `mailto:` and `://` stay literal.
#[requires(true)]
#[ensures(true)]
fn push_github_inline(output: &mut jbotci_budget::TextBuffer, text: &str) {
    let mut rest = text;
    while let Some(character) = rest.chars().next() {
        if let Some(stripped) = rest.strip_prefix("www.") {
            output.push_str("www\\.");
            rest = stripped;
            continue;
        }
        if let Some(stripped) = rest.strip_prefix("mailto:") {
            output.push_str("mailto\\:");
            rest = stripped;
            continue;
        }
        if let Some(stripped) = rest.strip_prefix("://") {
            output.push_str("\\://");
            rest = stripped;
            continue;
        }
        if github_inline_escape(character) {
            output.push('\\');
        }
        output.push(character);
        rest = &rest[character.len_utf8()..];
    }
}

/// Writes one GitHub line. Line-start markers are escaped only when `at_line_start` is set.
#[requires(!line.contains('\n'))]
#[ensures(true)]
fn push_github_line(output: &mut jbotci_budget::TextBuffer, line: &str, at_line_start: bool) {
    let mut rest = line;
    if at_line_start {
        let bytes = line.as_bytes();
        let mut indent = 0;
        while indent < bytes.len() && indent < 3 && bytes[indent] == b' ' {
            indent += 1;
        }
        let indented_code = bytes.get(indent) == Some(&b' ');
        if !indented_code {
            let tail = &line[indent..];
            if let Some(skip) = github_line_marker_bytes(tail) {
                for _ in 0..indent {
                    output.push(' ');
                }
                push_github_marker(output, &tail[..skip]);
                rest = &tail[skip..];
            }
        }
    }
    push_github_inline(output, rest);
}

/// `text` as literal GitHub Markdown.
///
/// This is the one GitHub escaper. Book prose does not use it, because the
/// reviewed section text is copied as it is. A cmavo-part run does use it.
/// Pass `at_line_start` only when `text` begins a line. A dictionary token
/// such as `.ui` is unchanged, unless the token contains `www.`.
#[requires(true)]
#[ensures(scope.is_exceeded() || (text.is_empty() == ret.is_empty()))]
#[ensures(scope.is_exceeded() || !ret.contains("]("))]
#[ensures(scope.is_exceeded() || !ret.contains("https://"))]
#[ensures(scope.is_exceeded() || !ret.contains("http://"))]
#[ensures(
    scope.is_exceeded()
        || !crate::cmavo_table::is_dictionary_token(text)
        || text.contains("www.")
        || ret == text
)]
fn escape_github_markdown_in_scope(
    text: &str,
    scope: &jbotci_budget::TextScope,
    at_line_start: bool,
) -> String {
    let mut escaped = scope.buffer();
    for (index, line) in text.split('\n').enumerate() {
        if index > 0 {
            escaped.push('\n');
        }
        push_github_line(&mut escaped, line, at_line_start || index > 0);
    }
    escaped.into_piece()
}

/// Joins part text with no added space.
#[requires(!parts.is_empty())]
#[ensures(scope.is_exceeded() || !ret.is_empty())]
fn join_part_text(parts: &[(&str, Option<&str>)], scope: &jbotci_budget::TextScope) -> String {
    let mut word = scope.buffer();
    for (text, _) in parts {
        word.push_str(text);
    }
    word.into_piece()
}

/// GitHub text for adjacent valsi parts, as one word.
///
/// Plain mode joins every part, then escapes that word once. Web mode writes
/// each linked part as a dictionary link. It joins each run of unlinked parts
/// and escapes that run once. A line-start escape is used only when the run
/// starts the line. No space is added between parts. A dictionary token such
/// as `.ui` is written as it is.
#[requires(parts.len() >= 2)]
#[requires(parts.iter().all(|(text, target)| {
    !text.is_empty() && target.is_none_or(|target| !target.is_empty())
}))]
#[ensures(scope.is_exceeded() || !ret.is_empty())]
#[ensures(scope.is_exceeded() || !ret.contains("https://"))]
#[ensures(scope.is_exceeded() || !ret.contains("http://"))]
#[ensures(scope.is_exceeded() || link_mode != CllLinkRenderMode::Plain || !ret.contains("]("))]
fn render_github_cmavo_word_parts(
    site: &CllSite,
    parts: &[(&str, Option<&str>)],
    link_mode: CllLinkRenderMode,
    scope: &jbotci_budget::TextScope,
    at_line_start: bool,
) -> String {
    match link_mode {
        CllLinkRenderMode::Plain => {
            let word = join_part_text(parts, scope);
            escape_github_markdown_in_scope(&word, scope, at_line_start)
        }
        CllLinkRenderMode::Web => {
            let mut output = scope.buffer();
            let mut index = 0;
            let mut at_start = at_line_start;
            while index < parts.len() {
                if let Some(target) = parts[index].1 {
                    output.push_str(&scope.format(format_args!(
                        "[{}]({})",
                        markdown_link_label_text(parts[index].0, scope),
                        cll_link_href(site, CllLinkKind::Dictionary, target)
                    )));
                    index += 1;
                    at_start = false;
                    continue;
                }
                let start = index;
                while index < parts.len() && parts[index].1.is_none() {
                    index += 1;
                }
                let run = join_part_text(&parts[start..index], scope);
                output.push_str(&escape_github_markdown_in_scope(&run, scope, at_start));
                at_start = false;
            }
            output.into_piece()
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_cmavo_list_discord(
    site: &CllSite,
    titles: &[Vec<CllInline>],
    headers: &[Vec<CllInline>],
    rows: &[Vec<CllCmavoListCell>],
    output: &mut jbotci_budget::TextBuffer,
    link_mode: CllLinkRenderMode,
) {
    let scope = output.scope();
    let dialect = CllMarkdownDialect::Discord;
    for title in titles {
        output.push_str("**");
        output.push_str(&render_inlines_markdown(
            site, title, link_mode, dialect, &scope,
        ));
        output.push_str("**\n\n");
    }
    if headers.is_empty() {
        for row in rows {
            let mut rendered_cells = Vec::new();
            let mut at_line_start = true;
            for cell in row {
                let rendered = if let Some(parts) = cell.word_parts() {
                    render_discord_cmavo_word_parts(site, &parts, link_mode, &scope, at_line_start)
                } else {
                    render_inlines_markdown(site, cell.inlines(), link_mode, dialect, &scope)
                        .trim()
                        .to_owned()
                };
                if rendered.is_empty() {
                    continue;
                }
                rendered_cells.push(rendered);
                at_line_start = false;
            }
            if rendered_cells.is_empty() {
                continue;
            }
            output.push_str(&scope.join(&rendered_cells, " · "));
            output.push('\n');
        }
        output.push('\n');
        return;
    }
    let header = headers
        .iter()
        .map(|cell| single_line(&inline_plain_text_in_scope(cell, &scope), &scope))
        .collect::<Vec<_>>();
    let body = rows.iter().map(|row| {
        row.iter()
            .map(|cell| single_line(&cmavo_list_cell_plain_text_in_scope(cell, &scope), &scope))
            .collect::<Vec<_>>()
    });
    push_discord_rows(std::iter::once(header).chain(body), output);
}

/// Discord text for adjacent valsi parts, as one word.
///
/// Plain mode joins the parts, then escapes that word once. It wraps the word
/// in one italic span when every part is unlinked. Web mode prints each linked
/// part as a link. It joins each run of unlinked parts, escapes that run once,
/// and wraps the run in one italic span. A line-start escape is used only when
/// the run starts the line. No space is added between parts. A dictionary
/// token such as `.ui` is written as it is.
#[requires(parts.len() >= 2)]
#[requires(parts.iter().all(|(text, target)| {
    !text.is_empty() && target.is_none_or(|target| !target.is_empty())
}))]
#[ensures(scope.is_exceeded() || !ret.is_empty())]
fn render_discord_cmavo_word_parts(
    site: &CllSite,
    parts: &[(&str, Option<&str>)],
    link_mode: CllLinkRenderMode,
    scope: &jbotci_budget::TextScope,
    at_line_start: bool,
) -> String {
    match link_mode {
        CllLinkRenderMode::Plain => {
            let word = join_part_text(parts, scope);
            let word = escape_discord_markdown_at(&word, scope, at_line_start);
            if parts.iter().all(|(_, target)| target.is_none()) {
                scope.format(format_args!("*{word}*"))
            } else {
                word
            }
        }
        CllLinkRenderMode::Web => {
            let mut output = scope.buffer();
            let mut index = 0;
            let mut at_start = at_line_start;
            while index < parts.len() {
                if let Some(target) = parts[index].1 {
                    let label = escape_discord_markdown_at(parts[index].0, scope, false);
                    output.push_str(&scope.format(format_args!(
                        "[{}]({})",
                        markdown_link_label_text(&label, scope),
                        cll_link_href(site, CllLinkKind::Dictionary, target)
                    )));
                    index += 1;
                    at_start = false;
                    continue;
                }
                let start = index;
                while index < parts.len() && parts[index].1.is_none() {
                    index += 1;
                }
                let run = join_part_text(&parts[start..index], scope);
                let run = escape_discord_markdown_at(&run, scope, at_start);
                output.push('*');
                output.push_str(&run);
                output.push('*');
                at_start = false;
            }
            output.into_piece()
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_lojbanization_markdown(
    site: &CllSite,
    lines: &[CllLojbanizationLine],
    output: &mut jbotci_budget::TextBuffer,
    link_mode: CllLinkRenderMode,
) {
    let scope = output.scope();
    let dialect = CllMarkdownDialect::GitHub;
    let rows = lines.iter().map(|line| {
        vec![
            line.kind.as_str().to_owned(),
            markdown_table_cell_text(
                &render_inlines_markdown(site, &line.body, link_mode, dialect, &scope),
                &scope,
            ),
            line.comment
                .as_deref()
                .map(|comment| {
                    markdown_table_cell_text(
                        &render_inlines_markdown(site, comment, link_mode, dialect, &scope),
                        &scope,
                    )
                })
                .unwrap_or_default(),
        ]
    });
    render_markdown_table_rows(rows, output, 3);
}

#[requires(true)]
#[ensures(true)]
fn render_ebnf_markdown(
    site: &CllSite,
    entries: &[CllEbnfEntry],
    output: &mut jbotci_budget::TextBuffer,
    link_mode: CllLinkRenderMode,
) {
    let scope = output.scope();
    for entry in entries {
        for source_anchor_id in &entry.source_anchor_ids {
            output.push_str("<a id=\"");
            output.push_str(&escape_html_in_scope(source_anchor_id, &scope));
            output.push_str("\"></a>");
        }
        output.push_str("**");
        output.push_str(&entry.rule_name);
        output.push_str("** ⩴\n");
        for line in wrap_ebnf_choice_lines(&entry.rhs) {
            output.push_str("  ");
            output.push_str(&render_ebnf_tokens_markdown(site, &line, link_mode, &scope));
            output.push('\n');
        }
        output.push_str("\n\n");
    }
}

/// Discord carries no anchors or in-text links, so grammar rules go into one
/// code block, which also preserves their indentation.
#[requires(true)]
#[ensures(true)]
fn render_ebnf_discord(entries: &[CllEbnfEntry], output: &mut jbotci_budget::TextBuffer) {
    let scope = output.scope();
    let mut text = scope.buffer();
    for (index, entry) in entries.iter().enumerate() {
        if index > 0 {
            text.push('\n');
        }
        text.push_str(&entry.rule_name);
        text.push_str(" ⩴\n");
        for line in wrap_ebnf_choice_lines(&entry.rhs) {
            text.push_str("  ");
            for token in &line {
                text.push_str(ebnf_token_body(token));
            }
            text.push('\n');
        }
    }
    if text.is_empty() {
        return;
    }
    output.push_str(&discord_code_block_in_scope(text.trim_end(), &scope));
    output.push_str("\n\n");
}

#[requires(true)]
#[ensures(true)]
fn ebnf_token_body(token: &CllEbnfToken) -> &str {
    match token {
        CllEbnfToken::Text { body }
        | CllEbnfToken::Operator { body }
        | CllEbnfToken::Hash { body }
        | CllEbnfToken::Terminal { body, .. }
        | CllEbnfToken::ElidableTerminator { body, .. }
        | CllEbnfToken::Nonterminal { body, .. } => body,
    }
}

#[requires(true)]
#[ensures(true)]
fn render_ebnf_tokens_markdown(
    site: &CllSite,
    tokens: &[CllEbnfToken],
    link_mode: CllLinkRenderMode,
    scope: &jbotci_budget::TextScope,
) -> String {
    let mut output = scope.buffer();
    for token in tokens {
        match token {
            CllEbnfToken::Text { body }
            | CllEbnfToken::Operator { body }
            | CllEbnfToken::Hash { body } => output.push_str(body),
            CllEbnfToken::Terminal { body, href }
            | CllEbnfToken::ElidableTerminator { body, href }
            | CllEbnfToken::Nonterminal { body, href } => {
                if link_mode == CllLinkRenderMode::Web
                    && let Some(href) = href
                {
                    output.push_str(
                        &scope.format(format_args!("[{body}]({})", render_ebnf_href(site, href))),
                    );
                } else {
                    output.push_str(body);
                }
            }
        }
    }
    output.into_piece()
}

#[cfg(test)]
#[requires(true)]
#[ensures(true)]
pub(crate) fn render_block_markdown(
    site: &CllSite,
    block: &CllBlock,
    output: &mut String,
    depth: usize,
    link_mode: CllLinkRenderMode,
    dialect: CllMarkdownDialect,
) {
    let scope = jbotci_budget::TextScope::new(
        jbotci_budget::ByteLimit::new(usize::MAX).expect("positive unlimited limit"),
    );
    let mut bounded = scope.buffer();
    render_block_markdown_to(site, block, &mut bounded, depth, link_mode, dialect);
    output.push_str(
        &scope
            .finish(bounded.into_piece())
            .expect("unrestricted Markdown rendering"),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[requires(true)]
    #[ensures(true)]
    fn github(text: &str, at_line_start: bool) -> String {
        let scope = jbotci_budget::TextScope::new(
            jbotci_budget::ByteLimit::new(usize::MAX).expect("positive unlimited limit"),
        );
        escape_github_markdown_in_scope(text, &scope, at_line_start)
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn github_escaper_makes_markup_literal_and_keeps_a_dictionary_token() {
        assert_eq!(github(".ui", true), ".ui");
        assert_eq!(github(".uinai", true), ".uinai");
        assert_eq!(github(".ui", false), ".ui");
        assert_eq!(github("www.example.com", false), r"www\.example.com");

        assert_eq!(github("# a", true), r"\# a");
        assert_eq!(github("## a", true), r"\## a");
        assert_eq!(github("#a", true), "#a");
        assert_eq!(github("# a", false), "# a");
        assert_eq!(github("- a", true), r"\- a");
        assert_eq!(github("+ a", true), r"\+ a");
        assert_eq!(github("roi-nai", true), "roi-nai");
        assert_eq!(github("-nai", false), "-nai");
        assert_eq!(github("a + b", true), "a + b");
        assert_eq!(github("> quote", true), r"\> quote");
        assert_eq!(github("1. aroi", true), r"1\. aroi");
        assert_eq!(github("1. aroi", false), "1. aroi");
        assert_eq!(github("1.5", true), "1.5");
        assert_eq!(github("*a* _b_", false), r"\*a\* \_b\_");
        assert_eq!(github("[a](b)", false), r"\[a\]\(b\)");
        assert_eq!(
            github("https://example.invalid", false),
            r"https\://example.invalid"
        );
        assert_eq!(
            github("http://www.lojban.org", false),
            r"http\://www\.lojban.org"
        );
        assert_eq!(github("user@example.com", false), r"user\@example.com");
        assert_eq!(github("&copy;", false), r"\&copy;");
        assert_eq!(github("<em>", false), r"\<em\>");
        assert_eq!(github("`code`", false), r"\`code\`");
        assert_eq!(github("^2^", false), r"\^2\^");
        assert_eq!(github("a|b", false), r"a\|b");
        assert_eq!(github("![alt](x)", false), r"\!\[alt\]\(x\)");
    }
}
