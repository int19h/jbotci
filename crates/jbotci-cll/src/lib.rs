//! The Complete Lojban Language reference model.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;
use std::sync::OnceLock;

#[allow(unused_imports)]
use bityzba::{data, ensures, invariant, new, requires};
use bzip2::read::BzDecoder;
use roxmltree::{Document, Node};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
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
    pub parse_href: Option<String>,
    pub blocks: Vec<CllBlock>,
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CllSimpleListOrientation {
    Horizontal,
    Vertical,
}

#[invariant(col_span.is_none_or(|span| span > 0))]
#[invariant(row_span.is_none_or(|span| span > 0))]
#[invariant(parse_href.as_ref().is_none_or(|href| !href.is_empty()))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CllTableCell {
    pub blocks: Vec<CllBlock>,
    pub col_span: Option<usize>,
    pub row_span: Option<usize>,
    pub parse_href: Option<String>,
}

#[invariant(!term.is_empty() || !blocks.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CllVariableEntry {
    pub term: Vec<CllInline>,
    pub blocks: Vec<CllBlock>,
}

#[invariant(!kind.is_empty())]
#[invariant(!cells.is_empty())]
#[invariant(cells.iter().all(|cell| !cell.is_empty()))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CllInterlinearRow {
    pub kind: String,
    pub cells: Vec<Vec<CllInline>>,
}

#[invariant(!kind.is_empty())]
#[invariant(!body.is_empty() || comment.as_ref().is_some_and(|line| !line.is_empty()))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CllLojbanizationLine {
    pub kind: String,
    pub body: Vec<CllInline>,
    pub comment: Option<Vec<CllInline>>,
}

#[invariant(!kind.is_empty())]
#[invariant(!body.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CllLujvoPart {
    pub kind: String,
    pub body: Vec<CllInline>,
}

#[invariant(!rule_name.is_empty())]
#[invariant(!anchor_id.is_empty())]
#[invariant(rule_href.as_ref().is_none_or(|href| !href.is_empty()))]
#[invariant(!rhs.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CllEbnfEntry {
    pub rule_name: String,
    pub anchor_id: String,
    pub rule_href: Option<String>,
    pub rhs: Vec<CllEbnfToken>,
}

#[invariant(true)]
#[invariant(::Text { .. } => true)]
#[invariant(::Operator { .. } => true)]
#[invariant(::Hash { .. } => true)]
#[invariant(::Terminal { .. } => true)]
#[invariant(::ElidableTerminator { .. } => true)]
#[invariant(::Nonterminal { .. } => true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CllEbnfToken {
    Text { body: String },
    Operator { body: String },
    Hash { body: String },
    Terminal { body: String, href: Option<String> },
    ElidableTerminator { body: String, href: Option<String> },
    Nonterminal { body: String, href: Option<String> },
}

#[invariant(true)]
#[invariant(::Paragraph { .. } => true)]
#[invariant(::List { .. } => true)]
#[invariant(::Example(_) => true)]
#[invariant(::Table { .. } => true)]
#[invariant(::SimpleListTable { .. } => true)]
#[invariant(::VariableList { .. } => true)]
#[invariant(::Media { .. } => true)]
#[invariant(::Rule { .. } => true)]
#[invariant(::Code { .. } => true)]
#[invariant(::Heading { .. } => true)]
#[invariant(::BlockQuote { .. } => true)]
#[invariant(::Definition { .. } => true)]
#[invariant(::InterlinearGloss { .. } => true)]
#[invariant(::CmavoList { .. } => true)]
#[invariant(::Lojbanization { .. } => true)]
#[invariant(::LujvoMaking { .. } => true)]
#[invariant(::GrammarTemplate { .. } => true)]
#[invariant(::Ebnf { .. } => true)]
#[invariant(::DisplayMath { .. } => true)]
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
        id: Option<String>,
        caption: Option<Vec<CllInline>>,
        header_rows: Vec<Vec<CllTableCell>>,
        body_rows: Vec<Vec<CllTableCell>>,
        classes: Vec<String>,
    },
    SimpleListTable {
        id: Option<String>,
        orientation: CllSimpleListOrientation,
        rows: Vec<Vec<Option<Vec<CllInline>>>>,
    },
    VariableList {
        id: Option<String>,
        entries: Vec<CllVariableEntry>,
    },
    Media {
        id: Option<String>,
        title: Option<Vec<CllInline>>,
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
        id: Option<String>,
        level: u8,
        title: String,
        inlines: Vec<CllInline>,
    },
    BlockQuote {
        id: Option<String>,
        blocks: Vec<CllBlock>,
    },
    Definition {
        id: Option<String>,
        body: Vec<CllInline>,
    },
    InterlinearGloss {
        id: Option<String>,
        aligned: bool,
        itemized: bool,
        parse_href: Option<String>,
        rows: Vec<CllInterlinearRow>,
        natlang: Vec<Vec<CllInline>>,
        comments: Vec<Vec<CllInline>>,
    },
    CmavoList {
        id: Option<String>,
        titles: Vec<Vec<CllInline>>,
        headers: Vec<Vec<CllInline>>,
        rows: Vec<Vec<Vec<CllInline>>>,
    },
    Lojbanization {
        id: Option<String>,
        lines: Vec<CllLojbanizationLine>,
    },
    LujvoMaking {
        id: Option<String>,
        parts: Vec<CllLujvoPart>,
    },
    GrammarTemplate {
        id: Option<String>,
        body: Vec<CllInline>,
    },
    Ebnf {
        id: Option<String>,
        entries: Vec<CllEbnfEntry>,
    },
    DisplayMath {
        id: Option<String>,
        text: String,
        latex: String,
        markup: String,
    },
}

#[invariant(true)]
#[invariant(::Text(_) => true)]
#[invariant(::Emphasis { .. } => true)]
#[invariant(::Quote { .. } => true)]
#[invariant(::LanguageSpan { .. } => true)]
#[invariant(::CiteTitle { .. } => true)]
#[invariant(::Subscript { .. } => true)]
#[invariant(::Superscript { .. } => true)]
#[invariant(::Link { .. } => true)]
#[invariant(::Code(_) => true)]
#[invariant(::Elidable { .. } => true)]
#[invariant(::InlineMath { .. } => true)]
#[invariant(::Anchor { .. } => true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CllInline {
    Text(String),
    Emphasis {
        language: Option<String>,
        inlines: Vec<CllInline>,
    },
    Quote {
        language: Option<String>,
        inlines: Vec<CllInline>,
    },
    LanguageSpan {
        kind: CllLanguageSpanKind,
        language: Option<String>,
        inlines: Vec<CllInline>,
    },
    CiteTitle {
        inlines: Vec<CllInline>,
    },
    Subscript {
        inlines: Vec<CllInline>,
    },
    Superscript {
        inlines: Vec<CllInline>,
    },
    Link {
        target: String,
        inlines: Vec<CllInline>,
        kind: CllLinkKind,
    },
    Code(String),
    Elidable {
        shown: String,
        forced: bool,
        inlines: Vec<CllInline>,
    },
    InlineMath {
        text: String,
        latex: String,
        markup: String,
    },
    Anchor {
        id: String,
    },
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CllMathDisplay {
    Inline,
    Block,
}

#[invariant(markup.starts_with("<math") && markup.ends_with("</math>"))]
#[derive(Debug, Clone, PartialEq, Eq)]
struct CllMathRender {
    text: String,
    latex: String,
    markup: String,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CllLanguageSpanKind {
    ForeignPhrase,
    JboPhrase,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
enum AnchorMode {
    TopLevel,
    Nested,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
struct BlockParseState {
    chapter_example_counter: usize,
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
    let mut parse_state = BlockParseState {
        chapter_example_counter: 0,
    };

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
                &mut parse_state,
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

    anchors.push((
        chapter_id.clone(),
        CllAnchor {
            section_id: root_section_ids
                .first()
                .cloned()
                .unwrap_or_else(|| chapter_id.clone()),
            label: chapter_xref_label(chapter_number),
        },
    ));

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

#[requires(chapter_number > 0)]
#[ensures(ret.starts_with("Chapter "))]
fn chapter_xref_label(chapter_number: u16) -> String {
    format!("Chapter {chapter_number}")
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
    parse_state: &mut BlockParseState,
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

    let content_nodes = section_node
        .children()
        .filter(|child| {
            child.is_text()
                || (child.is_element()
                    && !child.has_tag_name("title")
                    && !child.has_tag_name("indexterm"))
        })
        .collect::<Vec<_>>();
    let blocks = parse_blocks_from_nodes(
        &content_nodes,
        &context,
        AnchorMode::TopLevel,
        parse_state,
        &mut examples,
        &mut anchors,
    );
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

#[requires(true)]
#[ensures(true)]
fn parse_blocks_from_nodes(
    nodes: &[Node<'_, '_>],
    context: &SectionParseContext,
    anchor_mode: AnchorMode,
    parse_state: &mut BlockParseState,
    examples: &mut Vec<CllExample>,
    anchors: &mut Vec<(String, CllAnchor)>,
) -> Vec<CllBlock> {
    let mut blocks = Vec::new();
    let mut inline_nodes = Vec::new();
    for node in nodes {
        if node.is_element() && is_display_none_element(*node) {
            flush_inline_nodes_as_paragraph(&mut blocks, &mut inline_nodes, None, None);
            continue;
        }
        if node.is_element() && is_block_element(*node) {
            flush_inline_nodes_as_paragraph(&mut blocks, &mut inline_nodes, None, None);
            blocks.extend(parse_block(
                *node,
                context,
                anchor_mode,
                parse_state,
                examples,
                anchors,
            ));
        } else if node.is_text() || node.is_element() {
            inline_nodes.push(*node);
        }
    }
    flush_inline_nodes_as_paragraph(&mut blocks, &mut inline_nodes, None, None);
    blocks
}

#[requires(node.is_element())]
#[ensures(true)]
fn parse_block(
    node: Node<'_, '_>,
    context: &SectionParseContext,
    anchor_mode: AnchorMode,
    parse_state: &mut BlockParseState,
    examples: &mut Vec<CllExample>,
    anchors: &mut Vec<(String, CllAnchor)>,
) -> Vec<CllBlock> {
    if node.has_tag_name("para") || node.has_tag_name("simpara") {
        return parse_paragraph_blocks(node, context, anchor_mode, parse_state, examples, anchors);
    }
    if node.has_tag_name("itemizedlist") || node.has_tag_name("orderedlist") {
        return parse_list_block(node, context, parse_state, examples, anchors)
            .into_iter()
            .collect();
    }
    if node.has_tag_name("simplelist") {
        return parse_simple_list_block(node).into_iter().collect();
    }
    if node.has_tag_name("example") {
        return parse_example_block(node, context, parse_state, examples, anchors)
            .into_iter()
            .collect();
    }
    if node.has_tag_name("informaltable") || node.has_tag_name("table") {
        return parse_table_block(node, context, parse_state, examples, anchors)
            .into_iter()
            .collect();
    }
    if node.has_tag_name("mediaobject") {
        return parse_media_block(node).into_iter().collect();
    }
    if node.has_tag_name("programlisting")
        || node.has_tag_name("screen")
        || node.has_tag_name("literallayout")
    {
        let text = normalized_plain_text(&raw_text(node));
        return (!text.is_empty())
            .then_some(CllBlock::Code {
                language: attr_string(node, "language"),
                text,
            })
            .into_iter()
            .collect();
    }
    if node.has_tag_name("variablelist") {
        return parse_variable_list_block(node, context, parse_state, examples, anchors)
            .into_iter()
            .collect();
    }
    if node.has_tag_name("bridgehead") {
        let mut inlines = parse_inlines(node);
        let title = inline_plain_text(&inlines);
        let id = first_anchor_id(node)
            .or_else(|| block_anchor_id_for("heading", anchor_mode, context, node));
        inlines.retain(|inline| !matches!(inline, CllInline::Anchor { .. }));
        return (!title.is_empty())
            .then_some(CllBlock::Heading {
                id,
                level: 3,
                title,
                inlines,
            })
            .into_iter()
            .collect();
    }
    if node.has_tag_name("dbmath") || node.has_tag_name("math") {
        let rendered = render_math_node(node, CllMathDisplay::Block).into_data();
        return Some(CllBlock::DisplayMath {
            id: block_anchor_id_for("math", anchor_mode, context, node),
            text: rendered.text,
            latex: rendered.latex,
            markup: rendered.markup,
        })
        .into_iter()
        .collect();
    }
    if node.has_tag_name("blockquote") {
        let blocks = parse_blocks_from_nodes(
            &node.children().collect::<Vec<_>>(),
            context,
            AnchorMode::Nested,
            parse_state,
            examples,
            anchors,
        );
        return (!blocks.is_empty())
            .then_some(CllBlock::BlockQuote {
                id: block_anchor_id_for("quote", anchor_mode, context, node),
                blocks,
            })
            .into_iter()
            .collect();
    }
    if node.has_tag_name("definition") || node.has_tag_name("grammar-template") {
        let body = parse_inlines(node);
        return (!body.is_empty())
            .then_some(if node.has_tag_name("definition") {
                CllBlock::Definition {
                    id: block_anchor_id_for("definition", anchor_mode, context, node),
                    body,
                }
            } else {
                CllBlock::GrammarTemplate {
                    id: block_anchor_id_for("grammar-template", anchor_mode, context, node),
                    body,
                }
            })
            .into_iter()
            .collect();
    }
    if node.has_tag_name("interlinear-gloss") {
        return parse_interlinear_gloss_block(node, context, anchor_mode)
            .into_iter()
            .collect();
    }
    if node.has_tag_name("interlinear-gloss-itemized") {
        return parse_interlinear_gloss_itemized_block(node, context, anchor_mode)
            .into_iter()
            .collect();
    }
    if node.has_tag_name("cmavo-list") {
        return parse_cmavo_list_block(node, context, anchor_mode)
            .into_iter()
            .collect();
    }
    if node.has_tag_name("lojbanization") {
        return parse_lojbanization_block(node, context, anchor_mode)
            .into_iter()
            .collect();
    }
    if node.has_tag_name("lujvo-making") {
        return parse_lujvo_making_block(node, context, anchor_mode)
            .into_iter()
            .collect();
    }
    let text = visible_text(node);
    (!text.is_empty())
        .then_some(CllBlock::Paragraph {
            anchor_id: xml_id(node),
            role: attr_string(node, "role"),
            inlines: vec![CllInline::Text(text.clone())],
            text,
        })
        .into_iter()
        .collect()
}

#[requires(node.is_element())]
#[ensures(true)]
fn parse_paragraph_blocks(
    node: Node<'_, '_>,
    context: &SectionParseContext,
    anchor_mode: AnchorMode,
    parse_state: &mut BlockParseState,
    examples: &mut Vec<CllExample>,
    anchors: &mut Vec<(String, CllAnchor)>,
) -> Vec<CllBlock> {
    let mut blocks = Vec::new();
    let mut inline_nodes = Vec::new();
    for child in node.children() {
        if child.is_element() && is_block_element(child) {
            flush_inline_nodes_as_paragraph(
                &mut blocks,
                &mut inline_nodes,
                paragraph_anchor_id_for(anchor_mode, context, node),
                attr_string(node, "role"),
            );
            blocks.extend(parse_block(
                child,
                context,
                AnchorMode::Nested,
                parse_state,
                examples,
                anchors,
            ));
        } else if child.is_text()
            || (child.is_element()
                && !child.has_tag_name("title")
                && !child.has_tag_name("indexterm"))
        {
            inline_nodes.push(child);
        }
    }
    flush_inline_nodes_as_paragraph(
        &mut blocks,
        &mut inline_nodes,
        paragraph_anchor_id_for(anchor_mode, context, node),
        attr_string(node, "role"),
    );
    blocks
}

#[requires(true)]
#[ensures(inline_nodes.is_empty())]
fn flush_inline_nodes_as_paragraph(
    blocks: &mut Vec<CllBlock>,
    inline_nodes: &mut Vec<Node<'_, '_>>,
    anchor_id: Option<String>,
    role: Option<String>,
) {
    if inline_nodes.is_empty() {
        return;
    }
    let inlines = trim_inline_runs(parse_inline_nodes(inline_nodes));
    inline_nodes.clear();
    let text = normalized_plain_text(&inline_plain_text(&inlines));
    if !text.is_empty() {
        blocks.push(CllBlock::Paragraph {
            anchor_id,
            role,
            inlines,
            text,
        });
    }
}

#[requires(node.is_element())]
#[ensures(true)]
fn parse_list_block(
    node: Node<'_, '_>,
    context: &SectionParseContext,
    parse_state: &mut BlockParseState,
    examples: &mut Vec<CllExample>,
    anchors: &mut Vec<(String, CllAnchor)>,
) -> Option<CllBlock> {
    let items = node
        .children()
        .filter(|child| child.is_element() && child.has_tag_name("listitem"))
        .map(|item| {
            parse_blocks_from_nodes(
                &non_title_child_nodes(item),
                context,
                AnchorMode::Nested,
                parse_state,
                examples,
                anchors,
            )
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
fn parse_simple_list_block(node: Node<'_, '_>) -> Option<CllBlock> {
    let member_bodies = node
        .children()
        .filter(|child| child.is_element() && child.has_tag_name("member"))
        .map(parse_inlines)
        .map(trim_inline_runs)
        .filter(|body| !body.is_empty())
        .collect::<Vec<_>>();
    if member_bodies.is_empty() {
        return None;
    }
    let columns = attr_usize(node, "columns").unwrap_or(1).max(1);
    let orientation = match attr_string(node, "type").as_deref() {
        Some("horiz") => CllSimpleListOrientation::Horizontal,
        _ => CllSimpleListOrientation::Vertical,
    };
    let rows = match orientation {
        CllSimpleListOrientation::Horizontal => simple_list_rows_horizontal(columns, member_bodies),
        CllSimpleListOrientation::Vertical => simple_list_rows_vertical(columns, member_bodies),
    };
    Some(CllBlock::SimpleListTable {
        id: xml_id(node),
        orientation,
        rows,
    })
}

#[requires(columns > 0)]
#[ensures(true)]
fn simple_list_rows_horizontal(
    columns: usize,
    members: Vec<Vec<CllInline>>,
) -> Vec<Vec<Option<Vec<CllInline>>>> {
    members
        .chunks(columns)
        .map(|chunk| chunk.iter().cloned().map(Some).collect())
        .collect()
}

#[requires(columns > 0)]
#[ensures(true)]
fn simple_list_rows_vertical(
    columns: usize,
    members: Vec<Vec<CllInline>>,
) -> Vec<Vec<Option<Vec<CllInline>>>> {
    let row_count = members.len().div_ceil(columns).max(1);
    (0..row_count)
        .map(|row_index| {
            (0..columns)
                .map(|column_index| members.get(row_index + column_index * row_count).cloned())
                .collect()
        })
        .collect()
}

#[requires(node.is_element())]
#[ensures(true)]
fn parse_example_block(
    node: Node<'_, '_>,
    context: &SectionParseContext,
    parse_state: &mut BlockParseState,
    examples: &mut Vec<CllExample>,
    anchors: &mut Vec<(String, CllAnchor)>,
) -> Option<CllBlock> {
    parse_state.chapter_example_counter += 1;
    let example_number = format!(
        "{}.{}",
        context.chapter_number, parse_state.chapter_example_counter
    );
    let display_label = format!("Example {example_number}");
    let xml_id = xml_id(node);
    let title_node = child_element(node, "title");
    let explicit_title = title_node
        .map(visible_text)
        .filter(|value| !value.is_empty());
    let title_anchor = title_node.and_then(first_anchor_id);
    let anchor_id = title_anchor.or(xml_id.clone()).unwrap_or_else(|| {
        format!(
            "{}-example-{}",
            context.section_id, parse_state.chapter_example_counter
        )
    });
    let mut nested_examples = Vec::new();
    let mut blocks = parse_blocks_from_nodes(
        &non_title_child_nodes(node),
        context,
        AnchorMode::Nested,
        parse_state,
        &mut nested_examples,
        anchors,
    );
    examples.extend(nested_examples);
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
    if blocks.is_empty() && !plain_text.trim().is_empty() {
        blocks.push(CllBlock::Paragraph {
            anchor_id: None,
            role: None,
            inlines: vec![CllInline::Text(plain_text.clone())],
            text: normalized_plain_text(&plain_text),
        });
    }
    let example = CllExample {
        reference: CllReference {
            chapter: context.chapter_number,
            section_number: context.section_number.clone(),
            section_id: context.section_id.clone(),
            example_number: Some(example_number),
            example_id: Some(anchor_id.clone()),
            source_path: context.source_path.clone(),
        },
        label: display_label.clone(),
        anchor_id: anchor_id.clone(),
        title: explicit_title,
        parse_href: collect_jbo_snippet(node).and_then(|snippet| jbo_parse_href(&snippet)),
        blocks,
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
            label: display_label.clone(),
        },
    ));
    if let Some(xml_id) = xml_id {
        anchors.push((
            xml_id,
            CllAnchor {
                section_id: context.section_id.clone(),
                label: display_label,
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
    parse_state: &mut BlockParseState,
    examples: &mut Vec<CllExample>,
    anchors: &mut Vec<(String, CllAnchor)>,
) -> Option<CllBlock> {
    let source = child_element(node, "tgroup")
        .or_else(|| child_element(node, "tbody"))
        .unwrap_or(node);
    let header_rows = child_element(source, "thead")
        .map(|thead| parse_table_rows(thead, context, parse_state, examples, anchors))
        .unwrap_or_default();
    let tbody_rows = child_element(source, "tbody")
        .map(|tbody| parse_table_rows(tbody, context, parse_state, examples, anchors))
        .unwrap_or_default();
    let body_rows = if tbody_rows.is_empty() {
        parse_table_rows(source, context, parse_state, examples, anchors)
    } else {
        tbody_rows
    };
    let caption = child_element(node, "caption")
        .or_else(|| child_element(node, "title"))
        .map(parse_inlines)
        .map(trim_inline_runs)
        .filter(|inlines| !inlines.is_empty());
    if header_rows.is_empty() && body_rows.is_empty() {
        return None;
    }
    let mut classes: Vec<String> = attr_string(node, "class")
        .map(|value| value.split_whitespace().map(str::to_owned).collect())
        .unwrap_or_default();
    if table_is_simple_list_chart(&header_rows, &body_rows)
        && !classes.iter().any(|class| class == "simplelist-chart")
    {
        classes.push("simplelist-chart".to_owned());
    }
    Some(CllBlock::Table {
        id: block_anchor_id_for("table", AnchorMode::TopLevel, context, node),
        caption,
        header_rows,
        body_rows,
        classes,
    })
}

#[requires(node.is_element())]
#[ensures(true)]
fn parse_table_rows(
    node: Node<'_, '_>,
    context: &SectionParseContext,
    parse_state: &mut BlockParseState,
    examples: &mut Vec<CllExample>,
    anchors: &mut Vec<(String, CllAnchor)>,
) -> Vec<Vec<CllTableCell>> {
    node.children()
        .filter(|row| row.is_element() && (row.has_tag_name("row") || row.has_tag_name("tr")))
        .map(|row| parse_table_row(row, context, parse_state, examples, anchors))
        .filter(|row| !row.is_empty())
        .collect()
}

#[requires(row.is_element())]
#[ensures(true)]
fn parse_table_row(
    row: Node<'_, '_>,
    context: &SectionParseContext,
    parse_state: &mut BlockParseState,
    examples: &mut Vec<CllExample>,
    anchors: &mut Vec<(String, CllAnchor)>,
) -> Vec<CllTableCell> {
    row.children()
        .filter(|cell| {
            cell.is_element()
                && (cell.has_tag_name("entry")
                    || cell.has_tag_name("td")
                    || cell.has_tag_name("th"))
        })
        .enumerate()
        .map(|(cell_index, cell)| {
            let mut blocks = parse_blocks_from_nodes(
                &cell.children().collect::<Vec<_>>(),
                context,
                AnchorMode::Nested,
                parse_state,
                examples,
                anchors,
            );
            if blocks.is_empty() {
                let text = visible_text(cell);
                if !text.is_empty() {
                    blocks.push(CllBlock::Paragraph {
                        anchor_id: None,
                        role: None,
                        inlines: vec![CllInline::Text(text.clone())],
                        text,
                    });
                }
            }
            new!(CllTableCell {
                blocks,
                col_span: attr_usize(cell, "colspan"),
                row_span: attr_usize(cell, "rowspan"),
                parse_href: chrestomathy_parse_href(context, cell_index, cell),
            })
        })
        .collect()
}

#[requires(cell.is_element())]
#[ensures(ret.as_ref().is_none_or(|href| href.starts_with("../gentufa?text=")))]
fn chrestomathy_parse_href(
    context: &SectionParseContext,
    cell_index: usize,
    cell: Node<'_, '_>,
) -> Option<String> {
    if context.chapter_id != "volume-chrestomathy" || cell_index != 0 || !cell.has_tag_name("td") {
        return None;
    }
    let text = visible_text(cell);
    jbo_parse_href(&text)
}

#[requires(true)]
#[ensures(true)]
fn table_is_simple_list_chart(
    header_rows: &[Vec<CllTableCell>],
    body_rows: &[Vec<CllTableCell>],
) -> bool {
    header_rows.is_empty()
        && !body_rows.is_empty()
        && body_rows.iter().all(|row| {
            row.iter()
                .all(|cell| matches!(cell.blocks.as_slice(), [CllBlock::SimpleListTable { .. }]))
        })
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
        title: child_element(node, "title")
            .map(parse_inlines)
            .map(trim_inline_runs)
            .filter(|inlines| !inlines.is_empty()),
        src,
        alt,
    })
}

#[requires(node.is_element())]
#[ensures(true)]
fn parse_variable_list_block(
    node: Node<'_, '_>,
    context: &SectionParseContext,
    parse_state: &mut BlockParseState,
    examples: &mut Vec<CllExample>,
    anchors: &mut Vec<(String, CllAnchor)>,
) -> Option<CllBlock> {
    if context.section_id == "section-EBNF" {
        return parse_ebnf_block(node, context);
    }
    let entries = node
        .children()
        .filter(|child| child.is_element() && child.has_tag_name("varlistentry"))
        .filter_map(|entry| {
            parse_variable_list_entry(entry, context, parse_state, examples, anchors)
        })
        .collect::<Vec<_>>();
    (!entries.is_empty()).then_some(CllBlock::VariableList {
        id: block_anchor_id_for("variable-list", AnchorMode::TopLevel, context, node),
        entries,
    })
}

#[requires(entry.is_element())]
#[ensures(true)]
fn parse_variable_list_entry(
    entry: Node<'_, '_>,
    context: &SectionParseContext,
    parse_state: &mut BlockParseState,
    examples: &mut Vec<CllExample>,
    anchors: &mut Vec<(String, CllAnchor)>,
) -> Option<CllVariableEntry> {
    let term = entry
        .children()
        .find(|child| child.is_element() && child.has_tag_name("term"))
        .map(parse_inlines)
        .map(trim_inline_runs)
        .unwrap_or_default();
    let mut blocks = Vec::new();
    for listitem in entry
        .children()
        .filter(|child| child.is_element() && child.has_tag_name("listitem"))
    {
        blocks.extend(parse_blocks_from_nodes(
            &non_title_child_nodes(listitem),
            context,
            AnchorMode::Nested,
            parse_state,
            examples,
            anchors,
        ));
    }
    (!term.is_empty() || !blocks.is_empty()).then_some(new!(CllVariableEntry { term, blocks }))
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
    parse_inline_nodes(&node.children().collect::<Vec<_>>())
}

#[requires(true)]
#[ensures(true)]
fn parse_inline_nodes(nodes: &[Node<'_, '_>]) -> Vec<CllInline> {
    let mut inlines = Vec::new();
    for child in nodes {
        if child.is_text() {
            push_text_inline(&mut inlines, child.text().unwrap_or_default());
        } else if child.is_element() {
            if is_display_none_element(*child) || child.has_tag_name("indexterm") {
                continue;
            }
            match child.tag_name().name() {
                "anchor" => {
                    if let Some(id) = xml_id(*child) {
                        inlines.push(CllInline::Anchor { id });
                    }
                }
                "xref" => {
                    if let Some(target) = attr_string(*child, "linkend") {
                        let label =
                            attr_string(*child, "xreflabel").unwrap_or_else(|| target.clone());
                        inlines.push(CllInline::Link {
                            target,
                            inlines: vec![CllInline::Text(label)],
                            kind: CllLinkKind::Section,
                        });
                    }
                }
                "ulink" | "link" => {
                    let target = attr_string(*child, "href")
                        .or_else(|| attr_string(*child, "url"))
                        .or_else(|| attr_string(*child, "xlink:href"))
                        .or_else(|| attr_string(*child, "linkend"));
                    if let Some(target) = target {
                        let body = trim_inline_runs(parse_inlines(*child));
                        let body = if body.is_empty() {
                            vec![CllInline::Text(target.clone())]
                        } else {
                            body
                        };
                        inlines.push(CllInline::Link {
                            target,
                            inlines: body,
                            kind: CllLinkKind::External,
                        });
                    }
                }
                "quote" => {
                    let nested = trim_inline_runs(parse_inlines(*child));
                    if !nested.is_empty() {
                        inlines.push(CllInline::Quote {
                            language: attr_string(*child, "lang"),
                            inlines: nested,
                        });
                    }
                }
                "emphasis" => {
                    let nested = trim_inline_runs(parse_inlines(*child));
                    if !nested.is_empty() {
                        inlines.push(CllInline::Emphasis {
                            language: attr_string(*child, "lang"),
                            inlines: nested,
                        });
                    }
                }
                "citetitle" => {
                    let nested = trim_inline_runs(parse_inlines(*child));
                    if !nested.is_empty() {
                        inlines.push(CllInline::CiteTitle { inlines: nested });
                    }
                }
                "foreignphrase" => {
                    let nested = trim_inline_runs(parse_inlines(*child));
                    if !nested.is_empty() {
                        inlines.push(CllInline::LanguageSpan {
                            kind: CllLanguageSpanKind::ForeignPhrase,
                            language: attr_string(*child, "lang"),
                            inlines: nested,
                        });
                    }
                }
                "jbophrase" => {
                    let nested = trim_inline_runs(parse_inlines(*child));
                    if !nested.is_empty() {
                        inlines.push(CllInline::LanguageSpan {
                            kind: CllLanguageSpanKind::JboPhrase,
                            language: attr_string(*child, "lang"),
                            inlines: nested,
                        });
                    }
                }
                "subscript" => {
                    let nested = trim_inline_runs(parse_inlines(*child));
                    if !nested.is_empty() {
                        inlines.push(CllInline::Subscript { inlines: nested });
                    }
                }
                "superscript" => {
                    let nested = trim_inline_runs(parse_inlines(*child));
                    if !nested.is_empty() {
                        inlines.push(CllInline::Superscript { inlines: nested });
                    }
                }
                "valsi" | "cmavo" | "gismu" | "cmevla" | "rafsi" => {
                    let text = visible_text(*child);
                    if !text.is_empty() {
                        let is_rafsi = child.has_tag_name("rafsi");
                        inlines.push(CllInline::Link {
                            target: normalize_valsis_query(&text),
                            inlines: vec![CllInline::Text(text)],
                            kind: if is_rafsi {
                                CllLinkKind::Rafsi
                            } else {
                                CllLinkKind::Dictionary
                            },
                        });
                    }
                }
                "code" | "literal" => {
                    let text = visible_text(*child);
                    if !text.is_empty() {
                        inlines.push(CllInline::Code(text));
                    }
                }
                "elidable" => {
                    let nested = trim_inline_runs(parse_inlines(*child));
                    let shown = visible_text(*child);
                    inlines.push(CllInline::Elidable {
                        shown,
                        forced: attr_string(*child, "elidable")
                            .is_some_and(|value| value.eq_ignore_ascii_case("false")),
                        inlines: nested,
                    });
                }
                "dbmath" | "dbinlinemath" | "mmlmath" | "mmlinlinemath" | "math" => {
                    let rendered = render_math_node(*child, CllMathDisplay::Inline).into_data();
                    if !rendered.text.is_empty() || !rendered.markup.is_empty() {
                        inlines.push(CllInline::InlineMath {
                            text: rendered.text,
                            latex: rendered.latex,
                            markup: rendered.markup,
                        });
                    }
                }
                _ => {
                    let nested = parse_inlines(*child);
                    if nested.is_empty() {
                        push_text_inline(&mut inlines, &visible_text(*child));
                    } else {
                        inlines.extend(nested);
                    }
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
        if !inlines.is_empty() && !normalized.is_empty() {
            inlines.push(CllInline::Text(normalized));
        }
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
#[ensures(!ret.markup.is_empty())]
fn render_math_node(node: Node<'_, '_>, display: CllMathDisplay) -> CllMathRender {
    let text = normalized_plain_text(&raw_text(node));
    let latex = render_math_latex_node(node);
    let tag_name = node.tag_name().name();
    let markup = if tag_name == "math" {
        render_math_element(node)
    } else {
        let display_attr = match display {
            CllMathDisplay::Inline => "",
            CllMathDisplay::Block => " display=\"block\"",
        };
        format!("<math{display_attr}>{}</math>", render_math_body(node))
    };
    new!(CllMathRender {
        text,
        latex,
        markup,
    })
}

#[requires(node.is_element())]
#[ensures(!ret.is_empty())]
fn render_math_body(node: Node<'_, '_>) -> String {
    let rendered = render_math_nodes(node);
    if rendered.is_empty() {
        format!(
            "<mtext>{}</mtext>",
            escape_html(&normalized_plain_text(&raw_text(node)))
        )
    } else {
        rendered
    }
}

#[requires(parent.is_element())]
#[ensures(true)]
fn render_math_nodes(parent: Node<'_, '_>) -> String {
    let mut parts = Vec::new();
    for child in parent.children() {
        if child.is_text() {
            let text = child.text().unwrap_or_default().trim();
            if !text.is_empty() {
                parts.push(format!("<mtext>{}</mtext>", escape_html(text)));
            }
        } else if child.is_element() {
            let child_tag = child.tag_name().name();
            match child_tag {
                "superscript" => attach_math_script(&mut parts, "msup", render_math_script(child)),
                "subscript" => attach_math_script(&mut parts, "msub", render_math_script(child)),
                "indexterm" => {}
                _ if is_math_ml_tag_name(child_tag) => parts.push(render_math_element(child)),
                _ => {
                    let rendered = render_math_nodes(child);
                    if !rendered.is_empty() {
                        parts.push(rendered);
                    }
                }
            }
        }
    }
    parts.concat()
}

#[requires(node.is_element())]
#[ensures(!ret.is_empty())]
fn render_math_script(node: Node<'_, '_>) -> String {
    let rendered = render_math_nodes(node);
    if rendered.is_empty() {
        "<mtext></mtext>".to_owned()
    } else {
        rendered
    }
}

#[requires(!tag_name.is_empty())]
#[ensures(true)]
fn attach_math_script(parts: &mut Vec<String>, tag_name: &str, script: String) {
    let base = parts.pop().unwrap_or_else(|| "<mtext></mtext>".to_owned());
    parts.push(format!(
        "<{tag_name}><mrow>{base}</mrow><mrow>{script}</mrow></{tag_name}>"
    ));
}

#[requires(node.is_element())]
#[ensures(!ret.is_empty())]
fn render_math_element(node: Node<'_, '_>) -> String {
    let tag_name = node.tag_name().name();
    let attrs = node
        .attributes()
        .map(|attribute| {
            format!(
                " {}=\"{}\"",
                escape_html(attribute.name()),
                escape_html(attribute.value())
            )
        })
        .collect::<String>();
    format!(
        "<{tag_name}{attrs}>{}</{tag_name}>",
        render_math_nodes(node)
    )
}

#[requires(!tag_name.is_empty())]
#[ensures(true)]
fn is_math_ml_tag_name(tag_name: &str) -> bool {
    matches!(
        tag_name,
        "math"
            | "mrow"
            | "mfrac"
            | "msqrt"
            | "mroot"
            | "msub"
            | "msup"
            | "msubsup"
            | "munder"
            | "mover"
            | "munderover"
            | "mi"
            | "mn"
            | "mo"
            | "mtext"
            | "mtable"
            | "mtr"
            | "mtd"
            | "mlabeledtr"
            | "mstyle"
            | "mspace"
            | "mfenced"
            | "menclose"
            | "semantics"
            | "annotation"
            | "annotation-xml"
    )
}

#[requires(node.is_element())]
#[ensures(true)]
fn render_math_latex_node(node: Node<'_, '_>) -> String {
    match node.tag_name().name() {
        "superscript" => format!("^{{{}}}", render_math_latex_children(node)),
        "subscript" => format!("_{{{}}}", render_math_latex_children(node)),
        "math" | "mrow" | "mstyle" | "semantics" | "annotation" | "annotation-xml" => {
            render_math_latex_children(node)
        }
        "mfrac" => {
            let children = math_latex_child_elements(node);
            let numerator = children
                .first()
                .map(|child| render_math_latex_node(*child))
                .unwrap_or_default();
            let denominator = children
                .get(1)
                .map(|child| render_math_latex_node(*child))
                .unwrap_or_default();
            format!("\\frac{{{numerator}}}{{{denominator}}}")
        }
        "msqrt" => format!("\\sqrt{{{}}}", render_math_latex_children(node)),
        "mroot" => {
            let children = math_latex_child_elements(node);
            let body = children
                .first()
                .map(|child| render_math_latex_node(*child))
                .unwrap_or_default();
            let root = children
                .get(1)
                .map(|child| render_math_latex_node(*child))
                .unwrap_or_default();
            format!("\\sqrt[{root}]{{{body}}}")
        }
        "msub" | "msup" | "msubsup" | "munder" | "mover" | "munderover" => {
            render_math_latex_scripted(node)
        }
        "mfenced" => format!("({})", render_math_latex_children(node)),
        "mtable" => format!(
            "\\begin{{matrix}}{}\\end{{matrix}}",
            render_math_latex_table(node)
        ),
        "mtr" | "mlabeledtr" => render_math_latex_row(node),
        "mtd" => render_math_latex_children(node),
        "mi" | "mn" | "mo" | "mtext" => math_latex_text(&raw_text(node)),
        "mspace" => " ".to_owned(),
        "indexterm" => String::new(),
        _ => render_math_latex_children(node),
    }
}

#[requires(node.is_element())]
#[ensures(true)]
fn render_math_latex_children(node: Node<'_, '_>) -> String {
    let mut output = String::new();
    for child in node.children() {
        if child.is_text() {
            output.push_str(&math_latex_text(child.text().unwrap_or_default()));
        } else if child.is_element() {
            output.push_str(&render_math_latex_node(child));
        }
    }
    normalized_plain_text(&output)
}

#[requires(node.is_element())]
#[ensures(true)]
fn render_math_latex_scripted(node: Node<'_, '_>) -> String {
    let children = math_latex_child_elements(node);
    let base = children
        .first()
        .map(|child| render_math_latex_node(*child))
        .unwrap_or_default();
    let first_script = children
        .get(1)
        .map(|child| render_math_latex_node(*child))
        .unwrap_or_default();
    let second_script = children
        .get(2)
        .map(|child| render_math_latex_node(*child))
        .unwrap_or_default();
    match node.tag_name().name() {
        "msub" | "munder" => format!("{base}_{{{first_script}}}"),
        "msup" | "mover" => format!("{base}^{{{first_script}}}"),
        "msubsup" | "munderover" => format!("{base}_{{{first_script}}}^{{{second_script}}}"),
        _ => base,
    }
}

#[requires(node.is_element())]
#[ensures(true)]
fn render_math_latex_table(node: Node<'_, '_>) -> String {
    node.children()
        .filter(|child| child.is_element() && !child.has_tag_name("indexterm"))
        .map(render_math_latex_node)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" \\\\ ")
}

#[requires(node.is_element())]
#[ensures(true)]
fn render_math_latex_row(node: Node<'_, '_>) -> String {
    node.children()
        .filter(|child| child.is_element() && !child.has_tag_name("indexterm"))
        .map(render_math_latex_node)
        .collect::<Vec<_>>()
        .join(" & ")
}

#[requires(node.is_element())]
#[ensures(true)]
fn math_latex_child_elements<'a, 'input>(node: Node<'a, 'input>) -> Vec<Node<'a, 'input>> {
    node.children()
        .filter(|child| child.is_element() && !child.has_tag_name("indexterm"))
        .collect()
}

#[requires(true)]
#[ensures(true)]
fn math_latex_text(text: &str) -> String {
    let normalized = normalized_plain_text(text);
    let mut output = String::new();
    for character in normalized.chars() {
        match character {
            '\u{2062}' => {}
            '×' => output.push_str("\\times"),
            '∞' => output.push_str("\\infty"),
            '≠' => output.push_str("\\ne"),
            '≤' => output.push_str("\\le"),
            '≥' => output.push_str("\\ge"),
            '%' => output.push_str("\\%"),
            '{' => output.push_str("\\{"),
            '}' => output.push_str("\\}"),
            _ => output.push(character),
        }
    }
    output
}

#[requires(node.is_element())]
#[ensures(true)]
fn is_block_element(node: Node<'_, '_>) -> bool {
    matches!(
        node.tag_name().name(),
        "para"
            | "simpara"
            | "example"
            | "itemizedlist"
            | "orderedlist"
            | "simplelist"
            | "variablelist"
            | "informaltable"
            | "table"
            | "programlisting"
            | "screen"
            | "literallayout"
            | "blockquote"
            | "mediaobject"
            | "note"
            | "tip"
            | "warning"
            | "important"
            | "caution"
            | "bridgehead"
            | "definition"
            | "dbmath"
            | "math"
            | "interlinear-gloss"
            | "interlinear-gloss-itemized"
            | "cmavo-list"
            | "lojbanization"
            | "lujvo-making"
            | "grammar-template"
    )
}

#[requires(node.is_element())]
#[ensures(true)]
fn is_display_none_element(node: Node<'_, '_>) -> bool {
    attr_string(node, "role").is_some_and(|role| role.trim().eq_ignore_ascii_case("display-none"))
}

#[requires(node.is_element())]
#[ensures(true)]
fn non_title_child_nodes<'a, 'input>(node: Node<'a, 'input>) -> Vec<Node<'a, 'input>> {
    node.children()
        .filter(|child| {
            child.is_text()
                || (child.is_element()
                    && !is_display_none_element(*child)
                    && !child.has_tag_name("title")
                    && !child.has_tag_name("indexterm"))
        })
        .collect()
}

#[requires(node.is_element())]
#[ensures(true)]
fn paragraph_anchor_id_for(
    anchor_mode: AnchorMode,
    context: &SectionParseContext,
    node: Node<'_, '_>,
) -> Option<String> {
    xml_id(node).or_else(|| match anchor_mode {
        AnchorMode::TopLevel => Some(synthetic_anchor_id("para", context, node)),
        AnchorMode::Nested => None,
    })
}

#[requires(node.is_element())]
#[requires(!prefix.is_empty())]
#[ensures(true)]
fn block_anchor_id_for(
    prefix: &str,
    anchor_mode: AnchorMode,
    context: &SectionParseContext,
    node: Node<'_, '_>,
) -> Option<String> {
    xml_id(node).or_else(|| match anchor_mode {
        AnchorMode::TopLevel => Some(synthetic_anchor_id(prefix, context, node)),
        AnchorMode::Nested => None,
    })
}

#[requires(node.is_element())]
#[requires(!prefix.is_empty())]
#[ensures(!ret.is_empty())]
fn synthetic_anchor_id(prefix: &str, context: &SectionParseContext, node: Node<'_, '_>) -> String {
    let mut hasher = Sha256::new();
    hasher.update(prefix.as_bytes());
    hasher.update(b"|");
    hasher.update(context.section_id.as_bytes());
    hasher.update(b"|");
    hasher.update(normalized_plain_text(&visible_text_raw(node)).as_bytes());
    let digest = hasher.finalize();
    let hex = digest
        .iter()
        .take(10)
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("cll-{prefix}-{hex}")
}

#[requires(node.is_element())]
#[ensures(true)]
fn attr_usize(node: Node<'_, '_>, name: &str) -> Option<usize> {
    attr_string(node, name)
        .and_then(|value| value.trim().parse::<usize>().ok())
        .filter(|value| *value > 1)
}

#[requires(true)]
#[ensures(true)]
fn trim_inline_runs(inlines: Vec<CllInline>) -> Vec<CllInline> {
    let start = inlines
        .iter()
        .position(|inline| !inline_is_whitespace(inline))
        .unwrap_or(inlines.len());
    let end = inlines
        .iter()
        .rposition(|inline| !inline_is_whitespace(inline))
        .map(|index| index + 1)
        .unwrap_or(start);
    inlines[start..end].to_vec()
}

#[requires(true)]
#[ensures(true)]
fn inline_is_whitespace(inline: &CllInline) -> bool {
    matches!(inline, CllInline::Text(text) if text.chars().all(char::is_whitespace))
}

#[requires(true)]
#[ensures(ret.chars().all(|character| character.is_ascii_lowercase() || character.is_ascii_digit() || character == '\''))]
fn normalize_valsis_query(text: &str) -> String {
    text.trim()
        .trim_matches('.')
        .to_ascii_lowercase()
        .replace('h', "'")
        .chars()
        .filter(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || *character == '\''
        })
        .collect()
}

#[requires(true)]
#[ensures(true)]
fn linked_jbo_text_inlines(text: &str) -> Vec<CllInline> {
    let mut inlines = Vec::new();
    let mut current = String::new();
    let mut in_space = None::<bool>;
    for character in text.chars() {
        let character_is_space = character.is_whitespace();
        if in_space.is_some_and(|value| value != character_is_space) && !current.is_empty() {
            push_jbo_run(&mut inlines, &current, in_space.unwrap_or(false));
            current.clear();
        }
        in_space = Some(character_is_space);
        current.push(character);
    }
    if !current.is_empty() {
        push_jbo_run(&mut inlines, &current, in_space.unwrap_or(false));
    }
    inlines
}

#[requires(true)]
#[ensures(true)]
fn push_jbo_run(inlines: &mut Vec<CllInline>, run: &str, is_space: bool) {
    if is_space {
        inlines.push(CllInline::Text(run.to_owned()));
        return;
    }
    for (index, segment) in run.split("--").enumerate() {
        if index > 0 {
            inlines.push(CllInline::Text("--".to_owned()));
        }
        if segment.is_empty() {
            continue;
        }
        let query = normalize_valsis_query(segment);
        if query
            .chars()
            .any(|character| character.is_ascii_alphabetic() || character == '\'')
        {
            inlines.push(CllInline::Link {
                target: query,
                inlines: vec![CllInline::Text(segment.to_owned())],
                kind: CllLinkKind::Dictionary,
            });
        } else {
            inlines.push(CllInline::Text(segment.to_owned()));
        }
    }
}

#[requires(node.is_element())]
#[ensures(true)]
fn parse_interlinear_gloss_block(
    node: Node<'_, '_>,
    context: &SectionParseContext,
    anchor_mode: AnchorMode,
) -> Option<CllBlock> {
    let line_elements = node
        .children()
        .filter(|child| child.is_element() && !child.has_tag_name("indexterm"))
        .collect::<Vec<_>>();
    let maybe_aligned = aligned_interlinear_rows(&line_elements);
    let rows = maybe_aligned.clone().unwrap_or_else(|| {
        line_elements
            .iter()
            .filter(|line| !line.has_tag_name("natlang") && !line.has_tag_name("comment"))
            .filter_map(|line| plain_interlinear_row(*line))
            .collect()
    });
    let natlang = interlinear_side_lines(&line_elements, "natlang");
    let comments = interlinear_side_lines(&line_elements, "comment");
    (!rows.is_empty() || !natlang.is_empty() || !comments.is_empty()).then_some(
        CllBlock::InterlinearGloss {
            id: block_anchor_id_for("interlinear", anchor_mode, context, node),
            aligned: maybe_aligned.is_some(),
            itemized: false,
            parse_href: top_level_jbo_parse_href(anchor_mode, node),
            rows,
            natlang,
            comments,
        },
    )
}

#[requires(true)]
#[ensures(true)]
fn aligned_interlinear_rows(line_elements: &[Node<'_, '_>]) -> Option<Vec<CllInterlinearRow>> {
    let jbo_line = single_named_line(line_elements, "jbo")?;
    let gloss_line = single_named_line(line_elements, "gloss")?;
    if line_elements.iter().any(|line| {
        !matches!(
            line.tag_name().name(),
            "jbo" | "gloss" | "natlang" | "comment"
        )
    }) {
        return None;
    }
    let jbo_tokens = normalized_plain_text(&visible_text_raw(jbo_line))
        .split_whitespace()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let gloss_tokens = normalized_plain_text(&visible_text_raw(gloss_line))
        .split_whitespace()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if jbo_tokens.len() <= 1 || jbo_tokens.len() != gloss_tokens.len() {
        return None;
    }
    Some(vec![
        new!(CllInterlinearRow {
            kind: "jbo".to_owned(),
            cells: jbo_tokens
                .iter()
                .map(|token| linked_jbo_text_inlines(token))
                .collect(),
        }),
        new!(CllInterlinearRow {
            kind: "gloss".to_owned(),
            cells: gloss_tokens
                .into_iter()
                .map(|token| vec![CllInline::Text(token)])
                .collect(),
        }),
    ])
}

#[requires(true)]
#[ensures(true)]
fn single_named_line<'a, 'input>(
    line_elements: &[Node<'a, 'input>],
    name: &str,
) -> Option<Node<'a, 'input>> {
    let mut matches = line_elements
        .iter()
        .copied()
        .filter(|line| line.has_tag_name(name));
    let first = matches.next()?;
    matches.next().is_none().then_some(first)
}

#[requires(line.is_element())]
#[ensures(true)]
fn plain_interlinear_row(line: Node<'_, '_>) -> Option<CllInterlinearRow> {
    let kind = line.tag_name().name().to_owned();
    let body = if line.has_tag_name("jbo") || line.has_tag_name("jbophrase") {
        linked_jbo_text_inlines(&normalized_plain_text(&visible_text_raw(line)))
    } else if line.has_tag_name("dbmath")
        || line.has_tag_name("mmlmath")
        || line.has_tag_name("math")
    {
        let rendered = render_math_node(line, CllMathDisplay::Inline).into_data();
        vec![CllInline::InlineMath {
            text: rendered.text,
            latex: rendered.latex,
            markup: rendered.markup,
        }]
    } else {
        trim_inline_runs(parse_inlines(line))
    };
    (!body.is_empty()).then_some(new!(CllInterlinearRow {
        kind,
        cells: vec![body],
    }))
}

#[requires(true)]
#[ensures(true)]
fn interlinear_side_lines(line_elements: &[Node<'_, '_>], name: &str) -> Vec<Vec<CllInline>> {
    line_elements
        .iter()
        .filter(|line| line.has_tag_name(name))
        .map(|line| trim_inline_runs(parse_inlines(*line)))
        .filter(|body| !body.is_empty())
        .collect()
}

#[requires(node.is_element())]
#[ensures(true)]
fn parse_interlinear_gloss_itemized_block(
    node: Node<'_, '_>,
    context: &SectionParseContext,
    anchor_mode: AnchorMode,
) -> Option<CllBlock> {
    let line_elements = node
        .children()
        .filter(|child| child.is_element() && !child.has_tag_name("indexterm"))
        .collect::<Vec<_>>();
    let rows = line_elements
        .iter()
        .filter(|line| !line.has_tag_name("natlang") && !line.has_tag_name("comment"))
        .filter_map(|line| itemized_interlinear_row(*line))
        .collect::<Vec<_>>();
    let natlang = interlinear_side_lines(&line_elements, "natlang");
    let comments = interlinear_side_lines(&line_elements, "comment");
    (!rows.is_empty() || !natlang.is_empty() || !comments.is_empty()).then_some(
        CllBlock::InterlinearGloss {
            id: block_anchor_id_for("interlinear", anchor_mode, context, node),
            aligned: true,
            itemized: true,
            parse_href: top_level_jbo_parse_href(anchor_mode, node),
            rows,
            natlang,
            comments,
        },
    )
}

#[requires(line.is_element())]
#[ensures(true)]
fn itemized_interlinear_row(line: Node<'_, '_>) -> Option<CllInterlinearRow> {
    let kind = line.tag_name().name().to_owned();
    let cells = line
        .children()
        .flat_map(|child| collect_interlinear_cell(child, &kind))
        .map(trim_inline_runs)
        .filter(|cell| !cell.is_empty())
        .collect::<Vec<_>>();
    (!cells.is_empty()).then_some(new!(CllInterlinearRow { kind, cells }))
}

#[requires(true)]
#[ensures(true)]
fn collect_interlinear_cell(node: Node<'_, '_>, kind: &str) -> Vec<Vec<CllInline>> {
    if node.is_text() {
        let text = normalized_plain_text(node.text().unwrap_or_default());
        if text.is_empty() {
            return Vec::new();
        }
        return vec![if kind == "jbo" {
            linked_jbo_text_inlines(&text)
        } else {
            vec![CllInline::Text(text)]
        }];
    }
    if !node.is_element() || node.has_tag_name("indexterm") {
        return Vec::new();
    }
    if kind == "jbo" {
        if node.has_tag_name("elidable") {
            return vec![vec![CllInline::Elidable {
                shown: visible_text(node),
                forced: attr_string(node, "elidable")
                    .is_some_and(|value| value.eq_ignore_ascii_case("false")),
                inlines: linked_jbo_text_inlines(&visible_text(node)),
            }]];
        }
        return vec![linked_jbo_text_inlines(&visible_text(node))];
    }
    if node.has_tag_name("dbmath") || node.has_tag_name("mmlmath") || node.has_tag_name("math") {
        let rendered = render_math_node(node, CllMathDisplay::Inline).into_data();
        return vec![vec![CllInline::InlineMath {
            text: rendered.text,
            latex: rendered.latex,
            markup: rendered.markup,
        }]];
    }
    vec![parse_inlines(node)]
}

#[requires(node.is_element())]
#[ensures(true)]
fn parse_cmavo_list_block(
    node: Node<'_, '_>,
    context: &SectionParseContext,
    anchor_mode: AnchorMode,
) -> Option<CllBlock> {
    let entry_rows = node
        .children()
        .filter(|child| child.is_element() && child.has_tag_name("cmavo-entry"))
        .map(|entry| {
            entry
                .children()
                .filter(Node::is_element)
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let column_count = entry_rows.iter().map(Vec::len).max().unwrap_or(0);
    if column_count == 0 {
        return None;
    }
    let header_cells = child_element(node, "cmavo-list-head")
        .map(|head| head.children().filter(Node::is_element).collect::<Vec<_>>())
        .unwrap_or_default();
    let headers = (0..column_count)
        .map(|index| {
            header_cells
                .get(index)
                .map(|cell| trim_inline_runs(parse_inlines(*cell)))
                .filter(|body| !body.is_empty())
                .unwrap_or_else(|| vec![CllInline::Text(cmavo_column_label(index))])
        })
        .collect::<Vec<_>>();
    let rows = entry_rows
        .iter()
        .map(|entry_cells| {
            (0..column_count)
                .map(|index| {
                    entry_cells
                        .get(index)
                        .map(|cell| trim_inline_runs(parse_inlines(*cell)))
                        .unwrap_or_default()
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let titles = node
        .children()
        .filter(|child| child.is_element() && child.has_tag_name("title"))
        .map(parse_inlines)
        .map(trim_inline_runs)
        .filter(|body| !body.is_empty())
        .collect::<Vec<_>>();
    Some(CllBlock::CmavoList {
        id: block_anchor_id_for("cmavo-list", anchor_mode, context, node),
        titles,
        headers,
        rows,
    })
}

#[requires(true)]
#[ensures(true)]
fn cmavo_column_label(index: usize) -> String {
    match index {
        0 => "cmavo",
        1 => "selma'o",
        2 => "description",
        _ => "",
    }
    .to_owned()
}

#[requires(node.is_element())]
#[ensures(true)]
fn parse_lojbanization_block(
    node: Node<'_, '_>,
    context: &SectionParseContext,
    anchor_mode: AnchorMode,
) -> Option<CllBlock> {
    let lines = node
        .children()
        .filter(|child| child.is_element() && !child.has_tag_name("indexterm"))
        .filter_map(|line| {
            let kind = line.tag_name().name().to_owned();
            let body_nodes = line
                .children()
                .filter(|child| {
                    child.is_text()
                        || (child.is_element()
                            && !child.has_tag_name("comment")
                            && !child.has_tag_name("indexterm"))
                })
                .collect::<Vec<_>>();
            let body = if kind == "jbo" {
                linked_jbo_text_inlines(&normalized_plain_text(&visible_text_raw(line)))
            } else {
                trim_inline_runs(parse_inline_nodes(&body_nodes))
            };
            let comment = child_element(line, "comment")
                .map(parse_inlines)
                .map(trim_inline_runs)
                .filter(|value| !value.is_empty());
            (!body.is_empty() || comment.is_some()).then_some(new!(CllLojbanizationLine {
                kind,
                body,
                comment,
            }))
        })
        .collect::<Vec<_>>();
    (!lines.is_empty()).then_some(CllBlock::Lojbanization {
        id: block_anchor_id_for("lojbanization", anchor_mode, context, node),
        lines,
    })
}

#[requires(node.is_element())]
#[ensures(true)]
fn parse_lujvo_making_block(
    node: Node<'_, '_>,
    context: &SectionParseContext,
    anchor_mode: AnchorMode,
) -> Option<CllBlock> {
    let parts = node
        .children()
        .filter(|child| child.is_element() && !child.has_tag_name("indexterm"))
        .filter_map(|part| {
            let kind = part.tag_name().name().to_owned();
            let body = if matches!(kind.as_str(), "jbo" | "veljvo" | "rafsi") {
                linked_jbo_text_inlines(&normalized_plain_text(&visible_text_raw(part)))
            } else {
                trim_inline_runs(parse_inlines(part))
            };
            (!body.is_empty()).then_some(new!(CllLujvoPart { kind, body }))
        })
        .collect::<Vec<_>>();
    (!parts.is_empty()).then_some(CllBlock::LujvoMaking {
        id: block_anchor_id_for("lujvo-making", anchor_mode, context, node),
        parts,
    })
}

#[requires(node.is_element())]
#[ensures(true)]
fn parse_ebnf_block(node: Node<'_, '_>, context: &SectionParseContext) -> Option<CllBlock> {
    let entry_nodes = node
        .children()
        .filter(|child| child.is_element() && child.has_tag_name("varlistentry"))
        .collect::<Vec<_>>();
    let defined_rules = entry_nodes
        .iter()
        .filter_map(|entry| child_element(*entry, "term"))
        .map(|term| extract_ebnf_rule_name(&visible_text(term)))
        .filter(|rule| !rule.is_empty())
        .collect::<BTreeSet<_>>();
    let entries = entry_nodes
        .iter()
        .filter_map(|entry| parse_ebnf_entry(*entry, &defined_rules))
        .collect::<Vec<_>>();
    (!entries.is_empty()).then_some(CllBlock::Ebnf {
        id: block_anchor_id_for("ebnf", AnchorMode::TopLevel, context, node),
        entries,
    })
}

#[requires(entry.is_element())]
#[ensures(true)]
fn parse_ebnf_entry(entry: Node<'_, '_>, defined_rules: &BTreeSet<String>) -> Option<CllEbnfEntry> {
    let term = child_element(entry, "term")?;
    let listitem = child_element(entry, "listitem")?;
    let para = child_element(listitem, "para")?;
    let rule_name = extract_ebnf_rule_name(&visible_text(term));
    let rhs_text = normalized_plain_text(&visible_text_raw(para));
    if rule_name.is_empty() || rhs_text.is_empty() {
        return None;
    }
    Some(new!(CllEbnfEntry {
        anchor_id: ebnf_rule_anchor_id(&rule_name),
        rule_href: ebnf_symbol_href(&rule_name),
        rhs: tokenize_ebnf_rule_rhs(&rule_name, defined_rules, &rhs_text),
        rule_name,
    }))
}

#[requires(true)]
#[ensures(true)]
fn extract_ebnf_rule_name(text: &str) -> String {
    text.trim()
        .chars()
        .take_while(|character| {
            character.is_ascii_alphanumeric() || *character == '-' || *character == '\''
        })
        .collect()
}

#[requires(!rule_name.is_empty())]
#[ensures(ret.starts_with("ebnf-rule-"))]
pub fn ebnf_rule_anchor_id(rule_name: &str) -> String {
    let slug = rule_name
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    format!("ebnf-rule-{slug}")
}

#[requires(true)]
#[ensures(true)]
fn tokenize_ebnf_rule_rhs(
    rule_name: &str,
    defined_rules: &BTreeSet<String>,
    rhs_text: &str,
) -> Vec<CllEbnfToken> {
    if rule_name == "any-word" || rule_name == "anything" {
        return vec![CllEbnfToken::Text {
            body: rhs_text.to_owned(),
        }];
    }
    tokenize_ebnf_tokens(defined_rules, rhs_text)
}

#[requires(true)]
#[ensures(true)]
fn tokenize_ebnf_tokens(defined_rules: &BTreeSet<String>, rhs_text: &str) -> Vec<CllEbnfToken> {
    let chars = rhs_text.chars().collect::<Vec<_>>();
    let mut index = 0usize;
    let mut tokens = Vec::new();
    while index < chars.len() {
        let character = chars[index];
        if character.is_whitespace() {
            let start = index;
            while index < chars.len() && chars[index].is_whitespace() {
                index += 1;
            }
            tokens.push(CllEbnfToken::Text {
                body: chars[start..index].iter().collect(),
            });
        } else if character == '\u{201c}' {
            let start = index;
            index += 1;
            while index < chars.len() && chars[index] != '\u{201d}' {
                index += 1;
            }
            if index < chars.len() {
                index += 1;
            }
            tokens.push(CllEbnfToken::Text {
                body: chars[start..index].iter().collect(),
            });
        } else if character == '/' {
            if let Some((body, symbol, next_index)) = parse_ebnf_elidable(&chars, index) {
                tokens.push(CllEbnfToken::ElidableTerminator {
                    body,
                    href: ebnf_symbol_href(&symbol),
                });
                index = next_index;
            } else {
                tokens.push(CllEbnfToken::Operator {
                    body: character.to_string(),
                });
                index += 1;
            }
        } else if index + 3 <= chars.len() && chars[index..index + 3] == ['.', '.', '.'] {
            tokens.push(CllEbnfToken::Operator {
                body: "...".to_owned(),
            });
            index += 3;
        } else if is_ebnf_boundary(character) {
            let body = character.to_string();
            if character == '#' {
                tokens.push(CllEbnfToken::Hash { body });
            } else {
                tokens.push(CllEbnfToken::Operator { body });
            }
            index += 1;
        } else if is_ebnf_identifier_char(character) {
            let start = index;
            while index < chars.len() && is_ebnf_identifier_char(chars[index]) {
                index += 1;
            }
            let body = chars[start..index].iter().collect::<String>();
            tokens.push(classify_ebnf_identifier(&body, defined_rules));
        } else {
            tokens.push(CllEbnfToken::Text {
                body: character.to_string(),
            });
            index += 1;
        }
    }
    tokens
}

#[requires(index < chars.len())]
#[ensures(true)]
fn parse_ebnf_elidable(chars: &[char], index: usize) -> Option<(String, String, usize)> {
    let mut cursor = index + 1;
    let symbol_start = cursor;
    while cursor < chars.len() && is_ebnf_identifier_char(chars[cursor]) {
        cursor += 1;
    }
    if cursor == symbol_start {
        return None;
    }
    let symbol = chars[symbol_start..cursor].iter().collect::<String>();
    if cursor < chars.len() && chars[cursor] == '/' {
        cursor += 1;
        return Some((chars[index..cursor].iter().collect(), symbol, cursor));
    }
    if cursor + 1 < chars.len() && chars[cursor] == '#' && chars[cursor + 1] == '/' {
        cursor += 2;
        return Some((chars[index..cursor].iter().collect(), symbol, cursor));
    }
    None
}

#[requires(true)]
#[ensures(true)]
fn classify_ebnf_identifier(body: &str, defined_rules: &BTreeSet<String>) -> CllEbnfToken {
    if let Some(href) = ebnf_symbol_href(body) {
        return CllEbnfToken::Terminal {
            body: body.to_owned(),
            href: Some(href),
        };
    }
    if defined_rules.contains(body) {
        return CllEbnfToken::Nonterminal {
            body: body.to_owned(),
            href: Some(format!("#{}", ebnf_rule_anchor_id(body))),
        };
    }
    let letters = body
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .collect::<Vec<_>>();
    if !letters.is_empty()
        && letters
            .iter()
            .all(|character| !character.is_ascii_lowercase())
    {
        return CllEbnfToken::Terminal {
            body: body.to_owned(),
            href: ebnf_symbol_href(body),
        };
    }
    if letters
        .iter()
        .any(|character| character.is_ascii_lowercase())
    {
        return CllEbnfToken::Nonterminal {
            body: body.to_owned(),
            href: None,
        };
    }
    CllEbnfToken::Text {
        body: body.to_owned(),
    }
}

#[requires(true)]
#[ensures(true)]
fn ebnf_symbol_href(symbol: &str) -> Option<String> {
    match symbol {
        "BRIVLA" => Some(section_href("section-morphology-brivla")),
        "CMEVLA" => Some(section_href("section-cmevla")),
        "any-word" | "anything" => Some(section_href("section-more-quotations")),
        "null" => Some(section_href("section-erasure")),
        _ if symbol
            .chars()
            .any(|character| character.is_ascii_uppercase()) =>
        {
            Some(format!("{}#{symbol}", section_href("section-index")))
        }
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn is_ebnf_boundary(character: char) -> bool {
    matches!(character, '|' | '&' | '[' | ']' | '(' | ')' | '=' | '#')
}

#[requires(true)]
#[ensures(true)]
fn is_ebnf_identifier_char(character: char) -> bool {
    character.is_ascii_alphanumeric() || character == '-' || character == '\''
}

#[requires(true)]
#[ensures(true)]
pub fn wrap_ebnf_choice_lines(tokens: &[CllEbnfToken]) -> Vec<Vec<CllEbnfToken>> {
    let mut lines = Vec::new();
    let mut current = Vec::new();
    let mut depth = 0usize;
    for token in tokens {
        if depth == 0 && matches!(token, CllEbnfToken::Operator { body } if body == "|") {
            current.push(token.clone());
            push_trimmed_ebnf_line(&mut lines, &mut current);
        } else {
            depth = next_ebnf_depth(depth, token);
            current.push(token.clone());
        }
    }
    push_trimmed_ebnf_line(&mut lines, &mut current);
    if lines.len() <= 1 {
        vec![tokens.to_vec()]
    } else {
        lines
    }
}

#[requires(true)]
#[ensures(current.is_empty())]
fn push_trimmed_ebnf_line(lines: &mut Vec<Vec<CllEbnfToken>>, current: &mut Vec<CllEbnfToken>) {
    let line = std::mem::take(current);
    let start = line
        .iter()
        .position(|token| !ebnf_token_is_whitespace(token))
        .unwrap_or(line.len());
    let end = line
        .iter()
        .rposition(|token| !ebnf_token_is_whitespace(token))
        .map(|index| index + 1)
        .unwrap_or(start);
    if start < end {
        lines.push(line[start..end].to_vec());
    }
}

#[requires(true)]
#[ensures(true)]
fn ebnf_token_is_whitespace(token: &CllEbnfToken) -> bool {
    matches!(token, CllEbnfToken::Text { body } if body.chars().all(char::is_whitespace))
}

#[requires(true)]
#[ensures(true)]
fn next_ebnf_depth(depth: usize, token: &CllEbnfToken) -> usize {
    match token {
        CllEbnfToken::Operator { body } if body == "[" || body == "(" => depth + 1,
        CllEbnfToken::Operator { body } if body == "]" || body == ")" => depth.saturating_sub(1),
        _ => depth,
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|href| href.starts_with("../gentufa?text=")))]
fn jbo_parse_href(snippet: &str) -> Option<String> {
    (!snippet.is_empty()).then(|| format!("../gentufa?text={}", percent_encode_plain(snippet)))
}

#[requires(node.is_element())]
#[ensures(ret.as_ref().is_none_or(|href| href.starts_with("../gentufa?text=")))]
fn top_level_jbo_parse_href(anchor_mode: AnchorMode, node: Node<'_, '_>) -> Option<String> {
    (anchor_mode == AnchorMode::TopLevel)
        .then(|| collect_jbo_snippet(node).and_then(|snippet| jbo_parse_href(&snippet)))
        .flatten()
}

#[requires(node.is_element())]
#[ensures(true)]
fn collect_jbo_snippet(node: Node<'_, '_>) -> Option<String> {
    let lines = node
        .descendants()
        .filter(|descendant| {
            descendant.is_element()
                && (descendant.has_tag_name("jbo") || descendant.has_tag_name("jbophrase"))
        })
        .map(|line| normalized_plain_text(&visible_text_raw(line)))
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    let (first, rest) = lines.split_first()?;
    let mut parts = Vec::with_capacity(lines.len());
    parts.push(first.clone());
    parts.extend(rest.iter().map(|line| {
        line.trim_start()
            .trim_start_matches("...")
            .trim_start_matches('\u{2026}')
            .trim_start()
            .to_owned()
    }));
    Some(parts.join(" "))
}

#[requires(true)]
#[ensures(true)]
fn percent_encode_plain(text: &str) -> String {
    let mut output = String::new();
    for byte in text.as_bytes() {
        match *byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                output.push(char::from(*byte));
            }
            b' ' => output.push_str("%20"),
            value => output.push_str(&format!("%{value:02X}")),
        }
    }
    output
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
        if let Some(example_number) = &example.reference.example_number {
            insert_reference(&mut index, example_number, id);
        }
        if let Some(example_id) = &example.reference.example_id {
            insert_reference(&mut index, example_id, id);
        }
    }
    for (anchor_id, anchor) in &site.anchors_by_id {
        if let Some(example_label) = anchor.label.strip_prefix("Example ") {
            if let Some(example) = site.examples_by_id.values().find(|example| {
                example.label == anchor.label
                    || example
                        .reference
                        .example_number
                        .as_deref()
                        .is_some_and(|number| number == example_label)
            }) {
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
                label: example.label.clone(),
                kind: CllLinkKind::Example,
            },
        );
        insert_link_resolution(
            &mut resolutions,
            &example.label,
            LinkResolution {
                label: example.label.clone(),
                kind: CllLinkKind::Example,
            },
        );
        if let Some(example_number) = &example.reference.example_number {
            insert_link_resolution(
                &mut resolutions,
                example_number,
                LinkResolution {
                    label: example.label.clone(),
                    kind: CllLinkKind::Example,
                },
            );
        }
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
            CllBlock::Table {
                caption,
                header_rows,
                body_rows,
                ..
            } => {
                if let Some(caption) = caption {
                    resolve_inline_links(caption, resolutions);
                }
                for row in header_rows.iter_mut().chain(body_rows.iter_mut()) {
                    *row = std::mem::take(row)
                        .into_iter()
                        .map(|cell| resolve_table_cell_links(cell, resolutions))
                        .collect();
                }
            }
            CllBlock::SimpleListTable { rows, .. } => {
                for row in rows {
                    for cell in row.iter_mut().flatten() {
                        resolve_inline_links(cell, resolutions);
                    }
                }
            }
            CllBlock::VariableList { entries, .. } => {
                *entries = std::mem::take(entries)
                    .into_iter()
                    .map(|entry| resolve_variable_entry_links(entry, resolutions))
                    .collect();
            }
            CllBlock::Rule { body, .. } => resolve_block_links(body, resolutions),
            CllBlock::Example(example) => resolve_block_links(&mut example.blocks, resolutions),
            CllBlock::Media { title, .. } => {
                if let Some(title) = title {
                    resolve_inline_links(title, resolutions);
                }
            }
            CllBlock::BlockQuote { blocks, .. } => resolve_block_links(blocks, resolutions),
            CllBlock::Definition { body, .. } | CllBlock::GrammarTemplate { body, .. } => {
                resolve_inline_links(body, resolutions);
            }
            CllBlock::InterlinearGloss {
                rows,
                natlang,
                comments,
                ..
            } => {
                *rows = std::mem::take(rows)
                    .into_iter()
                    .map(|row| resolve_interlinear_row_links(row, resolutions))
                    .collect();
                for line in natlang.iter_mut().chain(comments.iter_mut()) {
                    resolve_inline_links(line, resolutions);
                }
            }
            CllBlock::CmavoList {
                titles,
                headers,
                rows,
                ..
            } => {
                for title in titles.iter_mut().chain(headers.iter_mut()) {
                    resolve_inline_links(title, resolutions);
                }
                for row in rows {
                    for cell in row {
                        resolve_inline_links(cell, resolutions);
                    }
                }
            }
            CllBlock::Lojbanization { lines, .. } => {
                *lines = std::mem::take(lines)
                    .into_iter()
                    .map(|line| resolve_lojbanization_line_links(line, resolutions))
                    .collect();
            }
            CllBlock::LujvoMaking { parts, .. } => {
                *parts = std::mem::take(parts)
                    .into_iter()
                    .map(|part| resolve_lujvo_part_links(part, resolutions))
                    .collect();
            }
            CllBlock::Heading { inlines, title, .. } => {
                resolve_inline_links(inlines, resolutions);
                *title = normalized_plain_text(&inline_plain_text(inlines));
            }
            CllBlock::Code { .. } | CllBlock::Ebnf { .. } | CllBlock::DisplayMath { .. } => {}
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn resolve_table_cell_links(
    cell: CllTableCell,
    resolutions: &BTreeMap<String, LinkResolution>,
) -> CllTableCell {
    let data = cell.into_data();
    let mut blocks = data.blocks;
    resolve_block_links(&mut blocks, resolutions);
    CllTableCell::from_data(data!(CllTableCell { blocks, ..data }))
}

#[requires(true)]
#[ensures(true)]
fn resolve_variable_entry_links(
    entry: CllVariableEntry,
    resolutions: &BTreeMap<String, LinkResolution>,
) -> CllVariableEntry {
    let data = entry.into_data();
    let mut term = data.term;
    let mut blocks = data.blocks;
    resolve_inline_links(&mut term, resolutions);
    resolve_block_links(&mut blocks, resolutions);
    CllVariableEntry::from_data(data!(CllVariableEntry { term, blocks }))
}

#[requires(true)]
#[ensures(true)]
fn resolve_interlinear_row_links(
    row: CllInterlinearRow,
    resolutions: &BTreeMap<String, LinkResolution>,
) -> CllInterlinearRow {
    let data = row.into_data();
    let mut cells = data.cells;
    for cell in &mut cells {
        resolve_inline_links(cell, resolutions);
    }
    CllInterlinearRow::from_data(data!(CllInterlinearRow {
        kind: data.kind,
        cells,
    }))
}

#[requires(true)]
#[ensures(true)]
fn resolve_lojbanization_line_links(
    line: CllLojbanizationLine,
    resolutions: &BTreeMap<String, LinkResolution>,
) -> CllLojbanizationLine {
    let data = line.into_data();
    let mut body = data.body;
    let mut comment = data.comment;
    resolve_inline_links(&mut body, resolutions);
    if let Some(comment) = &mut comment {
        resolve_inline_links(comment, resolutions);
    }
    CllLojbanizationLine::from_data(data!(CllLojbanizationLine {
        kind: data.kind,
        body,
        comment,
    }))
}

#[requires(true)]
#[ensures(true)]
fn resolve_lujvo_part_links(
    part: CllLujvoPart,
    resolutions: &BTreeMap<String, LinkResolution>,
) -> CllLujvoPart {
    let data = part.into_data();
    let mut body = data.body;
    resolve_inline_links(&mut body, resolutions);
    CllLujvoPart::from_data(data!(CllLujvoPart {
        kind: data.kind,
        body,
    }))
}

#[requires(true)]
#[ensures(true)]
fn resolve_inline_links(inlines: &mut [CllInline], resolutions: &BTreeMap<String, LinkResolution>) {
    for inline in inlines {
        match inline {
            CllInline::Link {
                target,
                inlines,
                kind,
            } => {
                resolve_inline_links(inlines, resolutions);
                if *kind == CllLinkKind::Section
                    && let Some(resolution) = resolutions.get(target)
                {
                    *kind = resolution.kind;
                    if inline_plain_text(inlines) == *target {
                        *inlines = vec![CllInline::Text(resolution.label.clone())];
                    }
                }
            }
            CllInline::Emphasis { inlines, .. }
            | CllInline::Quote { inlines, .. }
            | CllInline::LanguageSpan { inlines, .. }
            | CllInline::CiteTitle { inlines }
            | CllInline::Subscript { inlines }
            | CllInline::Superscript { inlines }
            | CllInline::Elidable { inlines, .. } => resolve_inline_links(inlines, resolutions),
            CllInline::Text(_)
            | CllInline::Code(_)
            | CllInline::InlineMath { .. }
            | CllInline::Anchor { .. } => {}
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
                collect_block_search_chunks(section, &example.blocks, chunks);
            }
            CllBlock::List { items, .. } => {
                for item in items {
                    collect_block_search_chunks(section, item, chunks);
                }
            }
            CllBlock::Table {
                header_rows,
                body_rows,
                ..
            } => {
                for row in header_rows.iter().chain(body_rows.iter()) {
                    for cell in row {
                        collect_block_search_chunks(section, &cell.blocks, chunks);
                    }
                }
            }
            CllBlock::VariableList { entries, .. } => {
                for entry in entries {
                    collect_block_search_chunks(section, &entry.blocks, chunks);
                }
            }
            CllBlock::Rule { body, .. } => collect_block_search_chunks(section, body, chunks),
            CllBlock::BlockQuote { blocks, .. } => {
                collect_block_search_chunks(section, blocks, chunks)
            }
            CllBlock::SimpleListTable { .. }
            | CllBlock::Media { .. }
            | CllBlock::Code { .. }
            | CllBlock::Heading { .. }
            | CllBlock::Definition { .. }
            | CllBlock::InterlinearGloss { .. }
            | CllBlock::CmavoList { .. }
            | CllBlock::Lojbanization { .. }
            | CllBlock::LujvoMaking { .. }
            | CllBlock::GrammarTemplate { .. }
            | CllBlock::Ebnf { .. }
            | CllBlock::DisplayMath { .. } => {}
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
                    if line.kind == "text" {
                        output.push_str(&line.text);
                        output.push('\n');
                    } else {
                        output.push_str(&format!("{}: {}\n", line.kind, line.text));
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
        CllBlock::Table {
            header_rows,
            body_rows,
            ..
        } => header_rows
            .iter()
            .chain(body_rows.iter())
            .flat_map(|row| {
                row.iter()
                    .flat_map(|cell| blocks_tagged_words(&cell.blocks))
            })
            .collect(),
        CllBlock::VariableList { entries, .. } => entries
            .iter()
            .flat_map(|entry| blocks_tagged_words(&entry.blocks))
            .collect(),
        CllBlock::Rule { body, .. } => blocks_tagged_words(body),
        CllBlock::BlockQuote { blocks, .. } => blocks_tagged_words(blocks),
        CllBlock::Heading { inlines, .. } => inlines_tagged_words(inlines),
        CllBlock::SimpleListTable { .. }
        | CllBlock::Media { .. }
        | CllBlock::Code { .. }
        | CllBlock::Definition { .. }
        | CllBlock::InterlinearGloss { .. }
        | CllBlock::CmavoList { .. }
        | CllBlock::Lojbanization { .. }
        | CllBlock::LujvoMaking { .. }
        | CllBlock::GrammarTemplate { .. }
        | CllBlock::Ebnf { .. }
        | CllBlock::DisplayMath { .. } => BTreeSet::new(),
    }
}

#[requires(true)]
#[ensures(true)]
fn inlines_tagged_words(inlines: &[CllInline]) -> BTreeSet<String> {
    let mut words = BTreeSet::new();
    for inline in inlines {
        match inline {
            CllInline::Link {
                target,
                inlines,
                kind: CllLinkKind::Dictionary | CllLinkKind::Rafsi,
            } => {
                words.extend(collect_tagged_words(target));
                words.extend(collect_tagged_words(&inline_plain_text(inlines)));
            }
            CllInline::Emphasis { inlines, .. }
            | CllInline::Quote { inlines, .. }
            | CllInline::LanguageSpan { inlines, .. }
            | CllInline::CiteTitle { inlines }
            | CllInline::Subscript { inlines }
            | CllInline::Superscript { inlines }
            | CllInline::Elidable { inlines, .. } => {
                words.extend(inlines_tagged_words(inlines));
            }
            CllInline::Text(_)
            | CllInline::Code(_)
            | CllInline::Link { .. }
            | CllInline::InlineMath { .. }
            | CllInline::Anchor { .. } => {}
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
            output.push_str(&render_example(site, example, CllRenderFormat::Markdown))
        }
        CllBlock::Table {
            caption,
            header_rows,
            body_rows,
            ..
        } => {
            render_table_markdown(site, caption.as_deref(), header_rows, body_rows, output);
        }
        CllBlock::SimpleListTable { rows, .. } => {
            render_simple_list_table_markdown(site, rows, output);
        }
        CllBlock::VariableList { entries, .. } => {
            for entry in entries {
                output.push_str("**");
                output.push_str(&render_inlines_markdown(site, &entry.term));
                output.push_str("**\n\n");
                for block in &entry.blocks {
                    render_block_markdown(site, block, output, depth);
                }
            }
        }
        CllBlock::Media {
            title, src, alt, ..
        } => {
            output.push_str(&format!("![{}]({})\n\n", alt, src));
            if let Some(title) = title {
                output.push_str(&render_inlines_markdown(site, title));
                output.push_str("\n\n");
            }
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
        CllBlock::DisplayMath { latex, .. } => {
            output.push_str("$$\n");
            output.push_str(latex);
            output.push_str("\n$$\n\n");
        }
        CllBlock::Heading { level, inlines, .. } => {
            output.push_str(&"#".repeat(usize::from(*level)));
            output.push(' ');
            output.push_str(&render_inlines_markdown(site, inlines));
            output.push_str("\n\n");
        }
        CllBlock::BlockQuote { blocks, .. } => {
            let mut inner = String::new();
            for block in blocks {
                render_block_markdown(site, block, &mut inner, depth);
            }
            for line in inner.trim().lines() {
                output.push_str("> ");
                output.push_str(line);
                output.push('\n');
            }
            output.push('\n');
        }
        CllBlock::Definition { body, .. } | CllBlock::GrammarTemplate { body, .. } => {
            output.push_str(&render_inlines_markdown(site, body));
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
        ),
        CllBlock::CmavoList {
            titles,
            headers,
            rows,
            ..
        } => render_cmavo_list_markdown(site, titles, headers, rows, output),
        CllBlock::Lojbanization { lines, .. } => {
            render_lojbanization_markdown(site, lines, output);
        }
        CllBlock::LujvoMaking { parts, .. } => {
            for part in parts {
                output.push_str("- **");
                output.push_str(&part.kind);
                output.push_str("**: ");
                output.push_str(&render_inlines_markdown(site, &part.body));
                output.push('\n');
            }
            output.push('\n');
        }
        CllBlock::Ebnf { entries, .. } => render_ebnf_markdown(site, entries, output),
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
        CllBlock::Example(example) => render_example(site, example, CllRenderFormat::Html),
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
                output.push_str(&render_inlines_html(site, caption));
                output.push_str("</caption>");
            }
            if !header_rows.is_empty() {
                output.push_str("<thead>");
                render_table_rows_html(site, "th", header_rows, &mut output);
                output.push_str("</thead>");
            }
            output.push_str("<tbody>");
            render_table_rows_html(site, "td", body_rows, &mut output);
            output.push_str("</tbody>");
            output.push_str("</table>");
            output
        }
        CllBlock::SimpleListTable {
            id,
            orientation,
            rows,
        } => render_simple_list_table_html(site, id.as_deref(), *orientation, rows),
        CllBlock::VariableList { id, entries } => {
            let mut output = format!(
                "<dl{} class=\"cll-variable-list\">",
                render_optional_id(id.as_deref())
            );
            for entry in entries {
                output.push_str("<dt>");
                output.push_str(&render_inlines_html(site, &entry.term));
                output.push_str("</dt><dd>");
                for block in &entry.blocks {
                    output.push_str(&render_block_html(site, block));
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
                "<figure{} class=\"cll-media\"><img src=\"{}\" alt=\"{}\" />",
                render_optional_id(id.as_deref()),
                escape_html(src),
                escape_html(alt)
            );
            if let Some(title) = title {
                output.push_str("<figcaption>");
                output.push_str(&render_inlines_html(site, title));
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
                render_inlines_html(site, inlines)
            )
        }
        CllBlock::BlockQuote { id, blocks } => {
            let mut output = format!(
                "<blockquote{} class=\"cll-blockquote\">",
                render_optional_id(id.as_deref())
            );
            for block in blocks {
                output.push_str(&render_block_html(site, block));
            }
            output.push_str("</blockquote>");
            output
        }
        CllBlock::Definition { id, body } => format!(
            "<p{} class=\"cll-definition\">{}</p>",
            render_optional_id(id.as_deref()),
            render_inlines_html(site, body)
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
        ),
        CllBlock::CmavoList {
            id,
            titles,
            headers,
            rows,
        } => render_cmavo_list_html(site, id.as_deref(), titles, headers, rows),
        CllBlock::Lojbanization { id, lines } => {
            render_lojbanization_html(site, id.as_deref(), lines)
        }
        CllBlock::LujvoMaking { id, parts } => render_lujvo_making_html(site, id.as_deref(), parts),
        CllBlock::GrammarTemplate { id, body } => format!(
            "<p{} class=\"cll-grammar-template\">{}</p>",
            render_optional_id(id.as_deref()),
            render_inlines_html(site, body)
        ),
        CllBlock::Ebnf { id, entries } => render_ebnf_html(site, id.as_deref(), entries),
    }
}

#[requires(true)]
#[ensures(true)]
fn render_inlines_markdown(site: &CllSite, inlines: &[CllInline]) -> String {
    let mut output = String::new();
    for inline in inlines {
        match inline {
            CllInline::Text(text) => output.push_str(text),
            CllInline::Emphasis { inlines, .. } => {
                output.push('*');
                output.push_str(&render_inlines_markdown(site, inlines));
                output.push('*');
            }
            CllInline::Quote { inlines, .. } => {
                output.push('"');
                output.push_str(&render_inlines_markdown(site, inlines));
                output.push('"');
            }
            CllInline::LanguageSpan { inlines, .. } | CllInline::CiteTitle { inlines } => {
                output.push_str(&render_inlines_markdown(site, inlines));
            }
            CllInline::Subscript { inlines } => {
                output.push('~');
                output.push_str(&render_inlines_markdown(site, inlines));
                output.push('~');
            }
            CllInline::Superscript { inlines } => {
                output.push('^');
                output.push_str(&render_inlines_markdown(site, inlines));
                output.push('^');
            }
            CllInline::Link {
                target,
                inlines,
                kind,
            } => {
                let text = render_inlines_markdown(site, inlines);
                let text = if text.is_empty() {
                    target.as_str()
                } else {
                    &text
                };
                output.push_str(&format!(
                    "[{}]({})",
                    markdown_link_label_text(text),
                    cll_link_href(site, *kind, target)
                ));
            }
            CllInline::Code(text) => output.push_str(&format!("`{text}`")),
            CllInline::Elidable { shown, inlines, .. } => {
                let text = render_inlines_markdown(site, inlines);
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
fn render_inlines_html(site: &CllSite, inlines: &[CllInline]) -> String {
    let mut output = String::new();
    for inline in inlines {
        match inline {
            CllInline::Text(text) => output.push_str(&escape_html(text)),
            CllInline::Emphasis { language, inlines } => {
                output.push_str("<em");
                output.push_str(&render_optional_lang(language.as_deref()));
                output.push('>');
                output.push_str(&render_inlines_html(site, inlines));
                output.push_str("</em>");
            }
            CllInline::Quote { language, inlines } => {
                output.push_str("<q");
                output.push_str(&render_optional_lang(language.as_deref()));
                output.push('>');
                output.push_str(&render_inlines_html(site, inlines));
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
                output.push_str(&render_inlines_html(site, inlines));
                output.push_str("</span>");
            }
            CllInline::CiteTitle { inlines } => {
                output.push_str("<cite>");
                output.push_str(&render_inlines_html(site, inlines));
                output.push_str("</cite>");
            }
            CllInline::Subscript { inlines } => {
                output.push_str("<sub>");
                output.push_str(&render_inlines_html(site, inlines));
                output.push_str("</sub>");
            }
            CllInline::Superscript { inlines } => {
                output.push_str("<sup>");
                output.push_str(&render_inlines_html(site, inlines));
                output.push_str("</sup>");
            }
            CllInline::Link {
                target,
                inlines,
                kind,
            } => {
                output.push_str("<a href=\"");
                output.push_str(&escape_html(&cll_link_href(site, *kind, target)));
                output.push_str("\" class=\"spa-cll-link ");
                output.push_str(link_kind_class(*kind));
                output.push_str("\">");
                output.push_str(&render_inlines_html(site, inlines));
                output.push_str("</a>");
            }
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
                    output.push_str(&render_inlines_html(site, inlines));
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
fn render_table_markdown(
    site: &CllSite,
    caption: Option<&[CllInline]>,
    header_rows: &[Vec<CllTableCell>],
    body_rows: &[Vec<CllTableCell>],
    output: &mut String,
) {
    if let Some(caption) = caption {
        output.push_str("**");
        output.push_str(&render_inlines_markdown(site, caption));
        output.push_str("**\n\n");
    }
    let rows = header_rows
        .iter()
        .chain(body_rows.iter())
        .collect::<Vec<_>>();
    render_markdown_table_rows(
        rows.iter()
            .map(|row| row.iter().map(table_cell_markdown_text).collect::<Vec<_>>()),
        output,
    );
}

#[requires(true)]
#[ensures(true)]
fn render_simple_list_table_markdown(
    site: &CllSite,
    rows: &[Vec<Option<Vec<CllInline>>>],
    output: &mut String,
) {
    render_markdown_table_rows(
        rows.iter().map(|row| {
            row.iter()
                .map(|cell| {
                    cell.as_deref()
                        .map(|inlines| {
                            markdown_table_cell_text(&render_inlines_markdown(site, inlines))
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
fn table_cell_markdown_text(cell: &CllTableCell) -> String {
    let mut text = markdown_table_cell_text(&blocks_plain_text(&cell.blocks));
    if let Some(parse_href) = &cell.parse_href {
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
) {
    if let Some(parse_href) = parse_href {
        output.push_str("[Parse](");
        output.push_str(parse_href);
        output.push_str(")\n\n");
    }
    if !aligned {
        for row in rows {
            let body = row
                .cells
                .iter()
                .map(|cell| render_inlines_markdown(site, cell))
                .filter(|cell| !cell.is_empty())
                .collect::<Vec<_>>()
                .join(" ");
            if !body.is_empty() {
                output.push_str(&row.kind);
                output.push_str(": ");
                output.push_str(&body);
                output.push('\n');
            }
        }
        for line in comments {
            output.push_str("comment: ");
            output.push_str(&render_inlines_markdown(site, line));
            output.push('\n');
        }
        for line in natlang {
            output.push_str("natlang: ");
            output.push_str(&render_inlines_markdown(site, line));
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
                .map(|cell| markdown_table_cell_text(&render_inlines_markdown(site, cell)))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    render_markdown_table_rows(table_rows, output);
    for line in comments {
        output.push_str("_");
        output.push_str(&render_inlines_markdown(site, line));
        output.push_str("_\n\n");
    }
    for line in natlang {
        output.push_str("> ");
        output.push_str(&render_inlines_markdown(site, line));
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
) {
    for title in titles {
        output.push_str("**");
        output.push_str(&render_inlines_markdown(site, title));
        output.push_str("**\n\n");
    }
    let header = if headers.is_empty() {
        Vec::new()
    } else {
        headers
            .iter()
            .map(|cell| markdown_table_cell_text(&render_inlines_markdown(site, cell)))
            .collect::<Vec<_>>()
    };
    let rendered_rows = rows.iter().map(|row| {
        row.iter()
            .map(|cell| markdown_table_cell_text(&render_inlines_markdown(site, cell)))
            .collect::<Vec<_>>()
    });
    if header.is_empty() {
        render_markdown_table_rows(rendered_rows, output);
    } else {
        render_markdown_table_rows(std::iter::once(header).chain(rendered_rows), output);
    }
}

#[requires(true)]
#[ensures(true)]
fn render_lojbanization_markdown(
    site: &CllSite,
    lines: &[CllLojbanizationLine],
    output: &mut String,
) {
    let rows = lines.iter().map(|line| {
        vec![
            line.kind.clone(),
            markdown_table_cell_text(&render_inlines_markdown(site, &line.body)),
            line.comment
                .as_deref()
                .map(|comment| markdown_table_cell_text(&render_inlines_markdown(site, comment)))
                .unwrap_or_default(),
        ]
    });
    render_markdown_table_rows(rows, output);
}

#[requires(true)]
#[ensures(true)]
fn render_ebnf_markdown(site: &CllSite, entries: &[CllEbnfEntry], output: &mut String) {
    for entry in entries {
        output.push_str("**");
        output.push_str(&entry.rule_name);
        output.push_str("** ⩴\n");
        for line in wrap_ebnf_choice_lines(&entry.rhs) {
            output.push_str("  ");
            output.push_str(&render_ebnf_tokens_markdown(site, &line));
            output.push('\n');
        }
        output.push_str("\n\n");
    }
}

#[requires(true)]
#[ensures(true)]
fn render_ebnf_tokens_markdown(site: &CllSite, tokens: &[CllEbnfToken]) -> String {
    let mut output = String::new();
    for token in tokens {
        match token {
            CllEbnfToken::Text { body }
            | CllEbnfToken::Operator { body }
            | CllEbnfToken::Hash { body } => output.push_str(body),
            CllEbnfToken::Terminal { body, href }
            | CllEbnfToken::ElidableTerminator { body, href }
            | CllEbnfToken::Nonterminal { body, href } => {
                if let Some(href) = href {
                    output.push_str(&format!("[{body}]({})", render_ebnf_href(site, href)));
                } else {
                    output.push_str(body);
                }
            }
        }
    }
    output
}

#[requires(true)]
#[ensures(true)]
fn render_ebnf_href(site: &CllSite, href: &str) -> String {
    if let Some(target) = href.strip_prefix("../vlacku/") {
        return cll_link_href(site, CllLinkKind::Dictionary, target);
    }
    href.to_owned()
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
) {
    for row in rows {
        output.push_str("<tr>");
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
            if let Some(parse_href) = &cell.parse_href {
                output.push_str(
                    "<a class=\"cll-parse-example cll-parse-snippet spa-cll-link spa-cll-link-parse\" href=\"",
                );
                output.push_str(&escape_html(parse_href));
                output.push_str("\">Parse</a>");
            }
            for block in &cell.blocks {
                output.push_str(&render_block_html(site, block));
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
fn render_simple_list_table_html(
    site: &CllSite,
    id: Option<&str>,
    orientation: CllSimpleListOrientation,
    rows: &[Vec<Option<Vec<CllInline>>>],
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
                output.push_str(&render_inlines_html(site, inlines));
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
    if let Some(parse_href) = parse_href {
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
                output.push_str(&escape_html(&row.kind));
                output.push_str("\">");
                for cell in &row.cells {
                    output.push_str("<td>");
                    output.push_str(&render_inlines_html(site, cell));
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
                output.push_str(&escape_html(&row.kind));
                output.push_str("\">");
                for cell in &row.cells {
                    output.push_str(&render_inlines_html(site, cell));
                }
                output.push_str("</p></div>");
            }
            output.push_str("</div>");
        }
    }
    for line in comments {
        output.push_str("<p class=\"cll-interlinear-comment\">");
        output.push_str(&render_inlines_html(site, line));
        output.push_str("</p>");
    }
    for line in natlang {
        output.push_str("<p class=\"cll-natlang\">");
        output.push_str(&render_inlines_html(site, line));
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
) -> String {
    let mut output = format!("<div{} class=\"cll-cmavo-list\">", render_optional_id(id));
    for title in titles {
        output.push_str("<p class=\"cll-cmavo-list-title\">");
        output.push_str(&render_inlines_html(site, title));
        output.push_str("</p>");
    }
    output.push_str("<table><tbody>");
    if !headers.is_empty() {
        output.push_str("<tr>");
        for header in headers {
            output.push_str("<th>");
            output.push_str(&render_inlines_html(site, header));
            output.push_str("</th>");
        }
        output.push_str("</tr>");
    }
    for row in rows {
        output.push_str("<tr>");
        for cell in row {
            output.push_str("<td>");
            output.push_str(&render_inlines_html(site, cell));
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
) -> String {
    let mut output = format!(
        "<table{} class=\"cll-lojbanization\"><tbody>",
        render_optional_id(id)
    );
    for line in lines {
        output.push_str("<tr class=\"cll-lojbanization-line cll-lojbanization-line-");
        output.push_str(&escape_html(&line.kind));
        output.push_str("\"><th>");
        output.push_str(&escape_html(&line.kind));
        output.push_str("</th><td>");
        output.push_str(&render_inlines_html(site, &line.body));
        output.push_str("</td><td>");
        if let Some(comment) = &line.comment {
            output.push_str(&render_inlines_html(site, comment));
        }
        output.push_str("</td></tr>");
    }
    output.push_str("</tbody></table>");
    output
}

#[requires(true)]
#[ensures(true)]
fn render_lujvo_making_html(site: &CllSite, id: Option<&str>, parts: &[CllLujvoPart]) -> String {
    let mut output = format!("<ul{} class=\"cll-lujvo-making\">", render_optional_id(id));
    for part in parts {
        output.push_str("<li class=\"cll-lujvo-part cll-lujvo-part-");
        output.push_str(&escape_html(&part.kind));
        output.push_str("\"><span class=\"cll-lujvo-part-kind\">");
        output.push_str(&escape_html(&part.kind));
        output.push_str("</span> ");
        output.push_str(&render_inlines_html(site, &part.body));
        output.push_str("</li>");
    }
    output.push_str("</ul>");
    output
}

#[requires(true)]
#[ensures(true)]
fn render_ebnf_html(site: &CllSite, id: Option<&str>, entries: &[CllEbnfEntry]) -> String {
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
        );
        output.push_str(" <span class=\"cll-ebnf-assign\">⩴</span></div>");
        output.push_str("<pre class=\"cll-ebnf-rhs\">");
        output.push_str(&render_ebnf_tokens_html(site, &entry.rhs));
        output.push_str("</pre></section>");
    }
    output.push_str("</div>");
    output
}

#[requires(true)]
#[ensures(true)]
fn render_ebnf_tokens_html(site: &CllSite, tokens: &[CllEbnfToken]) -> String {
    let lines = wrap_ebnf_choice_lines(tokens);
    if lines.len() == 1 {
        return render_ebnf_token_line_html(site, &lines[0]);
    }
    let mut output = String::new();
    for line in lines {
        output.push_str("<span class=\"cll-ebnf-choice-line\">");
        output.push_str(&render_ebnf_token_line_html(site, &line));
        output.push_str("</span>");
    }
    output
}

#[requires(true)]
#[ensures(true)]
fn render_ebnf_token_line_html(site: &CllSite, tokens: &[CllEbnfToken]) -> String {
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
                render_ebnf_link_html(site, "cll-ebnf-terminal", body, href, &mut output);
            }
            CllEbnfToken::ElidableTerminator { body, href } => {
                render_ebnf_elidable_html(site, body, href, &mut output);
            }
            CllEbnfToken::Nonterminal { body, href } => {
                render_ebnf_link_html(site, "cll-ebnf-nonterminal", body, href, &mut output);
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
    render_ebnf_link_body_html(site, "cll-ebnf-elidable", &body_html, href, output);
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
) {
    render_ebnf_link_body_html(site, class_name, &escape_html(body), href, output);
}

#[requires(!class_name.is_empty())]
#[ensures(true)]
fn render_ebnf_link_body_html(
    site: &CllSite,
    class_name: &str,
    body_html: &str,
    href: &Option<String>,
    output: &mut String,
) {
    if let Some(href) = href {
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
        CllLinkKind::Parse => format!("../gentufa?text={target}"),
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
    node.attribute(name)
        .or_else(|| {
            let local_name = name.rsplit(':').next().unwrap_or(name);
            node.attributes()
                .find(|attribute| attribute.name() == local_name)
                .map(|attribute| attribute.value())
        })
        .map(str::to_owned)
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
            if child.has_tag_name("indexterm")
                || child.has_tag_name("anchor")
                || is_display_none_element(child)
            {
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
        if child
            .ancestors()
            .any(|ancestor| ancestor.is_element() && is_display_none_element(ancestor))
        {
            continue;
        }
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
            CllInline::Text(text) | CllInline::Code(text) | CllInline::InlineMath { text, .. } => {
                output.push_str(text);
                output.push(' ');
            }
            CllInline::Emphasis { inlines, .. }
            | CllInline::Quote { inlines, .. }
            | CllInline::LanguageSpan { inlines, .. }
            | CllInline::CiteTitle { inlines }
            | CllInline::Subscript { inlines }
            | CllInline::Superscript { inlines }
            | CllInline::Link { inlines, .. } => {
                output.push_str(&inline_plain_text(inlines));
                output.push(' ');
            }
            CllInline::Elidable { shown, inlines, .. } => {
                if inlines.is_empty() {
                    output.push_str(shown);
                } else {
                    output.push_str(&inline_plain_text(inlines));
                }
                output.push(' ');
            }
            CllInline::Anchor { .. } => {}
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
            | CllBlock::Heading { title: text, .. }
            | CllBlock::DisplayMath { text, .. } => {
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
            CllBlock::Table {
                caption,
                header_rows,
                body_rows,
                ..
            } => {
                if let Some(caption) = caption {
                    output.push_str(&inline_plain_text(caption));
                    output.push('\n');
                }
                for row in header_rows.iter().chain(body_rows.iter()) {
                    for cell in row {
                        output.push_str(&blocks_plain_text(&cell.blocks));
                        output.push('\n');
                    }
                }
            }
            CllBlock::SimpleListTable { rows, .. } => {
                for row in rows {
                    for cell in row.iter().flatten() {
                        output.push_str(&inline_plain_text(cell));
                        output.push('\n');
                    }
                }
            }
            CllBlock::VariableList { entries, .. } => {
                for entry in entries {
                    output.push_str(&inline_plain_text(&entry.term));
                    output.push('\n');
                    output.push_str(&blocks_plain_text(&entry.blocks));
                    output.push('\n');
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
            CllBlock::BlockQuote { blocks, .. } => {
                output.push_str(&blocks_plain_text(blocks));
                output.push('\n');
            }
            CllBlock::Definition { body, .. } | CllBlock::GrammarTemplate { body, .. } => {
                output.push_str(&inline_plain_text(body));
                output.push('\n');
            }
            CllBlock::InterlinearGloss {
                rows,
                natlang,
                comments,
                ..
            } => {
                for row in rows {
                    for cell in &row.cells {
                        output.push_str(&inline_plain_text(cell));
                        output.push('\n');
                    }
                }
                for line in natlang.iter().chain(comments.iter()) {
                    output.push_str(&inline_plain_text(line));
                    output.push('\n');
                }
            }
            CllBlock::CmavoList {
                titles,
                headers,
                rows,
                ..
            } => {
                for line in titles.iter().chain(headers.iter()) {
                    output.push_str(&inline_plain_text(line));
                    output.push('\n');
                }
                for row in rows {
                    for cell in row {
                        output.push_str(&inline_plain_text(cell));
                        output.push('\n');
                    }
                }
            }
            CllBlock::Lojbanization { lines, .. } => {
                for line in lines {
                    output.push_str(&inline_plain_text(&line.body));
                    output.push('\n');
                    if let Some(comment) = &line.comment {
                        output.push_str(&inline_plain_text(comment));
                        output.push('\n');
                    }
                }
            }
            CllBlock::LujvoMaking { parts, .. } => {
                for part in parts {
                    output.push_str(&inline_plain_text(&part.body));
                    output.push('\n');
                }
            }
            CllBlock::Ebnf { entries, .. } => {
                for entry in entries {
                    output.push_str(&entry.rule_name);
                    output.push('\n');
                    for token in &entry.rhs {
                        output.push_str(&ebnf_token_plain_text(token));
                    }
                    output.push('\n');
                }
            }
        }
    }
    normalized_plain_text(&output)
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
        let parse_hrefs = collect_interlinear_parse_hrefs(&section.blocks);

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
    fn north_wind_section_omits_hidden_vocabulary_dump() {
        let site = embedded_cll_site().expect("embedded CLL should load");
        let section = cll_lookup_section(site, "section-north-wind").expect("section should exist");
        assert!(!section.plain_text.contains(".alf."));
        assert!(!blocks_plain_text(&section.blocks).contains(".alf."));
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
        let parse_hrefs = collect_table_parse_hrefs(&section.blocks);
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

    #[requires(true)]
    #[ensures(true)]
    fn collect_table_parse_hrefs(blocks: &[CllBlock]) -> Vec<String> {
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
                            hrefs.extend(collect_table_parse_hrefs(&cell.blocks));
                        }
                    }
                }
                CllBlock::List { items, .. } => {
                    for item in items {
                        hrefs.extend(collect_table_parse_hrefs(item));
                    }
                }
                CllBlock::Example(example) => {
                    hrefs.extend(collect_table_parse_hrefs(&example.blocks));
                }
                CllBlock::BlockQuote { blocks, .. } | CllBlock::Rule { body: blocks, .. } => {
                    hrefs.extend(collect_table_parse_hrefs(blocks));
                }
                CllBlock::VariableList { entries, .. } => {
                    for entry in entries {
                        hrefs.extend(collect_table_parse_hrefs(&entry.blocks));
                    }
                }
                _ => {}
            }
        }
        hrefs
    }

    #[requires(true)]
    #[ensures(true)]
    fn collect_interlinear_parse_hrefs(blocks: &[CllBlock]) -> Vec<String> {
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
                        hrefs.extend(collect_interlinear_parse_hrefs(item));
                    }
                }
                CllBlock::Example(example) => {
                    hrefs.extend(collect_interlinear_parse_hrefs(&example.blocks));
                }
                CllBlock::BlockQuote { blocks, .. } | CllBlock::Rule { body: blocks, .. } => {
                    hrefs.extend(collect_interlinear_parse_hrefs(blocks));
                }
                CllBlock::Table {
                    header_rows,
                    body_rows,
                    ..
                } => {
                    for row in header_rows.iter().chain(body_rows.iter()) {
                        for cell in row {
                            hrefs.extend(collect_interlinear_parse_hrefs(&cell.blocks));
                        }
                    }
                }
                CllBlock::VariableList { entries, .. } => {
                    for entry in entries {
                        hrefs.extend(collect_interlinear_parse_hrefs(&entry.blocks));
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
