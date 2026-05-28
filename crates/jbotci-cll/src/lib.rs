//! The Complete Lojban Language reference model.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;
use std::sync::OnceLock;

#[allow(unused_imports)]
use bityzba::{ensures, invariant, requires};
use bzip2::read::BzDecoder;
use roxmltree::{Document, Node};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const DEFAULT_CUKTA_CLI_RESULT_COUNT: usize = 10;
pub const DEFAULT_CUKTA_WEB_RESULT_COUNT: usize = 20;
pub const MAX_CUKTA_RESULT_COUNT: usize = 500;
pub const DEFAULT_CUKTA_SECTION_ID: &str = "section-what-is-lojban";
const PARAGRAPH_SEARCH_MIN_CHARS: usize = 200;

include!(concat!(env!("OUT_DIR"), "/embedded_cll.rs"));

static EMBEDDED_SITE: OnceLock<Result<CllSite, CllError>> = OnceLock::new();

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CllMetadata {
    pub title: String,
    pub chapter_count: usize,
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CllSite {
    pub metadata: CllMetadata,
    pub chapters: Vec<CllChapter>,
    pub sections_by_id: BTreeMap<String, CllSection>,
    pub section_order: Vec<String>,
    pub section_ids_by_normalized_reference: BTreeMap<String, String>,
    pub examples_by_id: BTreeMap<String, CllExample>,
    pub example_ids_by_normalized_reference: BTreeMap<String, String>,
    pub anchors_by_id: BTreeMap<String, CllAnchor>,
    pub index_entries: Vec<CllIndexEntry>,
    pub search_chunks: Vec<CllSearchChunk>,
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CllChapter {
    pub chapter_id: String,
    pub chapter_number: u16,
    pub chapter_title: String,
    pub root_section_ids: Vec<String>,
    pub prelude_blocks: Vec<CllBlock>,
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CllSection {
    pub section_id: String,
    pub chapter_id: String,
    pub chapter_number: u16,
    pub number: String,
    pub title: String,
    pub parent_section_id: Option<String>,
    pub child_section_ids: Vec<String>,
    pub blocks: Vec<CllBlock>,
    pub source_path: String,
    pub plain_text: String,
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CllAnchor {
    pub section_id: String,
    pub label: String,
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CllIndexEntry {
    pub key: String,
    pub section_ids: Vec<String>,
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CllReference {
    pub chapter: u16,
    pub section_number: String,
    pub section_id: String,
    pub example_number: Option<String>,
    pub example_id: Option<String>,
    pub source_path: String,
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CllExample {
    pub reference: CllReference,
    pub label: String,
    pub anchor_id: String,
    pub title: Option<String>,
    pub lojban: String,
    pub gloss_en: Option<String>,
    pub translation_en: Option<String>,
    pub lines: Vec<CllExampleLine>,
    pub plain_text: String,
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CllExampleLine {
    pub kind: String,
    pub text: String,
}

#[invariant(true)]
#[invariant(::Paragraph { .. } => true)]
#[invariant(::List { .. } => true)]
#[invariant(::Example(_) => true)]
#[invariant(::Table { .. } => true)]
#[invariant(::Media { .. } => true)]
#[invariant(::Rule { .. } => true)]
#[invariant(::Code { .. } => true)]
#[invariant(::Heading { .. } => true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CllBlock {
    Paragraph {
        anchor_id: Option<String>,
        role: Option<String>,
        inlines: Vec<CllInline>,
        text: String,
    },
    List {
        ordered: bool,
        items: Vec<Vec<CllBlock>>,
    },
    Example(CllExample),
    Table {
        caption: Option<String>,
        rows: Vec<Vec<Vec<CllBlock>>>,
    },
    Media {
        id: Option<String>,
        src: String,
        alt: String,
    },
    Rule {
        id: Option<String>,
        term: String,
        body: Vec<CllBlock>,
    },
    Code {
        language: Option<String>,
        text: String,
    },
    Heading {
        level: u8,
        title: String,
    },
}

#[invariant(true)]
#[invariant(::Text(_) => true)]
#[invariant(::Emphasis { .. } => true)]
#[invariant(::Quote { .. } => true)]
#[invariant(::Link { .. } => true)]
#[invariant(::Code(_) => true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CllInline {
    Text(String),
    Emphasis {
        role: Option<String>,
        text: String,
    },
    Quote {
        text: String,
    },
    Link {
        target: String,
        text: String,
        kind: CllLinkKind,
    },
    Code(String),
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CllLinkKind {
    Section,
    Example,
    Dictionary,
    Rafsi,
    Parse,
    Asset,
    External,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CllSearchChunkKind {
    Section,
    Paragraph,
    Example,
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CllSearchChunk {
    pub kind: CllSearchChunkKind,
    pub section_id: String,
    pub anchor_id: String,
    pub section_number: String,
    pub section_title: String,
    pub label: String,
    pub text: String,
    pub tagged_words: BTreeSet<String>,
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CllSearchMatch {
    pub rank: usize,
    pub similarity: Option<f32>,
    pub chunk: CllSearchChunk,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CuktaTargetFilter {
    pub sections: bool,
    pub paragraphs: bool,
    pub examples: bool,
}

impl Default for CuktaTargetFilter {
    #[requires(true)]
    #[ensures(ret.sections)]
    #[ensures(ret.paragraphs)]
    #[ensures(ret.examples)]
    fn default() -> Self {
        Self {
            sections: true,
            paragraphs: true,
            examples: true,
        }
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CuktaSearchMode {
    Meaning,
    Word,
}

#[invariant(true)]
#[invariant(::Section { .. } => true)]
#[invariant(::Example { .. } => true)]
#[invariant(::Search { .. } => true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CuktaRequest {
    Toc,
    Index,
    Section {
        reference: String,
    },
    Example {
        reference: String,
    },
    Search {
        mode: CuktaSearchMode,
        query: String,
        count: usize,
        targets: CuktaTargetFilter,
    },
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CuktaSearchOutput {
    pub mode: CuktaSearchMode,
    pub query: String,
    pub count: usize,
    pub matches: Vec<CllSearchMatch>,
    pub message: Option<String>,
    pub has_more: bool,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CllRenderFormat {
    Markdown,
    Html,
    Raw,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[invariant(true)]
#[invariant(::Load(_) => true)]
#[invariant(::Parse(_) => true)]
#[invariant(::NotFound(_) => true)]
pub enum CllError {
    #[error("failed to load CLL: {0}")]
    Load(String),
    #[error("failed to parse CLL: {0}")]
    Parse(String),
    #[error("{0}")]
    NotFound(String),
    #[error("cukta meaning search is not available yet; use --valsi for exact word search")]
    SemanticSearchDisabled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
struct SectionParseContext {
    chapter_id: String,
    chapter_number: u16,
    section_id: String,
    section_number: String,
    section_title: String,
    source_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
struct PendingIndexEntry {
    key: String,
    section_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
struct LinkResolution {
    label: String,
    kind: CllLinkKind,
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|site| !site.chapters.is_empty()))]
pub fn embedded_cll_site() -> Result<&'static CllSite, CllError> {
    EMBEDDED_SITE
        .get_or_init(load_embedded_cll_site)
        .as_ref()
        .map_err(Clone::clone)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|site| !site.chapters.is_empty()))]
pub fn load_embedded_cll_site() -> Result<CllSite, CllError> {
    let mut chapters = Vec::new();
    let mut sections_by_id = BTreeMap::new();
    let mut section_order = Vec::new();
    let mut examples_by_id = BTreeMap::new();
    let mut anchors_by_id = BTreeMap::new();
    let mut pending_index_entries = Vec::new();

    for (chapter_index, (source_path, compressed)) in EMBEDDED_CLL_CHAPTERS.iter().enumerate() {
        let xml = decode_chapter_xml(compressed)?;
        let xml = sanitize_xml_entities(&xml);
        let document = Document::parse(&xml)
            .map_err(|error| CllError::Parse(format!("{source_path}: {error}")))?;
        let root = document.root_element();
        let chapter_number =
            u16::try_from(chapter_index + 1).map_err(|error| CllError::Parse(error.to_string()))?;
        let (chapter, sections, examples, anchors, index_entries) =
            parse_chapter(root, chapter_number, source_path)?;
        for section in sections {
            section_order.push(section.section_id.clone());
            sections_by_id.insert(section.section_id.clone(), section);
        }
        for example in examples {
            examples_by_id.insert(example.anchor_id.clone(), example);
        }
        for anchor in anchors {
            anchors_by_id.insert(anchor.0, anchor.1);
        }
        pending_index_entries.extend(index_entries);
        chapters.push(chapter);
    }

    let mut site = CllSite {
        metadata: CllMetadata {
            title: "The Complete Lojban Language".to_owned(),
            chapter_count: chapters.len(),
        },
        chapters,
        sections_by_id,
        section_order,
        section_ids_by_normalized_reference: BTreeMap::new(),
        examples_by_id,
        example_ids_by_normalized_reference: BTreeMap::new(),
        anchors_by_id,
        index_entries: build_index_entries(&pending_index_entries),
        search_chunks: Vec::new(),
    };
    site.section_ids_by_normalized_reference = build_section_reference_index(&site);
    site.example_ids_by_normalized_reference = build_example_reference_index(&site);
    resolve_site_links(&mut site);
    site.search_chunks = build_search_chunks(&site);
    Ok(site)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()))]
fn decode_chapter_xml(compressed: &[u8]) -> Result<String, CllError> {
    let mut decoder = BzDecoder::new(compressed);
    let mut bytes = Vec::new();
    decoder
        .read_to_end(&mut bytes)
        .map_err(|error| CllError::Load(error.to_string()))?;
    String::from_utf8(bytes).map_err(|error| CllError::Load(error.to_string()))
}

#[requires(true)]
#[ensures(true)]
fn sanitize_xml_entities(xml: &str) -> String {
    xml.replace("&ndash;", "\u{2013}")
        .replace("&hellip;", "\u{2026}")
        .replace("&InvisibleTimes;", "\u{2062}")
}

#[requires(root.is_element())]
#[requires(chapter_number > 0)]
#[requires(!source_path.is_empty())]
#[ensures(ret.as_ref().is_ok_and(|(chapter, ..)| chapter.chapter_number == chapter_number))]
fn parse_chapter(
    root: Node<'_, '_>,
    chapter_number: u16,
    source_path: &str,
) -> Result<
    (
        CllChapter,
        Vec<CllSection>,
        Vec<CllExample>,
        Vec<(String, CllAnchor)>,
        Vec<PendingIndexEntry>,
    ),
    CllError,
> {
    let chapter_id = xml_id(root).unwrap_or_else(|| format!("chapter-{chapter_number}"));
    let title_node = child_element(root, "title");
    let chapter_title = title_node
        .map(visible_text)
        .filter(|title| !title.is_empty())
        .unwrap_or_else(|| format!("Chapter {chapter_number}"));
    let mut prelude_blocks = Vec::new();
    let mut sections = Vec::new();
    let mut examples = Vec::new();
    let mut anchors = Vec::new();
    let mut index_entries = Vec::new();
    let mut root_section_ids = Vec::new();
    let mut section_index = 0usize;

    if let Some(title_node) = title_node {
        collect_title_anchors(
            title_node,
            &chapter_id,
            &format!("{chapter_number}. {chapter_title}"),
            &mut anchors,
        );
    }

    for child in root.children().filter(Node::is_element) {
        if child.has_tag_name("title") {
            continue;
        }
        if child.has_tag_name("section") {
            section_index += 1;
            let parsed = parse_section(
                child,
                &chapter_id,
                chapter_number,
                section_index,
                source_path,
            )?;
            root_section_ids.push(parsed.0.section_id.clone());
            examples.extend(parsed.1);
            anchors.extend(parsed.2);
            index_entries.extend(parsed.3);
            sections.push(parsed.0);
        } else if let Some(block) = parse_standalone_chapter_block(child) {
            prelude_blocks.push(block);
        }
    }

    Ok((
        CllChapter {
            chapter_id,
            chapter_number,
            chapter_title,
            root_section_ids,
            prelude_blocks,
        },
        sections,
        examples,
        anchors,
        index_entries,
    ))
}

#[requires(section_node.is_element())]
#[requires(chapter_number > 0)]
#[requires(section_index > 0)]
#[ensures(ret.as_ref().is_ok_and(|(section, ..)| section.chapter_number == chapter_number))]
fn parse_section(
    section_node: Node<'_, '_>,
    chapter_id: &str,
    chapter_number: u16,
    section_index: usize,
    source_path: &str,
) -> Result<
    (
        CllSection,
        Vec<CllExample>,
        Vec<(String, CllAnchor)>,
        Vec<PendingIndexEntry>,
    ),
    CllError,
> {
    let section_id =
        xml_id(section_node).unwrap_or_else(|| format!("{chapter_id}-s{section_index}"));
    let section_number = format!("{chapter_number}.{section_index}");
    let title_node = child_element(section_node, "title");
    let section_title = title_node
        .map(visible_text)
        .filter(|title| !title.is_empty())
        .unwrap_or_else(|| format!("Section {section_number}"));
    let context = SectionParseContext {
        chapter_id: chapter_id.to_owned(),
        chapter_number,
        section_id: section_id.clone(),
        section_number: section_number.clone(),
        section_title: section_title.clone(),
        source_path: source_path.to_owned(),
    };
    let mut example_counter = 0usize;
    let mut examples = Vec::new();
    let mut anchors = Vec::new();
    let mut index_entries = Vec::new();

    if let Some(title_node) = title_node {
        collect_title_anchors(
            title_node,
            &section_id,
            &format!("{section_number}. {section_title}"),
            &mut anchors,
        );
    }
    for indexterm in section_node
        .descendants()
        .filter(|node| node.is_element() && node.has_tag_name("indexterm"))
    {
        if let Some(key) = index_key(indexterm) {
            index_entries.push(PendingIndexEntry {
                key,
                section_id: section_id.clone(),
            });
        }
    }

    let mut blocks = Vec::new();
    for child in section_node.children().filter(Node::is_element) {
        if child.has_tag_name("title") {
            continue;
        }
        if let Some(block) = parse_block(
            child,
            &context,
            &mut example_counter,
            &mut examples,
            &mut anchors,
        ) {
            blocks.push(block);
        }
    }
    let plain_text = normalized_plain_text(&blocks_plain_text(&blocks));
    anchors.push((
        section_id.clone(),
        CllAnchor {
            section_id: section_id.clone(),
            label: format!("{section_number}. {section_title}"),
        },
    ));

    Ok((
        CllSection {
            section_id,
            chapter_id: chapter_id.to_owned(),
            chapter_number,
            number: section_number,
            title: section_title,
            parent_section_id: None,
            child_section_ids: Vec::new(),
            blocks,
            source_path: source_path.to_owned(),
            plain_text,
        },
        examples,
        anchors,
        index_entries,
    ))
}

#[requires(node.is_element())]
#[ensures(true)]
fn parse_standalone_chapter_block(node: Node<'_, '_>) -> Option<CllBlock> {
    if node.has_tag_name("mediaobject") {
        parse_media_block(node)
    } else {
        let text = visible_text(node);
        (!text.is_empty()).then_some(CllBlock::Paragraph {
            anchor_id: xml_id(node),
            role: attr_string(node, "role"),
            inlines: vec![CllInline::Text(text.clone())],
            text,
        })
    }
}

#[requires(node.is_element())]
#[ensures(true)]
fn parse_block(
    node: Node<'_, '_>,
    context: &SectionParseContext,
    example_counter: &mut usize,
    examples: &mut Vec<CllExample>,
    anchors: &mut Vec<(String, CllAnchor)>,
) -> Option<CllBlock> {
    if node.has_tag_name("para") || node.has_tag_name("simpara") {
        return parse_paragraph_block(node);
    }
    if node.has_tag_name("itemizedlist") || node.has_tag_name("orderedlist") {
        return parse_list_block(node, context, example_counter, examples, anchors);
    }
    if node.has_tag_name("example") {
        return parse_example_block(node, context, example_counter, examples, anchors);
    }
    if node.has_tag_name("informaltable") || node.has_tag_name("table") {
        return parse_table_block(node, context, example_counter, examples, anchors);
    }
    if node.has_tag_name("mediaobject") {
        return parse_media_block(node);
    }
    if node.has_tag_name("programlisting") || node.has_tag_name("screen") {
        let text = normalized_plain_text(&raw_text(node));
        return (!text.is_empty()).then_some(CllBlock::Code {
            language: attr_string(node, "language"),
            text,
        });
    }
    if node.has_tag_name("variablelist") {
        let rows = node
            .children()
            .filter(|child| child.is_element() && child.has_tag_name("varlistentry"))
            .filter_map(|entry| {
                parse_variable_list_entry(entry, context, example_counter, examples, anchors)
            })
            .collect::<Vec<_>>();
        return (!rows.is_empty()).then_some(CllBlock::List {
            ordered: false,
            items: rows,
        });
    }
    if node.has_tag_name("bridgehead") {
        let title = visible_text(node);
        return (!title.is_empty()).then_some(CllBlock::Heading { level: 3, title });
    }
    let text = visible_text(node);
    (!text.is_empty()).then_some(CllBlock::Paragraph {
        anchor_id: xml_id(node),
        role: attr_string(node, "role"),
        inlines: vec![CllInline::Text(text.clone())],
        text,
    })
}

#[requires(node.is_element())]
#[ensures(true)]
fn parse_paragraph_block(node: Node<'_, '_>) -> Option<CllBlock> {
    let inlines = parse_inlines(node);
    let text = normalized_plain_text(&inline_plain_text(&inlines));
    (!text.is_empty()).then_some(CllBlock::Paragraph {
        anchor_id: xml_id(node),
        role: attr_string(node, "role"),
        inlines,
        text,
    })
}

#[requires(node.is_element())]
#[ensures(true)]
fn parse_list_block(
    node: Node<'_, '_>,
    context: &SectionParseContext,
    example_counter: &mut usize,
    examples: &mut Vec<CllExample>,
    anchors: &mut Vec<(String, CllAnchor)>,
) -> Option<CllBlock> {
    let items = node
        .children()
        .filter(|child| child.is_element() && child.has_tag_name("listitem"))
        .map(|item| {
            item.children()
                .filter(Node::is_element)
                .filter_map(|child| parse_block(child, context, example_counter, examples, anchors))
                .collect::<Vec<_>>()
        })
        .filter(|blocks| !blocks.is_empty())
        .collect::<Vec<_>>();
    (!items.is_empty()).then_some(CllBlock::List {
        ordered: node.has_tag_name("orderedlist"),
        items,
    })
}

#[requires(node.is_element())]
#[ensures(true)]
fn parse_example_block(
    node: Node<'_, '_>,
    context: &SectionParseContext,
    example_counter: &mut usize,
    examples: &mut Vec<CllExample>,
    anchors: &mut Vec<(String, CllAnchor)>,
) -> Option<CllBlock> {
    *example_counter += 1;
    let label = format!("{}.{}", context.section_number, *example_counter);
    let xml_id = xml_id(node);
    let title_node = child_element(node, "title");
    let title = title_node
        .map(visible_text)
        .filter(|value| !value.is_empty());
    let title_anchor = title_node.and_then(first_anchor_id);
    let anchor_id = title_anchor
        .or(xml_id.clone())
        .unwrap_or_else(|| format!("{}-example-{}", context.section_id, *example_counter));
    let mut lines = parse_example_lines(node);
    if lines.is_empty() {
        lines = parse_plain_example_lines(node);
    }
    let lojban = lines
        .iter()
        .filter(|line| line.kind == "jbo" || line.kind == "jbophrase")
        .map(|line| line.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    let gloss_en = lines
        .iter()
        .find(|line| line.kind == "gloss")
        .map(|line| line.text.clone());
    let translation_en = lines
        .iter()
        .find(|line| line.kind == "natlang")
        .map(|line| line.text.clone());
    let plain_text = if lines.is_empty() {
        visible_text(node)
    } else {
        lines
            .iter()
            .map(|line| line.text.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    };
    let example = CllExample {
        reference: CllReference {
            chapter: context.chapter_number,
            section_number: context.section_number.clone(),
            section_id: context.section_id.clone(),
            example_number: Some(label.clone()),
            example_id: Some(anchor_id.clone()),
            source_path: context.source_path.clone(),
        },
        label: label.clone(),
        anchor_id: anchor_id.clone(),
        title,
        lojban,
        gloss_en,
        translation_en,
        lines,
        plain_text,
    };
    anchors.push((
        anchor_id.clone(),
        CllAnchor {
            section_id: context.section_id.clone(),
            label: format!("Example {label}"),
        },
    ));
    if let Some(xml_id) = xml_id {
        anchors.push((
            xml_id,
            CllAnchor {
                section_id: context.section_id.clone(),
                label: format!("Example {label}"),
            },
        ));
    }
    examples.push(example.clone());
    Some(CllBlock::Example(example))
}

#[requires(node.is_element())]
#[ensures(true)]
fn parse_table_block(
    node: Node<'_, '_>,
    context: &SectionParseContext,
    example_counter: &mut usize,
    examples: &mut Vec<CllExample>,
    anchors: &mut Vec<(String, CllAnchor)>,
) -> Option<CllBlock> {
    let caption = child_element(node, "title").map(visible_text);
    let rows = node
        .descendants()
        .filter(|descendant| descendant.is_element() && descendant.has_tag_name("tr"))
        .map(|row| {
            row.children()
                .filter(|cell| {
                    cell.is_element() && (cell.has_tag_name("td") || cell.has_tag_name("th"))
                })
                .map(|cell| {
                    let blocks = cell
                        .children()
                        .filter(Node::is_element)
                        .filter_map(|child| {
                            parse_block(child, context, example_counter, examples, anchors)
                        })
                        .collect::<Vec<_>>();
                    if blocks.is_empty() {
                        let text = visible_text(cell);
                        if text.is_empty() {
                            Vec::new()
                        } else {
                            vec![CllBlock::Paragraph {
                                anchor_id: None,
                                role: None,
                                inlines: vec![CllInline::Text(text.clone())],
                                text,
                            }]
                        }
                    } else {
                        blocks
                    }
                })
                .collect::<Vec<_>>()
        })
        .filter(|row| !row.is_empty())
        .collect::<Vec<_>>();
    (!rows.is_empty()).then_some(CllBlock::Table { caption, rows })
}

#[requires(node.is_element())]
#[ensures(true)]
fn parse_media_block(node: Node<'_, '_>) -> Option<CllBlock> {
    let src = node
        .descendants()
        .find(|descendant| descendant.is_element() && descendant.has_tag_name("imagedata"))
        .and_then(|image| attr_string(image, "fileref"))?;
    let alt = node
        .descendants()
        .find(|descendant| descendant.is_element() && descendant.has_tag_name("phrase"))
        .map(visible_text)
        .unwrap_or_default();
    Some(CllBlock::Media {
        id: xml_id(node),
        src,
        alt,
    })
}

#[requires(entry.is_element())]
#[ensures(true)]
fn parse_variable_list_entry(
    entry: Node<'_, '_>,
    context: &SectionParseContext,
    example_counter: &mut usize,
    examples: &mut Vec<CllExample>,
    anchors: &mut Vec<(String, CllAnchor)>,
) -> Option<Vec<CllBlock>> {
    let term = entry
        .children()
        .find(|child| child.is_element() && child.has_tag_name("term"))
        .map(visible_text)
        .unwrap_or_default();
    let mut body = Vec::new();
    for listitem in entry
        .children()
        .filter(|child| child.is_element() && child.has_tag_name("listitem"))
    {
        body.extend(
            listitem
                .children()
                .filter(Node::is_element)
                .filter_map(|child| {
                    parse_block(child, context, example_counter, examples, anchors)
                }),
        );
    }
    (!term.is_empty() || !body.is_empty()).then_some(vec![CllBlock::Rule {
        id: xml_id(entry),
        term,
        body,
    }])
}

#[requires(node.is_element())]
#[ensures(true)]
fn parse_example_lines(node: Node<'_, '_>) -> Vec<CllExampleLine> {
    node.descendants()
        .filter(|descendant| {
            descendant.is_element()
                && matches!(
                    descendant.tag_name().name(),
                    "jbo" | "jbophrase" | "gloss" | "natlang"
                )
        })
        .filter_map(|line| {
            let text = visible_text(line);
            (!text.is_empty()).then_some(CllExampleLine {
                kind: line.tag_name().name().to_owned(),
                text,
            })
        })
        .collect()
}

#[requires(node.is_element())]
#[ensures(true)]
fn parse_plain_example_lines(node: Node<'_, '_>) -> Vec<CllExampleLine> {
    let lines = node
        .children()
        .filter(|child| {
            child.is_element() && (child.has_tag_name("para") || child.has_tag_name("simpara"))
        })
        .filter_map(|line| {
            let text = visible_text(line);
            (!text.is_empty()).then_some(CllExampleLine {
                kind: "text".to_owned(),
                text,
            })
        })
        .collect::<Vec<_>>();
    if lines.is_empty() {
        let text = visible_text(node);
        (!text.is_empty())
            .then_some(CllExampleLine {
                kind: "text".to_owned(),
                text,
            })
            .into_iter()
            .collect()
    } else {
        lines
    }
}

#[requires(node.is_element())]
#[ensures(true)]
fn parse_inlines(node: Node<'_, '_>) -> Vec<CllInline> {
    let mut inlines = Vec::new();
    for child in node.children() {
        if child.is_text() {
            push_text_inline(&mut inlines, child.text().unwrap_or_default());
        } else if child.is_element() {
            if child.has_tag_name("indexterm") || child.has_tag_name("anchor") {
                continue;
            }
            if child.has_tag_name("xref") {
                if let Some(target) = attr_string(child, "linkend") {
                    let label = attr_string(child, "xreflabel").unwrap_or_else(|| target.clone());
                    inlines.push(CllInline::Link {
                        target,
                        text: label,
                        kind: CllLinkKind::Section,
                    });
                }
            } else if child.has_tag_name("link") {
                let target = attr_string(child, "linkend")
                    .or_else(|| attr_string(child, "href"))
                    .or_else(|| attr_string(child, "xlink:href"));
                let text = visible_text(child);
                if let Some(target) = target {
                    if !text.is_empty() {
                        inlines.push(CllInline::Link {
                            target,
                            text,
                            kind: CllLinkKind::External,
                        });
                    }
                }
            } else if child.has_tag_name("quote") {
                let text = visible_text(child);
                if !text.is_empty() {
                    inlines.push(CllInline::Quote { text });
                }
            } else if child.has_tag_name("emphasis") || child.has_tag_name("citetitle") {
                let text = visible_text(child);
                if !text.is_empty() {
                    inlines.push(CllInline::Emphasis {
                        role: attr_string(child, "role"),
                        text,
                    });
                }
            } else if matches!(
                child.tag_name().name(),
                "valsi" | "cmavo" | "gismu" | "cmevla" | "rafsi"
            ) {
                let text = visible_text(child);
                if !text.is_empty() {
                    inlines.push(CllInline::Link {
                        target: text.clone(),
                        text,
                        kind: if child.has_tag_name("rafsi") {
                            CllLinkKind::Rafsi
                        } else {
                            CllLinkKind::Dictionary
                        },
                    });
                }
            } else if child.has_tag_name("code") || child.has_tag_name("literal") {
                let text = visible_text(child);
                if !text.is_empty() {
                    inlines.push(CllInline::Code(text));
                }
            } else {
                let nested = parse_inlines(child);
                if nested.is_empty() {
                    push_text_inline(&mut inlines, &visible_text(child));
                } else {
                    inlines.extend(nested);
                }
            }
        }
    }
    merge_adjacent_text_inlines(inlines)
}

#[requires(true)]
#[ensures(true)]
fn push_text_inline(inlines: &mut Vec<CllInline>, text: &str) {
    let normalized = normalize_text_fragment(text);
    if normalized.trim().is_empty() {
        return;
    }
    let piece = if inlines.is_empty() {
        normalized.trim_start().to_owned()
    } else {
        normalized
    };
    if piece.is_empty() {
        return;
    }
    inlines.push(CllInline::Text(piece));
}

#[requires(true)]
#[ensures(true)]
fn merge_adjacent_text_inlines(inlines: Vec<CllInline>) -> Vec<CllInline> {
    let mut merged = Vec::new();
    for inline in inlines {
        match (merged.last_mut(), inline) {
            (Some(CllInline::Text(existing)), CllInline::Text(next)) => {
                existing.push_str(&next);
            }
            (_, next) => merged.push(next),
        }
    }
    merged
}

#[requires(node.is_element())]
#[ensures(true)]
fn collect_title_anchors(
    node: Node<'_, '_>,
    section_id: &str,
    label: &str,
    anchors: &mut Vec<(String, CllAnchor)>,
) {
    for anchor_id in node
        .children()
        .filter(|child| child.is_element() && child.has_tag_name("anchor"))
        .filter_map(xml_id)
    {
        anchors.push((
            anchor_id,
            CllAnchor {
                section_id: section_id.to_owned(),
                label: label.to_owned(),
            },
        ));
    }
}

#[requires(node.is_element())]
#[ensures(true)]
fn first_anchor_id(node: Node<'_, '_>) -> Option<String> {
    node.children()
        .find(|child| child.is_element() && child.has_tag_name("anchor"))
        .and_then(xml_id)
}

#[requires(true)]
#[ensures(true)]
fn build_index_entries(entries: &[PendingIndexEntry]) -> Vec<CllIndexEntry> {
    let mut grouped: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for entry in entries {
        let section_ids = grouped.entry(entry.key.clone()).or_default();
        if !section_ids.contains(&entry.section_id) {
            section_ids.push(entry.section_id.clone());
        }
    }
    grouped
        .into_iter()
        .map(|(key, section_ids)| CllIndexEntry { key, section_ids })
        .collect()
}

#[requires(true)]
#[ensures(true)]
fn build_section_reference_index(site: &CllSite) -> BTreeMap<String, String> {
    let mut index = BTreeMap::new();
    for section in site.sections_by_id.values() {
        insert_reference(&mut index, &section.section_id, &section.section_id);
        insert_reference(&mut index, &section.number, &section.section_id);
        insert_reference(
            &mut index,
            &format!("section-{}", section.number),
            &section.section_id,
        );
    }
    for (anchor_id, anchor) in &site.anchors_by_id {
        if let Some(section_id) = resolve_anchor_section_id(site, anchor) {
            insert_reference(&mut index, anchor_id, section_id);
        }
    }
    index
}

#[requires(true)]
#[ensures(true)]
fn build_example_reference_index(site: &CllSite) -> BTreeMap<String, String> {
    let mut index = BTreeMap::new();
    for (id, example) in &site.examples_by_id {
        insert_reference(&mut index, id, id);
        insert_reference(&mut index, &example.label, id);
        if let Some(example_id) = &example.reference.example_id {
            insert_reference(&mut index, example_id, id);
        }
    }
    for (anchor_id, anchor) in &site.anchors_by_id {
        if let Some(example_label) = anchor.label.strip_prefix("Example ") {
            if let Some(example) = site
                .examples_by_id
                .values()
                .find(|example| example.label == example_label)
            {
                insert_reference(&mut index, anchor_id, &example.anchor_id);
            }
        }
    }
    index
}

#[requires(true)]
#[ensures(true)]
fn resolve_site_links(site: &mut CllSite) {
    let resolutions = build_link_resolutions(site);
    for chapter in &mut site.chapters {
        resolve_block_links(&mut chapter.prelude_blocks, &resolutions);
    }
    for section in site.sections_by_id.values_mut() {
        resolve_block_links(&mut section.blocks, &resolutions);
        section.plain_text = normalized_plain_text(&blocks_plain_text(&section.blocks));
    }
}

#[requires(true)]
#[ensures(true)]
fn build_link_resolutions(site: &CllSite) -> BTreeMap<String, LinkResolution> {
    let mut resolutions = BTreeMap::new();
    for section in site.sections_by_id.values() {
        resolutions.insert(
            section.section_id.clone(),
            LinkResolution {
                label: format_section_display_title(section),
                kind: CllLinkKind::Section,
            },
        );
    }
    for (anchor_id, anchor) in &site.anchors_by_id {
        resolutions.insert(
            anchor_id.clone(),
            LinkResolution {
                label: anchor.label.clone(),
                kind: if anchor.label.starts_with("Example ") {
                    CllLinkKind::Example
                } else {
                    CllLinkKind::Section
                },
            },
        );
    }
    for example in site.examples_by_id.values() {
        resolutions.insert(
            example.anchor_id.clone(),
            LinkResolution {
                label: format!("Example {}", example.label),
                kind: CllLinkKind::Example,
            },
        );
        insert_link_resolution(
            &mut resolutions,
            &example.label,
            LinkResolution {
                label: format!("Example {}", example.label),
                kind: CllLinkKind::Example,
            },
        );
    }
    resolutions
}

#[requires(!key.is_empty())]
#[ensures(true)]
fn insert_link_resolution(
    resolutions: &mut BTreeMap<String, LinkResolution>,
    key: &str,
    resolution: LinkResolution,
) {
    resolutions.entry(key.to_owned()).or_insert(resolution);
}

#[requires(true)]
#[ensures(true)]
fn resolve_block_links(blocks: &mut [CllBlock], resolutions: &BTreeMap<String, LinkResolution>) {
    for block in blocks {
        match block {
            CllBlock::Paragraph { inlines, text, .. } => {
                resolve_inline_links(inlines, resolutions);
                *text = normalized_plain_text(&inline_plain_text(inlines));
            }
            CllBlock::List { items, .. } => {
                for item in items {
                    resolve_block_links(item, resolutions);
                }
            }
            CllBlock::Table { rows, .. } => {
                for row in rows {
                    for cell in row {
                        resolve_block_links(cell, resolutions);
                    }
                }
            }
            CllBlock::Rule { body, .. } => resolve_block_links(body, resolutions),
            CllBlock::Example(_)
            | CllBlock::Media { .. }
            | CllBlock::Code { .. }
            | CllBlock::Heading { .. } => {}
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn resolve_inline_links(inlines: &mut [CllInline], resolutions: &BTreeMap<String, LinkResolution>) {
    for inline in inlines {
        if let CllInline::Link { target, text, kind } = inline {
            if *kind == CllLinkKind::Section
                && let Some(resolution) = resolutions.get(target)
            {
                *kind = resolution.kind;
                if text == target {
                    *text = resolution.label.clone();
                }
            }
        }
    }
}

#[requires(!reference.is_empty())]
#[requires(!value.is_empty())]
#[ensures(true)]
fn insert_reference(index: &mut BTreeMap<String, String>, reference: &str, value: &str) {
    let normalized = normalize_reference(reference);
    if !normalized.is_empty() {
        index.entry(normalized).or_insert_with(|| value.to_owned());
    }
}

#[requires(true)]
#[ensures(true)]
fn build_search_chunks(site: &CllSite) -> Vec<CllSearchChunk> {
    let mut chunks = Vec::new();
    for section_id in &site.section_order {
        if let Some(section) = site.sections_by_id.get(section_id) {
            let section_label = format_section_display_title(section);
            let section_text =
                normalized_plain_text(&format!("{}\n{}", section.title, section.plain_text));
            if !section_text.is_empty() {
                chunks.push(CllSearchChunk {
                    kind: CllSearchChunkKind::Section,
                    section_id: section.section_id.clone(),
                    anchor_id: section.section_id.clone(),
                    section_number: section.number.clone(),
                    section_title: section.title.clone(),
                    label: section_label.clone(),
                    text: section_text.clone(),
                    tagged_words: blocks_tagged_words(&section.blocks),
                });
            }
            collect_block_search_chunks(section, &section.blocks, &mut chunks);
        }
    }
    chunks
}

#[requires(true)]
#[ensures(true)]
fn collect_block_search_chunks(
    section: &CllSection,
    blocks: &[CllBlock],
    chunks: &mut Vec<CllSearchChunk>,
) {
    for block in blocks {
        match block {
            CllBlock::Paragraph {
                anchor_id,
                inlines,
                text,
                ..
            } => {
                if text.chars().count() > PARAGRAPH_SEARCH_MIN_CHARS {
                    chunks.push(CllSearchChunk {
                        kind: CllSearchChunkKind::Paragraph,
                        section_id: section.section_id.clone(),
                        anchor_id: anchor_id
                            .clone()
                            .unwrap_or_else(|| section.section_id.clone()),
                        section_number: section.number.clone(),
                        section_title: section.title.clone(),
                        label: format!("Paragraph in {}", format_section_display_title(section)),
                        text: text.clone(),
                        tagged_words: inlines_tagged_words(inlines),
                    });
                }
            }
            CllBlock::Example(example) => {
                if !example.plain_text.trim().is_empty() {
                    chunks.push(CllSearchChunk {
                        kind: CllSearchChunkKind::Example,
                        section_id: section.section_id.clone(),
                        anchor_id: example.anchor_id.clone(),
                        section_number: section.number.clone(),
                        section_title: section.title.clone(),
                        label: example.label.clone(),
                        text: example.plain_text.clone(),
                        tagged_words: example_tagged_words(example),
                    });
                }
            }
            CllBlock::List { items, .. } => {
                for item in items {
                    collect_block_search_chunks(section, item, chunks);
                }
            }
            CllBlock::Table { rows, .. } => {
                for row in rows {
                    for cell in row {
                        collect_block_search_chunks(section, cell, chunks);
                    }
                }
            }
            CllBlock::Rule { body, .. } => collect_block_search_chunks(section, body, chunks),
            CllBlock::Media { .. } | CllBlock::Code { .. } | CllBlock::Heading { .. } => {}
        }
    }
}

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
#[ensures(true)]
pub fn cll_lookup_example<'a>(site: &'a CllSite, example_id: &str) -> Option<&'a CllExample> {
    site.examples_by_id.get(example_id)
}

#[requires(true)]
#[ensures(true)]
pub fn cll_resolve_section_reference(site: &CllSite, reference: &str) -> Option<String> {
    site.section_ids_by_normalized_reference
        .get(&normalize_reference(reference))
        .cloned()
}

#[requires(true)]
#[ensures(true)]
pub fn cll_resolve_example_reference(site: &CllSite, reference: &str) -> Option<String> {
    site.example_ids_by_normalized_reference
        .get(&normalize_reference(reference))
        .cloned()
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
pub fn cll_search_all_chunks(site: &CllSite) -> &[CllSearchChunk] {
    &site.search_chunks
}

#[requires(true)]
#[ensures(true)]
pub fn cll_search_section_chunks(site: &CllSite) -> Vec<&CllSearchChunk> {
    site.search_chunks
        .iter()
        .filter(|chunk| chunk.kind == CllSearchChunkKind::Section)
        .collect()
}

#[requires(true)]
#[ensures(ret >= 1)]
pub fn clamp_cukta_result_count(count: usize) -> usize {
    count.clamp(1, MAX_CUKTA_RESULT_COUNT)
}

#[requires(true)]
#[ensures(true)]
pub fn cukta_word_search_matches(
    site: &CllSite,
    query: &str,
    count: usize,
    targets: CuktaTargetFilter,
) -> Vec<CllSearchMatch> {
    let terms = parse_word_search_terms(query);
    if terms.is_empty() || !target_filter_has_any(targets) {
        return Vec::new();
    }
    let selected = site
        .search_chunks
        .iter()
        .filter(|chunk| chunk_kind_allowed(chunk.kind, targets))
        .filter(|chunk| terms.iter().all(|term| chunk.tagged_words.contains(term)))
        .take(clamp_cukta_result_count(count))
        .cloned()
        .collect::<Vec<_>>();
    selected
        .into_iter()
        .enumerate()
        .map(|(index, chunk)| CllSearchMatch {
            rank: index + 1,
            similarity: None,
            chunk,
        })
        .collect()
}

#[requires(count > 0)]
#[ensures(ret.count == clamp_cukta_result_count(count))]
pub fn cukta_search(
    site: &CllSite,
    mode: CuktaSearchMode,
    query: &str,
    count: usize,
    targets: CuktaTargetFilter,
) -> CuktaSearchOutput {
    let count = clamp_cukta_result_count(count);
    let query = query.trim().to_owned();
    if query.is_empty() {
        return CuktaSearchOutput {
            mode,
            query,
            count,
            matches: Vec::new(),
            message: None,
            has_more: false,
        };
    }
    if mode == CuktaSearchMode::Meaning {
        return CuktaSearchOutput {
            mode,
            query,
            count,
            matches: Vec::new(),
            message: Some("Meaning search is not available yet.".to_owned()),
            has_more: false,
        };
    }
    if !target_filter_has_any(targets) {
        return CuktaSearchOutput {
            mode,
            query,
            count,
            matches: Vec::new(),
            message: Some("Select at least one search target.".to_owned()),
            has_more: false,
        };
    }
    let fetch_count = count.saturating_add(1).min(MAX_CUKTA_RESULT_COUNT);
    let mut matches = cukta_word_search_matches(site, &query, fetch_count, targets);
    let has_more = matches.len() > count;
    matches.truncate(count);
    let message = if matches.is_empty() {
        Some("No matches found.".to_owned())
    } else {
        None
    };
    CuktaSearchOutput {
        mode,
        query,
        count,
        matches,
        message,
        has_more,
    }
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
            Ok(render_example(example, format))
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
                    if let Some(section) = site.sections_by_id.get(section_id) {
                        output.push_str("<li><a href=\"");
                        output.push_str(&escape_html(&section_href(&section.section_id)));
                        output.push_str("\">");
                        output.push_str(&escape_html(&format_section_display_title(section)));
                        output.push_str("</a></li>");
                    }
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
                    if let Some(section) = site.sections_by_id.get(section_id) {
                        output
                            .push_str(&format!("  - {}\n", format_section_display_title(section)));
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
                        .filter_map(|section_id| site.sections_by_id.get(section_id))
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
                    .filter_map(|section_id| site.sections_by_id.get(section_id))
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
            output.push_str("<article class=\"cll-section-content\"><h1>");
            output.push_str(&escape_html(&format_section_display_title(section)));
            output.push_str("</h1>");
            for block in &section.blocks {
                output.push_str(&render_block_html(site, block));
            }
            output.push_str("</article>\n");
            output
        }
        CllRenderFormat::Markdown | CllRenderFormat::Raw => {
            let mut output = format!("# {}\n\n", format_section_display_title(section));
            for block in &section.blocks {
                render_block_markdown(site, block, &mut output, 0);
            }
            output
        }
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub fn render_example(example: &CllExample, format: CllRenderFormat) -> String {
    match format {
        CllRenderFormat::Html => {
            let mut output = format!(
                "<figure id=\"{}\" class=\"cll-example\"><figcaption>Example {}</figcaption>",
                escape_html(&example.anchor_id),
                escape_html(&example.label)
            );
            for line in &example.lines {
                output.push_str("<p class=\"cll-ig-line cll-ig-");
                output.push_str(&escape_html(&line.kind));
                output.push_str("\">");
                output.push_str(&escape_html(&line.text));
                output.push_str("</p>");
            }
            output.push_str("</figure>\n");
            output
        }
        CllRenderFormat::Markdown | CllRenderFormat::Raw => {
            let mut output = format!("### Example {}\n\n", example.label);
            for line in &example.lines {
                if line.kind == "text" {
                    output.push_str(&line.text);
                    output.push('\n');
                } else {
                    output.push_str(&format!("{}: {}\n", line.kind, line.text));
                }
            }
            output.push('\n');
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
#[ensures(ret.chars().count() <= max_chars + 1)]
pub fn truncate_preview(text: &str, max_chars: usize) -> String {
    let compact = normalized_plain_text(text);
    if compact.chars().count() <= max_chars {
        return compact;
    }
    let mut truncated = compact.chars().take(max_chars).collect::<String>();
    truncated.push('\u{2026}');
    truncated
}

#[requires(true)]
#[ensures(true)]
pub fn parse_word_search_terms(query: &str) -> BTreeSet<String> {
    collect_tagged_words(query)
}

#[requires(true)]
#[ensures(true)]
pub fn collect_tagged_words(text: &str) -> BTreeSet<String> {
    let mut words = BTreeSet::new();
    let mut current = String::new();
    for character in text.chars() {
        let normalized = character.to_ascii_lowercase();
        if normalized == 'h' {
            current.push('\'');
        } else if normalized == '.' {
            continue;
        } else if normalized.is_ascii_lowercase() || normalized == '\'' {
            current.push(normalized);
        } else if !current.is_empty() {
            words.insert(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        words.insert(current);
    }
    words
}

#[requires(true)]
#[ensures(true)]
fn blocks_tagged_words(blocks: &[CllBlock]) -> BTreeSet<String> {
    let mut words = BTreeSet::new();
    for block in blocks {
        words.extend(block_tagged_words(block));
    }
    words
}

#[requires(true)]
#[ensures(true)]
fn block_tagged_words(block: &CllBlock) -> BTreeSet<String> {
    match block {
        CllBlock::Paragraph { inlines, .. } => inlines_tagged_words(inlines),
        CllBlock::List { items, .. } => items
            .iter()
            .flat_map(|item| blocks_tagged_words(item))
            .collect(),
        CllBlock::Example(example) => example_tagged_words(example),
        CllBlock::Table { rows, .. } => rows
            .iter()
            .flat_map(|row| row.iter().flat_map(|cell| blocks_tagged_words(cell)))
            .collect(),
        CllBlock::Rule { body, .. } => blocks_tagged_words(body),
        CllBlock::Media { .. } | CllBlock::Code { .. } | CllBlock::Heading { .. } => {
            BTreeSet::new()
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn inlines_tagged_words(inlines: &[CllInline]) -> BTreeSet<String> {
    let mut words = BTreeSet::new();
    for inline in inlines {
        if let CllInline::Link {
            target,
            text,
            kind: CllLinkKind::Dictionary | CllLinkKind::Rafsi,
        } = inline
        {
            words.extend(collect_tagged_words(target));
            words.extend(collect_tagged_words(text));
        }
    }
    words
}

#[requires(true)]
#[ensures(true)]
fn example_tagged_words(example: &CllExample) -> BTreeSet<String> {
    example
        .lines
        .iter()
        .filter(|line| line.kind == "jbo" || line.kind == "jbophrase")
        .flat_map(|line| collect_tagged_words(&line.text))
        .collect()
}

#[requires(true)]
#[ensures(true)]
fn target_filter_has_any(filter: CuktaTargetFilter) -> bool {
    filter.sections || filter.paragraphs || filter.examples
}

#[requires(true)]
#[ensures(true)]
fn chunk_kind_allowed(kind: CllSearchChunkKind, filter: CuktaTargetFilter) -> bool {
    match kind {
        CllSearchChunkKind::Section => filter.sections,
        CllSearchChunkKind::Paragraph => filter.paragraphs,
        CllSearchChunkKind::Example => filter.examples,
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn search_chunk_kind_label(kind: CllSearchChunkKind) -> &'static str {
    match kind {
        CllSearchChunkKind::Section => "section",
        CllSearchChunkKind::Paragraph => "paragraph",
        CllSearchChunkKind::Example => "example",
    }
}

#[requires(true)]
#[ensures(true)]
fn render_block_markdown(site: &CllSite, block: &CllBlock, output: &mut String, depth: usize) {
    match block {
        CllBlock::Paragraph { inlines, text, .. } => {
            if inlines.is_empty() {
                output.push_str(text);
            } else {
                output.push_str(&render_inlines_markdown(site, inlines));
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
                    render_block_markdown(site, block, &mut item_text, depth + 1);
                }
                output.push_str(item_text.trim());
                output.push('\n');
            }
            output.push('\n');
        }
        CllBlock::Example(example) => {
            output.push_str(&render_example(example, CllRenderFormat::Markdown))
        }
        CllBlock::Table { caption, rows } => {
            if let Some(caption) = caption {
                output.push_str(&format!("**{caption}**\n\n"));
            }
            for row in rows {
                let cells = row
                    .iter()
                    .map(|cell| blocks_plain_text(cell))
                    .collect::<Vec<_>>();
                output.push_str("| ");
                output.push_str(&cells.join(" | "));
                output.push_str(" |\n");
            }
            output.push('\n');
        }
        CllBlock::Media { src, alt, .. } => {
            output.push_str(&format!("![{}]({})\n\n", alt, src));
        }
        CllBlock::Rule { term, body, .. } => {
            output.push_str(&format!("**{term}**\n\n"));
            for block in body {
                render_block_markdown(site, block, output, depth);
            }
        }
        CllBlock::Code { text, .. } => {
            output.push_str("```\n");
            output.push_str(text);
            output.push_str("\n```\n\n");
        }
        CllBlock::Heading { level, title } => {
            output.push_str(&"#".repeat(usize::from(*level)));
            output.push(' ');
            output.push_str(title);
            output.push_str("\n\n");
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_block_html(site: &CllSite, block: &CllBlock) -> String {
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
                render_inlines_html(site, inlines)
            };
            format!("<p{id}{class}>{body}</p>")
        }
        CllBlock::List { ordered, items } => {
            let tag = if *ordered { "ol" } else { "ul" };
            let mut output = format!("<{tag} class=\"cll-list\">");
            for item in items {
                output.push_str("<li>");
                for block in item {
                    output.push_str(&render_block_html(site, block));
                }
                output.push_str("</li>");
            }
            output.push_str(&format!("</{tag}>"));
            output
        }
        CllBlock::Example(example) => render_example(example, CllRenderFormat::Html),
        CllBlock::Table { caption, rows } => {
            let mut output = String::from("<table class=\"cll-table\">");
            if let Some(caption) = caption {
                output.push_str("<caption>");
                output.push_str(&escape_html(caption));
                output.push_str("</caption>");
            }
            for row in rows {
                output.push_str("<tr>");
                for cell in row {
                    output.push_str("<td>");
                    for block in cell {
                        output.push_str(&render_block_html(site, block));
                    }
                    output.push_str("</td>");
                }
                output.push_str("</tr>");
            }
            output.push_str("</table>");
            output
        }
        CllBlock::Media { id, src, alt } => {
            let id = id
                .as_ref()
                .map(|value| format!(" id=\"{}\"", escape_html(value)))
                .unwrap_or_default();
            format!(
                "<figure{id} class=\"cll-media\"><img src=\"{}\" alt=\"{}\" /></figure>",
                escape_html(src),
                escape_html(alt)
            )
        }
        CllBlock::Rule { id, term, body } => {
            let id = id
                .as_ref()
                .map(|value| format!(" id=\"{}\"", escape_html(value)))
                .unwrap_or_default();
            let mut output = format!(
                "<div{id} class=\"cll-rule\"><dt>{}</dt><dd>",
                escape_html(term)
            );
            for block in body {
                output.push_str(&render_block_html(site, block));
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
        CllBlock::Heading { level, title } => {
            let level = (*level).clamp(2, 6);
            format!("<h{level}>{}</h{level}>", escape_html(title))
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_inlines_markdown(site: &CllSite, inlines: &[CllInline]) -> String {
    let mut output = String::new();
    for inline in inlines {
        match inline {
            CllInline::Text(text) => output.push_str(text),
            CllInline::Emphasis { text, .. } => output.push_str(&format!("*{text}*")),
            CllInline::Quote { text } => output.push_str(&format!("\"{text}\"")),
            CllInline::Link { target, text, kind } => {
                output.push_str(&format!("[{text}]({})", cll_link_href(site, *kind, target)));
            }
            CllInline::Code(text) => output.push_str(&format!("`{text}`")),
        }
    }
    output
}

#[requires(true)]
#[ensures(true)]
fn render_inlines_html(site: &CllSite, inlines: &[CllInline]) -> String {
    let mut output = String::new();
    for inline in inlines {
        match inline {
            CllInline::Text(text) => output.push_str(&escape_html(text)),
            CllInline::Emphasis { text, .. } => {
                output.push_str("<em>");
                output.push_str(&escape_html(text));
                output.push_str("</em>");
            }
            CllInline::Quote { text } => {
                output.push_str("<q>");
                output.push_str(&escape_html(text));
                output.push_str("</q>");
            }
            CllInline::Link { target, text, kind } => {
                output.push_str("<a href=\"");
                output.push_str(&escape_html(&cll_link_href(site, *kind, target)));
                output.push_str("\">");
                output.push_str(&escape_html(text));
                output.push_str("</a>");
            }
            CllInline::Code(text) => {
                output.push_str("<code>");
                output.push_str(&escape_html(text));
                output.push_str("</code>");
            }
        }
    }
    output
}

#[requires(true)]
#[ensures(true)]
pub fn cll_link_href(site: &CllSite, kind: CllLinkKind, target: &str) -> String {
    match kind {
        CllLinkKind::Section | CllLinkKind::Example => {
            if let Some(example_id) = cll_resolve_example_reference(site, target)
                && let Some(example) = cll_lookup_example(site, &example_id)
            {
                return format!(
                    "{}#{}",
                    section_href(&example.reference.section_id),
                    example.anchor_id
                );
            }
            if let Some(anchor) = site.anchors_by_id.get(target)
                && let Some(section_id) = resolve_anchor_section_id(site, anchor)
            {
                if section_id == target {
                    return section_href(section_id);
                }
                return format!("{}#{target}", section_href(section_id));
            }
            cll_resolve_section_reference(site, target)
                .map(|section_id| section_href(&section_id))
                .unwrap_or_else(|| format!("#{target}"))
        }
        CllLinkKind::Dictionary => format!("../vlacku/{target}"),
        CllLinkKind::Rafsi => format!("../vlacku?mode=rafsi&q={target}"),
        CllLinkKind::Parse => format!("../gentufa?text={target}&dialect=allow-cgv"),
        CllLinkKind::Asset => target.to_owned(),
        CllLinkKind::External => target.to_owned(),
    }
}

#[requires(!section_id.is_empty())]
#[ensures(ret.contains(section_id))]
pub fn section_href(section_id: &str) -> String {
    format!("section/{section_id}")
}

#[requires(true)]
#[ensures(ret.is_none_or(|section_id| site.sections_by_id.contains_key(section_id)))]
fn resolve_anchor_section_id<'a>(site: &'a CllSite, anchor: &CllAnchor) -> Option<&'a str> {
    if let Some((section_id, _)) = site.sections_by_id.get_key_value(&anchor.section_id) {
        return Some(section_id);
    }
    site.chapters
        .iter()
        .find(|chapter| chapter.chapter_id == anchor.section_id)
        .and_then(|chapter| chapter.root_section_ids.first())
        .map(String::as_str)
}

#[requires(true)]
#[ensures(true)]
pub fn cll_search_chunk_href(chunk: &CllSearchChunk) -> String {
    if chunk.anchor_id == chunk.section_id {
        section_href(&chunk.section_id)
    } else {
        format!("{}#{}", section_href(&chunk.section_id), chunk.anchor_id)
    }
}

#[requires(true)]
#[ensures(true)]
fn index_key(node: Node<'_, '_>) -> Option<String> {
    let parts = ["primary", "secondary", "tertiary"]
        .iter()
        .filter_map(|name| child_element(node, name))
        .map(visible_text)
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>();
    (!parts.is_empty()).then(|| parts.join("; "))
}

#[requires(node.is_element())]
#[ensures(true)]
fn child_element<'a, 'input>(node: Node<'a, 'input>, name: &str) -> Option<Node<'a, 'input>> {
    node.children()
        .find(|child| child.is_element() && child.has_tag_name(name))
}

#[requires(node.is_element())]
#[ensures(true)]
fn xml_id(node: Node<'_, '_>) -> Option<String> {
    node.attribute(("http://www.w3.org/XML/1998/namespace", "id"))
        .or_else(|| node.attribute("xml:id"))
        .or_else(|| node.attribute("id"))
        .map(str::to_owned)
}

#[requires(node.is_element())]
#[ensures(true)]
fn attr_string(node: Node<'_, '_>, name: &str) -> Option<String> {
    node.attribute(name).map(str::to_owned)
}

#[requires(node.is_element())]
#[ensures(true)]
fn visible_text(node: Node<'_, '_>) -> String {
    normalized_plain_text(&visible_text_raw(node))
}

#[requires(node.is_element())]
#[ensures(true)]
fn visible_text_raw(node: Node<'_, '_>) -> String {
    let mut output = String::new();
    for child in node.children() {
        if child.is_text() {
            output.push_str(child.text().unwrap_or_default());
        } else if child.is_element() {
            if child.has_tag_name("indexterm") || child.has_tag_name("anchor") {
                continue;
            }
            output.push(' ');
            output.push_str(&visible_text_raw(child));
            output.push(' ');
        }
    }
    output
}

#[requires(node.is_element())]
#[ensures(true)]
fn raw_text(node: Node<'_, '_>) -> String {
    let mut output = String::new();
    for child in node.descendants() {
        if child.is_text() {
            output.push_str(child.text().unwrap_or_default());
        }
    }
    output
}

#[requires(true)]
#[ensures(true)]
fn normalize_text_fragment(text: &str) -> String {
    let mut output = String::new();
    let mut previous_was_space = false;
    for character in text.chars() {
        if character.is_whitespace() {
            if !previous_was_space {
                output.push(' ');
                previous_was_space = true;
            }
        } else {
            output.push(character);
            previous_was_space = false;
        }
    }
    output
}

#[requires(true)]
#[ensures(true)]
fn normalized_plain_text(text: &str) -> String {
    normalize_text_fragment(text).trim().to_owned()
}

#[requires(true)]
#[ensures(true)]
fn inline_plain_text(inlines: &[CllInline]) -> String {
    let mut output = String::new();
    for inline in inlines {
        match inline {
            CllInline::Text(text)
            | CllInline::Code(text)
            | CllInline::Emphasis { text, .. }
            | CllInline::Quote { text }
            | CllInline::Link { text, .. } => {
                output.push_str(text);
                output.push(' ');
            }
        }
    }
    normalized_plain_text(&output)
}

#[requires(true)]
#[ensures(true)]
fn blocks_plain_text(blocks: &[CllBlock]) -> String {
    let mut output = String::new();
    for block in blocks {
        match block {
            CllBlock::Paragraph { text, .. }
            | CllBlock::Code { text, .. }
            | CllBlock::Heading { title: text, .. } => {
                output.push_str(text);
                output.push('\n');
            }
            CllBlock::List { items, .. } => {
                for item in items {
                    output.push_str(&blocks_plain_text(item));
                    output.push('\n');
                }
            }
            CllBlock::Example(example) => {
                output.push_str(&example.plain_text);
                output.push('\n');
            }
            CllBlock::Table { caption, rows } => {
                if let Some(caption) = caption {
                    output.push_str(caption);
                    output.push('\n');
                }
                for row in rows {
                    for cell in row {
                        output.push_str(&blocks_plain_text(cell));
                        output.push('\n');
                    }
                }
            }
            CllBlock::Media { alt, .. } => {
                output.push_str(alt);
                output.push('\n');
            }
            CllBlock::Rule { term, body, .. } => {
                output.push_str(term);
                output.push('\n');
                output.push_str(&blocks_plain_text(body));
            }
        }
    }
    normalized_plain_text(&output)
}

#[requires(true)]
#[ensures(true)]
fn normalize_reference(reference: &str) -> String {
    reference
        .trim()
        .trim_start_matches('#')
        .to_ascii_lowercase()
}

#[requires(true)]
#[ensures(true)]
fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[allow(unused_imports)]
    use bityzba::{ensures, requires};

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
            cll_resolve_example_reference(site, "1.3.1")
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
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn xrefs_render_as_reference_labels_not_xml_ids() {
        let site = embedded_cll_site().expect("embedded CLL should load");
        let section = cll_lookup_section(site, "section-bridi").expect("section should exist");
        let rendered = render_section(site, section, CllRenderFormat::Markdown);
        assert!(rendered.contains("Example 2.1.1"));
        assert!(rendered.contains("John is the father of Sam."));
        assert!(!rendered.contains("[example-random-id-qIuj]"));
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
