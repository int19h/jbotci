//! Renderer for the source-backed syntax tree output format.

#[allow(unused_imports)]
use bityzba::{contract_trait, ensures, invariant, requires};
use jbotci_morphology::{
    Cmavo, Phonemes, TreeNode as MorphologyTreeNode, Word, WordKind, WordLike,
};
use jbotci_semantics::references::{RawSyntaxNodeId, ReferenceAnalysis, SyntaxIndex};
use jbotci_source::SourceSpan;
use jbotci_syntax::ast::{
    AtomRef as SyntaxAtomRef, NodeRef as SyntaxNodeRef, TextSyntax, TreeNode as SyntaxAstTreeNode,
};
use jbotci_syntax::generated_model::{
    self, AtomRef as GeneratedSyntaxAtomRef,
    IStatementConnectionTailSyntax as GeneratedIStatementConnectionTailSyntax,
    NodeRef as GeneratedSyntaxNodeRef, TextSyntax as GeneratedTextSyntax,
    TreeNode as GeneratedSyntaxAstTreeNode,
};
use jbotci_syntax::{
    Token, WithIndicators, elidable_terminator_for_absent_field, tree::WithFreeModifiers,
};
use jbotci_tree::{FieldRef, TreeVisitor};

use crate::references::ReferenceDisplayModel;
use crate::{GlyphStyle, OutputError, TreeRenderOptions};

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
#[invariant(::Primary(..) => true)]
#[invariant(::Labelled(..) => true)]
pub(crate) enum RenderEntry {
    Primary(TreeValue),
    Labelled(&'static str, TreeValue),
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
pub(crate) struct TreeEntry {
    pub(crate) label: Option<&'static str>,
    pub(crate) value: TreeValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
pub(crate) struct TreeNode {
    pub(crate) constructor: &'static str,
    pub(crate) entries: Vec<TreeEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
#[invariant(::Node(..) => true)]
#[invariant(::Collection(..) => true)]
#[invariant(::Syntax { .. } => true)]
#[invariant(::Word => true)]
#[invariant(::Verbatim => true)]
#[invariant(::Text(..) => true)]
#[invariant(::Span => true)]
pub(crate) enum TreeValue {
    Node(TreeNode),
    Collection(Vec<TreeValue>),
    Syntax {
        syntax_ids: Vec<RawSyntaxNodeId>,
        value: Box<TreeValue>,
    },
    Word {
        constructor: &'static str,
        phonemes: String,
        span: Option<(usize, usize)>,
        elided: bool,
    },
    Verbatim {
        text: String,
        span: Option<(usize, usize)>,
    },
    Text(String),
    Span {
        byte_start: usize,
        byte_end: usize,
        char_start: usize,
        char_end: usize,
    },
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()))]
pub(crate) fn pretty_tree_with_options(
    tree: &TextSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> Result<String, OutputError> {
    let reference_analysis = if options.show_refs {
        Some(
            ReferenceAnalysis::analyze(tree)
                .map_err(|error| OutputError::References(error.to_string()))?,
        )
    } else {
        None
    };
    let syntax_index = reference_analysis
        .as_ref()
        .map(|analysis| &analysis.syntax_index);
    let value = collapse_value(syntax_tree_value(tree, source, options, syntax_index));
    let references = reference_analysis
        .as_ref()
        .map(|analysis| ReferenceDisplayModel::new(analysis, &value, source, options));
    Ok(render_tree_value_with_options(
        &value,
        options,
        references.as_ref(),
    ))
}

#[doc(hidden)]
#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()) || ret.is_err())]
pub fn pretty_generated_model_tree_with_options(
    tree: &GeneratedTextSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> Result<String, OutputError> {
    if options.show_refs {
        return Err(OutputError::References(
            "generated-model syntax reference rendering is not wired yet".to_owned(),
        ));
    }
    let value = collapse_value(generated_syntax_tree_value(tree, source, options));
    Ok(render_tree_value_with_options(&value, options, None))
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()) || ret.is_err())]
pub(crate) fn pretty_generated_model_raw_tree_with_options(
    tree: &GeneratedTextSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> Result<String, OutputError> {
    let value = collapse_value(raw_generated_syntax_tree_value(tree, source, options));
    Ok(render_tree_value_with_options(&value, options, None))
}

#[requires(true)]
#[ensures(true)]
pub fn reference_display_model_for_syntax_tree(
    analysis: &ReferenceAnalysis<'_>,
    tree: &TextSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> ReferenceDisplayModel {
    let value = collapse_value(syntax_tree_value(
        tree,
        source,
        options,
        Some(&analysis.syntax_index),
    ));
    ReferenceDisplayModel::new(analysis, &value, source, options)
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn pretty_morphology_tree_with_options(
    words: &[WordLike],
    source: &str,
    options: TreeRenderOptions,
) -> Result<String, OutputError> {
    let value = collapse_value(TreeValue::Collection(
        words
            .iter()
            .map(|word_like| morphology_tree_value(word_like, source, options))
            .collect(),
    ));
    Ok(render_tree_value_with_options(&value, options, None))
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn render_tree_value_with_options(
    value: &TreeValue,
    options: TreeRenderOptions,
    references: Option<&ReferenceDisplayModel>,
) -> String {
    let mut renderer = TreeRenderer {
        color: options.color,
        glyphs: options.glyphs,
        indent_step: options.indent,
        show_spans: options.show_spans,
        references,
        output: String::new(),
    };
    renderer.render_value(&value, 0);
    renderer.output
}

#[requires(true)]
#[ensures(true)]
fn with_indicators_tree_value(
    word: &WithIndicators<WordLike>,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    match word {
        WithIndicators::Plain(word_like) => morphology_tree_value(word_like, source, options),
        WithIndicators::Emphasized {
            bahe,
            extra_bahe,
            word_like,
        } => {
            let mut entries = vec![TreeEntry {
                label: Some("bahe"),
                value: word_tree_value(bahe, source, options),
            }];
            if !extra_bahe.is_empty() {
                entries.push(TreeEntry {
                    label: Some("extra_bahe"),
                    value: TreeValue::Collection(
                        extra_bahe
                            .iter()
                            .map(|bahe| word_tree_value(bahe, source, options))
                            .collect(),
                    ),
                });
            }
            entries.push(TreeEntry {
                label: None,
                value: morphology_tree_value(word_like, source, options),
            });
            TreeValue::Node(TreeNode {
                constructor: "Emphasized",
                entries,
            })
        }
        WithIndicators::WithIndicator {
            base,
            indicator_bahe,
            indicator,
            nai_bahe,
            nai,
        } => {
            let mut entries = vec![
                TreeEntry {
                    label: None,
                    value: with_indicators_tree_value(base, source, options),
                },
                TreeEntry {
                    label: Some("indicator"),
                    value: modified_word_tree_value(indicator_bahe, indicator, source, options),
                },
            ];
            if let Some(nai) = nai {
                entries.push(TreeEntry {
                    label: Some("nai"),
                    value: modified_word_tree_value(nai_bahe, nai, source, options),
                });
            }
            TreeValue::Node(TreeNode {
                constructor: "WithIndicator",
                entries,
            })
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn modified_word_tree_value(
    bahe: &[Word],
    word: &Word,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    if bahe.is_empty() {
        word_tree_value(word, source, options)
    } else {
        let mut entries = vec![TreeEntry {
            label: Some("bahe"),
            value: word_tree_value(&bahe[0], source, options),
        }];
        if bahe.len() > 1 {
            entries.push(TreeEntry {
                label: Some("extra_bahe"),
                value: TreeValue::Collection(
                    bahe[1..]
                        .iter()
                        .map(|bahe| word_tree_value(bahe, source, options))
                        .collect(),
                ),
            });
        }
        entries.push(TreeEntry {
            label: None,
            value: word_tree_value(word, source, options),
        });
        TreeValue::Node(TreeNode {
            constructor: "Emphasized",
            entries,
        })
    }
}

#[requires(true)]
#[ensures(true)]
fn word_tree_value(word: &Word, source: &str, options: TreeRenderOptions) -> TreeValue {
    morphology_tree_value(&WordLike::bare(word.clone()), source, options)
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn morphology_tree_value(
    word_like: &WordLike,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    let mut visitor = MorphologyTreeBuilder::new(source, options);
    word_like.visit_in_order(&mut visitor);
    visitor.finish()
}

#[requires(true)]
#[ensures(true)]
fn syntax_tree_value(
    tree: &TextSyntax,
    source: &str,
    options: TreeRenderOptions,
    syntax_index: Option<&SyntaxIndex<'_>>,
) -> TreeValue {
    let mut visitor =
        SyntaxTreeBuilder::<LegacySyntaxRenderModel>::new(source, options, syntax_index);
    tree.visit_in_order(&mut visitor);
    visitor.finish()
}

#[requires(true)]
#[ensures(true)]
fn generated_syntax_tree_value(
    tree: &GeneratedTextSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    let mut visitor = SyntaxTreeBuilder::<GeneratedSyntaxRenderModel>::new(source, options, None);
    tree.visit_in_order(&mut visitor);
    visitor.finish()
}

#[requires(true)]
#[ensures(true)]
fn raw_generated_syntax_tree_value(
    tree: &GeneratedTextSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    let mut visitor =
        SyntaxTreeBuilder::<RawGeneratedSyntaxRenderModel>::new(source, options, None);
    tree.visit_in_order(&mut visitor);
    visitor.finish()
}

#[contract_trait]
trait SyntaxRenderModel {
    type Node<'tree>: Copy;
    type Atom<'tree>: Copy;

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn constructor_name<'tree>(node: Self::Node<'tree>) -> &'static str;

    #[requires(true)]
    #[ensures(true)]
    fn syntax_id<'tree>(
        node: Self::Node<'tree>,
        syntax_index: Option<&SyntaxIndex<'tree>>,
    ) -> Option<RawSyntaxNodeId>;

    #[requires(true)]
    #[ensures(true)]
    fn atom_tree_value<'tree>(
        atom: Self::Atom<'tree>,
        source: &str,
        options: TreeRenderOptions,
    ) -> TreeValue;

    #[requires(true)]
    #[ensures(true)]
    fn atom_end_position<'tree>(atom: Self::Atom<'tree>) -> Option<RenderedPosition>;

    #[requires(true)]
    #[ensures(true)]
    fn elidable_terminator<'tree>(node: Self::Node<'tree>, field: FieldRef) -> Option<Cmavo>;

    #[requires(true)]
    #[ensures(true)]
    fn custom_node_tree_value<'tree>(
        node: Self::Node<'tree>,
        source: &str,
        options: TreeRenderOptions,
    ) -> Option<TreeValue>;
}

#[invariant(true)]
struct LegacySyntaxRenderModel;

#[contract_trait]
impl SyntaxRenderModel for LegacySyntaxRenderModel {
    type Node<'tree> = SyntaxNodeRef<'tree>;
    type Atom<'tree> = SyntaxAtomRef<'tree>;

    fn constructor_name<'tree>(node: Self::Node<'tree>) -> &'static str {
        node.constructor_name()
    }

    fn syntax_id<'tree>(
        node: Self::Node<'tree>,
        syntax_index: Option<&SyntaxIndex<'tree>>,
    ) -> Option<RawSyntaxNodeId> {
        syntax_index.and_then(|index| index.id_of(node))
    }

    fn atom_tree_value<'tree>(
        atom: Self::Atom<'tree>,
        source: &str,
        options: TreeRenderOptions,
    ) -> TreeValue {
        match atom {
            SyntaxAtomRef::Token(word) => {
                with_indicators_tree_value(word.as_indicators(), source, options)
            }
            SyntaxAtomRef::Word(word) => word_tree_value(word, source, options),
        }
    }

    fn atom_end_position<'tree>(atom: Self::Atom<'tree>) -> Option<RenderedPosition> {
        match atom {
            SyntaxAtomRef::Token(token) => token
                .source_spans()
                .into_iter()
                .last()
                .map(span_end_position),
            SyntaxAtomRef::Word(word) => Some(span_end_position(word.span())),
        }
    }

    fn elidable_terminator<'tree>(node: Self::Node<'tree>, field: FieldRef) -> Option<Cmavo> {
        elidable_terminator_for_absent_field(node, field)
    }

    fn custom_node_tree_value<'tree>(
        _node: Self::Node<'tree>,
        _source: &str,
        _options: TreeRenderOptions,
    ) -> Option<TreeValue> {
        None
    }
}

#[invariant(true)]
struct GeneratedSyntaxRenderModel;

#[contract_trait]
impl SyntaxRenderModel for GeneratedSyntaxRenderModel {
    type Node<'tree> = GeneratedSyntaxNodeRef<'tree>;
    type Atom<'tree> = GeneratedSyntaxAtomRef<'tree>;

    fn constructor_name<'tree>(node: Self::Node<'tree>) -> &'static str {
        match node.constructor_name() {
            "ExplicitXauhaLohoi" | "Regular" => "TextSyntax",
            "INihoParagraph" | "NihoParagraph" | "SimpleParagraph" => "ParagraphSyntax",
            "FollowingParagraphStatement"
            | "IParagraphStatement"
            | "InitialParagraphStatement"
            | "TrailingIjekParagraphStatement" => "ParagraphStatementSyntax",
            "PrenexFragment" => "Prenex",
            "DescriptionHeadConnective"
            | "EkAfterthoughtConnective"
            | "JehiAfterthoughtConnective" => "Afterthought",
            "JekSelbriConnective" | "ParagraphJekConnective" => "Selbri",
            "GihekBridiTailConnective" => "BridiTail",
            "CeheNonLogicalConnective"
            | "JoiNonLogicalConnective"
            | "ParagraphJoiNonLogicalConnective"
            | "VuhuNonLogicalConnective" => "NonLogical",
            "ClosedIntervalConnective"
            | "ParagraphClosedIntervalConnective"
            | "ParagraphSimpleIntervalConnective"
            | "SimpleIntervalConnective" => "Interval",
            "GaForethoughtConnective"
            | "GikForethoughtConnective"
            | "GuhekForethoughtConnective" => "Forethought",
            constructor => constructor,
        }
    }

    fn syntax_id<'tree>(
        _node: Self::Node<'tree>,
        _syntax_index: Option<&SyntaxIndex<'tree>>,
    ) -> Option<RawSyntaxNodeId> {
        None
    }

    fn atom_tree_value<'tree>(
        atom: Self::Atom<'tree>,
        source: &str,
        options: TreeRenderOptions,
    ) -> TreeValue {
        match atom {
            GeneratedSyntaxAtomRef::DescriptionTailElementSyntax(value) => {
                required_legacy_syntax_subtree_value(value, source, options)
            }
            GeneratedSyntaxAtomRef::Indicator(value) => {
                required_legacy_syntax_subtree_value(value, source, options)
            }
            GeneratedSyntaxAtomRef::SumtiTagSyntax(value) => {
                required_legacy_syntax_subtree_value(value, source, options)
            }
            GeneratedSyntaxAtomRef::SumtiWrapperKindSyntax(value) => {
                required_legacy_syntax_subtree_value(value, source, options)
            }
            GeneratedSyntaxAtomRef::Token(word) => {
                with_indicators_tree_value(word.as_indicators(), source, options)
            }
        }
    }

    fn atom_end_position<'tree>(atom: Self::Atom<'tree>) -> Option<RenderedPosition> {
        match atom {
            GeneratedSyntaxAtomRef::DescriptionTailElementSyntax(value) => {
                last_legacy_syntax_subtree_position(value)
            }
            GeneratedSyntaxAtomRef::Indicator(value) => last_legacy_syntax_subtree_position(value),
            GeneratedSyntaxAtomRef::SumtiTagSyntax(value) => {
                last_legacy_syntax_subtree_position(value)
            }
            GeneratedSyntaxAtomRef::SumtiWrapperKindSyntax(value) => {
                last_legacy_syntax_subtree_position(value)
            }
            GeneratedSyntaxAtomRef::Token(token) => token
                .source_spans()
                .into_iter()
                .last()
                .map(span_end_position),
        }
    }

    fn elidable_terminator<'tree>(_node: Self::Node<'tree>, _field: FieldRef) -> Option<Cmavo> {
        None
    }

    fn custom_node_tree_value<'tree>(
        node: Self::Node<'tree>,
        source: &str,
        options: TreeRenderOptions,
    ) -> Option<TreeValue> {
        match node {
            GeneratedSyntaxNodeRef::TextSyntaxRegular(text) => {
                generated_regular_text_tree_value(text, source, options)
            }
            GeneratedSyntaxNodeRef::StatementSyntaxMultipleNaFragment(statement) => Some(
                generated_multiple_na_fragment_tree_value(statement, source, options),
            ),
            GeneratedSyntaxNodeRef::StatementSyntaxSingleNaFragment(statement) => Some(
                generated_single_na_fragment_tree_value(statement, source, options),
            ),
            GeneratedSyntaxNodeRef::StatementSyntaxLinkedSumtiFragment(statement) => Some(
                generated_linked_sumti_fragment_tree_value(statement, source, options),
            ),
            GeneratedSyntaxNodeRef::ParagraphStatementSyntaxTrailingIjekParagraphStatement(
                statement,
            ) => Some(generated_trailing_ijek_paragraph_statement_tree_value(
                statement, source, options,
            )),
            GeneratedSyntaxNodeRef::StatementSyntaxBridiStatement(statement) => Some(
                generated_bridi_statement_tree_value(statement, source, options),
            ),
            GeneratedSyntaxNodeRef::StatementSyntaxIStatementConnection(statement) => Some(
                generated_i_statement_connection_tree_value(statement, source, options),
            ),
            _ => None,
        }
    }
}

#[invariant(true)]
struct RawGeneratedSyntaxRenderModel;

#[contract_trait]
impl SyntaxRenderModel for RawGeneratedSyntaxRenderModel {
    type Node<'tree> = GeneratedSyntaxNodeRef<'tree>;
    type Atom<'tree> = GeneratedSyntaxAtomRef<'tree>;

    fn constructor_name<'tree>(node: Self::Node<'tree>) -> &'static str {
        node.constructor_name()
    }

    fn syntax_id<'tree>(
        _node: Self::Node<'tree>,
        _syntax_index: Option<&SyntaxIndex<'tree>>,
    ) -> Option<RawSyntaxNodeId> {
        None
    }

    fn atom_tree_value<'tree>(
        atom: Self::Atom<'tree>,
        source: &str,
        options: TreeRenderOptions,
    ) -> TreeValue {
        GeneratedSyntaxRenderModel::atom_tree_value(atom, source, options)
    }

    fn atom_end_position<'tree>(atom: Self::Atom<'tree>) -> Option<RenderedPosition> {
        GeneratedSyntaxRenderModel::atom_end_position(atom)
    }

    fn elidable_terminator<'tree>(_node: Self::Node<'tree>, _field: FieldRef) -> Option<Cmavo> {
        None
    }

    fn custom_node_tree_value<'tree>(
        _node: Self::Node<'tree>,
        _source: &str,
        _options: TreeRenderOptions,
    ) -> Option<TreeValue> {
        None
    }
}

#[requires(true)]
#[ensures(true)]
fn legacy_syntax_tuple_value<const N: usize>(values: [Option<TreeValue>; N]) -> TreeValue {
    TreeValue::Collection(values.into_iter().flatten().collect())
}

#[requires(true)]
#[ensures(true)]
fn legacy_syntax_subtree_value<T>(
    value: &T,
    source: &str,
    options: TreeRenderOptions,
) -> Option<TreeValue>
where
    T: SyntaxAstTreeNode,
{
    let mut visitor = SyntaxTreeBuilder::<LegacySyntaxRenderModel>::new(source, options, None);
    value.visit_in_order(&mut visitor);
    visitor.finish_optional()
}

#[requires(true)]
#[ensures(true)]
fn generated_syntax_subtree_value<T>(
    value: &T,
    source: &str,
    options: TreeRenderOptions,
) -> Option<TreeValue>
where
    T: GeneratedSyntaxAstTreeNode,
{
    let mut visitor = SyntaxTreeBuilder::<GeneratedSyntaxRenderModel>::new(source, options, None);
    value.visit_in_order(&mut visitor);
    visitor.finish_optional()
}

#[requires(true)]
#[ensures(true)]
fn raw_generated_syntax_subtree_value<T>(
    value: &T,
    source: &str,
    options: TreeRenderOptions,
) -> Option<TreeValue>
where
    T: GeneratedSyntaxAstTreeNode,
{
    let mut visitor =
        SyntaxTreeBuilder::<RawGeneratedSyntaxRenderModel>::new(source, options, None);
    value.visit_in_order(&mut visitor);
    visitor.finish_optional()
}

#[requires(true)]
#[ensures(true)]
fn required_legacy_syntax_subtree_value<T>(
    value: &T,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue
where
    T: SyntaxAstTreeNode,
{
    legacy_syntax_subtree_value(value, source, options)
        .expect("legacy syntax atom tree walk produced a root")
}

#[requires(true)]
#[ensures(true)]
fn required_generated_syntax_subtree_value<T>(
    value: &T,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue
where
    T: GeneratedSyntaxAstTreeNode,
{
    generated_syntax_subtree_value(value, source, options)
        .expect("generated syntax atom tree walk produced a root")
}

#[requires(true)]
#[ensures(true)]
fn generated_regular_text_tree_value(
    text: &GeneratedTextSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> Option<TreeValue> {
    let GeneratedTextSyntax::Regular {
        leading_nai,
        leading_cmevla,
        leading_indicators,
        leading_free_modifiers,
        leading_connective,
        leading_i_statements,
        paragraphs,
    } = text
    else {
        return None;
    };
    if leading_i_statements.is_empty() {
        return None;
    }

    let mut entries = Vec::new();
    if let Some(entry) = labelled_tree_collection_entry_from_values(
        "leading_nai",
        leading_nai
            .iter()
            .map(|token| generated_token_tree_value(token, source, options))
            .collect(),
    ) {
        entries.push(entry);
    }
    if let Some(entry) = labelled_tree_collection_entry_from_values(
        "leading_cmevla",
        leading_cmevla
            .iter()
            .map(|token| generated_token_tree_value(token, source, options))
            .collect(),
    ) {
        entries.push(entry);
    }
    if let Some(entry) = labelled_tree_collection_entry_from_values(
        "leading_indicators",
        leading_indicators
            .iter()
            .map(|indicator| required_legacy_syntax_subtree_value(indicator, source, options))
            .collect(),
    ) {
        entries.push(entry);
    }
    if let Some(entry) = labelled_tree_collection_entry_from_values(
        "leading_free_modifiers",
        generated_free_modifier_tree_values(leading_free_modifiers, source, options),
    ) {
        entries.push(entry);
    }
    if let Some(connective) = leading_connective {
        entries.push(TreeEntry {
            label: Some("leading_connective"),
            value: generated_statement_connective_tree_value(connective, source, options),
        });
    }

    let mut paragraph_values = paragraphs
        .iter()
        .map(|paragraph| required_generated_syntax_subtree_value(paragraph, source, options))
        .collect::<Vec<_>>();
    for marker in leading_i_statements.iter().rev() {
        prepend_generated_leading_i_statement_value(marker, &mut paragraph_values, source, options);
    }
    entries.extend(
        paragraph_values
            .into_iter()
            .map(|value| TreeEntry { label: None, value }),
    );

    Some(TreeValue::Node(TreeNode {
        constructor: "Text",
        entries,
    }))
}

#[requires(true)]
#[ensures(true)]
fn labelled_tree_entry_from_values(
    label: &'static str,
    values: Vec<TreeValue>,
) -> Option<TreeEntry> {
    if values.is_empty() {
        return None;
    }
    let value = if values.len() == 1 {
        values.into_iter().next().expect("length checked")
    } else {
        TreeValue::Collection(values)
    };
    Some(TreeEntry {
        label: Some(label),
        value,
    })
}

#[requires(true)]
#[ensures(true)]
fn labelled_tree_collection_entry_from_values(
    label: &'static str,
    values: Vec<TreeValue>,
) -> Option<TreeEntry> {
    if values.is_empty() {
        return None;
    }
    Some(TreeEntry {
        label: Some(label),
        value: TreeValue::Collection(values),
    })
}

#[requires(true)]
#[ensures(true)]
fn generated_free_modifier_tree_values(
    free_modifiers: &[generated_model::FreeModifierSyntax],
    source: &str,
    options: TreeRenderOptions,
) -> Vec<TreeValue> {
    free_modifiers
        .iter()
        .map(|free_modifier| {
            required_generated_syntax_subtree_value(free_modifier, source, options)
        })
        .collect()
}

#[requires(true)]
#[ensures(true)]
fn prepend_generated_leading_i_statement_value(
    marker: &generated_model::LeadingIStatementSyntax,
    paragraph_values: &mut Vec<TreeValue>,
    source: &str,
    options: TreeRenderOptions,
) {
    if paragraph_values.is_empty() {
        paragraph_values.push(generated_paragraph_with_marker_value(
            marker, None, source, options,
        ));
        return;
    }

    let first_paragraph =
        std::mem::replace(&mut paragraph_values[0], TreeValue::Collection(Vec::new()));
    paragraph_values[0] =
        generated_prepend_marker_to_paragraph_value(marker, first_paragraph, source, options);
}

#[requires(true)]
#[ensures(true)]
fn generated_prepend_marker_to_paragraph_value(
    marker: &generated_model::LeadingIStatementSyntax,
    paragraph: TreeValue,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    match paragraph {
        TreeValue::Node(mut node) if node.constructor == "Paragraph" => {
            if generated_paragraph_has_niho(&node) {
                prepend_generated_marker_to_paragraph_node(marker, &mut node, source, options);
                return TreeValue::Node(node);
            }
            let statement_position = node.entries.iter().position(|entry| entry.label.is_none());
            let statement_already_marked = statement_position
                .and_then(|position| node.entries.get(position))
                .is_some_and(|entry| generated_paragraph_statement_value_has_i(&entry.value));
            if statement_already_marked {
                node.entries.insert(
                    statement_position.expect("checked above"),
                    TreeEntry {
                        label: None,
                        value: generated_paragraph_statement_with_marker_value(
                            marker, None, source, options,
                        ),
                    },
                );
                return TreeValue::Node(node);
            }
            let statement_value = statement_position.map(|position| node.entries.remove(position));
            let replacement = generated_paragraph_statement_with_marker_value(
                marker,
                statement_value.map(|entry| entry.value),
                source,
                options,
            );
            match statement_position {
                Some(position) => node.entries.insert(
                    position,
                    TreeEntry {
                        label: None,
                        value: replacement,
                    },
                ),
                None => node.entries.insert(
                    0,
                    TreeEntry {
                        label: None,
                        value: replacement,
                    },
                ),
            }
            TreeValue::Node(node)
        }
        TreeValue::Syntax { syntax_ids, value } => syntax_value(
            syntax_ids,
            generated_prepend_marker_to_paragraph_value(marker, *value, source, options),
        ),
        TreeValue::Collection(mut items) => {
            if items.is_empty() {
                items.push(generated_paragraph_with_marker_value(
                    marker, None, source, options,
                ));
            } else {
                let first_item = items.remove(0);
                items.insert(
                    0,
                    generated_prepend_marker_to_paragraph_value(
                        marker, first_item, source, options,
                    ),
                );
            }
            TreeValue::Collection(items)
        }
        value => generated_paragraph_with_marker_value(marker, Some(value), source, options),
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_paragraph_has_niho(node: &TreeNode) -> bool {
    node.entries.iter().any(|entry| entry.label == Some("niho"))
}

#[requires(true)]
#[ensures(true)]
fn prepend_generated_marker_to_paragraph_node(
    marker: &generated_model::LeadingIStatementSyntax,
    node: &mut TreeNode,
    source: &str,
    options: TreeRenderOptions,
) {
    node.entries.insert(
        0,
        TreeEntry {
            label: Some("i"),
            value: generated_token_tree_value(&marker.i, source, options),
        },
    );
    attach_generated_marker_to_niho_paragraph_statement(marker, node, source, options);
}

#[requires(true)]
#[ensures(true)]
fn attach_generated_marker_to_niho_paragraph_statement(
    marker: &generated_model::LeadingIStatementSyntax,
    node: &mut TreeNode,
    source: &str,
    options: TreeRenderOptions,
) {
    let Some(statement_position) = node.entries.iter().position(|entry| entry.label.is_none())
    else {
        node.entries.push(TreeEntry {
            label: None,
            value: generated_niho_marker_paragraph_statement_value(marker, source, options),
        });
        return;
    };
    let statement = std::mem::replace(
        &mut node.entries[statement_position].value,
        TreeValue::Collection(Vec::new()),
    );
    node.entries[statement_position].value =
        attach_generated_marker_to_paragraph_statement_value(marker, statement, source, options);
}

#[requires(true)]
#[ensures(true)]
fn generated_niho_marker_paragraph_statement_value(
    marker: &generated_model::LeadingIStatementSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    TreeValue::Node(TreeNode {
        constructor: "ParagraphStatement",
        entries: generated_niho_marker_paragraph_statement_entries(marker, source, options),
    })
}

#[requires(true)]
#[ensures(true)]
fn generated_niho_marker_paragraph_statement_entries(
    marker: &generated_model::LeadingIStatementSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> Vec<TreeEntry> {
    let mut entries = Vec::new();
    if let Some(connective) = &marker.connective {
        entries.push(TreeEntry {
            label: Some("connective"),
            value: generated_statement_connective_tree_value(connective, source, options),
        });
    }
    if let Some(entry) = labelled_tree_collection_entry_from_values(
        "free_modifiers",
        generated_free_modifier_tree_values(&marker.free_modifiers, source, options),
    ) {
        entries.push(entry);
    }
    entries
}

#[requires(true)]
#[ensures(true)]
fn attach_generated_marker_to_paragraph_statement_value(
    marker: &generated_model::LeadingIStatementSyntax,
    statement: TreeValue,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    match statement {
        TreeValue::Node(mut node) if node.constructor == "ParagraphStatement" => {
            set_generated_paragraph_statement_connective(marker, &mut node, source, options);
            prepend_generated_paragraph_statement_free_modifiers(
                marker, &mut node, source, options,
            );
            TreeValue::Node(node)
        }
        TreeValue::Syntax { syntax_ids, value } => syntax_value(
            syntax_ids,
            attach_generated_marker_to_paragraph_statement_value(marker, *value, source, options),
        ),
        value => {
            let mut entries =
                generated_niho_marker_paragraph_statement_entries(marker, source, options);
            entries.push(TreeEntry { label: None, value });
            TreeValue::Node(TreeNode {
                constructor: "ParagraphStatement",
                entries,
            })
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn set_generated_paragraph_statement_connective(
    marker: &generated_model::LeadingIStatementSyntax,
    node: &mut TreeNode,
    source: &str,
    options: TreeRenderOptions,
) {
    if let Some(position) = node
        .entries
        .iter()
        .position(|entry| entry.label == Some("connective"))
    {
        node.entries.remove(position);
    }
    let Some(connective) = &marker.connective else {
        return;
    };
    let insertion_index = node
        .entries
        .iter()
        .position(|entry| entry.label == Some("free_modifiers") || entry.label.is_none())
        .unwrap_or(node.entries.len());
    node.entries.insert(
        insertion_index,
        TreeEntry {
            label: Some("connective"),
            value: generated_statement_connective_tree_value(connective, source, options),
        },
    );
}

#[requires(true)]
#[ensures(true)]
fn prepend_generated_paragraph_statement_free_modifiers(
    marker: &generated_model::LeadingIStatementSyntax,
    node: &mut TreeNode,
    source: &str,
    options: TreeRenderOptions,
) {
    let mut marker_free_modifiers =
        generated_free_modifier_tree_values(&marker.free_modifiers, source, options);
    if marker_free_modifiers.is_empty() {
        return;
    }

    if let Some(position) = node
        .entries
        .iter()
        .position(|entry| entry.label == Some("free_modifiers"))
    {
        let existing = node.entries.remove(position).value;
        marker_free_modifiers.extend(tree_value_collection_items(existing));
        node.entries.insert(
            position,
            TreeEntry {
                label: Some("free_modifiers"),
                value: TreeValue::Collection(marker_free_modifiers),
            },
        );
        return;
    }

    let insertion_index = node
        .entries
        .iter()
        .position(|entry| entry.label.is_none())
        .unwrap_or(node.entries.len());
    node.entries.insert(
        insertion_index,
        TreeEntry {
            label: Some("free_modifiers"),
            value: TreeValue::Collection(marker_free_modifiers),
        },
    );
}

#[requires(true)]
#[ensures(true)]
fn tree_value_collection_items(value: TreeValue) -> Vec<TreeValue> {
    match value {
        TreeValue::Collection(items) => items,
        value => vec![value],
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_paragraph_statement_value_has_i(value: &TreeValue) -> bool {
    match value {
        TreeValue::Node(node) if node.constructor == "ParagraphStatement" => {
            node.entries.iter().any(|entry| entry.label == Some("i"))
        }
        TreeValue::Syntax { value, .. } => generated_paragraph_statement_value_has_i(value),
        _ => false,
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_paragraph_with_marker_value(
    marker: &generated_model::LeadingIStatementSyntax,
    statement: Option<TreeValue>,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    TreeValue::Node(TreeNode {
        constructor: "Paragraph",
        entries: vec![TreeEntry {
            label: None,
            value: generated_paragraph_statement_with_marker_value(
                marker, statement, source, options,
            ),
        }],
    })
}

#[requires(true)]
#[ensures(true)]
fn generated_paragraph_statement_with_marker_value(
    marker: &generated_model::LeadingIStatementSyntax,
    statement: Option<TreeValue>,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    let mut entries = generated_leading_i_statement_entries(marker, source, options);
    match statement {
        Some(TreeValue::Node(node)) if node.constructor == "ParagraphStatement" => {
            entries.extend(node.entries);
        }
        Some(TreeValue::Syntax { syntax_ids, value }) => {
            entries.push(TreeEntry {
                label: None,
                value: syntax_value(syntax_ids, *value),
            });
        }
        Some(value) => entries.push(TreeEntry { label: None, value }),
        None => {}
    }
    TreeValue::Node(TreeNode {
        constructor: "ParagraphStatement",
        entries,
    })
}

#[requires(true)]
#[ensures(true)]
fn generated_leading_i_statement_entries(
    marker: &generated_model::LeadingIStatementSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> Vec<TreeEntry> {
    let mut entries = vec![TreeEntry {
        label: Some("i"),
        value: generated_token_tree_value(&marker.i, source, options),
    }];
    if let Some(connective) = &marker.connective {
        entries.push(TreeEntry {
            label: Some("connective"),
            value: generated_statement_connective_tree_value(connective, source, options),
        });
    }
    if let Some(entry) = labelled_tree_collection_entry_from_values(
        "free_modifiers",
        generated_free_modifier_tree_values(&marker.free_modifiers, source, options),
    ) {
        entries.push(entry);
    }
    entries
}

#[requires(true)]
#[ensures(true)]
fn generated_multiple_na_fragment_tree_value(
    statement: &generated_model::StatementSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    let generated_model::StatementSyntax::MultipleNaFragment {
        first_na,
        second_na,
        additional_na,
    } = statement
    else {
        return required_generated_syntax_subtree_value(statement, source, options);
    };
    let mut entries = vec![
        TreeEntry {
            label: None,
            value: generated_token_tree_value(first_na, source, options),
        },
        TreeEntry {
            label: None,
            value: generated_token_tree_value(second_na, source, options),
        },
    ];
    entries.extend(additional_na.iter().map(|token| TreeEntry {
        label: None,
        value: generated_token_tree_value(token, source, options),
    }));
    TreeValue::Node(TreeNode {
        constructor: "Other",
        entries,
    })
}

#[requires(true)]
#[ensures(true)]
fn generated_single_na_fragment_tree_value(
    statement: &generated_model::StatementSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    let generated_model::StatementSyntax::SingleNaFragment { na } = statement else {
        return required_generated_syntax_subtree_value(statement, source, options);
    };
    let mut entries = vec![TreeEntry {
        label: None,
        value: generated_token_tree_value(&na.value, source, options),
    }];
    if let Some(entry) = labelled_tree_collection_entry_from_values(
        "free_modifiers",
        generated_free_modifier_tree_values(&na.free_modifiers, source, options),
    ) {
        entries.push(entry);
    }
    TreeValue::Node(TreeNode {
        constructor: "Other",
        entries,
    })
}

#[requires(true)]
#[ensures(true)]
fn generated_linked_sumti_fragment_tree_value(
    statement: &generated_model::StatementSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    let generated_model::StatementSyntax::LinkedSumtiFragment { linkargs } = statement else {
        return required_generated_syntax_subtree_value(statement, source, options);
    };
    rename_tree_constructor(
        required_generated_syntax_subtree_value(linkargs, source, options),
        "LinkedSumtiList",
        "LinkedSumti",
    )
}

#[requires(!from.is_empty() && !to.is_empty())]
#[ensures(true)]
fn rename_tree_constructor(value: TreeValue, from: &'static str, to: &'static str) -> TreeValue {
    match value {
        TreeValue::Node(mut node) if node.constructor == from => {
            node.constructor = to;
            TreeValue::Node(node)
        }
        TreeValue::Syntax { syntax_ids, value } => {
            syntax_value(syntax_ids, rename_tree_constructor(*value, from, to))
        }
        value => value,
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_trailing_ijek_paragraph_statement_tree_value(
    statement: &generated_model::ParagraphStatementSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    let generated_model::ParagraphStatementSyntax::TrailingIjekParagraphStatement { i, connective } =
        statement
    else {
        return required_generated_syntax_subtree_value(statement, source, options);
    };
    TreeValue::Node(TreeNode {
        constructor: "BridiConnective",
        entries: vec![
            TreeEntry {
                label: Some("i"),
                value: generated_token_tree_value(i, source, options),
            },
            TreeEntry {
                label: Some("connective"),
                value: generated_statement_connective_tree_value(connective, source, options),
            },
        ],
    })
}

#[requires(true)]
#[ensures(true)]
fn generated_bridi_statement_tree_value(
    statement: &generated_model::StatementSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    let generated_model::StatementSyntax::BridiStatement {
        bridi,
        continuations,
    } = statement
    else {
        return required_generated_syntax_subtree_value(statement, source, options);
    };
    let mut value = required_generated_syntax_subtree_value(bridi.as_ref(), source, options);
    for continuation in continuations {
        value = TreeValue::Node(TreeNode {
            constructor: "ExperimentalBridiContinuation",
            entries: vec![
                TreeEntry {
                    label: Some("leading_statement"),
                    value,
                },
                TreeEntry {
                    label: Some("continuation"),
                    value: generated_bridi_statement_continuation_tree_value(
                        continuation,
                        source,
                        options,
                    ),
                },
            ],
        });
    }
    value
}

#[requires(true)]
#[ensures(true)]
fn generated_bridi_statement_continuation_tree_value(
    continuation: &generated_model::BridiStatementContinuationSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    match continuation {
        generated_model::BridiStatementContinuationSyntax::BoGroupedBridiStatementContinuation {
            connective,
            tense_modal,
            bo,
            trailing_subbridi,
        } => {
            let mut entries = vec![TreeEntry {
                label: Some("connective"),
                value: generated_statement_connective_tree_value(connective, source, options),
            }];
            if let Some(tense_modal) = tense_modal {
                entries.push(TreeEntry {
                    label: Some("tense_modal"),
                    value: required_generated_syntax_subtree_value(
                        tense_modal.as_ref(),
                        source,
                        options,
                    ),
                });
            }
            entries.push(TreeEntry {
                label: Some("marker"),
                value: generated_with_free_modifiers_token_tree_value(bo, source, options),
            });
            entries.push(TreeEntry {
                label: Some("trailing_subbridi"),
                value: required_generated_syntax_subtree_value(
                    trailing_subbridi.as_ref(),
                    source,
                    options,
                ),
            });
            TreeValue::Node(TreeNode {
                constructor: "BridiStatementContinuation",
                entries,
            })
        }
        generated_model::BridiStatementContinuationSyntax::KeGroupedBridiStatementContinuation {
            connective,
            tense_modal,
            ke,
            trailing_subbridi,
            kehe,
        } => {
            let mut entries = vec![TreeEntry {
                label: Some("connective"),
                value: generated_statement_connective_tree_value(connective, source, options),
            }];
            if let Some(tense_modal) = tense_modal {
                entries.push(TreeEntry {
                    label: Some("tense_modal"),
                    value: required_generated_syntax_subtree_value(
                        tense_modal.as_ref(),
                        source,
                        options,
                    ),
                });
            }
            let mut marker_entries = vec![TreeEntry {
                label: Some("ke"),
                value: generated_with_free_modifiers_token_tree_value(ke, source, options),
            }];
            if let Some(kehe) = kehe {
                marker_entries.push(TreeEntry {
                    label: Some("kehe"),
                    value: generated_with_free_modifiers_token_tree_value(kehe, source, options),
                });
            }
            entries.push(TreeEntry {
                label: Some("marker"),
                value: TreeValue::Node(TreeNode {
                    constructor: "KeGrouped",
                    entries: marker_entries,
                }),
            });
            entries.push(TreeEntry {
                label: Some("trailing_subbridi"),
                value: required_generated_syntax_subtree_value(
                    trailing_subbridi.as_ref(),
                    source,
                    options,
                ),
            });
            TreeValue::Node(TreeNode {
                constructor: "BridiStatementContinuation",
                entries,
            })
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
struct GeneratedStatementConnectionPart {
    i: TreeValue,
    connective: TreeValue,
    connective_has_bo: bool,
    trailing_statement: TreeValue,
}

#[requires(true)]
#[ensures(true)]
fn generated_i_statement_connection_tree_value(
    statement: &generated_model::StatementSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    let generated_model::StatementSyntax::IStatementConnection {
        leading_statement,
        continuations,
    } = statement
    else {
        return required_generated_syntax_subtree_value(statement, source, options);
    };

    let mut statements = vec![required_generated_syntax_subtree_value(
        leading_statement.as_ref(),
        source,
        options,
    )];
    let mut connectors = Vec::new();
    for continuation in continuations {
        let part = generated_statement_connection_part(continuation, source, options);
        statements.push(part.trailing_statement.clone());
        connectors.push(part);
    }

    let mut right_statement = statements
        .pop()
        .expect("I statement connection has a leading statement");
    let mut pending_non_bo = Vec::new();
    while let Some(connector) = connectors.pop() {
        let left_statement = statements
            .pop()
            .expect("connectors are paired with leading statements");
        if connector.connective_has_bo {
            right_statement = generated_statement_connection_tree_node(
                connector.i,
                connector.connective,
                left_statement,
                right_statement,
            );
        } else {
            pending_non_bo.push(GeneratedStatementConnectionPart {
                trailing_statement: right_statement,
                ..connector
            });
            right_statement = left_statement;
        }
    }

    let mut connected_statement = right_statement;
    for connector in pending_non_bo.into_iter().rev() {
        connected_statement = generated_statement_connection_tree_node(
            connector.i,
            connector.connective,
            connected_statement,
            connector.trailing_statement,
        );
    }
    connected_statement
}

#[requires(true)]
#[ensures(true)]
fn generated_statement_connection_part(
    continuation: &GeneratedIStatementConnectionTailSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> GeneratedStatementConnectionPart {
    match continuation {
        GeneratedIStatementConnectionTailSyntax::Simple {
            i,
            connective,
            trailing_statement,
        } => GeneratedStatementConnectionPart {
            i: generated_token_tree_value(i, source, options),
            connective: generated_statement_connective_tree_value(connective, source, options),
            connective_has_bo: generated_connective_has_bo(connective),
            trailing_statement: required_generated_syntax_subtree_value(
                trailing_statement.as_ref(),
                source,
                options,
            ),
        },
        GeneratedIStatementConnectionTailSyntax::Chained {
            pending,
            i,
            connective,
            trailing_statement,
        } => {
            let first_pending = pending
                .first()
                .expect("chained I statement tails parse pending with many1");
            let mut extra = Vec::new();
            for pending_connective in pending.iter().skip(1) {
                extra.push(generated_token_tree_value(
                    &pending_connective.i,
                    source,
                    options,
                ));
                extra.push(generated_statement_connective_tree_value(
                    &pending_connective.connective,
                    source,
                    options,
                ));
            }
            extra.push(generated_token_tree_value(i, source, options));
            extra.push(generated_statement_connective_tree_value(
                connective, source, options,
            ));
            GeneratedStatementConnectionPart {
                i: generated_token_tree_value(&first_pending.i, source, options),
                connective: generated_connective_tree_value_with_extra_words(
                    &first_pending.connective,
                    extra,
                    source,
                    options,
                ),
                connective_has_bo: generated_connective_has_bo(connective),
                trailing_statement: required_generated_syntax_subtree_value(
                    trailing_statement.as_ref(),
                    source,
                    options,
                ),
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_statement_connection_tree_node(
    i: TreeValue,
    connective: TreeValue,
    leading_statement: TreeValue,
    trailing_statement: TreeValue,
) -> TreeValue {
    TreeValue::Node(TreeNode {
        constructor: "StatementConnection",
        entries: vec![
            TreeEntry {
                label: Some("leading_statement"),
                value: leading_statement,
            },
            TreeEntry {
                label: Some("i"),
                value: i,
            },
            TreeEntry {
                label: Some("connective"),
                value: connective,
            },
            TreeEntry {
                label: Some("trailing_statement"),
                value: trailing_statement,
            },
        ],
    })
}

#[requires(true)]
#[ensures(true)]
fn generated_token_tree_value(
    token: &Token,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    with_indicators_tree_value(token.as_indicators(), source, options)
}

#[requires(true)]
#[ensures(true)]
fn generated_with_free_modifiers_token_tree_value(
    token: &WithFreeModifiers<Token, generated_model::FreeModifierSyntax>,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    let value = generated_token_tree_value(&token.value, source, options);
    if token.free_modifiers.is_empty() {
        return value;
    }

    TreeValue::Node(TreeNode {
        constructor: "WithFreeModifiers",
        entries: vec![
            TreeEntry {
                label: Some("value"),
                value,
            },
            TreeEntry {
                label: Some("free_modifiers"),
                value: TreeValue::Collection(generated_free_modifier_tree_values(
                    &token.free_modifiers,
                    source,
                    options,
                )),
            },
        ],
    })
}

#[requires(true)]
#[ensures(true)]
fn generated_statement_connective_tree_value(
    connective: &generated_model::ConnectiveSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    match connective {
        generated_model::ConnectiveSyntax::IStandardStatementConnective {
            connective,
            tag_bo: Some((tense_modal, bo)),
        } => {
            let mut extra = Vec::new();
            if let Some(tense_modal) = tense_modal {
                extra.extend(generated_tense_modal_word_tree_values(
                    tense_modal.as_ref(),
                    source,
                    options,
                ));
            }
            extra.push(generated_with_free_modifiers_token_tree_value(
                bo, source, options,
            ));
            generated_connective_tree_value_with_extra_words(connective, extra, source, options)
        }
        generated_model::ConnectiveSyntax::IStandardParagraphStatementConnective {
            connective,
            tag_bo: Some((tense_modal, bo)),
        } => {
            let mut extra = Vec::new();
            if let Some(tense_modal) = tense_modal {
                extra.extend(generated_tense_modal_word_tree_values(
                    tense_modal.as_ref(),
                    source,
                    options,
                ));
            }
            extra.push(generated_token_tree_value(bo, source, options));
            generated_connective_tree_value_with_extra_words(connective, extra, source, options)
        }
        generated_model::ConnectiveSyntax::ITagBoParagraphStatementConnective {
            tense_modal,
            bo,
        } => {
            let mut words = Vec::new();
            if let Some(tense_modal) = tense_modal {
                words.extend(generated_tense_modal_word_tree_values(
                    tense_modal.as_ref(),
                    source,
                    options,
                ));
            }
            words.push(generated_token_tree_value(bo, source, options));
            generated_connective_word_node("Selbri", words)
        }
        generated_model::ConnectiveSyntax::ITagBoStatementConnective { tense_modal, bo } => {
            let mut words = Vec::new();
            if let Some(tense_modal) = tense_modal {
                words.extend(generated_tense_modal_word_tree_values(
                    tense_modal.as_ref(),
                    source,
                    options,
                ));
            }
            words.push(generated_with_free_modifiers_token_tree_value(
                bo, source, options,
            ));
            generated_connective_word_node("Selbri", words)
        }
        _ => collapse_value(required_generated_syntax_subtree_value(
            connective, source, options,
        )),
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_tense_modal_word_tree_values(
    tense_modal: &generated_model::TenseModalSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> Vec<TreeValue> {
    let mut visitor = GeneratedSyntaxTokenTreeValueCollector {
        source,
        options,
        values: Vec::new(),
    };
    tense_modal.visit_in_order(&mut visitor);
    visitor.values
}

#[invariant(true)]
struct GeneratedSyntaxTokenTreeValueCollector<'source> {
    source: &'source str,
    options: TreeRenderOptions,
    values: Vec<TreeValue>,
}

impl<'source, 'tree> TreeVisitor<'tree> for GeneratedSyntaxTokenTreeValueCollector<'source> {
    type Node = GeneratedSyntaxNodeRef<'tree>;
    type Atom = GeneratedSyntaxAtomRef<'tree>;

    #[requires(true)]
    #[ensures(true)]
    fn visit_atom(&mut self, atom: Self::Atom) {
        if let GeneratedSyntaxAtomRef::Token(token) = atom {
            self.values
                .push(generated_token_tree_value(token, self.source, self.options));
        }
    }
}

#[requires(!constructor.is_empty())]
#[ensures(true)]
fn generated_connective_word_node(constructor: &'static str, words: Vec<TreeValue>) -> TreeValue {
    TreeValue::Node(TreeNode {
        constructor,
        entries: words
            .into_iter()
            .map(|value| TreeEntry { label: None, value })
            .collect(),
    })
}

#[requires(true)]
#[ensures(true)]
fn generated_connective_tree_value_with_extra_words(
    connective: &generated_model::ConnectiveSyntax,
    extra_words: Vec<TreeValue>,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    let constructor = generated_connective_constructor(connective);
    append_primary_tree_values(
        collapse_value(required_generated_syntax_subtree_value(
            connective, source, options,
        )),
        constructor,
        extra_words,
    )
}

#[requires(!constructor.is_empty())]
#[ensures(true)]
fn append_primary_tree_values(
    value: TreeValue,
    constructor: &'static str,
    extra_values: Vec<TreeValue>,
) -> TreeValue {
    match value {
        TreeValue::Node(mut node) => {
            node.entries.extend(
                extra_values
                    .into_iter()
                    .map(|value| TreeEntry { label: None, value }),
            );
            TreeValue::Node(node)
        }
        TreeValue::Syntax { syntax_ids, value } => syntax_value(
            syntax_ids,
            append_primary_tree_values(*value, constructor, extra_values),
        ),
        TreeValue::Collection(items) => TreeValue::Node(TreeNode {
            constructor,
            entries: items
                .into_iter()
                .chain(extra_values)
                .map(|value| TreeEntry { label: None, value })
                .collect(),
        }),
        value => TreeValue::Node(TreeNode {
            constructor,
            entries: std::iter::once(value)
                .chain(extra_values)
                .map(|value| TreeEntry { label: None, value })
                .collect(),
        }),
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn generated_connective_constructor(
    connective: &generated_model::ConnectiveSyntax,
) -> &'static str {
    match connective {
        generated_model::ConnectiveSyntax::EkAfterthoughtConnective { .. }
        | generated_model::ConnectiveSyntax::JehiAfterthoughtConnective { .. }
        | generated_model::ConnectiveSyntax::DescriptionHeadConnective { .. } => "Afterthought",
        generated_model::ConnectiveSyntax::JekSelbriConnective { .. }
        | generated_model::ConnectiveSyntax::ParagraphJekConnective { .. } => "Selbri",
        generated_model::ConnectiveSyntax::GihekBridiTailConnective { .. } => "BridiTail",
        generated_model::ConnectiveSyntax::CeheNonLogicalConnective { .. }
        | generated_model::ConnectiveSyntax::JoiNonLogicalConnective { .. }
        | generated_model::ConnectiveSyntax::ParagraphJoiNonLogicalConnective { .. }
        | generated_model::ConnectiveSyntax::VuhuNonLogicalConnective { .. } => "NonLogical",
        generated_model::ConnectiveSyntax::ClosedIntervalConnective { .. }
        | generated_model::ConnectiveSyntax::ParagraphClosedIntervalConnective { .. }
        | generated_model::ConnectiveSyntax::ParagraphSimpleIntervalConnective { .. }
        | generated_model::ConnectiveSyntax::SimpleIntervalConnective { .. } => "Interval",
        generated_model::ConnectiveSyntax::GaForethoughtConnective { .. }
        | generated_model::ConnectiveSyntax::GikForethoughtConnective { .. }
        | generated_model::ConnectiveSyntax::GuhekForethoughtConnective { .. } => "Forethought",
        generated_model::ConnectiveSyntax::IStandardParagraphStatementConnective {
            connective,
            ..
        }
        | generated_model::ConnectiveSyntax::IStandardStatementConnective { connective, .. }
        | generated_model::ConnectiveSyntax::JoikJekGiForethoughtConnective {
            connective, ..
        }
        | generated_model::ConnectiveSyntax::RelationConnectiveAsBridiTail { connective } => {
            generated_connective_constructor(connective)
        }
        generated_model::ConnectiveSyntax::ITagBoParagraphStatementConnective { .. }
        | generated_model::ConnectiveSyntax::ITagBoStatementConnective { .. } => "Selbri",
        generated_model::ConnectiveSyntax::JekGiForethoughtConnective { .. }
        | generated_model::ConnectiveSyntax::ModalGiForethoughtConnective { .. }
        | generated_model::ConnectiveSyntax::ZantufaInitialGiForethoughtConnective { .. } => {
            "Forethought"
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_connective_has_bo(connective: &generated_model::ConnectiveSyntax) -> bool {
    match connective {
        generated_model::ConnectiveSyntax::IStandardParagraphStatementConnective {
            connective,
            tag_bo,
        } => tag_bo.is_some() || generated_connective_has_bo(connective),
        generated_model::ConnectiveSyntax::IStandardStatementConnective { connective, tag_bo } => {
            tag_bo.is_some() || generated_connective_has_bo(connective)
        }
        generated_model::ConnectiveSyntax::ITagBoParagraphStatementConnective { .. }
        | generated_model::ConnectiveSyntax::ITagBoStatementConnective { .. } => true,
        generated_model::ConnectiveSyntax::JoikJekGiForethoughtConnective {
            connective,
            bo,
            ..
        } => bo.is_some() || generated_connective_has_bo(connective),
        generated_model::ConnectiveSyntax::JekGiForethoughtConnective { bo, .. }
        | generated_model::ConnectiveSyntax::ModalGiForethoughtConnective { bo, .. }
        | generated_model::ConnectiveSyntax::ZantufaInitialGiForethoughtConnective { bo, .. } => {
            bo.is_some()
        }
        generated_model::ConnectiveSyntax::RelationConnectiveAsBridiTail { connective } => {
            generated_connective_has_bo(connective)
        }
        _ => false,
    }
}

#[requires(true)]
#[ensures(true)]
fn last_legacy_syntax_subtree_position<T>(value: &T) -> Option<RenderedPosition>
where
    T: SyntaxAstTreeNode,
{
    let mut visitor = LastLegacySyntaxAtomVisitor { last: None };
    value.visit_in_order(&mut visitor);
    visitor.last
}

#[invariant(true)]
struct LastLegacySyntaxAtomVisitor {
    last: Option<RenderedPosition>,
}

impl<'tree> TreeVisitor<'tree> for LastLegacySyntaxAtomVisitor {
    type Node = SyntaxNodeRef<'tree>;
    type Atom = SyntaxAtomRef<'tree>;

    #[requires(true)]
    #[ensures(true)]
    fn visit_atom(&mut self, atom: Self::Atom) {
        self.last = LegacySyntaxRenderModel::atom_end_position(atom);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
#[invariant(::Node => true)]
#[invariant(::Field => true)]
#[invariant(::Collection => true)]
enum SyntaxFrame<'tree, M: SyntaxRenderModel> {
    Node {
        node_ref: M::Node<'tree>,
        constructor: &'static str,
        syntax_id: Option<RawSyntaxNodeId>,
        entries: Vec<TreeEntry>,
    },
    Field {
        name: Option<&'static str>,
        primary: bool,
        values: Vec<TreeValue>,
        nested_entries: Vec<TreeEntry>,
    },
    Collection {
        items: Vec<TreeValue>,
    },
}

#[invariant(true)]
struct SyntaxTreeBuilder<'source, 'index, 'tree, M: SyntaxRenderModel> {
    source: &'source str,
    options: TreeRenderOptions,
    syntax_index: Option<&'index SyntaxIndex<'tree>>,
    stack: Vec<SyntaxFrame<'tree, M>>,
    last_position: Option<RenderedPosition>,
    root: Option<TreeValue>,
    _model: std::marker::PhantomData<M>,
}

impl<'source, 'index, 'tree, M> SyntaxTreeBuilder<'source, 'index, 'tree, M>
where
    M: SyntaxRenderModel,
{
    #[requires(true)]
    #[ensures(ret.source == source)]
    fn new(
        source: &'source str,
        options: TreeRenderOptions,
        syntax_index: Option<&'index SyntaxIndex<'tree>>,
    ) -> Self {
        Self {
            source,
            options,
            syntax_index,
            stack: Vec::new(),
            last_position: None,
            root: None,
            _model: std::marker::PhantomData,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn finish(self) -> TreeValue {
        self.root.expect("syntax tree walk produced a root")
    }

    #[requires(true)]
    #[ensures(true)]
    fn finish_optional(self) -> Option<TreeValue> {
        self.root
    }

    #[requires(true)]
    #[ensures(true)]
    fn push_value(&mut self, value: TreeValue) {
        match self.stack.last_mut() {
            Some(SyntaxFrame::Field { values, .. }) => values.push(value),
            Some(SyntaxFrame::Collection { items }) => items.push(value),
            Some(SyntaxFrame::Node { entries, .. }) => {
                entries.push(TreeEntry { label: None, value })
            }
            None => self.root = Some(value),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn push_labelled_entry_to_nearest_node(&mut self, label: &'static str, value: TreeValue) {
        for frame in self.stack.iter_mut().rev() {
            match frame {
                SyntaxFrame::Field { nested_entries, .. } => {
                    nested_entries.push(TreeEntry {
                        label: Some(label),
                        value,
                    });
                    return;
                }
                SyntaxFrame::Node { entries, .. } => {
                    entries.push(TreeEntry {
                        label: Some(label),
                        value,
                    });
                    return;
                }
                SyntaxFrame::Collection { .. } => {}
            }
        }
        panic!("syntax tree labelled field has no containing node");
    }

    #[requires(true)]
    #[ensures(true)]
    fn push_values_in_order(&mut self, values: Vec<TreeValue>) {
        for value in values {
            match value {
                TreeValue::Collection(items) => {
                    for value in items {
                        self.push_value(value);
                    }
                }
                TreeValue::Syntax { syntax_ids, value } => match *value {
                    TreeValue::Collection(items) => {
                        for value in items {
                            self.push_value(syntax_value(syntax_ids.clone(), value));
                        }
                    }
                    value => self.push_value(syntax_value(syntax_ids, value)),
                },
                value => self.push_value(value),
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn push_entries_in_order(&mut self, entries: Vec<TreeEntry>) {
        for entry in entries {
            match entry.label {
                Some(label) => self.push_labelled_entry_to_nearest_node(label, entry.value),
                None => self.push_value(entry.value),
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn push_labelled_field_value(&mut self, label: &'static str, values: Vec<TreeValue>) {
        if values.is_empty() {
            return;
        }
        let value = if values.len() == 1 {
            values.into_iter().next().expect("length checked")
        } else {
            TreeValue::Collection(values)
        };
        self.push_labelled_entry_to_nearest_node(label, value);
    }
}

impl<'source, 'index, 'tree, M> TreeVisitor<'tree> for SyntaxTreeBuilder<'source, 'index, 'tree, M>
where
    M: SyntaxRenderModel,
{
    type Node = M::Node<'tree>;
    type Atom = M::Atom<'tree>;

    #[requires(true)]
    #[ensures(true)]
    fn enter_node(&mut self, node: Self::Node) {
        self.stack.push(SyntaxFrame::Node {
            node_ref: node,
            constructor: syntax_constructor_name(M::constructor_name(node)),
            syntax_id: M::syntax_id(node, self.syntax_index),
            entries: Vec::new(),
        });
    }

    #[requires(true)]
    #[ensures(true)]
    fn exit_node(&mut self, _node: Self::Node) {
        let Some(SyntaxFrame::Node {
            node_ref,
            constructor,
            syntax_id,
            entries,
        }) = self.stack.pop()
        else {
            panic!("syntax tree walker exited a node without entering it");
        };
        let value = M::custom_node_tree_value(node_ref, self.source, self.options).unwrap_or(
            TreeValue::Node(TreeNode {
                constructor,
                entries,
            }),
        );
        self.push_value(match syntax_id {
            Some(id) => syntax_value(vec![id], value),
            None => value,
        });
    }

    #[requires(true)]
    #[ensures(true)]
    fn enter_field(&mut self, field: FieldRef) {
        self.stack.push(SyntaxFrame::Field {
            name: field.name,
            primary: field.primary,
            values: Vec::new(),
            nested_entries: Vec::new(),
        });
    }

    #[requires(true)]
    #[ensures(true)]
    fn exit_field(&mut self, _field: FieldRef) {
        let Some(SyntaxFrame::Field {
            name,
            primary,
            values,
            nested_entries,
        }) = self.stack.pop()
        else {
            panic!("syntax tree walker exited a field without entering it");
        };
        if values.is_empty() && nested_entries.is_empty() {
            return;
        }
        if primary || name.is_none() {
            self.push_values_in_order(values);
        } else {
            self.push_labelled_field_value(name.expect("checked above"), values);
        }
        self.push_entries_in_order(nested_entries);
    }

    #[requires(true)]
    #[ensures(true)]
    fn enter_sequence(&mut self) {
        self.stack
            .push(SyntaxFrame::Collection { items: Vec::new() });
    }

    #[requires(matches!(self.stack.last(), Some(SyntaxFrame::Collection { .. })))]
    #[ensures(true)]
    fn exit_sequence(&mut self) {
        let Some(SyntaxFrame::Collection { items }) = self.stack.pop() else {
            panic!("syntax tree walker exited a collection without entering it");
        };
        if !items.is_empty() {
            self.push_value(TreeValue::Collection(items));
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_atom(&mut self, atom: Self::Atom) {
        self.last_position = M::atom_end_position(atom);
        self.push_value(M::atom_tree_value(atom, self.source, self.options));
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_absent_optional_field(&mut self, field: FieldRef) {
        if !self.options.show_elided {
            return;
        }
        let Some(node) = current_syntax_node(&self.stack) else {
            return;
        };
        let Some(cmavo) = M::elidable_terminator(node, field) else {
            return;
        };
        let Some(position) = self.last_position.clone() else {
            return;
        };
        self.push_value(elided_cmavo_tree_value(cmavo, position, self.options));
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
struct RenderedPosition {
    byte_end: usize,
    char_end: usize,
}

#[requires(true)]
#[ensures(true)]
fn current_syntax_node<'tree, M>(stack: &[SyntaxFrame<'tree, M>]) -> Option<M::Node<'tree>>
where
    M: SyntaxRenderModel,
{
    stack.iter().rev().find_map(|frame| match frame {
        SyntaxFrame::Node { node_ref, .. } => Some(*node_ref),
        SyntaxFrame::Field { .. } | SyntaxFrame::Collection { .. } => None,
    })
}

#[requires(span.byte_start <= span.byte_end)]
#[requires(span.char_start <= span.char_end)]
#[ensures(ret.byte_end == span.byte_end)]
fn span_end_position(span: &SourceSpan) -> RenderedPosition {
    RenderedPosition {
        byte_end: span.byte_end,
        char_end: span.char_end,
    }
}

#[requires(true)]
#[ensures(true)]
fn elided_cmavo_tree_value(
    cmavo: Cmavo,
    position: RenderedPosition,
    options: TreeRenderOptions,
) -> TreeValue {
    TreeValue::Word {
        constructor: "Cmavo",
        phonemes: elided_cmavo_text(cmavo, options.phonemes),
        span: Some((position.char_end, position.char_end)),
        elided: true,
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn elided_cmavo_text(cmavo: Cmavo, options: jbotci_morphology::PhonemeRenderOptions) -> String {
    Phonemes::from_canonical(cmavo.canonical_text().to_owned())
        .expect("cmavo canonical text is valid phoneme text")
        .render(options)
}

#[requires(true)]
#[ensures(!ret.ends_with("Syntax"))]
fn syntax_constructor_name(constructor: &'static str) -> &'static str {
    constructor.strip_suffix("Syntax").unwrap_or(constructor)
}

#[requires(true)]
#[ensures(true)]
fn syntax_value(syntax_ids: Vec<RawSyntaxNodeId>, value: TreeValue) -> TreeValue {
    if syntax_ids.is_empty() {
        return value;
    }
    match value {
        TreeValue::Syntax {
            syntax_ids: mut inner_ids,
            value,
        } => {
            inner_ids.extend(syntax_ids);
            TreeValue::Syntax {
                syntax_ids: inner_ids,
                value,
            }
        }
        value => TreeValue::Syntax {
            syntax_ids,
            value: Box::new(value),
        },
    }
}

#[requires(true)]
#[ensures(true)]
fn collapse_value(value: TreeValue) -> TreeValue {
    match value {
        TreeValue::Node(node) => collapse_node(node),
        TreeValue::Collection(items) => {
            TreeValue::Collection(items.into_iter().map(collapse_value).collect())
        }
        TreeValue::Syntax { syntax_ids, value } => syntax_value(syntax_ids, collapse_value(*value)),
        TreeValue::Word { .. }
        | TreeValue::Verbatim { .. }
        | TreeValue::Text(..)
        | TreeValue::Span { .. } => value,
    }
}

#[requires(true)]
#[ensures(true)]
fn collapse_node(node: TreeNode) -> TreeValue {
    let entries = node
        .entries
        .into_iter()
        .map(|entry| TreeEntry {
            label: entry.label,
            value: collapse_value(entry.value),
        })
        .collect::<Vec<_>>();
    if entries.len() == 1 && entries[0].label.is_none() {
        let mut entries = entries;
        return entries
            .pop()
            .expect("length check guarantees one entry")
            .value;
    }
    TreeValue::Node(TreeNode {
        constructor: node.constructor,
        entries,
    })
}

#[derive(Debug)]
#[invariant(true)]
struct TreeRenderer<'references> {
    color: bool,
    glyphs: GlyphStyle,
    indent_step: usize,
    show_spans: bool,
    references: Option<&'references ReferenceDisplayModel>,
    output: String,
}

impl TreeRenderer<'_> {
    #[requires(true)]
    #[ensures(true)]
    fn render_value(&mut self, value: &TreeValue, indent: usize) {
        match value {
            TreeValue::Node(node) => self.render_node(node, indent),
            TreeValue::Collection(items) => self.render_collection(items, indent),
            TreeValue::Syntax { syntax_ids, value } => {
                self.render_syntax_value(syntax_ids, value, indent)
            }
            TreeValue::Word {
                constructor,
                phonemes,
                span,
                elided,
            } => self.render_word(constructor, phonemes, *span, *elided),
            TreeValue::Verbatim { text, span } => self.render_verbatim(text, *span),
            TreeValue::Text(text) => self.output.push_str(&self.string_literal(text)),
            TreeValue::Span {
                byte_start: _,
                byte_end: _,
                char_start,
                char_end,
            } => self
                .output
                .push_str(&self.span_literal(*char_start, *char_end)),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn render_syntax_value(
        &mut self,
        syntax_ids: &[RawSyntaxNodeId],
        value: &TreeValue,
        indent: usize,
    ) {
        let annotations = self
            .references
            .map(|references| references.annotations_for_syntax_ids(syntax_ids));
        if let Some(annotations) = annotations.as_ref() {
            for name in &annotations.incoming {
                self.output
                    .push_str(&self.reference_name(name, ReferenceRenderRole::Referent));
                self.output
                    .push_str(&self.punctuation_token(self.glyphs.arrow()));
                self.output.push(' ');
            }
        }
        self.render_value(value, indent);
        if let Some(annotations) = annotations.as_ref() {
            for name in &annotations.outgoing {
                self.output.push(' ');
                self.output
                    .push_str(&self.punctuation_token(self.glyphs.arrow()));
                self.output
                    .push_str(&self.reference_name(name, ReferenceRenderRole::Reference));
            }
        }
    }

    #[requires(!constructor.is_empty())]
    #[ensures(true)]
    fn render_word(
        &mut self,
        constructor: &str,
        phonemes: &str,
        span: Option<(usize, usize)>,
        elided: bool,
    ) {
        self.output.push_str(&self.constructor_token(constructor));
        self.render_optional_node_span(span);
        self.output.push(' ');
        if elided {
            self.output.push_str(&self.elided_string_literal(phonemes));
        } else {
            self.output.push_str(&self.string_literal(phonemes));
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn render_verbatim(&mut self, text: &str, span: Option<(usize, usize)>) {
        self.output.push_str(&self.constructor_token("Verbatim"));
        self.render_optional_node_span(span);
        self.output.push(' ');
        self.output.push_str(&self.string_literal(text));
    }

    #[requires(true)]
    #[ensures(true)]
    fn render_node(&mut self, node: &TreeNode, indent: usize) {
        self.output
            .push_str(&self.constructor_token(node.constructor));
        self.render_optional_node_span(tree_node_span(node));
        if self.indent_step != 0 {
            self.output.push(' ');
        }
        self.output.push_str(&self.punctuation_token("{"));
        if node.entries.is_empty() {
            self.output.push_str(&self.punctuation_token("}"));
            return;
        }
        let entries = node.entries.iter().map(render_entry).collect::<Vec<_>>();
        if self.indent_step == 0 {
            self.render_inline_entries(&entries);
        } else {
            self.render_entries(&entries, indent);
            self.output.push('\n');
            self.push_indent(indent);
        }
        self.output.push_str(&self.punctuation_token("}"));
    }

    #[requires(true)]
    #[ensures(true)]
    fn render_entries(&mut self, entries: &[RenderEntry], indent: usize) {
        let child_indent = indent + self.indent_step;
        for entry in entries {
            self.output.push('\n');
            self.push_indent(child_indent);
            match entry {
                RenderEntry::Primary(value) => self.render_value(value, child_indent),
                RenderEntry::Labelled(label, value) => {
                    self.output.push_str(&self.field_token(label));
                    self.output.push_str(&self.punctuation_token(":"));
                    self.output.push(' ');
                    self.render_value(value, child_indent);
                }
            }
            self.output.push_str(&self.punctuation_token(","));
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn render_inline_entries(&mut self, entries: &[RenderEntry]) {
        for (index, entry) in entries.iter().enumerate() {
            if index > 0 {
                self.output.push_str(&self.punctuation_token(","));
            }
            match entry {
                RenderEntry::Primary(value) => self.render_value(value, 0),
                RenderEntry::Labelled(label, value) => {
                    self.output.push_str(&self.field_token(label));
                    self.output.push_str(&self.punctuation_token(":"));
                    self.render_value(value, 0);
                }
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn render_collection(&mut self, items: &[TreeValue], indent: usize) {
        self.output.push_str(&self.punctuation_token("["));
        if items.is_empty() {
            self.output.push_str(&self.punctuation_token("]"));
            return;
        }
        if self.indent_step == 0 {
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    self.output.push_str(&self.punctuation_token(","));
                }
                self.render_value(item, 0);
            }
            self.output.push_str(&self.punctuation_token("]"));
            return;
        }
        let child_indent = indent + self.indent_step;
        for item in items {
            self.output.push('\n');
            self.push_indent(child_indent);
            self.render_value(item, child_indent);
            self.output.push_str(&self.punctuation_token(","));
        }
        self.output.push('\n');
        self.push_indent(indent);
        self.output.push_str(&self.punctuation_token("]"));
    }

    #[requires(true)]
    #[ensures(true)]
    fn push_indent(&mut self, indent: usize) {
        self.output.extend(std::iter::repeat_n(' ', indent));
    }

    #[requires(true)]
    #[ensures(!self.color -> ret.starts_with('"'))]
    fn string_literal(&self, text: &str) -> String {
        let literal = serde_json::to_string(text).expect("serializing string literal cannot fail");
        self.color_token(&literal, ColorRole::String)
    }

    #[requires(true)]
    #[ensures(!self.color -> ret.starts_with('"'))]
    fn elided_string_literal(&self, text: &str) -> String {
        let literal = serde_json::to_string(text).expect("serializing string literal cannot fail");
        self.elided_color_token(&literal, ColorRole::String)
    }

    #[requires(char_start <= char_end)]
    #[ensures(!ret.is_empty())]
    fn span_literal(&self, char_start: usize, char_end: usize) -> String {
        let mut output = String::new();
        output.push_str(&self.punctuation_token("["));
        output.push_str(&self.number_token(char_start));
        output.push_str(&self.punctuation_token(","));
        output.push_str(&self.number_token(char_end));
        output.push_str(&self.punctuation_token("]"));
        output
    }

    #[requires(span.is_none_or(|(start, end)| start <= end))]
    #[ensures(true)]
    fn render_optional_node_span(&mut self, span: Option<(usize, usize)>) {
        if !self.show_spans {
            return;
        }
        if let Some((char_start, char_end)) = span {
            self.output.push(' ');
            self.output
                .push_str(&self.span_marker(char_start, char_end));
        }
    }

    #[requires(char_start <= char_end)]
    #[ensures(!ret.is_empty())]
    fn span_marker(&self, char_start: usize, char_end: usize) -> String {
        let mut output = String::new();
        output.push_str(&self.punctuation_token("@"));
        output.push_str(&self.punctuation_token("["));
        output.push_str(&self.span_number_token(char_start));
        output.push_str(&self.punctuation_token(self.glyphs.span_leader()));
        output.push_str(&self.span_number_token(char_end));
        output.push_str(&self.punctuation_token(")"));
        output
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn reference_name(
        &self,
        name: &crate::references::ReferenceName,
        role: ReferenceRenderRole,
    ) -> String {
        let mut output = String::new();
        output.push_str(&self.color_token(&name.stem, role.stem_color()));
        if let Some(index) = name.occurrence {
            output.push_str(
                &self.color_token(&self.glyphs.numeric_suffix(index), role.suffix_color()),
            );
        }
        if let Some(slot) = &name.slot {
            output.push_str(&self.punctuation_token(self.glyphs.slot_open()));
            output.push_str(&self.color_token(&slot.text(), ColorRole::ReferenceSlot));
            output.push_str(&self.punctuation_token(self.glyphs.slot_close()));
        }
        output
    }

    #[requires(!text.is_empty())]
    #[ensures(!ret.is_empty())]
    fn constructor_token(&self, text: &str) -> String {
        self.color_token(text, ColorRole::Constructor)
    }

    #[requires(!text.is_empty())]
    #[ensures(!ret.is_empty())]
    fn field_token(&self, text: &str) -> String {
        self.color_token(text, ColorRole::Field)
    }

    #[requires(!text.is_empty())]
    #[ensures(!ret.is_empty())]
    fn punctuation_token(&self, text: &str) -> String {
        self.color_token(text, ColorRole::Punctuation)
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn number_token(&self, value: usize) -> String {
        self.color_token(&value.to_string(), ColorRole::Number)
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn span_number_token(&self, value: usize) -> String {
        self.color_token(&value.to_string(), ColorRole::SpanNumber)
    }

    #[requires(true)]
    #[ensures(!self.color -> ret == text)]
    fn color_token(&self, text: &str, role: ColorRole) -> String {
        if !self.color {
            return text.to_owned();
        }
        format!("{}{}{}", role.open(), text, "\x1b[39m")
    }

    #[requires(true)]
    #[ensures(!self.color -> ret == text)]
    fn elided_color_token(&self, text: &str, role: ColorRole) -> String {
        if !self.color {
            return text.to_owned();
        }
        format!("{}\x1b[9m{}\x1b[29m{}", role.open(), text, "\x1b[39m")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
enum ReferenceRenderRole {
    Reference,
    Referent,
}

impl ReferenceRenderRole {
    #[requires(true)]
    #[ensures(matches!(ret, ColorRole::ReferenceStem | ColorRole::ReferentStem))]
    fn stem_color(self) -> ColorRole {
        match self {
            Self::Reference => ColorRole::ReferenceStem,
            Self::Referent => ColorRole::ReferentStem,
        }
    }

    #[requires(true)]
    #[ensures(matches!(ret, ColorRole::ReferenceSuffix | ColorRole::ReferentSuffix))]
    fn suffix_color(self) -> ColorRole {
        match self {
            Self::Reference => ColorRole::ReferenceSuffix,
            Self::Referent => ColorRole::ReferentSuffix,
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_entry(entry: &TreeEntry) -> RenderEntry {
    match entry.label {
        Some(label) => RenderEntry::Labelled(label, entry.value.clone()),
        None => RenderEntry::Primary(entry.value.clone()),
    }
}

#[requires(true)]
#[ensures(ret.is_none_or(|(start, end)| start <= end))]
fn tree_node_span(node: &TreeNode) -> Option<(usize, usize)> {
    span_from_values(
        node.entries
            .iter()
            .filter_map(|entry| value_span(&entry.value)),
    )
}

#[requires(true)]
#[ensures(ret.is_none_or(|(start, end)| start <= end))]
fn value_span(value: &TreeValue) -> Option<(usize, usize)> {
    match value {
        TreeValue::Node(node) => tree_node_span(node),
        TreeValue::Collection(items) => span_from_values(items.iter().filter_map(value_span)),
        TreeValue::Syntax { value, .. } => value_span(value),
        TreeValue::Word { span, .. } | TreeValue::Verbatim { span, .. } => *span,
        TreeValue::Text(_) => None,
        TreeValue::Span {
            char_start,
            char_end,
            ..
        } => Some((*char_start, *char_end)),
    }
}

#[requires(true)]
#[ensures(ret.is_none_or(|(start, end)| start <= end))]
fn span_from_values<I>(spans: I) -> Option<(usize, usize)>
where
    I: IntoIterator<Item = (usize, usize)>,
{
    let mut iter = spans.into_iter();
    let (mut start, mut end) = iter.next()?;
    for (item_start, item_end) in iter {
        start = start.min(item_start);
        end = end.max(item_end);
    }
    Some((start, end))
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
#[invariant(::Node => true)]
#[invariant(::Field => true)]
enum MorphologyFrame {
    Node {
        constructor: &'static str,
        entries: Vec<TreeEntry>,
    },
    Field {
        name: Option<&'static str>,
        primary: bool,
        values: Vec<TreeValue>,
    },
}

#[derive(Debug)]
#[invariant(true)]
struct MorphologyTreeBuilder<'source> {
    source: &'source str,
    options: TreeRenderOptions,
    stack: Vec<MorphologyFrame>,
    root: Option<TreeValue>,
}

impl<'source> MorphologyTreeBuilder<'source> {
    #[requires(true)]
    #[ensures(ret.source == source)]
    fn new(source: &'source str, options: TreeRenderOptions) -> Self {
        Self {
            source,
            options,
            stack: Vec::new(),
            root: None,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn finish(self) -> TreeValue {
        self.root.expect("morphology tree walk produced a root")
    }

    #[requires(true)]
    #[ensures(true)]
    fn push_value(&mut self, value: TreeValue) {
        match self.stack.last_mut() {
            Some(MorphologyFrame::Field { values, .. }) => values.push(value),
            Some(MorphologyFrame::Node { entries, .. }) => {
                entries.push(TreeEntry { label: None, value })
            }
            None => self.root = Some(value),
        }
    }
}

impl<'tree> TreeVisitor<'tree> for MorphologyTreeBuilder<'_> {
    type Node = jbotci_morphology::NodeRef<'tree>;
    type Atom = jbotci_morphology::AtomRef<'tree>;

    #[requires(true)]
    #[ensures(true)]
    fn enter_node(&mut self, node: Self::Node) {
        self.stack.push(MorphologyFrame::Node {
            constructor: node.constructor_name(),
            entries: Vec::new(),
        });
    }

    #[requires(true)]
    #[ensures(true)]
    fn exit_node(&mut self, _node: Self::Node) {
        let Some(MorphologyFrame::Node {
            constructor,
            entries,
        }) = self.stack.pop()
        else {
            panic!("morphology tree walker exited a node without entering it");
        };
        let value = match morphology_node_value(constructor, &entries, self.options) {
            Some(value) => value,
            None => TreeValue::Node(TreeNode {
                constructor,
                entries,
            }),
        };
        self.push_value(value);
    }

    #[requires(true)]
    #[ensures(true)]
    fn enter_field(&mut self, field: FieldRef) {
        self.stack.push(MorphologyFrame::Field {
            name: field.name,
            primary: field.primary,
            values: Vec::new(),
        });
    }

    #[requires(true)]
    #[ensures(true)]
    fn exit_field(&mut self, _field: FieldRef) {
        let Some(MorphologyFrame::Field {
            name,
            primary,
            values,
        }) = self.stack.pop()
        else {
            panic!("morphology tree walker exited a field without entering it");
        };
        if values.is_empty() {
            return;
        }
        let Some(MorphologyFrame::Node { entries, .. }) = self.stack.last_mut() else {
            panic!("morphology tree field has no containing node");
        };
        if primary {
            for value in values {
                match value {
                    TreeValue::Collection(items) => {
                        entries.extend(
                            items
                                .into_iter()
                                .map(|value| TreeEntry { label: None, value }),
                        );
                    }
                    value => entries.push(TreeEntry { label: None, value }),
                }
            }
        } else {
            let value = if values.len() == 1 {
                values.into_iter().next().expect("length checked")
            } else {
                TreeValue::Collection(values)
            };
            entries.push(TreeEntry { label: name, value });
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_atom(&mut self, atom: Self::Atom) {
        self.push_value(match atom {
            jbotci_morphology::AtomRef::Phonemes(phonemes) => {
                TreeValue::Text(phonemes.render(self.options.phonemes))
            }
            jbotci_morphology::AtomRef::String(text) => TreeValue::Text(text.clone()),
            jbotci_morphology::AtomRef::SourceSpan(span) => source_span_value(span),
        });
    }
}

#[requires(true)]
#[ensures(true)]
fn morphology_node_value(
    constructor: &'static str,
    entries: &[TreeEntry],
    options: TreeRenderOptions,
) -> Option<TreeValue> {
    word_node_value(constructor, entries, options)
        .or_else(|| jvopau_node_value(constructor, entries, options))
        .or_else(|| verbatim_node_value(constructor, entries))
}

#[requires(true)]
#[ensures(true)]
fn word_node_value(
    constructor: &'static str,
    entries: &[TreeEntry],
    options: TreeRenderOptions,
) -> Option<TreeValue> {
    let kind = word_kind_from_constructor(constructor)?;
    let phonemes = if kind == WordKind::Lujvo && options.decompose_lujvo {
        lujvo_phoneme_text_from_entries(entries, true, options.glyphs)?
    } else if kind == WordKind::Lujvo {
        lujvo_phoneme_text_from_entries(entries, false, options.glyphs)?
    } else {
        phonemes_from_labelled_entries(entries)?.render(options.phonemes)
    };
    Some(TreeValue::Word {
        constructor,
        phonemes,
        span: span_from_labelled_entries(entries),
        elided: false,
    })
}

#[requires(true)]
#[ensures(true)]
fn jvopau_node_value(
    constructor: &'static str,
    entries: &[TreeEntry],
    options: TreeRenderOptions,
) -> Option<TreeValue> {
    if !matches!(constructor, "Rafsi" | "Hyphen") {
        return None;
    }
    Some(TreeValue::Text(
        phonemes_from_labelled_entries(entries)?.render(options.phonemes),
    ))
}

#[requires(true)]
#[ensures(true)]
fn verbatim_node_value(constructor: &'static str, entries: &[TreeEntry]) -> Option<TreeValue> {
    if constructor != "Verbatim" {
        return None;
    }
    for entry in entries {
        if let (Some("text"), TreeValue::Text(text)) = (entry.label, &entry.value) {
            return Some(TreeValue::Verbatim {
                text: text.trim().to_owned(),
                span: span_from_labelled_entries(entries),
            });
        }
    }
    None
}

#[requires(true)]
#[ensures(true)]
fn phonemes_from_labelled_entries(entries: &[TreeEntry]) -> Option<Phonemes> {
    for entry in entries {
        if let (Some("phonemes") | None, TreeValue::Text(text)) = (entry.label, &entry.value) {
            return Phonemes::from_canonical(text.clone()).ok();
        }
    }
    None
}

#[requires(true)]
#[ensures(true)]
fn lujvo_phoneme_text_from_entries(
    entries: &[TreeEntry],
    decompose: bool,
    glyphs: GlyphStyle,
) -> Option<String> {
    let mut parts = Vec::new();
    for entry in entries {
        match &entry.value {
            TreeValue::Text(part) => parts.push(part.clone()),
            TreeValue::Collection(values) => {
                for part in values {
                    if let TreeValue::Text(part) = part {
                        parts.push(part.clone());
                    }
                }
            }
            _ => {}
        }
    }
    (!parts.is_empty()).then(|| {
        if decompose {
            parts.join(glyphs.lujvo_separator())
        } else {
            parts.join("")
        }
    })
}

#[requires(true)]
#[ensures(ret.is_none_or(|(start, end)| start <= end))]
fn span_from_labelled_entries(entries: &[TreeEntry]) -> Option<(usize, usize)> {
    entries
        .iter()
        .find_map(|entry| match (&entry.label, &entry.value) {
            (
                Some("span"),
                TreeValue::Span {
                    char_start,
                    char_end,
                    ..
                },
            ) => Some((*char_start, *char_end)),
            _ => None,
        })
}

#[requires(true)]
#[ensures(true)]
fn word_kind_from_constructor(constructor: &str) -> Option<WordKind> {
    Some(match constructor {
        "Cmavo" => WordKind::Cmavo,
        "Gismu" => WordKind::Gismu,
        "Lujvo" => WordKind::Lujvo,
        "Fuhivla" => WordKind::Fuhivla,
        "Cmevla" => WordKind::Cmevla,
        _ => return None,
    })
}

#[requires(span.char_start <= span.char_end)]
#[ensures(true)]
fn source_span_value(span: &SourceSpan) -> TreeValue {
    TreeValue::Span {
        byte_start: span.byte_start,
        byte_end: span.byte_end,
        char_start: span.char_start,
        char_end: span.char_end,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ColorRole {
    Constructor,
    Field,
    Punctuation,
    Number,
    SpanNumber,
    ReferenceStem,
    ReferenceSuffix,
    ReferentStem,
    ReferentSuffix,
    ReferenceSlot,
    String,
}

impl ColorRole {
    #[requires(true)]
    #[ensures(ret.starts_with("\u{1b}["))]
    fn open(self) -> &'static str {
        match self {
            Self::Constructor => "\x1b[94m",
            Self::Field => "\x1b[32m",
            Self::Punctuation => "\x1b[90m",
            Self::Number => "\x1b[35m",
            Self::SpanNumber => "\x1b[37m",
            Self::ReferenceStem => "\x1b[36m",
            Self::ReferenceSuffix => "\x1b[96m",
            Self::ReferentStem => "\x1b[35m",
            Self::ReferentSuffix => "\x1b[95m",
            Self::ReferenceSlot => "\x1b[97m",
            Self::String => "\x1b[33m",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[allow(unused_imports)]
    use bityzba::{ensures, requires};
    use jbotci_morphology::{
        GlideMark, PhonemeRenderOptions, StressMark, segment_words_with_modifiers,
    };
    use jbotci_syntax::parse_syntax_tree;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn renders_basic_tree_with_primary_collapse() {
        let output = render("mi klama", false);
        assert_eq!(
            output,
            "Bridi {\n  leading_terms: [\n    Cmavo \"mi\",\n  ],\n  Gismu \"kláma\",\n}"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn colorizes_tree_tokens() {
        let output = render("mi klama", true);
        assert!(output.contains("\x1b[94mBridi\x1b[39m"));
        assert!(output.contains("\x1b[32mleading_terms\x1b[39m"));
        assert!(output.contains("\x1b[33m\"mi\"\x1b[39m"));
        assert!(output.contains("\x1b[94mCmavo\x1b[39m"));
        assert!(output.contains("\x1b[90m{\x1b[39m"));
        assert!(output.contains("\x1b[90m[\x1b[39m"));
        assert!(output.contains("\x1b[90m]\x1b[39m"));
        assert!(!output.contains("\x1b[36m"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn colorizes_visible_span_markers_with_white_offsets() {
        let output = run_on_normal_stack(|| {
            let words = segment_words_with_modifiers("mi klama").expect("morphology");
            let parsed = parse_syntax_tree(&words).expect("syntax");
            pretty_tree_with_options(
                &parsed.parse_tree,
                "mi klama",
                TreeRenderOptions {
                    color: true,
                    show_spans: true,
                    ..TreeRenderOptions::default()
                },
            )
            .expect("tree render")
        });

        assert!(output.contains(
            "\x1b[90m@\x1b[39m\x1b[90m[\x1b[39m\x1b[37m0\x1b[39m\x1b[90m‥\x1b[39m\x1b[37m8\x1b[39m\x1b[90m)\x1b[39m"
        ));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn colorizes_reference_names_by_direction() {
        let output = render_refs_with_options(
            "mi klama do i do klama mi",
            TreeRenderOptions {
                color: true,
                show_refs: true,
                ..TreeRenderOptions::default()
            },
        );

        assert!(output.contains("\x1b[35mk\x1b[39m\x1b[95m₁\x1b[39m"));
        assert!(output.contains("\x1b[36mk\x1b[39m\x1b[96m₁\x1b[39m"));
        assert!(output.contains("\x1b[90m⟨\x1b[39m\x1b[97m1\x1b[39m\x1b[90m⟩\x1b[39m"));
        assert!(output.contains("\x1b[90m→\x1b[39m"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn keeps_free_modifiers_label_when_present() {
        let output = render("mi klama to coi toi", false);
        assert!(output.contains("free_modifiers: ["));
        assert!(output.contains("ParentheticalText {"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn renders_compound_word_like_values_as_structured_nodes() {
        let zo = render("zo broda cu melbi", false);
        assert!(zo.contains("QuotedWord {"));
        assert!(zo.contains("Cmavo \"zo\""));
        assert!(zo.contains("Gismu \"bróda\""));

        let zoi = render("zoi gy hello gy cu melbi", false);
        assert!(zoi.contains("DelimitedNonLojbanQuote {"));
        assert!(zoi.contains("quoted_text: Verbatim \"hello\""));

        let lohu = render("lo'u mi klama le'u cu melbi", false);
        assert!(lohu.contains("QuotedWords {"));
        assert!(lohu.contains("Gismu \"kláma\""));

        let bu = render("abu cu lerfu", false);
        assert!(bu.contains("LerfuWord {"));
        assert!(bu.contains("bu: Cmavo \"bu\""));

        let zei = render("mi broda zei brode", false);
        assert!(zei.contains("ZeiCompound {"));
        assert!(zei.contains("Gismu \"bróde\""));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn renders_single_line_when_indent_is_zero() {
        run_on_normal_stack(|| {
            let words = segment_words_with_modifiers("mi klama").expect("morphology");
            let parsed = parse_syntax_tree(&words).expect("syntax");
            let output = pretty_tree_with_options(
                &parsed.parse_tree,
                "mi klama",
                TreeRenderOptions {
                    color: false,
                    indent: 0,
                    ..TreeRenderOptions::default()
                },
            )
            .expect("tree render");
            assert_eq!(output, r#"Bridi{leading_terms:[Cmavo "mi"],Gismu "kláma"}"#);
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn renders_resolved_references_in_tree_output() {
        let output = render_refs("mi klama le zarci i do klama ri", true);
        assert_eq!(
            output,
            "Paragraph @[0‥31) {\n  Bridi @[0‥17) {\n    leading_terms: [\n      k₁⟨1⟩→ Cmavo @[0‥2) \"mi\",\n    ],\n    SelbriBridiTail @[3‥17) {\n      Gismu @[3‥8) \"kláma\" →k₁,\n      terms: [\n        k₁⟨2⟩→ ri₁→ Description @[9‥17) {\n          description: Cmavo @[9‥11) \"le\",\n          selbri: Gismu @[12‥17) \"zárci\",\n        },\n      ],\n    },\n  },\n  ParagraphStatement @[18‥31) {\n    i: Cmavo @[18‥19) \"i\",\n    Bridi @[20‥31) {\n      leading_terms: [\n        k₂⟨1⟩→ Cmavo @[20‥22) \"do\",\n      ],\n      SelbriBridiTail @[23‥31) {\n        Gismu @[23‥28) \"kláma\" →k₂,\n        terms: [\n          k₂⟨2⟩→ Cmavo @[29‥31) \"ri\" →ri₁,\n        ],\n      },\n    },\n  },\n}"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn renders_ascii_references_spans_and_phonemes() {
        let output = render_refs_with_options(
            "mi klama le zarci i do klama ri",
            TreeRenderOptions {
                glyphs: GlyphStyle::Ascii,
                show_spans: true,
                show_refs: true,
                phonemes: PhonemeRenderOptions {
                    mark_stress: StressMark::None,
                    mark_glides: GlideMark::None,
                },
                ..TreeRenderOptions::default()
            },
        );

        assert!(output.contains("k1<1>-> Cmavo @[0..2) \"mi\""));
        assert!(output.contains("Gismu @[3..8) \"klama\" ->k1"));
        assert!(output.contains("k1<2>-> ri1-> Description"));
        assert!(output.contains("Cmavo @[29..31) \"ri\" ->ri1"));
        assert!(!output.contains('→'));
        assert!(!output.contains('⟨'));
        assert!(!output.contains('⟩'));
        assert!(!output.contains('‥'));
        assert!(!output.contains('á'));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn renders_only_base_frame_for_converted_selbri() {
        let output = render_refs("mi se klama do", false);
        assert!(output.contains("k⟨2⟩→ Cmavo \"mi\""));
        assert!(output.contains("k⟨1⟩→ Cmavo \"do\""));
        assert!(output.contains("Gismu \"kláma\" →k"));
        assert!(!output.contains("s⟨"));
        assert!(!output.contains("Cmavo \"se\" →"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn renders_duplicate_place_fillers_with_same_label() {
        let output = render_refs("fa mi fa do klama", false);
        assert_eq!(output.matches("k⟨1⟩→ Cmavo").count(), 2);
        assert!(output.contains("Gismu \"kláma\" →k"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn renders_modal_place_labels() {
        let output = render_refs("mi ta'i do klama", false);
        assert!(output.contains("k⟨ta'i⟩→ Cmavo \"do\""));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn renders_duplicate_prefixes_across_repeated_words() {
        let output = render_refs("mi klama le karce be do i do klama le karce be mi", false);
        assert!(output.contains("Gismu \"kláma\" →kl₁"));
        assert!(output.contains("Gismu \"kláma\" →kl₂"));
        assert!(output.contains("Gismu \"kárce\" →ká₁"));
        assert!(output.contains("Gismu \"kárce\" →ká₂"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn renders_resolved_discourse_reference_kinds() {
        let gohi = render_refs("mi klama .i go'i", false);
        assert!(gohi.contains("go'i₁→ Bridi"));
        assert!(gohi.contains("Cmavo \"go'i\" →go'i₁"));

        let goi = render_refs("le nanmu goi ko'a cu klama .i ko'a cadzu", false);
        assert!(goi.contains("Cmavo \"ko'a\" →ko'a₁"));
        assert!(goi.contains("Cmavo \"ko'a\" →ko'a₂"));
        assert!(goi.contains("ko'a₁→ ko'a₂→ Description"));

        let cei = render_refs("mi broda cei klama do", false);
        assert!(cei.contains("k→ Bridi"));
        assert!(cei.contains("Gismu \"bróda\" →b"));
        assert!(!cei.contains("k→ Gismu \"bróda\""));
        assert!(cei.contains("Gismu \"kláma\" →k"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn omits_unresolved_and_vague_discourse_references() {
        let output = render_refs("ri klama .i ra klama .i ru klama", false);
        assert!(!output.contains("→ri"));
        assert!(!output.contains("→ra"));
        assert!(!output.contains("→ru"));
        assert!(!output.contains("ri₁→"));
        assert!(!output.contains("ra₁→"));
        assert!(!output.contains("ru₁→"));
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn render(text: &str, color: bool) -> String {
        let text = text.to_owned();
        run_on_normal_stack(move || {
            let words = segment_words_with_modifiers(&text).expect("morphology");
            let parsed = parse_syntax_tree(&words).expect("syntax");
            pretty_tree_with_options(
                &parsed.parse_tree,
                &text,
                TreeRenderOptions {
                    color,
                    indent: 2,
                    ..TreeRenderOptions::default()
                },
            )
            .expect("tree render")
        })
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn render_refs(text: &str, show_spans: bool) -> String {
        render_refs_with_options(
            text,
            TreeRenderOptions {
                color: false,
                indent: 2,
                show_spans,
                show_refs: true,
                ..TreeRenderOptions::default()
            },
        )
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn render_refs_with_options(text: &str, options: TreeRenderOptions) -> String {
        let text = text.to_owned();
        run_on_normal_stack(move || {
            let words = segment_words_with_modifiers(&text).expect("morphology");
            let parsed = parse_syntax_tree(&words).expect("syntax");
            pretty_tree_with_options(&parsed.parse_tree, &text, options).expect("tree render")
        })
    }

    #[requires(true)]
    #[ensures(true)]
    fn run_on_normal_stack<R>(f: impl FnOnce() -> R) -> R {
        f()
    }
}
