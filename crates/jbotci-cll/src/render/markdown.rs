use super::*;

#[requires(true)]
#[ensures(true)]
pub(crate) fn render_block_markdown(
    site: &CllSite,
    block: &CllBlock,
    output: &mut String,
    depth: usize,
    link_mode: CllLinkRenderMode,
) {
    match block {
        CllBlock::Paragraph {
            role,
            inlines,
            text,
            ..
        } => {
            let body: std::borrow::Cow<'_, str> = if inlines.is_empty() {
                std::borrow::Cow::Borrowed(text.as_str())
            } else {
                std::borrow::Cow::Owned(render_inlines_markdown(site, inlines, link_mode))
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
                    render_block_markdown(site, block, &mut item_text, depth + 1, link_mode);
                }
                output.push_str(item_text.trim());
                output.push('\n');
            }
            output.push('\n');
        }
        CllBlock::Example { example_id } => {
            if let Some(example) = cll_lookup_example(site, example_id) {
                output.push_str(&render_example(
                    site,
                    example,
                    CllRenderFormat::Markdown,
                    link_mode,
                ));
            }
        }
        CllBlock::Table {
            caption,
            header_rows,
            body_rows,
            ..
        } => {
            render_table_markdown(
                site,
                caption.as_deref(),
                header_rows,
                body_rows,
                output,
                link_mode,
            );
        }
        CllBlock::SimpleListTable { rows, .. } => {
            render_simple_list_table_markdown(site, rows, output, link_mode);
        }
        CllBlock::VariableList { entries, .. } => {
            for entry in entries {
                output.push_str("**");
                output.push_str(&render_inlines_markdown(site, &entry.term, link_mode));
                output.push_str("**\n\n");
                for block in &entry.blocks {
                    render_block_markdown(site, block, output, depth, link_mode);
                }
            }
        }
        CllBlock::Media {
            title, src, alt, ..
        } => {
            if link_mode == CllLinkRenderMode::Web {
                output.push_str(&format!("![{}]({})\n\n", alt, src));
            } else {
                output.push_str(alt);
                output.push_str("\n\n");
            }
            if let Some(title) = title {
                output.push_str(&render_inlines_markdown(site, title, link_mode));
                output.push_str("\n\n");
            }
        }
        CllBlock::Rule { term, body, .. } => {
            output.push_str(&format!("**{term}**\n\n"));
            for block in body {
                render_block_markdown(site, block, output, depth, link_mode);
            }
        }
        CllBlock::Code { text, .. } => {
            output.push_str("```\n");
            output.push_str(text);
            output.push_str("\n```\n\n");
        }
        CllBlock::DisplayMath { latex, .. } => {
            output.push_str("$$\n");
            output.push_str(latex);
            output.push_str("\n$$\n\n");
        }
        CllBlock::Heading { level, inlines, .. } => {
            output.push_str(&"#".repeat(usize::from(*level)));
            output.push(' ');
            output.push_str(&render_inlines_markdown(site, inlines, link_mode));
            output.push_str("\n\n");
        }
        CllBlock::BlockQuote { blocks, .. } => {
            let mut inner = String::new();
            for block in blocks {
                render_block_markdown(site, block, &mut inner, depth, link_mode);
            }
            for line in inner.trim().lines() {
                output.push_str("> ");
                output.push_str(line);
                output.push('\n');
            }
            output.push('\n');
        }
        CllBlock::Definition { body, .. } | CllBlock::GrammarTemplate { body, .. } => {
            output.push_str(&render_inlines_markdown(site, body, link_mode));
            output.push_str("\n\n");
        }
        CllBlock::InterlinearGloss {
            aligned,
            parse_href,
            rows,
            natlang,
            comments,
            ..
        } => render_interlinear_markdown(
            site,
            *aligned,
            parse_href.as_deref(),
            rows,
            natlang,
            comments,
            output,
            link_mode,
        ),
        CllBlock::CmavoList {
            titles,
            headers,
            rows,
            ..
        } => render_cmavo_list_markdown(site, titles, headers, rows, output, link_mode),
        CllBlock::Lojbanization { lines, .. } => {
            render_lojbanization_markdown(site, lines, output, link_mode);
        }
        CllBlock::LujvoMaking { parts, .. } => {
            for part in parts {
                output.push_str("- **");
                output.push_str(part.kind.as_str());
                output.push_str("**: ");
                output.push_str(&render_inlines_markdown(site, &part.body, link_mode));
                output.push('\n');
            }
            output.push('\n');
        }
        CllBlock::Ebnf { entries, .. } => {
            render_ebnf_markdown(site, entries, output, link_mode);
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_inlines_markdown(
    site: &CllSite,
    inlines: &[CllInline],
    link_mode: CllLinkRenderMode,
) -> String {
    let mut output = String::new();
    for inline in inlines {
        match inline {
            CllInline::Text(text) => output.push_str(text),
            CllInline::Emphasis { inlines, .. } => {
                output.push('*');
                output.push_str(&render_inlines_markdown(site, inlines, link_mode));
                output.push('*');
            }
            CllInline::Quote { inlines, .. } => {
                output.push('"');
                output.push_str(&render_inlines_markdown(site, inlines, link_mode));
                output.push('"');
            }
            CllInline::LanguageSpan { inlines, .. } | CllInline::CiteTitle { inlines } => {
                output.push_str(&render_inlines_markdown(site, inlines, link_mode));
            }
            CllInline::Subscript { inlines } => {
                output.push('~');
                output.push_str(&render_inlines_markdown(site, inlines, link_mode));
                output.push('~');
            }
            CllInline::Superscript { inlines } => {
                output.push('^');
                output.push_str(&render_inlines_markdown(site, inlines, link_mode));
                output.push('^');
            }
            CllInline::Link {
                target,
                inlines,
                kind,
            } => {
                let text = render_inlines_markdown(site, inlines, link_mode);
                let text = if text.is_empty() {
                    target.as_str()
                } else {
                    &text
                };
                match link_mode {
                    CllLinkRenderMode::Web => output.push_str(&format!(
                        "[{}]({})",
                        markdown_link_label_text(text),
                        cll_link_href(site, *kind, target)
                    )),
                    CllLinkRenderMode::Plain => match kind.plain_disposition() {
                        CllPlainLinkDisposition::KeepContent => output.push_str(text),
                        CllPlainLinkDisposition::Drop => {}
                    },
                }
            }
            CllInline::Code(text) => output.push_str(&format!("`{text}`")),
            CllInline::Elidable { shown, inlines, .. } => {
                let text = render_inlines_markdown(site, inlines, link_mode);
                output.push('[');
                if text.is_empty() {
                    output.push_str(shown);
                } else {
                    output.push_str(&text);
                }
                output.push(']');
            }
            CllInline::InlineMath { latex, .. } => {
                output.push('$');
                output.push_str(latex);
                output.push('$');
            }
            CllInline::Anchor { .. } => {}
        }
    }
    output
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
        output.push_str(&render_inlines_markdown(site, caption, link_mode));
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
                                site, inlines, link_mode,
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
                .map(|cell| render_inlines_markdown(site, cell, link_mode))
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
            output.push_str(&render_inlines_markdown(site, line, link_mode));
            output.push('\n');
        }
        for line in natlang {
            output.push_str("natlang: ");
            output.push_str(&render_inlines_markdown(site, line, link_mode));
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
                    markdown_table_cell_text(&render_inlines_markdown(site, cell, link_mode))
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    render_markdown_table_rows(table_rows, output);
    for line in comments {
        output.push_str("_");
        output.push_str(&render_inlines_markdown(site, line, link_mode));
        output.push_str("_\n\n");
    }
    for line in natlang {
        output.push_str("> ");
        output.push_str(&render_inlines_markdown(site, line, link_mode));
        output.push_str("\n\n");
    }
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
    for title in titles {
        output.push_str("**");
        output.push_str(&render_inlines_markdown(site, title, link_mode));
        output.push_str("**\n\n");
    }
    if headers.is_empty() {
        for row in rows {
            let rendered_cells = row
                .iter()
                .map(|cell| {
                    render_inlines_markdown(site, cell, link_mode)
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
        .map(|cell| markdown_table_cell_text(&render_inlines_markdown(site, cell, link_mode)))
        .collect::<Vec<_>>();
    let rendered_rows = rows.iter().map(|row| {
        row.iter()
            .map(|cell| markdown_table_cell_text(&render_inlines_markdown(site, cell, link_mode)))
            .collect::<Vec<_>>()
    });
    render_markdown_table_rows(std::iter::once(header).chain(rendered_rows), output);
}

#[requires(true)]
#[ensures(true)]
fn render_lojbanization_markdown(
    site: &CllSite,
    lines: &[CllLojbanizationLine],
    output: &mut String,
    link_mode: CllLinkRenderMode,
) {
    let rows = lines.iter().map(|line| {
        vec![
            line.kind.as_str().to_owned(),
            markdown_table_cell_text(&render_inlines_markdown(site, &line.body, link_mode)),
            line.comment
                .as_deref()
                .map(|comment| {
                    markdown_table_cell_text(&render_inlines_markdown(site, comment, link_mode))
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
