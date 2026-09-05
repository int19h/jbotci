//! Typed compound projection over the collected block tree, before collapse and positioning.

use std::collections::{BTreeMap, HashMap, HashSet};

use super::*;

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GentufaCompoundKind {
    CmavoSequence,
    Zei,
}

#[invariant(::Cmavo { canonical } => !canonical.is_empty())]
#[invariant(::ZeiMember => true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GentufaCompoundExpectation {
    Cmavo { canonical: String },
    ZeiMember,
}

#[invariant(range.byte_start < range.byte_end && range.char_start < range.char_end)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GentufaCompoundMember {
    pub range: WebSourceRange,
    pub expectation: GentufaCompoundExpectation,
}

#[invariant(members.len() >= 2 && columns.get() >= 2 && !lookup_text.is_empty())]
#[invariant(range.byte_start == members[0].range.byte_start && range.byte_end == members[members.len() - 1].range.byte_end)]
#[invariant(range.char_start == members[0].range.char_start && range.char_end == members[members.len() - 1].range.char_end)]
#[expensive_invariant(members.windows(2).all(|pair| pair[0].range.byte_end <= pair[1].range.byte_start))]
#[expensive_invariant(members.iter().all(|member| matches!((kind, member.expectation.as_data()),
    (GentufaCompoundKind::CmavoSequence, data!(GentufaCompoundExpectation::Cmavo { .. })) |
    (GentufaCompoundKind::Zei, data!(GentufaCompoundExpectation::ZeiMember))))) ]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GentufaCompoundSpec {
    pub kind: GentufaCompoundKind,
    pub range: WebSourceRange,
    pub members: Vec<GentufaCompoundMember>,
    pub lookup_text: String,
    pub columns: NonZeroUsize,
}

#[invariant(!block_id.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppliedGentufaCompound {
    pub block_id: String,
    pub spec: GentufaCompoundSpec,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GentufaCompoundNonApplicationReason {
    MissingOrAmbiguousMember,
    RecoveryOrVerbatim,
    OriginMismatch,
    NonConsecutiveMembers,
    IncompleteZei,
    WidthMismatch,
    OverlappingMembers,
    SourceRangeMismatch,
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GentufaCompoundNonApplication {
    pub spec_index: usize,
    pub reason: GentufaCompoundNonApplicationReason,
}

#[expensive_invariant(applied.iter().all(|item| layout.blocks.iter().filter(|block| block.block_id == item.block_id && block.compound_kind == Some(item.spec.kind) && block.col_span == item.spec.columns.get()).count() == 1))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GentufaCompoundLayout<Tooltip = ()> {
    pub layout: GentufaBlocksLayout<Tooltip>,
    pub applied: Vec<AppliedGentufaCompound>,
    pub unapplied: Vec<GentufaCompoundNonApplication>,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CompositeKind {
    Zei,
    Bu,
    Quote,
}

#[invariant(::PlainCmavo { canonical } => !canonical.is_empty())]
#[invariant(::PlainOther => true)]
#[invariant(::CompositeMember { .. } => true)]
#[invariant(::Verbatim => true)]
#[invariant(::Elided => true)]
#[invariant(::Error => true)]
#[invariant(::Compound { .. } => true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum BlockLeafOrigin {
    PlainCmavo {
        canonical: String,
    },
    PlainOther,
    CompositeMember {
        group: RawSyntaxNodeId,
        kind: CompositeKind,
    },
    Verbatim,
    Elided,
    Error,
    Compound {
        kind: GentufaCompoundKind,
    },
}

#[requires(true)]
#[ensures(true)]
pub(super) fn part_compound_kind(part: &BlockLeafPart) -> Option<GentufaCompoundKind> {
    match part.origin.as_data() {
        data!(BlockLeafOrigin::Compound { kind }) => Some(*kind),
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn node_compound_kind(node: &BlockTreeNode) -> Option<GentufaCompoundKind> {
    if node.children.is_empty()
        && let [part] = node.leaf_parts.as_slice()
    {
        part_compound_kind(part)
    } else {
        None
    }
}

#[invariant(!path.is_empty())]
struct IndexedLeaf<'a> {
    part: &'a BlockLeafPart,
    path: Vec<RawSyntaxNodeId>,
}

#[requires(true)]
#[ensures(true)]
fn index_leaves<'a>(
    node: &'a BlockTreeNode,
    path: &mut Vec<RawSyntaxNodeId>,
    leaves: &mut Vec<IndexedLeaf<'a>>,
) {
    path.push(node.id);
    for part in &node.leaf_parts {
        leaves.push(new!(IndexedLeaf {
            part,
            path: path.clone()
        }));
    }
    for child in &node.children {
        index_leaves(child, path, leaves);
    }
    path.pop();
}

#[invariant(true)]
struct PreparedCompound<'a> {
    spec: &'a GentufaCompoundSpec,
    anchor: RawSyntaxNodeId,
    anchor_depth: usize,
    id: RawSyntaxNodeId,
    parts: Vec<BlockLeafPart>,
    node_ids: Vec<RawSyntaxNodeId>,
    node_types: Vec<String>,
    ref_markers: Vec<ReferenceMarker>,
}

#[requires(true)]
#[ensures(ret.1.len() + ret.2.len() == specs.len())]
pub(super) fn rewrite_compounds(
    root: BlockTreeNode,
    source: &str,
    specs: &[GentufaCompoundSpec],
) -> (
    BlockTreeNode,
    Vec<AppliedGentufaCompound>,
    Vec<GentufaCompoundNonApplication>,
) {
    let mut leaves = Vec::new();
    index_leaves(&root, &mut Vec::new(), &mut leaves);
    leaves.sort_by_key(|leaf| (leaf.part.range.byte_start, leaf.part.role.sort_key()));
    let normal: Vec<_> = leaves
        .iter()
        .filter(|leaf| !leaf.part.role.is_elided())
        .collect();
    let mut by_range: BTreeMap<(usize, usize), Vec<usize>> = BTreeMap::new();
    let mut zei_group_lengths = HashMap::<RawSyntaxNodeId, usize>::new();
    for (index, leaf) in normal.iter().enumerate() {
        by_range
            .entry((leaf.part.range.byte_start, leaf.part.range.byte_end))
            .or_default()
            .push(index);
        if let data!(BlockLeafOrigin::CompositeMember {
            group,
            kind: CompositeKind::Zei
        }) = leaf.part.origin.as_data()
        {
            *zei_group_lengths.entry(*group).or_default() += 1;
        }
    }
    let mut owners = HashMap::new();
    let mut groups = Vec::new();
    let mut applied = Vec::new();
    let mut unapplied = Vec::new();
    for (spec_index, spec) in specs.iter().enumerate() {
        match prepare_compound(
            spec,
            source,
            &normal,
            &by_range,
            &zei_group_lengths,
            &owners,
        ) {
            Ok(indices) => {
                let first = normal[indices[0]];
                let mut common_len = first.path.len();
                for &index in &indices[1..] {
                    common_len = first.path[..common_len]
                        .iter()
                        .zip(&normal[index].path)
                        .take_while(|(left, right)| left == right)
                        .count();
                }
                for index in indices {
                    owners.insert(normal[index].part.id, groups.len());
                }
                groups.push(PreparedCompound {
                    spec,
                    anchor: first.path[common_len - 1],
                    anchor_depth: common_len - 1,
                    id: first.part.id,
                    parts: Vec::new(),
                    node_ids: vec![first.part.id],
                    node_types: Vec::new(),
                    ref_markers: Vec::new(),
                });
                applied.push(new!(AppliedGentufaCompound {
                    block_id: format!("n{}", first.part.id.0),
                    spec: spec.clone()
                }));
            }
            Err(reason) => unapplied.push(GentufaCompoundNonApplication { spec_index, reason }),
        }
    }
    // The borrowed index is gone before the tree is consumed. Only stable IDs survive.
    drop(by_range);
    drop(normal);
    drop(leaves);
    let mut anchors: HashMap<RawSyntaxNodeId, Vec<usize>> = HashMap::new();
    for (index, group) in groups.iter().enumerate() {
        anchors.entry(group.anchor).or_default().push(index);
    }
    let mut shared_donors = HashMap::new();
    let root = rewrite_node(
        root,
        source,
        &owners,
        &anchors,
        &mut groups,
        &mut shared_donors,
    )
    .into_data()
    .node
    .expect("the root is an insertion anchor or retains non-members");
    assert!(
        shared_donors.is_empty(),
        "every shared donor reached its surviving insertion ancestor"
    );
    (root, applied, unapplied)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|indices| indices.len() == spec.members.len()) || ret.is_err())]
fn prepare_compound(
    spec: &GentufaCompoundSpec,
    source: &str,
    leaves: &[&IndexedLeaf<'_>],
    by_range: &BTreeMap<(usize, usize), Vec<usize>>,
    zei_group_lengths: &HashMap<RawSyntaxNodeId, usize>,
    owners: &HashMap<RawSyntaxNodeId, usize>,
) -> Result<Vec<usize>, GentufaCompoundNonApplicationReason> {
    use GentufaCompoundNonApplicationReason as Reason;
    if source
        .get(spec.range.byte_start..spec.range.byte_end)
        .is_none()
    {
        return Err(Reason::SourceRangeMismatch);
    }
    let mut indices = Vec::with_capacity(spec.members.len());
    let mut zei_group = None;
    for member in &spec.members {
        let Some(indices_at_range) =
            by_range.get(&(member.range.byte_start, member.range.byte_end))
        else {
            return Err(Reason::MissingOrAmbiguousMember);
        };
        let [index] = indices_at_range.as_slice() else {
            return Err(Reason::MissingOrAmbiguousMember);
        };
        let part = leaves[*index].part;
        if part.range != member.range {
            return Err(Reason::SourceRangeMismatch);
        }
        if !part.role.is_normal()
            || matches!(part.origin.as_data(), data!(BlockLeafOrigin::Verbatim))
        {
            return Err(Reason::RecoveryOrVerbatim);
        }
        if owners.contains_key(&part.id) {
            return Err(Reason::OverlappingMembers);
        }
        match (member.expectation.as_data(), part.origin.as_data()) {
            (
                data!(GentufaCompoundExpectation::Cmavo {
                    canonical: expected
                }),
                data!(BlockLeafOrigin::PlainCmavo { canonical }),
            ) if expected == canonical => {}
            (
                data!(GentufaCompoundExpectation::ZeiMember),
                data!(BlockLeafOrigin::CompositeMember {
                    group,
                    kind: CompositeKind::Zei
                }),
            ) => {
                if zei_group.is_some_and(|previous| previous != *group) {
                    return Err(Reason::IncompleteZei);
                }
                zei_group = Some(*group);
            }
            _ => return Err(Reason::OriginMismatch),
        }
        if indices
            .last()
            .is_some_and(|previous| *previous + 1 != *index)
        {
            return Err(Reason::NonConsecutiveMembers);
        }
        indices.push(*index);
    }
    if let Some(group) = zei_group
        && zei_group_lengths.get(&group) != Some(&indices.len())
    {
        return Err(Reason::IncompleteZei);
    }
    if indices
        .iter()
        .map(|index| leaves[*index].part.columns.get())
        .sum::<usize>()
        != spec.columns.get()
    {
        return Err(Reason::WidthMismatch);
    }
    Ok(indices)
}

#[invariant(node.is_some() || !removed_groups.is_empty())]
struct RewrittenNode {
    node: Option<BlockTreeNode>,
    removed_groups: HashSet<usize>,
}

/// Consume only emptied donor paths. Surviving ancestors retain their own identities.
#[requires(true)]
#[ensures(ret.node.is_some() || !ret.removed_groups.is_empty())]
fn rewrite_node(
    node: BlockTreeNode,
    source: &str,
    owners: &HashMap<RawSyntaxNodeId, usize>,
    anchors: &HashMap<RawSyntaxNodeId, Vec<usize>>,
    groups: &mut [PreparedCompound<'_>],
    shared_donors: &mut HashMap<RawSyntaxNodeId, Vec<BlockTreeNode>>,
) -> RewrittenNode {
    let mut node = node.into_data();
    let mut touched = HashSet::new();
    node.leaf_parts = node
        .leaf_parts
        .into_iter()
        .filter_map(|part| {
            if let Some(&group) = owners.get(&part.id) {
                groups[group].parts.push(part);
                touched.insert(group);
                None
            } else {
                Some(part)
            }
        })
        .collect();
    let mut children = Vec::with_capacity(node.children.len());
    for child in node.children {
        let rewritten =
            rewrite_node(child, source, owners, anchors, groups, shared_donors).into_data();
        if let Some(child) = rewritten.node {
            children.push(child);
        }
        touched.extend(rewritten.removed_groups);
    }
    node.children = children;
    for &index in anchors.get(&node.id).into_iter().flatten() {
        let group = &mut groups[index];
        group.parts.sort_by_key(|part| part.range.byte_start);
        let display_text = group
            .parts
            .iter()
            .map(|part| part.display_text.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        let token_kind =
            (group.spec.kind == GentufaCompoundKind::CmavoSequence).then_some(WordKind::Cmavo);
        let part = new!(BlockLeafPart {
            id: group.id,
            range: group.spec.range,
            role: GentufaBlockRole::Normal,
            error_index: None,
            token_kind,
            raw_text: source_text_for_range(source, Some(group.spec.range)),
            display_text,
            origin: new!(BlockLeafOrigin::Compound {
                kind: group.spec.kind
            }),
            columns: group.spec.columns
        });
        let compound = generated_block_tree_node_from_parts(
            group.id,
            None,
            std::mem::take(&mut group.node_ids),
            "Compound".to_owned(),
            GentufaBlockRole::Normal,
            None,
            token_kind,
            std::mem::take(&mut group.ref_markers),
            std::mem::take(&mut group.node_types),
            Vec::new(),
            vec![part],
            source,
            None,
        )
        .expect("compound has positive source coverage");
        node.children.push(compound);
    }
    if let Some(donors) = shared_donors.remove(&node.id) {
        for donor in donors {
            let donor = donor.into_data();
            extend_unique_node_ids(&mut node.node_ids, donor.node_ids);
            extend_unique_strings(&mut node.node_types, donor.node_types);
            extend_unique_ref_markers(&mut node.ref_markers, donor.ref_markers);
        }
        node.keep_structural_host = true;
    }
    if node.children.is_empty() && node.leaf_parts.is_empty() && !touched.is_empty() {
        if touched.len() == 1 {
            let group = &mut groups[*touched.iter().next().expect("one donor group")];
            extend_unique_node_ids(&mut group.node_ids, node.node_ids);
            extend_unique_strings(&mut group.node_types, node.node_types);
            extend_unique_ref_markers(&mut group.ref_markers, node.ref_markers);
        } else {
            // Every participating insertion anchor is on this emptied node's
            // ancestor path. The shallowest is their LCA and owns the shared
            // identity; assigning it to any one compound would be arbitrary.
            let target = touched
                .iter()
                .map(|index| &groups[*index])
                .min_by_key(|group| group.anchor_depth)
                .expect("multiple donor groups")
                .anchor;
            shared_donors
                .entry(target)
                .or_default()
                .push(BlockTreeNode::from_data(node));
        }
        return new!(RewrittenNode {
            node: None,
            removed_groups: touched,
        });
    }
    if !touched.is_empty() {
        let summary = generated_block_leaf_summary(&node.children, &node.leaf_parts);
        node.leaf_word = summary.leaf_word.map(str::to_owned);
        node.token_kind = summary.token_kind;
        node.span = generated_block_source_range(&node.children, &node.leaf_parts);
        node.raw_text = source_text_for_range(source, node.span);
    }
    new!(RewrittenNode {
        node: Some(BlockTreeNode::from_data(node)),
        removed_groups: touched,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[invariant(projected.applied.len() == 1 && projected.applied[0].spec == *spec)]
    struct CompoundFixture {
        bare: GentufaBlocksLayout,
        projected: GentufaCompoundLayout,
        spec: GentufaCompoundSpec,
    }

    #[requires(count >= 2)]
    #[ensures(true)]
    fn fixture(
        source: &str,
        kind: GentufaCompoundKind,
        count: usize,
        show_elided: bool,
    ) -> CompoundFixture {
        let words = segment_words_with_modifiers(source).unwrap();
        let syntax = jbotci_syntax::parse_syntax_tree_generated_model_with_source_and_options(
            &words,
            source,
            &jbotci_syntax::ParseOptions::default(),
        )
        .unwrap();
        let mut spans = Vec::new();
        if kind == GentufaCompoundKind::Zei {
            words[0].source_spans_into(&mut spans);
        } else {
            spans.extend(
                words[..count]
                    .iter()
                    .map(|word| word.bare_word().unwrap().span()),
            );
        }
        let range = range_from_spans(spans.iter().copied()).unwrap();
        let members = spans
            .iter()
            .enumerate()
            .map(|(index, span)| {
                new!(GentufaCompoundMember {
                    range: range_from_span(span),
                    expectation: if kind == GentufaCompoundKind::Zei {
                        new!(GentufaCompoundExpectation::ZeiMember)
                    } else {
                        new!(GentufaCompoundExpectation::Cmavo {
                            canonical: words[index].bare_word().unwrap().canonical_phonemes()
                        })
                    },
                })
            })
            .collect();
        let spec = new!(GentufaCompoundSpec {
            kind,
            range,
            members,
            lookup_text: "test attestation".to_owned(),
            columns: NonZeroUsize::new(count).unwrap()
        });
        let options = GentufaBlockOptions {
            show_elided,
            ..GentufaBlockOptions::default()
        };
        let bare = generated_model_blocks_layout(&syntax, source, &[], &options);
        let projected = generated_model_blocks_layout_with_compounds(
            &syntax,
            source,
            None,
            None,
            &[],
            &options,
            std::slice::from_ref(&spec),
        );
        new!(CompoundFixture {
            bare,
            projected,
            spec,
        })
    }

    #[requires(true)]
    #[ensures(true)]
    fn assert_grid(layout: &GentufaBlocksLayout) {
        for row in 0..layout.max_row {
            for col in 0..layout.max_col {
                let covering = layout
                    .blocks
                    .iter()
                    .filter(|block| {
                        block.row <= row
                            && row < block.row + block.row_span
                            && block.col <= col
                            && col < block.col + block.col_span
                    })
                    .count();
                assert_eq!(
                    covering, 1,
                    "cell ({row}, {col}) must be covered exactly once: {layout:#?}"
                );
            }
        }
        for block in layout.blocks.iter().filter(|block| block.is_leaf) {
            assert_eq!(block.row + block.row_span, layout.max_row);
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn pre_collapse_groups_keep_columns_across_depths_and_transparent_elision() {
        for (source, kind, count, label) in [
            (
                "la pa da cu klama",
                GentufaCompoundKind::CmavoSequence,
                3,
                "la pa da",
            ),
            (
                "bapuba klama",
                GentufaCompoundKind::CmavoSequence,
                2,
                "ba pu",
            ),
            (
                "batke zei uidje",
                GentufaCompoundKind::Zei,
                3,
                "batke zei uidje",
            ),
            (
                "batke zei uidje zei klama",
                GentufaCompoundKind::Zei,
                5,
                "batke zei uidje zei klama",
            ),
            (
                "denpa bu zei sance",
                GentufaCompoundKind::Zei,
                4,
                "denpa bu zei sance",
            ),
        ] {
            for show_elided in [false, true] {
                let fixture = fixture(source, kind, count, show_elided).into_data();
                assert!(
                    fixture.projected.unapplied.is_empty(),
                    "{source}: {:?}",
                    fixture.projected.unapplied
                );
                assert_eq!(fixture.projected.applied.len(), 1);
                let layout = &fixture.projected.layout;
                assert_eq!(layout.max_col, fixture.bare.max_col);
                let compounds: Vec<_> = layout
                    .blocks
                    .iter()
                    .filter(|block| block.compound_kind.is_some())
                    .collect();
                assert_eq!(compounds.len(), 1);
                let mut original_members: Vec<_> = fixture
                    .bare
                    .blocks
                    .iter()
                    .filter(|block| {
                        block.is_leaf
                            && block.role.is_normal()
                            && block.span.is_some_and(|range| {
                                fixture
                                    .spec
                                    .members
                                    .iter()
                                    .any(|member| member.range == range)
                            })
                    })
                    .collect();
                original_members.sort_by_key(|block| block.col);
                assert_eq!(
                    compounds[0].display_text,
                    original_members
                        .iter()
                        .map(|block| block.display_text.as_str())
                        .collect::<Vec<_>>()
                        .join(" ")
                );
                assert_eq!(
                    jbotci_morphology::canonicalize_text(&compounds[0].display_text),
                    label
                );
                assert_eq!(compounds[0].span, Some(fixture.spec.range));
                assert_eq!(compounds[0].col_span, count);
                assert!(compounds[0].is_leaf);
                assert_eq!(
                    compounds[0].token_kind,
                    (kind == GentufaCompoundKind::CmavoSequence).then_some(WordKind::Cmavo)
                );
                assert_grid(layout);
                if source.starts_with("la") && show_elided {
                    let mut leaves: Vec<_> =
                        layout.blocks.iter().filter(|block| block.is_leaf).collect();
                    leaves.sort_by_key(|block| block.col);
                    // The renderer adds the star from the typed Elided role.
                    assert_eq!(
                        leaves[..3]
                            .iter()
                            .map(|block| block.display_text.as_str())
                            .collect::<Vec<_>>(),
                        ["la pa da", "boi", "ku"]
                    );
                    assert!(leaves[1].role.is_elided() && leaves[2].role.is_elided());
                }
            }
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn invalid_specs_leave_the_ordinary_projection_identical() {
        let source = "ba pu klama";
        let fixture = fixture(source, GentufaCompoundKind::CmavoSequence, 2, false).into_data();
        let words = segment_words_with_modifiers(source).unwrap();
        let syntax = jbotci_syntax::parse_syntax_tree_generated_model_with_source_and_options(
            &words,
            source,
            &jbotci_syntax::ParseOptions::default(),
        )
        .unwrap();
        let invalid = fixture
            .spec
            .with_data(data! {columns: NonZeroUsize::new(3).unwrap()});
        let result = generated_model_blocks_layout_with_compounds(
            &syntax,
            source,
            None,
            None,
            &[],
            &GentufaBlockOptions::default(),
            &[invalid],
        );
        assert_eq!(result.layout, fixture.bare);
        assert!(result.applied.is_empty());
        assert_eq!(
            result.unapplied[0].reason,
            GentufaCompoundNonApplicationReason::WidthMismatch
        );
        let empty = generated_model_blocks_layout_with_compounds(
            &syntax,
            source,
            None,
            None,
            &[],
            &GentufaBlockOptions::default(),
            &[],
        );
        assert_eq!(empty.layout, fixture.bare);
    }

    #[requires(words.len() >= 2)]
    #[ensures(ret.members.len() == words.len())]
    fn cmavo_spec(words: &[WordLike]) -> GentufaCompoundSpec {
        let members = words
            .iter()
            .map(|word| {
                let word = word.bare_word().expect("plain fixture cmavo");
                new!(GentufaCompoundMember {
                    range: range_from_span(word.span()),
                    expectation: new!(GentufaCompoundExpectation::Cmavo {
                        canonical: word.canonical_phonemes()
                    })
                })
            })
            .collect::<Vec<_>>();
        let first = members.first().unwrap().range;
        let last = members.last().unwrap().range;
        new!(GentufaCompoundSpec {
            kind: GentufaCompoundKind::CmavoSequence,
            range: new!(WebSourceRange {
                byte_start: first.byte_start,
                byte_end: last.byte_end,
                char_start: first.char_start,
                char_end: last.char_end
            }),
            members,
            lookup_text: "test attestation".to_owned(),
            columns: NonZeroUsize::new(words.len()).unwrap(),
        })
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn straddling_compound_preserves_the_donor_nodes_remaining_word() {
        let source = "mi re pa moi";
        let words = segment_words_with_modifiers(source).unwrap();
        let syntax = jbotci_syntax::parse_syntax_tree_generated_model_with_source_and_options(
            &words,
            source,
            &jbotci_syntax::ParseOptions::default(),
        )
        .unwrap();
        let spec = cmavo_spec(&words[2..]);
        for show_elided in [false, true] {
            let options = GentufaBlockOptions {
                show_elided,
                ..GentufaBlockOptions::default()
            };
            let bare = generated_model_blocks_layout::<()>(&syntax, source, &[], &options);
            let projected = generated_model_blocks_layout_with_compounds::<()>(
                &syntax,
                source,
                None,
                None,
                &[],
                &options,
                std::slice::from_ref(&spec),
            );
            assert!(projected.unapplied.is_empty());
            assert_eq!(projected.applied.len(), 1);
            assert_eq!(projected.layout.max_col, bare.max_col);
            let retained = projected
                .layout
                .blocks
                .iter()
                .filter(|block| block.is_leaf && block.raw_text == "re")
                .collect::<Vec<_>>();
            assert_eq!(retained.len(), 1, "the donor must keep its remaining word");
            assert_eq!(retained[0].display_text, "re");
            assert_eq!(retained[0].token_kind, Some(WordKind::Cmavo));
            assert_eq!(retained[0].compound_kind, None);
            assert_eq!(retained[0].col_span, 1);
            assert_eq!(
                retained[0].span,
                Some(range_from_span(words[1].bare_word().unwrap().span()))
            );
            assert_grid(&projected.layout);
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn recovery_members_are_rejected_without_changing_the_recovered_projection() {
        let source = "mi ku i do";
        let words = segment_words_with_modifiers(source).unwrap();
        let recovered = jbotci_syntax::parse_syntax_tree_recovered_with_source_and_options(
            &words,
            source,
            &jbotci_syntax::ParseOptions::default(),
        );
        assert_eq!(recovered.errors.len(), 1);
        let options = GentufaBlockOptions::default();
        let bare = recovered_generated_model_blocks_layout::<()>(
            recovered.parse_tree.as_ref(),
            source,
            recovered.errors.len(),
            &[],
            &options,
        );
        let result = recovered_generated_model_blocks_layout_with_compounds::<()>(
            recovered.parse_tree.as_ref(),
            source,
            recovered.errors.len(),
            &[],
            &options,
            &[cmavo_spec(&words[..2])],
        );
        assert_eq!(result.layout, bare);
        assert!(result.applied.is_empty());
        assert_eq!(
            result.unapplied[0].reason,
            GentufaCompoundNonApplicationReason::RecoveryOrVerbatim
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn partial_zei_and_nonconsecutive_cmavo_specs_cannot_absorb_other_terminals() {
        for (source, kind, count) in [
            ("batke zei uidje zei klama", GentufaCompoundKind::Zei, 5),
            ("ba pu ba klama", GentufaCompoundKind::CmavoSequence, 3),
        ] {
            let fixture = fixture(source, kind, count, false).into_data();
            let words = segment_words_with_modifiers(source).unwrap();
            let syntax = jbotci_syntax::parse_syntax_tree_generated_model_with_source_and_options(
                &words,
                source,
                &jbotci_syntax::ParseOptions::default(),
            )
            .unwrap();
            let mut spec = fixture.spec.into_data();
            let expected = if kind == GentufaCompoundKind::Zei {
                spec.members.truncate(3);
                let end = spec.members.last().unwrap().range;
                spec.range = spec
                    .range
                    .with_data(data! {byte_end: end.byte_end, char_end: end.char_end});
                GentufaCompoundNonApplicationReason::IncompleteZei
            } else {
                spec.members.remove(1);
                GentufaCompoundNonApplicationReason::NonConsecutiveMembers
            };
            spec.columns = NonZeroUsize::new(spec.members.len()).unwrap();
            let result = generated_model_blocks_layout_with_compounds::<()>(
                &syntax,
                source,
                None,
                None,
                &[],
                &GentufaBlockOptions::default(),
                &[GentufaCompoundSpec::from_data(spec)],
            );
            assert_eq!(result.layout, fixture.bare);
            assert!(result.applied.is_empty());
            assert_eq!(result.unapplied[0].reason, expected);
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn disjoint_groups_preserve_all_structural_identities() {
        for (source, split, end) in [
            ("pu ba pu ba klama", 2, 4),
            ("la pa da ca klama", 2, 4),
            ("la pa da la re de cu klama", 2, 6),
        ] {
            for show_elided in [false, true] {
                let words = segment_words_with_modifiers(source).unwrap();
                let syntax =
                    jbotci_syntax::parse_syntax_tree_generated_model_with_source_and_options(
                        &words,
                        source,
                        &jbotci_syntax::ParseOptions::default(),
                    )
                    .unwrap();
                let options = GentufaBlockOptions {
                    show_elided,
                    ..GentufaBlockOptions::default()
                };
                let bare = generated_model_blocks_layout::<()>(&syntax, source, &[], &options);
                let result = generated_model_blocks_layout_with_compounds::<()>(
                    &syntax,
                    source,
                    None,
                    None,
                    &[],
                    &options,
                    &[cmavo_spec(&words[..split]), cmavo_spec(&words[split..end])],
                );
                assert!(
                    result.unapplied.is_empty(),
                    "{source}: {:?}",
                    result.unapplied
                );
                assert_eq!(result.applied.len(), 2);
                assert_eq!(
                    result.layout.max_col, bare.max_col,
                    "source width for {source}"
                );
                assert_grid(&result.layout);
                let before = bare
                    .blocks
                    .iter()
                    .flat_map(|block| &block.node_ids)
                    .collect::<HashSet<_>>();
                let after = result
                    .layout
                    .blocks
                    .iter()
                    .flat_map(|block| &block.node_ids)
                    .collect::<HashSet<_>>();
                assert!(
                    before.is_subset(&after),
                    "lost identities for {source}: {:?}",
                    before.difference(&after).collect::<Vec<_>>()
                );
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn mark_shared_donors(
        node: BlockTreeNode,
        range: WebSourceRange,
        markers: &mut Vec<ReferenceMarker>,
    ) -> BlockTreeNode {
        let mut node = node.into_data();
        if node.span == Some(range) {
            let marker = ReferenceMarker {
                role: ReferenceMarkerRole::Referent,
                kind: ReferenceMarkerKind::Reference,
                label: ReferenceLabel::new("shared", NonZeroUsize::new(node.id.0 + 1), None),
                source: Some(new!(ReferenceMarkerSource::DiscourseEdge {
                    edge: node.id.0,
                    source_node: node.id.0,
                    target_node: node.id.0 + 1,
                    display_word: "shared".to_owned(),
                    lookup_word: "shared".to_owned()
                })),
                tooltip: None,
            };
            markers.push(marker.clone());
            node.ref_markers.push(marker);
        }
        node.children = node
            .children
            .into_iter()
            .map(|child| mark_shared_donors(child, range, markers))
            .collect();
        BlockTreeNode::from_data(node)
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn jointly_emptied_paths_keep_one_shared_structural_reference_host() {
        let source = "la pa da ca klama";
        let words = segment_words_with_modifiers(source).unwrap();
        let syntax = jbotci_syntax::parse_syntax_tree_generated_model_with_source_and_options(
            &words,
            source,
            &jbotci_syntax::ParseOptions::default(),
        )
        .unwrap();
        let shared_range = cmavo_spec(&words[1..3]).range;
        for show_elided in [false, true] {
            let options = GentufaBlockOptions {
                show_elided,
                ..GentufaBlockOptions::default()
            };
            let bare = generated_model_blocks_layout::<()>(&syntax, source, &[], &options);
            let mut collector = GeneratedBlockCollector::<false>::new(source, &options, None, None);
            syntax.visit_in_order(&mut collector);
            let mut markers = Vec::new();
            collector.root = Some(mark_shared_donors(
                collector.root.take().unwrap(),
                shared_range,
                &mut markers,
            ));
            assert!(!markers.is_empty());
            let result = finish_blocks_layout::<(), false>(
                collector,
                &[],
                &[cmavo_spec(&words[..2]), cmavo_spec(&words[2..4])],
            );
            assert!(result.unapplied.is_empty());
            assert_eq!(result.layout.max_col, bare.max_col);
            assert_grid(&result.layout);
            let mut shared_host = None;
            for marker in &markers {
                let hosts = result
                    .layout
                    .blocks
                    .iter()
                    .filter(|block| block.ref_markers.contains(marker))
                    .collect::<Vec<_>>();
                assert_eq!(hosts.len(), 1);
                let host = hosts[0];
                assert!(host.compound_kind.is_none());
                if !show_elided {
                    assert!(!host.is_leaf);
                    assert_eq!(host.col_span, bare.max_col);
                    if let Some(previous) = shared_host {
                        assert_eq!(host.block_id, previous);
                    }
                    shared_host = Some(host.block_id.clone());
                    let data!(ReferenceMarkerSource::DiscourseEdge { source_node, .. }) =
                        marker.source.as_ref().unwrap().as_data()
                    else {
                        unreachable!()
                    };
                    assert_eq!(
                        result
                            .layout
                            .blocks
                            .iter()
                            .filter(|block| block.node_ids.contains(source_node))
                            .count(),
                        1
                    );
                }
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn mark_donors(
        node: BlockTreeNode,
        moved: &mut Vec<ReferenceMarker>,
        retained: &mut Vec<ReferenceMarker>,
    ) -> BlockTreeNode {
        let mut node = node.into_data();
        let role = node
            .leaf_parts
            .iter()
            .find_map(|part| match part.origin.as_data() {
                data!(BlockLeafOrigin::PlainCmavo { canonical }) if canonical == "pa" => {
                    Some(ReferenceMarkerRole::Reference)
                }
                data!(BlockLeafOrigin::PlainCmavo { canonical }) if canonical == "da" => {
                    Some(ReferenceMarkerRole::Referent)
                }
                _ => None,
            });
        let retains_elided = node.leaf_parts.iter().any(|part| part.role.is_elided());
        if role.is_some() || retains_elided {
            let marker = ReferenceMarker {
                role: role.unwrap_or(ReferenceMarkerRole::Reference),
                kind: ReferenceMarkerKind::Reference,
                label: ReferenceLabel::new("test", NonZeroUsize::new(node.id.0 + 1), None),
                source: Some(new!(ReferenceMarkerSource::DiscourseEdge {
                    edge: node.id.0,
                    source_node: node.id.0,
                    target_node: node.id.0 + 1,
                    display_word: "test".to_owned(),
                    lookup_word: "test".to_owned()
                })),
                tooltip: None,
            };
            if role.is_some() {
                moved.push(marker.clone());
            } else {
                retained.push(marker.clone());
            }
            node.ref_markers.push(marker);
        }
        node.children = node
            .children
            .into_iter()
            .map(|child| mark_donors(child, moved, retained))
            .collect();
        BlockTreeNode::from_data(node)
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn member_markers_move_once_and_elided_ancestors_keep_their_own_markers() {
        let source = "la pa da cu klama";
        for show_elided in [false, true] {
            let fixture =
                fixture(source, GentufaCompoundKind::CmavoSequence, 3, show_elided).into_data();
            let words = segment_words_with_modifiers(source).unwrap();
            let syntax = jbotci_syntax::parse_syntax_tree_generated_model_with_source_and_options(
                &words,
                source,
                &jbotci_syntax::ParseOptions::default(),
            )
            .unwrap();
            let options = GentufaBlockOptions {
                show_elided,
                ..GentufaBlockOptions::default()
            };
            let mut collector = GeneratedBlockCollector::<false>::new(source, &options, None, None);
            syntax.visit_in_order(&mut collector);
            let mut moved = Vec::new();
            let mut retained = Vec::new();
            collector.root = Some(mark_donors(
                collector.root.take().unwrap(),
                &mut moved,
                &mut retained,
            ));
            assert_eq!(moved.len(), 2);
            assert_eq!(retained.is_empty(), !show_elided);
            let result = finish_blocks_layout(
                collector,
                &Vec::<GentufaBlockAnnotation>::new(),
                &[fixture.spec],
            );
            assert!(result.unapplied.is_empty());
            let compound = result
                .layout
                .blocks
                .iter()
                .find(|block| block.compound_kind.is_some())
                .unwrap();
            for marker in moved {
                assert!(compound.ref_markers.contains(&marker));
                assert_eq!(
                    result
                        .layout
                        .blocks
                        .iter()
                        .filter(|block| block.ref_markers.contains(&marker))
                        .count(),
                    1
                );
            }
            for marker in retained {
                assert!(!compound.ref_markers.contains(&marker));
                assert_eq!(
                    result
                        .layout
                        .blocks
                        .iter()
                        .filter(|block| block.ref_markers.contains(&marker))
                        .count(),
                    1
                );
            }
        }
    }
}
