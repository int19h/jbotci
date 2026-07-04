use std::collections::{BTreeMap, BTreeSet};

#[allow(unused_imports)]
use bityzba::{ensures, expensive_invariant, invariant, requires};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[invariant(!title.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CllMetadata {
    pub title: String,
    pub chapter_count: usize,
}

#[invariant(metadata.chapter_count == chapters.len(), "metadata chapter count mirrors loaded chapters")]
#[expensive_invariant(
    cll_site_section_references_are_consistent(
        chapters,
        sections_by_id,
        section_order,
        section_ids_by_normalized_reference,
        anchors_by_id,
        index_entries,
    ),
    "section navigation, anchors, and indexes must reference existing section ids"
)]
#[expensive_invariant(
    cll_site_example_references_are_consistent(
        sections_by_id,
        examples_by_id,
        example_ids_by_normalized_reference,
    ),
    "example indexes must be keyed by anchor id and reference existing sections"
)]
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

#[invariant(!chapter_id.is_empty())]
#[invariant(*chapter_number > 0)]
#[invariant(!chapter_title.is_empty())]
#[expensive_invariant(root_section_ids.iter().all(|section_id| !section_id.is_empty()))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CllChapter {
    pub chapter_id: String,
    pub chapter_number: u16,
    pub chapter_title: String,
    pub root_section_ids: Vec<String>,
    pub prelude_blocks: Vec<CllBlock>,
}

#[invariant(!section_id.is_empty())]
#[invariant(!chapter_id.is_empty())]
#[invariant(*chapter_number > 0)]
#[invariant(!number.is_empty())]
#[invariant(!title.is_empty())]
#[invariant(parent_section_id.as_ref().is_none_or(|section_id| !section_id.is_empty()))]
#[invariant(!source_path.is_empty())]
#[expensive_invariant(child_section_ids.iter().all(|section_id| !section_id.is_empty()))]
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

#[invariant(!section_id.is_empty())]
#[invariant(!label.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CllAnchor {
    pub section_id: String,
    pub label: String,
}

#[invariant(!key.is_empty())]
#[invariant(!section_ids.is_empty())]
#[expensive_invariant(section_ids.iter().all(|section_id| !section_id.is_empty()))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CllIndexEntry {
    pub key: String,
    pub section_ids: Vec<String>,
}

#[invariant(*chapter > 0)]
#[invariant(!section_number.is_empty())]
#[invariant(!section_id.is_empty())]
#[invariant(example_number.is_some() == example_id.is_some())]
#[invariant(example_number.as_ref().is_none_or(|number| !number.is_empty()))]
#[invariant(example_id.as_ref().is_none_or(|id| !id.is_empty()))]
#[invariant(!source_path.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CllReference {
    pub chapter: u16,
    pub section_number: String,
    pub section_id: String,
    pub example_number: Option<String>,
    pub example_id: Option<String>,
    pub source_path: String,
}

#[invariant(reference.example_id.as_ref().is_some_and(|example_id| example_id == anchor_id))]
#[invariant(!label.is_empty())]
#[invariant(!anchor_id.is_empty())]
#[invariant(title.as_ref().is_none_or(|title| !title.is_empty()))]
#[invariant(parse_href.as_ref().is_none_or(|href| !href.is_empty()))]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CllExampleLineKind {
    #[serde(rename = "jbo")]
    Jbo,
    #[serde(rename = "jbophrase")]
    Jbophrase,
    #[serde(rename = "gloss")]
    Gloss,
    #[serde(rename = "natlang")]
    Natlang,
    #[serde(rename = "text")]
    Text,
}

impl CllExampleLineKind {
    #[requires(true)]
    #[ensures(ret.as_ref().is_some_and(|kind| !kind.as_str().is_empty()) || ret.is_none())]
    pub fn parse_tag(tag_name: &str) -> Option<Self> {
        match tag_name.as_bytes() {
            b"jbo" => Some(Self::Jbo),
            b"jbophrase" => Some(Self::Jbophrase),
            b"gloss" => Some(Self::Gloss),
            b"natlang" => Some(Self::Natlang),
            b"text" => Some(Self::Text),
            _ => None,
        }
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Jbo => "jbo",
            Self::Jbophrase => "jbophrase",
            Self::Gloss => "gloss",
            Self::Natlang => "natlang",
            Self::Text => "text",
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub const fn is_lojban(self) -> bool {
        matches!(self, Self::Jbo | Self::Jbophrase)
    }
}

#[invariant(!text.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CllExampleLine {
    pub kind: CllExampleLineKind,
    pub text: String,
}

#[requires(true)]
#[ensures(true)]
fn cll_site_section_references_are_consistent(
    chapters: &[CllChapter],
    sections_by_id: &BTreeMap<String, CllSection>,
    section_order: &[String],
    section_ids_by_normalized_reference: &BTreeMap<String, String>,
    anchors_by_id: &BTreeMap<String, CllAnchor>,
    index_entries: &[CllIndexEntry],
) -> bool {
    section_order.len() == sections_by_id.len()
        && section_order.iter().enumerate().all(|(index, section_id)| {
            !section_id.is_empty()
                && sections_by_id.contains_key(section_id)
                && !section_order[..index].contains(section_id)
        })
        && sections_by_id.iter().all(|(section_id, section)| {
            section_id == &section.section_id
                && chapters.iter().any(|chapter| {
                    chapter.chapter_id == section.chapter_id
                        && chapter.chapter_number == section.chapter_number
                })
                && section
                    .parent_section_id
                    .as_ref()
                    .is_none_or(|parent_id| sections_by_id.contains_key(parent_id))
                && section
                    .child_section_ids
                    .iter()
                    .all(|child_id| sections_by_id.contains_key(child_id))
        })
        && chapters.iter().all(|chapter| {
            chapter
                .root_section_ids
                .iter()
                .all(|section_id| sections_by_id.contains_key(section_id))
        })
        && section_ids_by_normalized_reference
            .iter()
            .all(|(reference, section_id)| {
                !reference.is_empty() && sections_by_id.contains_key(section_id)
            })
        && anchors_by_id.iter().all(|(anchor_id, anchor)| {
            !anchor_id.is_empty()
                && cll_anchor_resolves_to_section(chapters, sections_by_id, anchor)
        })
        && index_entries.iter().all(|entry| {
            entry
                .section_ids
                .iter()
                .all(|section_id| sections_by_id.contains_key(section_id))
        })
}

#[requires(true)]
#[ensures(true)]
fn cll_anchor_resolves_to_section(
    chapters: &[CllChapter],
    sections_by_id: &BTreeMap<String, CllSection>,
    anchor: &CllAnchor,
) -> bool {
    sections_by_id.contains_key(&anchor.section_id)
        || chapters.iter().any(|chapter| {
            chapter.chapter_id == anchor.section_id
                && chapter
                    .root_section_ids
                    .first()
                    .is_some_and(|section_id| sections_by_id.contains_key(section_id))
        })
}

#[requires(true)]
#[ensures(true)]
fn cll_site_example_references_are_consistent(
    sections_by_id: &BTreeMap<String, CllSection>,
    examples_by_id: &BTreeMap<String, CllExample>,
    example_ids_by_normalized_reference: &BTreeMap<String, String>,
) -> bool {
    examples_by_id.iter().all(|(example_id, example)| {
        example_id == &example.anchor_id
            && example
                .reference
                .example_id
                .as_ref()
                .is_some_and(|reference_id| reference_id == example_id)
            && sections_by_id.contains_key(&example.reference.section_id)
    }) && example_ids_by_normalized_reference
        .iter()
        .all(|(reference, example_id)| {
            !reference.is_empty() && examples_by_id.contains_key(example_id)
        })
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
    pub parse_group: Option<CllTableParseGroup>,
}

#[invariant(!group_id.is_empty())]
#[invariant(*row_count > 0)]
#[invariant(*row_index < *row_count)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CllTableParseGroup {
    pub group_id: String,
    pub row_count: usize,
    pub row_index: usize,
}

#[invariant(!section.is_empty())]
#[invariant(!area.is_empty())]
#[invariant(!group_id.is_empty())]
#[invariant(!text.trim().is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CllChrestomathyGroupText {
    pub(crate) section: String,
    pub(crate) area: String,
    pub(crate) group_id: String,
    pub(crate) text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
pub(crate) enum CllTableRowArea {
    Header,
    Body,
}

#[invariant(parse_href.as_ref().is_none_or(|href| !href.is_empty()))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CllChrestomathyParseInfo {
    pub(crate) parse_href: Option<String>,
    pub(crate) parse_group: Option<CllTableParseGroup>,
}

#[invariant(!section.is_empty())]
#[invariant(section.iter().all(|item| !item.id.is_empty()))]
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct CllChrestomathyMetadata {
    pub(crate) section: Vec<CllChrestomathySectionMetadata>,
}

#[invariant(!chrestomathy_chapter_id.is_empty())]
#[invariant(!ebnf_section_id.is_empty())]
#[invariant(!ebnf_symbols.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct CllImportMetadata {
    pub(crate) chrestomathy_chapter_id: String,
    pub(crate) ebnf_section_id: String,
    pub(crate) ebnf_symbols: BTreeMap<String, String>,
}

#[invariant(!id.is_empty())]
#[invariant(header_groups.iter().all(|group| !group.is_empty() && group.iter().all(|row| *row > 0)))]
#[invariant(body_groups.iter().all(|group| !group.is_empty() && group.iter().all(|row| *row > 0)))]
#[invariant(header_no_parse.iter().all(|row| *row > 0))]
#[invariant(body_no_parse.iter().all(|row| *row > 0))]
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct CllChrestomathySectionMetadata {
    pub(crate) id: String,
    #[serde(default)]
    pub(crate) header_groups: Vec<Vec<usize>>,
    #[serde(default)]
    pub(crate) body_groups: Vec<Vec<usize>>,
    #[serde(default)]
    pub(crate) header_no_parse: Vec<usize>,
    #[serde(default)]
    pub(crate) body_no_parse: Vec<usize>,
}

#[invariant(!term.is_empty() || !blocks.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CllVariableEntry {
    pub term: Vec<CllInline>,
    pub blocks: Vec<CllBlock>,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CllInterlinearRowKind {
    #[serde(rename = "jbo")]
    Jbo,
    #[serde(rename = "jbophrase")]
    Jbophrase,
    #[serde(rename = "gloss")]
    Gloss,
    #[serde(rename = "dbmath")]
    Dbmath,
    #[serde(rename = "mmlmath")]
    Mmlmath,
    #[serde(rename = "math")]
    Math,
    #[serde(rename = "para")]
    Para,
}

impl CllInterlinearRowKind {
    #[requires(true)]
    #[ensures(ret.as_ref().is_some_and(|kind| !kind.as_str().is_empty()) || ret.is_none())]
    pub fn parse_tag(tag_name: &str) -> Option<Self> {
        match tag_name.as_bytes() {
            b"jbo" => Some(Self::Jbo),
            b"jbophrase" => Some(Self::Jbophrase),
            b"gloss" => Some(Self::Gloss),
            b"dbmath" => Some(Self::Dbmath),
            b"mmlmath" => Some(Self::Mmlmath),
            b"math" => Some(Self::Math),
            b"para" => Some(Self::Para),
            _ => None,
        }
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Jbo => "jbo",
            Self::Jbophrase => "jbophrase",
            Self::Gloss => "gloss",
            Self::Dbmath => "dbmath",
            Self::Mmlmath => "mmlmath",
            Self::Math => "math",
            Self::Para => "para",
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub const fn is_lojban(self) -> bool {
        matches!(self, Self::Jbo | Self::Jbophrase)
    }

    #[requires(true)]
    #[ensures(true)]
    pub const fn is_math(self) -> bool {
        matches!(self, Self::Dbmath | Self::Mmlmath | Self::Math)
    }
}

#[invariant(!cells.is_empty())]
#[invariant(cells.iter().all(|cell| !cell.is_empty()))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CllInterlinearRow {
    pub kind: CllInterlinearRowKind,
    pub cells: Vec<Vec<CllInline>>,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CllLojbanizationLineKind {
    #[serde(rename = "jbo")]
    Jbo,
    #[serde(rename = "natlang")]
    Natlang,
}

impl CllLojbanizationLineKind {
    #[requires(true)]
    #[ensures(ret.as_ref().is_some_and(|kind| !kind.as_str().is_empty()) || ret.is_none())]
    pub fn parse_tag(tag_name: &str) -> Option<Self> {
        match tag_name.as_bytes() {
            b"jbo" => Some(Self::Jbo),
            b"natlang" => Some(Self::Natlang),
            _ => None,
        }
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Jbo => "jbo",
            Self::Natlang => "natlang",
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub const fn is_lojban(self) -> bool {
        matches!(self, Self::Jbo)
    }
}

#[invariant(!body.is_empty() || comment.as_ref().is_some_and(|line| !line.is_empty()))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CllLojbanizationLine {
    pub kind: CllLojbanizationLineKind,
    pub body: Vec<CllInline>,
    pub comment: Option<Vec<CllInline>>,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CllLujvoPartKind {
    #[serde(rename = "jbo")]
    Jbo,
    #[serde(rename = "veljvo")]
    Veljvo,
    #[serde(rename = "rafsi")]
    Rafsi,
    #[serde(rename = "gloss")]
    Gloss,
    #[serde(rename = "natlang")]
    Natlang,
    #[serde(rename = "score")]
    Score,
}

impl CllLujvoPartKind {
    #[requires(true)]
    #[ensures(ret.as_ref().is_some_and(|kind| !kind.as_str().is_empty()) || ret.is_none())]
    pub fn parse_tag(tag_name: &str) -> Option<Self> {
        match tag_name.as_bytes() {
            b"jbo" => Some(Self::Jbo),
            b"veljvo" => Some(Self::Veljvo),
            b"rafsi" => Some(Self::Rafsi),
            b"gloss" => Some(Self::Gloss),
            b"natlang" => Some(Self::Natlang),
            b"score" => Some(Self::Score),
            _ => None,
        }
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Jbo => "jbo",
            Self::Veljvo => "veljvo",
            Self::Rafsi => "rafsi",
            Self::Gloss => "gloss",
            Self::Natlang => "natlang",
            Self::Score => "score",
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub const fn is_lojban(self) -> bool {
        matches!(self, Self::Jbo | Self::Veljvo | Self::Rafsi)
    }
}

#[invariant(!body.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CllLujvoPart {
    pub kind: CllLujvoPartKind,
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
#[invariant(::Example { .. } => true)]
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
    Example {
        example_id: String,
    },
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
