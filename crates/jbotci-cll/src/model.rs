use std::collections::BTreeMap;
use std::fmt;
use std::num::{NonZeroU16, NonZeroUsize};
use std::str::FromStr;

#[allow(unused_imports)]
use bityzba::{data, ensures, expensive_invariant, invariant, new, requires};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// An earlier edition in the vendored book's lineage, named as that edition
/// named itself.
#[invariant(!title.is_empty(), "an ancestor edition is identified by its own title")]
#[invariant(!version.is_empty(), "an ancestor edition is identified by its own version")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct CllEditionAncestor {
    pub title: String,
    pub version: String,
}

/// The identity of the reference book jbotci answers from.
///
/// Every field is derived at build time from the vendored sources — the book's
/// own `.env` for `title`, `version`, and `publisher`, `vendor/cll.VENDORED_FROM`
/// for the pin, and `vendor/cll-import-metadata.toml` for `ancestry` — so the
/// reported edition cannot drift from the vendored text. The book is an
/// unofficial community publication; `publisher` is its own statement of that
/// and is reproduced rather than paraphrased.
#[invariant(!title.is_empty(), "the vendored edition states its title")]
#[invariant(!version.is_empty(), "the vendored edition states its version")]
#[invariant(!publisher.is_empty(), "the vendored edition states its publisher line")]
#[invariant(!ancestry.is_empty(), "the vendored edition descends from at least one earlier edition")]
#[invariant(!upstream_url.is_empty(), "the vendored edition records where it was vendored from")]
#[invariant(!release_tag.is_empty(), "the vendored edition records the release it was pinned to")]
#[invariant(!commit.is_empty(), "the vendored edition records the commit it was pinned to")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct CllEdition {
    pub title: String,
    pub version: String,
    pub publisher: String,
    /// Oldest first; the vendored edition itself is not repeated here.
    pub ancestry: Vec<CllEditionAncestor>,
    pub upstream_url: String,
    pub release_tag: String,
    pub commit: String,
}

impl CllEdition {
    /// The book's title followed by its edition version in parentheses, naming
    /// the exact edition an answer came from.
    #[requires(true)]
    #[ensures(ret.starts_with(&self.title) && ret.contains(&self.version))]
    pub fn display_title(&self) -> String {
        format!("{} ({})", self.title, self.version)
    }

    /// Each ancestor edition as `<title> <version>`, oldest first, joined by a
    /// rightwards arrow and ending at this edition's own version.
    #[requires(true)]
    #[ensures(ret.ends_with(&self.version))]
    pub fn lineage(&self) -> String {
        let mut lineage = String::new();
        for ancestor in &self.ancestry {
            lineage.push_str(&ancestor.title);
            lineage.push(' ');
            lineage.push_str(&ancestor.version);
            lineage.push_str(" \u{2192} ");
        }
        lineage.push_str(&self.version);
        lineage
    }
}

/// The site's own description of itself. `chapter_count` is unconstrained here
/// because `CllSite` already ties it to the loaded chapters, and an empty site
/// is a legitimate value; the edition's own validity is `CllEdition`'s.
#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CllMetadata {
    pub edition: CllEdition,
    pub chapter_count: usize,
}

/// How the book designates one of its top-level divisions.
///
/// Numbered chapters are designated by a chapter number. Appendices are
/// designated by their title alone: the book neither numbers nor letters them,
/// and its own table of contents runs 1-N and then lists the appendices by
/// title. The appendix variant therefore has no number to carry, which makes an
/// appendix with a chapter number unrepresentable rather than merely
/// discouraged, and `NonZeroU16` keeps a numbered chapter from claiming
/// chapter 0.
#[invariant(true)]
#[invariant(::Chapter => true)]
#[invariant(::Appendix => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CllDivision {
    Chapter { number: NonZeroU16 },
    Appendix,
}

impl CllDivision {
    /// The chapter number of a numbered chapter; `None` for an appendix.
    #[requires(true)]
    #[ensures(ret.is_some() == matches!(self, Self::Chapter { .. }))]
    pub const fn chapter_number(self) -> Option<NonZeroU16> {
        match self {
            Self::Chapter { number } => Some(number),
            Self::Appendix => None,
        }
    }

    /// The number the book gives to this division's `section_index`-th section
    /// (`6.3`), or `None` when the division is an appendix and its sections are
    /// cited by title and stable id instead.
    #[requires(true)]
    #[ensures(ret.is_some() == matches!(self, Self::Chapter { .. }))]
    #[ensures(ret.map(CllSectionNumber::chapter) == self.chapter_number())]
    pub fn section_number(self, section_index: NonZeroUsize) -> Option<CllSectionNumber> {
        self.chapter_number()
            .map(|chapter| CllSectionNumber::Section {
                chapter,
                index: section_index,
            })
    }

    /// The number the book prints for a division that has no `<section>`
    /// children of its own - the bare chapter number - or `None` for an
    /// appendix.
    #[requires(true)]
    #[ensures(ret.is_some() == matches!(self, Self::Chapter { .. }))]
    #[ensures(ret.map(CllSectionNumber::chapter) == self.chapter_number())]
    pub fn whole_chapter_number(self) -> Option<CllSectionNumber> {
        self.chapter_number()
            .map(|chapter| CllSectionNumber::WholeChapter { chapter })
    }

    /// How a cross-reference to the division as a whole reads. Numbered
    /// chapters cite as `Chapter N`; appendices cite by their own title, which
    /// is the only designation the book gives them.
    #[requires(!title.is_empty())]
    #[ensures(!ret.is_empty())]
    pub fn xref_label(self, title: &str) -> String {
        match self.chapter_number() {
            Some(number) => format!("Chapter {number}"),
            None => title.to_owned(),
        }
    }
}

/// The number the book prints for a section of a numbered chapter.
///
/// The chapter is part of the number rather than a field beside it, so a
/// section number that disagrees with the chapter it belongs to cannot be
/// written down - not through a constructor, and not through serde, which
/// reads and writes these as the strings the book prints (`6.3`, `20`).
#[invariant(true)]
#[invariant(::WholeChapter => true)]
#[invariant(::Section => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CllSectionNumber {
    /// A numbered chapter with no `<section>` children of its own, printed as
    /// the bare chapter number.
    WholeChapter { chapter: NonZeroU16 },
    /// The `index`-th section of a numbered chapter, printed `chapter.index`.
    Section {
        chapter: NonZeroU16,
        index: NonZeroUsize,
    },
}

impl CllSectionNumber {
    /// The chapter this number names. Reading it off the number is what makes
    /// division/number agreement checkable rather than assumed.
    #[requires(true)]
    #[ensures(true)]
    pub const fn chapter(self) -> NonZeroU16 {
        match self {
            Self::WholeChapter { chapter } | Self::Section { chapter, .. } => chapter,
        }
    }
}

impl fmt::Display for CllSectionNumber {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WholeChapter { chapter } => write!(formatter, "{chapter}"),
            Self::Section { chapter, index } => write!(formatter, "{chapter}.{index}"),
        }
    }
}

impl FromStr for CllSectionNumber {
    type Err = CllSectionNumberError;

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|number| number.to_string() == text) || ret.is_err())]
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let invalid = || CllSectionNumberError::Invalid {
            text: text.to_owned(),
        };
        let number = match text.split_once('.') {
            Some((chapter, index)) => Self::Section {
                chapter: chapter.parse().map_err(|_| invalid())?,
                index: index.parse().map_err(|_| invalid())?,
            },
            None => Self::WholeChapter {
                chapter: text.parse().map_err(|_| invalid())?,
            },
        };
        // Rust's integer parsers accept a leading `+` and leading zeroes, so
        // `+6.3`, `06.3` and `6.03` would all parse to the number that prints
        // as `6.3`. A section number is the exact string the book prints, and
        // callers rely on it printing back byte for byte, so a spelling that
        // does not round-trip is not that number.
        if number.to_string() != text {
            return Err(invalid());
        }
        Ok(number)
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[invariant(true)]
#[invariant(::Invalid => true)]
pub enum CllSectionNumberError {
    #[error(
        "`{text}` is not a CLL section number; expected a chapter number such as `20` or a chapter and section such as `6.3`, both counted from one"
    )]
    Invalid { text: String },
}

impl Serialize for CllSectionNumber {
    #[requires(true)]
    #[ensures(true)]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for CllSectionNumber {
    #[requires(true)]
    #[ensures(true)]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let text = String::deserialize(deserializer)?;
        text.parse().map_err(serde::de::Error::custom)
    }
}

/// The heading a division or section shows: `"6.3. Some title"` where the book
/// gives a number, and the bare title where it designates by title alone.
#[requires(!title.is_empty())]
#[ensures(ret.ends_with(title))]
pub fn cll_numbered_title(number: Option<impl fmt::Display>, title: &str) -> String {
    match number {
        Some(number) => format!("{number}. {title}"),
        None => title.to_owned(),
    }
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
    "example indexes must be keyed by anchor id and agree with the section they reference"
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
#[invariant(!chapter_title.is_empty())]
#[expensive_invariant(root_section_ids.iter().all(|section_id| !section_id.is_empty()))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CllChapter {
    pub chapter_id: String,
    pub division: CllDivision,
    pub chapter_title: String,
    pub root_section_ids: Vec<String>,
    pub prelude_blocks: Vec<CllBlock>,
}

#[invariant(!section_id.is_empty())]
#[invariant(!chapter_id.is_empty())]
#[invariant(
    number.map(CllSectionNumber::chapter) == division.chapter_number(),
    "a section carries the number the book prints for it, which names the section's own chapter; an appendix section carries no number at all"
)]
#[invariant(!title.is_empty())]
#[invariant(parent_section_id.as_ref().is_none_or(|section_id| !section_id.is_empty()))]
#[invariant(!source_path.is_empty())]
#[expensive_invariant(child_section_ids.iter().all(|section_id| !section_id.is_empty()))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CllSection {
    pub section_id: String,
    pub chapter_id: String,
    pub division: CllDivision,
    pub number: Option<CllSectionNumber>,
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

#[invariant(
    section_number.map(CllSectionNumber::chapter) == division.chapter_number(),
    "a reference into a numbered chapter carries that chapter's own section number; appendix references are keyed by section id alone"
)]
#[invariant(!section_id.is_empty())]
#[invariant(example_number.is_some() == example_id.is_some())]
#[invariant(example_number.as_ref().is_none_or(|number| !number.is_empty()))]
#[invariant(example_id.as_ref().is_none_or(|id| !id.is_empty()))]
#[invariant(!source_path.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CllReference {
    pub division: CllDivision,
    pub section_number: Option<CllSectionNumber>,
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
                    chapter.chapter_id == section.chapter_id && chapter.division == section.division
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
            && sections_by_id
                .get(&example.reference.section_id)
                .is_some_and(|section| {
                    section.division == example.reference.division
                        && section.number == example.reference.section_number
                })
    }) && example_ids_by_normalized_reference
        .iter()
        .all(|(reference, example_id)| {
            !reference.is_empty() && examples_by_id.contains_key(example_id)
        })
}

/// The DocBook `role` a paragraph carries in the vendored sources.
///
/// The vendored edition labels every rule it teaches differently from the first
/// edition with a `status-note` paragraph — the edition's headline feature — so
/// that designation is modelled explicitly and renderers set those notes off
/// from ordinary prose instead of string-matching a raw attribute. Every other
/// role is presentational styling that the renderers pass through to CSS
/// unchanged, which is also what keeps a newer vendored edition from needing
/// code changes when it introduces a presentational role we have never seen.
///
/// The upstream design (`vendor/cll/docs/status-markup.md`) also reserves a
/// machine-readable `condition="status:..."` attribute that would name the
/// authority level of each note — ratified, committee-approved, checkpointed,
/// de facto, editorial. No release has emitted one yet: both `v1.3.2` and
/// `v1.3.3` carry only the transitional bare form, so the level exists solely
/// in each note's prose and this type deliberately does not guess at it.
#[invariant(true)]
#[invariant(
    ::Presentation { name } => name.trim() == name.as_str()
        && !name.is_empty()
        && !name.eq_ignore_ascii_case("status-note"),
    "a presentational role is canonical (already trimmed), non-empty, and never shadows the typed status-note designation, including through serde"
)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CllParagraphRole {
    StatusNote,
    Presentation { name: String },
}

impl CllParagraphRole {
    /// Interprets a DocBook `role` attribute value. An attribute that is absent
    /// or blank designates nothing and yields `None`.
    #[requires(true)]
    #[ensures(ret.is_none() == value.trim().is_empty())]
    pub fn parse(value: &str) -> Option<Self> {
        let value = value.trim();
        if value.is_empty() {
            return None;
        }
        if value.eq_ignore_ascii_case("status-note") {
            return Some(new!(CllParagraphRole::StatusNote));
        }
        Some(new!(CllParagraphRole::Presentation {
            name: value.to_owned()
        }))
    }

    /// Whether this paragraph is one of the edition's rule-status notes.
    #[requires(true)]
    #[ensures(true)]
    pub fn is_status_note(&self) -> bool {
        matches!(self.as_data(), data!(CllParagraphRole::StatusNote))
    }

    /// The presentational CSS suffix for a role that only carries styling.
    #[requires(true)]
    #[ensures(ret.is_none_or(|name| !name.is_empty()))]
    pub fn presentation_name(&self) -> Option<&str> {
        match self.as_data() {
            data!(CllParagraphRole::Presentation { name }) => Some(name),
            data!(CllParagraphRole::StatusNote) => None,
        }
    }
}

/// The label renderers put on a rule-status note so a reader can tell one from
/// ordinary prose in transports that carry no styling.
pub const CLL_STATUS_NOTE_LABEL: &str = "Rule status";

/// What separates that label from the note's body in element-tree transports.
/// It is a real text node rather than only a CSS margin, so the DOM text a
/// reader copies out of the web app reads the same as the HTML the CLI and the
/// MCP tool emit.
pub const CLL_STATUS_NOTE_LABEL_SEPARATOR: &str = " ";

/// The classes a rule-status note carries when a section is read. Shared by the
/// HTML renderer and the web reader so the two presentations cannot drift.
pub const CLL_STATUS_NOTE_BLOCK_CLASSES: &str = "cll-para cll-status-note";

/// The classes a rule-status note carries when it is a search hit's preview.
pub const CLL_STATUS_NOTE_PREVIEW_CLASSES: &str = "cll-search-preview cll-status-note";

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

#[invariant(!section_id.is_empty())]
#[invariant(!section_title.is_empty())]
#[invariant(!source_path.is_empty())]
#[invariant(!text.trim().is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CllChrestomathySectionText {
    pub section_id: String,
    pub section_title: String,
    pub source_path: String,
    pub text: String,
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

use crate::ebnf::CllEbnfEntry;

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
        role: Option<CllParagraphRole>,
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
    /// A CLL section reference. Plain rendering keeps its human-readable
    /// section label and omits the route.
    Section,
    /// A worked-example reference. Plain rendering keeps its example label and
    /// omits the route.
    Example,
    /// A dictionary word reference. Plain rendering keeps the linked word and
    /// omits the dictionary route.
    Dictionary,
    /// A rafsi-search reference. Plain rendering keeps the displayed rafsi and
    /// omits the search route.
    Rafsi,
    /// A navigation-only parser action. Plain rendering drops the action
    /// because "Parse" has no useful content without its route.
    Parse,
    /// A bundled-asset reference. Plain rendering keeps the descriptive label
    /// and omits the asset path.
    Asset,
    /// An external reference. Plain rendering keeps the descriptive label and
    /// omits the external URI.
    External,
}

/// Whether rendered CLL links target the interactive web application or a
/// route-free transport such as the CLI and MCP tool.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CllLinkRenderMode {
    /// Preserve content-bearing labels but omit destinations and
    /// navigation-only controls.
    Plain,
    /// Preserve the interactive SPA links exactly.
    Web,
}

use crate::search::CllSearchChunk;

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
