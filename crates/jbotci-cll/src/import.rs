use std::collections::BTreeMap;
use std::io::Read;
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
    pub(crate) section_number: Option<String>,
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
            &cll_numbered_title(division.number_label().as_deref(), &chapter_title),
            &mut anchors,
        );
    }

    if has_sections {
        for child in root.children().filter(Node::is_element) {
            if child.has_tag_name("title") {
                continue;
            }
            if child.has_tag_name("section") {
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
            } else if let Some(block) = parse_standalone_chapter_block(child) {
                prelude_blocks.push(block);
            }
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
    let section_number = division.number_label();
    let context = SectionParseContext {
        chapter_id: chapter_id.to_owned(),
        division,
        section_id: chapter_id.to_owned(),
        section_number: section_number.clone(),
        section_title: chapter_title.to_owned(),
        source_path: source_path.to_owned(),
    };
    let mut examples = Vec::new();
    let mut anchors = Vec::new();
    let index_entries = root
        .descendants()
        .filter(|node| node.is_element() && node.has_tag_name("indexterm"))
        .filter_map(index_key)
        .map(|key| PendingIndexEntry {
            key,
            section_id: chapter_id.to_owned(),
        })
        .collect();
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
            label: cll_numbered_title(section_number.as_deref(), chapter_title),
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
    let section_id =
        xml_id(section_node).unwrap_or_else(|| format!("{chapter_id}-s{section_index}"));
    let section_number = division.section_number(section_index);
    let title_node = child_element(section_node, "title");
    let section_title = title_node
        .map(visible_text)
        .filter(|title| !title.is_empty())
        .unwrap_or_else(|| match &section_number {
            Some(section_number) => format!("Section {section_number}"),
            None => section_id.clone(),
        });
    let context = SectionParseContext {
        chapter_id: chapter_id.to_owned(),
        division,
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
            &cll_numbered_title(section_number.as_deref(), &section_title),
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
    anchors.push((
        section_id.clone(),
        new!(CllAnchor {
            section_id: section_id.clone(),
            label: cll_numbered_title(section_number.as_deref(), &section_title),
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

#[requires(node.is_element())]
#[ensures(true)]
fn parse_standalone_chapter_block(node: Node<'_, '_>) -> Option<CllBlock> {
    if node.has_tag_name("mediaobject") {
        parse_media_block(node)
    } else {
        let text = visible_text(node);
        (!text.is_empty()).then_some(CllBlock::Paragraph {
            anchor_id: xml_id(node),
            role: paragraph_role(node),
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
    let example_number = match context.division.number_label() {
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
            section_number: context.section_number.clone(),
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
        || !(cell.has_tag_name("td") || cell.has_tag_name("th"))
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
        .filter(|cell| {
            cell.is_element()
                && (cell.has_tag_name("entry")
                    || cell.has_tag_name("td")
                    || cell.has_tag_name("th"))
        })
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
