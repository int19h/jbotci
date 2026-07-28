use super::*;

#[requires(true)]
#[ensures(true)]
pub(crate) fn render_block_html(
    site: &CllSite,
    block: &CllBlock,
    link_mode: CllLinkRenderMode,
) -> String {
    match block {
        CllBlock::Paragraph {
            anchor_id,
            role,
            inlines,
            text,
        } => {
            let class = role
                .as_ref()
                .map(|role| format!(" class=\"cll-para cll-para-{role}\""))
                .unwrap_or_else(|| " class=\"cll-para\"".to_owned());
            let id = anchor_id
                .as_ref()
                .map(|id| format!(" id=\"{}\"", escape_html(id)))
                .unwrap_or_default();
            let body = if inlines.is_empty() {
                escape_html(text)
            } else {
                render_inlines_html(site, inlines, link_mode)
            };
            format!("<p{id}{class}>{body}</p>")
        }
        CllBlock::List { ordered, items } => {
            let tag = if *ordered { "ol" } else { "ul" };
            let mut output = format!("<{tag} class=\"cll-list\">");
            for item in items {
                output.push_str("<li>");
                for block in item {
                    output.push_str(&render_block_html(site, block, link_mode));
                }
                output.push_str("</li>");
            }
            output.push_str(&format!("</{tag}>"));
            output
        }
        CllBlock::Example { example_id } => cll_lookup_example(site, example_id)
            .map(|example| render_example(site, example, CllRenderFormat::Html, link_mode))
            .unwrap_or_default(),
        CllBlock::Table {
            id,
            caption,
            header_rows,
            body_rows,
            classes,
        } => {
            let mut output = format!(
                "<table{} class=\"{}\">",
                render_optional_id(id.as_deref()),
                table_classes(classes)
            );
            if let Some(caption) = caption {
                output.push_str("<caption>");
                output.push_str(&render_inlines_html(site, caption, link_mode));
                output.push_str("</caption>");
            }
            if !header_rows.is_empty() {
                output.push_str("<thead>");
                render_table_rows_html(site, "th", header_rows, &mut output, link_mode);
                output.push_str("</thead>");
            }
            output.push_str("<tbody>");
            render_table_rows_html(site, "td", body_rows, &mut output, link_mode);
            output.push_str("</tbody>");
            output.push_str("</table>");
            output
        }
        CllBlock::SimpleListTable {
            id,
            orientation,
            rows,
        } => render_simple_list_table_html(site, id.as_deref(), *orientation, rows, link_mode),
        CllBlock::VariableList { id, entries } => {
            let mut output = format!(
                "<dl{} class=\"cll-variable-list\">",
                render_optional_id(id.as_deref())
            );
            for entry in entries {
                output.push_str("<dt>");
                output.push_str(&render_inlines_html(site, &entry.term, link_mode));
                output.push_str("</dt><dd>");
                for block in &entry.blocks {
                    output.push_str(&render_block_html(site, block, link_mode));
                }
                output.push_str("</dd>");
            }
            output.push_str("</dl>");
            output
        }
        CllBlock::Media {
            id,
            title,
            src,
            alt,
        } => {
            let mut output = format!(
                "<figure{} class=\"cll-media\">",
                render_optional_id(id.as_deref())
            );
            match link_mode {
                CllLinkRenderMode::Web => output.push_str(&format!(
                    "<img src=\"{}\" alt=\"{}\" />",
                    escape_html(src),
                    escape_html(alt)
                )),
                CllLinkRenderMode::Plain if !alt.is_empty() => {
                    output.push_str("<p class=\"cll-media-alt\">");
                    output.push_str(&escape_html(alt));
                    output.push_str("</p>");
                }
                CllLinkRenderMode::Plain => {}
            }
            if let Some(title) = title {
                output.push_str("<figcaption>");
                output.push_str(&render_inlines_html(site, title, link_mode));
                output.push_str("</figcaption>");
            }
            output.push_str("</figure>");
            output
        }
        CllBlock::Rule { id, term, body } => {
            let mut output = format!(
                "<div{} class=\"cll-rule\"><dt>{}</dt><dd>",
                render_optional_id(id.as_deref()),
                escape_html(term)
            );
            for block in body {
                output.push_str(&render_block_html(site, block, link_mode));
            }
            output.push_str("</dd></div>");
            output
        }
        CllBlock::Code { text, .. } => {
            format!(
                "<pre class=\"cll-code\"><code>{}</code></pre>",
                escape_html(text)
            )
        }
        CllBlock::DisplayMath { id, markup, .. } => format!(
            "<div{} class=\"cll-math-block\">{}</div>",
            render_optional_id(id.as_deref()),
            markup
        ),
        CllBlock::Heading {
            id, level, inlines, ..
        } => {
            let level = (*level).clamp(2, 6);
            format!(
                "<h{level}{}>{}</h{level}>",
                render_optional_id(id.as_deref()),
                render_inlines_html(site, inlines, link_mode)
            )
        }
        CllBlock::BlockQuote { id, blocks } => {
            let mut output = format!(
                "<blockquote{} class=\"cll-blockquote\">",
                render_optional_id(id.as_deref())
            );
            for block in blocks {
                output.push_str(&render_block_html(site, block, link_mode));
            }
            output.push_str("</blockquote>");
            output
        }
        CllBlock::Definition { id, body } => format!(
            "<p{} class=\"cll-definition\">{}</p>",
            render_optional_id(id.as_deref()),
            render_inlines_html(site, body, link_mode)
        ),
        CllBlock::InterlinearGloss {
            id,
            aligned,
            itemized,
            parse_href,
            rows,
            natlang,
            comments,
        } => render_interlinear_html(
            site,
            id.as_deref(),
            *aligned,
            *itemized,
            parse_href.as_deref(),
            rows,
            natlang,
            comments,
            link_mode,
        ),
        CllBlock::CmavoList {
            id,
            titles,
            headers,
            rows,
        } => render_cmavo_list_html(site, id.as_deref(), titles, headers, rows, link_mode),
        CllBlock::Lojbanization { id, lines } => {
            render_lojbanization_html(site, id.as_deref(), lines, link_mode)
        }
        CllBlock::LujvoMaking { id, parts } => {
            render_lujvo_making_html(site, id.as_deref(), parts, link_mode)
        }
        CllBlock::GrammarTemplate { id, body } => format!(
            "<p{} class=\"cll-grammar-template\">{}</p>",
            render_optional_id(id.as_deref()),
            render_inlines_html(site, body, link_mode)
        ),
        CllBlock::Ebnf { id, entries } => render_ebnf_html(site, id.as_deref(), entries, link_mode),
    }
}

#[requires(true)]
#[ensures(true)]
fn render_inlines_html(
    site: &CllSite,
    inlines: &[CllInline],
    link_mode: CllLinkRenderMode,
) -> String {
    let mut output = String::new();
    for inline in inlines {
        match inline {
            CllInline::Text(text) => output.push_str(&escape_html(text)),
            CllInline::Emphasis { language, inlines } => {
                output.push_str("<em");
                output.push_str(&render_optional_lang(language.as_deref()));
                output.push('>');
                output.push_str(&render_inlines_html(site, inlines, link_mode));
                output.push_str("</em>");
            }
            CllInline::Quote { language, inlines } => {
                output.push_str("<q");
                output.push_str(&render_optional_lang(language.as_deref()));
                output.push('>');
                output.push_str(&render_inlines_html(site, inlines, link_mode));
                output.push_str("</q>");
            }
            CllInline::LanguageSpan {
                kind,
                language,
                inlines,
            } => {
                output.push_str("<span class=\"");
                output.push_str(language_span_class(*kind));
                output.push('"');
                output.push_str(&render_optional_lang(language.as_deref()));
                output.push('>');
                output.push_str(&render_inlines_html(site, inlines, link_mode));
                output.push_str("</span>");
            }
            CllInline::CiteTitle { inlines } => {
                output.push_str("<cite>");
                output.push_str(&render_inlines_html(site, inlines, link_mode));
                output.push_str("</cite>");
            }
            CllInline::Subscript { inlines } => {
                output.push_str("<sub>");
                output.push_str(&render_inlines_html(site, inlines, link_mode));
                output.push_str("</sub>");
            }
            CllInline::Superscript { inlines } => {
                output.push_str("<sup>");
                output.push_str(&render_inlines_html(site, inlines, link_mode));
                output.push_str("</sup>");
            }
            CllInline::Link {
                target,
                inlines,
                kind,
            } => match link_mode {
                CllLinkRenderMode::Web => {
                    output.push_str("<a href=\"");
                    output.push_str(&escape_html(&cll_link_href(site, *kind, target)));
                    output.push_str("\" class=\"spa-cll-link ");
                    output.push_str(link_kind_class(*kind));
                    output.push_str("\">");
                    output.push_str(&render_inlines_html(site, inlines, link_mode));
                    output.push_str("</a>");
                }
                CllLinkRenderMode::Plain => match kind.plain_disposition() {
                    CllPlainLinkDisposition::KeepContent => {
                        output.push_str(&render_inlines_html(site, inlines, link_mode));
                    }
                    CllPlainLinkDisposition::Drop => {}
                },
            },
            CllInline::Code(text) => {
                output.push_str("<code>");
                output.push_str(&escape_html(text));
                output.push_str("</code>");
            }
            CllInline::Elidable {
                shown,
                forced,
                inlines,
            } => {
                let class = if *forced {
                    "cll-elidable cll-elidable-forced"
                } else {
                    "cll-elidable"
                };
                output.push_str("<span class=\"");
                output.push_str(class);
                output.push_str("\">");
                if inlines.is_empty() {
                    output.push_str(&escape_html(shown));
                } else {
                    output.push_str(&render_inlines_html(site, inlines, link_mode));
                }
                output.push_str("</span>");
            }
            CllInline::InlineMath { markup, .. } => {
                output.push_str("<span class=\"cll-inline-math\">");
                output.push_str(markup);
                output.push_str("</span>");
            }
            CllInline::Anchor { id } => {
                output.push_str("<span id=\"");
                output.push_str(&escape_html(id));
                output.push_str("\"></span>");
            }
        }
    }
    output
}

#[requires(true)]
#[ensures(true)]
fn render_optional_id(id: Option<&str>) -> String {
    id.map(|value| format!(" id=\"{}\"", escape_html(value)))
        .unwrap_or_default()
}

#[requires(true)]
#[ensures(true)]
fn render_optional_lang(language: Option<&str>) -> String {
    language
        .map(|value| format!(" lang=\"{}\"", escape_html(value)))
        .unwrap_or_default()
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn table_classes(classes: &[String]) -> String {
    let mut output = String::from("cll-table");
    for class in classes {
        output.push(' ');
        output.push_str("cll-table-");
        output.push_str(class);
    }
    output
}

#[requires(!tag_name.is_empty())]
#[ensures(true)]
fn render_table_rows_html(
    site: &CllSite,
    tag_name: &str,
    rows: &[Vec<CllTableCell>],
    output: &mut String,
    link_mode: CllLinkRenderMode,
) {
    for row in rows {
        output.push_str("<tr");
        output.push_str(&render_table_row_parse_attrs(row));
        output.push('>');
        for cell in row {
            output.push('<');
            output.push_str(tag_name);
            if let Some(col_span) = cell.col_span {
                output.push_str(&format!(" colspan=\"{col_span}\""));
            }
            if let Some(row_span) = cell.row_span {
                output.push_str(&format!(" rowspan=\"{row_span}\""));
            }
            output.push('>');
            if link_mode == CllLinkRenderMode::Web
                && let Some(parse_href) = &cell.parse_href
            {
                output.push_str("<a class=\"");
                output.push_str(&escape_html(&table_cell_parse_link_class(cell)));
                output.push_str("\" href=\"");
                output.push_str(&escape_html(parse_href));
                output.push_str("\">Parse</a>");
            }
            for block in &cell.blocks {
                output.push_str(&render_block_html(site, block, link_mode));
            }
            output.push_str("</");
            output.push_str(tag_name);
            output.push('>');
        }
        output.push_str("</tr>");
    }
}

#[requires(true)]
#[ensures(true)]
fn render_table_row_parse_attrs(row: &[CllTableCell]) -> String {
    let Some(group) = table_row_parse_group(row) else {
        return String::new();
    };
    let mut classes = vec!["cll-parse-group-row"];
    if group.row_count > 1 {
        classes.push("cll-parse-group-multi");
    }
    if group.row_index == 0 {
        classes.push("cll-parse-group-start");
    }
    if group.row_index + 1 == group.row_count {
        classes.push("cll-parse-group-end");
    }
    if group.row_index > 0 {
        classes.push("cll-parse-group-continuation");
    }
    format!(
        " class=\"{}\" data-cll-parse-group=\"{}\"",
        classes.join(" "),
        escape_html(&group.group_id)
    )
}

#[requires(true)]
#[ensures(true)]
fn table_row_parse_group(row: &[CllTableCell]) -> Option<&CllTableParseGroup> {
    row.first().and_then(|cell| cell.parse_group.as_ref())
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn table_cell_parse_link_class(cell: &CllTableCell) -> String {
    let mut class_name =
        "cll-parse-example cll-parse-snippet spa-cll-link spa-cll-link-parse".to_owned();
    if cell
        .parse_group
        .as_ref()
        .is_some_and(|group| group.row_count > 1)
    {
        class_name.push_str(" cll-parse-group-link");
    }
    class_name
}

#[requires(true)]
#[ensures(true)]
fn render_simple_list_table_html(
    site: &CllSite,
    id: Option<&str>,
    orientation: CllSimpleListOrientation,
    rows: &[Vec<Option<Vec<CllInline>>>],
    link_mode: CllLinkRenderMode,
) -> String {
    let orientation_class = match orientation {
        CllSimpleListOrientation::Horizontal => "horizontal",
        CllSimpleListOrientation::Vertical => "vertical",
    };
    let mut output = format!(
        "<table{} class=\"cll-simplelist cll-simplelist-{orientation_class}\"><tbody>",
        render_optional_id(id)
    );
    for row in rows {
        output.push_str("<tr>");
        for cell in row {
            output.push_str("<td>");
            if let Some(inlines) = cell {
                output.push_str(&render_inlines_html(site, inlines, link_mode));
            }
            output.push_str("</td>");
        }
        output.push_str("</tr>");
    }
    output.push_str("</tbody></table>");
    output
}

#[requires(true)]
#[ensures(true)]
fn render_interlinear_html(
    site: &CllSite,
    id: Option<&str>,
    aligned: bool,
    itemized: bool,
    parse_href: Option<&str>,
    rows: &[CllInterlinearRow],
    natlang: &[Vec<CllInline>],
    comments: &[Vec<CllInline>],
    link_mode: CllLinkRenderMode,
) -> String {
    let mut output = format!(
        "<div{} class=\"cll-interlinear{}\">",
        render_optional_id(id),
        if aligned || itemized {
            " cll-interlinear-aligned"
        } else {
            ""
        }
    );
    if link_mode == CllLinkRenderMode::Web
        && let Some(parse_href) = parse_href
    {
        output.push_str("<a class=\"cll-parse-example spa-cll-link spa-cll-link-parse\" href=\"");
        output.push_str(&escape_html(parse_href));
        output.push_str("\">Parse</a>");
    }
    if !rows.is_empty() {
        if aligned {
            output.push_str("<table class=\"cll-interlinear-table");
            if !itemized {
                output.push_str(" cll-interlinear-table-plain");
            }
            output.push_str("\"><tbody>");
            for row in rows {
                output.push_str("<tr class=\"cll-interlinear-row cll-interlinear-row-");
                output.push_str(&escape_html(row.kind.as_str()));
                output.push_str("\">");
                for cell in &row.cells {
                    output.push_str("<td>");
                    output.push_str(&render_inlines_html(site, cell, link_mode));
                    output.push_str("</td>");
                }
                output.push_str("</tr>");
            }
            output.push_str("</tbody></table>");
        } else {
            output.push_str("<div class=\"cll-interlinear-itemized\">");
            for row in rows {
                output.push_str(
                    "<div class=\"cll-ig-line-wrap\"><p class=\"cll-ig-line cll-ig-inline cll-ig-",
                );
                output.push_str(&escape_html(row.kind.as_str()));
                output.push_str("\">");
                for cell in &row.cells {
                    output.push_str(&render_inlines_html(site, cell, link_mode));
                }
                output.push_str("</p></div>");
            }
            output.push_str("</div>");
        }
    }
    for line in comments {
        output.push_str("<p class=\"cll-interlinear-comment\">");
        output.push_str(&render_inlines_html(site, line, link_mode));
        output.push_str("</p>");
    }
    for line in natlang {
        output.push_str("<p class=\"cll-natlang\">");
        output.push_str(&render_inlines_html(site, line, link_mode));
        output.push_str("</p>");
    }
    output.push_str("</div>");
    output
}

#[requires(true)]
#[ensures(true)]
fn render_cmavo_list_html(
    site: &CllSite,
    id: Option<&str>,
    titles: &[Vec<CllInline>],
    headers: &[Vec<CllInline>],
    rows: &[Vec<Vec<CllInline>>],
    link_mode: CllLinkRenderMode,
) -> String {
    let mut output = format!("<div{} class=\"cll-cmavo-list\">", render_optional_id(id));
    for title in titles {
        output.push_str("<p class=\"cll-cmavo-list-title\">");
        output.push_str(&render_inlines_html(site, title, link_mode));
        output.push_str("</p>");
    }
    output.push_str("<table><tbody>");
    if !headers.is_empty() {
        output.push_str("<tr>");
        for header in headers {
            output.push_str("<th>");
            output.push_str(&render_inlines_html(site, header, link_mode));
            output.push_str("</th>");
        }
        output.push_str("</tr>");
    }
    for row in rows {
        output.push_str("<tr>");
        for cell in row {
            output.push_str("<td>");
            output.push_str(&render_inlines_html(site, cell, link_mode));
            output.push_str("</td>");
        }
        output.push_str("</tr>");
    }
    output.push_str("</tbody></table></div>");
    output
}

#[requires(true)]
#[ensures(true)]
fn render_lojbanization_html(
    site: &CllSite,
    id: Option<&str>,
    lines: &[CllLojbanizationLine],
    link_mode: CllLinkRenderMode,
) -> String {
    let mut output = format!(
        "<table{} class=\"cll-lojbanization\"><tbody>",
        render_optional_id(id)
    );
    for line in lines {
        output.push_str("<tr class=\"cll-lojbanization-line cll-lojbanization-line-");
        output.push_str(&escape_html(line.kind.as_str()));
        output.push_str("\"><th>");
        output.push_str(&escape_html(line.kind.as_str()));
        output.push_str("</th><td>");
        output.push_str(&render_inlines_html(site, &line.body, link_mode));
        output.push_str("</td><td>");
        if let Some(comment) = &line.comment {
            output.push_str(&render_inlines_html(site, comment, link_mode));
        }
        output.push_str("</td></tr>");
    }
    output.push_str("</tbody></table>");
    output
}

#[requires(true)]
#[ensures(true)]
fn render_lujvo_making_html(
    site: &CllSite,
    id: Option<&str>,
    parts: &[CllLujvoPart],
    link_mode: CllLinkRenderMode,
) -> String {
    let mut output = format!("<ul{} class=\"cll-lujvo-making\">", render_optional_id(id));
    for part in parts {
        output.push_str("<li class=\"cll-lujvo-part cll-lujvo-part-");
        output.push_str(&escape_html(part.kind.as_str()));
        output.push_str("\"><span class=\"cll-lujvo-part-kind\">");
        output.push_str(&escape_html(part.kind.as_str()));
        output.push_str("</span> ");
        output.push_str(&render_inlines_html(site, &part.body, link_mode));
        output.push_str("</li>");
    }
    output.push_str("</ul>");
    output
}

#[requires(true)]
#[ensures(true)]
fn render_ebnf_html(
    site: &CllSite,
    id: Option<&str>,
    entries: &[CllEbnfEntry],
    link_mode: CllLinkRenderMode,
) -> String {
    let mut output = format!("<div{} class=\"cll-ebnf\">", render_optional_id(id));
    for entry in entries {
        output.push_str("<section class=\"cll-ebnf-entry\" id=\"");
        output.push_str(&escape_html(&entry.anchor_id));
        output.push_str("\"><div class=\"cll-ebnf-head\">");
        render_ebnf_link_html(
            site,
            "cll-ebnf-rule",
            &entry.rule_name,
            &entry.rule_href,
            &mut output,
            link_mode,
        );
        output.push_str(" <span class=\"cll-ebnf-assign\">⩴</span></div>");
        output.push_str("<pre class=\"cll-ebnf-rhs\">");
        output.push_str(&render_ebnf_tokens_html(site, &entry.rhs, link_mode));
        output.push_str("</pre></section>");
    }
    output.push_str("</div>");
    output
}

#[requires(true)]
#[ensures(true)]
fn render_ebnf_tokens_html(
    site: &CllSite,
    tokens: &[CllEbnfToken],
    link_mode: CllLinkRenderMode,
) -> String {
    let lines = wrap_ebnf_choice_lines(tokens);
    if lines.len() == 1 {
        return render_ebnf_token_line_html(site, &lines[0], link_mode);
    }
    let mut output = String::new();
    for line in lines {
        output.push_str("<span class=\"cll-ebnf-choice-line\">");
        output.push_str(&render_ebnf_token_line_html(site, &line, link_mode));
        output.push_str("</span>");
    }
    output
}

#[requires(true)]
#[ensures(true)]
fn render_ebnf_token_line_html(
    site: &CllSite,
    tokens: &[CllEbnfToken],
    link_mode: CllLinkRenderMode,
) -> String {
    let mut output = String::new();
    for token in tokens {
        match token {
            CllEbnfToken::Text { body } => {
                output.push_str(&escape_html(body));
            }
            CllEbnfToken::Operator { body } => {
                output.push_str("<span class=\"cll-ebnf-op\">");
                output.push_str(&escape_html(body));
                output.push_str("</span>");
            }
            CllEbnfToken::Hash { body } => {
                output.push_str("<span class=\"cll-ebnf-hash\">");
                output.push_str(&escape_html(body));
                output.push_str("</span>");
            }
            CllEbnfToken::Terminal { body, href } => {
                render_ebnf_link_html(
                    site,
                    "cll-ebnf-terminal",
                    body,
                    href,
                    &mut output,
                    link_mode,
                );
            }
            CllEbnfToken::ElidableTerminator { body, href } => {
                render_ebnf_elidable_html(site, body, href, &mut output, link_mode);
            }
            CllEbnfToken::Nonterminal { body, href } => {
                render_ebnf_link_html(
                    site,
                    "cll-ebnf-nonterminal",
                    body,
                    href,
                    &mut output,
                    link_mode,
                );
            }
        }
    }
    output
}

#[requires(true)]
#[ensures(true)]
fn render_ebnf_elidable_html(
    site: &CllSite,
    body: &str,
    href: &Option<String>,
    output: &mut String,
    link_mode: CllLinkRenderMode,
) {
    let body_html = if let Some((prefix, suffix)) = cll_ebnf_elidable_hash_pieces(body) {
        format!(
            "{}<span class=\"cll-ebnf-hash\">#</span>{}",
            escape_html(&prefix),
            escape_html(&suffix)
        )
    } else {
        escape_html(body)
    };
    render_ebnf_link_body_html(
        site,
        "cll-ebnf-elidable",
        &body_html,
        href,
        output,
        link_mode,
    );
}

#[requires(true)]
#[ensures(true)]
fn cll_ebnf_elidable_hash_pieces(body: &str) -> Option<(String, String)> {
    let inner = body.strip_prefix('/')?.strip_suffix('/')?;
    let inner_without_hash = inner.strip_suffix('#')?;
    Some((format!("/{inner_without_hash}"), "/".to_owned()))
}

#[requires(!class_name.is_empty())]
#[ensures(true)]
fn render_ebnf_link_html(
    site: &CllSite,
    class_name: &str,
    body: &str,
    href: &Option<String>,
    output: &mut String,
    link_mode: CllLinkRenderMode,
) {
    render_ebnf_link_body_html(
        site,
        class_name,
        &escape_html(body),
        href,
        output,
        link_mode,
    );
}

#[requires(!class_name.is_empty())]
#[ensures(true)]
fn render_ebnf_link_body_html(
    site: &CllSite,
    class_name: &str,
    body_html: &str,
    href: &Option<String>,
    output: &mut String,
    link_mode: CllLinkRenderMode,
) {
    if link_mode == CllLinkRenderMode::Web
        && let Some(href) = href
    {
        output.push_str("<a class=\"");
        output.push_str(class_name);
        output.push_str("\" href=\"");
        output.push_str(&escape_html(&render_ebnf_href(site, href)));
        output.push_str("\">");
        output.push_str(body_html);
        output.push_str("</a>");
    } else {
        output.push_str("<span class=\"");
        output.push_str(class_name);
        output.push_str("\">");
        output.push_str(body_html);
        output.push_str("</span>");
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn language_span_class(kind: CllLanguageSpanKind) -> &'static str {
    match kind {
        CllLanguageSpanKind::ForeignPhrase => "spa-cll-foreignphrase",
        CllLanguageSpanKind::JboPhrase => "spa-cll-jbophrase",
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn link_kind_class(kind: CllLinkKind) -> &'static str {
    match kind {
        CllLinkKind::Section => "spa-cll-link-section",
        CllLinkKind::Example => "spa-cll-link-example",
        CllLinkKind::Dictionary => "spa-cll-link-dictionary",
        CllLinkKind::Rafsi => "spa-cll-link-rafsi",
        CllLinkKind::Parse => "spa-cll-link-parse",
        CllLinkKind::Asset => "spa-cll-link-asset",
        CllLinkKind::External => "spa-cll-link-external",
    }
}
