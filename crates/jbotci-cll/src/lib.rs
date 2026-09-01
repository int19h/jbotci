//! Reference model for *The Contemporary Lojban Language*, the unofficial
//! community edition of the Lojban reference grammar that jbotci vendors and
//! that the `cukta` tool answers from. [`cll_edition`] reports which edition a
//! given build carries.

#[allow(unused_imports)]
use bityzba::{contract_trait, data, ensures, expensive_invariant, invariant, new, requires};
#[cfg(test)]
use roxmltree::Document;

pub const DEFAULT_CUKTA_CLI_RESULT_COUNT: usize = 10;
pub const DEFAULT_CUKTA_WEB_RESULT_COUNT: usize = 20;
pub const MAX_CUKTA_RESULT_COUNT: usize = 500;
pub const DEFAULT_CUKTA_SECTION_ID: &str = "section-what-is-lojban";
const PARAGRAPH_SEARCH_MIN_CHARS: usize = 200;

// The build script turns the vendored identity records into constants using
// this module; the crate compiles it only for tests, so a test can check the
// same parse the build performed instead of approximating it.
#[cfg(test)]
mod vendor_metadata;

mod import;
#[cfg(test)]
use import::{
    BlockParseState, EMBEDDED_CLL_CHAPTERS, chrestomathy_area_no_parse_rows, decode_chapter_xml,
    normalize_valsis_query, parse_block, parse_paragraph_blocks, sanitize_xml_entities,
};
pub(crate) use import::{
    PendingIndexEntry, SectionParseContext, attr_string, block_anchor_id_for, child_element,
    raw_text, visible_text, visible_text_raw, xml_id,
};
use import::{
    chrestomathy_area_groups, chrestomathy_area_label, chrestomathy_group_id,
    chrestomathy_metadata, chrestomathy_section_metadata, cll_import_metadata,
    normalized_plain_text,
};
pub use import::{cll_edition, embedded_cll_site, load_embedded_cll_site};

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
pub use search::search_chunk_kind_label;
pub use search::{
    CllSearchChunk, CllSearchChunkKind, CllSearchMatch, CuktaRequest, CuktaSearchMode,
    CuktaSearchOutput, CuktaTargetFilter, clamp_cukta_result_count, cll_search_all_chunks,
    cll_search_section_chunks, collect_tagged_words, cukta_search, cukta_word_search_matches,
    parse_word_search_terms, truncate_preview,
};
use search::{build_search_chunks, example_plain_text};

mod render;
use render::{
    push_status_note_markdown, render_block_html, render_block_markdown, render_status_note_html,
};

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

/// The chapter-level blocks displayed above `section`: the chapter's front
/// matter belongs to no section of its own, so it is shown - and searched, and
/// indexed - with the chapter's first section, and is empty for every other
/// section.
#[requires(true)]
#[ensures(
    ret.is_empty()
        || site.chapters.iter().any(|chapter| {
            chapter.root_section_ids.first() == Some(&section.section_id)
        }),
    "only a chapter's first section carries that chapter's prelude"
)]
pub fn cll_section_prelude_blocks<'a>(site: &'a CllSite, section: &CllSection) -> &'a [CllBlock] {
    site.chapters
        .iter()
        .find(|chapter| chapter.chapter_id == section.chapter_id)
        .filter(|chapter| chapter.root_section_ids.first() == Some(&section.section_id))
        .map(|chapter| chapter.prelude_blocks.as_slice())
        .unwrap_or_default()
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
    link_mode: CllLinkRenderMode,
) -> Result<String, CllError> {
    match request {
        CuktaRequest::Toc => Ok(render_toc(site, format, link_mode)),
        CuktaRequest::Index => Ok(render_index(site, format, link_mode)),
        CuktaRequest::Section { reference } => {
            let section_id = cll_resolve_section_reference(site, reference)
                .ok_or_else(|| CllError::NotFound(format!("CLL section not found: {reference}")))?;
            let section = cll_lookup_section(site, &section_id)
                .ok_or_else(|| CllError::NotFound(format!("CLL section not found: {reference}")))?;
            Ok(render_section(site, section, format, link_mode))
        }
        CuktaRequest::Example { reference } => {
            let example_id = cll_resolve_example_reference(site, reference)
                .ok_or_else(|| CllError::NotFound(format!("CLL example not found: {reference}")))?;
            let example = cll_lookup_example(site, &example_id)
                .ok_or_else(|| CllError::NotFound(format!("CLL example not found: {reference}")))?;
            Ok(render_example(site, example, format, link_mode))
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
                link_mode,
            ))
        }
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub fn render_toc(site: &CllSite, format: CllRenderFormat, link_mode: CllLinkRenderMode) -> String {
    match format {
        CllRenderFormat::Html => {
            let edition = &site.metadata.edition;
            let mut output = format!(
                "<nav class=\"cll-toc-rendered\"><h1>{}</h1><p class=\"cll-edition\">{}</p><p class=\"cll-edition-lineage\">{}</p><h2>Table of Contents</h2><ol>",
                escape_html(&edition.title),
                escape_html(&format!("{} — {}", edition.version, edition.publisher)),
                escape_html(&format!("Lineage: {}", edition.lineage())),
            );
            for chapter in &site.chapters {
                output.push_str("<li>");
                output.push_str(&escape_html(&cll_numbered_title(
                    chapter.division.chapter_number(),
                    &chapter.chapter_title,
                )));
                output.push_str("<ol>");
                for section_id in &chapter.root_section_ids {
                    let section = site
                        .sections_by_id
                        .get(section_id)
                        .expect("CllSite invariant guarantees chapter root section ids resolve");
                    output.push_str("<li>");
                    if link_mode == CllLinkRenderMode::Web {
                        output.push_str("<a href=\"");
                        output.push_str(&escape_html(&section_href(&section.section_id)));
                        output.push_str("\">");
                    }
                    output.push_str(&escape_html(&format_section_display_title(section)));
                    if link_mode == CllLinkRenderMode::Web {
                        output.push_str("</a>");
                    }
                    output.push_str("</li>");
                }
                output.push_str("</ol></li>");
            }
            output.push_str("</ol></nav>\n");
            output
        }
        CllRenderFormat::Markdown | CllRenderFormat::Raw => {
            let edition = &site.metadata.edition;
            let mut output = format!(
                "# {}\n\n{} — {}\n\nLineage: {}\n\n## Table of Contents\n\n",
                edition.title,
                edition.version,
                edition.publisher,
                edition.lineage(),
            );
            for chapter in &site.chapters {
                output.push_str(&cll_numbered_title(
                    chapter.division.chapter_number(),
                    &chapter.chapter_title,
                ));
                output.push('\n');
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
pub fn render_index(
    site: &CllSite,
    format: CllRenderFormat,
    link_mode: CllLinkRenderMode,
) -> String {
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
                        .map(|section| match link_mode {
                            CllLinkRenderMode::Web => format!(
                                "<a href=\"{}\">{}</a>",
                                escape_html(&section_href(&section.section_id)),
                                escape_html(&cll_section_index_label(section))
                            ),
                            CllLinkRenderMode::Plain => {
                                escape_html(&cll_section_index_label(section))
                            }
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
                    .map(cll_section_index_label)
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
pub fn render_section(
    site: &CllSite,
    section: &CllSection,
    format: CllRenderFormat,
    link_mode: CllLinkRenderMode,
) -> String {
    match format {
        CllRenderFormat::Html => {
            let mut output = String::new();
            output.push_str(
                "<article class=\"cll-section-content\"><div class=\"cll-section-heading\"><h1>",
            );
            output.push_str(&escape_html(&format_section_display_title(section)));
            output.push_str("</h1>");
            if link_mode == CllLinkRenderMode::Web
                && let Some(parse_href) = chrestomathy_section_parse_href(site, section)
            {
                output.push_str(
                    "<a class=\"cll-parse-example cll-parse-section spa-cll-link spa-cll-link-parse\" href=\"",
                );
                output.push_str(&escape_html(&parse_href));
                output.push_str("\">Parse</a>");
            }
            output.push_str("</div>");
            let prelude_blocks = cll_section_prelude_blocks(site, section);
            if !prelude_blocks.is_empty() {
                output.push_str("<div class=\"cll-chapter-prelude\">");
                for block in prelude_blocks {
                    output.push_str(&render_block_html(site, block, link_mode));
                }
                output.push_str("</div>");
            }
            for block in &section.blocks {
                output.push_str(&render_block_html(site, block, link_mode));
            }
            output.push_str("</article>\n");
            output
        }
        CllRenderFormat::Markdown | CllRenderFormat::Raw => {
            let mut output = format!("# {}\n\n", format_section_display_title(section));
            if link_mode == CllLinkRenderMode::Web
                && let Some(parse_href) = chrestomathy_section_parse_href(site, section)
            {
                output.push_str(&format!("[Parse]({parse_href})\n\n"));
            }
            for block in cll_section_prelude_blocks(site, section) {
                render_block_markdown(site, block, &mut output, 0, link_mode);
            }
            for block in &section.blocks {
                render_block_markdown(site, block, &mut output, 0, link_mode);
            }
            output
        }
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub fn render_example(
    site: &CllSite,
    example: &CllExample,
    format: CllRenderFormat,
    link_mode: CllLinkRenderMode,
) -> String {
    match format {
        CllRenderFormat::Html => {
            let mut output = format!(
                "<figure id=\"{}\" class=\"cll-example\"><figcaption class=\"cll-example-head\"><span class=\"cll-example-title\">{}</span>",
                escape_html(&example.anchor_id),
                escape_html(&example.label)
            );
            if link_mode == CllLinkRenderMode::Web
                && let Some(parse_href) = &example.parse_href
            {
                output.push_str(
                    "<a class=\"cll-parse-example spa-cll-link spa-cll-link-parse\" href=\"",
                );
                output.push_str(&escape_html(parse_href));
                output.push_str("\">Parse</a>");
            }
            output.push_str("</figcaption>");
            for block in &example.blocks {
                output.push_str(&render_block_html(site, block, link_mode));
            }
            output.push_str("</figure>\n");
            output
        }
        CllRenderFormat::Markdown | CllRenderFormat::Raw => {
            let mut output = format!("### {}", example.label);
            if link_mode == CllLinkRenderMode::Web
                && let Some(parse_href) = &example.parse_href
            {
                output.push_str(&format!(" [Parse]({parse_href})"));
            }
            output.push_str("\n\n");
            for block in &example.blocks {
                render_block_markdown(site, block, &mut output, 0, link_mode);
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
pub fn render_search_output(
    output: &CuktaSearchOutput,
    format: CllRenderFormat,
    _link_mode: CllLinkRenderMode,
) -> String {
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
                rendered.push_str(&escape_html(&cll_numbered_title(
                    item.chunk.section_number.as_deref(),
                    &item.chunk.section_title,
                )));
                rendered.push_str("</p>");
                let preview = escape_html(&truncate_preview(&item.chunk.text, 420));
                if item.chunk.is_status_note() {
                    rendered.push_str(&render_status_note_html(
                        "",
                        CLL_STATUS_NOTE_PREVIEW_CLASSES,
                        &preview,
                    ));
                } else {
                    rendered.push_str("<p class=\"cll-search-preview\">");
                    rendered.push_str(&preview);
                    rendered.push_str("</p>");
                }
                rendered.push_str("</article>");
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
                    "{} in {}\n\n",
                    search_chunk_kind_label(item.chunk.kind),
                    cll_numbered_title(
                        item.chunk.section_number.as_deref(),
                        &item.chunk.section_title,
                    ),
                ));
                let preview = truncate_preview(&item.chunk.text, 420);
                if item.chunk.is_status_note() {
                    push_status_note_markdown(&mut rendered, &preview);
                } else {
                    rendered.push_str(&preview);
                }
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
    cll_numbered_title(section.number, &section.title)
}

/// The compact designation an index or breadcrumb shows for a section: the
/// book's section number where it has one, and the section title where the
/// book designates the whole division by title alone.
#[requires(true)]
#[ensures(!ret.is_empty())]
pub fn cll_section_index_label(section: &CllSection) -> String {
    match section.number {
        Some(number) => number.to_string(),
        None => section.title.clone(),
    }
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
                let column_count = headers
                    .len()
                    .max(rows.iter().map(Vec::len).max().unwrap_or(0));
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
    use std::collections::{BTreeMap, BTreeSet};
    use std::num::{NonZeroU16, NonZeroUsize};

    use super::*;
    #[allow(unused_imports)]
    use bityzba::{contract_trait, ensures, invariant, new, requires};
    use jbotci_morphology::segment_words_with_modifiers;
    use jbotci_syntax::{ParseOptions, parse_syntax_tree_with_options};
    use sha2::{Digest, Sha256};

    /// The vendored records are the authority for the book's identity, so the
    /// reported edition has to be exactly what they parse to — checked through
    /// the same parser the build used, not through substring matches that would
    /// bake in the current quoting style.
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn edition_is_taken_from_the_vendored_sources() {
        const ENV_NAME: &str = "vendor/cll/.env";
        const PIN_NAME: &str = "vendor/cll.VENDORED_FROM";
        let vendored_env = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../vendor/cll/.env"
        ));
        let vendored_pin = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../vendor/cll.VENDORED_FROM"
        ));
        // The separators are literal tokens: `.env` records are written
        // `KEY=VALUE` and the pin record `key: value`, and the space after the
        // colon is part of the separator rather than padding to discard.
        let env = vendor_metadata::parse_key_value_file(vendored_env, "=", ENV_NAME)
            .expect("the vendored .env should parse");
        let pin = vendor_metadata::parse_key_value_file(vendored_pin, ": ", PIN_NAME)
            .expect("the vendored pin record should parse");
        let field = |fields: &[(String, String)], key: &str, source: &str| {
            vendor_metadata::required_field(fields, key, source)
                .expect("required vendored field")
                .to_owned()
        };
        let edition = cll_edition();

        assert_eq!(edition.title, field(&env, "TITLE", ENV_NAME));
        assert_eq!(edition.version, field(&env, "VERSION", ENV_NAME));
        assert_eq!(edition.publisher, field(&env, "PUBLISHER", ENV_NAME));
        assert_eq!(edition.upstream_url, field(&pin, "upstream-url", PIN_NAME));
        assert_eq!(edition.release_tag, field(&pin, "release-tag", PIN_NAME));
        assert_eq!(edition.commit, field(&pin, "commit", PIN_NAME));
        assert!(
            vendor_metadata::check_version_matches_release_tag(
                &edition.version,
                &edition.release_tag,
            )
            .is_ok()
        );
        assert_eq!(
            embedded_cll_site()
                .expect("embedded CLL should load")
                .metadata
                .edition,
            *edition,
        );
    }

    /// The vendored records fix the reported edition, so the parser must refuse
    /// every shape it does not actually interpret rather than guess.
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn vendored_metadata_parser_rejects_uninterpreted_shapes() {
        let parse = |text: &str| vendor_metadata::parse_key_value_file(text, "=", "test");

        assert!(parse("A=one\n# comment\n\nB=two\r\n").is_ok());
        assert!(parse("A=one\nA=two\n").is_err(), "duplicate key");
        assert!(parse("A='one'\n").is_err(), "single-quoted value");
        assert!(parse("A=\"one\n").is_err(), "unterminated quote");
        assert!(parse("A=one # trailing\n").is_err(), "inline comment");
        assert!(parse("A=one\\ntwo\n").is_err(), "backslash escape");
        assert!(parse("A=\n").is_err(), "empty value");
        assert!(parse("A=\"   \"\n").is_err(), "quoted whitespace value");
        assert!(parse("export A=one\n").is_err(), "unsupported key syntax");
        assert!(parse("no separator here\n").is_err(), "missing separator");
        // Whitespace adjoining the separator is not padding to be discarded: a
        // record spelled any way but exactly `KEY=VALUE` is one this module
        // does not interpret, so it is an error rather than a normalization.
        assert!(parse("A =one\n").is_err(), "padded key");
        assert!(parse("A= one\n").is_err(), "value padded before");
        assert!(parse("A=one \n").is_err(), "value padded after");
        assert!(parse("A= \"one\"\n").is_err(), "padded quoted value");
        assert!(parse("  A=one\n").is_err(), "indented record");
        assert_eq!(
            parse("A=\"one\"\nB=two\n")
                .expect("canonical values")
                .into_iter()
                .map(|(_, value)| value)
                .collect::<Vec<_>>(),
            vec!["one".to_owned(), "two".to_owned()],
        );

        // The pin record's separator carries its own space, so the same rules
        // apply to the spelling that file actually uses.
        let parse_pin = |text: &str| vendor_metadata::parse_key_value_file(text, ": ", "test");
        assert_eq!(
            parse_pin("upstream-url: https://example.invalid/cll\n")
                .expect("the pin spelling should parse")
                .into_iter()
                .map(|(key, value)| (key, value))
                .collect::<Vec<_>>(),
            vec![(
                "upstream-url".to_owned(),
                "https://example.invalid/cll".to_owned(),
            )],
        );
        assert!(
            parse_pin("commit:abc\n").is_err(),
            "separator space missing"
        );
        assert!(parse_pin("commit:  abc\n").is_err(), "value padded before");
    }

    /// The pin record and the book's own version must name the same release in
    /// the exact documented shape.
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn edition_version_must_match_the_pinned_release_tag_exactly() {
        let check = vendor_metadata::check_version_matches_release_tag;

        assert!(check("colojban-1.3.2", "v1.3.2").is_ok());
        assert!(check("colojban-1.3.3", "v1.3.3").is_ok());
        assert!(check("1.3.2", "v1.3.2").is_err(), "no edition prefix");
        assert!(check("colojban-1.3.2", "1.3.2").is_err(), "tag without v");
        assert!(check("-1.3.2", "v1.3.2").is_err(), "empty edition prefix");
        assert!(check("colojban1.3.2", "v1.3.2").is_err(), "missing hyphen");
        assert!(check("colojban-1.3.2", "v1.3.3").is_err(), "version drift");
        assert!(check("colojban-1.3.2", "v").is_err(), "empty tag version");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn edition_lineage_ends_at_the_vendored_edition() {
        let edition = cll_edition();
        let lineage = edition.lineage();

        for ancestor in &edition.ancestry {
            assert!(lineage.contains(&ancestor.title));
            assert!(lineage.contains(&ancestor.version));
        }
        assert!(lineage.ends_with(&edition.version));
        assert!(!edition.display_title().is_empty());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn paragraph_roles_separate_status_notes_from_presentation() {
        let status_note = CllParagraphRole::parse("status-note").expect("role should parse");
        let indent = CllParagraphRole::parse("indent").expect("role should parse");

        assert!(status_note.is_status_note());
        assert_eq!(status_note.presentation_name(), None);
        assert!(!indent.is_status_note());
        assert_eq!(indent.presentation_name(), Some("indent"));
        assert_eq!(CllParagraphRole::parse("   "), None);
    }

    /// Counts `<para role="status-note">` elements in the embedded chapter XML.
    /// This is the source-side oracle for the import: it is derived from the
    /// vendored text itself, so a newer vendored edition moves it automatically
    /// instead of leaving a hardcoded release count behind.
    #[requires(true)]
    #[ensures(true)]
    fn source_status_note_count() -> usize {
        let mut total = 0;
        for (source_path, _chapter_number, compressed) in EMBEDDED_CLL_CHAPTERS {
            let xml = decode_chapter_xml(compressed).expect("embedded chapter should decode");
            let xml = sanitize_xml_entities(&xml);
            let document = Document::parse(&xml)
                .unwrap_or_else(|error| panic!("{source_path} should parse: {error}"));
            total += document
                .descendants()
                .filter(|node| {
                    node.is_element()
                        && node.has_tag_name("para")
                        && node
                            .attribute("role")
                            .is_some_and(|role| role.trim().eq_ignore_ascii_case("status-note"))
                })
                .count();
        }
        total
    }

    /// Counts typed status-note designations anywhere in a block tree, so
    /// notes nested in lists, tables, block quotes, or admonitions are counted
    /// the same as top-level ones.
    #[invariant(true)]
    struct StatusNoteCountVisitor {
        count: usize,
    }

    #[contract_trait]
    impl CllBlockVisitor for StatusNoteCountVisitor {
        #[requires(true)]
        #[ensures(true)]
        fn visit_block(&mut self, block: &CllBlock) {
            if let CllBlock::Paragraph {
                role: Some(role), ..
            } = block
                && role.is_status_note()
            {
                self.count += 1;
            }
            walk_block(self, block);
        }
    }

    /// The status notes are the vendored edition's headline feature, so every
    /// one of them has to survive import as a typed designation rather than as
    /// prose — not merely one of them.
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn every_source_status_note_is_imported_as_a_typed_designation() {
        let site = embedded_cll_site().expect("embedded CLL should load");
        let mut visitor = StatusNoteCountVisitor { count: 0 };
        for chapter in &site.chapters {
            visitor.visit_blocks(&chapter.prelude_blocks);
        }
        for section in site.sections_by_id.values() {
            visitor.visit_blocks(&section.blocks);
        }
        // Example bodies are reached by id from the blocks above rather than by
        // descent, so they are walked separately.
        for example in site.examples_by_id.values() {
            visitor.visit_blocks(&example.blocks);
        }

        let source_count = source_status_note_count();
        assert!(source_count > 0, "the vendored edition marks moved rules");
        // Exact equality, not a floor: a `<para role="status-note">` that wraps
        // a block child would import as several paragraphs sharing the role,
        // which is worth noticing rather than silently accepting.
        assert_eq!(
            visitor.count, source_count,
            "every source status note should import as exactly one typed designation"
        );
    }

    /// A status note must be visually set off wherever it is rendered, in every
    /// transport, including the ones that carry no stylesheet.
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn status_notes_render_distinctly_in_sections() {
        let site = embedded_cll_site().expect("embedded CLL should load");
        let section = site
            .sections_by_id
            .values()
            .find(|section| {
                section.blocks.iter().any(|block| {
                    matches!(
                        block,
                        CllBlock::Paragraph { role: Some(role), .. } if role.is_status_note()
                    )
                })
            })
            .expect("some section teaches a rule that carries a status note");

        let markdown = render_section(
            site,
            section,
            CllRenderFormat::Markdown,
            CllLinkRenderMode::Plain,
        );
        let html = render_section(site, section, CllRenderFormat::Html, CllLinkRenderMode::Web);

        assert!(markdown.contains(&format!("> **{CLL_STATUS_NOTE_LABEL}.** ")));
        assert!(html.contains("<aside"));
        assert!(html.contains("class=\"cll-para cll-status-note\""));
        assert!(html.contains(&format!(
            "<span class=\"cll-status-note-label\">{CLL_STATUS_NOTE_LABEL}</span>"
        )));
        assert!(!html.contains("cll-para-status-note"));
    }

    /// Search is `cukta`'s default query path, so a status-note hit has to keep
    /// its designation through the search projection and be set off in the
    /// rendered results exactly as it is when its section is read.
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn status_note_search_hits_keep_and_render_their_designation() {
        let site = embedded_cll_site().expect("embedded CLL should load");
        let status_note_chunks = cll_search_all_chunks(site)
            .iter()
            .filter(|chunk| {
                chunk
                    .role
                    .as_ref()
                    .is_some_and(CllParagraphRole::is_status_note)
            })
            .collect::<Vec<_>>();
        assert!(
            !status_note_chunks.is_empty(),
            "long status notes should be searchable paragraph chunks"
        );
        assert!(
            status_note_chunks
                .iter()
                .all(|chunk| chunk.kind == CllSearchChunkKind::Paragraph),
            "only paragraph chunks can carry a paragraph designation"
        );

        let output = CuktaSearchOutput {
            mode: CuktaSearchMode::Word,
            query: "test".to_owned(),
            count: 1,
            matches: vec![CllSearchMatch {
                rank: 1,
                similarity: Some(0.5),
                chunk: (*status_note_chunks[0]).clone(),
            }],
            message: None,
            has_more: false,
        };

        let markdown =
            render_search_output(&output, CllRenderFormat::Markdown, CllLinkRenderMode::Plain);
        let html = render_search_output(&output, CllRenderFormat::Html, CllLinkRenderMode::Plain);

        assert!(markdown.contains(&format!("> **{CLL_STATUS_NOTE_LABEL}.** ")));
        assert!(html.contains("class=\"cll-search-preview cll-status-note\""));
        assert!(html.contains(&format!(
            "<span class=\"cll-status-note-label\">{CLL_STATUS_NOTE_LABEL}</span>"
        )));
    }

    /// An ordinary paragraph hit must not pick up the rule-status treatment.
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn ordinary_search_hits_are_not_labelled_as_status_notes() {
        let site = embedded_cll_site().expect("embedded CLL should load");
        let plain_chunk = cll_search_all_chunks(site)
            .iter()
            .find(|chunk| {
                chunk.kind == CllSearchChunkKind::Paragraph
                    && !chunk
                        .role
                        .as_ref()
                        .is_some_and(CllParagraphRole::is_status_note)
            })
            .expect("the corpus has ordinary paragraph chunks");

        let output = CuktaSearchOutput {
            mode: CuktaSearchMode::Word,
            query: "test".to_owned(),
            count: 1,
            matches: vec![CllSearchMatch {
                rank: 1,
                similarity: None,
                chunk: plain_chunk.clone(),
            }],
            message: None,
            has_more: false,
        };

        let markdown =
            render_search_output(&output, CllRenderFormat::Markdown, CllLinkRenderMode::Plain);
        let html = render_search_output(&output, CllRenderFormat::Html, CllLinkRenderMode::Plain);

        assert!(!markdown.contains(CLL_STATUS_NOTE_LABEL));
        assert!(!html.contains(CLL_STATUS_NOTE_LABEL));
        assert!(html.contains("class=\"cll-search-preview\""));
    }

    /// The presentational role is a canonical name by construction, so serde
    /// cannot smuggle in a value that shadows the status-note designation.
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn presentational_roles_reject_non_canonical_names_through_serde() {
        let accepted =
            serde_json::from_str::<CllParagraphRole>(r#"{"presentation":{"name":"indent"}}"#)
                .expect("a canonical presentational role should deserialize");
        assert_eq!(accepted.presentation_name(), Some("indent"));
        assert!(
            serde_json::from_str::<CllParagraphRole>("\"status-note\"")
                .expect("the status-note designation should deserialize")
                .is_status_note()
        );

        for rejected in [
            r#"{"presentation":{"name":" status-note "}}"#,
            r#"{"presentation":{"name":"status-note"}}"#,
            r#"{"presentation":{"name":"STATUS-NOTE"}}"#,
            r#"{"presentation":{"name":"   "}}"#,
            r#"{"presentation":{"name":""}}"#,
            r#"{"presentation":{"name":" indent"}}"#,
        ] {
            assert!(
                serde_json::from_str::<CllParagraphRole>(rejected).is_err(),
                "{rejected} should not deserialize into a presentational role"
            );
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn toc_names_the_edition_it_renders() {
        let site = embedded_cll_site().expect("embedded CLL should load");
        let edition = &site.metadata.edition;

        let markdown = render_toc(site, CllRenderFormat::Markdown, CllLinkRenderMode::Plain);
        assert!(markdown.starts_with(&format!("# {}\n", edition.title)));
        assert!(markdown.contains(&edition.version));
        assert!(markdown.contains(&edition.publisher));
        assert!(markdown.contains(&format!("Lineage: {}", edition.lineage())));

        let html = render_toc(site, CllRenderFormat::Html, CllLinkRenderMode::Web);
        assert!(html.contains(&escape_html(&edition.title)));
        assert!(html.contains(&escape_html(&edition.publisher)));
        assert!(html.contains("<h2>Table of Contents</h2>"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn embedded_site_loads_default_section() {
        let site = embedded_cll_site().expect("embedded CLL should load");
        let section = cll_lookup_section(site, DEFAULT_CUKTA_SECTION_ID)
            .expect("default section should exist");
        assert_eq!(
            section.number.map(|number| number.to_string()).as_deref(),
            Some("1.1")
        );
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
            "section/section-grammars-introduction#chapter-grammars"
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
        render_block_markdown(site, block, &mut block_markdown, 0, CllLinkRenderMode::Web);

        assert_eq!(
            block_markdown,
            render_example(
                site,
                example,
                CllRenderFormat::Markdown,
                CllLinkRenderMode::Web,
            )
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn plain_markdown_removes_routes_and_parse_actions_but_keeps_content() {
        let site = embedded_cll_site().expect("embedded CLL should load");
        let cases = [
            ("9.6", "mi", "| mi | viska | do | sepi'o |"),
            ("section-EBNF", "BRIVLA", "**text** ⩴"),
            (
                "2.1",
                "bridi",
                "bridi (predication) ______________|__________________",
            ),
            ("1.8", "llg-board@lojban.org", "http://www.lojban.org"),
        ];

        for (reference, preserved_word, preserved_structure) in cases {
            let section_id = cll_resolve_section_reference(site, reference)
                .unwrap_or_else(|| panic!("section {reference} should resolve"));
            let section =
                cll_lookup_section(site, &section_id).expect("resolved section should exist");
            let rendered = render_section(
                site,
                section,
                CllRenderFormat::Markdown,
                CllLinkRenderMode::Plain,
            );

            assert!(
                !rendered.contains("]("),
                "{reference} retained Markdown link syntax:\n{rendered}"
            );
            assert!(
                !rendered.contains("Parse"),
                "{reference} retained a Parse artifact:\n{rendered}"
            );
            assert!(
                rendered.contains(preserved_word),
                "{reference} lost linked content `{preserved_word}`:\n{rendered}"
            );
            assert!(
                rendered.contains(preserved_structure),
                "{reference} lost representative structure `{preserved_structure}`:\n{rendered}"
            );
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn plain_html_removes_links_and_parse_actions_but_keeps_markup() {
        let site = embedded_cll_site().expect("embedded CLL should load");
        let section_id =
            cll_resolve_section_reference(site, "9.6").expect("section 9.6 should resolve");
        let section = cll_lookup_section(site, &section_id).expect("resolved section should exist");
        let html = render_section(
            site,
            section,
            CllRenderFormat::Html,
            CllLinkRenderMode::Plain,
        );

        assert!(!html.contains("<a "), "{html}");
        assert!(!html.contains("Parse"), "{html}");
        assert!(html.contains("<table"), "{html}");
        assert!(html.contains(">mi<"), "{html}");

        let toc = render_toc(site, CllRenderFormat::Html, CllLinkRenderMode::Plain);
        let index = render_index(site, CllRenderFormat::Html, CllLinkRenderMode::Plain);
        assert!(!toc.contains("<a "), "{toc}");
        assert!(!index.contains("<a "), "{index}");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn plain_html_media_keeps_descriptions_and_markup_without_asset_routes() {
        let site = embedded_cll_site().expect("embedded CLL should load");
        let block = CllBlock::Media {
            id: Some("diagram".to_owned()),
            title: Some(vec![CllInline::Emphasis {
                language: None,
                inlines: vec![CllInline::Text("Diagram title".to_owned())],
            }]),
            src: "assets/media/dead-spa-route.svg".to_owned(),
            alt: "Meaningful diagram description".to_owned(),
        };
        let html = render_block_html(site, &block, CllLinkRenderMode::Plain);

        assert_eq!(
            html,
            "<figure id=\"diagram\" class=\"cll-media\"><p class=\"cll-media-alt\">Meaningful diagram description</p><figcaption><em>Diagram title</em></figcaption></figure>"
        );
        assert!(!html.contains("<img"), "{html}");
        assert!(!html.contains("src="), "{html}");
        assert!(!html.contains("dead-spa-route.svg"), "{html}");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn plain_link_disposition_is_exhaustive_for_inline_link_kinds() {
        let site = embedded_cll_site().expect("embedded CLL should load");
        let cases = [
            (CllLinkKind::Section, true),
            (CllLinkKind::Example, true),
            (CllLinkKind::Dictionary, true),
            (CllLinkKind::Rafsi, true),
            (CllLinkKind::Parse, false),
            (CllLinkKind::Asset, true),
            (CllLinkKind::External, true),
        ];

        for (kind, keeps_content) in cases {
            let block = CllBlock::Paragraph {
                anchor_id: None,
                role: None,
                inlines: vec![CllInline::Link {
                    target: "target".to_owned(),
                    inlines: vec![CllInline::Emphasis {
                        language: None,
                        inlines: vec![CllInline::Text("linked content".to_owned())],
                    }],
                    kind,
                }],
                text: "linked content".to_owned(),
            };
            let mut markdown = String::new();
            render_block_markdown(site, &block, &mut markdown, 0, CllLinkRenderMode::Plain);
            let html = render_block_html(site, &block, CllLinkRenderMode::Plain);

            assert_eq!(markdown.contains("*linked content*"), keeps_content);
            assert_eq!(html.contains("<em>linked content</em>"), keeps_content);
            assert!(!markdown.contains("]("), "{kind:?}: {markdown}");
            assert!(!html.contains("<a "), "{kind:?}: {html}");
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn web_markdown_matches_issue_655_pre_change_baseline_hashes() {
        let site = embedded_cll_site().expect("embedded CLL should load");
        let cases = [
            (
                "1.8",
                "431f3bacf76041930750732951d356e792574c85f31a41e717e7aab6abfe1710",
            ),
            (
                "2.1",
                "d2c62de569f7761ee9119d7d23541c853d2ea004517db9020c85e87e8b3d2655",
            ),
            (
                "9.6",
                "93e8a2ea9d7b806413d17cccf07996d569bffccf09b1911f5d682d670c2c92e7",
            ),
            (
                "section-EBNF",
                "0685302ea249fbcaaedef565e917d6866bdafc362e41440c235603f8ed3599aa",
            ),
        ];

        for (reference, expected_sha256) in cases {
            let section_id = cll_resolve_section_reference(site, reference)
                .unwrap_or_else(|| panic!("section {reference} should resolve"));
            let section =
                cll_lookup_section(site, &section_id).expect("resolved section should exist");
            let rendered = render_section(
                site,
                section,
                CllRenderFormat::Markdown,
                CllLinkRenderMode::Web,
            );

            assert_eq!(sha256_hex(&rendered), expected_sha256, "{reference}");
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn web_html_matches_issue_655_pre_change_baseline_hashes() {
        let site = embedded_cll_site().expect("embedded CLL should load");
        let cases = [
            (
                "1.8",
                "d2eb92b7063ce00201b4ab4b7c4a76ba8dded192b74a5ccad84f29082a5d4366",
            ),
            (
                "2.1",
                "99e79b0d0171b7f134309d5dd94fdcd2b129c36dea1587bbc4c38f2bcaa68351",
            ),
            (
                "9.6",
                "d8f3a4ecca12a8da76ce60964006413dc6c1cc00b35addd214c476c29964aeb2",
            ),
            (
                "section-EBNF",
                "a7aa9ac42cbf574aaeaee9c0ad0ba33771099ca3f7804c84723c2e845608ee5a",
            ),
        ];

        for (reference, expected_sha256) in cases {
            let section_id = cll_resolve_section_reference(site, reference)
                .unwrap_or_else(|| panic!("section {reference} should resolve"));
            let section =
                cll_lookup_section(site, &section_id).expect("resolved section should exist");
            let rendered =
                render_section(site, section, CllRenderFormat::Html, CllLinkRenderMode::Web);

            assert_eq!(sha256_hex(&rendered), expected_sha256, "{reference}");
        }
    }

    #[requires(true)]
    #[ensures(ret.len() == 64)]
    fn sha256_hex(text: &str) -> String {
        format!("{:x}", Sha256::digest(text.as_bytes()))
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
                    && chunk.section_number.as_deref() == Some("1.3")
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

        // The chrestomathy is an appendix: the book gives it a title, not a
        // chapter number, and this edition's real chapter 22 no longer collides
        // with the number the old scheme would have synthesized for it.
        assert_eq!(chrestomathy.division, CllDivision::Appendix);
        assert_eq!(chrestomathy.chapter_title, "Chrestomathy");
        let ebnf = cll_lookup_section(site, &metadata.ebnf_section_id)
            .expect("metadata EBNF section should exist");
        assert_eq!(
            ebnf.number.map(|number| number.to_string()).as_deref(),
            Some("21.2")
        );
        assert_eq!(
            ebnf.blocks
                .iter()
                .filter_map(|block| match block {
                    CllBlock::Ebnf { entries, .. } => Some(entries.len()),
                    _ => None,
                })
                .sum::<usize>(),
            92
        );
        let expected_symbols = [
            ("BRIVLA", "section-morphology-brivla"),
            ("CMEVLA", "section-cmevla"),
            ("any-word", "section-more-quotations"),
            ("anything", "section-more-quotations"),
            ("null", "section-erasure"),
        ];
        assert_eq!(metadata.ebnf_symbols.len(), expected_symbols.len());
        for (symbol, section_id) in expected_symbols {
            assert_eq!(
                metadata.ebnf_symbols.get(symbol).map(String::as_str),
                Some(section_id)
            );
            assert!(cll_lookup_section(site, section_id).is_some());
            assert_eq!(ebnf_symbol_href(symbol), Some(section_href(section_id)));
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn peg_morphology_appendix_imports_as_a_sectioned_appendix() {
        // colojban 1.3.4 breaks the PEG word-form grammar out of a single
        // program listing into thirteen numbered-in-the-book-by-nothing
        // sections. It is the first appendix to have sections at all, so this
        // pins that the sectioned path stays appendix-shaped: real sections,
        // none of them carrying a number.
        let site = embedded_cll_site().expect("embedded CLL should load");
        let chapter = site
            .chapters
            .iter()
            .find(|chapter| chapter.chapter_id == "appendix-peg-morphology")
            .expect("the PEG morphology appendix should be imported");
        assert_eq!(chapter.division, CllDivision::Appendix);
        assert_eq!(
            chapter.root_section_ids,
            [
                "a02-classes",
                "a02-words",
                "a02-cmevla",
                "a02-cmavo",
                "a02-brivla",
                "a02-fuhivla",
                "a02-gismu",
                "a02-syllables",
                "a02-vowels",
                "a02-consonants",
                "a02-boundaries",
                "a02-spaces",
                "a02-selmaho",
            ]
        );
        assert!(
            cll_lookup_section(site, "appendix-peg-morphology").is_none(),
            "an appendix with sections of its own gets no synthetic root section"
        );

        // Every grammar rule in the appendix survives import, and each one is a
        // `varlistentry` in one of the appendix's variable lists.
        let mut entries = 0usize;
        for section_id in &chapter.root_section_ids {
            let section = cll_lookup_section(site, section_id).expect("a02 section should exist");
            assert_eq!(section.division, CllDivision::Appendix);
            assert_eq!(
                section.number, None,
                "{section_id} is in an appendix and must carry no number"
            );
            assert_eq!(section.chapter_id, "appendix-peg-morphology");
            assert_eq!(section.source_path, "a02.xml");
            entries += section
                .blocks
                .iter()
                .filter_map(|block| match block {
                    CllBlock::VariableList { entries, .. } => Some(entries.len()),
                    _ => None,
                })
                .sum::<usize>();
            assert!(
                !section
                    .blocks
                    .iter()
                    .any(|block| matches!(block, CllBlock::Ebnf { .. })),
                "{section_id} is PEG, not the chapter 21 EBNF, so it must not be tokenized as EBNF"
            );
        }
        assert_eq!(entries, 236);

        // Chapter 21's EBNF section is the one variable list that gets the
        // bespoke EBNF treatment; every other variable list in the book, the
        // appendix's included, renders as a definition list.
        let ebnf = cll_lookup_section(site, &cll_import_metadata().ebnf_section_id)
            .expect("the EBNF section should exist");
        assert!(
            ebnf.blocks
                .iter()
                .any(|block| matches!(block, CllBlock::Ebnf { .. }))
        );
        assert!(
            !ebnf
                .blocks
                .iter()
                .any(|block| matches!(block, CllBlock::VariableList { .. }))
        );

        // The appendix is addressable through cukta by any of its section ids,
        // and a cross-reference to the appendix as a whole lands on its first
        // section under the title the book gives it.
        let rendered = render_cukta_request(
            site,
            &CuktaRequest::Section {
                reference: "a02-cmevla".to_owned(),
            },
            CllRenderFormat::Markdown,
            CllLinkRenderMode::Plain,
        )
        .expect("a PEG morphology section should render through cukta");
        assert!(rendered.starts_with("# cmevla\n"));
        assert!(rendered.contains("**zifcme ←**\n\n!h (nucleus / glide / h / consonant !pause"));
        assert_eq!(
            site.anchors_by_id
                .get("appendix-peg-morphology")
                .map(|anchor| (anchor.section_id.as_str(), anchor.label.as_str())),
            Some(("a02-classes", "The PEG word-form grammar"))
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn chapter_front_matter_is_parsed_and_addressed_as_first_section_content() {
        // Content that sits outside every section belongs to the chapter, but
        // the reader only ever meets it above the chapter's first section. It
        // goes through the ordinary block pipeline, so its cross-references,
        // lists, and inline markup survive, and everything it contributes -
        // search chunks and index entries alike - names that first section.
        let site = embedded_cll_site().expect("embedded CLL should load");
        let appendix = site
            .chapters
            .iter()
            .find(|chapter| chapter.chapter_id == "appendix-peg-morphology")
            .expect("the PEG morphology appendix should be imported");
        let first_section =
            cll_lookup_section(site, "a02-classes").expect("a02's first section should exist");
        assert_eq!(
            cll_section_prelude_blocks(site, first_section),
            appendix.prelude_blocks
        );
        assert!(
            cll_lookup_section(site, "a02-selmaho")
                .is_some_and(|section| cll_section_prelude_blocks(site, section).is_empty()),
            "only the first section of a chapter shows that chapter's front matter"
        );

        // The appendix opens with prose and closes its front matter with a
        // bulleted key to the PEG notation.
        let (paragraphs, lists): (Vec<_>, Vec<_>) = appendix
            .prelude_blocks
            .iter()
            .partition(|block| matches!(block, CllBlock::Paragraph { .. }));
        assert_eq!(paragraphs.len(), 5);
        assert_eq!(lists.len(), 1);
        assert!(matches!(lists[0], CllBlock::List { .. }));

        // The opening paragraph's two cross-references into the numbered
        // chapters survive as links, rather than being flattened away.
        let opening = paragraphs
            .first()
            .expect("the appendix opens with a paragraph");
        let CllBlock::Paragraph { inlines, .. } = opening else {
            unreachable!("partitioned on Paragraph")
        };
        assert_eq!(
            inlines
                .iter()
                .filter_map(|inline| match inline {
                    CllInline::Link { target, .. } => Some(target.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>(),
            ["chapter-morphology", "chapter-phonology"]
        );

        // cukta renders the front matter above the first section, exactly where
        // the web shows it, and searching finds it under that section.
        let rendered = render_section(
            site,
            first_section,
            CllRenderFormat::Markdown,
            CllLinkRenderMode::Plain,
        );
        assert!(rendered.starts_with("# Word classes\n\nThis appendix reproduces"));
        assert!(rendered.contains("\n**CMEVLA ←**\n"));
        assert!(site.search_chunks.iter().any(|chunk| {
            chunk.section_id == "a02-classes"
                && chunk.kind == CllSearchChunkKind::Paragraph
                && chunk
                    .text
                    .starts_with("A parsing expression grammar differs")
        }));
        assert!(
            cll_index_entries(site)
                .iter()
                .find(|entry| entry.key == "slinku'i test; formal statement of")
                .is_some_and(|entry| entry.section_ids == ["a02-classes"]),
            "an index term in a chapter's front matter is indexed at its first section"
        );

        // A numbered chapter's front matter is its illustration, and it keeps
        // being modelled as media rather than as flattened alt text.
        let tour = site
            .chapters
            .iter()
            .find(|chapter| chapter.chapter_id == "chapter-tour")
            .expect("chapter 2 should be imported");
        assert!(matches!(
            tour.prelude_blocks.as_slice(),
            [CllBlock::Media { .. }]
        ));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn appendices_are_designated_by_title_rather_than_by_a_chapter_number() {
        let site = embedded_cll_site().expect("embedded CLL should load");

        // Every division after the last numbered chapter is an appendix, and no
        // numbered chapter is misclassified as one.
        let numbered = site
            .chapters
            .iter()
            .filter(|chapter| chapter.division.chapter_number().is_some())
            .count();
        assert_eq!(numbered, 22);
        assert!(
            site.chapters[numbered..]
                .iter()
                .all(|chapter| chapter.division == CllDivision::Appendix)
        );
        assert_eq!(
            site.chapters[numbered..]
                .iter()
                .map(|chapter| chapter.chapter_title.as_str())
                .collect::<Vec<_>>(),
            [
                "Chrestomathy",
                "The PEG word-form grammar",
                "Changes from the first edition",
            ]
        );

        // A cross-reference to an appendix renders the appendix title, the only
        // designation the book gives it, rather than a synthesized chapter
        // number that would have collided with the real chapter 22.
        let rafsi_for_fuivla = cll_lookup_section(site, "section-rafsi-fuhivla")
            .expect("chapter 4's fu'ivla rafsi section should exist");
        let rendered = render_section(
            site,
            rafsi_for_fuivla,
            CllRenderFormat::Markdown,
            CllLinkRenderMode::Plain,
        );
        assert!(rendered.contains("printed in The PEG word-form grammar"));
        assert!(rendered.contains("see Changes from the first edition"));
        assert!(!rendered.contains("Chapter 24"));
        assert!(!rendered.contains("Chapter 25"));

        // Appendix sections are addressed by their stable id, and no positional
        // number is registered for them.
        assert_eq!(
            cll_resolve_section_reference(site, "section-north-wind").as_deref(),
            Some("section-north-wind")
        );
        assert_eq!(cll_resolve_section_reference(site, "23.1"), None);
        assert_eq!(cll_resolve_section_reference(site, "24"), None);
        assert_eq!(cll_resolve_section_reference(site, "25.1"), None);
        assert_eq!(
            cll_resolve_section_reference(site, "22.1").as_deref(),
            Some("section-dialects-introduction"),
            "the real chapter 22 keeps the numbers the old scheme handed to the chrestomathy"
        );

        // Display and index labels drop the number rather than inventing one.
        let north_wind =
            cll_lookup_section(site, "section-north-wind").expect("chrestomathy section exists");
        assert_eq!(
            format_section_display_title(north_wind),
            "The North Wind and the Sun"
        );
        assert_eq!(
            cll_section_index_label(north_wind),
            "The North Wind and the Sun"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn section_numbers_must_name_their_own_chapter() {
        // A section number is parsed from - and printed as - the string the
        // book prints, and it carries its own chapter.
        let six_three: CllSectionNumber = "6.3".parse().expect("6.3 is a section number");
        assert_eq!(six_three.to_string(), "6.3");
        assert_eq!(six_three.chapter().get(), 6);
        let twenty: CllSectionNumber = "20".parse().expect("20 is a whole-chapter number");
        assert_eq!(twenty.to_string(), "20");
        assert_eq!(twenty.chapter().get(), 20);
        for text in ["", "6.", ".3", "6.0", "0.3", "6.3.1", "6.x", "a01"] {
            assert!(
                text.parse::<CllSectionNumber>().is_err(),
                "`{text}` is not a section number"
            );
        }

        // Rust's integer parsers accept a leading `+` and leading zeroes, so
        // these all parse to numbers that print as `6.3` or `20`. A section
        // number is the exact string the book prints, so a spelling that does
        // not round-trip is rejected rather than silently canonicalized - and
        // rejected through serde too, which is where untrusted text arrives.
        for text in ["06.3", "+6.3", "6.+3", "6.03", "020", "+20", "6.3 "] {
            assert!(
                text.parse::<CllSectionNumber>().is_err(),
                "`{text}` does not print back as itself and is not a section number"
            );
            assert!(
                serde_json::from_str::<CllSectionNumber>(&format!("{text:?}")).is_err(),
                "serde should reject `{text}`"
            );
        }

        // The canonical spellings round-trip through serde unchanged.
        for text in ["6.3", "20", "21.17"] {
            let number = serde_json::from_str::<CllSectionNumber>(&format!("{text:?}"))
                .unwrap_or_else(|error| panic!("`{text}` should deserialize: {error}"));
            assert_eq!(number.to_string(), text);
            assert_eq!(
                serde_json::to_string(&number).expect("section numbers serialize"),
                format!("{text:?}")
            );
        }

        // A section of chapter 6 cannot claim chapter 22's number, and an
        // appendix section cannot claim any number - through serde either.
        let section = |division: &str, number: &str| {
            serde_json::from_str::<CllSection>(&format!(
                r#"{{"section_id":"s","chapter_id":"c","division":{division},"number":{number},
                    "title":"t","parent_section_id":null,"child_section_ids":[],"blocks":[],
                    "source_path":"06.xml","plain_text":""}}"#
            ))
        };
        let chapter_six = r#"{"chapter":{"number":6}}"#;
        assert!(section(chapter_six, r#""6.3""#).is_ok());
        assert!(section(chapter_six, r#""6""#).is_ok());
        assert!(section(r#""appendix""#, "null").is_ok());
        assert!(
            section(chapter_six, r#""22.1""#).is_err(),
            "a chapter 6 section must not carry chapter 22's number"
        );
        assert!(
            section(chapter_six, "null").is_err(),
            "a numbered chapter's section always carries a number"
        );
        assert!(
            section(r#""appendix""#, r#""23.1""#).is_err(),
            "an appendix section carries no number at all"
        );
        assert!(
            section(chapter_six, r#""6.3.1""#).is_err(),
            "a malformed section number is rejected while parsing"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn restored_ebnf_cross_reference_links_to_rendered_rules() {
        let site = embedded_cll_site().expect("embedded CLL should load");
        let cross_reference = cll_lookup_section(site, "section-cross-reference")
            .expect("restored EBNF cross-reference section should exist");
        assert_eq!(
            cross_reference
                .number
                .map(|number| number.to_string())
                .as_deref(),
            Some("21.3")
        );
        assert_eq!(
            cross_reference
                .blocks
                .iter()
                .filter_map(|block| match block {
                    CllBlock::VariableList { entries, .. } => Some(entries.len()),
                    _ => None,
                })
                .sum::<usize>(),
            214
        );

        let ebnf = cll_lookup_section(site, "section-EBNF").expect("EBNF section should exist");
        let mut source_anchor_count = 0;
        let mut source_anchor_ids = BTreeSet::new();
        for anchor_id in ebnf
            .blocks
            .iter()
            .filter_map(|block| match block {
                CllBlock::Ebnf { entries, .. } => Some(entries),
                _ => None,
            })
            .flatten()
            .flat_map(|entry| &entry.source_anchor_ids)
            .filter(|anchor_id| anchor_id.starts_with("cll_bnf-"))
        {
            source_anchor_count += 1;
            source_anchor_ids.insert(anchor_id.as_str());
        }
        assert_eq!(source_anchor_count, 90);
        assert_eq!(source_anchor_ids.len(), 90);

        let mut link_targets = CllLinkTargetCounts {
            counts: BTreeMap::new(),
        };
        link_targets.visit_blocks(&cross_reference.blocks);
        let reference_count = link_targets
            .counts
            .iter()
            .filter(|(target, _)| target.starts_with("cll_bnf-"))
            .map(|(_, count)| count)
            .sum::<usize>();
        assert_eq!(reference_count, 456);
        let referenced_anchor_ids = link_targets
            .counts
            .keys()
            .filter(|target| target.starts_with("cll_bnf-"))
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        assert_eq!(referenced_anchor_ids.len(), 90);
        assert_eq!(referenced_anchor_ids, source_anchor_ids);

        let emitted_anchor_ids = site
            .anchors_by_id
            .iter()
            .filter(|(anchor_id, anchor)| {
                anchor_id.starts_with("cll_bnf-") && anchor.section_id == "section-EBNF"
            })
            .map(|(anchor_id, _)| anchor_id.as_str())
            .collect::<BTreeSet<_>>();
        assert_eq!(emitted_anchor_ids, source_anchor_ids);

        let rendered_cross_reference = render_section(
            site,
            cross_reference,
            CllRenderFormat::Html,
            CllLinkRenderMode::Web,
        );
        let rendered_ebnf =
            render_section(site, ebnf, CllRenderFormat::Html, CllLinkRenderMode::Web);
        for anchor_id in referenced_anchor_ids {
            let href = cll_link_href(site, CllLinkKind::Section, anchor_id);
            assert_eq!(href, format!("section/section-EBNF#{anchor_id}"));
            assert!(
                rendered_cross_reference.contains(&format!("href=\"{href}\"")),
                "cross-reference output should link to {anchor_id}"
            );
            assert!(
                rendered_ebnf.contains(&format!("id=\"{anchor_id}\"")),
                "EBNF output should emit {anchor_id}"
            );
        }
        assert!(rendered_ebnf.contains("id=\"ebnf-rule-ek\""));
    }

    #[invariant(
        true,
        "all combinations of link targets and occurrence counts are valid collector state"
    )]
    struct CllLinkTargetCounts {
        counts: BTreeMap<String, usize>,
    }

    #[contract_trait]
    impl CllBlockVisitor for CllLinkTargetCounts {
        #[requires(true)]
        #[ensures(true)]
        fn visit_inline(&mut self, inline: &CllInline) {
            if let CllInline::Link { target, .. } = inline {
                if let Some(count) = self.counts.get_mut(target) {
                    *count += 1;
                } else {
                    self.counts.insert(target.clone(), 1);
                }
            }
            super::visitor::walk_inline(self, inline);
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn colojban_import_covers_all_source_sections_and_anchored_examples() {
        let site = load_embedded_cll_site().expect("all embedded colojban chapters should import");
        assert_eq!(site.chapters.len(), 25);
        // Every division of colojban 1.3.4 has sections of its own - the PEG
        // appendix was the last one that did not - so the site holds exactly the
        // 350 `section` elements the sources contain and synthesizes none.
        assert_eq!(site.sections_by_id.len(), 350);
        assert_eq!(site.section_order.len(), 350);
        assert_eq!(site.examples_by_id.len(), 1857);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn xrefs_render_as_reference_labels_not_xml_ids() {
        let site = embedded_cll_site().expect("embedded CLL should load");
        let section = cll_lookup_section(site, "section-bridi").expect("section should exist");
        let rendered = render_section(
            site,
            section,
            CllRenderFormat::Markdown,
            CllLinkRenderMode::Web,
        );
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
        let rendered = render_section(
            site,
            section,
            CllRenderFormat::Markdown,
            CllLinkRenderMode::Web,
        );

        assert!(
            rendered
                .contains("[Chapter 21](section/section-grammars-introduction#chapter-grammars)")
        );
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

        let html = render_section(site, section, CllRenderFormat::Html, CllLinkRenderMode::Web);
        assert!(!html.contains("<th>cmavo</th>"));
        assert!(html.contains("<tr><td>coi</td><td>greetings</td></tr>"));
        assert!(html.contains(
            "<tr><td>ju'i</td><td>[jundi]</td><td>attention</td><td>at ease</td><td>ignore me/us</td></tr>"
        ));

        let markdown = render_section(
            site,
            section,
            CllRenderFormat::Markdown,
            CllLinkRenderMode::Web,
        );
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

        let html = render_section(site, section, CllRenderFormat::Html, CllLinkRenderMode::Web);
        assert!(html.contains("<th>cmavo</th><th>gismu</th><th>comments</th>"));

        let markdown = render_section(
            site,
            section,
            CllRenderFormat::Markdown,
            CllLinkRenderMode::Web,
        );
        assert!(markdown.contains("| cmavo | gismu | comments |"));
        assert!(markdown.contains("| --- | --- | --- |"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn plain_text_preserves_cmavo_headers_wider_than_rows() {
        let site = embedded_cll_site().expect("embedded CLL should load");
        let block = CllBlock::CmavoList {
            id: None,
            titles: Vec::new(),
            headers: vec![
                vec![CllInline::Text("cmavo".to_owned())],
                vec![CllInline::Text("selma'o".to_owned())],
                vec![CllInline::Text("notes".to_owned())],
            ],
            rows: vec![vec![vec![CllInline::Text("coi".to_owned())]]],
        };

        let plain_text = blocks_plain_text(site, &[block]);

        assert_eq!(plain_text, "cmavo selma'o notes coi");
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
        let rendered = render_section(site, section, CllRenderFormat::Html, CllLinkRenderMode::Web);
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
            render_section(site, section, CllRenderFormat::Html, CllLinkRenderMode::Web,)
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
            render_example(site, example, CllRenderFormat::Html, CllLinkRenderMode::Web,)
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

        let markdown = render_section(
            site,
            section,
            CllRenderFormat::Markdown,
            CllLinkRenderMode::Web,
        );
        assert!(markdown.contains("### Example 16.45"));
        assert!(markdown.contains("jbo: - [ci](../vlacku/ci)"));
        assert!(markdown.contains("jbo: [nu'i](../vlacku/nu'i)"));
        assert!(markdown.contains("gloss: - Three dogs [plus] two men, - - bite."));
        assert!(!markdown.contains("| - [ci](../vlacku/ci)"));

        let html = render_section(site, section, CllRenderFormat::Html, CllLinkRenderMode::Web);
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

        let markdown = render_section(
            site,
            section,
            CllRenderFormat::Markdown,
            CllLinkRenderMode::Web,
        );
        assert!(markdown.contains(&format!("```\n{expected}\n```")));
        assert!(
            !markdown
                .contains("Affirmations (positive) Negations (negative) |-----------|-----------|")
        );

        let html = render_section(site, section, CllRenderFormat::Html, CllLinkRenderMode::Web);
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
        assert!(
            !render_section(site, section, CllRenderFormat::Html, CllLinkRenderMode::Web,)
                .contains(".alf.")
        );
        assert!(
            !render_section(
                site,
                section,
                CllRenderFormat::Markdown,
                CllLinkRenderMode::Web,
            )
            .contains(".alf.")
        );
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
            assert_chrestomathy_metadata_rows_exist(metadata, CllTableRowArea::Header, header_rows);
            assert_chrestomathy_metadata_rows_exist(metadata, CllTableRowArea::Body, body_rows);
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
        // In the poem, line 9 opens with `fa` and continues line 8's bridi, so
        // the two are one parse target with a single button on the first row.
        let site = embedded_cll_site().expect("embedded CLL should load");
        let section = cll_lookup_section(site, "section-soft-rains").expect("section should exist");
        let (_header_rows, body_rows) = first_table_rows(section);
        let row_8_group = body_rows[7][0]
            .parse_group
            .as_ref()
            .expect("row 8 should have parse group");
        let row_9_group = body_rows[8][0]
            .parse_group
            .as_ref()
            .expect("row 9 should have parse group");
        assert_eq!(row_8_group.group_id, row_9_group.group_id);
        assert_eq!(row_8_group.row_count, 2);
        assert_eq!(row_8_group.row_index, 0);
        assert_eq!(row_9_group.row_index, 1);
        assert!(body_rows[7][0].parse_href.is_some());
        assert!(body_rows[8][0].parse_href.is_none());

        let html = render_section(site, section, CllRenderFormat::Html, CllLinkRenderMode::Web);
        assert!(html.contains("cll-parse-section"));
        assert!(html.contains("cll-parse-group-start"));
        assert!(html.contains("cll-parse-group-continuation"));
        assert!(html.contains("cll-parse-group-link"));
        assert!(html.contains("data-cll-parse-group=\"section-soft-rains-body-7-8-9\""));
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

    /// The inverse check has to be provable on its own. In the corpus the
    /// stronger `parse_table_block` postcondition fires first when headers are
    /// dropped, so this exercises the metadata rule directly: a declared header
    /// group against an empty header area is a failure.
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn metadata_naming_a_row_the_table_does_not_have_is_a_failure() {
        let metadata = new!(CllChrestomathySectionMetadata {
            id: "section-probe".to_owned(),
            header_groups: vec![vec![1]],
            body_groups: Vec::new(),
            header_no_parse: Vec::new(),
            body_no_parse: Vec::new(),
        });

        // One header row present: the declaration is satisfied.
        assert_chrestomathy_metadata_rows_exist(
            &metadata,
            CllTableRowArea::Header,
            &[vec![new!(CllTableCell {
                blocks: Vec::new(),
                col_span: None,
                row_span: None,
                parse_href: None,
                parse_group: None,
            })]],
        );

        // No header rows at all - the shape the dropped-`thead` bug produced.
        let empty = std::panic::catch_unwind(|| {
            assert_chrestomathy_metadata_rows_exist(&metadata, CllTableRowArea::Header, &[]);
        });
        assert!(
            empty.is_err(),
            "a declared header group with no imported header rows must fail"
        );
    }

    /// The inverse of `assert_chrestomathy_rows_are_covered`: every row the
    /// metadata names has to exist in the imported table.
    ///
    /// Coverage alone is one-directional. It walks the rows the importer
    /// produced, so when an area imports as empty it asserts nothing at all -
    /// which is precisely why `header_groups = [[1]]` sat in this file for a
    /// whole edition while the importer was dropping every `thead`. Naming a
    /// row that does not exist is now a failure in its own right.
    #[requires(true)]
    #[ensures(true)]
    fn assert_chrestomathy_metadata_rows_exist(
        metadata: &CllChrestomathySectionMetadata,
        area: CllTableRowArea,
        rows: &[Vec<CllTableCell>],
    ) {
        for row_index in chrestomathy_area_groups(metadata, area)
            .iter()
            .flatten()
            .chain(chrestomathy_area_no_parse_rows(metadata, area))
        {
            assert!(
                *row_index > 0 && *row_index <= rows.len(),
                "{} names {} row {}, but the imported table has {} such row(s)",
                metadata.id,
                chrestomathy_area_label(area),
                row_index,
                rows.len()
            );
        }
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
            division: CllDivision::Chapter {
                number: NonZeroU16::new(1).expect("test chapter number is non-zero"),
            },
            section_id: "section-test".to_owned(),
            section_number: Some(CllSectionNumber::Section {
                chapter: NonZeroU16::new(1).expect("test chapter number is non-zero"),
                index: NonZeroUsize::new(1).expect("test section index is non-zero"),
            }),
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
