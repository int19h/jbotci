use std::collections::BTreeSet;

#[allow(unused_imports)]
use bityzba::{contract_trait, ensures, invariant, new, requires};
use jbotci_morphology::normalize_lojban_input_text;
use serde::{Deserialize, Serialize};

use crate::visitor::{CllBlockVisitor, walk_block, walk_inline};

use super::*;

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CllSearchChunkKind {
    Section,
    Paragraph,
    Example,
}

/// One indexed unit of the book. `role` carries the designation of the
/// paragraph a `Paragraph` chunk was projected from, so search results — the
/// default way readers reach the book — can set a rule-status note off exactly
/// as reading its section does. It is deliberately not folded into `label` or
/// `text`: those two are the embedding input, and changing them would change
/// the published packs' fingerprint.
#[invariant(
    role.is_none() || *kind == CllSearchChunkKind::Paragraph,
    "only a paragraph chunk projects a paragraph's designation"
)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CllSearchChunk {
    pub kind: CllSearchChunkKind,
    pub role: Option<CllParagraphRole>,
    pub section_id: String,
    pub anchor_id: String,
    pub section_number: Option<String>,
    pub section_title: String,
    pub label: String,
    pub text: String,
    pub tagged_words: BTreeSet<String>,
}

impl CllSearchChunk {
    /// Whether this hit is one of the edition's rule-status notes.
    #[requires(true)]
    #[ensures(ret -> self.kind == CllSearchChunkKind::Paragraph)]
    pub fn is_status_note(&self) -> bool {
        self.role
            .as_ref()
            .is_some_and(CllParagraphRole::is_status_note)
    }
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

#[requires(true)]
#[ensures(true)]
pub(super) fn build_search_chunks(site: &CllSite) -> Vec<CllSearchChunk> {
    let mut chunks = Vec::new();
    for section_id in &site.section_order {
        let section = site
            .sections_by_id
            .get(section_id)
            .expect("CllSite invariant guarantees section_order ids resolve");
        let section_label = format_section_display_title(section);
        let section_text =
            normalized_plain_text(&format!("{}\n{}", section.title, section.plain_text));
        if !section_text.is_empty() {
            chunks.push(new!(CllSearchChunk {
                kind: CllSearchChunkKind::Section,
                role: None,
                section_id: section.section_id.clone(),
                anchor_id: section.section_id.clone(),
                section_number: section.number.map(|number| number.to_string()),
                section_title: section.title.clone(),
                label: section_label.clone(),
                text: section_text.clone(),
                tagged_words: blocks_tagged_words(site, &section.blocks),
            }));
        }
        // A chapter's front matter is displayed with the chapter's first
        // section, so it is searchable there and nowhere else.
        collect_block_search_chunks(
            site,
            section,
            cll_section_prelude_blocks(site, section),
            &mut chunks,
        );
        collect_block_search_chunks(site, section, &section.blocks, &mut chunks);
    }
    chunks
}

#[requires(true)]
#[ensures(true)]
fn collect_block_search_chunks(
    site: &CllSite,
    section: &CllSection,
    blocks: &[CllBlock],
    chunks: &mut Vec<CllSearchChunk>,
) {
    let mut visitor = SearchChunkVisitor {
        site,
        section,
        chunks,
    };
    visitor.visit_blocks(blocks);
}

#[invariant(true)]
struct SearchChunkVisitor<'site, 'section, 'chunks> {
    site: &'site CllSite,
    section: &'section CllSection,
    chunks: &'chunks mut Vec<CllSearchChunk>,
}

#[contract_trait]
impl CllBlockVisitor for SearchChunkVisitor<'_, '_, '_> {
    #[requires(true)]
    #[ensures(true)]
    fn visit_block(&mut self, block: &CllBlock) {
        match block {
            CllBlock::Paragraph {
                anchor_id,
                role,
                inlines,
                text,
            } => {
                if text.chars().count() > PARAGRAPH_SEARCH_MIN_CHARS {
                    self.chunks.push(new!(CllSearchChunk {
                        kind: CllSearchChunkKind::Paragraph,
                        role: role.clone(),
                        section_id: self.section.section_id.clone(),
                        anchor_id: anchor_id
                            .clone()
                            .unwrap_or_else(|| self.section.section_id.clone()),
                        section_number: self.section.number.map(|number| number.to_string()),
                        section_title: self.section.title.clone(),
                        label: format!(
                            "Paragraph in {}",
                            format_section_display_title(self.section)
                        ),
                        text: text.clone(),
                        tagged_words: inlines_tagged_words(inlines),
                    }));
                }
            }
            CllBlock::Example { example_id } => {
                if let Some(example) = cll_lookup_example(self.site, example_id) {
                    if !example.plain_text.trim().is_empty() {
                        self.chunks.push(new!(CllSearchChunk {
                            kind: CllSearchChunkKind::Example,
                            role: None,
                            section_id: self.section.section_id.clone(),
                            anchor_id: example.anchor_id.clone(),
                            section_number: self.section.number.map(|number| number.to_string()),
                            section_title: self.section.title.clone(),
                            label: example.label.clone(),
                            text: example.plain_text.clone(),
                            tagged_words: example_tagged_words(example),
                        }));
                    }
                    self.visit_blocks(&example.blocks);
                }
            }
            _ => walk_block(self, block),
        }
    }
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
    let normalized = normalize_lojban_input_text(query).unwrap_or_else(|| query.to_owned());
    collect_tagged_words(&normalized)
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
fn blocks_tagged_words(site: &CllSite, blocks: &[CllBlock]) -> BTreeSet<String> {
    let mut visitor = TaggedWordsVisitor::new(site);
    visitor.visit_blocks(blocks);
    visitor.words
}

#[requires(true)]
#[ensures(true)]
pub(super) fn block_tagged_words(site: &CllSite, block: &CllBlock) -> BTreeSet<String> {
    let mut visitor = TaggedWordsVisitor::new(site);
    visitor.visit_block(block);
    visitor.words
}

#[requires(true)]
#[ensures(true)]
fn inlines_tagged_words(inlines: &[CllInline]) -> BTreeSet<String> {
    let mut visitor = TaggedWordsVisitor::new_without_site();
    visitor.visit_inline_run(inlines);
    visitor.words
}

#[invariant(true)]
struct TaggedWordsVisitor<'site> {
    site: Option<&'site CllSite>,
    words: BTreeSet<String>,
}

impl<'site> TaggedWordsVisitor<'site> {
    #[requires(true)]
    #[ensures(ret.site.is_some())]
    fn new(site: &'site CllSite) -> Self {
        Self {
            site: Some(site),
            words: BTreeSet::new(),
        }
    }

    #[requires(true)]
    #[ensures(ret.site.is_none())]
    fn new_without_site() -> Self {
        Self {
            site: None,
            words: BTreeSet::new(),
        }
    }
}

#[contract_trait]
impl CllBlockVisitor for TaggedWordsVisitor<'_> {
    #[requires(true)]
    #[ensures(true)]
    fn visit_block(&mut self, block: &CllBlock) {
        match block {
            CllBlock::Example { example_id } => {
                if let Some(site) = self.site
                    && let Some(example) = cll_lookup_example(site, example_id)
                {
                    self.words.extend(example_tagged_words(example));
                }
            }
            CllBlock::Ebnf { entries, .. } => {
                for entry in entries {
                    self.words.extend(collect_tagged_words(&entry.rule_name));
                    for token in &entry.rhs {
                        self.words
                            .extend(collect_tagged_words(&ebnf_token_plain_text(token)));
                    }
                }
            }
            CllBlock::Media { .. } | CllBlock::Code { .. } | CllBlock::DisplayMath { .. } => {}
            _ => walk_block(self, block),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_inline(&mut self, inline: &CllInline) {
        match inline {
            CllInline::Link {
                target,
                inlines,
                kind: CllLinkKind::Dictionary | CllLinkKind::Rafsi,
            } => {
                self.words.extend(collect_tagged_words(target));
                self.words
                    .extend(collect_tagged_words(&inline_plain_text(inlines)));
            }
            CllInline::Link { .. } => {}
            _ => walk_inline(self, inline),
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn example_tagged_words(example: &CllExample) -> BTreeSet<String> {
    example
        .lines
        .iter()
        .filter(|line| line.kind.is_lojban())
        .flat_map(|line| collect_tagged_words(&line.text))
        .collect()
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn example_plain_text(example: &CllExample) -> String {
    if example.lines.is_empty() {
        normalized_plain_text(&example.plain_text)
    } else {
        example_lines_plain_text(&example.lines)
    }
}

#[requires(true)]
#[ensures(true)]
fn example_lines_plain_text(lines: &[CllExampleLine]) -> String {
    let mut output = String::new();
    for line in lines {
        if !output.is_empty() {
            output.push('\n');
        }
        output.push_str(&line.text);
    }
    output
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
pub fn search_chunk_kind_label(kind: CllSearchChunkKind) -> &'static str {
    match kind {
        CllSearchChunkKind::Section => "section",
        CllSearchChunkKind::Paragraph => "paragraph",
        CllSearchChunkKind::Example => "example",
    }
}
