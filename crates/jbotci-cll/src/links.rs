use std::collections::BTreeMap;

#[allow(unused_imports)]
use bityzba::{contract_trait, ensures, invariant, requires};
use roxmltree::Node;

use crate::visitor::{CllBlockVisitor, walk_block_mut, walk_inline_mut};

use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
struct LinkResolution {
    label: String,
    kind: CllLinkKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
pub(crate) enum AnchorMode {
    TopLevel,
    Nested,
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|href| href.starts_with("../gentufa?text=")))]
pub(super) fn jbo_parse_href(snippet: &str) -> Option<String> {
    (!snippet.is_empty()).then(|| format!("../gentufa?text={}", percent_encode_plain(snippet)))
}

#[requires(node.is_element())]
#[ensures(ret.as_ref().is_none_or(|href| href.starts_with("../gentufa?text=")))]
pub(super) fn top_level_jbo_parse_href(
    anchor_mode: AnchorMode,
    node: Node<'_, '_>,
) -> Option<String> {
    (anchor_mode == AnchorMode::TopLevel)
        .then(|| collect_jbo_snippet(node).and_then(|snippet| jbo_parse_href(&snippet)))
        .flatten()
}

#[requires(node.is_element())]
#[ensures(true)]
pub(super) fn collect_jbo_snippet(node: Node<'_, '_>) -> Option<String> {
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
pub(super) fn collect_title_anchors(
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
            new!(CllAnchor {
                section_id: section_id.to_owned(),
                label: label.to_owned(),
            }),
        ));
    }
}

#[requires(node.is_element())]
#[ensures(true)]
pub(super) fn first_anchor_id(node: Node<'_, '_>) -> Option<String> {
    node.children()
        .find(|child| child.is_element() && child.has_tag_name("anchor"))
        .and_then(xml_id)
}

#[requires(true)]
#[ensures(true)]
pub(super) fn build_index_entries(entries: &[PendingIndexEntry]) -> Vec<CllIndexEntry> {
    let mut grouped: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for entry in entries {
        let section_ids = grouped.entry(entry.key.clone()).or_default();
        if !section_ids.contains(&entry.section_id) {
            section_ids.push(entry.section_id.clone());
        }
    }
    grouped
        .into_iter()
        .map(|(key, section_ids)| new!(CllIndexEntry { key, section_ids }))
        .collect()
}

#[requires(true)]
#[ensures(true)]
pub(super) fn build_section_reference_index(site: &CllSite) -> BTreeMap<String, String> {
    let mut index = BTreeMap::new();
    for section in site.sections_by_id.values() {
        insert_reference(&mut index, &section.section_id, &section.section_id);
        // Appendix sections have no number to cite: their stable `xml:id`,
        // registered above and recorded in fixture provenance, is their
        // addressable form, and their title is their displayed designation.
        if let Some(number) = section.number {
            insert_reference(&mut index, &number.to_string(), &section.section_id);
            insert_reference(
                &mut index,
                &format!("section-{number}"),
                &section.section_id,
            );
        }
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
pub(super) fn build_example_reference_index(site: &CllSite) -> BTreeMap<String, String> {
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
pub(super) fn resolve_site_links(site: CllSite) -> CllSite {
    let resolutions = build_link_resolutions(&site);
    let mut site_data = site.into_data();
    site_data.chapters = site_data
        .chapters
        .into_iter()
        .map(|chapter| {
            let mut chapter_data = chapter.into_data();
            resolve_block_links(&mut chapter_data.prelude_blocks, &resolutions);
            CllChapter::from_data(chapter_data)
        })
        .collect();
    site_data.examples_by_id = site_data
        .examples_by_id
        .into_iter()
        .map(|(id, example)| {
            let mut example_data = example.into_data();
            resolve_block_links(&mut example_data.blocks, &resolutions);
            (id, CllExample::from_data(example_data))
        })
        .collect();
    let site = CllSite::from_data(site_data);
    let example_plain_texts = site
        .examples_by_id
        .iter()
        .map(|(id, example)| (id.clone(), example_plain_text(example)))
        .collect::<BTreeMap<_, _>>();
    let mut site_data = site.into_data();
    site_data.examples_by_id = site_data
        .examples_by_id
        .into_iter()
        .map(|(id, example)| {
            let mut example_data = example.into_data();
            if let Some(plain_text) = example_plain_texts.get(&id) {
                example_data.plain_text = plain_text.clone();
            }
            (id, CllExample::from_data(example_data))
        })
        .collect();
    site_data.sections_by_id = site_data
        .sections_by_id
        .into_iter()
        .map(|(id, section)| {
            let mut section_data = section.into_data();
            resolve_block_links(&mut section_data.blocks, &resolutions);
            (id, CllSection::from_data(section_data))
        })
        .collect();
    let site = CllSite::from_data(site_data);
    let section_plain_texts = site
        .sections_by_id
        .iter()
        .map(|(id, section)| (id.clone(), blocks_plain_text(&site, &section.blocks)))
        .collect::<BTreeMap<_, _>>();
    let mut site_data = site.into_data();
    site_data.sections_by_id = site_data
        .sections_by_id
        .into_iter()
        .map(|(id, section)| {
            let mut section_data = section.into_data();
            if let Some(plain_text) = section_plain_texts.get(&id) {
                section_data.plain_text = normalized_plain_text(plain_text);
            }
            (id, CllSection::from_data(section_data))
        })
        .collect();
    CllSite::from_data(site_data)
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
    let mut visitor = LinkResolutionVisitor { resolutions };
    visitor.visit_blocks_mut(blocks);
}

#[invariant(true)]
struct LinkResolutionVisitor<'a> {
    resolutions: &'a BTreeMap<String, LinkResolution>,
}

#[contract_trait]
impl CllBlockVisitor for LinkResolutionVisitor<'_> {
    #[requires(true)]
    #[ensures(true)]
    fn visit_block_mut(&mut self, block: &mut CllBlock) {
        match block {
            CllBlock::Paragraph { inlines, text, .. } => {
                self.visit_inline_run_mut(inlines);
                *text = normalized_plain_text(&inline_plain_text(inlines));
            }
            CllBlock::Heading { inlines, title, .. } => {
                self.visit_inline_run_mut(inlines);
                *title = normalized_plain_text(&inline_plain_text(inlines));
            }
            _ => walk_block_mut(self, block),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_inline_mut(&mut self, inline: &mut CllInline) {
        match inline {
            CllInline::Link {
                target,
                inlines,
                kind,
            } => {
                self.visit_inline_run_mut(inlines);
                if *kind == CllLinkKind::Section
                    && let Some(resolution) = self.resolutions.get(target)
                {
                    *kind = resolution.kind;
                    if inline_plain_text(inlines) == *target {
                        *inlines = vec![CllInline::Text(resolution.label.clone())];
                    }
                }
            }
            _ => walk_inline_mut(self, inline),
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
fn normalize_reference(reference: &str) -> String {
    reference
        .trim()
        .trim_start_matches('#')
        .to_ascii_lowercase()
}
