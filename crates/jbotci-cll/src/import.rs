use std::collections::BTreeMap;
use std::io::Read;
use std::num::NonZeroUsize;
use std::sync::OnceLock;

use bzip2::read::BzDecoder;
use jbotci_morphology::normalize_lojban_input_text;
use roxmltree::{Document, Node};
use sha2::{Digest, Sha256};

use super::*;

include!(concat!(env!("OUT_DIR"), "/embedded_cll.rs"));

/// The build-time shape of the vendored edition's identity. It exists so the
/// generated constant is plain data with no allocation; `cll_edition()` turns it
/// into the validated model type.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct EmbeddedCllEdition {
    pub(crate) title: &'static str,
    pub(crate) version: &'static str,
    pub(crate) publisher: &'static str,
    pub(crate) ancestry: &'static [EmbeddedCllEditionAncestor],
    pub(crate) upstream_url: &'static str,
    pub(crate) release_tag: &'static str,
    pub(crate) commit: &'static str,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct EmbeddedCllEditionAncestor {
    pub(crate) title: &'static str,
    pub(crate) version: &'static str,
}

static EMBEDDED_SITE: OnceLock<Result<CllSite, CllError>> = OnceLock::new();
static EDITION: OnceLock<CllEdition> = OnceLock::new();
static CLL_IMPORT_METADATA: OnceLock<Result<CllImportMetadata, String>> = OnceLock::new();
static CHRESTOMATHY_METADATA: OnceLock<Result<CllChrestomathyMetadata, String>> = OnceLock::new();

const CLL_IMPORT_METADATA_TOML: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../vendor/cll-import-metadata.toml"
));

const CHRESTOMATHY_METADATA_TOML: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../vendor/cll-chrestomathy.toml"
));

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
pub(crate) struct SectionParseContext {
    pub(crate) chapter_id: String,
    pub(crate) division: CllDivision,
    pub(crate) section_id: String,
    pub(crate) section_number: Option<CllSectionNumber>,
    pub(crate) section_title: String,
    pub(crate) source_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
pub(crate) struct PendingIndexEntry {
    pub(crate) key: String,
    pub(crate) section_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
pub(crate) struct BlockParseState {
    pub(crate) chapter_example_counter: usize,
}

/// The edition of the reference book this build answers from.
///
/// The values are fixed at build time from the vendored sources, so this is the
/// single place any surface — the tool description, the rendered book, the web
/// reader — should take the edition from.
#[requires(true)]
#[ensures(ret.version == EMBEDDED_CLL_EDITION.version)]
pub fn cll_edition() -> &'static CllEdition {
    EDITION.get_or_init(|| {
        new!(CllEdition {
            title: EMBEDDED_CLL_EDITION.title.to_owned(),
            version: EMBEDDED_CLL_EDITION.version.to_owned(),
            publisher: EMBEDDED_CLL_EDITION.publisher.to_owned(),
            ancestry: EMBEDDED_CLL_EDITION
                .ancestry
                .iter()
                .map(|ancestor| {
                    new!(CllEditionAncestor {
                        title: ancestor.title.to_owned(),
                        version: ancestor.version.to_owned(),
                    })
                })
                .collect(),
            upstream_url: EMBEDDED_CLL_EDITION.upstream_url.to_owned(),
            release_tag: EMBEDDED_CLL_EDITION.release_tag.to_owned(),
            commit: EMBEDDED_CLL_EDITION.commit.to_owned(),
        })
    })
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|site| !site.chapters.is_empty()) || ret.is_err())]
pub fn embedded_cll_site() -> Result<&'static CllSite, CllError> {
    EMBEDDED_SITE
        .get_or_init(load_embedded_cll_site)
        .as_ref()
        .map_err(Clone::clone)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|site| !site.chapters.is_empty()) || ret.is_err())]
pub fn load_embedded_cll_site() -> Result<CllSite, CllError> {
    let mut chapters = Vec::new();
    let mut sections_by_id = BTreeMap::new();
    let mut section_order = Vec::new();
    let mut examples_by_id = BTreeMap::new();
    let mut anchors_by_id = BTreeMap::new();
    let mut pending_index_entries = Vec::new();

    for (source_path, division, compressed) in EMBEDDED_CLL_CHAPTERS {
        let xml = decode_chapter_xml(compressed)?;
        let xml = sanitize_xml_entities(&xml);
        let document = Document::parse(&xml)
            .map_err(|error| CllError::Parse(format!("{source_path}: {error}")))?;
        let root = document.root_element();
        let (chapter, sections, examples, anchors, index_entries) =
            parse_chapter(root, *division, source_path)?;
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

    let mut site = new!(CllSite {
        metadata: CllMetadata {
            edition: cll_edition().clone(),
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
    });
    let section_ids_by_normalized_reference = build_section_reference_index(&site);
    site = site.with_data(data! {
        section_ids_by_normalized_reference: section_ids_by_normalized_reference,
    });
    let example_ids_by_normalized_reference = build_example_reference_index(&site);
    site = site.with_data(data! {
        example_ids_by_normalized_reference: example_ids_by_normalized_reference,
    });
    site = resolve_site_links(site);
    let search_chunks = build_search_chunks(&site);
    site = site.with_data(data! {
        search_chunks: search_chunks,
    });
    Ok(site)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()) || ret.is_err())]
pub(crate) fn decode_chapter_xml(compressed: &[u8]) -> Result<String, CllError> {
    let mut decoder = BzDecoder::new(compressed);
    let mut bytes = Vec::new();
    decoder
        .read_to_end(&mut bytes)
        .map_err(|error| CllError::Load(error.to_string()))?;
    String::from_utf8(bytes).map_err(|error| CllError::Load(error.to_string()))
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn sanitize_xml_entities(xml: &str) -> String {
    // These named XML entities appear in the vendored CLL sources but are not
    // predefined XML entities, and roxmltree deliberately does not load an
    // external DTD to resolve them for us.
    xml.replace("&ndash;", "\u{2013}")
        .replace("&hellip;", "\u{2026}")
        .replace("&InvisibleTimes;", "\u{2062}")
}

#[requires(root.is_element())]
#[requires(!source_path.is_empty())]
#[ensures(ret.as_ref().is_ok_and(|(chapter, ..)| chapter.division == division) || ret.is_err())]
fn parse_chapter(
    root: Node<'_, '_>,
    division: CllDivision,
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
    // Every vendored division carries an `xml:id` and a title; the fallbacks
    // below only keep a malformed division addressable, and never invent a
    // designation the book does not use.
    let chapter_id = xml_id(root).unwrap_or_else(|| match division.chapter_number() {
        Some(number) => format!("chapter-{number}"),
        None => format!("appendix-{}", source_path.trim_end_matches(".xml")),
    });
    let title_node = child_element(root, "title");
    let chapter_title = title_node
        .map(visible_text)
        .filter(|title| !title.is_empty())
        .unwrap_or_else(|| match division.chapter_number() {
            Some(number) => format!("Chapter {number}"),
            None => chapter_id.clone(),
        });
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
    let has_sections = root
        .children()
        .any(|child| child.is_element() && child.has_tag_name("section"));

    if let Some(title_node) = title_node {
        collect_title_anchors(
            title_node,
            &chapter_id,
            &cll_numbered_title(division.chapter_number(), &chapter_title),
            &mut anchors,
        );
    }

    if has_sections {
        // Content outside every section - the chapter illustration, and the
        // front matter the appendices open with - belongs to the chapter, but
        // the reader only ever meets it at the top of the chapter's first
        // section. Parsing it with that section's context is therefore not a
        // convenience: every anchor, example, and index entry it contributes
        // has to name the page it is actually displayed on, and it has to go
        // through the same block pipeline as section content so that
        // cross-references, lists, and inline markup survive.
        let prelude_context = SectionParseContext {
            chapter_id: chapter_id.clone(),
            division,
            section_id: first_section_id(root, &chapter_id),
            section_number: division.section_number(
                NonZeroUsize::new(1).expect("the first section is counted from one"),
            ),
            section_title: chapter_title.clone(),
            source_path: source_path.to_owned(),
        };
        // Index terms come from every non-section child, not from the nodes
        // that survive block parsing: an `<indexterm>` carries no visible text,
        // so it is filtered out of `prelude_nodes` below, and a term written as
        // a direct child of the chapter would be lost if the two lists were the
        // same one. This mirrors `parse_section`, which scans its whole
        // container including the title.
        index_entries.extend(index_entries_in(
            &root
                .children()
                .filter(|child| child.is_element() && !child.has_tag_name("section"))
                .collect::<Vec<_>>(),
            &prelude_context.section_id,
        ));
        let prelude_nodes = root
            .children()
            .filter(|child| {
                child.is_text()
                    || (child.is_element()
                        && !child.has_tag_name("title")
                        && !child.has_tag_name("section")
                        && !child.has_tag_name("indexterm"))
            })
            .collect::<Vec<_>>();
        prelude_blocks = parse_blocks_from_nodes(
            &prelude_nodes,
            &prelude_context,
            AnchorMode::TopLevel,
            &mut parse_state,
            &mut examples,
            &mut anchors,
        );

        for child in root
            .children()
            .filter(|child| child.is_element() && child.has_tag_name("section"))
        {
            section_index += 1;
            let parsed = parse_section(
                child,
                &chapter_id,
                division,
                section_index,
                source_path,
                &mut parse_state,
            )?;
            root_section_ids.push(parsed.0.section_id.clone());
            examples.extend(parsed.1);
            anchors.extend(parsed.2);
            index_entries.extend(parsed.3);
            sections.push(parsed.0);
        }
    } else {
        let parsed = parse_sectionless_chapter(
            root,
            &chapter_id,
            division,
            &chapter_title,
            source_path,
            &mut parse_state,
        );
        root_section_ids.push(parsed.0.section_id.clone());
        examples.extend(parsed.1);
        anchors.extend(parsed.2);
        index_entries.extend(parsed.3);
        sections.push(parsed.0);
    }

    anchors.push((
        chapter_id.clone(),
        new!(CllAnchor {
            section_id: root_section_ids
                .first()
                .cloned()
                .unwrap_or_else(|| chapter_id.clone()),
            label: division.xref_label(&chapter_title),
        }),
    ));

    Ok((
        new!(CllChapter {
            chapter_id,
            division,
            chapter_title,
            root_section_ids,
            prelude_blocks,
        }),
        sections,
        examples,
        anchors,
        index_entries,
    ))
}

#[requires(root.is_element())]
#[requires(!chapter_id.is_empty())]
#[requires(!chapter_title.is_empty())]
#[requires(!source_path.is_empty())]
#[ensures(ret.0.division == division)]
#[ensures(ret.0.section_id == chapter_id)]
fn parse_sectionless_chapter(
    root: Node<'_, '_>,
    chapter_id: &str,
    division: CllDivision,
    chapter_title: &str,
    source_path: &str,
    parse_state: &mut BlockParseState,
) -> (
    CllSection,
    Vec<CllExample>,
    Vec<(String, CllAnchor)>,
    Vec<PendingIndexEntry>,
) {
    let section_number = division.whole_chapter_number();
    let context = SectionParseContext {
        chapter_id: chapter_id.to_owned(),
        division,
        section_id: chapter_id.to_owned(),
        section_number,
        section_title: chapter_title.to_owned(),
        source_path: source_path.to_owned(),
    };
    let mut examples = Vec::new();
    let mut anchors = Vec::new();
    let index_entries = index_entries_in(&[root], chapter_id);
    let content_nodes = root
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
    anchors.push((
        chapter_id.to_owned(),
        new!(CllAnchor {
            section_id: chapter_id.to_owned(),
            label: cll_numbered_title(section_number, chapter_title),
        }),
    ));

    (
        new!(CllSection {
            section_id: chapter_id.to_owned(),
            chapter_id: chapter_id.to_owned(),
            division,
            number: section_number,
            title: chapter_title.to_owned(),
            parent_section_id: None,
            child_section_ids: Vec::new(),
            blocks,
            source_path: source_path.to_owned(),
            plain_text: String::new(),
        }),
        examples,
        anchors,
        index_entries,
    )
}

#[requires(section_node.is_element())]
#[requires(section_index > 0)]
#[ensures(ret.as_ref().is_ok_and(|(section, ..)| section.division == division) || ret.is_err())]
fn parse_section(
    section_node: Node<'_, '_>,
    chapter_id: &str,
    division: CllDivision,
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
    let section_id = section_id_for(section_node, chapter_id, section_index);
    let section_number = division.section_number(
        NonZeroUsize::new(section_index).expect("section indexes are counted from one"),
    );
    let title_node = child_element(section_node, "title");
    let section_title = title_node
        .map(visible_text)
        .filter(|title| !title.is_empty())
        .unwrap_or_else(|| match section_number {
            Some(section_number) => format!("Section {section_number}"),
            None => section_id.clone(),
        });
    let context = SectionParseContext {
        chapter_id: chapter_id.to_owned(),
        division,
        section_id: section_id.clone(),
        section_number,
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
            &cll_numbered_title(section_number, &section_title),
            &mut anchors,
        );
    }
    index_entries.extend(index_entries_in(&[section_node], &section_id));

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
    anchors.push((
        section_id.clone(),
        new!(CllAnchor {
            section_id: section_id.clone(),
            label: cll_numbered_title(section_number, &section_title),
        }),
    ));

    Ok((
        new!(CllSection {
            section_id,
            chapter_id: chapter_id.to_owned(),
            division,
            number: section_number,
            title: section_title,
            parent_section_id: None,
            child_section_ids: Vec::new(),
            blocks,
            source_path: source_path.to_owned(),
            plain_text: String::new(),
        }),
        examples,
        anchors,
        index_entries,
    ))
}

/// Every index term under `containers`, each container included in its own
/// scan. Terms are collected from the source containers rather than from the
/// blocks those containers render into: an `<indexterm>` carries no visible
/// text of its own, so it never survives block parsing, and scanning only the
/// nodes that do survive would silently drop a term written as a direct child.
#[requires(!section_id.is_empty())]
#[ensures(ret.iter().all(|entry| entry.section_id == section_id))]
fn index_entries_in(containers: &[Node<'_, '_>], section_id: &str) -> Vec<PendingIndexEntry> {
    containers
        .iter()
        .flat_map(|container| container.descendants())
        .filter(|node| node.is_element() && node.has_tag_name("indexterm"))
        .filter_map(index_key)
        .map(|key| PendingIndexEntry {
            key,
            section_id: section_id.to_owned(),
        })
        .collect()
}

/// The id `parse_section` will give the section at `section_index`, computed
/// without parsing it. Chapter-level content has to name the section it is
/// displayed with before that section has been reached, so both callers derive
/// the id here rather than each spelling out the fallback.
#[requires(section_node.is_element())]
#[requires(!chapter_id.is_empty())]
#[requires(section_index > 0)]
#[ensures(!ret.is_empty())]
fn section_id_for(section_node: Node<'_, '_>, chapter_id: &str, section_index: usize) -> String {
    xml_id(section_node).unwrap_or_else(|| format!("{chapter_id}-s{section_index}"))
}

/// The id of the chapter's first section, or the chapter's own id when the
/// chapter has none - the same fallback `parse_sectionless_chapter` uses, so a
/// chapter's front matter always names an addressable section.
#[requires(root.is_element())]
#[requires(!chapter_id.is_empty())]
#[ensures(!ret.is_empty())]
fn first_section_id(root: Node<'_, '_>, chapter_id: &str) -> String {
    root.children()
        .find(|child| child.is_element() && child.has_tag_name("section"))
        .map(|section| section_id_for(section, chapter_id, 1))
        .unwrap_or_else(|| chapter_id.to_owned())
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
pub(crate) fn parse_block(
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
    // `literallayout` is the only one of these three the current edition uses;
    // `programlisting` left the book when colojban 1.3.4 typeset the PEG
    // appendix, and `screen` has never been in it. All three stay handled: the
    // importer's vocabulary is DocBook's, not one edition's, and unrecognized
    // markup is flattened into prose without any diagnostic, so narrowing this
    // arm would turn a future edition's program listing into silent data loss.
    // `preformatted_block_handling_covers_docbook_rather_than_one_edition`
    // pins the decision.
    if node.has_tag_name("programlisting")
        || node.has_tag_name("screen")
        || node.has_tag_name("literallayout")
    {
        let text = preformatted_text(&raw_text(node));
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
    if is_admonition_element(node) {
        return parse_admonition_blocks(node, context, parse_state, examples, anchors);
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
            role: paragraph_role(node),
            inlines: vec![CllInline::Text(text.clone())],
            text,
        })
        .into_iter()
        .collect()
}

#[requires(node.is_element())]
#[ensures(true)]
pub(crate) fn parse_paragraph_blocks(
    node: Node<'_, '_>,
    context: &SectionParseContext,
    anchor_mode: AnchorMode,
    parse_state: &mut BlockParseState,
    examples: &mut Vec<CllExample>,
    anchors: &mut Vec<(String, CllAnchor)>,
) -> Vec<CllBlock> {
    let mut blocks = Vec::new();
    let mut inline_nodes = Vec::new();
    let mut paragraph_anchor_id = paragraph_anchor_id_for(anchor_mode, context, node);
    let paragraph_role = paragraph_role(node);
    for child in node.children() {
        if child.is_element() && is_block_element(child) {
            flush_inline_nodes_as_paragraph(
                &mut blocks,
                &mut inline_nodes,
                paragraph_anchor_id.take(),
                paragraph_role.clone(),
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
        paragraph_anchor_id.take(),
        paragraph_role,
    );
    blocks
}

#[requires(node.is_element())]
#[ensures(true)]
fn parse_admonition_blocks(
    node: Node<'_, '_>,
    context: &SectionParseContext,
    parse_state: &mut BlockParseState,
    examples: &mut Vec<CllExample>,
    anchors: &mut Vec<(String, CllAnchor)>,
) -> Vec<CllBlock> {
    let mut blocks = parse_blocks_from_nodes(
        &node.children().collect::<Vec<_>>(),
        context,
        AnchorMode::Nested,
        parse_state,
        examples,
        anchors,
    );
    if blocks.is_empty() {
        let inlines = trim_inline_runs(parse_inlines(node));
        let text = normalized_plain_text(&inline_plain_text(&inlines));
        if !text.is_empty() {
            blocks.push(CllBlock::Paragraph {
                anchor_id: xml_id(node),
                role: CllParagraphRole::parse(node.tag_name().name()),
                inlines,
                text,
            });
        }
    }
    blocks
}

#[requires(true)]
#[ensures(inline_nodes.is_empty())]
fn flush_inline_nodes_as_paragraph(
    blocks: &mut Vec<CllBlock>,
    inline_nodes: &mut Vec<Node<'_, '_>>,
    anchor_id: Option<String>,
    role: Option<CllParagraphRole>,
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
    // Numbered chapters number their examples `chapter.position`; an appendix
    // has no number to qualify with, so its examples are numbered by position
    // within the appendix, exactly as the book's own formatter would.
    let example_number = match context.division.chapter_number() {
        Some(chapter_number) => {
            format!("{chapter_number}.{}", parse_state.chapter_example_counter)
        }
        None => parse_state.chapter_example_counter.to_string(),
    };
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
        .filter(|line| line.kind.is_lojban())
        .map(|line| line.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    let gloss_en = lines
        .iter()
        .find(|line| line.kind == CllExampleLineKind::Gloss)
        .map(|line| line.text.clone());
    let translation_en = lines
        .iter()
        .find(|line| line.kind == CllExampleLineKind::Natlang)
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
    let example = new!(CllExample {
        reference: new!(CllReference {
            division: context.division,
            section_number: context.section_number,
            section_id: context.section_id.clone(),
            example_number: Some(example_number),
            example_id: Some(anchor_id.clone()),
            source_path: context.source_path.clone(),
        }),
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
    });
    anchors.push((
        anchor_id.clone(),
        new!(CllAnchor {
            section_id: context.section_id.clone(),
            label: display_label.clone(),
        }),
    ));
    if let Some(xml_id) = xml_id {
        anchors.push((
            xml_id,
            new!(CllAnchor {
                section_id: context.section_id.clone(),
                label: display_label,
            }),
        ));
    }
    examples.push(example);
    Some(CllBlock::Example {
        example_id: anchor_id,
    })
}

#[requires(node.is_element())]
#[ensures(
    ret.as_ref().is_none_or(|block| matches!(
        block,
        CllBlock::Table { header_rows, body_rows, .. } if !header_rows.is_empty() || !body_rows.is_empty()
    )),
    "a table imports as a table with rows, or not at all"
)]
#[ensures(
    !declares_populated_header_row(node)
        || ret.as_ref().is_some_and(|block| matches!(
            block,
            CllBlock::Table { header_rows, .. } if !header_rows.is_empty()
        )),
    "a `thead` holding at least one cell always imports as header rows: resolving the two row areas under different nodes used to drop them silently"
)]
fn parse_table_block(
    node: Node<'_, '_>,
    context: &SectionParseContext,
    parse_state: &mut BlockParseState,
    examples: &mut Vec<CllExample>,
    anchors: &mut Vec<(String, CllAnchor)>,
) -> Option<CllBlock> {
    let source = table_row_area_root(node);
    let header_rows = child_element(source, "thead")
        .map(|thead| {
            parse_table_rows(
                thead,
                CllTableRowArea::Header,
                context,
                parse_state,
                examples,
                anchors,
            )
        })
        .unwrap_or_default();
    let tbody_rows = child_element(source, "tbody")
        .map(|tbody| {
            parse_table_rows(
                tbody,
                CllTableRowArea::Body,
                context,
                parse_state,
                examples,
                anchors,
            )
        })
        .unwrap_or_default();
    let body_rows = if tbody_rows.is_empty() {
        parse_table_rows(
            source,
            CllTableRowArea::Body,
            context,
            parse_state,
            examples,
            anchors,
        )
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
    area: CllTableRowArea,
    context: &SectionParseContext,
    parse_state: &mut BlockParseState,
    examples: &mut Vec<CllExample>,
    anchors: &mut Vec<(String, CllAnchor)>,
) -> Vec<Vec<CllTableCell>> {
    let rows = node
        .children()
        .filter(|row| row.is_element() && (row.has_tag_name("row") || row.has_tag_name("tr")))
        .collect::<Vec<_>>();
    rows.iter()
        .enumerate()
        .map(|(index, row)| {
            parse_table_row(
                *row,
                area,
                index + 1,
                &rows,
                context,
                parse_state,
                examples,
                anchors,
            )
        })
        .filter(|row| !row.is_empty())
        .collect()
}

/// The node whose children hold a table's `thead` and `tbody`. DocBook wraps
/// the row areas in `tgroup`; the vendored sources also use the HTML shape,
/// where both are direct children of the table. Both areas must be resolved
/// under this one node - descending into `tbody` to look for `thead` would hide
/// a sibling header and silently drop the table's header row.
#[requires(node.is_element())]
#[ensures(ret == node || ret.parent() == Some(node))]
fn table_row_area_root<'a, 'input>(node: Node<'a, 'input>) -> Node<'a, 'input> {
    child_element(node, "tgroup").unwrap_or(node)
}

/// Whether the source table declares a header row carrying at least one cell.
///
/// This deliberately does **not** go through `table_row_area_root`: it is the
/// antecedent of `parse_table_block`'s postcondition, and a postcondition that
/// resolved the header the same way the implementation does would restate the
/// implementation instead of constraining it - reintroducing the resolver bug
/// would then leave the contract silently vacuous. So it looks for a `thead`
/// in both places the vendored sources put one, directly under the table and
/// inside a DocBook `tgroup`, and asks the question the importer must answer.
#[requires(node.is_element())]
#[ensures(true)]
fn declares_populated_header_row(node: Node<'_, '_>) -> bool {
    node.children()
        .chain(
            child_element(node, "tgroup")
                .into_iter()
                .flat_map(|tgroup| tgroup.children()),
        )
        .filter(|child| child.is_element() && child.has_tag_name("thead"))
        .flat_map(|thead| thead.children())
        .filter(|row| row.is_element() && (row.has_tag_name("row") || row.has_tag_name("tr")))
        .any(|row| row.children().any(is_table_cell))
}

/// The cell elements of a table row, in the DocBook (`entry`) and HTML
/// (`td`/`th`) spellings the vendored sources mix.
#[requires(true)]
#[ensures(ret == (cell.is_element() && (cell.has_tag_name("entry") || cell.has_tag_name("td") || cell.has_tag_name("th"))))]
fn is_table_cell(cell: Node<'_, '_>) -> bool {
    cell.is_element()
        && (cell.has_tag_name("entry") || cell.has_tag_name("td") || cell.has_tag_name("th"))
}

#[requires(row.is_element())]
#[ensures(true)]
fn parse_table_row(
    row: Node<'_, '_>,
    area: CllTableRowArea,
    row_index: usize,
    area_rows: &[Node<'_, '_>],
    context: &SectionParseContext,
    parse_state: &mut BlockParseState,
    examples: &mut Vec<CllExample>,
    anchors: &mut Vec<(String, CllAnchor)>,
) -> Vec<CllTableCell> {
    row.children()
        .filter(|cell| is_table_cell(*cell))
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
            let parse_info =
                chrestomathy_parse_info(context, area, row_index, area_rows, cell_index, cell);
            let data!(CllChrestomathyParseInfo {
                parse_href,
                parse_group,
            }) = parse_info.into_data();
            new!(CllTableCell {
                blocks,
                col_span: attr_usize(cell, "colspan"),
                row_span: attr_usize(cell, "rowspan"),
                parse_href,
                parse_group,
            })
        })
        .collect()
}

#[requires(cell.is_element())]
#[requires(row_index > 0)]
#[ensures(ret.parse_href.as_ref().is_none_or(|href| href.starts_with("../gentufa?text=")))]
fn chrestomathy_parse_info(
    context: &SectionParseContext,
    area: CllTableRowArea,
    row_index: usize,
    area_rows: &[Node<'_, '_>],
    cell_index: usize,
    cell: Node<'_, '_>,
) -> CllChrestomathyParseInfo {
    if context.chapter_id != cll_import_metadata().chrestomathy_chapter_id
        || cell_index != 0
        || !is_table_cell(cell)
    {
        return new!(CllChrestomathyParseInfo {
            parse_href: None,
            parse_group: None,
        });
    }
    let Some(metadata) = chrestomathy_section_metadata(&context.section_id) else {
        return new!(CllChrestomathyParseInfo {
            parse_href: None,
            parse_group: None,
        });
    };
    let Some((group_index, group_rows)) =
        chrestomathy_group_containing_row(metadata, area, row_index)
    else {
        return new!(CllChrestomathyParseInfo {
            parse_href: None,
            parse_group: None,
        });
    };
    let row_position = group_rows
        .iter()
        .position(|row| *row == row_index)
        .expect("group lookup returns groups containing the requested row");
    let group_id = chrestomathy_group_id(&context.section_id, area, group_index, group_rows);
    let parse_href = (row_position == 0)
        .then(|| chrestomathy_group_text_from_rows(area_rows, group_rows))
        .flatten()
        .and_then(|text| jbo_parse_href(&text));
    new!(CllChrestomathyParseInfo {
        parse_href,
        parse_group: Some(new!(CllTableParseGroup {
            group_id,
            row_count: group_rows.len(),
            row_index: row_position,
        })),
    })
}

#[requires(true)]
#[ensures(!ret.chrestomathy_chapter_id.is_empty())]
pub(crate) fn cll_import_metadata() -> &'static CllImportMetadata {
    CLL_IMPORT_METADATA
        .get_or_init(|| {
            toml::from_str::<CllImportMetadata>(CLL_IMPORT_METADATA_TOML)
                .map_err(|error| error.to_string())
        })
        .as_ref()
        .expect("vendor/cll-import-metadata.toml must be valid")
}

#[requires(true)]
#[ensures(!ret.section.is_empty())]
pub(crate) fn chrestomathy_metadata() -> &'static CllChrestomathyMetadata {
    CHRESTOMATHY_METADATA
        .get_or_init(|| {
            toml::from_str::<CllChrestomathyMetadata>(CHRESTOMATHY_METADATA_TOML)
                .map_err(|error| error.to_string())
        })
        .as_ref()
        .expect("vendor/cll-chrestomathy.toml must be valid")
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|metadata| metadata.id == section_id))]
pub(crate) fn chrestomathy_section_metadata(
    section_id: &str,
) -> Option<&'static CllChrestomathySectionMetadata> {
    chrestomathy_metadata()
        .section
        .iter()
        .find(|metadata| metadata.id == section_id)
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn chrestomathy_area_groups(
    metadata: &CllChrestomathySectionMetadata,
    area: CllTableRowArea,
) -> &[Vec<usize>] {
    match area {
        CllTableRowArea::Header => &metadata.header_groups,
        CllTableRowArea::Body => &metadata.body_groups,
    }
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn chrestomathy_area_no_parse_rows(
    metadata: &CllChrestomathySectionMetadata,
    area: CllTableRowArea,
) -> &[usize] {
    match area {
        CllTableRowArea::Header => &metadata.header_no_parse,
        CllTableRowArea::Body => &metadata.body_no_parse,
    }
}

#[requires(row_index > 0)]
#[ensures(ret.as_ref().is_none_or(|(_, rows)| rows.contains(&row_index)))]
fn chrestomathy_group_containing_row(
    metadata: &CllChrestomathySectionMetadata,
    area: CllTableRowArea,
    row_index: usize,
) -> Option<(usize, &[usize])> {
    chrestomathy_area_groups(metadata, area)
        .iter()
        .enumerate()
        .find(|(_, group)| group.contains(&row_index))
        .map(|(index, group)| (index, group.as_slice()))
}

#[requires(!section_id.is_empty())]
#[requires(!group_rows.is_empty())]
#[ensures(!ret.is_empty())]
pub(crate) fn chrestomathy_group_id(
    section_id: &str,
    area: CllTableRowArea,
    group_index: usize,
    group_rows: &[usize],
) -> String {
    let first = group_rows
        .first()
        .expect("precondition requires non-empty group");
    let last = group_rows
        .last()
        .expect("precondition requires non-empty group");
    format!(
        "{}-{}-{}-{}-{}",
        section_id,
        chrestomathy_area_label(area),
        group_index + 1,
        first,
        last
    )
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub(crate) fn chrestomathy_area_label(area: CllTableRowArea) -> &'static str {
    match area {
        CllTableRowArea::Header => "header",
        CllTableRowArea::Body => "body",
    }
}

#[requires(group_rows.iter().all(|row| *row > 0))]
#[ensures(ret.as_ref().is_none_or(|text| !text.trim().is_empty()))]
fn chrestomathy_group_text_from_rows(
    area_rows: &[Node<'_, '_>],
    group_rows: &[usize],
) -> Option<String> {
    let mut lines = Vec::new();
    for row_index in group_rows {
        let row = area_rows.get(row_index.checked_sub(1)?)?;
        let text = chrestomathy_source_row_text(*row)?;
        if !text.trim().is_empty() {
            lines.push(text);
        }
    }
    (!lines.is_empty()).then(|| lines.join("\n"))
}

#[requires(row.is_element())]
#[ensures(ret.as_ref().is_none_or(|text| !text.trim().is_empty()))]
fn chrestomathy_source_row_text(row: Node<'_, '_>) -> Option<String> {
    row.children()
        .filter(|cell| is_table_cell(*cell))
        .next()
        .map(visible_text)
        .filter(|text| !text.trim().is_empty())
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
    if context.section_id == cll_import_metadata().ebnf_section_id {
        return parse_ebnf_block(node, context, anchors);
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
        .filter_map(|descendant| {
            if !descendant.is_element() {
                return None;
            }
            let kind = CllExampleLineKind::parse_tag(descendant.tag_name().name())?;
            (kind != CllExampleLineKind::Text).then_some((descendant, kind))
        })
        .filter_map(|(line, kind)| {
            let text = visible_text(line);
            (!text.is_empty()).then_some(new!(CllExampleLine { kind, text }))
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
            (!text.is_empty()).then_some(new!(CllExampleLine {
                kind: CllExampleLineKind::Text,
                text,
            }))
        })
        .collect::<Vec<_>>();
    if lines.is_empty() {
        let text = visible_text(node);
        (!text.is_empty())
            .then_some(new!(CllExampleLine {
                kind: CllExampleLineKind::Text,
                text,
            }))
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

/// The DocBook block vocabulary the importer understands. It deliberately
/// exceeds what the vendored edition happens to contain: nine of these are
/// absent from the book today - `simpara`, `screen`, `math`, the five
/// admonitions, and, since colojban 1.3.4 typeset the PEG appendix,
/// `programlisting`. They stay because an element that is not listed here is
/// not rejected: it is merged into the surrounding prose without any
/// diagnostic. Narrowing the list to the current edition would make the next
/// edition's markup fail silently instead of loudly.
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
fn is_admonition_element(node: Node<'_, '_>) -> bool {
    matches!(
        node.tag_name().name(),
        "note" | "tip" | "warning" | "important" | "caution"
    )
}

#[requires(node.is_element())]
#[ensures(true)]
fn is_display_none_element(node: Node<'_, '_>) -> bool {
    attr_value(node, "role").is_some_and(|role| role.trim().eq_ignore_ascii_case("display-none"))
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
pub(crate) fn block_anchor_id_for(
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
pub(crate) fn normalize_valsis_query(text: &str) -> String {
    let normalized = normalize_lojban_input_text(text).unwrap_or_else(|| text.to_owned());
    normalized
        .trim()
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
            kind: CllInterlinearRowKind::Jbo,
            cells: jbo_tokens
                .iter()
                .map(|token| linked_jbo_text_inlines(token))
                .collect(),
        }),
        new!(CllInterlinearRow {
            kind: CllInterlinearRowKind::Gloss,
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
    let kind = CllInterlinearRowKind::parse_tag(line.tag_name().name())?;
    let body = if kind.is_lojban() {
        linked_jbo_text_inlines(&normalized_plain_text(&visible_text_raw(line)))
    } else if kind.is_math() {
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
    let kind = CllInterlinearRowKind::parse_tag(line.tag_name().name())?;
    let cells = line
        .children()
        .flat_map(|child| collect_interlinear_cell(child, kind))
        .map(trim_inline_runs)
        .filter(|cell| !cell.is_empty())
        .collect::<Vec<_>>();
    (!cells.is_empty()).then_some(new!(CllInterlinearRow { kind, cells }))
}

#[requires(true)]
#[ensures(true)]
fn collect_interlinear_cell(
    node: Node<'_, '_>,
    kind: CllInterlinearRowKind,
) -> Vec<Vec<CllInline>> {
    if node.is_text() {
        let text = normalized_plain_text(node.text().unwrap_or_default());
        if text.is_empty() {
            return Vec::new();
        }
        return vec![if kind.is_lojban() {
            linked_jbo_text_inlines(&text)
        } else {
            vec![CllInline::Text(text)]
        }];
    }
    if !node.is_element() || node.has_tag_name("indexterm") {
        return Vec::new();
    }
    if kind.is_lojban() {
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
    let headers = header_cells
        .iter()
        .map(|cell| trim_inline_runs(parse_inlines(*cell)))
        .filter(|body| !body.is_empty())
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
            let kind = CllLojbanizationLineKind::parse_tag(line.tag_name().name())?;
            let body_nodes = line
                .children()
                .filter(|child| {
                    child.is_text()
                        || (child.is_element()
                            && !child.has_tag_name("comment")
                            && !child.has_tag_name("indexterm"))
                })
                .collect::<Vec<_>>();
            let body = if kind == CllLojbanizationLineKind::Jbo {
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
            let kind = CllLujvoPartKind::parse_tag(part.tag_name().name())?;
            let body = if kind.is_lojban() {
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
pub(crate) fn child_element<'a, 'input>(
    node: Node<'a, 'input>,
    name: &str,
) -> Option<Node<'a, 'input>> {
    node.children()
        .find(|child| child.is_element() && child.has_tag_name(name))
}

#[requires(node.is_element())]
#[ensures(true)]
pub(crate) fn xml_id(node: Node<'_, '_>) -> Option<String> {
    node.attribute(("http://www.w3.org/XML/1998/namespace", "id"))
        .or_else(|| node.attribute("xml:id"))
        .or_else(|| node.attribute("id"))
        .map(str::to_owned)
}

#[requires(node.is_element())]
#[ensures(true)]
pub(crate) fn attr_string(node: Node<'_, '_>, name: &str) -> Option<String> {
    attr_value(node, name).map(str::to_owned)
}

/// The borrowed form of [`attr_string`], for callers that only inspect the
/// value or build something other than a plain copy of it.
#[requires(!name.is_empty())]
#[ensures(true)]
pub(crate) fn attr_value<'a>(node: Node<'a, '_>, name: &str) -> Option<&'a str> {
    node.attribute(name).or_else(|| {
        let local_name = name.rsplit(':').next().unwrap_or(name);
        node.attributes()
            .find(|attribute| attribute.name() == local_name)
            .map(|attribute| attribute.value())
    })
}

/// The typed designation of a paragraph-bearing element, from its DocBook
/// `role`.
#[requires(node.is_element())]
#[ensures(true)]
fn paragraph_role(node: Node<'_, '_>) -> Option<CllParagraphRole> {
    attr_value(node, "role").and_then(CllParagraphRole::parse)
}

#[requires(node.is_element())]
#[ensures(true)]
pub(crate) fn visible_text(node: Node<'_, '_>) -> String {
    normalized_plain_text(&visible_text_raw(node))
}

#[requires(node.is_element())]
#[ensures(true)]
pub(crate) fn visible_text_raw(node: Node<'_, '_>) -> String {
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
pub(crate) fn raw_text(node: Node<'_, '_>) -> String {
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
pub(crate) fn normalized_plain_text(text: &str) -> String {
    normalize_text_fragment(text).trim().to_owned()
}

#[requires(true)]
#[ensures(!ret.starts_with('\n'))]
#[ensures(!ret.ends_with('\n'))]
fn preformatted_text(text: &str) -> String {
    let text = text.replace("\r\n", "\n").replace('\r', "\n");
    let lines = text.split('\n').collect::<Vec<_>>();
    let Some(start) = lines.iter().position(|line| !line.trim().is_empty()) else {
        return String::new();
    };
    let end = lines
        .iter()
        .rposition(|line| !line.trim().is_empty())
        .map(|index| index + 1)
        .expect("start proves at least one non-empty line is present");
    let body = &lines[start..end];
    let common_indent = body
        .iter()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.bytes().take_while(|byte| *byte == b' ').count())
        .min()
        .unwrap_or(0);

    // Literal blocks are nested in pretty-printed XML; remove that common source
    // margin while preserving the layout inside the block.
    body.iter()
        .map(|line| {
            if line.trim().is_empty() {
                String::new()
            } else {
                line[common_indent..].to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    /// A probe chapter used to exercise the division skeleton.
    const DIVISION_PROBE: &str = concat!(
        r#"<article xml:id="probe-chapter"><title>PROBE</title>"#,
        r#"<section xml:id="probe-section"><title>PROBE</title><para>PROBE</para></section>"#,
        "</article>",
    );
    const CHAPTER_ROOT_PROBE: &str = concat!(
        r#"<chapter xml:id="probe-chapter"><title>PROBE</title>"#,
        r#"<section xml:id="probe-section"><title>PROBE</title><para>PROBE</para></section>"#,
        "</chapter>",
    );
    const TABLE_PROBE: &str = concat!(
        "<informaltable><thead><tr><th>PROBE</th></tr></thead>",
        "<tbody><tr><td>PROBE</td></tr></tbody></informaltable>",
    );
    const TGROUP_TABLE_PROBE: &str = concat!(
        "<informaltable><tgroup><thead><row><entry>PROBE</entry></row></thead>",
        "<tbody><row><entry>PROBE</entry></row></tbody></tgroup></informaltable>",
    );
    const CAPTION_TABLE_PROBE: &str =
        "<table><caption>PROBE</caption><tbody><tr><td>x</td></tr></tbody></table>";
    const CMAVO_LIST_PROBE: &str = concat!(
        "<cmavo-list><cmavo-list-head><cmavo>PROBE</cmavo></cmavo-list-head>",
        "<cmavo-entry><cmavo>PROBE</cmavo><description>PROBE</description>",
        "<selmaho>PROBE</selmaho><attitudinal-scale>PROBE</attitudinal-scale>",
        "<modal-place>PROBE</modal-place><rafsi-group>PROBE</rafsi-group>",
        "<pseudo-cmavo>PROBE</pseudo-cmavo><series>PROBE</series></cmavo-entry></cmavo-list>",
    );
    const MEDIA_PROBE: &str = concat!(
        r#"<mediaobject><imageobject><imagedata fileref="PROBE.png"/></imageobject>"#,
        "<textobject><phrase>PROBE</phrase></textobject></mediaobject>",
    );
    const GLOSS_PROBE: &str = "<interlinear-gloss><jbo>PROBE</jbo><gloss>PROBE</gloss><natlang>PROBE</natlang></interlinear-gloss>";
    const GLOSS_ITEMIZED_PROBE: &str = concat!(
        "<interlinear-gloss-itemized><jbo><sumti>PROBE</sumti><selbri>PROBE</selbri></jbo>",
        "</interlinear-gloss-itemized>",
    );
    const LUJVO_PROBE: &str = concat!(
        "<lujvo-making><jbo>PROBE</jbo><veljvo>PROBE</veljvo><gloss>g</gloss>",
        "<natlang>n</natlang><score>PROBE</score></lujvo-making>",
    );
    const EXAMPLE_PROBE: &str = concat!(
        "<example><title/><pronunciation><ipa>PROBE</ipa></pronunciation>",
        "<interlinear-gloss><jbo><compound-cmavo>PROBE</compound-cmavo></jbo></interlinear-gloss></example>",
    );
    const MATH_PROBE: &str = concat!(
        "<dbmath><mrow><mfrac><mi>PROBE</mi><mn>2</mn></mfrac><mo>+</mo>",
        "<msqrt><mi>x</mi></msqrt><msup><mi>y</mi><mn>2</mn></msup></mrow></dbmath>",
    );

    /// What the importer does with an element, as something the test can check
    /// rather than something the inventory merely asserts.
    ///
    /// The importer has no error channel for markup it does not recognize: an
    /// unhandled block element falls through `parse_block` to a flowed
    /// paragraph of its visible text, and an unhandled inline element falls
    /// through `parse_inlines` to its children's text. Both losses are silent.
    /// Declaring a disposition therefore has to *cost* something: every variant
    /// below carries a probe, and the test runs it through the real importer
    /// entry point and compares what came back. Adding a newly vendored tag to
    /// the inventory without also handling it leaves only `Flattened`, whose
    /// written reason is the loud part a reviewer reads.
    // Audited no-op markers. Every field of every variant is a probe or an
    // expected name, and `every_inventoried_element_is_treated_as_the_inventory
    // _declares` runs each one through the real importer: a malformed probe
    // fails to parse, an empty expectation fails to match, and a blank
    // `Flattened` reason fails its own assertion. A structural invariant here
    // would restate, more weakly, what that test already enforces per entry.
    #[invariant(true)]
    #[invariant(::Division { .. } => true)]
    #[invariant(::Block { .. } => true)]
    #[invariant(::Inline { .. } => true)]
    #[invariant(::Consumed { .. } => true)]
    #[invariant(::IndexKey { .. } => true)]
    #[invariant(::Structural { .. } => true)]
    #[invariant(::Transparent { .. } => true)]
    #[invariant(::Flattened { .. } => true)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum ElementDisposition {
        /// Part of a division's skeleton, read by `parse_chapter`/`parse_section`.
        Division { probe: &'static str },
        /// `is_block_element` is true, and `parse_block` yields `block`.
        Block {
            probe: &'static str,
            block: &'static str,
        },
        /// `parse_inlines` yields at least one inline of kind `inline`.
        Inline {
            probe: &'static str,
            inline: &'static str,
        },
        /// Read only for a side effect - registering an anchor, feeding the
        /// index - and contributing no inline of its own.
        Consumed { probe: &'static str },
        /// Read by `index_key` when building an `<indexterm>`'s entry.
        IndexKey {
            probe: &'static str,
            key: &'static str,
        },
        /// Meaningful only inside a container whose parser reads it *by name*:
        /// renaming the element changes what the container imports.
        Structural {
            probe: &'static str,
            block: &'static str,
        },
        /// Inside a container that reaches its content without consulting the
        /// tag name - by descendant search, or by taking element children in
        /// order. The content survives import; the name itself is invisible to
        /// the importer, so renaming the element changes nothing. A new sibling
        /// added to such a container would silently become another cell.
        Transparent {
            probe: &'static str,
            block: &'static str,
        },
        /// No handler at all: reduced to its own text, deliberately.
        Flattened { reason: &'static str },
    }

    /// Every element name that appears anywhere in the vendored sources, with
    /// the importer's treatment of it.
    ///
    /// This is the place where a newly vendored edition's markup becomes loud.
    /// The test below checks the list against the sources in both directions,
    /// so a new tag cannot appear without an entry; and it checks every entry's
    /// disposition against the importer, so an entry cannot be added without
    /// either a handler or an explicit statement that the tag is flattened.
    const VENDORED_ELEMENTS: &[(&str, ElementDisposition)] = &[
        (
            "anchor",
            ElementDisposition::Inline {
                probe: r#"<para><anchor xml:id="probe"/>PROBE</para>"#,
                inline: "Anchor",
            },
        ),
        (
            "article",
            ElementDisposition::Division {
                probe: DIVISION_PROBE,
            },
        ),
        (
            "attitudinal-scale",
            ElementDisposition::Transparent {
                probe: CMAVO_LIST_PROBE,
                block: "CmavoList",
            },
        ),
        (
            "blockquote",
            ElementDisposition::Block {
                probe: "<blockquote><para>PROBE</para></blockquote>",
                block: "BlockQuote",
            },
        ),
        (
            "bridgehead",
            ElementDisposition::Block {
                probe: "<bridgehead>PROBE</bridgehead>",
                block: "Heading",
            },
        ),
        (
            "caption",
            ElementDisposition::Structural {
                probe: CAPTION_TABLE_PROBE,
                block: "Table",
            },
        ),
        (
            "chapter",
            ElementDisposition::Division {
                probe: CHAPTER_ROOT_PROBE,
            },
        ),
        (
            "citetitle",
            ElementDisposition::Inline {
                probe: "<para><citetitle>PROBE</citetitle></para>",
                inline: "CiteTitle",
            },
        ),
        (
            "cmavo",
            ElementDisposition::Inline {
                probe: "<para><cmavo>PROBE</cmavo></para>",
                inline: "Link",
            },
        ),
        (
            "cmavo-entry",
            ElementDisposition::Structural {
                probe: CMAVO_LIST_PROBE,
                block: "CmavoList",
            },
        ),
        (
            "cmavo-list",
            ElementDisposition::Block {
                probe: CMAVO_LIST_PROBE,
                block: "CmavoList",
            },
        ),
        (
            "cmavo-list-head",
            ElementDisposition::Structural {
                probe: CMAVO_LIST_PROBE,
                block: "CmavoList",
            },
        ),
        (
            "cmevla",
            ElementDisposition::Inline {
                probe: "<para><cmevla>PROBE</cmevla></para>",
                inline: "Link",
            },
        ),
        (
            "colgroup",
            ElementDisposition::Flattened {
                reason: "a column-width declaration carrying no text; the importer reads a table's rows, never its column groups",
            },
        ),
        (
            "comment",
            ElementDisposition::Flattened {
                reason: "an inline parenthetical printed as part of the gloss line it annotates, so its own text is the content",
            },
        ),
        (
            "compound-cmavo",
            ElementDisposition::Transparent {
                probe: EXAMPLE_PROBE,
                block: "Example",
            },
        ),
        (
            "content",
            ElementDisposition::Transparent {
                probe: "<definition><content>PROBE</content></definition>",
                block: "Definition",
            },
        ),
        (
            "dbinlinemath",
            ElementDisposition::Inline {
                probe: "<para><dbinlinemath><mi>PROBE</mi></dbinlinemath></para>",
                inline: "InlineMath",
            },
        ),
        (
            "dbmath",
            ElementDisposition::Block {
                probe: MATH_PROBE,
                block: "DisplayMath",
            },
        ),
        (
            "definition",
            ElementDisposition::Block {
                probe: "<definition><content>PROBE</content></definition>",
                block: "Definition",
            },
        ),
        (
            "description",
            ElementDisposition::Transparent {
                probe: CMAVO_LIST_PROBE,
                block: "CmavoList",
            },
        ),
        (
            "diphthong",
            ElementDisposition::Flattened {
                reason: "names a diphthong inline; the book prints the letters themselves, which is what the text carries",
            },
        ),
        (
            "elidable",
            ElementDisposition::Inline {
                probe: "<para><elidable>PROBE</elidable></para>",
                inline: "Elidable",
            },
        ),
        (
            "emphasis",
            ElementDisposition::Inline {
                probe: "<para><emphasis>PROBE</emphasis></para>",
                inline: "Emphasis",
            },
        ),
        (
            "example",
            ElementDisposition::Block {
                probe: EXAMPLE_PROBE,
                block: "Example",
            },
        ),
        (
            "foreignphrase",
            ElementDisposition::Inline {
                probe: "<para><foreignphrase>PROBE</foreignphrase></para>",
                inline: "LanguageSpan",
            },
        ),
        (
            "gismu",
            ElementDisposition::Inline {
                probe: "<para><gismu>PROBE</gismu></para>",
                inline: "Link",
            },
        ),
        (
            "gloss",
            ElementDisposition::Structural {
                probe: GLOSS_PROBE,
                block: "InterlinearGloss",
            },
        ),
        (
            "grammar-template",
            ElementDisposition::Block {
                probe: "<grammar-template>PROBE</grammar-template>",
                block: "GrammarTemplate",
            },
        ),
        (
            "imagedata",
            ElementDisposition::Structural {
                probe: MEDIA_PROBE,
                block: "Media",
            },
        ),
        (
            "imageobject",
            ElementDisposition::Transparent {
                probe: MEDIA_PROBE,
                block: "Media",
            },
        ),
        (
            "indexterm",
            ElementDisposition::Consumed {
                probe: "<para><indexterm><primary>PROBE</primary></indexterm>PROBE</para>",
            },
        ),
        (
            "informaltable",
            ElementDisposition::Block {
                probe: TABLE_PROBE,
                block: "Table",
            },
        ),
        (
            "interlinear-gloss",
            ElementDisposition::Block {
                probe: GLOSS_PROBE,
                block: "InterlinearGloss",
            },
        ),
        (
            "interlinear-gloss-itemized",
            ElementDisposition::Block {
                probe: GLOSS_ITEMIZED_PROBE,
                block: "InterlinearGloss",
            },
        ),
        (
            "ipa",
            ElementDisposition::Transparent {
                probe: EXAMPLE_PROBE,
                block: "Example",
            },
        ),
        (
            "itemizedlist",
            ElementDisposition::Block {
                probe: "<itemizedlist><listitem><para>PROBE</para></listitem></itemizedlist>",
                block: "List",
            },
        ),
        (
            "jbo",
            ElementDisposition::Structural {
                probe: GLOSS_PROBE,
                block: "InterlinearGloss",
            },
        ),
        (
            "jbophrase",
            ElementDisposition::Inline {
                probe: "<para><jbophrase>PROBE</jbophrase></para>",
                inline: "LanguageSpan",
            },
        ),
        (
            "letteral",
            ElementDisposition::Flattened {
                reason: "names a letter inline; the book prints the letter itself, which is what the text carries",
            },
        ),
        (
            "link",
            ElementDisposition::Inline {
                probe: r#"<para><link xlink:href="https://example.invalid/" xmlns:xlink="http://www.w3.org/1999/xlink">PROBE</link></para>"#,
                inline: "Link",
            },
        ),
        (
            "listitem",
            ElementDisposition::Structural {
                probe: "<itemizedlist><listitem><para>PROBE</para></listitem></itemizedlist>",
                block: "List",
            },
        ),
        (
            "literallayout",
            ElementDisposition::Block {
                probe: "<literallayout>PROBE</literallayout>",
                block: "Code",
            },
        ),
        (
            "lojbanization",
            ElementDisposition::Block {
                probe: "<lojbanization><jbo>PROBE</jbo><natlang>n</natlang></lojbanization>",
                block: "Lojbanization",
            },
        ),
        (
            "lujvo-making",
            ElementDisposition::Block {
                probe: LUJVO_PROBE,
                block: "LujvoMaking",
            },
        ),
        (
            "mediaobject",
            ElementDisposition::Block {
                probe: MEDIA_PROBE,
                block: "Media",
            },
        ),
        (
            "member",
            ElementDisposition::Structural {
                probe: "<simplelist><member>PROBE</member></simplelist>",
                block: "SimpleListTable",
            },
        ),
        (
            "mfrac",
            ElementDisposition::Structural {
                probe: MATH_PROBE,
                block: "DisplayMath",
            },
        ),
        (
            "mi",
            ElementDisposition::Structural {
                probe: MATH_PROBE,
                block: "DisplayMath",
            },
        ),
        (
            "mmlinlinemath",
            ElementDisposition::Inline {
                probe: "<para><mmlinlinemath><mi>PROBE</mi></mmlinlinemath></para>",
                inline: "InlineMath",
            },
        ),
        (
            "mmlmath",
            ElementDisposition::Inline {
                probe: "<para><mmlmath><mi>PROBE</mi></mmlmath></para>",
                inline: "InlineMath",
            },
        ),
        (
            "mn",
            ElementDisposition::Structural {
                probe: MATH_PROBE,
                block: "DisplayMath",
            },
        ),
        (
            "mo",
            ElementDisposition::Structural {
                probe: MATH_PROBE,
                block: "DisplayMath",
            },
        ),
        (
            "modal-place",
            ElementDisposition::Transparent {
                probe: CMAVO_LIST_PROBE,
                block: "CmavoList",
            },
        ),
        (
            "morphology",
            ElementDisposition::Flattened {
                reason: "names a morphological fragment inline; the book prints the fragment itself, which is what the text carries",
            },
        ),
        (
            "mrow",
            ElementDisposition::Structural {
                probe: MATH_PROBE,
                block: "DisplayMath",
            },
        ),
        (
            "msqrt",
            ElementDisposition::Structural {
                probe: MATH_PROBE,
                block: "DisplayMath",
            },
        ),
        (
            "msup",
            ElementDisposition::Structural {
                probe: MATH_PROBE,
                block: "DisplayMath",
            },
        ),
        (
            "natlang",
            ElementDisposition::Structural {
                probe: GLOSS_PROBE,
                block: "InterlinearGloss",
            },
        ),
        (
            "orderedlist",
            ElementDisposition::Block {
                probe: "<orderedlist><listitem><para>PROBE</para></listitem></orderedlist>",
                block: "List",
            },
        ),
        (
            "para",
            ElementDisposition::Block {
                probe: "<para>PROBE</para>",
                block: "Paragraph",
            },
        ),
        (
            "phrase",
            ElementDisposition::Structural {
                probe: MEDIA_PROBE,
                block: "Media",
            },
        ),
        (
            "primary",
            ElementDisposition::IndexKey {
                probe: "<indexterm><primary>PROBE</primary></indexterm>",
                key: "PROBE",
            },
        ),
        (
            "pronunciation",
            ElementDisposition::Transparent {
                probe: EXAMPLE_PROBE,
                block: "Example",
            },
        ),
        (
            "pseudo-cmavo",
            ElementDisposition::Transparent {
                probe: CMAVO_LIST_PROBE,
                block: "CmavoList",
            },
        ),
        (
            "quote",
            ElementDisposition::Inline {
                probe: "<para><quote>PROBE</quote></para>",
                inline: "Quote",
            },
        ),
        (
            "rafsi",
            ElementDisposition::Inline {
                probe: "<para><rafsi>PROBE</rafsi></para>",
                inline: "Link",
            },
        ),
        (
            "rafsi-group",
            ElementDisposition::Transparent {
                probe: CMAVO_LIST_PROBE,
                block: "CmavoList",
            },
        ),
        (
            "score",
            ElementDisposition::Structural {
                probe: LUJVO_PROBE,
                block: "LujvoMaking",
            },
        ),
        (
            "secondary",
            ElementDisposition::IndexKey {
                probe: "<indexterm><primary>a</primary><secondary>PROBE</secondary></indexterm>",
                key: "a; PROBE",
            },
        ),
        (
            "section",
            ElementDisposition::Division {
                probe: DIVISION_PROBE,
            },
        ),
        (
            "selbri",
            ElementDisposition::Transparent {
                probe: GLOSS_ITEMIZED_PROBE,
                block: "InterlinearGloss",
            },
        ),
        (
            "selmaho",
            ElementDisposition::Transparent {
                probe: CMAVO_LIST_PROBE,
                block: "CmavoList",
            },
        ),
        (
            "series",
            ElementDisposition::Transparent {
                probe: CMAVO_LIST_PROBE,
                block: "CmavoList",
            },
        ),
        (
            "simplelist",
            ElementDisposition::Block {
                probe: "<simplelist><member>PROBE</member></simplelist>",
                block: "SimpleListTable",
            },
        ),
        (
            "subscript",
            ElementDisposition::Inline {
                probe: "<para><subscript>PROBE</subscript></para>",
                inline: "Subscript",
            },
        ),
        (
            "sumti",
            ElementDisposition::Transparent {
                probe: GLOSS_ITEMIZED_PROBE,
                block: "InterlinearGloss",
            },
        ),
        (
            "superscript",
            ElementDisposition::Inline {
                probe: "<para><superscript>PROBE</superscript></para>",
                inline: "Superscript",
            },
        ),
        (
            "table",
            ElementDisposition::Block {
                probe: CAPTION_TABLE_PROBE,
                block: "Table",
            },
        ),
        (
            "tbody",
            ElementDisposition::Structural {
                probe: TABLE_PROBE,
                block: "Table",
            },
        ),
        (
            "td",
            ElementDisposition::Structural {
                probe: TABLE_PROBE,
                block: "Table",
            },
        ),
        (
            "term",
            ElementDisposition::Structural {
                probe: "<variablelist><varlistentry><term>PROBE</term><listitem><para>x</para></listitem></varlistentry></variablelist>",
                block: "VariableList",
            },
        ),
        (
            "tertiary",
            ElementDisposition::IndexKey {
                probe: "<indexterm><primary>a</primary><secondary>b</secondary><tertiary>PROBE</tertiary></indexterm>",
                key: "a; b; PROBE",
            },
        ),
        (
            "textobject",
            ElementDisposition::Transparent {
                probe: MEDIA_PROBE,
                block: "Media",
            },
        ),
        (
            "th",
            ElementDisposition::Structural {
                probe: TABLE_PROBE,
                block: "Table",
            },
        ),
        (
            "thead",
            ElementDisposition::Structural {
                probe: TABLE_PROBE,
                block: "Table",
            },
        ),
        (
            "title",
            ElementDisposition::Division {
                probe: DIVISION_PROBE,
            },
        ),
        (
            "tr",
            ElementDisposition::Structural {
                probe: TABLE_PROBE,
                block: "Table",
            },
        ),
        (
            "valsi",
            ElementDisposition::Inline {
                probe: "<para><valsi>PROBE</valsi></para>",
                inline: "Link",
            },
        ),
        (
            "variablelist",
            ElementDisposition::Block {
                probe: "<variablelist><varlistentry><term>PROBE</term><listitem><para>x</para></listitem></varlistentry></variablelist>",
                block: "VariableList",
            },
        ),
        (
            "varlistentry",
            ElementDisposition::Structural {
                probe: "<variablelist><varlistentry><term>PROBE</term><listitem><para>x</para></listitem></varlistentry></variablelist>",
                block: "VariableList",
            },
        ),
        (
            "veljvo",
            ElementDisposition::Structural {
                probe: LUJVO_PROBE,
                block: "LujvoMaking",
            },
        ),
        (
            "xref",
            ElementDisposition::Inline {
                probe: r#"<para><xref linkend="probe-section"/>PROBE</para>"#,
                inline: "Link",
            },
        ),
    ];

    /// Probes for the block handlers the current edition exercises least: the
    /// nine that are absent from the book altogether - `programlisting` among
    /// them, since colojban 1.3.4 typeset the PEG appendix and took the book's
    /// last program listing with it, while `screen`, `simpara`, `math`, and the
    /// five admonitions have never appeared in it at all - plus
    /// `literallayout`, the one preformatted element the book still uses, which
    /// shares their handler.
    const LEAST_EXERCISED_BLOCK_PROBES: &[(&str, &str, &str)] = &[
        (
            "programlisting",
            "<programlisting>a\n  b</programlisting>",
            "Code",
        ),
        ("screen", "<screen>a\n  b</screen>", "Code"),
        (
            "literallayout",
            "<literallayout>a\n  b</literallayout>",
            "Code",
        ),
        ("simpara", "<simpara>PROBE</simpara>", "Paragraph"),
        (
            "math",
            "<math><mrow><mi>PROBE</mi></mrow></math>",
            "DisplayMath",
        ),
        ("note", "<note><para>PROBE</para></note>", "Paragraph"),
        ("tip", "<tip><para>PROBE</para></tip>", "Paragraph"),
        (
            "warning",
            "<warning><para>PROBE</para></warning>",
            "Paragraph",
        ),
        (
            "important",
            "<important><para>PROBE</para></important>",
            "Paragraph",
        ),
        (
            "caution",
            "<caution><para>PROBE</para></caution>",
            "Paragraph",
        ),
    ];

    /// A payload distinct from the shared `PROBE` text the container probes
    /// use, so an element's own content can be told from its siblings'.
    const UNIQUE_MARKER: &str = "PROBEUNIQUE";

    #[requires(true)]
    #[ensures(true)]
    fn probe_context() -> SectionParseContext {
        SectionParseContext {
            chapter_id: "probe-chapter".to_owned(),
            division: CllDivision::Appendix,
            section_id: "probe-section".to_owned(),
            section_number: None,
            section_title: "Probe".to_owned(),
            source_path: "probe.xml".to_owned(),
        }
    }

    /// Runs `parse_block` over a probe document's root element and returns the
    /// blocks together with any examples it registered, so a probe whose
    /// content lands in an example body can still be inspected.
    #[requires(!xml.is_empty())]
    #[ensures(true)]
    fn probe_blocks(xml: &str) -> (Vec<CllBlock>, Vec<CllExample>) {
        let document = Document::parse(xml).unwrap_or_else(|error| panic!("{xml}: {error}"));
        let mut examples = Vec::new();
        let blocks = parse_block(
            document.root_element(),
            &probe_context(),
            AnchorMode::TopLevel,
            &mut BlockParseState {
                chapter_example_counter: 0,
            },
            &mut examples,
            &mut Vec::new(),
        );
        (blocks, examples)
    }

    #[requires(!xml.is_empty())]
    #[ensures(true)]
    fn probe_inlines(xml: &str) -> Vec<CllInline> {
        let document = Document::parse(xml).unwrap_or_else(|error| panic!("{xml}: {error}"));
        parse_inlines(document.root_element())
    }

    /// The probe with `name`'s own payload text made unique, so that a check
    /// for it proves *this* element's content survived rather than some
    /// sibling's. Returns `None` for an element that carries no text of its own
    /// in the probe - a wrapper, or an attribute-only element - where the
    /// rename differential is the only available evidence.
    #[requires(!name.is_empty())]
    #[ensures(ret.as_ref().is_none_or(|marked| marked.contains(UNIQUE_MARKER)))]
    fn with_unique_payload(probe: &str, name: &str) -> Option<String> {
        let plain = format!("<{name}>PROBE</{name}>");
        probe
            .contains(&plain)
            .then(|| probe.replace(&plain, &format!("<{name}>{UNIQUE_MARKER}</{name}>")))
    }

    /// The probe with `name`'s own tags renamed to an element the importer has
    /// never heard of. If the element really is read by its container, the
    /// container's parsed output must change; if it is not in the probe at all,
    /// or the importer sees straight through it, the two parses are identical
    /// and the declared disposition is wrong.
    #[requires(!name.is_empty())]
    #[ensures(true)]
    fn without_element(probe: &str, name: &str) -> String {
        probe
            .replace(&format!("<{name}>"), "<zz-unhandled>")
            .replace(&format!("</{name}>"), "</zz-unhandled>")
            .replace(&format!("<{name} "), "<zz-unhandled ")
            .replace(&format!("<{name}/>"), "<zz-unhandled/>")
    }

    #[requires(true)]
    #[ensures(true)]
    fn variant_name(rendered: &str) -> String {
        rendered
            .split(|character: char| character == ' ' || character == '(' || character == '{')
            .next()
            .unwrap_or_default()
            .to_owned()
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn vendored_markup_uses_only_the_inventoried_element_vocabulary() {
        let mut found = BTreeSet::new();
        for (source_path, _, compressed) in EMBEDDED_CLL_CHAPTERS {
            let xml = decode_chapter_xml(compressed).expect("embedded chapter should decompress");
            let xml = sanitize_xml_entities(&xml);
            let document = Document::parse(&xml)
                .unwrap_or_else(|error| panic!("{source_path} should parse: {error}"));
            found.extend(
                document
                    .descendants()
                    .filter(Node::is_element)
                    .map(|node| node.tag_name().name().to_owned()),
            );
        }
        let inventoried = VENDORED_ELEMENTS
            .iter()
            .map(|(name, _)| (*name).to_owned())
            .collect::<BTreeSet<_>>();
        assert_eq!(
            inventoried.len(),
            VENDORED_ELEMENTS.len(),
            "the inventory lists each element once"
        );
        assert_eq!(
            found.difference(&inventoried).collect::<Vec<_>>(),
            Vec::<&String>::new(),
            "the vendored sources use markup the importer has never been checked against"
        );
        assert_eq!(
            inventoried.difference(&found).collect::<Vec<_>>(),
            Vec::<&String>::new(),
            "the inventory lists markup the vendored sources no longer use"
        );
    }

    /// The inventory's declared dispositions are checked against the importer,
    /// not merely written down. Without this, a newly vendored tag could be
    /// added to the name list and pass while taking the silent fallthrough the
    /// inventory exists to prevent.
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn every_inventoried_element_is_treated_as_the_inventory_declares() {
        for (name, disposition) in VENDORED_ELEMENTS {
            let is_block = matches!(disposition, ElementDisposition::Block { .. });
            let block_probe = format!("<{name}>PROBE</{name}>");
            let document = Document::parse(&block_probe).expect("bare probe parses");
            assert_eq!(
                is_block_element(document.root_element()),
                is_block,
                "`{name}` is declared {}a block element, and `is_block_element` disagrees",
                if is_block { "" } else { "not " }
            );

            match disposition {
                ElementDisposition::Division { probe } => {
                    let document = Document::parse(probe).expect("division probe parses");
                    let (chapter, sections, ..) =
                        parse_chapter(document.root_element(), CllDivision::Appendix, "probe.xml")
                            .expect("a division probe imports");
                    assert!(
                        format!("{chapter:?}{sections:?}").contains("PROBE"),
                        "`{name}` belongs to the division skeleton, so its text must survive import"
                    );
                }
                ElementDisposition::Block { probe, block } => {
                    let (blocks, examples) = probe_blocks(probe);
                    assert_eq!(
                        blocks
                            .iter()
                            .map(|value| variant_name(&format!("{value:?}")))
                            .collect::<Vec<_>>(),
                        vec![(*block).to_owned()],
                        "`{name}` must import as a single {block} block"
                    );
                    assert!(
                        format!("{blocks:?}{examples:?}").contains("PROBE"),
                        "`{name}`'s content must survive import"
                    );
                }
                ElementDisposition::Inline { probe, inline } => {
                    let inlines = probe_inlines(probe);
                    assert!(
                        inlines
                            .iter()
                            .any(|value| variant_name(&format!("{value:?}")) == *inline),
                        "`{name}` must import as a {inline} inline, got {inlines:?}"
                    );
                }
                ElementDisposition::Consumed { probe } => {
                    let inlines = probe_inlines(probe);
                    assert!(
                        inlines
                            .iter()
                            .all(|value| matches!(value, CllInline::Text(_))),
                        "`{name}` is read for its side effect and contributes no inline of its own, got {inlines:?}"
                    );
                }
                ElementDisposition::IndexKey { probe, key } => {
                    let document = Document::parse(probe).expect("index probe parses");
                    assert_eq!(
                        index_key(document.root_element()).as_deref(),
                        Some(*key),
                        "`{name}` must contribute to the index key"
                    );
                }
                ElementDisposition::Structural { probe, block } => {
                    assert!(
                        probe.contains(&format!("<{name}>"))
                            || probe.contains(&format!("<{name} ")),
                        "`{name}` must appear in its own container probe"
                    );
                    let (blocks, examples) = probe_blocks(probe);
                    assert_eq!(
                        blocks
                            .iter()
                            .map(|value| variant_name(&format!("{value:?}")))
                            .collect::<Vec<_>>(),
                        vec![(*block).to_owned()],
                        "`{name}`'s container must import as a single {block} block"
                    );
                    assert!(
                        format!("{blocks:?}{examples:?}").contains("PROBE"),
                        "`{name}` is read by its container, so its text must survive import"
                    );
                    if let Some(marked) = with_unique_payload(probe, name) {
                        let (marked_blocks, marked_examples) = probe_blocks(&marked);
                        assert!(
                            format!("{marked_blocks:?}{marked_examples:?}").contains(UNIQUE_MARKER),
                            "`{name}`'s own text must reach the import, not just some sibling's"
                        );
                    }
                    let (without, without_examples) = probe_blocks(&without_element(probe, name));
                    assert_ne!(
                        format!("{blocks:?}{examples:?}"),
                        format!("{without:?}{without_examples:?}"),
                        "`{name}` is declared structural, but renaming it away leaves the container's import unchanged, so nothing reads it"
                    );
                }
                ElementDisposition::Transparent { probe, block } => {
                    assert!(
                        probe.contains(&format!("<{name}>"))
                            || probe.contains(&format!("<{name} ")),
                        "`{name}` must appear in its own container probe"
                    );
                    let (blocks, examples) = probe_blocks(probe);
                    assert_eq!(
                        blocks
                            .iter()
                            .map(|value| variant_name(&format!("{value:?}")))
                            .collect::<Vec<_>>(),
                        vec![(*block).to_owned()],
                        "`{name}`'s container must import as a single {block} block"
                    );
                    assert!(
                        format!("{blocks:?}{examples:?}").contains("PROBE"),
                        "`{name}`'s content must survive import even though its name is not consulted"
                    );
                    if let Some(marked) = with_unique_payload(probe, name) {
                        let (marked_blocks, marked_examples) = probe_blocks(&marked);
                        assert!(
                            format!("{marked_blocks:?}{marked_examples:?}").contains(UNIQUE_MARKER),
                            "`{name}`'s own text must reach the import, not just some sibling's"
                        );
                    }
                    let (without, without_examples) = probe_blocks(&without_element(probe, name));
                    assert_eq!(
                        format!("{blocks:?}{examples:?}"),
                        format!("{without:?}{without_examples:?}"),
                        "`{name}` is declared transparent, but renaming it changes what the container imports, so the name is consulted after all"
                    );
                }
                ElementDisposition::Flattened { reason } => {
                    assert!(
                        !reason.is_empty(),
                        "`{name}` is flattened, which has to be a stated decision"
                    );
                    let inlines = probe_inlines(&format!("<para><{name}>PROBE</{name}></para>"));
                    assert!(
                        inlines
                            .iter()
                            .all(|value| matches!(value, CllInline::Text(_))),
                        "`{name}` is declared flattened but the importer gives it a typed inline: {inlines:?}"
                    );
                    assert!(
                        inlines.iter().any(
                            |value| matches!(value, CllInline::Text(text) if text.contains("PROBE"))
                        ),
                        "`{name}` is declared flattened, so its own text is all that survives"
                    );
                }
            }
        }
    }

    /// The importer's block vocabulary is a statement about the input format,
    /// DocBook, not about one edition of one document. Nine of the block
    /// elements it handles are absent from the vendored book - `programlisting`
    /// among them, since colojban 1.3.4 typeset the PEG appendix. Because
    /// unrecognized markup degrades silently (see `VENDORED_ELEMENTS`),
    /// narrowing the vocabulary to whatever the current edition happens to use
    /// would turn any future reintroduction into invisible data loss rather
    /// than an error. Each absent handler is exercised for the block it
    /// produces, not merely for its presence in `is_block_element`, so removing
    /// its specialized `parse_block` arm fails here.
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn absent_block_handlers_still_produce_their_own_blocks() {
        let inventoried = VENDORED_ELEMENTS
            .iter()
            .map(|(name, _)| *name)
            .collect::<BTreeSet<_>>();
        let absent = LEAST_EXERCISED_BLOCK_PROBES
            .iter()
            .filter(|(name, _, _)| !inventoried.contains(name))
            .collect::<Vec<_>>();
        assert_eq!(
            absent.len(),
            9,
            "nine handled block elements are absent from the vendored book in total, `programlisting` among them: {:?}",
            absent.iter().map(|(name, _, _)| *name).collect::<Vec<_>>()
        );
        assert!(
            absent.iter().any(|(name, _, _)| *name == "programlisting"),
            "`programlisting` is one of the nine, not a tenth alongside them"
        );

        for (name, probe, expected) in LEAST_EXERCISED_BLOCK_PROBES {
            let document = Document::parse(probe).expect("probe parses");
            assert!(
                is_block_element(document.root_element()),
                "`{name}` must stay a block element even while the book does not use it"
            );
            let (blocks, _) = probe_blocks(probe);
            assert_eq!(
                blocks
                    .iter()
                    .map(|value| variant_name(&format!("{value:?}")))
                    .collect::<Vec<_>>(),
                vec![(*expected).to_owned()],
                "`{name}` must still import as a {expected} block"
            );
        }

        let (code, _) = probe_blocks("<programlisting>a\n  b</programlisting>");
        assert!(
            matches!(code.as_slice(), [CllBlock::Code { text, .. }] if text == "a\n  b"),
            "a program listing keeps its own line structure: {code:?}"
        );
    }

    /// A chapter's index terms are collected from every non-section child, not
    /// from the nodes that survive block parsing. An `<indexterm>` carries no
    /// visible text, so it is filtered out of the renderable prelude; scanning
    /// only that list would silently lose a term written as a direct child of
    /// the chapter. colojban 1.3.4 nests all of its front-matter terms, so this
    /// shape is not currently in the book - which is exactly why it needs a
    /// test rather than a corpus count.
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn chapter_front_matter_index_terms_survive_wherever_they_are_written() {
        let probe = concat!(
            r#"<article xml:id="probe-chapter"><title>Probe</title>"#,
            "<indexterm><primary>direct child</primary></indexterm>",
            "<para><indexterm><primary>nested in prose</primary></indexterm>Front matter.</para>",
            r#"<section xml:id="probe-first"><title>First</title>"#,
            "<para><indexterm><primary>inside a section</primary></indexterm>Body.</para>",
            "</section></article>",
        );
        let document = Document::parse(probe).expect("probe chapter parses");
        let (chapter, _, _, _, index_entries) =
            parse_chapter(document.root_element(), CllDivision::Appendix, "probe.xml")
                .expect("the probe chapter imports");

        let entries = index_entries
            .iter()
            .map(|entry| (entry.key.as_str(), entry.section_id.as_str()))
            .collect::<BTreeSet<_>>();
        assert!(
            entries.contains(&("direct child", "probe-first")),
            "an index term written as a direct chapter child is indexed at the first section: {entries:?}"
        );
        assert!(
            entries.contains(&("nested in prose", "probe-first")),
            "an index term nested in front-matter prose is indexed at the first section: {entries:?}"
        );
        assert!(
            entries.contains(&("inside a section", "probe-first")),
            "a section's own index terms keep naming that section: {entries:?}"
        );
        assert_eq!(chapter.root_section_ids, ["probe-first"]);

        // The term itself is invisible, so it contributes no rendered block.
        assert!(
            !format!("{:?}", chapter.prelude_blocks).contains("direct child"),
            "an index term is not rendered as prose"
        );
    }

    /// A table writes its row areas either as DocBook `tgroup` children or as
    /// direct `thead`/`tbody` children of the table itself. Both areas have to
    /// be resolved under the same node: resolving `tbody` first and then
    /// searching inside it for `thead` hid every header of the HTML-shaped
    /// tables, which is how four chrestomathy texts and one appendix table lost
    /// their header rows silently.
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn both_table_shapes_import_their_header_and_body_rows() {
        for (shape, probe) in [
            ("direct thead/tbody siblings", TABLE_PROBE),
            ("DocBook tgroup", TGROUP_TABLE_PROBE),
        ] {
            let (blocks, _) = probe_blocks(probe);
            let [
                CllBlock::Table {
                    header_rows,
                    body_rows,
                    ..
                },
            ] = blocks.as_slice()
            else {
                panic!("{shape}: expected one table, got {blocks:?}");
            };
            assert_eq!(header_rows.len(), 1, "{shape}: one header row");
            assert_eq!(body_rows.len(), 1, "{shape}: one body row");
            assert!(
                format!("{header_rows:?}").contains("PROBE"),
                "{shape}: the header row keeps its text"
            );
            assert!(
                format!("{body_rows:?}").contains("PROBE"),
                "{shape}: the body row keeps its text"
            );
        }

        // A table with no header area still imports, with an empty header.
        let (blocks, _) =
            probe_blocks("<informaltable><tbody><tr><td>PROBE</td></tr></tbody></informaltable>");
        let [
            CllBlock::Table {
                header_rows,
                body_rows,
                ..
            },
        ] = blocks.as_slice()
        else {
            panic!("expected one table, got {blocks:?}");
        };
        assert!(header_rows.is_empty());
        assert_eq!(body_rows.len(), 1);
    }
}
