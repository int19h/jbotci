//! Markdown rendering of the book model in two dialects.
//!
//! GitHub-flavoured Markdown is what the CLI and the MCP tool emit: pipe
//! tables, `$…$` math, sub- and superscript markers and the book's text
//! verbatim. A joined run of cmavo parts is not book prose. That run uses one
//! GitHub escaper so markup in the run stays text. Discord's dialect has none
//! of those, escapes ordinary text so the
//! client cannot misread it as formatting, and folds tabular content into code
//! blocks. Both dialects omit link destinations. HTML rendering is separate
//! and can keep them. Which link kinds survive, how a rule-status note is
//! labelled, and the example line kinds are shared between the two dialects.

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
                Cow::Owned(render_inlines_markdown(site, inlines, dialect, &scope))
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
                    scope.format(format_args!("{}.", index + 1))
                } else {
                    "-".to_owned()
                };
                output.push_str(&scope.repeat("  ", depth));
                output.push_str(&marker);
                output.push(' ');
                let mut item_text = scope.buffer();
                for block in item {
                    render_block_markdown_to(site, block, &mut item_text, depth + 1, dialect);
                }
                output.push_str(item_text.trim());
                output.push('\n');
            }
            output.push('\n');
        }
        CllBlock::Example { example_id } => {
            if let Some(example) = cll_lookup_example(site, example_id) {
                render_example_markdown_to(site, example, output, dialect);
            }
        }
        CllBlock::Table {
            caption,
            header_rows,
            body_rows,
            ..
        } => match dialect {
            CllMarkdownDialect::GitHub => {
                render_table_markdown(site, caption.as_deref(), header_rows, body_rows, output)
            }
            CllMarkdownDialect::Discord => {
                if let Some(caption) = caption {
                    output.push_str("**");
                    output.push_str(&render_inlines_markdown(site, caption, dialect, &scope));
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
                render_simple_list_table_markdown(site, rows, output);
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
                output.push_str(&render_inlines_markdown(site, &entry.term, dialect, &scope));
                output.push_str("**\n\n");
                for block in &entry.blocks {
                    render_block_markdown_to(site, block, output, depth, dialect);
                }
            }
        }
        CllBlock::Media { title, alt, .. } => {
            // Neither dialect writes an image link. The description is the
            // content a reader can use.
            push_text(output, alt, dialect);
            output.push_str("\n\n");
            if let Some(title) = title {
                output.push_str(&render_inlines_markdown(site, title, dialect, &scope));
                output.push_str("\n\n");
            }
        }
        CllBlock::Rule { term, body, .. } => {
            output.push_str("**");
            push_text(output, term, dialect);
            output.push_str("**\n\n");
            for block in body {
                render_block_markdown_to(site, block, output, depth, dialect);
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
            output.push_str(&render_inlines_markdown(site, inlines, dialect, &scope));
            output.push_str("\n\n");
        }
        CllBlock::BlockQuote { blocks, .. } => {
            let mut inner = scope.buffer();
            for block in blocks {
                render_block_markdown_to(site, block, &mut inner, depth, dialect);
            }
            // The inner blocks end with a blank line. trim removes it.
            // The quote writes that blank line again after its own lines.
            let quoted = inner.trim();
            if quoted.is_empty() {
                output.push('\n');
            } else {
                push_markdown_quote_lines(output, quoted, true);
                output.push_str("\n\n");
            }
        }
        CllBlock::Definition { body, .. } | CllBlock::GrammarTemplate { body, .. } => {
            output.push_str(&render_inlines_markdown(site, body, dialect, &scope));
            output.push_str("\n\n");
        }
        CllBlock::InterlinearGloss {
            aligned,
            rows,
            natlang,
            comments,
            ..
        } => match dialect {
            CllMarkdownDialect::GitHub => {
                render_interlinear_markdown(site, *aligned, rows, natlang, comments, output)
            }
            CllMarkdownDialect::Discord => {
                render_interlinear_discord(site, rows, natlang, comments, output);
            }
        },
        CllBlock::CmavoList {
            titles,
            headers,
            rows,
            ..
        } => match dialect {
            CllMarkdownDialect::GitHub => {
                render_cmavo_list_markdown(site, titles, headers, rows, output);
            }
            CllMarkdownDialect::Discord => {
                render_cmavo_list_discord(site, titles, headers, rows, output);
            }
        },
        CllBlock::Lojbanization { lines, .. } => match dialect {
            CllMarkdownDialect::GitHub => {
                render_lojbanization_markdown(site, lines, output);
            }
            CllMarkdownDialect::Discord => {
                for line in lines {
                    output.push_str("- **");
                    output.push_str(line.kind.as_str());
                    output.push_str("**: ");
                    output.push_str(&render_inlines_markdown(site, &line.body, dialect, &scope));
                    if let Some(comment) = &line.comment {
                        output.push_str(" — ");
                        output.push_str(&render_inlines_markdown(site, comment, dialect, &scope));
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
                output.push_str(&render_inlines_markdown(site, &part.body, dialect, &scope));
                output.push('\n');
            }
            output.push('\n');
        }
        CllBlock::Ebnf { entries, .. } => match dialect {
            CllMarkdownDialect::GitHub => render_ebnf_markdown(entries, output),
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
    dialect: CllMarkdownDialect,
) {
    let scope = output.scope();
    match dialect {
        CllMarkdownDialect::GitHub => {
            output.push_str(&scope.format(format_args!("### {}\n\n", example.label)));
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
        render_block_markdown_to(site, block, output, 0, dialect);
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
    dialect: CllMarkdownDialect,
    scope: &jbotci_budget::TextScope,
) -> String {
    let mut output = scope.buffer();
    for inline in inlines {
        match inline {
            CllInline::Text(text) => push_text(&mut output, text, dialect),
            CllInline::Emphasis { inlines, .. } => {
                output.push('*');
                output.push_str(&render_inlines_markdown(site, inlines, dialect, &scope));
                output.push('*');
            }
            CllInline::Quote { inlines, .. } => {
                output.push('"');
                output.push_str(&render_inlines_markdown(site, inlines, dialect, &scope));
                output.push('"');
            }
            CllInline::LanguageSpan { inlines, .. } | CllInline::CiteTitle { inlines } => {
                match dialect {
                    CllMarkdownDialect::GitHub => {
                        output.push_str(&render_inlines_markdown(site, inlines, dialect, &scope));
                    }
                    // The web reader sets Lojban and foreign phrases and cited
                    // titles in italics; Discord can do the same.
                    CllMarkdownDialect::Discord => {
                        output.push('*');
                        output.push_str(&render_inlines_markdown(site, inlines, dialect, &scope));
                        output.push('*');
                    }
                }
            }
            CllInline::Subscript { inlines } => match dialect {
                CllMarkdownDialect::GitHub => {
                    output.push('~');
                    output.push_str(&render_inlines_markdown(site, inlines, dialect, &scope));
                    output.push('~');
                }
                CllMarkdownDialect::Discord => {
                    output.push_str(&render_inlines_markdown(site, inlines, dialect, &scope));
                }
            },
            CllInline::Superscript { inlines } => match dialect {
                CllMarkdownDialect::GitHub => {
                    output.push('^');
                    output.push_str(&render_inlines_markdown(site, inlines, dialect, &scope));
                    output.push('^');
                }
                CllMarkdownDialect::Discord => {
                    output.push_str(&render_inlines_markdown(site, inlines, dialect, &scope));
                }
            },
            CllInline::Link {
                target,
                inlines,
                kind,
            } => {
                let text = render_inlines_markdown(site, inlines, dialect, &scope);
                let text = if text.is_empty() {
                    dialect_text_in_scope(target, dialect, &scope)
                } else {
                    Cow::Owned(text)
                };
                match kind.plain_disposition() {
                    CllPlainLinkDisposition::KeepContent => output.push_str(&text),
                    CllPlainLinkDisposition::Drop => {}
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
                let text = render_inlines_markdown(site, inlines, dialect, &scope);
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
) {
    let scope = output.scope();
    if let Some(caption) = caption {
        output.push_str("**");
        output.push_str(&render_inlines_markdown(
            site,
            caption,
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
                .map(|cell| table_cell_markdown_text(site, cell, &scope))
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

/// One GitHub pipe-table cell.
///
/// Each line ending becomes `<br>`. A line ending is LF, CRLF, or a lone
/// CR. CRLF is one ending. When `escape_pipe` is set, each `|` becomes
/// `\|`. A word-part cell passes false. The GitHub escaper already wrote
/// `\|` in that cell.
#[requires(true)]
#[ensures(scope.is_exceeded() || (!ret.contains('\n') && !ret.contains('\r')))]
#[ensures(scope.is_exceeded() || ret.matches('\\').count() == text.matches('\\').count() + if escape_pipe { text.matches('|').count() } else { 0 })]
fn github_pipe_cell_text(
    text: &str,
    scope: &jbotci_budget::TextScope,
    escape_pipe: bool,
) -> String {
    let mut output = scope.buffer();
    let bytes = text.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'\r' {
            output.push_str("<br>");
            index += 1;
            if bytes.get(index) == Some(&b'\n') {
                index += 1;
            }
            continue;
        }
        if bytes[index] == b'\n' {
            output.push_str("<br>");
            index += 1;
            continue;
        }
        if escape_pipe && bytes[index] == b'|' {
            output.push_str("\\|");
            index += 1;
            continue;
        }
        let start = index;
        index += 1;
        while index < bytes.len()
            && bytes[index] != b'\n'
            && bytes[index] != b'\r'
            && !(escape_pipe && bytes[index] == b'|')
        {
            index += 1;
        }
        output.push_str(&text[start..index]);
    }
    output.into_piece()
}

/// A pipe-table cell that escapes each `|`.
///
/// Header cells and ordinary cells use this. Line endings go through
/// `github_pipe_cell_text`.
#[requires(true)]
#[ensures(scope.is_exceeded() || (!ret.contains('\n') && !ret.contains('\r')))]
fn markdown_table_cell_text(text: &str, scope: &jbotci_budget::TextScope) -> String {
    github_pipe_cell_text(text, scope, true)
}

#[requires(true)]
#[ensures(true)]
fn table_cell_markdown_text(
    site: &CllSite,
    cell: &CllTableCell,
    scope: &jbotci_budget::TextScope,
) -> String {
    markdown_table_cell_text(
        &blocks_plain_text_in_scope(site, &cell.blocks, &scope),
        &scope,
    )
}

#[requires(true)]
#[ensures(true)]
fn render_interlinear_markdown(
    site: &CllSite,
    aligned: bool,
    rows: &[CllInterlinearRow],
    natlang: &[Vec<CllInline>],
    comments: &[Vec<CllInline>],
    output: &mut jbotci_budget::TextBuffer,
) {
    let scope = output.scope();
    let dialect = CllMarkdownDialect::GitHub;
    if !aligned {
        for row in rows {
            let body = scope.join(
                row.cells
                    .iter()
                    .map(|cell| render_inlines_markdown(site, cell, dialect, &scope))
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
            output.push_str(&render_inlines_markdown(site, line, dialect, &scope));
            output.push('\n');
        }
        for line in natlang {
            output.push_str("natlang: ");
            output.push_str(&render_inlines_markdown(site, line, dialect, &scope));
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
                    &render_inlines_markdown(site, cell, dialect, &scope),
                    &scope,
                )
            })
            .collect::<Vec<_>>()
    });
    render_markdown_table_rows(table_rows, output, width);
    for line in comments {
        output.push_str("_");
        output.push_str(&render_inlines_markdown(site, line, dialect, &scope));
        output.push_str("_\n\n");
    }
    for line in natlang {
        output.push_str("> ");
        output.push_str(&render_inlines_markdown(site, line, dialect, &scope));
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
) {
    let scope = output.scope();
    let dialect = CllMarkdownDialect::Discord;
    for row in rows {
        let body = scope.join(
            row.cells
                .iter()
                .map(|cell| render_inlines_markdown(site, cell, dialect, &scope))
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
        output.push_str(&render_inlines_markdown(site, line, dialect, &scope));
        output.push('\n');
    }
    for line in natlang {
        output.push_str("- **natlang**: ");
        output.push_str(&render_inlines_markdown(site, line, dialect, &scope));
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
) {
    let scope = output.scope();
    let dialect = CllMarkdownDialect::GitHub;
    for title in titles {
        output.push_str("**");
        output.push_str(&render_inlines_markdown(site, title, dialect, &scope));
        output.push_str("**\n\n");
    }
    if headers.is_empty() {
        for row in rows {
            let mut rendered_cells = Vec::new();
            let mut at_line_start = true;
            for cell in row {
                let rendered = if let Some(parts) = cell.word_parts() {
                    render_github_cmavo_word_parts(&parts, &scope, at_line_start)
                } else {
                    render_inlines_markdown(site, cell.inlines(), dialect, &scope)
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
                &render_inlines_markdown(site, cell, dialect, &scope),
                &scope,
            )
        })
        .collect::<Vec<_>>();
    let rendered_rows = rows.iter().map(|row| {
        row.iter()
            .map(|cell| {
                if let Some(parts) = cell.word_parts() {
                    // The cell sits after `| `, so it does not start a line.
                    // The GitHub escaper already wrote `\|`. Pass false so
                    // the cell helper does not escape `|` again. A part has
                    // no line ending, so this text has none.
                    github_pipe_cell_text(
                        &render_github_cmavo_word_parts(&parts, &scope, false),
                        &scope,
                        false,
                    )
                } else {
                    markdown_table_cell_text(
                        &render_inlines_markdown(site, cell.inlines(), dialect, &scope),
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
/// when it completes an ordered-list marker at the start of a line. `!` is
/// in this set. An unescaped `!` starts a Markdown image. The escaper writes
/// the character as text.
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
            | '!'
            | '~'
            | '$'
            | '^'
            | '&'
            | '|'
    )
}

/// Bytes of a heading, bullet, ordered list, or thematic break at the start of `tail`.
///
/// The caller has already skipped up to three leading spaces. A heading is one
/// to six `#` characters and then a space, a tab, or the end of the line. A
/// bullet is `-` or `+` and then a space, a tab, or the end of the line. An
/// ordered list is one to nine digits, a `.`, and then a space, a tab, or the
/// end of the line. A thematic break is three or more of the same `-`, `*`, or
/// `_`, with spaces or tabs between them and nothing else on the line.
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
    github_thematic_break_bytes(tail)
}

/// One when `tail` is a thematic break, so the caller escapes its first byte.
///
/// The line is three or more of the same `-`, `*`, or `_`. Spaces or tabs may
/// sit between those characters. No other character is present.
#[requires(true)]
#[ensures(ret.is_none_or(|skip| skip == 1 && skip <= tail.len()))]
fn github_thematic_break_bytes(tail: &str) -> Option<usize> {
    let bytes = tail.as_bytes();
    let Some(&marker) = bytes.first() else {
        return None;
    };
    if !matches!(marker, b'-' | b'*' | b'_') {
        return None;
    }
    let mut count = 0;
    for byte in bytes {
        if *byte == marker {
            count += 1;
        } else if *byte != b' ' && *byte != b'\t' {
            return None;
        }
    }
    (count >= 3).then_some(1)
}

/// True when `tail` is a setext heading underline.
///
/// The caller has skipped up to three leading spaces. The line is one or more
/// `=` characters, or one or more `-` characters, and then only spaces or tabs.
/// A vertical tab and a form feed do not count. GitHub accepts only a space
/// or a tab after that underline. If the previous line has text, the caller
/// uses this.
#[requires(true)]
#[ensures(!ret || tail.starts_with(['=', '-']))]
fn github_setext_underline(tail: &str) -> bool {
    let bytes = tail.as_bytes();
    let Some(&marker) = bytes.first() else {
        return false;
    };
    if marker != b'=' && marker != b'-' {
        return false;
    }
    let count = bytes.iter().take_while(|byte| **byte == marker).count();
    count >= 1
        && bytes[count..]
            .iter()
            .all(|byte| *byte == b' ' || *byte == b'\t')
}

/// True when `tail` is a one-column table delimiter.
///
/// The caller has skipped up to three leading spaces. The line starts with
/// `:` or `-`. It holds only those two characters, spaces, tabs, vertical
/// tabs, and form feeds. It has at least one `-`. GitHub accepts a vertical
/// tab and a form feed as a space in a delimiter line. If the previous line
/// has text, the caller uses this. A line of only `-` is also a setext
/// underline. Both rules escape the first character.
#[requires(true)]
#[ensures(!ret || (tail.starts_with([':', '-']) && tail.contains('-')))]
fn github_one_column_delimiter(tail: &str) -> bool {
    let bytes = tail.as_bytes();
    matches!(bytes.first(), Some(b':' | b'-'))
        && bytes.contains(&b'-')
        && bytes
            .iter()
            .all(|byte| matches!(*byte, b':' | b'-' | b' ' | b'\t' | b'\x0b' | b'\x0c'))
}

/// Writes `marker` so GitHub does not treat it as a heading, a bullet, or a list.
///
/// `marker` is one of `#`, `-`, `+`, `*`, `_`, `=`, and `:`, or it is one or
/// more ASCII digits and then `.`.
#[requires({
    let bytes = marker.as_bytes();
    matches!(bytes, [b'#' | b'-' | b'+' | b'*' | b'_' | b'=' | b':'])
        || (bytes.len() >= 2
            && bytes[..bytes.len() - 1].iter().all(u8::is_ascii_digit)
            && bytes.last() == Some(&b'.'))
})]
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

/// Writes inline GitHub text. A markup character gets a backslash.
#[requires(true)]
#[ensures(true)]
fn push_github_inline(output: &mut jbotci_budget::TextBuffer, text: &str) {
    for character in text.chars() {
        if github_inline_escape(character) {
            output.push('\\');
        }
        output.push(character);
    }
}

/// The first indent byte when `line` is an indented code block.
///
/// A space counts as one column. A tab moves to the next column that is a
/// multiple of four. The result is that first byte when the indent reaches
/// four columns and a later character is not a space or a tab. A blank line
/// returns nothing. This function does not look at the previous line.
#[requires(true)]
#[ensures(ret.is_none_or(|byte| {
    (byte == b' ' || byte == b'\t') && line.as_bytes().first() == Some(&byte)
}))]
fn github_indented_code_byte(line: &str) -> Option<u8> {
    let bytes = line.as_bytes();
    let mut column = 0usize;
    for (index, byte) in bytes.iter().enumerate() {
        column = match *byte {
            b' ' => column + 1,
            b'\t' => column + 4 - (column % 4),
            _ => return None,
        };
        if column >= 4 {
            let rest_is_blank = bytes[index + 1..]
                .iter()
                .all(|later| *later == b' ' || *later == b'\t');
            if rest_is_blank {
                return None;
            }
            return Some(bytes[0]);
        }
    }
    None
}

/// Writes one GitHub line.
///
/// If `at_line_start` is set, a heading, a list, or a thematic break on this
/// line stays text. If `under_text` is also set, a setext underline stays
/// text, and so does a one-column table line. An indented code block stays
/// text when the line starts the text or follows a blank line.
#[requires(!line.contains('\n') && !line.contains('\r'))]
#[ensures(true)]
fn push_github_line(
    output: &mut jbotci_budget::TextBuffer,
    line: &str,
    at_line_start: bool,
    under_text: bool,
) {
    // An indented code block is a non-blank line whose indent reaches four
    // columns. It opens when the line starts the text or follows a blank line.
    // A space is one column. A tab moves to the next multiple of four. A
    // backslash before a space is not an escape. `&#32;` or `&#9;` is ordinary
    // text, so the indent is gone. The other characters stay in place. Marker
    // rules do not run on this line, so `    ---` does not become a thematic
    // break.
    if at_line_start && !under_text {
        if let Some(indent) = github_indented_code_byte(line) {
            output.push_str(if indent == b' ' { "&#32;" } else { "&#9;" });
            push_github_inline(output, &line[1..]);
            return;
        }
    }
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
            let skip = github_line_marker_bytes(tail).or_else(|| {
                (under_text && (github_setext_underline(tail) || github_one_column_delimiter(tail)))
                    .then_some(1)
            });
            if let Some(skip) = skip {
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

/// True when escaped `text` has no live GitHub line marker.
///
/// A line that starts the text is checked when `at_line_start` is set. Each
/// later line starts a line. Up to three leading spaces are indent. A fourth
/// space is an indented code line, so it is not a marker. A setext underline
/// and a one-column delimiter count only when the previous line has text.
#[requires(true)]
#[ensures(true)]
fn github_escaped_lines_stay_text(text: &str, at_line_start: bool) -> bool {
    let mut previous_has_text = false;
    for (index, (line, _)) in super::commonmark_lines(text).into_iter().enumerate() {
        let starts_line = at_line_start || index > 0;
        if starts_line {
            let bytes = line.as_bytes();
            let mut indent = 0;
            while indent < bytes.len() && indent < 3 && bytes[indent] == b' ' {
                indent += 1;
            }
            if bytes.get(indent) != Some(&b' ') {
                let tail = &line[indent..];
                if github_line_marker_bytes(tail).is_some()
                    || (previous_has_text
                        && (github_setext_underline(tail) || github_one_column_delimiter(tail)))
                {
                    return false;
                }
            }
        }
        previous_has_text = (index == 0 && !at_line_start) || super::commonmark_line_has_text(line);
    }
    true
}

/// `text` as literal GitHub Markdown.
///
/// This is the one GitHub escaper. Book prose does not use it, because the
/// reviewed section text is copied as it is. A cmavo-part run does use it.
/// If `text` begins a line, pass `at_line_start` as true. A line ends at LF,
/// CRLF, or a lone CR. The ending stays in the output. A dictionary token
/// such as `.ui` is unchanged. GitHub can turn a bare address into a link.
/// This escaper does not stop that.
#[requires(true)]
#[ensures(scope.is_exceeded() || (text.is_empty() == ret.is_empty()))]
#[ensures(scope.is_exceeded() || !ret.contains("]("))]
#[ensures(
    scope.is_exceeded()
        || !crate::cmavo_table::is_dictionary_token(text)
        || ret == text
)]
#[expensive_ensures(
    scope.is_exceeded() || github_escaped_lines_stay_text(&ret, at_line_start)
)]
fn escape_github_markdown_in_scope(
    text: &str,
    scope: &jbotci_budget::TextScope,
    at_line_start: bool,
) -> String {
    let mut escaped = scope.buffer();
    let mut previous_has_text = false;
    for (index, (line, ending)) in super::commonmark_lines(text).into_iter().enumerate() {
        push_github_line(
            &mut escaped,
            line,
            at_line_start || index > 0,
            previous_has_text,
        );
        escaped.push_str(ending);
        // Line zero of a run that continues earlier text already has text
        // before it. A blank line there still counts as text. Every later
        // line uses the CommonMark blank-line rule: only a space or a tab
        // is blank. A no-break space is text.
        previous_has_text = (index == 0 && !at_line_start) || super::commonmark_line_has_text(line);
    }
    escaped.into_piece()
}

/// GitHub text for adjacent valsi parts, as one word.
///
/// The function joins every part, then escapes that word once. When the word
/// starts a line, a heading, a list, or a thematic break in it stays text.
/// No space is added between parts. A dictionary token such as `.ui` is
/// written as it is.
#[requires(parts.len() >= 2)]
#[ensures(scope.is_exceeded() || !ret.is_empty())]
#[ensures(scope.is_exceeded() || !ret.contains("]("))]
fn render_github_cmavo_word_parts(
    parts: &[CllCmavoWordPart<'_>],
    scope: &jbotci_budget::TextScope,
    at_line_start: bool,
) -> String {
    let word = crate::cmavo_parts_plain_text(parts, scope);
    escape_github_markdown_in_scope(&word, scope, at_line_start)
}

#[requires(true)]
#[ensures(true)]
fn render_cmavo_list_discord(
    site: &CllSite,
    titles: &[Vec<CllInline>],
    headers: &[Vec<CllInline>],
    rows: &[Vec<CllCmavoListCell>],
    output: &mut jbotci_budget::TextBuffer,
) {
    let scope = output.scope();
    let dialect = CllMarkdownDialect::Discord;
    for title in titles {
        output.push_str("**");
        output.push_str(&render_inlines_markdown(site, title, dialect, &scope));
        output.push_str("**\n\n");
    }
    if headers.is_empty() {
        for row in rows {
            let mut rendered_cells = Vec::new();
            let mut at_line_start = true;
            for cell in row {
                let rendered = if let Some(parts) = cell.word_parts() {
                    render_discord_cmavo_word_parts(&parts, &scope, at_line_start)
                } else {
                    render_inlines_markdown(site, cell.inlines(), dialect, &scope)
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
/// The function joins the parts, then escapes that word once. It wraps the
/// word in one italic span when every part is unlinked. When the word starts
/// a line, a heading or a list in it stays text. No space is added between
/// parts. A dictionary token such as `.ui` is written as it is.
#[requires(parts.len() >= 2)]
#[ensures(scope.is_exceeded() || !ret.is_empty())]
#[ensures(scope.is_exceeded() || !ret.contains("]("))]
fn render_discord_cmavo_word_parts(
    parts: &[CllCmavoWordPart<'_>],
    scope: &jbotci_budget::TextScope,
    at_line_start: bool,
) -> String {
    let word = crate::cmavo_parts_plain_text(parts, scope);
    let word = escape_discord_markdown_at(&word, scope, at_line_start);
    if parts.iter().all(|part| part.target.is_none()) {
        scope.format(format_args!("*{word}*"))
    } else {
        word
    }
}

#[requires(true)]
#[ensures(true)]
fn render_lojbanization_markdown(
    site: &CllSite,
    lines: &[CllLojbanizationLine],
    output: &mut jbotci_budget::TextBuffer,
) {
    let scope = output.scope();
    let dialect = CllMarkdownDialect::GitHub;
    let rows = lines.iter().map(|line| {
        vec![
            line.kind.as_str().to_owned(),
            markdown_table_cell_text(
                &render_inlines_markdown(site, &line.body, dialect, &scope),
                &scope,
            ),
            line.comment
                .as_deref()
                .map(|comment| {
                    markdown_table_cell_text(
                        &render_inlines_markdown(site, comment, dialect, &scope),
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
fn render_ebnf_markdown(entries: &[CllEbnfEntry], output: &mut jbotci_budget::TextBuffer) {
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
            output.push_str(&render_ebnf_tokens_markdown(&line, &scope));
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
    tokens: &[CllEbnfToken],
    scope: &jbotci_budget::TextScope,
) -> String {
    let mut output = scope.buffer();
    for token in tokens {
        match token {
            CllEbnfToken::Text { body }
            | CllEbnfToken::Operator { body }
            | CllEbnfToken::Hash { body } => output.push_str(body),
            CllEbnfToken::Terminal { body, .. }
            | CllEbnfToken::ElidableTerminator { body, .. }
            | CllEbnfToken::Nonterminal { body, .. } => output.push_str(body),
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
    dialect: CllMarkdownDialect,
) {
    let scope = jbotci_budget::TextScope::new(
        jbotci_budget::ByteLimit::new(usize::MAX).expect("positive unlimited limit"),
    );
    let mut bounded = scope.buffer();
    render_block_markdown_to(site, block, &mut bounded, depth, dialect);
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
        assert_eq!(github("www.example.com", false), "www.example.com");

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
            "https://example.invalid"
        );
        assert_eq!(
            github("http://www.lojban.org", false),
            "http://www.lojban.org"
        );
        assert_eq!(github("user@example.com", false), "user@example.com");
        assert_eq!(github("mailto:a@b.co", false), "mailto:a@b.co");
        assert_eq!(github("xmpp:a@b.co", false), "xmpp:a@b.co");
        assert_eq!(github("&copy;", false), r"\&copy;");
        assert_eq!(github("<em>", false), r"\<em\>");
        assert_eq!(github("`code`", false), r"\`code\`");
        assert_eq!(github("^2^", false), r"\^2\^");
        assert_eq!(github("a|b", false), r"a\|b");
        assert_eq!(github("![alt](x)", false), r"\!\[alt\]\(x\)");

        assert_eq!(github("---", true), r"\---");
        assert_eq!(github("---", false), "---");
        assert_eq!(github("----", true), r"\----");
        assert_eq!(github("---\t", true), "\\---\t");
        assert_eq!(github("- - -", true), r"\- - -");
        assert_eq!(github("   ---", true), r"   \---");
        assert_eq!(github("    ---\n", true), "&#32;   ---\n");
        assert_eq!(github("    ---\r\n", true), "&#32;   ---\r\n");
        assert_eq!(github("    ---\r", true), "&#32;   ---\r");
        assert_eq!(github("\t---\n", true), "&#9;---\n");
        assert_eq!(github("\t---\r\n", true), "&#9;---\r\n");
        assert_eq!(github("\t---\r", true), "&#9;---\r");
        assert_eq!(github("***", true), r"\*\*\*");
        assert_eq!(github("___", true), r"\_\_\_");
        assert_eq!(github("* * *", true), r"\* \* \*");
        assert_eq!(github("--", true), "--");
        assert_eq!(github("---a", true), "---a");
        assert_eq!(github("===", true), "===");
        assert_eq!(github("Title\n---", true), "Title\n\\---");
        assert_eq!(github("Title\n--", true), "Title\n\\--");
        assert_eq!(github("Title\n=", true), "Title\n\\=");
        assert_eq!(github("Title\n===", false), "Title\n\\===");
        assert_eq!(github("Title\n\n=", true), "Title\n\n=");
        assert_eq!(github("Title\n= =", true), "Title\n= =");
        assert_eq!(github("---\r\n", true), "\\---\r\n");
        assert_eq!(github("Title\r\n==\r\n", true), "Title\r\n\\==\r\n");
        assert_eq!(github("#\r\n", true), "\\#\r\n");
        assert_eq!(github("1.\r\n", true), "1\\.\r\n");
        assert_eq!(github("Title\r==\r", true), "Title\r\\==\r");
        assert_eq!(github("---\r", true), "\\---\r");
        assert_eq!(github("\n===", false), "\n\\===");
        assert_eq!(github("   \n--", false), "   \n\\--");
        assert_eq!(github("\n\n===", false), "\n\n===");
        assert_eq!(github("\n    code", false), "\n    code");
        assert_eq!(github("\u{00a0}\n===", true), "\u{00a0}\n\\===");
        assert_eq!(github("Title\n:-", true), "Title\n\\:-");
        assert_eq!(github("Title\n-:", true), "Title\n\\-:");
        assert_eq!(github("Title\n:-:", true), "Title\n\\:-:");
        assert_eq!(github("Title\n   :-", true), "Title\n   \\:-");
        assert_eq!(github("Title\r:-", true), "Title\r\\:-");
        assert_eq!(github("Title\r\n:-:", true), "Title\r\n\\:-:");
        assert_eq!(github(":-", true), ":-");
        assert_eq!(github("Title\n:", true), "Title\n:");
        assert_eq!(github("Title\n:::", true), "Title\n:::");
        assert_eq!(github("Title\n:-\u{000b}", true), "Title\n\\:-\u{000b}");
        assert_eq!(github("Title\n:-\u{000c}", true), "Title\n\\:-\u{000c}");
        assert_eq!(github("Title\n:- \u{000c}", true), "Title\n\\:- \u{000c}");
        assert_eq!(github("Title\n-\u{000c}", true), "Title\n\\-\u{000c}");
        assert_eq!(github("Title\n=\u{000b}", true), "Title\n=\u{000b}");
        assert_eq!(github("Title\n=\u{000c}", true), "Title\n=\u{000c}");
        assert!(github_escaped_lines_stay_text(
            &github("Title\n:-", true),
            true
        ));
        assert!(github_escaped_lines_stay_text(
            &github("Title\n:-\u{000c}", true),
            true
        ));
        assert!(github_escaped_lines_stay_text(&github(":-", true), true));
        assert!(github_escaped_lines_stay_text(
            &github("Title\n\n=", true),
            true
        ));
        assert!(github_escaped_lines_stay_text(&github("# a", false), false));
        assert!(!github_escaped_lines_stay_text("Title\n:-", true));
        assert!(!github_escaped_lines_stay_text("# a", true));
        assert!(github_escaped_lines_stay_text(":-", true));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_joined_word_has_no_link_and_no_comment() {
        let site = crate::import::embedded_cll_site().expect("embedded CLL site");
        let word = CllCmavoListCell::Word {
            inlines: vec![
                CllInline::LanguageSpan {
                    kind: CllLanguageSpanKind::JboPhrase,
                    language: None,
                    inlines: vec![CllInline::Text("*www.example.com".to_owned())],
                },
                CllInline::Link {
                    target: "roi".to_owned(),
                    inlines: vec![CllInline::Text("roi".to_owned())],
                    kind: CllLinkKind::Dictionary,
                },
            ],
        };
        assert!(word.word_parts().is_some());
        let block = CllBlock::CmavoList {
            id: None,
            titles: Vec::new(),
            headers: Vec::new(),
            rows: vec![vec![
                word,
                CllCmavoListCell::Other {
                    inlines: vec![CllInline::Text("never".to_owned())],
                },
            ]],
        };
        let mut rendered = String::new();
        render_block_markdown(site, &block, &mut rendered, 0, CllMarkdownDialect::GitHub);
        assert_eq!(rendered, "\\*www.example.comroi | never\n\n");
        assert!(!rendered.contains("<!-- -->"));
        assert!(!rendered.contains("]("));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_headerless_discord_row_escapes_a_marker_at_the_end_of_a_cell() {
        for (cell, escaped) in [("-", "\\-"), ("1.", "1\\."), ("-#", "\\-#")] {
            let block = CllBlock::CmavoList {
                id: None,
                titles: Vec::new(),
                headers: Vec::new(),
                rows: vec![vec![
                    CllCmavoListCell::Word {
                        inlines: vec![CllInline::Text(cell.to_owned())],
                    },
                    CllCmavoListCell::Other {
                        inlines: vec![CllInline::Text("none".to_owned())],
                    },
                ]],
            };
            assert_eq!(
                render_dialect(&block, CllMarkdownDialect::Discord),
                format!("{escaped} · none\n\n"),
                "{cell:?}"
            );
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_headered_word_cell_escapes_a_pipe_inside_a_part() {
        let site = crate::import::embedded_cll_site().expect("embedded CLL site");
        let word = CllCmavoListCell::Word {
            inlines: vec![
                CllInline::LanguageSpan {
                    kind: CllLanguageSpanKind::JboPhrase,
                    language: None,
                    inlines: vec![CllInline::Text("a|b".to_owned())],
                },
                CllInline::LanguageSpan {
                    kind: CllLanguageSpanKind::JboPhrase,
                    language: None,
                    inlines: vec![CllInline::Text("z".to_owned())],
                },
            ],
        };
        assert!(word.word_parts().is_some());
        let block = CllBlock::CmavoList {
            id: None,
            titles: Vec::new(),
            headers: vec![
                vec![CllInline::Text("cmavo".to_owned())],
                vec![CllInline::Text("gloss".to_owned())],
            ],
            rows: vec![vec![
                word,
                CllCmavoListCell::Other {
                    inlines: vec![CllInline::Text("never".to_owned())],
                },
            ]],
        };
        let mut web = String::new();
        render_block_markdown(site, &block, &mut web, 0, CllMarkdownDialect::GitHub);
        assert_eq!(
            web,
            "| cmavo | gloss |\n| --- | --- |\n| a\\|bz | never |\n\n"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_word_cell_with_a_line_ending_uses_the_ordinary_cell() {
        let site = crate::import::embedded_cll_site().expect("embedded CLL site");
        let word = CllCmavoListCell::Word {
            inlines: vec![
                CllInline::LanguageSpan {
                    kind: CllLanguageSpanKind::JboPhrase,
                    language: None,
                    inlines: vec![CllInline::Text("a|b\r\nc\nd\re".to_owned())],
                },
                CllInline::LanguageSpan {
                    kind: CllLanguageSpanKind::JboPhrase,
                    language: None,
                    inlines: vec![CllInline::Text("z".to_owned())],
                },
            ],
        };
        assert!(word.word_parts().is_none());
        let block = CllBlock::CmavoList {
            id: None,
            titles: Vec::new(),
            headers: vec![
                vec![CllInline::Text("cmavo".to_owned())],
                vec![CllInline::Text("gloss".to_owned())],
            ],
            rows: vec![vec![
                word,
                CllCmavoListCell::Other {
                    inlines: vec![CllInline::Text("never".to_owned())],
                },
            ]],
        };
        let mut web = String::new();
        render_block_markdown(site, &block, &mut web, 0, CllMarkdownDialect::GitHub);
        assert_eq!(
            web,
            "| cmavo | gloss |\n| --- | --- |\n| a\\|b<br>c<br>d<br>ez | never |\n\n"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_header_and_an_ordinary_cell_keep_each_line_ending_inside_the_row() {
        let site = crate::import::embedded_cll_site().expect("embedded CLL site");
        let block = CllBlock::CmavoList {
            id: None,
            titles: Vec::new(),
            headers: vec![
                vec![CllInline::Text("p|q\nr".to_owned())],
                vec![CllInline::Text("c\r\nd".to_owned())],
                vec![CllInline::Text("e\rf".to_owned())],
            ],
            rows: vec![vec![
                CllCmavoListCell::Other {
                    inlines: vec![CllInline::Text("g\nh".to_owned())],
                },
                CllCmavoListCell::Other {
                    inlines: vec![CllInline::Text("i|j\r\nk".to_owned())],
                },
                CllCmavoListCell::Other {
                    inlines: vec![CllInline::Text("l\rm".to_owned())],
                },
            ]],
        };
        let mut web = String::new();
        render_block_markdown(site, &block, &mut web, 0, CllMarkdownDialect::GitHub);
        assert_eq!(
            web,
            "| p\\|q<br>r | c<br>d | e<br>f |\n| --- | --- | --- |\n| g<br>h | i\\|j<br>k | l<br>m |\n\n"
        );
    }

    #[requires(true)]
    #[ensures(true)]
    fn render_dialect(block: &CllBlock, dialect: CllMarkdownDialect) -> String {
        let site = crate::import::embedded_cll_site().expect("embedded CLL site");
        let mut output = String::new();
        render_block_markdown(site, block, &mut output, 0, dialect);
        output
    }

    #[requires(true)]
    #[ensures(true)]
    fn prose_paragraph(text: &str, status_note: bool) -> CllBlock {
        CllBlock::Paragraph {
            anchor_id: None,
            role: status_note.then(|| CllParagraphRole::parse("status-note").expect("status note")),
            inlines: Vec::new(),
            text: text.to_owned(),
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_quote_prefixes_every_line_and_keeps_the_line_ending() {
        // `quoted` is the text after the first `> `, with each later line
        // already marked. A blank line keeps its `> `.
        let cases = [
            ("first\rsecond", "first\r> second"),
            ("first\r\rsecond", "first\r> \r> second"),
            ("first\r# second", "first\r> # second"),
            ("first\r\nsecond", "first\r\n> second"),
            ("first\r\n\r\nsecond", "first\r\n> \r\n> second"),
            ("first\r\n# second", "first\r\n> # second"),
        ];
        for (text, quoted) in cases {
            let note = format!("> **{CLL_STATUS_NOTE_LABEL}.** {quoted}");
            assert_eq!(status_note_markdown(text), note, "{text:?}");
            assert_eq!(
                render_dialect(&prose_paragraph(text, true), CllMarkdownDialect::GitHub),
                format!("{note}\n\n"),
                "status {text:?}"
            );
            assert_eq!(
                render_dialect(
                    &CllBlock::BlockQuote {
                        id: None,
                        blocks: vec![prose_paragraph(text, false)],
                    },
                    CllMarkdownDialect::GitHub,
                ),
                format!("> {quoted}\n\n"),
                "quote {text:?}"
            );
        }
        // Discord escapes the heading before the quote prefix. GitHub copies
        // the book text, so the same heading stays `#`.
        for (text, ending) in [("first\r# second", "\r"), ("first\r\n# second", "\r\n")] {
            let discord_quoted = format!("first{ending}> \\# second");
            assert_eq!(
                render_dialect(&prose_paragraph(text, true), CllMarkdownDialect::Discord),
                format!("> **{CLL_STATUS_NOTE_LABEL}.** {discord_quoted}\n\n"),
                "discord status {text:?}"
            );
            assert_eq!(
                render_dialect(
                    &CllBlock::BlockQuote {
                        id: None,
                        blocks: vec![prose_paragraph(text, false)],
                    },
                    CllMarkdownDialect::Discord,
                ),
                format!("> {discord_quoted}\n\n"),
                "discord quote {text:?}"
            );
        }
    }
}
