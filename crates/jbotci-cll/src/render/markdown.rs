//! Markdown rendering of the book model in two dialects.
//!
//! GitHub-flavoured Markdown is what the CLI and the MCP tool emit: pipe
//! tables, `$…$` math, sub- and superscript markers and the book's text
//! verbatim. Discord's dialect has none of those, escapes ordinary text so the
//! client cannot misread it as formatting, and folds tabular content into code
//! blocks. Everything that is book semantics rather than syntax (which link
//! kinds survive a route-free rendering, how a rule-status note is labelled,
//! the example line kinds) is shared between the two.

use std::borrow::Cow;

use super::*;

/// Appends `text` as inert content in the dialect.
#[requires(true)]
#[ensures(true)]
fn push_text(output: &mut String, text: &str, dialect: CllMarkdownDialect) {
    match dialect {
        CllMarkdownDialect::GitHub => output.push_str(text),
        CllMarkdownDialect::Discord => output.push_str(&escape_discord_markdown(text)),
    }
}

/// `text` as inert content in the dialect.
#[requires(true)]
#[ensures(true)]
pub(crate) fn dialect_text(text: &str, dialect: CllMarkdownDialect) -> Cow<'_, str> {
    match dialect {
        CllMarkdownDialect::GitHub => Cow::Borrowed(text),
        CllMarkdownDialect::Discord => Cow::Owned(escape_discord_markdown(text)),
    }
}

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
    match block {
        CllBlock::Paragraph {
            role,
            inlines,
            text,
            ..
        } => {
            let body = if inlines.is_empty() {
                dialect_text(text, dialect)
            } else {
                Cow::Owned(render_inlines_markdown(site, inlines, link_mode, dialect))
            };
            if role.as_ref().is_some_and(CllParagraphRole::is_status_note) {
                push_status_note_markdown(output, &body);
            } else {
                output.push_str(&body);
            }
            output.push_str("\n\n");
        }
        CllBlock::List { ordered, items } => {
            for (index, item) in items.iter().enumerate() {
                let marker = if *ordered {
                    format!("{}.", index + 1)
                } else {
                    "-".to_owned()
                };
                output.push_str(&"  ".repeat(depth));
                output.push_str(&marker);
                output.push(' ');
                let mut item_text = String::new();
                for block in item {
                    render_block_markdown(
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
                render_example_markdown(site, example, output, link_mode, dialect);
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
                    output.push_str(&render_inlines_markdown(site, caption, link_mode, dialect));
                    output.push_str("**\n\n");
                }
                push_discord_rows(
                    header_rows.iter().chain(body_rows.iter()).map(|row| {
                        row.iter()
                            .map(|cell| single_line(&blocks_plain_text(site, &cell.blocks)))
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
                                .map(|inlines| single_line(&inline_plain_text(inlines)))
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
                ));
                output.push_str("**\n\n");
                for block in &entry.blocks {
                    render_block_markdown(site, block, output, depth, link_mode, dialect);
                }
            }
        }
        CllBlock::Media {
            title, src, alt, ..
        } => {
            // Discord does not render Markdown images, so its dialect always
            // carries the description.
            if link_mode == CllLinkRenderMode::Web && dialect == CllMarkdownDialect::GitHub {
                output.push_str(&format!("![{}]({})\n\n", alt, src));
            } else {
                push_text(output, alt, dialect);
                output.push_str("\n\n");
            }
            if let Some(title) = title {
                output.push_str(&render_inlines_markdown(site, title, link_mode, dialect));
                output.push_str("\n\n");
            }
        }
        CllBlock::Rule { term, body, .. } => {
            output.push_str("**");
            push_text(output, term, dialect);
            output.push_str("**\n\n");
            for block in body {
                render_block_markdown(site, block, output, depth, link_mode, dialect);
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
            output.push_str(&"#".repeat(level));
            output.push(' ');
            output.push_str(&render_inlines_markdown(site, inlines, link_mode, dialect));
            output.push_str("\n\n");
        }
        CllBlock::BlockQuote { blocks, .. } => {
            let mut inner = String::new();
            for block in blocks {
                render_block_markdown(site, block, &mut inner, depth, link_mode, dialect);
            }
            for line in inner.trim().lines() {
                output.push_str("> ");
                output.push_str(line);
                output.push('\n');
            }
            output.push('\n');
        }
        CllBlock::Definition { body, .. } | CllBlock::GrammarTemplate { body, .. } => {
            output.push_str(&render_inlines_markdown(site, body, link_mode, dialect));
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
                        site, &line.body, link_mode, dialect,
                    ));
                    if let Some(comment) = &line.comment {
                        output.push_str(" — ");
                        output
                            .push_str(&render_inlines_markdown(site, comment, link_mode, dialect));
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
                    site, &part.body, link_mode, dialect,
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
#[ensures(output.contains(example.label.as_str()) || dialect == CllMarkdownDialect::Discord)]
pub(crate) fn render_example_markdown(
    site: &CllSite,
    example: &CllExample,
    output: &mut String,
    link_mode: CllLinkRenderMode,
    dialect: CllMarkdownDialect,
) {
    match dialect {
        CllMarkdownDialect::GitHub => {
            output.push_str(&format!("### {}", example.label));
            if link_mode == CllLinkRenderMode::Web
                && let Some(parse_href) = &example.parse_href
            {
                output.push_str(&format!(" [Parse]({parse_href})"));
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
        render_block_markdown(site, block, output, 0, link_mode, dialect);
    }
    if example.blocks.is_empty() {
        for line in &example.lines {
            match dialect {
                CllMarkdownDialect::GitHub => {
                    if line.kind == CllExampleLineKind::Text {
                        output.push_str(&line.text);
                    } else {
                        output.push_str(&format!("{}: {}", line.kind.as_str(), line.text));
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
) -> String {
    let mut output = String::new();
    for inline in inlines {
        match inline {
            CllInline::Text(text) => push_text(&mut output, text, dialect),
            CllInline::Emphasis { inlines, .. } => {
                output.push('*');
                output.push_str(&render_inlines_markdown(site, inlines, link_mode, dialect));
                output.push('*');
            }
            CllInline::Quote { inlines, .. } => {
                output.push('"');
                output.push_str(&render_inlines_markdown(site, inlines, link_mode, dialect));
                output.push('"');
            }
            CllInline::LanguageSpan { inlines, .. } | CllInline::CiteTitle { inlines } => {
                match dialect {
                    CllMarkdownDialect::GitHub => {
                        output
                            .push_str(&render_inlines_markdown(site, inlines, link_mode, dialect));
                    }
                    // The web reader sets Lojban and foreign phrases and cited
                    // titles in italics; Discord can do the same.
                    CllMarkdownDialect::Discord => {
                        output.push('*');
                        output
                            .push_str(&render_inlines_markdown(site, inlines, link_mode, dialect));
                        output.push('*');
                    }
                }
            }
            CllInline::Subscript { inlines } => match dialect {
                CllMarkdownDialect::GitHub => {
                    output.push('~');
                    output.push_str(&render_inlines_markdown(site, inlines, link_mode, dialect));
                    output.push('~');
                }
                CllMarkdownDialect::Discord => {
                    output.push_str(&render_inlines_markdown(site, inlines, link_mode, dialect));
                }
            },
            CllInline::Superscript { inlines } => match dialect {
                CllMarkdownDialect::GitHub => {
                    output.push('^');
                    output.push_str(&render_inlines_markdown(site, inlines, link_mode, dialect));
                    output.push('^');
                }
                CllMarkdownDialect::Discord => {
                    output.push_str(&render_inlines_markdown(site, inlines, link_mode, dialect));
                }
            },
            CllInline::Link {
                target,
                inlines,
                kind,
            } => {
                let text = render_inlines_markdown(site, inlines, link_mode, dialect);
                let text = if text.is_empty() {
                    dialect_text(target, dialect)
                } else {
                    Cow::Owned(text)
                };
                match link_mode {
                    CllLinkRenderMode::Web => output.push_str(&format!(
                        "[{}]({})",
                        markdown_link_label_text(&text),
                        cll_link_href(site, *kind, target)
                    )),
                    CllLinkRenderMode::Plain => match kind.plain_disposition() {
                        CllPlainLinkDisposition::KeepContent => output.push_str(&text),
                        CllPlainLinkDisposition::Drop => {}
                    },
                }
            }
            CllInline::Code(text) => match dialect {
                CllMarkdownDialect::GitHub => output.push_str(&format!("`{text}`")),
                CllMarkdownDialect::Discord => output.push_str(&discord_inline_code(text)),
            },
            CllInline::Elidable { shown, inlines, .. } => {
                let text = render_inlines_markdown(site, inlines, link_mode, dialect);
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
                    output.push_str(&discord_inline_code(math_text(text, latex)));
                }
            },
            CllInline::Anchor { .. } => {}
        }
    }
    output
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
fn single_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[requires(true)]
#[ensures(true)]
fn push_code_block(output: &mut String, text: &str, dialect: CllMarkdownDialect) {
    match dialect {
        CllMarkdownDialect::GitHub => {
            output.push_str("```\n");
            output.push_str(text);
            output.push_str("\n```\n\n");
        }
        CllMarkdownDialect::Discord => {
            output.push_str(&discord_code_block(text));
            output.push_str("\n\n");
        }
    }
}

/// Discord has no tables: rows become ` | `-separated lines in a code block,
/// which keeps them legible without relying on column alignment.
#[requires(true)]
#[ensures(true)]
fn push_discord_rows<I>(rows: I, output: &mut String)
where
    I: IntoIterator<Item = Vec<String>>,
{
    let lines = rows
        .into_iter()
        .map(|row| row.join(" | "))
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>();
    if lines.is_empty() {
        return;
    }
    output.push_str(&discord_code_block(&lines.join("\n")));
    output.push_str("\n\n");
}

#[requires(true)]
#[ensures(true)]
fn render_table_markdown(
    site: &CllSite,
    caption: Option<&[CllInline]>,
    header_rows: &[Vec<CllTableCell>],
    body_rows: &[Vec<CllTableCell>],
    output: &mut String,
    link_mode: CllLinkRenderMode,
) {
    if let Some(caption) = caption {
        output.push_str("**");
        output.push_str(&render_inlines_markdown(
            site,
            caption,
            link_mode,
            CllMarkdownDialect::GitHub,
        ));
        output.push_str("**\n\n");
    }
    let rows = header_rows
        .iter()
        .chain(body_rows.iter())
        .collect::<Vec<_>>();
    render_markdown_table_rows(
        rows.iter().map(|row| {
            row.iter()
                .map(|cell| table_cell_markdown_text(site, cell, link_mode))
                .collect::<Vec<_>>()
        }),
        output,
    );
}

#[requires(true)]
#[ensures(true)]
fn render_simple_list_table_markdown(
    site: &CllSite,
    rows: &[Vec<Option<Vec<CllInline>>>],
    output: &mut String,
    link_mode: CllLinkRenderMode,
) {
    render_markdown_table_rows(
        rows.iter().map(|row| {
            row.iter()
                .map(|cell| {
                    cell.as_deref()
                        .map(|inlines| {
                            markdown_table_cell_text(&render_inlines_markdown(
                                site,
                                inlines,
                                link_mode,
                                CllMarkdownDialect::GitHub,
                            ))
                        })
                        .unwrap_or_default()
                })
                .collect::<Vec<_>>()
        }),
        output,
    );
}

#[requires(true)]
#[ensures(true)]
fn render_markdown_table_rows<I>(rows: I, output: &mut String)
where
    I: IntoIterator<Item = Vec<String>>,
{
    let rows = rows.into_iter().collect::<Vec<_>>();
    if rows.is_empty() {
        return;
    }
    let width = rows.iter().map(Vec::len).max().unwrap_or(0);
    if width == 0 {
        return;
    }
    for (row_index, row) in rows.iter().enumerate() {
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
fn markdown_table_cell_text(text: &str) -> String {
    text.replace('|', "\\|").replace('\n', "<br>")
}

#[requires(true)]
#[ensures(true)]
fn table_cell_markdown_text(
    site: &CllSite,
    cell: &CllTableCell,
    link_mode: CllLinkRenderMode,
) -> String {
    let mut text = markdown_table_cell_text(&blocks_plain_text(site, &cell.blocks));
    if link_mode == CllLinkRenderMode::Web
        && let Some(parse_href) = &cell.parse_href
    {
        if !text.is_empty() {
            text.push(' ');
        }
        text.push_str(&format!("[Parse]({parse_href})"));
    }
    text
}

#[requires(true)]
#[ensures(true)]
fn markdown_link_label_text(text: &str) -> String {
    text.replace('[', "\\[").replace(']', "\\]")
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
    output: &mut String,
    link_mode: CllLinkRenderMode,
) {
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
            let body = row
                .cells
                .iter()
                .map(|cell| render_inlines_markdown(site, cell, link_mode, dialect))
                .filter(|cell| !cell.is_empty())
                .collect::<Vec<_>>()
                .join(" ");
            if !body.is_empty() {
                output.push_str(row.kind.as_str());
                output.push_str(": ");
                output.push_str(&body);
                output.push('\n');
            }
        }
        for line in comments {
            output.push_str("comment: ");
            output.push_str(&render_inlines_markdown(site, line, link_mode, dialect));
            output.push('\n');
        }
        for line in natlang {
            output.push_str("natlang: ");
            output.push_str(&render_inlines_markdown(site, line, link_mode, dialect));
            output.push('\n');
        }
        output.push('\n');
        return;
    }

    let table_rows = rows
        .iter()
        .map(|row| {
            row.cells
                .iter()
                .map(|cell| {
                    markdown_table_cell_text(&render_inlines_markdown(
                        site, cell, link_mode, dialect,
                    ))
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    render_markdown_table_rows(table_rows, output);
    for line in comments {
        output.push_str("_");
        output.push_str(&render_inlines_markdown(site, line, link_mode, dialect));
        output.push_str("_\n\n");
    }
    for line in natlang {
        output.push_str("> ");
        output.push_str(&render_inlines_markdown(site, line, link_mode, dialect));
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
    output: &mut String,
    link_mode: CllLinkRenderMode,
) {
    let dialect = CllMarkdownDialect::Discord;
    for row in rows {
        let body = row
            .cells
            .iter()
            .map(|cell| render_inlines_markdown(site, cell, link_mode, dialect))
            .filter(|cell| !cell.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
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
        output.push_str(&render_inlines_markdown(site, line, link_mode, dialect));
        output.push('\n');
    }
    for line in natlang {
        output.push_str("- **natlang**: ");
        output.push_str(&render_inlines_markdown(site, line, link_mode, dialect));
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
    rows: &[Vec<Vec<CllInline>>],
    output: &mut String,
    link_mode: CllLinkRenderMode,
) {
    let dialect = CllMarkdownDialect::GitHub;
    for title in titles {
        output.push_str("**");
        output.push_str(&render_inlines_markdown(site, title, link_mode, dialect));
        output.push_str("**\n\n");
    }
    if headers.is_empty() {
        for row in rows {
            let rendered_cells = row
                .iter()
                .map(|cell| {
                    render_inlines_markdown(site, cell, link_mode, dialect)
                        .trim()
                        .to_owned()
                })
                .filter(|cell| !cell.is_empty())
                .collect::<Vec<_>>();
            if rendered_cells.is_empty() {
                continue;
            }
            output.push_str(&rendered_cells.join(" | "));
            output.push_str("\n\n");
        }
        return;
    }
    let header = headers
        .iter()
        .map(|cell| {
            markdown_table_cell_text(&render_inlines_markdown(site, cell, link_mode, dialect))
        })
        .collect::<Vec<_>>();
    let rendered_rows = rows.iter().map(|row| {
        row.iter()
            .map(|cell| {
                markdown_table_cell_text(&render_inlines_markdown(site, cell, link_mode, dialect))
            })
            .collect::<Vec<_>>()
    });
    render_markdown_table_rows(std::iter::once(header).chain(rendered_rows), output);
}

#[requires(true)]
#[ensures(true)]
fn render_cmavo_list_discord(
    site: &CllSite,
    titles: &[Vec<CllInline>],
    headers: &[Vec<CllInline>],
    rows: &[Vec<Vec<CllInline>>],
    output: &mut String,
    link_mode: CllLinkRenderMode,
) {
    let dialect = CllMarkdownDialect::Discord;
    for title in titles {
        output.push_str("**");
        output.push_str(&render_inlines_markdown(site, title, link_mode, dialect));
        output.push_str("**\n\n");
    }
    if headers.is_empty() {
        for row in rows {
            let rendered_cells = row
                .iter()
                .map(|cell| {
                    render_inlines_markdown(site, cell, link_mode, dialect)
                        .trim()
                        .to_owned()
                })
                .filter(|cell| !cell.is_empty())
                .collect::<Vec<_>>();
            if rendered_cells.is_empty() {
                continue;
            }
            output.push_str(&rendered_cells.join(" · "));
            output.push('\n');
        }
        output.push('\n');
        return;
    }
    let header = headers
        .iter()
        .map(|cell| single_line(&inline_plain_text(cell)))
        .collect::<Vec<_>>();
    let body = rows.iter().map(|row| {
        row.iter()
            .map(|cell| single_line(&inline_plain_text(cell)))
            .collect::<Vec<_>>()
    });
    push_discord_rows(std::iter::once(header).chain(body), output);
}

#[requires(true)]
#[ensures(true)]
fn render_lojbanization_markdown(
    site: &CllSite,
    lines: &[CllLojbanizationLine],
    output: &mut String,
    link_mode: CllLinkRenderMode,
) {
    let dialect = CllMarkdownDialect::GitHub;
    let rows = lines.iter().map(|line| {
        vec![
            line.kind.as_str().to_owned(),
            markdown_table_cell_text(&render_inlines_markdown(
                site, &line.body, link_mode, dialect,
            )),
            line.comment
                .as_deref()
                .map(|comment| {
                    markdown_table_cell_text(&render_inlines_markdown(
                        site, comment, link_mode, dialect,
                    ))
                })
                .unwrap_or_default(),
        ]
    });
    render_markdown_table_rows(rows, output);
}

#[requires(true)]
#[ensures(true)]
fn render_ebnf_markdown(
    site: &CllSite,
    entries: &[CllEbnfEntry],
    output: &mut String,
    link_mode: CllLinkRenderMode,
) {
    for entry in entries {
        for source_anchor_id in &entry.source_anchor_ids {
            output.push_str("<a id=\"");
            output.push_str(&escape_html(source_anchor_id));
            output.push_str("\"></a>");
        }
        output.push_str("**");
        output.push_str(&entry.rule_name);
        output.push_str("** ⩴\n");
        for line in wrap_ebnf_choice_lines(&entry.rhs) {
            output.push_str("  ");
            output.push_str(&render_ebnf_tokens_markdown(site, &line, link_mode));
            output.push('\n');
        }
        output.push_str("\n\n");
    }
}

/// Discord carries no anchors or in-text links, so grammar rules go into one
/// code block, which also preserves their indentation.
#[requires(true)]
#[ensures(true)]
fn render_ebnf_discord(entries: &[CllEbnfEntry], output: &mut String) {
    let mut text = String::new();
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
    output.push_str(&discord_code_block(text.trim_end()));
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
) -> String {
    let mut output = String::new();
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
                    output.push_str(&format!("[{body}]({})", render_ebnf_href(site, href)));
                } else {
                    output.push_str(body);
                }
            }
        }
    }
    output
}
