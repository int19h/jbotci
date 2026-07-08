//! The Complete Lojban Language reference model.

#[allow(unused_imports)]
use bityzba::{contract_trait, data, ensures, expensive_invariant, invariant, new, requires};
#[cfg(test)]
use roxmltree::Document;

pub const DEFAULT_CUKTA_CLI_RESULT_COUNT: usize = 10;
pub const DEFAULT_CUKTA_WEB_RESULT_COUNT: usize = 20;
pub const MAX_CUKTA_RESULT_COUNT: usize = 500;
pub const DEFAULT_CUKTA_SECTION_ID: &str = "section-what-is-lojban";
const PARAGRAPH_SEARCH_MIN_CHARS: usize = 200;

mod import;
#[cfg(test)]
use import::{
    BlockParseState, chrestomathy_area_no_parse_rows, normalize_valsis_query, parse_block,
    parse_paragraph_blocks,
};
pub(crate) use import::{
    PendingIndexEntry, SectionParseContext, block_anchor_id_for, child_element, raw_text,
    visible_text, visible_text_raw, xml_id,
};
use import::{
    chrestomathy_area_groups, chrestomathy_area_label, chrestomathy_group_id,
    chrestomathy_metadata, chrestomathy_section_metadata, cll_import_metadata,
    normalized_plain_text,
};
pub use import::{embedded_cll_site, load_embedded_cll_site};

mod ebnf;
#[cfg(test)]
use ebnf::ebnf_symbol_href;
use ebnf::parse_ebnf_block;
pub use ebnf::{CllEbnfEntry, CllEbnfToken, ebnf_rule_anchor_id, wrap_ebnf_choice_lines};

mod model;
pub use model::*;

mod math;
use math::{CllMathDisplay, render_math_node};

mod links;
use links::{
    AnchorMode, build_example_reference_index, build_index_entries, build_section_reference_index,
    collect_jbo_snippet, collect_title_anchors, first_anchor_id, jbo_parse_href,
    resolve_site_links, top_level_jbo_parse_href,
};
pub use links::{
    cll_link_href, cll_resolve_example_reference, cll_resolve_section_reference,
    cll_search_chunk_href, section_href,
};

mod search;
#[cfg(test)]
use search::block_tagged_words;
pub use search::{
    CllSearchChunk, CllSearchChunkKind, CllSearchMatch, CuktaRequest, CuktaSearchMode,
    CuktaSearchOutput, CuktaTargetFilter, clamp_cukta_result_count, cll_search_all_chunks,
    cll_search_section_chunks, collect_tagged_words, cukta_search, cukta_word_search_matches,
    parse_word_search_terms, truncate_preview,
};
use search::{build_search_chunks, example_plain_text, search_chunk_kind_label};

mod render;
use render::{render_block_html, render_block_markdown};

mod visitor;
use visitor::{CllBlockVisitor, walk_block};

#[requires(true)]
#[ensures(true)]
pub fn cll_chapters(site: &CllSite) -> &[CllChapter] {
    &site.chapters
}

#[requires(true)]
#[ensures(true)]
pub fn cll_index_entries(site: &CllSite) -> &[CllIndexEntry] {
    &site.index_entries
}

#[requires(true)]
#[ensures(true)]
pub fn cll_lookup_section<'a>(site: &'a CllSite, section_id: &str) -> Option<&'a CllSection> {
    site.sections_by_id.get(section_id)
}

#[requires(true)]
#[ensures(ret.iter().all(|section| !section.text.trim().is_empty()))]
pub fn chrestomathy_section_texts(site: &CllSite) -> Vec<CllChrestomathySectionText> {
    chrestomathy_metadata()
        .section
        .iter()
        .filter_map(|metadata| {
            let section = cll_lookup_section(site, &metadata.id)?;
            let text = chrestomathy_section_group_texts(site, section)
                .into_iter()
                .map(|group| group.into_data().text)
                .collect::<Vec<_>>()
                .join("\n");
            (!text.trim().is_empty()).then(|| {
                new!(CllChrestomathySectionText {
                    section_id: section.section_id.clone(),
                    section_number: section.number.clone(),
                    section_title: section.title.clone(),
                    source_path: section.source_path.clone(),
                    text,
                })
            })
        })
        .collect()
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|href| href.starts_with("../gentufa?text=")))]
pub fn chrestomathy_section_parse_href(site: &CllSite, section: &CllSection) -> Option<String> {
    if section.chapter_id != cll_import_metadata().chrestomathy_chapter_id
        || chrestomathy_section_metadata(&section.section_id).is_none()
    {
        return None;
    }
    let text = chrestomathy_section_group_texts(site, section)
        .into_iter()
        .map(|group| group.into_data().text)
        .collect::<Vec<_>>()
        .join("\n");
    jbo_parse_href(&text)
}

#[requires(true)]
#[ensures(true)]
fn chrestomathy_section_group_texts(
    site: &CllSite,
    section: &CllSection,
) -> Vec<CllChrestomathyGroupText> {
    let mut groups = Vec::new();
    if section.chapter_id != cll_import_metadata().chrestomathy_chapter_id
        || chrestomathy_section_metadata(&section.section_id).is_none()
    {
        return groups;
    }
    for block in &section.blocks {
        chrestomathy_block_group_texts(site, &section.section_id, block, &mut groups);
    }
    groups
}

#[requires(!section_id.is_empty())]
#[ensures(true)]
fn chrestomathy_block_group_texts(
    site: &CllSite,
    section_id: &str,
    block: &CllBlock,
    groups: &mut Vec<CllChrestomathyGroupText>,
) {
    let mut visitor = ChrestomathyGroupVisitor {
        site,
        section_id,
        groups,
    };
    visitor.visit_block(block);
}

#[invariant(true)]
struct ChrestomathyGroupVisitor<'site, 'section, 'groups> {
    site: &'site CllSite,
    section_id: &'section str,
    groups: &'groups mut Vec<CllChrestomathyGroupText>,
}

#[contract_trait]
impl CllBlockVisitor for ChrestomathyGroupVisitor<'_, '_, '_> {
    #[requires(true)]
    #[ensures(true)]
    fn visit_block(&mut self, block: &CllBlock) {
        match block {
            CllBlock::Table {
                header_rows,
                body_rows,
                ..
            } => {
                chrestomathy_table_group_texts(
                    self.site,
                    self.section_id,
                    CllTableRowArea::Header,
                    header_rows,
                    self.groups,
                );
                chrestomathy_table_group_texts(
                    self.site,
                    self.section_id,
                    CllTableRowArea::Body,
                    body_rows,
                    self.groups,
                );
            }
            CllBlock::Example { example_id } => {
                if let Some(example) = cll_lookup_example(self.site, example_id) {
                    self.visit_blocks(&example.blocks);
                }
            }
            _ => walk_block(self, block),
        }
    }
}

#[requires(!section_id.is_empty())]
#[ensures(true)]
fn chrestomathy_table_group_texts(
    site: &CllSite,
    section_id: &str,
    area: CllTableRowArea,
    rows: &[Vec<CllTableCell>],
    groups: &mut Vec<CllChrestomathyGroupText>,
) {
    let Some(metadata) = chrestomathy_section_metadata(section_id) else {
        return;
    };
    for (group_index, group_rows) in chrestomathy_area_groups(metadata, area).iter().enumerate() {
        let mut lines = Vec::new();
        for row_index in group_rows {
            let Some(row) = row_index.checked_sub(1).and_then(|index| rows.get(index)) else {
                continue;
            };
            if let Some(text) = chrestomathy_table_source_cell_text(site, row) {
                lines.push(text);
            }
        }
        if lines.is_empty() {
            continue;
        }
        groups.push(new!(CllChrestomathyGroupText {
            section: section_id.to_owned(),
            area: chrestomathy_area_label(area).to_owned(),
            group_id: chrestomathy_group_id(section_id, area, group_index, group_rows),
            text: lines.join("\n"),
        }));
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|text| !text.trim().is_empty()))]
fn chrestomathy_table_source_cell_text(site: &CllSite, row: &[CllTableCell]) -> Option<String> {
    row.first()
        .map(|cell| normalized_plain_text(&blocks_plain_text(site, &cell.blocks)))
        .filter(|text| !text.trim().is_empty())
}

#[requires(true)]
#[ensures(true)]
pub fn cll_lookup_example<'a>(site: &'a CllSite, example_id: &str) -> Option<&'a CllExample> {
    site.examples_by_id.get(example_id)
}

#[requires(true)]
#[ensures(true)]
pub fn cll_first_section_id(site: &CllSite) -> Option<&str> {
    site.section_order.first().map(String::as_str)
}

#[requires(true)]
#[ensures(true)]
pub fn cll_previous_section_id<'a>(site: &'a CllSite, section_id: &str) -> Option<&'a str> {
    let index = site
        .section_order
        .iter()
        .position(|candidate| candidate == section_id)?;
    index
        .checked_sub(1)
        .and_then(|previous| site.section_order.get(previous))
        .map(String::as_str)
}

#[requires(true)]
#[ensures(true)]
pub fn cll_next_section_id<'a>(site: &'a CllSite, section_id: &str) -> Option<&'a str> {
    let index = site
        .section_order
        .iter()
        .position(|candidate| candidate == section_id)?;
    site.section_order.get(index + 1).map(String::as_str)
}

#[requires(true)]
#[ensures(true)]
pub fn render_cukta_request(
    site: &CllSite,
    request: &CuktaRequest,
    format: CllRenderFormat,
) -> Result<String, CllError> {
    match request {
        CuktaRequest::Toc => Ok(render_toc(site, format)),
        CuktaRequest::Index => Ok(render_index(site, format)),
        CuktaRequest::Section { reference } => {
            let section_id = cll_resolve_section_reference(site, reference)
                .ok_or_else(|| CllError::NotFound(format!("CLL section not found: {reference}")))?;
            let section = cll_lookup_section(site, &section_id)
                .ok_or_else(|| CllError::NotFound(format!("CLL section not found: {reference}")))?;
            Ok(render_section(site, section, format))
        }
        CuktaRequest::Example { reference } => {
            let example_id = cll_resolve_example_reference(site, reference)
                .ok_or_else(|| CllError::NotFound(format!("CLL example not found: {reference}")))?;
            let example = cll_lookup_example(site, &example_id)
                .ok_or_else(|| CllError::NotFound(format!("CLL example not found: {reference}")))?;
            Ok(render_example(site, example, format))
        }
        CuktaRequest::Search {
            mode,
            query,
            count,
            targets,
        } => {
            if *mode == CuktaSearchMode::Meaning {
                return Err(CllError::SemanticSearchDisabled);
            }
            Ok(render_search_output(
                &cukta_search(site, *mode, query, *count, *targets),
                format,
            ))
        }
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub fn render_toc(site: &CllSite, format: CllRenderFormat) -> String {
    match format {
        CllRenderFormat::Html => {
            let mut output =
                String::from("<nav class=\"cll-toc-rendered\"><h1>Table of Contents</h1><ol>");
            for chapter in &site.chapters {
                output.push_str("<li>");
                output.push_str(&escape_html(&format!(
                    "{}. {}",
                    chapter.chapter_number, chapter.chapter_title
                )));
                output.push_str("<ol>");
                for section_id in &chapter.root_section_ids {
                    let section = site
                        .sections_by_id
                        .get(section_id)
                        .expect("CllSite invariant guarantees chapter root section ids resolve");
                    output.push_str("<li><a href=\"");
                    output.push_str(&escape_html(&section_href(&section.section_id)));
                    output.push_str("\">");
                    output.push_str(&escape_html(&format_section_display_title(section)));
                    output.push_str("</a></li>");
                }
                output.push_str("</ol></li>");
            }
            output.push_str("</ol></nav>\n");
            output
        }
        CllRenderFormat::Markdown | CllRenderFormat::Raw => {
            let mut output = String::from("# Table of Contents\n\n");
            for chapter in &site.chapters {
                output.push_str(&format!(
                    "{}. {}\n",
                    chapter.chapter_number, chapter.chapter_title
                ));
                for section_id in &chapter.root_section_ids {
                    let section = site
                        .sections_by_id
                        .get(section_id)
                        .expect("CllSite invariant guarantees chapter root section ids resolve");
                    output.push_str(&format!("  - {}\n", format_section_display_title(section)));
                }
                output.push('\n');
            }
            output
        }
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub fn render_index(site: &CllSite, format: CllRenderFormat) -> String {
    match format {
        CllRenderFormat::Html => {
            let mut output = String::from("<section class=\"cll-index\"><h1>Index</h1>");
            for entry in &site.index_entries {
                output.push_str("<p><strong>");
                output.push_str(&escape_html(&entry.key));
                output.push_str("</strong>: ");
                output.push_str(
                    &entry
                        .section_ids
                        .iter()
                        .map(|section_id| {
                            site.sections_by_id.get(section_id).expect(
                                "CllSite invariant guarantees index entry section ids resolve",
                            )
                        })
                        .map(|section| {
                            format!(
                                "<a href=\"{}\">{}</a>",
                                escape_html(&section_href(&section.section_id)),
                                escape_html(&section.number)
                            )
                        })
                        .collect::<Vec<_>>()
                        .join(", "),
                );
                output.push_str("</p>");
            }
            output.push_str("</section>\n");
            output
        }
        CllRenderFormat::Markdown | CllRenderFormat::Raw => {
            let mut output = String::from("# Index\n\n");
            for entry in &site.index_entries {
                let refs = entry
                    .section_ids
                    .iter()
                    .map(|section_id| {
                        site.sections_by_id
                            .get(section_id)
                            .expect("CllSite invariant guarantees index entry section ids resolve")
                    })
                    .map(|section| section.number.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                output.push_str(&format!("- **{}**: {refs}\n", entry.key));
            }
            output
        }
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub fn render_section(site: &CllSite, section: &CllSection, format: CllRenderFormat) -> String {
    match format {
        CllRenderFormat::Html => {
            let mut output = String::new();
            output.push_str(
                "<article class=\"cll-section-content\"><div class=\"cll-section-heading\"><h1>",
            );
            output.push_str(&escape_html(&format_section_display_title(section)));
            output.push_str("</h1>");
            if let Some(parse_href) = chrestomathy_section_parse_href(site, section) {
                output.push_str(
                    "<a class=\"cll-parse-example cll-parse-section spa-cll-link spa-cll-link-parse\" href=\"",
                );
                output.push_str(&escape_html(&parse_href));
                output.push_str("\">Parse</a>");
            }
            output.push_str("</div>");
            for block in &section.blocks {
                output.push_str(&render_block_html(site, block));
            }
            output.push_str("</article>\n");
            output
        }
        CllRenderFormat::Markdown | CllRenderFormat::Raw => {
            let mut output = format!("# {}\n\n", format_section_display_title(section));
            if let Some(parse_href) = chrestomathy_section_parse_href(site, section) {
                output.push_str(&format!("[Parse]({parse_href})\n\n"));
            }
            for block in &section.blocks {
                render_block_markdown(site, block, &mut output, 0);
            }
            output
        }
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub fn render_example(site: &CllSite, example: &CllExample, format: CllRenderFormat) -> String {
    match format {
        CllRenderFormat::Html => {
            let mut output = format!(
                "<figure id=\"{}\" class=\"cll-example\"><figcaption class=\"cll-example-head\"><span class=\"cll-example-title\">{}</span>",
                escape_html(&example.anchor_id),
                escape_html(&example.label)
            );
            if let Some(parse_href) = &example.parse_href {
                output.push_str(
                    "<a class=\"cll-parse-example spa-cll-link spa-cll-link-parse\" href=\"",
                );
                output.push_str(&escape_html(parse_href));
                output.push_str("\">Parse</a>");
            }
            output.push_str("</figcaption>");
            for block in &example.blocks {
                output.push_str(&render_block_html(site, block));
            }
            output.push_str("</figure>\n");
            output
        }
        CllRenderFormat::Markdown | CllRenderFormat::Raw => {
            let mut output = format!("### {}", example.label);
            if let Some(parse_href) = &example.parse_href {
                output.push_str(&format!(" [Parse]({parse_href})"));
            }
            output.push_str("\n\n");
            for block in &example.blocks {
                render_block_markdown(site, block, &mut output, 0);
            }
            if example.blocks.is_empty() {
                for line in &example.lines {
                    if line.kind == CllExampleLineKind::Text {
                        output.push_str(&line.text);
                        output.push('\n');
                    } else {
                        output.push_str(&format!("{}: {}\n", line.kind.as_str(), line.text));
                    }
                }
                output.push('\n');
            }
            output
        }
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub fn render_search_output(output: &CuktaSearchOutput, format: CllRenderFormat) -> String {
    match format {
        CllRenderFormat::Html => {
            let mut rendered = String::from("<section class=\"cll-search-results\">");
            if let Some(message) = &output.message {
                rendered.push_str("<p>");
                rendered.push_str(&escape_html(message));
                rendered.push_str("</p>");
            }
            for item in &output.matches {
                rendered.push_str("<article class=\"cll-search-result\"><h2>");
                rendered.push_str(&escape_html(&format!(
                    "{}. {}",
                    item.rank, item.chunk.label
                )));
                rendered.push_str("</h2><p class=\"cll-search-result-meta\">");
                rendered.push_str(&escape_html(search_chunk_kind_label(item.chunk.kind)));
                rendered.push_str(" in ");
                rendered.push_str(&escape_html(&format!(
                    "{}. {}",
                    item.chunk.section_number, item.chunk.section_title
                )));
                rendered.push_str("</p><p>");
                rendered.push_str(&escape_html(&truncate_preview(&item.chunk.text, 420)));
                rendered.push_str("</p></article>");
            }
            rendered.push_str("</section>\n");
            rendered
        }
        CllRenderFormat::Markdown | CllRenderFormat::Raw => {
            let mut rendered = String::new();
            if let Some(message) = &output.message {
                rendered.push_str(message);
                rendered.push_str("\n\n");
            }
            for item in &output.matches {
                rendered.push_str(&format!("### {}. {}\n\n", item.rank, item.chunk.label));
                rendered.push_str(&format!(
                    "{} in {}. {}\n\n",
                    search_chunk_kind_label(item.chunk.kind),
                    item.chunk.section_number,
                    item.chunk.section_title
                ));
                rendered.push_str(&truncate_preview(&item.chunk.text, 420));
                rendered.push_str("\n\n");
            }
            if rendered.is_empty() {
                "No matches found.\n".to_owned()
            } else {
                rendered
            }
        }
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub fn format_section_display_title(section: &CllSection) -> String {
    format!("{}. {}", section.number, section.title)
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|title| !title.is_empty()))]
pub fn cll_section_chapter_title(site: &CllSite, section_id: &str) -> Option<String> {
    let section = site.sections_by_id.get(section_id)?;
    site.chapters
        .iter()
        .find(|chapter| chapter.chapter_id == section.chapter_id)
        .map(|chapter| chapter.chapter_title.clone())
}

#[requires(true)]
#[ensures(true)]
fn inline_plain_text(inlines: &[CllInline]) -> String {
    let mut visitor = InlinePlainTextVisitor {
        output: String::new(),
    };
    visitor.visit_inline_run(inlines);
    normalized_plain_text(&visitor.output)
}

#[invariant(true)]
struct InlinePlainTextVisitor {
    output: String,
}

#[contract_trait]
impl CllBlockVisitor for InlinePlainTextVisitor {
    #[requires(true)]
    #[ensures(true)]
    fn visit_inline(&mut self, inline: &CllInline) {
        match inline {
            CllInline::Text(text) | CllInline::Code(text) | CllInline::InlineMath { text, .. } => {
                self.output.push_str(text);
                self.output.push(' ');
            }
            CllInline::Emphasis { inlines, .. }
            | CllInline::Quote { inlines, .. }
            | CllInline::LanguageSpan { inlines, .. }
            | CllInline::CiteTitle { inlines }
            | CllInline::Subscript { inlines }
            | CllInline::Superscript { inlines }
            | CllInline::Link { inlines, .. } => {
                self.visit_inline_run(inlines);
                self.output.push(' ');
            }
            CllInline::Elidable { shown, inlines, .. } => {
                if inlines.is_empty() {
                    self.output.push_str(shown);
                } else {
                    self.visit_inline_run(inlines);
                }
                self.output.push(' ');
            }
            CllInline::Anchor { .. } => {}
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn blocks_plain_text(site: &CllSite, blocks: &[CllBlock]) -> String {
    let mut visitor = BlockPlainTextVisitor {
        site,
        output: String::new(),
    };
    visitor.visit_blocks(blocks);
    normalized_plain_text(&visitor.output)
}

#[invariant(true)]
struct BlockPlainTextVisitor<'site> {
    site: &'site CllSite,
    output: String,
}

#[contract_trait]
impl CllBlockVisitor for BlockPlainTextVisitor<'_> {
    #[requires(true)]
    #[ensures(true)]
    fn visit_block(&mut self, block: &CllBlock) {
        match block {
            CllBlock::Paragraph { text, .. }
            | CllBlock::Code { text, .. }
            | CllBlock::Heading { title: text, .. }
            | CllBlock::DisplayMath { text, .. } => {
                self.output.push_str(text);
                self.output.push('\n');
            }
            CllBlock::List { items, .. } => {
                for item in items {
                    self.visit_blocks(item);
                    self.output.push('\n');
                }
            }
            CllBlock::Example { example_id } => {
                if let Some(example) = cll_lookup_example(self.site, example_id) {
                    self.output.push_str(&example.plain_text);
                    self.output.push('\n');
                }
            }
            CllBlock::Table {
                caption,
                header_rows,
                body_rows,
                ..
            } => {
                if let Some(caption) = caption {
                    self.output.push_str(&inline_plain_text(caption));
                    self.output.push('\n');
                }
                for row in header_rows.iter().chain(body_rows.iter()) {
                    for cell in row {
                        self.visit_blocks(&cell.blocks);
                        self.output.push('\n');
                    }
                }
            }
            CllBlock::SimpleListTable { rows, .. } => {
                for row in rows {
                    for cell in row.iter().flatten() {
                        self.output.push_str(&inline_plain_text(cell));
                        self.output.push('\n');
                    }
                }
            }
            CllBlock::VariableList { entries, .. } => {
                for entry in entries {
                    self.output.push_str(&inline_plain_text(&entry.term));
                    self.output.push('\n');
                    self.visit_blocks(&entry.blocks);
                    self.output.push('\n');
                }
            }
            CllBlock::Media { alt, .. } => {
                self.output.push_str(alt);
                self.output.push('\n');
            }
            CllBlock::Rule { term, body, .. } => {
                self.output.push_str(term);
                self.output.push('\n');
                self.visit_blocks(body);
            }
            CllBlock::BlockQuote { blocks, .. } => {
                self.visit_blocks(blocks);
                self.output.push('\n');
            }
            CllBlock::Definition { body, .. } | CllBlock::GrammarTemplate { body, .. } => {
                self.output.push_str(&inline_plain_text(body));
                self.output.push('\n');
            }
            CllBlock::InterlinearGloss {
                rows,
                natlang,
                comments,
                ..
            } => {
                for row in rows {
                    for cell in &row.cells {
                        self.output.push_str(&inline_plain_text(cell));
                        self.output.push('\n');
                    }
                }
                for line in natlang.iter().chain(comments.iter()) {
                    self.output.push_str(&inline_plain_text(line));
                    self.output.push('\n');
                }
            }
            CllBlock::CmavoList {
                titles,
                headers,
                rows,
                ..
            } => {
                for line in titles {
                    self.output.push_str(&inline_plain_text(line));
                    self.output.push('\n');
                }
                let column_count = rows.iter().map(Vec::len).max().unwrap_or(0);
                for index in 0..column_count {
                    if let Some(header) = headers.get(index) {
                        self.output.push_str(&inline_plain_text(header));
                    } else {
                        self.output
                            .push_str(cmavo_list_plain_text_column_label(index));
                    }
                    self.output.push('\n');
                }
                for row in rows {
                    for cell in row {
                        self.output.push_str(&inline_plain_text(cell));
                        self.output.push('\n');
                    }
                }
            }
            CllBlock::Lojbanization { lines, .. } => {
                for line in lines {
                    self.output.push_str(&inline_plain_text(&line.body));
                    self.output.push('\n');
                    if let Some(comment) = &line.comment {
                        self.output.push_str(&inline_plain_text(comment));
                        self.output.push('\n');
                    }
                }
            }
            CllBlock::LujvoMaking { parts, .. } => {
                for part in parts {
                    self.output.push_str(&inline_plain_text(&part.body));
                    self.output.push('\n');
                }
            }
            CllBlock::Ebnf { entries, .. } => {
                for entry in entries {
                    self.output.push_str(&entry.rule_name);
                    self.output.push('\n');
                    for token in &entry.rhs {
                        self.output.push_str(&ebnf_token_plain_text(token));
                    }
                    self.output.push('\n');
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn cmavo_list_plain_text_column_label(index: usize) -> &'static str {
    match index {
        0 => "cmavo",
        1 => "selma'o",
        2 => "description",
        _ => "",
    }
}

#[requires(true)]
#[ensures(true)]
fn ebnf_token_plain_text(token: &CllEbnfToken) -> String {
    match token {
        CllEbnfToken::Text { body }
        | CllEbnfToken::Operator { body }
        | CllEbnfToken::Hash { body }
        | CllEbnfToken::Terminal { body, .. }
        | CllEbnfToken::ElidableTerminator { body, .. }
        | CllEbnfToken::Nonterminal { body, .. } => body.clone(),
    }
}

#[requires(true)]
#[ensures(true)]
fn escape_html(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    escape_html_into(&mut output, input);
    output
}

#[requires(true)]
#[ensures(true)]
fn escape_html_into(output: &mut String, input: &str) {
    for ch in input.chars() {
        match ch {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            '"' => output.push_str("&quot;"),
            _ => output.push(ch),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    #[allow(unused_imports)]
    use bityzba::{ensures, new, requires};
    use jbotci_morphology::segment_words_with_modifiers;
    use jbotci_syntax::{ParseOptions, parse_syntax_tree_with_options};

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn embedded_site_loads_default_section() {
        let site = embedded_cll_site().expect("embedded CLL should load");
        let section = cll_lookup_section(site, DEFAULT_CUKTA_SECTION_ID)
            .expect("default section should exist");
        assert_eq!(section.number, "1.1");
        assert_eq!(section.title, "What is Lojban?");
        assert!(!site.index_entries.is_empty());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn references_resolve_sections_and_examples() {
        let site = embedded_cll_site().expect("embedded CLL should load");
        assert_eq!(
            cll_resolve_section_reference(site, "1.1").as_deref(),
            Some(DEFAULT_CUKTA_SECTION_ID)
        );
        assert_eq!(
            cll_resolve_section_reference(site, "c2").as_deref(),
            Some("section-bridi")
        );
        assert!(
            cll_resolve_example_reference(site, "2.1")
                .as_deref()
                .is_some_and(|value| !value.is_empty())
        );
        assert!(
            cll_resolve_example_reference(site, "example-random-id-qIuj")
                .as_deref()
                .is_some_and(|value| !value.is_empty())
        );
        assert_eq!(
            cll_link_href(site, CllLinkKind::Section, "example-random-id-qIuj"),
            "section/section-bridi#c2e1d1"
        );
        assert_eq!(
            cll_link_href(site, CllLinkKind::Section, "chapter-tour"),
            "section/section-bridi#chapter-tour"
        );
        assert_eq!(
            cll_link_href(site, CllLinkKind::Section, "chapter-grammars"),
            "section/section-EBNF#chapter-grammars"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn section_example_blocks_render_from_canonical_examples() {
        let site = embedded_cll_site().expect("embedded CLL should load");
        let (block, example_id) = site
            .section_order
            .iter()
            .filter_map(|section_id| cll_lookup_section(site, section_id))
            .find_map(|section| first_example_block(&section.blocks))
            .expect("embedded CLL should contain examples");
        let example = cll_lookup_example(site, example_id).expect("example id should resolve");
        let mut block_markdown = String::new();
        render_block_markdown(site, block, &mut block_markdown, 0);

        assert_eq!(
            block_markdown,
            render_example(site, example, CllRenderFormat::Markdown)
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn example_search_chunks_preserve_line_breaks() {
        let site = embedded_cll_site().expect("embedded CLL should load");
        let chunk = site
            .search_chunks
            .iter()
            .find(|chunk| {
                chunk.kind == CllSearchChunkKind::Example
                    && chunk.section_number == "1.3"
                    && chunk.label == "Example 1.1"
            })
            .expect("example 1.1 search chunk should exist");

        assert_eq!(
            chunk.text,
            "mi klama le zarci\nI go-to that-which-I-describe-as-a store.\nI go to the store."
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn specialized_blocks_contribute_tagged_words() {
        let site = embedded_cll_site().expect("embedded CLL should load");
        let coi = vec![CllInline::Link {
            target: "coi".to_owned(),
            inlines: vec![CllInline::Text("coi".to_owned())],
            kind: CllLinkKind::Dictionary,
        }];
        let cmavo_list = CllBlock::CmavoList {
            id: None,
            titles: Vec::new(),
            headers: Vec::new(),
            rows: vec![vec![coi.clone()]],
        };
        let interlinear = CllBlock::InterlinearGloss {
            id: None,
            aligned: false,
            itemized: false,
            parse_href: None,
            rows: vec![new!(CllInterlinearRow {
                kind: CllInterlinearRowKind::Jbo,
                cells: vec![coi],
            })],
            natlang: Vec::new(),
            comments: Vec::new(),
        };
        let mut words = block_tagged_words(site, &cmavo_list);
        words.extend(block_tagged_words(site, &interlinear));

        assert!(words.contains("coi"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn split_paragraph_keeps_anchor_on_one_segment_only() {
        let document = Document::parse(
            r#"<para id="split-para">before <example id="ex"><para>coi</para></example> after</para>"#,
        )
        .expect("test XML should parse");
        let context = test_section_context();
        let mut parse_state = BlockParseState {
            chapter_example_counter: 0,
        };
        let mut examples = Vec::new();
        let mut anchors = Vec::new();
        let blocks = parse_paragraph_blocks(
            document.root_element(),
            &context,
            AnchorMode::TopLevel,
            &mut parse_state,
            &mut examples,
            &mut anchors,
        );

        assert_eq!(
            count_paragraph_anchor(&blocks, "split-para"),
            1,
            "{blocks:#?}"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn admonitions_preserve_inline_links() {
        let document = Document::parse(
            r#"<note id="note"><para>See <xref linkend="section-erasure" /></para></note>"#,
        )
        .expect("test XML should parse");
        let context = test_section_context();
        let mut parse_state = BlockParseState {
            chapter_example_counter: 0,
        };
        let mut examples = Vec::new();
        let mut anchors = Vec::new();
        let blocks = parse_block(
            document.root_element(),
            &context,
            AnchorMode::TopLevel,
            &mut parse_state,
            &mut examples,
            &mut anchors,
        );

        assert!(blocks.iter().any(block_contains_section_erasure_link));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn import_metadata_drives_special_cll_ids() {
        let site = embedded_cll_site().expect("embedded CLL should load");
        let metadata = cll_import_metadata();
        let chrestomathy = site
            .chapters
            .iter()
            .find(|chapter| chapter.chapter_id == metadata.chrestomathy_chapter_id)
            .expect("metadata chrestomathy chapter should exist");

        assert_eq!(chrestomathy.chapter_number, 22);
        assert!(cll_lookup_section(site, &metadata.ebnf_section_id).is_some());
        assert_eq!(
            ebnf_symbol_href("BRIVLA").as_deref(),
            Some("section/section-morphology-brivla")
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn xrefs_render_as_reference_labels_not_xml_ids() {
        let site = embedded_cll_site().expect("embedded CLL should load");
        let section = cll_lookup_section(site, "section-bridi").expect("section should exist");
        let rendered = render_section(site, section, CllRenderFormat::Markdown);
        assert!(rendered.contains("Example 2.1"));
        assert!(rendered.contains("John is the father of Sam."));
        assert!(!rendered.contains("[example-random-id-qIuj]"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn chapter_xrefs_render_as_chapter_labels_not_xml_ids() {
        let site = embedded_cll_site().expect("embedded CLL should load");
        let section =
            cll_lookup_section(site, "section-what-is-cll").expect("section should exist");
        let rendered = render_section(site, section, CllRenderFormat::Markdown);

        assert!(rendered.contains("[Chapter 21](section/section-EBNF#chapter-grammars)"));
        assert!(rendered.contains("[Chapter 2](section/section-bridi#chapter-tour)"));
        assert!(!rendered.contains("[chapter-grammars]"));
        assert!(!rendered.contains("[chapter-tour]"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn headerless_cmavo_lists_do_not_synthesize_headers() {
        let site = embedded_cll_site().expect("embedded CLL should load");
        let section = cll_lookup_section(site, "section-vocative-scales")
            .expect("vocative scales section should exist");
        let cmavo_lists = section
            .blocks
            .iter()
            .filter_map(|block| match block {
                CllBlock::CmavoList { headers, rows, .. } => Some((headers, rows)),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert!(cmavo_lists.len() > 5);
        assert!(
            cmavo_lists
                .iter()
                .take(6)
                .all(|(headers, _)| headers.is_empty())
        );
        assert_eq!(
            cmavo_list_row_texts(&cmavo_lists[0].1[0]),
            vec!["coi".to_owned(), "greetings".to_owned()]
        );
        assert_eq!(
            cmavo_list_row_texts(&cmavo_lists[2].1[0]),
            vec!["co'o".to_owned(), "partings".to_owned()]
        );
        assert_eq!(
            cmavo_list_row_texts(&cmavo_lists[4].1[0]),
            vec![
                "ju'i".to_owned(),
                "[jundi]".to_owned(),
                "attention".to_owned(),
                "at ease".to_owned(),
                "ignore me/us".to_owned(),
            ]
        );

        let html = render_section(site, section, CllRenderFormat::Html);
        assert!(!html.contains("<th>cmavo</th>"));
        assert!(html.contains("<tr><td>coi</td><td>greetings</td></tr>"));
        assert!(html.contains(
            "<tr><td>ju'i</td><td>[jundi]</td><td>attention</td><td>at ease</td><td>ignore me/us</td></tr>"
        ));

        let markdown = render_section(site, section, CllRenderFormat::Markdown);
        assert!(!markdown.contains("| cmavo |"));
        assert!(!markdown.contains("| --- |"));
        assert!(markdown.contains("coi | greetings\n\n"));
        assert!(markdown.contains("ju'i | [jundi] | attention | at ease | ignore me/us\n\n"));

        let plain_text = blocks_plain_text(site, &section.blocks);
        assert!(plain_text.contains("cmavo selma'o coi greetings"));
        assert!(plain_text.contains("cmavo selma'o co'o partings"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn explicit_cmavo_list_headers_are_preserved() {
        let site = embedded_cll_site().expect("embedded CLL should load");
        let section = cll_lookup_section(site, "section-irregular-BAI")
            .expect("irregular BAI section should exist");
        let (headers, _rows) = section
            .blocks
            .iter()
            .find_map(|block| match block {
                CllBlock::CmavoList { headers, rows, .. } if !headers.is_empty() => {
                    Some((headers, rows))
                }
                _ => None,
            })
            .expect("section should contain an explicitly headed cmavo list");

        assert_eq!(
            cmavo_list_row_texts(headers),
            vec![
                "cmavo".to_owned(),
                "gismu".to_owned(),
                "comments".to_owned()
            ]
        );

        let html = render_section(site, section, CllRenderFormat::Html);
        assert!(html.contains("<th>cmavo</th><th>gismu</th><th>comments</th>"));

        let markdown = render_section(site, section, CllRenderFormat::Markdown);
        assert!(markdown.contains("| cmavo | gismu | comments |"));
        assert!(markdown.contains("| --- | --- | --- |"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn bridgehead_anchors_render_as_heading_ids() {
        let site = embedded_cll_site().expect("embedded CLL should load");
        let section = cll_lookup_section(site, "section-index").expect("section should exist");

        assert!(section.blocks.iter().any(|block| {
            matches!(
                block,
                CllBlock::Heading {
                    id: Some(id),
                    title,
                    ..
                } if id == "NAI" && title.contains("selma'o NAI")
            )
        }));
        let rendered = render_section(site, section, CllRenderFormat::Html);
        assert!(rendered.contains("id=\"NAI\""));
        assert!(rendered.contains("selma'o NAI"));
        assert!(section.blocks.iter().any(|block| {
            matches!(
                block,
                CllBlock::Heading { title, .. }
                    if title.contains("selma'o UI")
                        && !title.contains("section-attitudinals-introduction")
            )
        }));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn standalone_interlinear_glosses_have_parse_links() {
        let site = embedded_cll_site().expect("embedded CLL should load");
        let section = cll_lookup_section(site, "section-index").expect("section should exist");
        let parse_hrefs = collect_interlinear_parse_hrefs(site, &section.blocks);

        assert!(!parse_hrefs.is_empty());
        assert!(
            parse_hrefs
                .iter()
                .all(|href| href.starts_with("../gentufa?text=") && !href.contains("dialect="))
        );
        assert!(
            render_section(site, section, CllRenderFormat::Html)
                .contains("class=\"cll-parse-example spa-cll-link spa-cll-link-parse\"")
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn jbophrase_examples_have_parse_links() {
        let site = embedded_cll_site().expect("embedded CLL should load");
        let example = cll_lookup_example(site, "c19e11d6").expect("example should exist");

        let parse_href = example
            .parse_href
            .as_deref()
            .expect("example should have parse link");
        assert!(parse_href.contains("ba%27e%20mi%20viska%20la%20.djordj."));
        assert!(!parse_href.contains("dialect="));
        assert!(
            render_example(site, example, CllRenderFormat::Html)
                .contains("class=\"cll-parse-example spa-cll-link spa-cll-link-parse\"")
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn multiline_interlinear_examples_keep_line_rendering() {
        let site = embedded_cll_site().expect("embedded CLL should load");
        let section = cll_lookup_section(site, "section-quantifier-grouping")
            .expect("quantifier grouping section should exist");

        let markdown = render_section(site, section, CllRenderFormat::Markdown);
        assert!(markdown.contains("### Example 16.45"));
        assert!(markdown.contains("jbo: - [ci](../vlacku/ci)"));
        assert!(markdown.contains("jbo: [nu'i](../vlacku/nu'i)"));
        assert!(markdown.contains("gloss: - Three dogs [plus] two men, - - bite."));
        assert!(!markdown.contains("| - [ci](../vlacku/ci)"));

        let html = render_section(site, section, CllRenderFormat::Html);
        let example_start = html
            .find("Example 16.45")
            .expect("Example 16.45 should render in HTML");
        let example_end = html[example_start..]
            .find("</figure>")
            .map(|offset| example_start + offset)
            .expect("example figure should close");
        let example_html = &html[example_start..example_end];
        assert!(example_html.contains("cll-interlinear-itemized"));
        assert!(example_html.contains("cll-ig-line cll-ig-inline cll-ig-jbo"));
        assert!(!example_html.contains("cll-interlinear-table"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn literal_layout_blocks_preserve_lines_and_alignment() {
        let site = embedded_cll_site().expect("embedded CLL should load");
        let section =
            cll_lookup_section(site, "section-scalar-negation").expect("section should exist");
        let expected = concat!(
            "Affirmations (positive)      Negations (negative)\n",
            "|-----------|-----------|-----------|-----------|\n",
            "All       Most        Some         Few       None\n",
            "Excellent Good        Fair         Poor     Awful",
        );
        let code_text = section
            .blocks
            .iter()
            .find_map(|block| match block {
                CllBlock::Code { text, .. } if text.contains("Affirmations") => Some(text),
                _ => None,
            })
            .expect("scale literal layout should be a code block");

        assert_eq!(code_text, expected);

        let markdown = render_section(site, section, CllRenderFormat::Markdown);
        assert!(markdown.contains(&format!("```\n{expected}\n```")));
        assert!(
            !markdown
                .contains("Affirmations (positive) Negations (negative) |-----------|-----------|")
        );

        let html = render_section(site, section, CllRenderFormat::Html);
        assert!(html.contains(&format!(
            "<pre class=\"cll-code\"><code>{expected}</code></pre>"
        )));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn north_wind_section_omits_hidden_vocabulary_dump() {
        let site = embedded_cll_site().expect("embedded CLL should load");
        let section = cll_lookup_section(site, "section-north-wind").expect("section should exist");
        assert!(!section.plain_text.contains(".alf."));
        assert!(!blocks_plain_text(site, &section.blocks).contains(".alf."));
        assert!(!render_section(site, section, CllRenderFormat::Html).contains(".alf."));
        assert!(!render_section(site, section, CllRenderFormat::Markdown).contains(".alf."));
        assert!(
            site.search_chunks
                .iter()
                .filter(|chunk| chunk.section_id == "section-north-wind")
                .all(|chunk| !chunk.text.contains(".alf."))
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn chrestomathy_table_source_cells_have_baseline_parse_links() {
        let site = embedded_cll_site().expect("embedded CLL should load");
        let section = cll_lookup_section(site, "section-north-wind").expect("section should exist");
        let parse_hrefs = collect_table_parse_hrefs(site, &section.blocks);
        assert!(!parse_hrefs.is_empty());
        assert!(
            parse_hrefs
                .iter()
                .all(|href| href.starts_with("../gentufa?text=") && !href.contains("dialect="))
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn chrestomathy_metadata_has_no_overlaps_and_covers_source_rows() {
        let site = embedded_cll_site().expect("embedded CLL should load");
        for metadata in &chrestomathy_metadata().section {
            let section =
                cll_lookup_section(site, &metadata.id).expect("metadata section should exist");
            assert_chrestomathy_area_metadata_is_disjoint(metadata, CllTableRowArea::Header);
            assert_chrestomathy_area_metadata_is_disjoint(metadata, CllTableRowArea::Body);
            let (header_rows, body_rows) = first_table_rows(section);
            assert_chrestomathy_rows_are_covered(
                site,
                metadata,
                CllTableRowArea::Header,
                header_rows,
            );
            assert_chrestomathy_rows_are_covered(site, metadata, CllTableRowArea::Body, body_rows);
        }
    }

    #[test]
    #[ignore = "corpus-wide chrestomathy parse target check is useful but too long for default test runs"]
    #[requires(true)]
    #[ensures(true)]
    fn chrestomathy_metadata_group_and_section_parse_targets_parse() {
        let site = embedded_cll_site().expect("embedded CLL should load");
        for metadata in &chrestomathy_metadata().section {
            let section =
                cll_lookup_section(site, &metadata.id).expect("metadata section should exist");
            for group in chrestomathy_section_group_texts(site, section) {
                let data!(CllChrestomathyGroupText { group_id, text, .. }) = group.into_data();
                assert_parseable_chrestomathy_text(&group_id, &text);
            }
            let section_text = chrestomathy_section_group_texts(site, section)
                .into_iter()
                .map(|group| group.into_data().text)
                .collect::<Vec<_>>()
                .join("\n");
            assert_parseable_chrestomathy_text(&section.section_id, &section_text);
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn chrestomathy_grouped_rows_render_single_parse_button_and_group_classes() {
        let site = embedded_cll_site().expect("embedded CLL should load");
        let section =
            cll_lookup_section(site, "section-forest-nymph").expect("section should exist");
        let (_header_rows, body_rows) = first_table_rows(section);
        let row_12_group = body_rows[11][0]
            .parse_group
            .as_ref()
            .expect("row 12 should have parse group");
        let row_13_group = body_rows[12][0]
            .parse_group
            .as_ref()
            .expect("row 13 should have parse group");
        assert_eq!(row_12_group.group_id, row_13_group.group_id);
        assert_eq!(row_12_group.row_count, 2);
        assert_eq!(row_12_group.row_index, 0);
        assert_eq!(row_13_group.row_index, 1);
        assert!(body_rows[11][0].parse_href.is_some());
        assert!(body_rows[12][0].parse_href.is_none());

        let html = render_section(site, section, CllRenderFormat::Html);
        assert!(html.contains("cll-parse-section"));
        assert!(html.contains("cll-parse-group-start"));
        assert!(html.contains("cll-parse-group-continuation"));
        assert!(html.contains("cll-parse-group-link"));
        assert!(html.contains("data-cll-parse-group=\"section-forest-nymph-body-12-12-13\""));
    }

    #[requires(true)]
    #[ensures(true)]
    fn assert_chrestomathy_area_metadata_is_disjoint(
        metadata: &CllChrestomathySectionMetadata,
        area: CllTableRowArea,
    ) {
        let mut covered = BTreeSet::new();
        for group in chrestomathy_area_groups(metadata, area) {
            for row in group {
                assert!(
                    covered.insert(*row),
                    "{} {} row {} is listed more than once",
                    metadata.id,
                    chrestomathy_area_label(area),
                    row
                );
            }
        }
        for row in chrestomathy_area_no_parse_rows(metadata, area) {
            assert!(
                covered.insert(*row),
                "{} {} row {} is both parseable and no-parse",
                metadata.id,
                chrestomathy_area_label(area),
                row
            );
        }
    }

    #[requires(true)]
    #[ensures(ret.iter().all(|cell| !cell.is_empty()))]
    fn cmavo_list_row_texts(row: &[Vec<CllInline>]) -> Vec<String> {
        row.iter()
            .map(|cell| inline_plain_text(cell))
            .filter(|cell| !cell.is_empty())
            .collect()
    }

    #[requires(true)]
    #[ensures(true)]
    fn assert_chrestomathy_rows_are_covered(
        site: &CllSite,
        metadata: &CllChrestomathySectionMetadata,
        area: CllTableRowArea,
        rows: &[Vec<CllTableCell>],
    ) {
        let covered = chrestomathy_area_groups(metadata, area)
            .iter()
            .flatten()
            .copied()
            .chain(
                chrestomathy_area_no_parse_rows(metadata, area)
                    .iter()
                    .copied(),
            )
            .collect::<BTreeSet<_>>();
        let no_parse = chrestomathy_area_no_parse_rows(metadata, area)
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        for (index, row) in rows.iter().enumerate() {
            let row_index = index + 1;
            let Some(text) = chrestomathy_table_source_cell_text(site, row) else {
                continue;
            };
            assert!(
                covered.contains(&row_index),
                "{} {} row {} is missing metadata: {}",
                metadata.id,
                chrestomathy_area_label(area),
                row_index,
                text
            );
            let parse_group = row.first().and_then(|cell| cell.parse_group.as_ref());
            if no_parse.contains(&row_index) {
                assert!(
                    parse_group.is_none(),
                    "no-parse rows should not have groups"
                );
            } else {
                assert!(parse_group.is_some(), "parseable rows should have groups");
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn assert_parseable_chrestomathy_text(label: &str, text: &str) {
        let words = segment_words_with_modifiers(text)
            .unwrap_or_else(|error| panic!("{label} morphology failed: {error:?}"));
        parse_syntax_tree_with_options(&words, &ParseOptions::default())
            .unwrap_or_else(|error| panic!("{label} syntax failed: {error:?}"));
    }

    #[requires(true)]
    #[ensures(!ret.0.is_empty() || !ret.1.is_empty())]
    fn first_table_rows(section: &CllSection) -> (&[Vec<CllTableCell>], &[Vec<CllTableCell>]) {
        section
            .blocks
            .iter()
            .find_map(|block| match block {
                CllBlock::Table {
                    header_rows,
                    body_rows,
                    ..
                } => Some((header_rows.as_slice(), body_rows.as_slice())),
                _ => None,
            })
            .expect("chrestomathy section should have a table")
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn exact_word_search_uses_normalized_terms_and_targets() {
        let site = embedded_cll_site().expect("embedded CLL should load");
        let matches = cukta_word_search_matches(
            site,
            ".lojban.",
            5,
            CuktaTargetFilter {
                sections: true,
                paragraphs: false,
                examples: false,
            },
        );
        assert!(!matches.is_empty());
        assert!(
            matches
                .iter()
                .all(|item| item.chunk.kind == CllSearchChunkKind::Section)
        );
        assert_eq!(
            collect_tagged_words("lojbanh")
                .into_iter()
                .next()
                .as_deref(),
            Some("lojban'")
        );
        assert_eq!(
            parse_word_search_terms("шой")
                .into_iter()
                .collect::<Vec<_>>(),
            vec!["coi"]
        );
        assert_eq!(normalize_valsis_query("\u{ed86}\u{eda8}"), "coi");

        let non_latin_matches =
            cukta_word_search_matches(site, "\u{ed86}\u{eda8}", 5, CuktaTargetFilter::default());
        assert!(!non_latin_matches.is_empty());
    }

    #[requires(true)]
    #[ensures(!ret.section_id.is_empty())]
    fn test_section_context() -> SectionParseContext {
        SectionParseContext {
            chapter_id: "chapter-test".to_owned(),
            chapter_number: 1,
            section_id: "section-test".to_owned(),
            section_number: "1.1".to_owned(),
            section_title: "Test".to_owned(),
            source_path: "test.xml".to_owned(),
        }
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_none_or(|(_, id)| !id.is_empty()))]
    fn first_example_block(blocks: &[CllBlock]) -> Option<(&CllBlock, &str)> {
        for block in blocks {
            match block {
                CllBlock::Example { example_id } => return Some((block, example_id)),
                CllBlock::List { items, .. } => {
                    for item in items {
                        if let Some(found) = first_example_block(item) {
                            return Some(found);
                        }
                    }
                }
                CllBlock::Table {
                    header_rows,
                    body_rows,
                    ..
                } => {
                    for cell in header_rows.iter().chain(body_rows.iter()).flatten() {
                        if let Some(found) = first_example_block(&cell.blocks) {
                            return Some(found);
                        }
                    }
                }
                CllBlock::VariableList { entries, .. } => {
                    for entry in entries {
                        if let Some(found) = first_example_block(&entry.blocks) {
                            return Some(found);
                        }
                    }
                }
                CllBlock::Rule { body, .. } | CllBlock::BlockQuote { blocks: body, .. } => {
                    if let Some(found) = first_example_block(body) {
                        return Some(found);
                    }
                }
                _ => {}
            }
        }
        None
    }

    #[requires(!anchor_id.is_empty())]
    #[ensures(true)]
    fn count_paragraph_anchor(blocks: &[CllBlock], anchor_id: &str) -> usize {
        blocks
            .iter()
            .map(|block| match block {
                CllBlock::Paragraph {
                    anchor_id: Some(id),
                    ..
                } if id == anchor_id => 1,
                CllBlock::List { items, .. } => items
                    .iter()
                    .map(|item| count_paragraph_anchor(item, anchor_id))
                    .sum(),
                CllBlock::Table {
                    header_rows,
                    body_rows,
                    ..
                } => header_rows
                    .iter()
                    .chain(body_rows.iter())
                    .flatten()
                    .map(|cell| count_paragraph_anchor(&cell.blocks, anchor_id))
                    .sum(),
                CllBlock::VariableList { entries, .. } => entries
                    .iter()
                    .map(|entry| count_paragraph_anchor(&entry.blocks, anchor_id))
                    .sum(),
                CllBlock::Rule { body, .. } | CllBlock::BlockQuote { blocks: body, .. } => {
                    count_paragraph_anchor(body, anchor_id)
                }
                _ => 0,
            })
            .sum()
    }

    #[requires(true)]
    #[ensures(true)]
    fn block_contains_section_erasure_link(block: &CllBlock) -> bool {
        match block {
            CllBlock::Paragraph { inlines, .. } => inlines_contain_section_erasure_link(inlines),
            CllBlock::List { items, .. } => items
                .iter()
                .any(|item| item.iter().any(block_contains_section_erasure_link)),
            CllBlock::Table {
                header_rows,
                body_rows,
                ..
            } => header_rows.iter().chain(body_rows.iter()).any(|row| {
                row.iter()
                    .any(|cell| cell.blocks.iter().any(block_contains_section_erasure_link))
            }),
            CllBlock::VariableList { entries, .. } => entries
                .iter()
                .any(|entry| entry.blocks.iter().any(block_contains_section_erasure_link)),
            CllBlock::Rule { body, .. } | CllBlock::BlockQuote { blocks: body, .. } => {
                body.iter().any(block_contains_section_erasure_link)
            }
            _ => false,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn inlines_contain_section_erasure_link(inlines: &[CllInline]) -> bool {
        inlines.iter().any(|inline| match inline {
            CllInline::Link { target, .. } => target == "section-erasure",
            CllInline::Emphasis { inlines, .. }
            | CllInline::Quote { inlines, .. }
            | CllInline::LanguageSpan { inlines, .. }
            | CllInline::CiteTitle { inlines }
            | CllInline::Subscript { inlines }
            | CllInline::Superscript { inlines }
            | CllInline::Elidable { inlines, .. } => inlines_contain_section_erasure_link(inlines),
            _ => false,
        })
    }

    #[requires(true)]
    #[ensures(true)]
    fn collect_table_parse_hrefs(site: &CllSite, blocks: &[CllBlock]) -> Vec<String> {
        let mut hrefs = Vec::new();
        for block in blocks {
            match block {
                CllBlock::Table {
                    header_rows,
                    body_rows,
                    ..
                } => {
                    for row in header_rows.iter().chain(body_rows.iter()) {
                        for cell in row {
                            if let Some(parse_href) = &cell.parse_href {
                                hrefs.push(parse_href.clone());
                            }
                            hrefs.extend(collect_table_parse_hrefs(site, &cell.blocks));
                        }
                    }
                }
                CllBlock::List { items, .. } => {
                    for item in items {
                        hrefs.extend(collect_table_parse_hrefs(site, item));
                    }
                }
                CllBlock::Example { example_id } => {
                    if let Some(example) = cll_lookup_example(site, example_id) {
                        hrefs.extend(collect_table_parse_hrefs(site, &example.blocks));
                    }
                }
                CllBlock::BlockQuote { blocks, .. } | CllBlock::Rule { body: blocks, .. } => {
                    hrefs.extend(collect_table_parse_hrefs(site, blocks));
                }
                CllBlock::VariableList { entries, .. } => {
                    for entry in entries {
                        hrefs.extend(collect_table_parse_hrefs(site, &entry.blocks));
                    }
                }
                _ => {}
            }
        }
        hrefs
    }

    #[requires(true)]
    #[ensures(true)]
    fn collect_interlinear_parse_hrefs(site: &CllSite, blocks: &[CllBlock]) -> Vec<String> {
        let mut hrefs = Vec::new();
        for block in blocks {
            match block {
                CllBlock::InterlinearGloss { parse_href, .. } => {
                    if let Some(parse_href) = parse_href {
                        hrefs.push(parse_href.clone());
                    }
                }
                CllBlock::List { items, .. } => {
                    for item in items {
                        hrefs.extend(collect_interlinear_parse_hrefs(site, item));
                    }
                }
                CllBlock::Example { example_id } => {
                    if let Some(example) = cll_lookup_example(site, example_id) {
                        hrefs.extend(collect_interlinear_parse_hrefs(site, &example.blocks));
                    }
                }
                CllBlock::BlockQuote { blocks, .. } | CllBlock::Rule { body: blocks, .. } => {
                    hrefs.extend(collect_interlinear_parse_hrefs(site, blocks));
                }
                CllBlock::Table {
                    header_rows,
                    body_rows,
                    ..
                } => {
                    for row in header_rows.iter().chain(body_rows.iter()) {
                        for cell in row {
                            hrefs.extend(collect_interlinear_parse_hrefs(site, &cell.blocks));
                        }
                    }
                }
                CllBlock::VariableList { entries, .. } => {
                    for entry in entries {
                        hrefs.extend(collect_interlinear_parse_hrefs(site, &entry.blocks));
                    }
                }
                _ => {}
            }
        }
        hrefs
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn semantic_search_is_disabled() {
        let site = embedded_cll_site().expect("embedded CLL should load");
        let output = cukta_search(
            site,
            CuktaSearchMode::Meaning,
            "lojban",
            10,
            CuktaTargetFilter::default(),
        );
        assert!(output.matches.is_empty());
        assert_eq!(
            output.message.as_deref(),
            Some("Meaning search is not available yet.")
        );
    }
}
