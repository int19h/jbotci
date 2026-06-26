//! Renderer for the source-backed syntax tree output format.

use std::cell::RefCell;

#[allow(unused_imports)]
use bityzba::{contract_trait, ensures, invariant, requires};
use jbotci_morphology::{
    Cmavo, Phonemes, Selmaho, TreeNode as MorphologyTreeNode, Word, WordKind, WordLike,
    WordLikeData,
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

thread_local! {
    static LEGACY_GENERATED_TOKEN_STREAM: RefCell<Option<Vec<Token>>> = const { RefCell::new(None) };
}

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

#[invariant(::Operand(operand) => legacy_first_token_byte_start(operand).is_some())]
#[invariant(::Operation { left, right, operator } => legacy_reverse_polish_expr_is_positioned(left.as_ref()) && legacy_reverse_polish_expr_is_positioned(right.as_ref()) && legacy_first_token_byte_start(operator).is_some())]
#[derive(Debug)]
enum LegacyReversePolishExpr {
    Operand(jbotci_syntax::ast::MeksoSyntax),
    Operation {
        left: Box<LegacyReversePolishExpr>,
        right: Box<LegacyReversePolishExpr>,
        operator: jbotci_syntax::ast::MeksoOperatorSyntax,
    },
}

#[invariant(::Operand { start, operand } => legacy_first_token_byte_start(operand).is_some_and(|operand_start| operand_start == *start))]
#[invariant(::Operator { start, operator } => legacy_first_token_byte_start(operator).is_some_and(|operator_start| operator_start == *start))]
#[derive(Debug)]
enum LegacyReversePolishItem {
    Operand {
        start: usize,
        operand: jbotci_syntax::ast::MeksoSyntax,
    },
    Operator {
        start: usize,
        operator: jbotci_syntax::ast::MeksoOperatorSyntax,
    },
}

#[requires(true)]
#[ensures(true)]
fn legacy_reverse_polish_expr_is_positioned(expression: &LegacyReversePolishExpr) -> bool {
    match expression.as_data() {
        bityzba::data!(LegacyReversePolishExpr::Operand(operand)) => {
            legacy_first_token_byte_start(operand).is_some()
        }
        bityzba::data!(LegacyReversePolishExpr::Operation {
            left,
            right,
            operator,
        }) => {
            legacy_reverse_polish_expr_is_positioned(left.as_ref())
                && legacy_reverse_polish_expr_is_positioned(right.as_ref())
                && legacy_first_token_byte_start(operator).is_some()
        }
    }
}

impl LegacyReversePolishItem {
    #[requires(true)]
    #[ensures(true)]
    fn start(&self) -> usize {
        match self.as_data() {
            bityzba::data!(LegacyReversePolishItem::Operand { start, .. })
            | bityzba::data!(LegacyReversePolishItem::Operator { start, .. }) => *start,
        }
    }
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

#[doc(hidden)]
#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()) || ret.is_err())]
pub fn pretty_legacy_as_generated_model_tree_with_options(
    tree: &TextSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> Result<String, OutputError> {
    if options.show_refs {
        return Err(OutputError::References(
            "legacy-as-generated syntax reference rendering is not wired yet".to_owned(),
        ));
    }
    let mut token_stream = Vec::new();
    tree.visit_words(&mut |token| token_stream.push(token.clone()));
    let previous_token_stream =
        LEGACY_GENERATED_TOKEN_STREAM.with(|stream| stream.replace(Some(token_stream)));
    let value = collapse_value(legacy_as_generated_text_tree_value(tree, source, options));
    LEGACY_GENERATED_TOKEN_STREAM.with(|stream| {
        stream.replace(previous_token_stream);
    });
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
fn legacy_as_generated_leading_indicator_tree_value(
    indicator: &jbotci_syntax::ast::Indicator,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    let mut entries = vec![TreeEntry {
        label: Some("indicator"),
        value: generated_token_tree_value(&indicator.indicator, source, options),
    }];
    if let Some(nai) = &indicator.nai {
        let nai = Token::bare(WordLike::bare(nai.clone()));
        entries.push(TreeEntry {
            label: Some("nai"),
            value: generated_token_tree_value(&nai, source, options),
        });
    }
    TreeValue::Node(TreeNode {
        constructor: "LeadingIndicator",
        entries,
    })
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

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_text_tree_value(
    tree: &TextSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    let mut entries = Vec::new();
    if let Some(entry) = labelled_tree_collection_entry_from_values(
        "leading_nai",
        tree.leading_nai
            .iter()
            .map(|token| generated_token_tree_value(token, source, options))
            .collect(),
    ) {
        entries.push(entry);
    }
    if let Some(entry) = labelled_tree_collection_entry_from_values(
        "leading_cmevla",
        tree.leading_cmevla
            .iter()
            .map(|token| generated_token_tree_value(token, source, options))
            .collect(),
    ) {
        entries.push(entry);
    }
    if let Some(entry) = labelled_tree_collection_entry_from_values(
        "leading_indicators",
        tree.leading_indicators
            .iter()
            .map(|indicator| {
                legacy_as_generated_leading_indicator_tree_value(indicator, source, options)
            })
            .collect(),
    ) {
        entries.push(entry);
    }
    if let Some(entry) = labelled_tree_collection_entry_from_values(
        "leading_free_modifiers",
        tree.leading_free_modifiers
            .iter()
            .map(|free_modifier| {
                legacy_as_generated_free_modifier_tree_value(free_modifier, source, options)
            })
            .collect(),
    ) {
        entries.push(entry);
    }
    if let Some(connective) = &tree.leading_connective {
        entries.push(TreeEntry {
            label: Some("leading_connective"),
            value: required_legacy_syntax_subtree_value(connective.as_ref(), source, options),
        });
    }
    entries.extend(tree.paragraphs.iter().map(|paragraph| TreeEntry {
        label: None,
        value: legacy_as_generated_paragraph_tree_value(paragraph, source, options),
    }));
    TreeValue::Node(TreeNode {
        constructor: "Text",
        entries,
    })
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_text_child_tree_value(
    tree: &TextSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    if tree.leading_nai.is_empty()
        && tree.leading_cmevla.is_empty()
        && tree.leading_indicators.is_empty()
        && tree.leading_free_modifiers.is_empty()
        && tree.leading_connective.is_none()
        && tree.paragraphs.len() == 1
    {
        return legacy_as_generated_paragraph_tree_value(
            tree.paragraphs.first().expect("length checked"),
            source,
            options,
        );
    }

    legacy_as_generated_text_tree_value(tree, source, options)
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_subbridi_tree_value(
    subbridi: &jbotci_syntax::ast::SubbridiSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    match subbridi.as_data() {
        bityzba::data!(jbotci_syntax::ast::SubbridiSyntax::Bridi(bridi)) => {
            TreeValue::Node(TreeNode {
                constructor: "BridiSubbridi",
                entries: vec![TreeEntry {
                    label: Some("bridi_subbridi"),
                    value: TreeValue::Node(TreeNode {
                        constructor: "BridiSubbridi",
                        entries: vec![TreeEntry {
                            label: Some("bridi"),
                            value: legacy_as_generated_bridi_tree_value(
                                bridi.as_ref(),
                                source,
                                options,
                            ),
                        }],
                    }),
                }],
            })
        }
        bityzba::data!(jbotci_syntax::ast::SubbridiSyntax::Prenex {
            prenex_terms,
            zohu,
            inner_subbridi,
        }) => {
            let mut entries = Vec::new();
            if let Some(entry) = labelled_tree_collection_entry_from_values(
                "prenex_terms",
                prenex_terms
                    .iter()
                    .map(|term| legacy_as_generated_term_tree_value(term, source, options))
                    .collect(),
            ) {
                entries.push(entry);
            }
            entries.push(TreeEntry {
                label: Some("zohu"),
                value: required_legacy_syntax_subtree_value(zohu, source, options),
            });
            entries.push(TreeEntry {
                label: Some("inner_subbridi"),
                value: legacy_as_generated_subbridi_tree_value(
                    inner_subbridi.as_ref(),
                    source,
                    options,
                ),
            });
            TreeValue::Node(TreeNode {
                constructor: "PrenexSubbridi",
                entries: vec![TreeEntry {
                    label: Some("prenex_subbridi"),
                    value: TreeValue::Node(TreeNode {
                        constructor: "PrenexSubbridi",
                        entries,
                    }),
                }],
            })
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_paragraph_tree_value(
    paragraph: &jbotci_syntax::ast::ParagraphSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    if paragraph.i.is_none() && paragraph.niho.is_empty() && paragraph.free_modifiers.is_empty() {
        let statement_values = paragraph
            .statements
            .iter()
            .map(|statement| {
                legacy_as_generated_paragraph_statement_tree_value(statement, source, options)
            })
            .collect::<Vec<_>>();
        if statement_values.len() == 1 {
            return statement_values.into_iter().next().expect("length checked");
        }
        return TreeValue::Node(TreeNode {
            constructor: "Paragraph",
            entries: statement_values
                .into_iter()
                .map(|value| TreeEntry { label: None, value })
                .collect(),
        });
    }

    let mut entries = Vec::new();
    if let Some(i) = &paragraph.i {
        entries.push(TreeEntry {
            label: Some("i"),
            value: generated_token_tree_value(i, source, options),
        });
    }
    if let Some(entry) = labelled_tree_collection_entry_from_values(
        "niho",
        paragraph
            .niho
            .iter()
            .map(|token| generated_token_tree_value(token, source, options))
            .collect(),
    ) {
        entries.push(entry);
    }
    if let Some(entry) = labelled_tree_collection_entry_from_values(
        "free_modifiers",
        paragraph
            .free_modifiers
            .iter()
            .map(|free_modifier| {
                legacy_as_generated_free_modifier_tree_value(free_modifier, source, options)
            })
            .collect(),
    ) {
        entries.push(entry);
    }
    entries.extend(paragraph.statements.iter().map(|statement| TreeEntry {
        label: None,
        value: legacy_as_generated_paragraph_statement_tree_value(statement, source, options),
    }));
    TreeValue::Node(TreeNode {
        constructor: "Paragraph",
        entries,
    })
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_paragraph_statements_tree_value(
    statements: &[jbotci_syntax::ast::ParagraphStatementSyntax],
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    let values = statements
        .iter()
        .map(|statement| {
            legacy_as_generated_paragraph_statement_tree_value(statement, source, options)
        })
        .collect::<Vec<_>>();
    if values.len() == 1 {
        values.into_iter().next().expect("length checked")
    } else {
        TreeValue::Collection(values)
    }
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_paragraph_statement_tree_value(
    statement: &jbotci_syntax::ast::ParagraphStatementSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    if statement.i.is_none()
        && statement.connective.is_none()
        && statement.free_modifiers.is_empty()
        && let Some(inner) = &statement.statement
    {
        return legacy_as_generated_statement_tree_value(inner.as_ref(), source, options);
    }

    let mut entries = Vec::new();
    if let Some(i) = &statement.i {
        entries.push(TreeEntry {
            label: Some("i"),
            value: generated_token_tree_value(i, source, options),
        });
    }
    if let Some(connective) = &statement.connective {
        entries.push(TreeEntry {
            label: Some("connective"),
            value: required_legacy_syntax_subtree_value(connective.as_ref(), source, options),
        });
    }
    if let Some(entry) = labelled_tree_collection_entry_from_values(
        "free_modifiers",
        statement
            .free_modifiers
            .iter()
            .map(|free_modifier| {
                legacy_as_generated_free_modifier_tree_value(free_modifier, source, options)
            })
            .collect(),
    ) {
        entries.push(entry);
    }
    if let Some(inner) = &statement.statement {
        entries.push(TreeEntry {
            label: None,
            value: legacy_as_generated_statement_tree_value(inner.as_ref(), source, options),
        });
    }
    TreeValue::Node(TreeNode {
        constructor: "ParagraphStatement",
        entries,
    })
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_statement_tree_value(
    statement: &jbotci_syntax::ast::StatementSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    match statement.as_data() {
        bityzba::data!(jbotci_syntax::ast::StatementSyntax::TextGroup {
            tense_modal,
            tuhe,
            text,
            tuhu,
        }) => {
            let mut entries = Vec::new();
            if let Some(tense_modal) = tense_modal {
                entries.push(TreeEntry {
                    label: Some("tense_modal"),
                    value: legacy_as_generated_tense_modal_tree_value(
                        tense_modal.as_ref(),
                        source,
                        options,
                    ),
                });
            }
            entries.extend(legacy_token_field_entries("tuhe", tuhe, source, options));
            entries.push(TreeEntry {
                label: None,
                value: legacy_as_generated_text_tree_value(text.as_ref(), source, options),
            });
            if let Some(tuhu) = tuhu {
                entries.extend(legacy_token_field_entries("tuhu", tuhu, source, options));
            }
            TreeValue::Node(TreeNode {
                constructor: "TextGroup",
                entries,
            })
        }
        bityzba::data!(jbotci_syntax::ast::StatementSyntax::Bridi(bridi)) => {
            legacy_as_generated_bridi_tree_value(bridi.as_ref(), source, options)
        }
        bityzba::data!(jbotci_syntax::ast::StatementSyntax::Prenex {
            prenex_terms,
            zohu,
            inner_statement,
        }) => {
            let mut entries = Vec::new();
            if let Some(entry) = labelled_tree_collection_entry_from_values(
                "prenex_terms",
                prenex_terms
                    .iter()
                    .map(|term| legacy_as_generated_term_tree_value(term, source, options))
                    .collect(),
            ) {
                entries.push(entry);
            }
            entries.extend(legacy_token_field_entries("zohu", zohu, source, options));
            entries.push(TreeEntry {
                label: None,
                value: legacy_as_generated_statement_tree_value(
                    inner_statement.as_ref(),
                    source,
                    options,
                ),
            });
            TreeValue::Node(TreeNode {
                constructor: "Prenex",
                entries,
            })
        }
        bityzba::data!(jbotci_syntax::ast::StatementSyntax::Fragment(fragment)) => {
            legacy_as_generated_fragment_tree_value(fragment.as_ref(), source, options)
        }
        bityzba::data!(jbotci_syntax::ast::StatementSyntax::StatementConnection {
            leading_statement,
            i,
            connective,
            trailing_statement,
        }) => TreeValue::Node(TreeNode {
            constructor: "StatementConnection",
            entries: vec![
                TreeEntry {
                    label: Some("leading_statement"),
                    value: legacy_as_generated_statement_tree_value(
                        leading_statement.as_ref(),
                        source,
                        options,
                    ),
                },
                TreeEntry {
                    label: Some("i"),
                    value: generated_token_tree_value(i, source, options),
                },
                TreeEntry {
                    label: Some("connective"),
                    value: required_legacy_syntax_subtree_value(connective, source, options),
                },
                TreeEntry {
                    label: Some("trailing_statement"),
                    value: legacy_as_generated_statement_tree_value(
                        trailing_statement.as_ref(),
                        source,
                        options,
                    ),
                },
            ],
        }),
        bityzba::data!(
            jbotci_syntax::ast::StatementSyntax::PreposedIStatementConnection {
                leading_statement,
                connective,
                i,
                trailing_statement,
            }
        ) => TreeValue::Node(TreeNode {
            constructor: "PreposedIStatementConnection",
            entries: vec![
                TreeEntry {
                    label: Some("leading_statement"),
                    value: legacy_as_generated_statement_tree_value(
                        leading_statement.as_ref(),
                        source,
                        options,
                    ),
                },
                TreeEntry {
                    label: Some("connective"),
                    value: required_legacy_syntax_subtree_value(connective, source, options),
                },
                TreeEntry {
                    label: Some("i"),
                    value: generated_token_tree_value(i, source, options),
                },
                TreeEntry {
                    label: Some("trailing_statement"),
                    value: legacy_as_generated_statement_tree_value(
                        trailing_statement.as_ref(),
                        source,
                        options,
                    ),
                },
            ],
        }),
        bityzba::data!(
            jbotci_syntax::ast::StatementSyntax::ExperimentalBridiContinuation {
                leading_statement,
                continuation,
            }
        ) => TreeValue::Node(TreeNode {
            constructor: "ExperimentalBridiContinuation",
            entries: vec![
                TreeEntry {
                    label: Some("leading_statement"),
                    value: legacy_as_generated_statement_tree_value(
                        leading_statement.as_ref(),
                        source,
                        options,
                    ),
                },
                TreeEntry {
                    label: Some("continuation"),
                    value: legacy_as_generated_bridi_statement_continuation_tree_value(
                        continuation,
                        source,
                        options,
                    ),
                },
            ],
        }),
        _ => required_legacy_syntax_subtree_value(statement, source, options),
    }
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_bridi_statement_continuation_tree_value(
    continuation: &jbotci_syntax::ast::BridiStatementContinuationSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    let mut entries = vec![TreeEntry {
        label: Some("connective"),
        value: legacy_as_generated_connective_tree_value(&continuation.connective, source, options),
    }];
    if let Some(tense_modal) = &continuation.tense_modal {
        entries.push(TreeEntry {
            label: Some("tense_modal"),
            value: legacy_as_generated_tense_modal_tree_value(
                tense_modal.as_ref(),
                source,
                options,
            ),
        });
    }
    entries.push(TreeEntry {
        label: Some("marker"),
        value: legacy_as_generated_bridi_statement_continuation_marker_tree_value(
            &continuation.marker,
            source,
            options,
        ),
    });
    entries.push(TreeEntry {
        label: Some("trailing_subbridi"),
        value: legacy_as_generated_subbridi_tree_value(
            continuation.trailing_subbridi.as_ref(),
            source,
            options,
        ),
    });
    TreeValue::Node(TreeNode {
        constructor: "BridiStatementContinuation",
        entries,
    })
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_bridi_statement_continuation_marker_tree_value(
    marker: &jbotci_syntax::ast::BridiStatementContinuationMarkerSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    match marker.as_data() {
        bityzba::data!(
            jbotci_syntax::ast::BridiStatementContinuationMarkerSyntax::BoGrouped(bo)
        ) => legacy_token_tree_value_with_extra_free_modifiers(
            &bo.value,
            &bo.free_modifiers,
            source,
            options,
        ),
        bityzba::data!(
            jbotci_syntax::ast::BridiStatementContinuationMarkerSyntax::KeGrouped { ke, kehe }
        ) => {
            let mut entries = legacy_token_field_entries("ke", ke, source, options);
            if let Some(kehe) = kehe {
                entries.extend(legacy_token_field_entries("kehe", kehe, source, options));
            }
            TreeValue::Node(TreeNode {
                constructor: "KeGrouped",
                entries,
            })
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_fragment_tree_value(
    fragment: &jbotci_syntax::ast::FragmentSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    match fragment.as_data() {
        bityzba::data!(jbotci_syntax::ast::FragmentSyntax::Ek(connective))
        | bityzba::data!(jbotci_syntax::ast::FragmentSyntax::BridiTailConnective(
            connective
        )) => legacy_as_generated_connective_tree_value(connective, source, options),
        bityzba::data!(jbotci_syntax::ast::FragmentSyntax::Other(words)) => {
            let mut entries = words
                .value
                .iter()
                .map(|word| TreeEntry {
                    label: None,
                    value: generated_token_tree_value(word, source, options),
                })
                .collect::<Vec<_>>();
            if let Some(entry) = labelled_tree_collection_entry_from_values(
                "free_modifiers",
                words
                    .free_modifiers
                    .iter()
                    .map(|free_modifier| {
                        legacy_as_generated_free_modifier_tree_value(free_modifier, source, options)
                    })
                    .collect(),
            ) {
                entries.push(entry);
            }
            TreeValue::Node(TreeNode {
                constructor: "Other",
                entries,
            })
        }
        bityzba::data!(jbotci_syntax::ast::FragmentSyntax::Terms { terms, vau }) => {
            if terms.len() == 1 && vau.is_none() {
                return legacy_as_generated_term_tree_value(
                    terms.first().expect("length checked"),
                    source,
                    options,
                );
            }
            let mut entries = terms
                .iter()
                .map(|term| TreeEntry {
                    label: None,
                    value: legacy_as_generated_term_tree_value(term, source, options),
                })
                .collect::<Vec<_>>();
            if let Some(vau) = vau {
                entries.push(TreeEntry {
                    label: Some("vau"),
                    value: required_legacy_syntax_subtree_value(vau, source, options),
                });
            }
            TreeValue::Node(TreeNode {
                constructor: "Terms",
                entries,
            })
        }
        bityzba::data!(jbotci_syntax::ast::FragmentSyntax::RelativeClauses(
            relative_clauses
        )) => {
            let values = relative_clauses
                .iter()
                .map(|relative_clause| {
                    legacy_as_generated_relative_clause_tree_value(relative_clause, source, options)
                })
                .collect::<Vec<_>>();
            if values.len() == 1 {
                values.into_iter().next().expect("length checked")
            } else {
                TreeValue::Collection(values)
            }
        }
        bityzba::data!(jbotci_syntax::ast::FragmentSyntax::Prenex { terms, zohu }) => {
            let mut entries = Vec::new();
            if let Some(entry) = labelled_tree_collection_entry_from_values(
                "terms",
                terms
                    .iter()
                    .map(|term| legacy_as_generated_term_tree_value(term, source, options))
                    .collect(),
            ) {
                entries.push(entry);
            }
            entries.extend(legacy_token_field_entries("zohu", zohu, source, options));
            TreeValue::Node(TreeNode {
                constructor: "Prenex",
                entries,
            })
        }
        bityzba::data!(jbotci_syntax::ast::FragmentSyntax::LinkedSumti {
            be,
            fa,
            first_sumti,
            bei_links,
            beho,
        }) => rename_tree_constructor(
            legacy_as_generated_linked_sumti_list_tree_value(
                be,
                fa.as_ref(),
                first_sumti.as_deref(),
                bei_links,
                beho.as_ref(),
                source,
                options,
            ),
            "Linkargs",
            "LinkedSumti",
        ),
        bityzba::data!(jbotci_syntax::ast::FragmentSyntax::LinkedSumtiContinuation(
            bei_links
        )) => {
            let values = bei_links
                .iter()
                .map(|link| {
                    legacy_as_generated_additional_linked_sumti_tree_value(link, source, options)
                })
                .collect::<Vec<_>>();
            if values.len() == 1 {
                values.into_iter().next().expect("length checked")
            } else {
                TreeValue::Collection(values)
            }
        }
        bityzba::data!(jbotci_syntax::ast::FragmentSyntax::Mekso(mekso)) => {
            legacy_as_generated_mekso_fragment_tree_value(mekso.as_ref(), source, options)
        }
        _ => required_legacy_syntax_subtree_value(fragment, source, options),
    }
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_mekso_fragment_tree_value(
    mekso: &jbotci_syntax::ast::MeksoSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    match mekso.as_data() {
        bityzba::data!(jbotci_syntax::ast::MeksoSyntax::NumberMekso(quantifier)) => {
            legacy_as_generated_quantifier_tree_value(quantifier.as_ref(), source, options)
        }
        bityzba::data!(jbotci_syntax::ast::MeksoSyntax::ParenthesizedMekso {
            vei,
            inner_expression,
            veho,
        }) => {
            let mut entries = legacy_token_field_entries("vei", vei, source, options);
            entries.push(TreeEntry {
                label: Some("mekso"),
                value: legacy_as_generated_mekso_tree_value(
                    inner_expression.as_ref(),
                    source,
                    options,
                ),
            });
            if let Some(veho) = veho {
                entries.extend(legacy_token_field_entries("veho", veho, source, options));
            }
            TreeValue::Node(TreeNode {
                constructor: "MeksoQuantifier",
                entries,
            })
        }
        _ => legacy_as_generated_mekso_tree_value(mekso, source, options),
    }
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_bridi_tree_value(
    bridi: &jbotci_syntax::ast::BridiSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    let mut entries = Vec::new();
    if let Some(entry) = labelled_tree_collection_entry_from_values(
        "leading_terms",
        bridi
            .leading_terms
            .iter()
            .map(|term| legacy_as_generated_term_tree_value(term, source, options))
            .collect(),
    ) {
        entries.push(entry);
    }
    if let Some(cu) = &bridi.cu {
        entries.push(TreeEntry {
            label: Some("cu"),
            value: required_legacy_syntax_subtree_value(cu.as_ref(), source, options),
        });
    }
    let bridi_tail = if bridi.cu.is_some() {
        legacy_as_generated_post_cu_bridi_tail_tree_value(
            bridi.bridi_tail.as_ref(),
            source,
            options,
        )
        .unwrap_or_else(|| {
            legacy_as_generated_bridi_tail_tree_value(bridi.bridi_tail.as_ref(), source, options)
        })
    } else {
        legacy_as_generated_bridi_tail_tree_value(bridi.bridi_tail.as_ref(), source, options)
    };
    entries.push(TreeEntry {
        label: Some("bridi_tail"),
        value: bridi_tail,
    });
    if let Some(entry) = labelled_tree_collection_entry_from_values(
        "free_modifiers",
        bridi
            .free_modifiers
            .iter()
            .map(|free_modifier| {
                legacy_as_generated_free_modifier_tree_value(free_modifier, source, options)
            })
            .collect(),
    ) {
        entries.push(entry);
    }
    TreeValue::Node(TreeNode {
        constructor: "Bridi",
        entries,
    })
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_post_cu_bridi_tail_tree_value(
    bridi_tail: &jbotci_syntax::ast::BridiTailSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> Option<TreeValue> {
    if bridi_tail.ke_continuation.is_some()
        || !bridi_tail.first.continuations.is_empty()
        || bridi_tail.first.first.bo_continuation.is_some()
    {
        return None;
    }

    let bityzba::data!(
        jbotci_syntax::ast::SimpleBridiTailSyntax::TermPrefixedBridiTail {
            terms,
            bridi_tail: inner_bridi_tail,
        }
    ) = bridi_tail.first.first.first.as_data()
    else {
        return None;
    };

    let mut entries = vec![TreeEntry {
        label: Some("terms"),
        value: TreeValue::Collection(
            terms
                .iter()
                .map(|term| legacy_as_generated_term_tree_value(term, source, options))
                .collect(),
        ),
    }];
    entries.push(TreeEntry {
        label: Some("bridi_tail"),
        value: legacy_as_generated_bridi_tail_tree_value(
            inner_bridi_tail.as_ref(),
            source,
            options,
        ),
    });
    Some(TreeValue::Node(TreeNode {
        constructor: "CuTermsBridiTail",
        entries,
    }))
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_bridi_tail_tree_value(
    bridi_tail: &jbotci_syntax::ast::BridiTailSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    let mut entries = vec![TreeEntry {
        label: Some("first"),
        value: legacy_as_generated_afterthought_bridi_tail_tree_value(
            bridi_tail.first.as_ref(),
            source,
            options,
        ),
    }];
    if let Some(ke_continuation) = &bridi_tail.ke_continuation {
        entries.push(TreeEntry {
            label: Some("ke_continuation"),
            value: legacy_as_generated_grouped_bridi_tail_connection_tree_value(
                ke_continuation.as_ref(),
                source,
                options,
            ),
        });
    }
    TreeValue::Node(TreeNode {
        constructor: "BridiTailWithPossibleTailTerms",
        entries,
    })
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_afterthought_bridi_tail_tree_value(
    bridi_tail: &jbotci_syntax::ast::AfterthoughtBridiTailSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    let mut entries = vec![TreeEntry {
        label: Some("first"),
        value: legacy_as_generated_bo_grouped_bridi_tail_tree_value(
            bridi_tail.first.as_ref(),
            source,
            options,
        ),
    }];
    if let Some(entry) = labelled_tree_collection_entry_from_values(
        "continuations",
        bridi_tail
            .continuations
            .iter()
            .map(|continuation| {
                legacy_as_generated_bridi_tail_connection_tree_value(continuation, source, options)
            })
            .collect(),
    ) {
        entries.push(entry);
    }
    TreeValue::Node(TreeNode {
        constructor: "AfterthoughtBridiTail",
        entries,
    })
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_bo_grouped_bridi_tail_tree_value(
    bridi_tail: &jbotci_syntax::ast::BoGroupedBridiTailSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    let mut entries = vec![TreeEntry {
        label: Some("first"),
        value: legacy_as_generated_simple_bridi_tail_tree_value(
            bridi_tail.first.as_ref(),
            source,
            options,
        ),
    }];
    if let Some(bo_continuation) = &bridi_tail.bo_continuation {
        entries.push(TreeEntry {
            label: Some("bo_continuation"),
            value: legacy_as_generated_bound_bridi_tail_connection_tree_value(
                bo_continuation.as_ref(),
                source,
                options,
            ),
        });
    }
    TreeValue::Node(TreeNode {
        constructor: "BoGroupedBridiTail",
        entries,
    })
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_grouped_bridi_tail_connection_tree_value(
    continuation: &jbotci_syntax::ast::GroupedBridiTailConnectionSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    let mut entries = vec![TreeEntry {
        label: Some("connective"),
        value: required_legacy_syntax_subtree_value(&continuation.connective, source, options),
    }];
    if let Some(tense_modal) = &continuation.tense_modal {
        entries.push(TreeEntry {
            label: Some("tense_modal"),
            value: legacy_as_generated_tense_modal_tree_value(
                tense_modal.as_ref(),
                source,
                options,
            ),
        });
    }
    entries.push(TreeEntry {
        label: Some("ke"),
        value: required_legacy_syntax_subtree_value(&continuation.ke, source, options),
    });
    entries.push(TreeEntry {
        label: Some("bridi_tail"),
        value: legacy_as_generated_bridi_tail_tree_value(
            continuation.bridi_tail.as_ref(),
            source,
            options,
        ),
    });
    if let Some(kehe) = &continuation.kehe {
        entries.push(TreeEntry {
            label: Some("kehe"),
            value: required_legacy_syntax_subtree_value(kehe.as_ref(), source, options),
        });
    }
    if let Some(entry) = labelled_tree_collection_entry_from_values(
        "tail_terms",
        continuation
            .tail_terms
            .iter()
            .map(|term| legacy_as_generated_term_tree_value(term, source, options))
            .collect(),
    ) {
        entries.push(entry);
    }
    if let Some(vau) = &continuation.vau {
        entries.extend(legacy_token_field_entries(
            "vau",
            vau.as_ref(),
            source,
            options,
        ));
    }
    if let Some(entry) = labelled_tree_collection_entry_from_values(
        "free_modifiers",
        continuation
            .free_modifiers
            .iter()
            .map(|free_modifier| {
                legacy_as_generated_free_modifier_tree_value(free_modifier, source, options)
            })
            .collect(),
    ) {
        entries.push(entry);
    }
    TreeValue::Node(TreeNode {
        constructor: "GroupedBridiTailConnection",
        entries,
    })
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_bridi_tail_connection_tree_value(
    continuation: &jbotci_syntax::ast::BridiTailConnectionSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    let mut entries = vec![TreeEntry {
        label: Some("connective"),
        value: required_legacy_syntax_subtree_value(&continuation.connective, source, options),
    }];
    if let Some(tense_modal) = &continuation.tense_modal {
        entries.push(TreeEntry {
            label: Some("tense_modal"),
            value: legacy_as_generated_tense_modal_tree_value(
                tense_modal.as_ref(),
                source,
                options,
            ),
        });
    }
    if let Some(cu) = &continuation.cu {
        entries.push(TreeEntry {
            label: Some("cu"),
            value: required_legacy_syntax_subtree_value(cu.as_ref(), source, options),
        });
    }
    entries.push(TreeEntry {
        label: Some("bridi_tail"),
        value: legacy_as_generated_bo_grouped_bridi_tail_tree_value(
            continuation.bridi_tail.as_ref(),
            source,
            options,
        ),
    });
    if let Some(entry) = labelled_tree_collection_entry_from_values(
        "tail_terms",
        continuation
            .tail_terms
            .iter()
            .map(|term| legacy_as_generated_term_tree_value(term, source, options))
            .collect(),
    ) {
        entries.push(entry);
    }
    if let Some(vau) = &continuation.vau {
        entries.extend(legacy_token_field_entries(
            "vau",
            vau.as_ref(),
            source,
            options,
        ));
    }
    if let Some(entry) = labelled_tree_collection_entry_from_values(
        "free_modifiers",
        continuation
            .free_modifiers
            .iter()
            .map(|free_modifier| {
                legacy_as_generated_free_modifier_tree_value(free_modifier, source, options)
            })
            .collect(),
    ) {
        entries.push(entry);
    }
    TreeValue::Node(TreeNode {
        constructor: "BridiTailConnection",
        entries,
    })
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_bound_bridi_tail_connection_tree_value(
    continuation: &jbotci_syntax::ast::BoundBridiTailConnectionSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    let mut entries = vec![TreeEntry {
        label: Some("connective"),
        value: required_legacy_syntax_subtree_value(&continuation.connective, source, options),
    }];
    if let Some(tense_modal) = &continuation.tense_modal {
        entries.push(TreeEntry {
            label: Some("tense_modal"),
            value: legacy_as_generated_tense_modal_tree_value(
                tense_modal.as_ref(),
                source,
                options,
            ),
        });
    }
    entries.push(TreeEntry {
        label: Some("bo"),
        value: required_legacy_syntax_subtree_value(&continuation.bo, source, options),
    });
    if let Some(cu) = &continuation.cu {
        entries.push(TreeEntry {
            label: Some("cu"),
            value: required_legacy_syntax_subtree_value(cu.as_ref(), source, options),
        });
    }
    entries.push(TreeEntry {
        label: Some("bridi_tail"),
        value: legacy_as_generated_bo_grouped_bridi_tail_tree_value(
            continuation.bridi_tail.as_ref(),
            source,
            options,
        ),
    });
    if let Some(entry) = labelled_tree_collection_entry_from_values(
        "tail_terms",
        continuation
            .tail_terms
            .iter()
            .map(|term| legacy_as_generated_term_tree_value(term, source, options))
            .collect(),
    ) {
        entries.push(entry);
    }
    if let Some(vau) = &continuation.vau {
        entries.extend(legacy_token_field_entries(
            "vau",
            vau.as_ref(),
            source,
            options,
        ));
    }
    if let Some(entry) = labelled_tree_collection_entry_from_values(
        "free_modifiers",
        continuation
            .free_modifiers
            .iter()
            .map(|free_modifier| {
                legacy_as_generated_free_modifier_tree_value(free_modifier, source, options)
            })
            .collect(),
    ) {
        entries.push(entry);
    }
    TreeValue::Node(TreeNode {
        constructor: "BoundBridiTailConnection",
        entries,
    })
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_simple_bridi_tail_tree_value(
    bridi_tail: &jbotci_syntax::ast::SimpleBridiTailSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    match bridi_tail.as_data() {
        bityzba::data!(jbotci_syntax::ast::SimpleBridiTailSyntax::SelbriBridiTail {
            selbri,
            terms,
            vau,
            free_modifiers,
        }) => {
            let mut entries = vec![TreeEntry {
                label: Some("selbri"),
                value: legacy_as_generated_selbri_tree_value(selbri.as_ref(), source, options),
            }];
            if let Some(entry) = labelled_tree_collection_entry_from_values(
                "terms",
                terms
                    .iter()
                    .map(|term| legacy_as_generated_term_tree_value(term, source, options))
                    .collect(),
            ) {
                entries.push(entry);
            }
            if let Some(vau) = vau {
                entries.extend(legacy_token_field_entries(
                    "vau",
                    vau.as_ref(),
                    source,
                    options,
                ));
            }
            if let Some(entry) = labelled_tree_collection_entry_from_values(
                "free_modifiers",
                free_modifiers
                    .iter()
                    .map(|free_modifier| {
                        legacy_as_generated_free_modifier_tree_value(free_modifier, source, options)
                    })
                    .collect(),
            ) {
                entries.push(entry);
            }
            TreeValue::Node(TreeNode {
                constructor: "SelbriBridiTail",
                entries,
            })
        }
        bityzba::data!(
            jbotci_syntax::ast::SimpleBridiTailSyntax::ForethoughtBridiTailConnection(connection)
        ) => legacy_as_generated_forethought_bridi_connection_tree_value(
            connection.as_ref(),
            source,
            options,
        ),
        _ => required_legacy_syntax_subtree_value(bridi_tail, source, options),
    }
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_forethought_bridi_connection_tree_value(
    connection: &jbotci_syntax::ast::ForethoughtBridiConnectionSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    match connection.as_data() {
        bityzba::data!(
            jbotci_syntax::ast::ForethoughtBridiConnectionSyntax::BridiConnection {
                gek,
                first,
                gik,
                second,
                gihi,
                tail_terms,
                vau,
                free_modifiers,
            }
        ) => {
            let mut entries = vec![
                TreeEntry {
                    label: Some("gek"),
                    value: legacy_as_generated_connective_tree_value(gek, source, options),
                },
                TreeEntry {
                    label: Some("first"),
                    value: legacy_as_generated_subbridi_tree_value(first.as_ref(), source, options),
                },
                TreeEntry {
                    label: Some("gik"),
                    value: required_legacy_syntax_subtree_value(gik, source, options),
                },
                TreeEntry {
                    label: Some("second"),
                    value: legacy_as_generated_subbridi_tree_value(
                        second.as_ref(),
                        source,
                        options,
                    ),
                },
            ];
            if let Some(gihi) = gihi {
                entries.push(TreeEntry {
                    label: Some("gihi"),
                    value: generated_token_tree_value(gihi, source, options),
                });
            }
            if let Some(entry) = labelled_tree_collection_entry_from_values(
                "tail_terms",
                tail_terms
                    .iter()
                    .map(|term| legacy_as_generated_term_tree_value(term, source, options))
                    .collect(),
            ) {
                entries.push(entry);
            }
            if let Some(vau) = vau {
                entries.push(TreeEntry {
                    label: Some("vau"),
                    value: required_legacy_syntax_subtree_value(vau.as_ref(), source, options),
                });
            }
            if let Some(entry) = labelled_tree_collection_entry_from_values(
                "free_modifiers",
                free_modifiers
                    .iter()
                    .map(|free_modifier| {
                        legacy_as_generated_free_modifier_tree_value(free_modifier, source, options)
                    })
                    .collect(),
            ) {
                entries.push(entry);
            }
            TreeValue::Node(TreeNode {
                constructor: "BridiConnection",
                entries,
            })
        }
        bityzba::data!(
            jbotci_syntax::ast::ForethoughtBridiConnectionSyntax::GroupedBridiConnection {
                tense_modal,
                ke,
                inner,
                kehe,
            }
        ) => {
            let mut entries = Vec::new();
            if let Some(tense_modal) = tense_modal {
                entries.push(TreeEntry {
                    label: Some("tense_modal"),
                    value: legacy_as_generated_tense_modal_tree_value(
                        tense_modal.as_ref(),
                        source,
                        options,
                    ),
                });
            }
            entries.push(TreeEntry {
                label: Some("ke"),
                value: required_legacy_syntax_subtree_value(ke, source, options),
            });
            entries.push(TreeEntry {
                label: Some("inner"),
                value: legacy_as_generated_forethought_bridi_connection_tree_value(
                    inner.as_ref(),
                    source,
                    options,
                ),
            });
            if let Some(kehe) = kehe {
                entries.push(TreeEntry {
                    label: Some("kehe"),
                    value: required_legacy_syntax_subtree_value(kehe.as_ref(), source, options),
                });
            }
            TreeValue::Node(TreeNode {
                constructor: "GroupedBridiConnection",
                entries,
            })
        }
        bityzba::data!(
            jbotci_syntax::ast::ForethoughtBridiConnectionSyntax::NegatedBridiConnection {
                na,
                inner,
            }
        ) => TreeValue::Node(TreeNode {
            constructor: "NegatedBridiConnection",
            entries: vec![
                TreeEntry {
                    label: Some("na"),
                    value: required_legacy_syntax_subtree_value(na, source, options),
                },
                TreeEntry {
                    label: Some("inner"),
                    value: legacy_as_generated_forethought_bridi_connection_tree_value(
                        inner.as_ref(),
                        source,
                        options,
                    ),
                },
            ],
        }),
    }
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_connective_tree_value(
    connective: &jbotci_syntax::ast::ConnectiveSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    match connective.as_data() {
        bityzba::data!(jbotci_syntax::ast::ConnectiveSyntax::NonLogical {
            se: None,
            nahe: None,
            na: None,
            cmavo,
            nai: None,
        })
        | bityzba::data!(jbotci_syntax::ast::ConnectiveSyntax::Selbri {
            se: None,
            nahe: None,
            na: None,
            cmavo,
            nai: None,
        })
        | bityzba::data!(jbotci_syntax::ast::ConnectiveSyntax::Interval {
            se: None,
            nahe: None,
            na: None,
            cmavo,
            nai: None,
        }) => {
            if let Some(value) = legacy_as_generated_joik_jek_gi_forethought_connective_tree_value(
                &cmavo.value,
                source,
                options,
            ) {
                return value;
            }
        }
        _ => {}
    }
    if let bityzba::data!(jbotci_syntax::ast::ConnectiveSyntax::Forethought {
        se: None,
        nahe: None,
        na: None,
        cmavo,
        nai: None,
    }) = connective.as_data()
    {
        let words = &cmavo.value;
        if let Some(value) = legacy_as_generated_joik_jek_gi_forethought_connective_tree_value(
            words, source, options,
        ) {
            return value;
        }
        if let Some(value) =
            legacy_as_generated_modal_gi_forethought_connective_tree_value(words, source, options)
        {
            return value;
        }
        if (words.len() == 2 || words.len() == 3)
            && words[0].is_cmavo(Cmavo::Gi)
            && words.get(2).is_none_or(|word| word.is_cmavo(Cmavo::Bo))
        {
            let mut entries = vec![
                TreeEntry {
                    label: Some("gi"),
                    value: generated_token_tree_value(&words[0], source, options),
                },
                TreeEntry {
                    label: Some("tail"),
                    value: generated_token_tree_value(&words[1], source, options),
                },
            ];
            if let Some(bo) = words.get(2) {
                entries.push(TreeEntry {
                    label: Some("bo"),
                    value: generated_token_tree_value(bo, source, options),
                });
            }
            return TreeValue::Node(TreeNode {
                constructor: "ZantufaInitialGiForethoughtConnective",
                entries,
            });
        }
    }

    if let bityzba::data!(jbotci_syntax::ast::ConnectiveSyntax::Afterthought {
        se: None,
        nahe: None,
        na: None,
        cmavo,
        nai: None,
    }) = connective.as_data()
        && !cmavo.free_modifiers.is_empty()
    {
        let mut entries = cmavo
            .value
            .iter()
            .map(|token| TreeEntry {
                label: None,
                value: generated_token_tree_value(token, source, options),
            })
            .collect::<Vec<_>>();
        if let Some(entry) = labelled_tree_collection_entry_from_values(
            "free_modifiers",
            cmavo
                .free_modifiers
                .iter()
                .map(|free_modifier| {
                    legacy_as_generated_free_modifier_tree_value(free_modifier, source, options)
                })
                .collect(),
        ) {
            entries.push(entry);
        }
        return TreeValue::Node(TreeNode {
            constructor: "Afterthought",
            entries,
        });
    }

    required_legacy_syntax_subtree_value(connective, source, options)
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_joik_jek_gi_forethought_connective_tree_value(
    words: &[Token],
    source: &str,
    options: TreeRenderOptions,
) -> Option<TreeValue> {
    let (gi, bo) = match words {
        [.., gi] if gi.is_cmavo(Cmavo::Gi) => (gi, None),
        [.., gi, bo] if gi.is_cmavo(Cmavo::Gi) && bo.is_cmavo(Cmavo::Bo) => (gi, Some(bo)),
        _ => return None,
    };
    let prefix_len = words.len() - usize::from(bo.is_some()) - 1;
    let prefix = &words[..prefix_len];
    if let Some(value) = legacy_as_generated_jek_gi_forethought_connective_tree_value(
        prefix, gi, bo, source, options,
    ) {
        return Some(value);
    }

    let prefix_tokens = prefix.iter().collect::<Vec<_>>();
    let mut index = 0;
    let connective = legacy_as_generated_joik_tag_connective_tree_value(
        &prefix_tokens,
        &mut index,
        source,
        options,
    )?;
    if index != prefix_tokens.len() {
        return None;
    }
    let mut entries = vec![
        TreeEntry {
            label: Some("connective"),
            value: connective,
        },
        TreeEntry {
            label: Some("gi"),
            value: generated_token_tree_value(gi, source, options),
        },
    ];
    if let Some(bo) = bo {
        entries.push(TreeEntry {
            label: Some("bo"),
            value: generated_token_tree_value(bo, source, options),
        });
    }
    Some(TreeValue::Node(TreeNode {
        constructor: "JoikJekGiForethoughtConnective",
        entries,
    }))
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_jek_gi_forethought_connective_tree_value(
    prefix: &[Token],
    gi: &Token,
    bo: Option<&Token>,
    source: &str,
    options: TreeRenderOptions,
) -> Option<TreeValue> {
    let mut index = 0;
    let na = prefix
        .get(index)
        .filter(|token| token.is_selmaho(Selmaho::Na));
    if na.is_some() {
        index += 1;
    }
    let se = prefix
        .get(index)
        .filter(|token| token.is_selmaho(Selmaho::Se));
    if se.is_some() {
        index += 1;
    }
    let ja = prefix
        .get(index)
        .filter(|token| token.is_selmaho(Selmaho::Ja))?;
    index += 1;
    let nai = prefix.get(index).filter(|token| token.is_cmavo(Cmavo::Nai));
    if nai.is_some() {
        index += 1;
    }
    if index != prefix.len() {
        return None;
    }
    let mut entries = Vec::new();
    if let Some(na) = na {
        entries.push(TreeEntry {
            label: Some("na"),
            value: generated_token_tree_value(na, source, options),
        });
    }
    if let Some(se) = se {
        entries.push(TreeEntry {
            label: Some("se"),
            value: generated_token_tree_value(se, source, options),
        });
    }
    entries.push(TreeEntry {
        label: Some("ja"),
        value: generated_token_tree_value(ja, source, options),
    });
    if let Some(nai) = nai {
        entries.push(TreeEntry {
            label: Some("nai"),
            value: generated_token_tree_value(nai, source, options),
        });
    }
    entries.push(TreeEntry {
        label: Some("gi"),
        value: generated_token_tree_value(gi, source, options),
    });
    if let Some(bo) = bo {
        entries.push(TreeEntry {
            label: Some("bo"),
            value: generated_token_tree_value(bo, source, options),
        });
    }
    Some(TreeValue::Node(TreeNode {
        constructor: "JekGiForethoughtConnective",
        entries,
    }))
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_grouped_sumti_connective_entries(
    connective: &jbotci_syntax::ast::ConnectiveSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> Vec<TreeEntry> {
    let Some(cmavo) = legacy_unsuffixed_argument_connective_word_run(connective) else {
        return vec![TreeEntry {
            label: Some("connective"),
            value: required_legacy_syntax_subtree_value(connective, source, options),
        }];
    };
    let Some((connective_token, modal_tokens)) = cmavo.value.split_first() else {
        return vec![TreeEntry {
            label: Some("connective"),
            value: required_legacy_syntax_subtree_value(connective, source, options),
        }];
    };
    let mut entries = vec![TreeEntry {
        label: Some("connective"),
        value: generated_token_tree_value(connective_token, source, options),
    }];
    if !modal_tokens.is_empty() {
        let Some(tense_modal) = legacy_as_generated_flat_or_composite_tense_words_tree_value(
            modal_tokens,
            source,
            options,
        ) else {
            return vec![TreeEntry {
                label: Some("connective"),
                value: required_legacy_syntax_subtree_value(connective, source, options),
            }];
        };
        entries.push(TreeEntry {
            label: Some("tense_modal"),
            value: tense_modal,
        });
    }
    entries
}

#[requires(true)]
#[ensures(true)]
fn legacy_unsuffixed_argument_connective_word_run(
    connective: &jbotci_syntax::ast::ConnectiveSyntax,
) -> Option<&WithFreeModifiers<Vec<Token>>> {
    match connective.as_data() {
        bityzba::data!(jbotci_syntax::ast::ConnectiveSyntax::Afterthought {
            se: None,
            nahe: None,
            na: None,
            cmavo,
            nai: None,
        })
        | bityzba::data!(jbotci_syntax::ast::ConnectiveSyntax::NonLogical {
            se: None,
            nahe: None,
            na: None,
            cmavo,
            nai: None,
        })
        | bityzba::data!(jbotci_syntax::ast::ConnectiveSyntax::Interval {
            se: None,
            nahe: None,
            na: None,
            cmavo,
            nai: None,
        }) if cmavo.free_modifiers.is_empty() => Some(cmavo.as_ref()),
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_modal_gi_forethought_connective_tree_value(
    words: &[Token],
    source: &str,
    options: TreeRenderOptions,
) -> Option<TreeValue> {
    let (gi, bo) = match words {
        [.., gi] if gi.is_cmavo(Cmavo::Gi) => (gi, None),
        [.., gi, bo] if gi.is_cmavo(Cmavo::Gi) && bo.is_cmavo(Cmavo::Bo) => (gi, Some(bo)),
        _ => return None,
    };
    let prefix_len = words.len() - usize::from(bo.is_some()) - 1;
    let tense_modal = legacy_as_generated_flat_or_composite_tense_words_tree_value(
        &words[..prefix_len],
        source,
        options,
    )?;
    let mut entries = vec![
        TreeEntry {
            label: Some("tense_modal"),
            value: tense_modal,
        },
        TreeEntry {
            label: Some("gi"),
            value: generated_token_tree_value(gi, source, options),
        },
    ];
    if let Some(bo) = bo {
        entries.push(TreeEntry {
            label: Some("bo"),
            value: generated_token_tree_value(bo, source, options),
        });
    }
    Some(TreeValue::Node(TreeNode {
        constructor: "ModalGiForethoughtConnective",
        entries,
    }))
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_flat_or_composite_tense_words_tree_value(
    words: &[Token],
    source: &str,
    options: TreeRenderOptions,
) -> Option<TreeValue> {
    if let Some(value) = legacy_as_generated_flat_modal_tense_tree_value(words, source, options) {
        return Some(value);
    }
    let tokens = words.iter().collect::<Vec<_>>();
    if let Some(value) =
        legacy_as_generated_connected_tense_modal_tree_value(&tokens, source, options)
    {
        return Some(value);
    }
    if let Some(value) =
        legacy_as_generated_time_tense_sequence_tree_value(&tokens, source, options)
    {
        return Some(value);
    }
    if let [ki] = words
        && ki.is_cmavo(Cmavo::Ki)
    {
        return Some(TreeValue::Node(TreeNode {
            constructor: "StickyTense",
            entries: vec![TreeEntry {
                label: Some("ki"),
                value: generated_token_tree_value(ki, source, options),
            }],
        }));
    }
    None
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_flat_modal_tense_tree_value(
    words: &[Token],
    source: &str,
    options: TreeRenderOptions,
) -> Option<TreeValue> {
    let mut index = 0;
    let nahe = words
        .get(index)
        .filter(|word| word.is_selmaho(Selmaho::Nahe));
    if nahe.is_some() {
        index += 1;
    }
    let se = words.get(index).filter(|word| word.is_selmaho(Selmaho::Se));
    if se.is_some() {
        index += 1;
    }
    let bai = words
        .get(index)
        .filter(|word| word.is_selmaho(Selmaho::Bai))?;
    index += 1;
    let nai = words.get(index).filter(|word| word.is_cmavo(Cmavo::Nai));
    if nai.is_some() {
        index += 1;
    }
    let ki = words.get(index).filter(|word| word.is_cmavo(Cmavo::Ki));
    if ki.is_some() {
        index += 1;
    }
    if index != words.len() {
        return None;
    }

    let mut entries = Vec::new();
    if let Some(nahe) = nahe {
        entries.push(TreeEntry {
            label: Some("nahe"),
            value: generated_token_tree_value(nahe, source, options),
        });
    }
    if let Some(se) = se {
        entries.push(TreeEntry {
            label: Some("se"),
            value: generated_token_tree_value(se, source, options),
        });
    }
    entries.push(TreeEntry {
        label: Some("bai"),
        value: generated_token_tree_value(bai, source, options),
    });
    if let Some(nai) = nai {
        entries.push(TreeEntry {
            label: Some("nai"),
            value: generated_token_tree_value(nai, source, options),
        });
    }
    if let Some(ki) = ki {
        entries.push(TreeEntry {
            label: Some("ki"),
            value: generated_token_tree_value(ki, source, options),
        });
    }
    Some(TreeValue::Node(TreeNode {
        constructor: "ModalTense",
        entries,
    }))
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_term_tree_value(
    term: &jbotci_syntax::ast::TermSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    match term.as_data() {
        bityzba::data!(jbotci_syntax::ast::TermSyntax::BoundTermConnection {
            leading_terms,
            bo_connective,
            tense_modal,
            bo,
            trailing_term,
        }) => {
            let Some(leading_term) = leading_terms.first() else {
                return required_legacy_syntax_subtree_value(term, source, options);
            };
            let mut entries = vec![TreeEntry {
                label: Some("leading_term"),
                value: legacy_as_generated_simple_term_tree_value(leading_term, source, options),
            }];
            if let Some(connective) = bo_connective {
                entries.push(TreeEntry {
                    label: Some("connective"),
                    value: required_legacy_syntax_subtree_value(
                        connective.as_ref(),
                        source,
                        options,
                    ),
                });
            }
            if let Some(tense_modal) = tense_modal {
                entries.push(TreeEntry {
                    label: Some("tense_modal"),
                    value: legacy_as_generated_tense_modal_tree_value(
                        tense_modal.as_ref(),
                        source,
                        options,
                    ),
                });
            }
            entries.push(TreeEntry {
                label: Some("bo"),
                value: required_legacy_syntax_subtree_value(bo, source, options),
            });
            entries.push(TreeEntry {
                label: Some("trailing_term"),
                value: legacy_as_generated_simple_term_tree_value(
                    trailing_term.as_ref(),
                    source,
                    options,
                ),
            });
            TreeValue::Node(TreeNode {
                constructor: "BoundTermConnection",
                entries,
            })
        }
        bityzba::data!(jbotci_syntax::ast::TermSyntax::TermConnection {
            leading_terms,
            connective,
            trailing_terms,
        }) => {
            let Some(first_term) = leading_terms.first() else {
                return required_legacy_syntax_subtree_value(term, source, options);
            };
            let mut entries = vec![TreeEntry {
                label: Some("leading_term"),
                value: legacy_as_generated_simple_term_tree_value(first_term, source, options),
            }];
            let continuations = leading_terms
                .iter()
                .skip(1)
                .map(|term| {
                    TreeValue::Collection(vec![
                        required_legacy_syntax_subtree_value(connective, source, options),
                        legacy_as_generated_simple_term_tree_value(term, source, options),
                    ])
                })
                .chain(trailing_terms.iter().map(|term| {
                    TreeValue::Collection(vec![
                        required_legacy_syntax_subtree_value(connective, source, options),
                        legacy_as_generated_simple_term_tree_value(term, source, options),
                    ])
                }))
                .collect::<Vec<_>>();
            if let Some(entry) =
                labelled_tree_collection_entry_from_values("continuations", continuations)
            {
                entries.push(entry);
            }
            TreeValue::Node(TreeNode {
                constructor: "ConnectedTerm",
                entries,
            })
        }
        bityzba::data!(jbotci_syntax::ast::TermSyntax::TermsetGroup { .. })
        | bityzba::data!(jbotci_syntax::ast::TermSyntax::TermsetConnection { .. }) => {
            legacy_as_generated_simple_term_tree_value(term, source, options)
        }
        _ => TreeValue::Node(TreeNode {
            constructor: "ConnectedTerm",
            entries: vec![TreeEntry {
                label: Some("leading_term"),
                value: legacy_as_generated_simple_term_tree_value(term, source, options),
            }],
        }),
    }
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_simple_term_tree_value(
    term: &jbotci_syntax::ast::TermSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    match term.as_data() {
        bityzba::data!(jbotci_syntax::ast::TermSyntax::Sumti(sumti)) => {
            if let Some((tense_modal, None, free_modifiers)) =
                legacy_elided_tense_tagged_sumti_parts(sumti.as_ref())
                && free_modifiers.is_empty()
                && let Some(tense_modal) =
                    legacy_as_generated_leading_term_tag_tense_modal_tree_value(
                        tense_modal,
                        source,
                        options,
                    )
            {
                return TreeValue::Node(TreeNode {
                    constructor: "TaggedSumtiBeforeTagTerm",
                    entries: vec![TreeEntry {
                        label: Some("tense_modal"),
                        value: tense_modal,
                    }],
                });
            }
            legacy_as_generated_sumti_tree_value(sumti.as_ref(), source, options)
        }
        bityzba::data!(jbotci_syntax::ast::TermSyntax::PlaceTaggedSumti { fa, sumti, ku }) => {
            let mut entries = legacy_token_field_entries("fa", fa, source, options);
            entries.push(TreeEntry {
                label: Some("sumti"),
                value: legacy_as_generated_tagged_or_elided_sumti_tree_value(
                    sumti.as_ref(),
                    source,
                    options,
                ),
            });
            if let Some(ku) = ku {
                entries.extend(legacy_token_field_entries("ku", ku, source, options));
            }
            TreeValue::Node(TreeNode {
                constructor: "PlaceTaggedSumti",
                entries,
            })
        }
        bityzba::data!(jbotci_syntax::ast::TermSyntax::JaiTaggedSumti { jai, tag, sumti }) => {
            let mut entries = legacy_token_field_entries("jai", jai, source, options);
            if let Some(tag) = tag {
                entries.push(TreeEntry {
                    label: Some("tag"),
                    value: legacy_as_generated_tense_modal_tree_value(
                        tag.as_ref(),
                        source,
                        options,
                    ),
                });
            }
            entries.push(TreeEntry {
                label: Some("sumti"),
                value: legacy_as_generated_tagged_or_elided_sumti_tree_value(
                    sumti.as_ref(),
                    source,
                    options,
                ),
            });
            TreeValue::Node(TreeNode {
                constructor: "JaiTaggedSumti",
                entries,
            })
        }
        bityzba::data!(jbotci_syntax::ast::TermSyntax::BridiNegation { na, na_ku }) => {
            let mut entries = vec![
                TreeEntry {
                    label: Some("na"),
                    value: generated_token_tree_value(na, source, options),
                },
                TreeEntry {
                    label: Some("na_ku"),
                    value: generated_token_tree_value(&na_ku.value, source, options),
                },
            ];
            if let Some(entry) = labelled_tree_collection_entry_from_values(
                "free_modifiers",
                na_ku
                    .free_modifiers
                    .iter()
                    .map(|free_modifier| {
                        legacy_as_generated_free_modifier_tree_value(free_modifier, source, options)
                    })
                    .collect(),
            ) {
                entries.push(entry);
            }
            TreeValue::Node(TreeNode {
                constructor: "BridiNegation",
                entries,
            })
        }
        bityzba::data!(jbotci_syntax::ast::TermSyntax::BareNegation(na)) => {
            let mut entries = vec![TreeEntry {
                label: None,
                value: generated_token_tree_value(&na.value, source, options),
            }];
            if let Some(entry) = labelled_tree_collection_entry_from_values(
                "free_modifiers",
                na.free_modifiers
                    .iter()
                    .map(|free_modifier| {
                        legacy_as_generated_free_modifier_tree_value(free_modifier, source, options)
                    })
                    .collect(),
            ) {
                entries.push(entry);
            }
            TreeValue::Node(TreeNode {
                constructor: "BareNegation",
                entries,
            })
        }
        bityzba::data!(jbotci_syntax::ast::TermSyntax::TaggedSumti { tense_modal, sumti }) => {
            let mut entries = Vec::new();
            let before_tag_tense_modal = tense_modal.as_ref().and_then(|tense_modal| {
                legacy_as_generated_leading_term_tag_tense_modal_tree_value(
                    tense_modal.as_ref(),
                    source,
                    options,
                )
            });
            if legacy_is_empty_elided_sumti(sumti.as_ref())
                && let Some(tense_modal) = before_tag_tense_modal
            {
                entries.push(TreeEntry {
                    label: Some("tense_modal"),
                    value: tense_modal,
                });
                return TreeValue::Node(TreeNode {
                    constructor: "TaggedSumtiBeforeTagTerm",
                    entries,
                });
            }
            if let Some(tense_modal) = tense_modal {
                entries.push(TreeEntry {
                    label: Some("tense_modal"),
                    value: legacy_as_generated_tense_modal_tree_value(
                        tense_modal.as_ref(),
                        source,
                        options,
                    ),
                });
            }
            entries.push(TreeEntry {
                label: Some("sumti"),
                value: legacy_as_generated_tagged_or_elided_sumti_tree_value(
                    sumti.as_ref(),
                    source,
                    options,
                ),
            });
            TreeValue::Node(TreeNode {
                constructor: "TaggedSumti",
                entries,
            })
        }
        bityzba::data!(jbotci_syntax::ast::TermSyntax::RelativeAdverbialTerm {
            noiha,
            tail_elements,
            selbri,
            relative_clauses,
            fehu,
        }) => {
            assert!(
                tail_elements.is_empty(),
                "legacy RelativeAdverbialTerm tail elements are not represented in generated adverbial term shape"
            );
            let mut entries = vec![TreeEntry {
                label: Some("noiha"),
                value: required_legacy_syntax_subtree_value(noiha, source, options),
            }];
            if let Some(selbri) = selbri {
                entries.push(TreeEntry {
                    label: Some("selbri"),
                    value: legacy_as_generated_selbri_tree_value(selbri.as_ref(), source, options),
                });
            }
            if let Some(entry) = labelled_tree_collection_entry_from_values(
                "relative_clauses",
                relative_clauses
                    .iter()
                    .map(|relative_clause| {
                        legacy_as_generated_relative_clause_tree_value(
                            relative_clause,
                            source,
                            options,
                        )
                    })
                    .collect(),
            ) {
                entries.push(entry);
            }
            if let Some(fehu) = fehu {
                entries.push(TreeEntry {
                    label: Some("fehu"),
                    value: required_legacy_syntax_subtree_value(fehu, source, options),
                });
            }
            TreeValue::Node(TreeNode {
                constructor: "RelativeAdverbialTerm",
                entries,
            })
        }
        bityzba::data!(jbotci_syntax::ast::TermSyntax::BridiVariableAdverbialTerm {
            poiha,
            tail_elements,
            selbri,
            relative_clauses,
            brigahi_ku,
        }) => {
            assert!(
                tail_elements.is_empty(),
                "legacy BridiVariableAdverbialTerm tail elements are not represented in generated adverbial term shape"
            );
            let mut entries = vec![TreeEntry {
                label: Some("poiha"),
                value: required_legacy_syntax_subtree_value(poiha, source, options),
            }];
            if let Some(selbri) = selbri {
                entries.push(TreeEntry {
                    label: Some("selbri"),
                    value: legacy_as_generated_selbri_tree_value(selbri.as_ref(), source, options),
                });
            }
            if let Some(entry) = labelled_tree_collection_entry_from_values(
                "relative_clauses",
                relative_clauses
                    .iter()
                    .map(|relative_clause| {
                        legacy_as_generated_relative_clause_tree_value(
                            relative_clause,
                            source,
                            options,
                        )
                    })
                    .collect(),
            ) {
                entries.push(entry);
            }
            entries.push(TreeEntry {
                label: Some("brigahi_ku"),
                value: required_legacy_syntax_subtree_value(brigahi_ku, source, options),
            });
            TreeValue::Node(TreeNode {
                constructor: "BridiVariableAdverbialTerm",
                entries,
            })
        }
        bityzba::data!(jbotci_syntax::ast::TermSyntax::AdHocBridiAdverbialTerm {
            fihoi,
            subbridi,
            fihau,
        }) => {
            let mut entries = vec![
                TreeEntry {
                    label: Some("fihoi"),
                    value: required_legacy_syntax_subtree_value(fihoi, source, options),
                },
                TreeEntry {
                    label: Some("subbridi"),
                    value: legacy_as_generated_subbridi_tree_value(
                        subbridi.as_ref(),
                        source,
                        options,
                    ),
                },
            ];
            if let Some(fihau) = fihau {
                entries.push(TreeEntry {
                    label: Some("fihau"),
                    value: required_legacy_syntax_subtree_value(fihau, source, options),
                });
            }
            TreeValue::Node(TreeNode {
                constructor: "AdHocBridiAdverbialTerm",
                entries,
            })
        }
        bityzba::data!(
            jbotci_syntax::ast::TermSyntax::ReciprocalBridiAdverbialTerm {
                soi,
                subbridi,
                sehu,
            }
        ) => {
            let mut entries = vec![
                TreeEntry {
                    label: Some("soi"),
                    value: required_legacy_syntax_subtree_value(soi, source, options),
                },
                TreeEntry {
                    label: Some("subbridi"),
                    value: legacy_as_generated_subbridi_tree_value(
                        subbridi.as_ref(),
                        source,
                        options,
                    ),
                },
            ];
            if let Some(sehu) = sehu {
                entries.push(TreeEntry {
                    label: Some("sehu"),
                    value: required_legacy_syntax_subtree_value(sehu, source, options),
                });
            }
            TreeValue::Node(TreeNode {
                constructor: "ReciprocalBridiAdverbialTerm",
                entries,
            })
        }
        bityzba::data!(jbotci_syntax::ast::TermSyntax::Termset {
            nuhi,
            termset,
            nuhu,
        }) => {
            let (constructor, opening_label, closing_label) = if nuhi.value.is_cmavo(Cmavo::Ke) {
                ("KeTermset", "ke", "kehe")
            } else {
                ("NuhiTermset", "nuhi", "nuhu")
            };
            let mut entries = legacy_token_field_entries(opening_label, nuhi, source, options);
            if let Some(entry) = labelled_tree_collection_entry_from_values(
                "termset",
                termset
                    .iter()
                    .map(|term| legacy_as_generated_term_tree_value(term, source, options))
                    .collect(),
            ) {
                entries.push(entry);
            }
            if let Some(nuhu) = nuhu {
                entries.extend(legacy_token_field_entries(
                    closing_label,
                    nuhu,
                    source,
                    options,
                ));
            }
            TreeValue::Node(TreeNode {
                constructor,
                entries,
            })
        }
        bityzba::data!(
            jbotci_syntax::ast::TermSyntax::ForethoughtTermsetConnection {
                m_nuhi,
                gek,
                terms,
                nuhu,
                gik,
                gik_terms,
                gihi,
                gik_nuhu,
            }
        ) => {
            let mut entries = Vec::new();
            if let Some(m_nuhi) = m_nuhi {
                entries.extend(legacy_token_field_entries(
                    "m_nuhi", m_nuhi, source, options,
                ));
            }
            entries.push(TreeEntry {
                label: Some("gek"),
                value: legacy_as_generated_connective_tree_value(gek, source, options),
            });
            if let Some(entry) = labelled_tree_collection_entry_from_values(
                "terms",
                terms
                    .iter()
                    .map(|term| legacy_as_generated_term_tree_value(term, source, options))
                    .collect(),
            ) {
                entries.push(entry);
            }
            if let Some(nuhu) = nuhu {
                entries.extend(legacy_token_field_entries("nuhu", nuhu, source, options));
            }
            entries.push(TreeEntry {
                label: Some("gik"),
                value: legacy_as_generated_connective_tree_value(gik, source, options),
            });
            if let Some(entry) = labelled_tree_collection_entry_from_values(
                "gik_terms",
                gik_terms
                    .iter()
                    .map(|term| legacy_as_generated_term_tree_value(term, source, options))
                    .collect(),
            ) {
                entries.push(entry);
            }
            if let Some(gihi) = gihi {
                entries.push(TreeEntry {
                    label: Some("gihi"),
                    value: generated_token_tree_value(gihi, source, options),
                });
            }
            if let Some(gik_nuhu) = gik_nuhu {
                entries.extend(legacy_token_field_entries(
                    "gik_nuhu", gik_nuhu, source, options,
                ));
            }
            TreeValue::Node(TreeNode {
                constructor: "ForethoughtTermsetConnection",
                entries,
            })
        }
        bityzba::data!(jbotci_syntax::ast::TermSyntax::TermsetGroup {
            leading_terms,
            cehe,
            trailing_terms,
        }) => {
            let Some(first_term) = leading_terms.first() else {
                return required_legacy_syntax_subtree_value(term, source, options);
            };
            let mut entries = vec![TreeEntry {
                label: Some("leading_term"),
                value: legacy_as_generated_simple_term_tree_value(first_term, source, options),
            }];
            let continuations = leading_terms
                .iter()
                .skip(1)
                .map(|term| {
                    TreeValue::Collection(vec![
                        required_legacy_syntax_subtree_value(cehe, source, options),
                        legacy_as_generated_simple_term_tree_value(term, source, options),
                    ])
                })
                .chain(trailing_terms.iter().map(|term| {
                    TreeValue::Collection(vec![
                        required_legacy_syntax_subtree_value(cehe, source, options),
                        legacy_as_generated_simple_term_tree_value(term, source, options),
                    ])
                }))
                .collect::<Vec<_>>();
            if let Some(entry) =
                labelled_tree_collection_entry_from_values("continuations", continuations)
            {
                entries.push(entry);
            }
            TreeValue::Node(TreeNode {
                constructor: "TermsetGroup",
                entries,
            })
        }
        bityzba::data!(jbotci_syntax::ast::TermSyntax::TermsetConnection {
            leading_terms,
            pehe,
            connective,
            trailing_terms,
        }) => {
            let Some(first_term) = leading_terms.first() else {
                return required_legacy_syntax_subtree_value(term, source, options);
            };
            let mut entries = vec![TreeEntry {
                label: Some("leading_term"),
                value: legacy_as_generated_simple_term_tree_value(first_term, source, options),
            }];
            let continuations = leading_terms
                .iter()
                .skip(1)
                .map(|term| {
                    TreeValue::Collection(vec![
                        TreeValue::Collection(vec![
                            required_legacy_syntax_subtree_value(pehe, source, options),
                            legacy_as_generated_connective_tree_value(connective, source, options),
                        ]),
                        legacy_as_generated_simple_term_tree_value(term, source, options),
                    ])
                })
                .chain(trailing_terms.iter().map(|term| {
                    TreeValue::Collection(vec![
                        TreeValue::Collection(vec![
                            required_legacy_syntax_subtree_value(pehe, source, options),
                            legacy_as_generated_connective_tree_value(connective, source, options),
                        ]),
                        legacy_as_generated_simple_term_tree_value(term, source, options),
                    ])
                }))
                .collect::<Vec<_>>();
            if let Some(entry) =
                labelled_tree_collection_entry_from_values("continuations", continuations)
            {
                entries.push(entry);
            }
            TreeValue::Node(TreeNode {
                constructor: "PeheTermsetConnection",
                entries,
            })
        }
        _ => required_legacy_syntax_subtree_value(term, source, options),
    }
}

#[requires(true)]
#[ensures(true)]
fn legacy_is_empty_elided_sumti(sumti: &jbotci_syntax::ast::SumtiSyntax) -> bool {
    matches!(
        sumti.as_data(),
        bityzba::data!(jbotci_syntax::ast::SumtiSyntax::ElidedSumti {
            tag: None,
            maybe_ku: None,
            free_modifiers,
        }) if free_modifiers.is_empty()
    )
}

#[requires(true)]
#[ensures(true)]
fn legacy_tense_modal_has_following_tense_modal(
    tense_modal: &jbotci_syntax::ast::TenseModalSyntax,
) -> bool {
    let mut last = None;
    tense_modal.visit_words(&mut |token| {
        last = Some(token.clone());
    });
    let Some(last) = last else {
        return false;
    };
    let next = legacy_next_tree_token_after(&last);
    next.as_ref()
        .is_some_and(legacy_token_can_start_tense_modal)
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_free_modifier_tree_value(
    free_modifier: &jbotci_syntax::ast::FreeModifierSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    match free_modifier.as_data() {
        bityzba::data!(
            jbotci_syntax::ast::FreeModifierSyntax::MetalinguisticBridi {
                sei,
                terms,
                cu,
                selbri,
                sehu,
            }
        ) => {
            let mut entries = legacy_token_field_entries("sei", sei, source, options);
            if let Some(entry) = labelled_tree_collection_entry_from_values(
                "terms",
                terms
                    .iter()
                    .map(|term| legacy_as_generated_term_tree_value(term, source, options))
                    .collect(),
            ) {
                entries.push(entry);
            }
            if let Some(cu) = cu {
                entries.extend(legacy_token_field_entries("cu", cu, source, options));
            }
            entries.push(TreeEntry {
                label: Some("selbri"),
                value: legacy_as_generated_selbri_tree_value(selbri.as_ref(), source, options),
            });
            if let Some(sehu) = sehu {
                entries.extend(legacy_token_field_entries("sehu", sehu, source, options));
            }
            legacy_as_generated_free_modifier_variant_tree_value(
                "SeiFreeModifier",
                "sei_free_modifier",
                entries,
            )
        }
        bityzba::data!(jbotci_syntax::ast::FreeModifierSyntax::ReciprocalSumti {
            soi,
            leading_sumti,
            trailing_sumti,
            sehu,
        }) => {
            let mut entries = legacy_token_field_entries("soi", soi, source, options);
            entries.push(TreeEntry {
                label: Some("leading_sumti"),
                value: legacy_as_generated_sumti_tree_value(
                    leading_sumti.as_ref(),
                    source,
                    options,
                ),
            });
            if let Some(trailing_sumti) = trailing_sumti {
                entries.push(TreeEntry {
                    label: Some("trailing_sumti"),
                    value: legacy_as_generated_sumti_tree_value(
                        trailing_sumti.as_ref(),
                        source,
                        options,
                    ),
                });
            }
            if let Some(sehu) = sehu {
                entries.extend(legacy_token_field_entries("sehu", sehu, source, options));
            }
            legacy_as_generated_free_modifier_variant_tree_value(
                "SoiFreeModifier",
                "soi_free_modifier",
                entries,
            )
        }
        bityzba::data!(jbotci_syntax::ast::FreeModifierSyntax::Vocative {
            vocative_markers,
            sumti,
            dohu,
        }) => {
            let mut entries = vec![TreeEntry {
                label: Some("vocative_markers"),
                value: legacy_as_generated_vocative_markers_tree_value(
                    vocative_markers,
                    source,
                    options,
                ),
            }];
            if let Some(entry) = labelled_tree_collection_entry_from_values(
                "free_modifiers",
                vocative_markers
                    .free_modifiers
                    .iter()
                    .map(|free_modifier| {
                        legacy_as_generated_free_modifier_tree_value(free_modifier, source, options)
                    })
                    .collect(),
            ) {
                entries.push(entry);
            }
            if let Some(sumti) = sumti {
                entries.push(TreeEntry {
                    label: Some("sumti"),
                    value: legacy_as_generated_vocative_argument_tree_value(
                        sumti.as_ref(),
                        source,
                        options,
                    ),
                });
            }
            if let Some(dohu) = dohu {
                entries.extend(legacy_token_field_entries("dohu", dohu, source, options));
            }
            legacy_as_generated_free_modifier_variant_tree_value(
                "VocativeFreeModifier",
                "vocative_free_modifier",
                entries,
            )
        }
        bityzba::data!(jbotci_syntax::ast::FreeModifierSyntax::ParentheticalText {
            to,
            text,
            toi,
        }) => {
            let mut entries = legacy_token_field_entries("to", to, source, options);
            entries.push(TreeEntry {
                label: Some("text"),
                value: legacy_as_generated_text_child_tree_value(text.as_ref(), source, options),
            });
            if let Some(toi) = toi {
                entries.extend(legacy_token_field_entries("toi", toi, source, options));
            }
            legacy_as_generated_free_modifier_variant_tree_value(
                "ParentheticalText",
                "parenthetical_text",
                entries,
            )
        }
        bityzba::data!(jbotci_syntax::ast::FreeModifierSyntax::Subscript { xi, expression }) => {
            let mut entries = legacy_token_field_entries("xi", xi, source, options);
            entries.push(TreeEntry {
                label: Some("expression"),
                value: legacy_as_generated_subscript_expression_tree_value(
                    expression.as_ref(),
                    source,
                    options,
                ),
            });
            legacy_as_generated_free_modifier_variant_tree_value(
                "XiFreeModifier",
                "xi_free_modifier",
                entries,
            )
        }
        bityzba::data!(jbotci_syntax::ast::FreeModifierSyntax::UtteranceOrdinal {
            number,
            mai,
        }) => {
            let mut entries = vec![
                TreeEntry {
                    label: Some("number"),
                    value: legacy_word_run_tree_value(number, source, options),
                },
                TreeEntry {
                    label: Some("mai"),
                    value: generated_token_tree_value(&mai.value, source, options),
                },
            ];
            if let Some(entry) = labelled_tree_collection_entry_from_values(
                "free_modifiers",
                mai.free_modifiers
                    .iter()
                    .map(|free_modifier| {
                        legacy_as_generated_free_modifier_tree_value(free_modifier, source, options)
                    })
                    .collect(),
            ) {
                entries.push(entry);
            }
            legacy_as_generated_free_modifier_variant_tree_value(
                "MaiFreeModifier",
                "mai_free_modifier",
                entries,
            )
        }
        bityzba::data!(jbotci_syntax::ast::FreeModifierSyntax::TextReplacement {
            lohai,
            old_words,
            sahai,
            new_words,
            lehai,
        }) => legacy_as_generated_text_replacement_free_modifier_tree_value(
            lohai.as_ref(),
            old_words,
            sahai.as_ref(),
            new_words,
            lehai,
            source,
            options,
        ),
    }
}

#[requires(!constructor.is_empty() && !label.is_empty())]
#[ensures(true)]
fn legacy_as_generated_free_modifier_variant_tree_value(
    constructor: &'static str,
    label: &'static str,
    entries: Vec<TreeEntry>,
) -> TreeValue {
    let inner = TreeValue::Node(TreeNode {
        constructor,
        entries,
    });
    TreeValue::Node(TreeNode {
        constructor,
        entries: vec![TreeEntry {
            label: Some(label),
            value: inner,
        }],
    })
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_text_replacement_free_modifier_tree_value(
    lohai: Option<&Token>,
    old_words: &[Token],
    sahai: Option<&Token>,
    new_words: &[Token],
    lehai: &WithFreeModifiers<Token>,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    let (constructor, label, entries) = if let Some(lohai) = lohai {
        let mut entries = vec![TreeEntry {
            label: Some("lohai"),
            value: generated_token_tree_value(lohai, source, options),
        }];
        if let Some(entry) =
            legacy_raw_replacement_words_tree_entry("old_words", old_words, source, options)
        {
            entries.push(entry);
        }
        if let Some(sahai) = sahai {
            entries.push(TreeEntry {
                label: Some("sahai"),
                value: generated_token_tree_value(sahai, source, options),
            });
        }
        if let Some(entry) =
            legacy_raw_replacement_words_tree_entry("new_words", new_words, source, options)
        {
            entries.push(entry);
        }
        entries.extend(legacy_token_field_entries("lehai", lehai, source, options));
        (
            "FullTextReplacementFreeModifier",
            "full_text_replacement_free_modifier",
            entries,
        )
    } else if let Some(sahai) = sahai {
        let mut entries = vec![TreeEntry {
            label: Some("sahai"),
            value: generated_token_tree_value(sahai, source, options),
        }];
        if let Some(entry) =
            legacy_raw_replacement_words_tree_entry("new_words", new_words, source, options)
        {
            entries.push(entry);
        }
        entries.extend(legacy_token_field_entries("lehai", lehai, source, options));
        (
            "NewOnlyTextReplacementFreeModifier",
            "new_only_text_replacement_free_modifier",
            entries,
        )
    } else {
        let entries = legacy_token_field_entries("lehai", lehai, source, options);
        (
            "CloseOnlyTextReplacementFreeModifier",
            "close_only_text_replacement_free_modifier",
            entries,
        )
    };
    let replacement =
        legacy_as_generated_free_modifier_variant_tree_value(constructor, label, entries);
    TreeValue::Node(TreeNode {
        constructor: "TextReplacementFreeModifier",
        entries: vec![TreeEntry {
            label: Some("text_replacement_free_modifier"),
            value: replacement,
        }],
    })
}

#[requires(!label.is_empty())]
#[ensures(true)]
fn legacy_raw_replacement_words_tree_entry(
    label: &'static str,
    words: &[Token],
    source: &str,
    options: TreeRenderOptions,
) -> Option<TreeEntry> {
    labelled_tree_collection_entry_from_values(
        label,
        words
            .iter()
            .map(|word| generated_token_tree_value(word, source, options))
            .collect(),
    )
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_vocative_argument_tree_value(
    sumti: &jbotci_syntax::ast::SumtiSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    match sumti.as_data() {
        bityzba::data!(jbotci_syntax::ast::SumtiSyntax::SumtiWithRelativeClauses {
            base_sumti,
            vuho: None,
            relative_clauses,
        }) if let bityzba::data!(jbotci_syntax::ast::SumtiSyntax::NameWords(names)) =
            base_sumti.as_data() =>
        {
            let mut entries = vec![TreeEntry {
                label: Some("names"),
                value: legacy_word_run_tree_value(&names.value, source, options),
            }];
            if let Some(entry) = labelled_tree_collection_entry_from_values(
                "free_modifiers",
                names
                    .free_modifiers
                    .iter()
                    .map(|free_modifier| {
                        legacy_as_generated_free_modifier_tree_value(free_modifier, source, options)
                    })
                    .collect(),
            ) {
                entries.push(entry);
            }
            if let Some(entry) = labelled_tree_collection_entry_from_values(
                "trailing_relative_clauses",
                legacy_as_generated_relative_clause_list_tree_values(
                    relative_clauses,
                    source,
                    options,
                ),
            ) {
                entries.push(entry);
            }
            TreeValue::Node(TreeNode {
                constructor: "CmevlaVocativeSumti",
                entries,
            })
        }
        bityzba::data!(jbotci_syntax::ast::SumtiSyntax::NameWords(names)) => {
            let mut entries = vec![TreeEntry {
                label: Some("names"),
                value: legacy_word_run_tree_value(&names.value, source, options),
            }];
            if let Some(entry) = labelled_tree_collection_entry_from_values(
                "free_modifiers",
                names
                    .free_modifiers
                    .iter()
                    .map(|free_modifier| {
                        legacy_as_generated_free_modifier_tree_value(free_modifier, source, options)
                    })
                    .collect(),
            ) {
                entries.push(entry);
            }
            TreeValue::Node(TreeNode {
                constructor: "CmevlaVocativeSumti",
                entries,
            })
        }
        bityzba::data!(jbotci_syntax::ast::SumtiSyntax::SelbriVocative {
            leading_relative_clauses,
            selbri,
            trailing_relative_clauses,
        }) => {
            let mut entries = Vec::new();
            if let Some(entry) = labelled_tree_collection_entry_from_values(
                "leading_relative_clauses",
                leading_relative_clauses
                    .iter()
                    .map(|relative_clause| {
                        legacy_as_generated_relative_clause_tree_value(
                            relative_clause,
                            source,
                            options,
                        )
                    })
                    .collect(),
            ) {
                entries.push(entry);
            }
            entries.push(TreeEntry {
                label: Some("selbri"),
                value: legacy_as_generated_selbri_tree_value(selbri.as_ref(), source, options),
            });
            if let Some(entry) = labelled_tree_collection_entry_from_values(
                "trailing_relative_clauses",
                trailing_relative_clauses
                    .iter()
                    .map(|relative_clause| {
                        legacy_as_generated_relative_clause_tree_value(
                            relative_clause,
                            source,
                            options,
                        )
                    })
                    .collect(),
            ) {
                entries.push(entry);
            }
            TreeValue::Node(TreeNode {
                constructor: "SelbriVocativeSumti",
                entries,
            })
        }
        _ => legacy_as_generated_sumti_tree_value(sumti, source, options),
    }
}

#[requires(!vocative_markers.value.is_empty())]
#[ensures(true)]
fn legacy_as_generated_vocative_markers_tree_value(
    vocative_markers: &WithFreeModifiers<Vec<Token>>,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    let marker_words = &vocative_markers.value;
    let marker_value = if marker_words
        .first()
        .is_some_and(|word| word.is_cmavo(Cmavo::Doi))
    {
        let inner = TreeValue::Node(TreeNode {
            constructor: "DoiVocativeMarkerWords",
            entries: vec![TreeEntry {
                label: Some("doi"),
                value: generated_token_tree_value(&marker_words[0], source, options),
            }],
        });
        TreeValue::Node(TreeNode {
            constructor: "DoiVocativeMarkerWords",
            entries: vec![TreeEntry {
                label: Some("doi_vocative_marker_words"),
                value: inner,
            }],
        })
    } else {
        let inner =
            legacy_as_generated_coi_vocative_markers_tree_value(marker_words, source, options);
        TreeValue::Node(TreeNode {
            constructor: "CoiVocativeMarkerWords",
            entries: vec![TreeEntry {
                label: Some("coi_vocative_marker_words"),
                value: inner,
            }],
        })
    };

    marker_value
}

#[requires(!marker_words.is_empty())]
#[ensures(true)]
fn legacy_as_generated_coi_vocative_markers_tree_value(
    marker_words: &[Token],
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    let mut entries = vec![TreeEntry {
        label: Some("first_coi"),
        value: generated_token_tree_value(&marker_words[0], source, options),
    }];
    let mut index = 1usize;
    if marker_words
        .get(index)
        .is_some_and(|word| word.is_cmavo(Cmavo::Nai))
    {
        entries.push(TreeEntry {
            label: Some("first_nai"),
            value: generated_token_tree_value(&marker_words[index], source, options),
        });
        index += 1;
    }

    let mut additional_coi = Vec::new();
    while marker_words
        .get(index)
        .is_some_and(|word| !word.is_cmavo(Cmavo::Doi))
    {
        let mut pair = vec![generated_token_tree_value(
            &marker_words[index],
            source,
            options,
        )];
        index += 1;
        if marker_words
            .get(index)
            .is_some_and(|word| word.is_cmavo(Cmavo::Nai))
        {
            pair.push(generated_token_tree_value(
                &marker_words[index],
                source,
                options,
            ));
            index += 1;
        }
        additional_coi.push(TreeValue::Collection(pair));
    }
    if let Some(entry) =
        labelled_tree_collection_entry_from_values("additional_coi", additional_coi)
    {
        entries.push(entry);
    }
    if let Some(doi) = marker_words.get(index) {
        entries.push(TreeEntry {
            label: Some("doi"),
            value: generated_token_tree_value(doi, source, options),
        });
    }

    TreeValue::Node(TreeNode {
        constructor: "CoiVocativeMarkerWords",
        entries,
    })
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_sumti_tree_value(
    sumti: &jbotci_syntax::ast::SumtiSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    if let bityzba::data!(jbotci_syntax::ast::SumtiSyntax::SumtiWithRelativeClauses {
        base_sumti,
        vuho: Some(vuho),
        relative_clauses,
    }) = sumti.as_data()
    {
        return TreeValue::Node(TreeNode {
            constructor: "Sumti",
            entries: vec![
                TreeEntry {
                    label: Some("base_sumti"),
                    value: legacy_as_generated_sumti_grouped_tree_value(
                        base_sumti.as_ref(),
                        source,
                        options,
                    ),
                },
                TreeEntry {
                    label: Some("vuho_attachment"),
                    value: legacy_as_generated_vuho_sumti_attachment_tree_value(
                        vuho,
                        relative_clauses,
                        None,
                        source,
                        options,
                    ),
                },
            ],
        });
    }
    if let bityzba::data!(
        jbotci_syntax::ast::SumtiSyntax::SumtiWithComplexRelativeClauses {
            base_sumti,
            vuho_marker,
            relative_clauses,
            sumti_connection,
        }
    ) = sumti.as_data()
    {
        return TreeValue::Node(TreeNode {
            constructor: "Sumti",
            entries: vec![
                TreeEntry {
                    label: Some("base_sumti"),
                    value: legacy_as_generated_sumti_grouped_tree_value(
                        base_sumti.as_ref(),
                        source,
                        options,
                    ),
                },
                TreeEntry {
                    label: Some("vuho_attachment"),
                    value: legacy_as_generated_vuho_sumti_attachment_tree_value(
                        vuho_marker,
                        relative_clauses,
                        sumti_connection.as_deref(),
                        source,
                        options,
                    ),
                },
            ],
        });
    }
    TreeValue::Node(TreeNode {
        constructor: "Sumti",
        entries: vec![TreeEntry {
            label: Some("base_sumti"),
            value: legacy_as_generated_sumti_grouped_tree_value(sumti, source, options),
        }],
    })
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_tagged_or_elided_sumti_tree_value(
    sumti: &jbotci_syntax::ast::SumtiSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    if matches!(
        sumti.as_data(),
        bityzba::data!(jbotci_syntax::ast::SumtiSyntax::ElidedSumti { .. })
    ) {
        return legacy_as_generated_sumti_base_tree_value(sumti, source, options);
    }

    legacy_as_generated_sumti_tree_value(sumti, source, options)
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_vuho_sumti_attachment_tree_value(
    vuho: &WithFreeModifiers<Token>,
    relative_clauses: &[jbotci_syntax::ast::RelativeClauseSyntax],
    sumti_connection: Option<&jbotci_syntax::ast::SumtiConnectionSyntax>,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    let mut entries = vec![TreeEntry {
        label: Some("vuho"),
        value: required_legacy_syntax_subtree_value(vuho, source, options),
    }];
    if let Some(entry) = labelled_tree_collection_entry_from_values(
        "relative_clauses",
        relative_clauses
            .iter()
            .map(|relative_clause| {
                legacy_as_generated_relative_clause_tree_value(relative_clause, source, options)
            })
            .collect(),
    ) {
        entries.push(entry);
    }
    if let Some(sumti_connection) = sumti_connection {
        entries.push(TreeEntry {
            label: Some("sumti_connection"),
            value: legacy_as_generated_sumti_connection_tail_tree_value(
                sumti_connection,
                source,
                options,
            ),
        });
    }
    TreeValue::Node(TreeNode {
        constructor: "VuhoSumtiAttachment",
        entries,
    })
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_sumti_connection_tail_tree_value(
    sumti_connection: &jbotci_syntax::ast::SumtiConnectionSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    TreeValue::Node(TreeNode {
        constructor: "SumtiConnection",
        entries: vec![
            TreeEntry {
                label: Some("connective"),
                value: required_legacy_syntax_subtree_value(
                    &sumti_connection.connective,
                    source,
                    options,
                ),
            },
            TreeEntry {
                label: Some("sumti"),
                value: legacy_as_generated_sumti_tree_value(
                    sumti_connection.sumti.as_ref(),
                    source,
                    options,
                ),
            },
        ],
    })
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_sumti_grouped_tree_value(
    sumti: &jbotci_syntax::ast::SumtiSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    if let bityzba::data!(jbotci_syntax::ast::SumtiSyntax::SumtiConnection {
        leading_sumti,
        connective,
        trailing_sumti,
    }) = sumti.as_data()
        && let bityzba::data!(jbotci_syntax::ast::SumtiSyntax::GroupedSumti {
            ke,
            inner_sumti,
            kehe,
        }) = trailing_sumti.as_data()
    {
        let mut grouped_tail_entries =
            legacy_as_generated_grouped_sumti_connective_entries(connective, source, options);
        grouped_tail_entries.extend([
            TreeEntry {
                label: Some("ke"),
                value: required_legacy_syntax_subtree_value(ke, source, options),
            },
            TreeEntry {
                label: Some("inner_sumti"),
                value: legacy_as_generated_sumti_tree_value(inner_sumti.as_ref(), source, options),
            },
        ]);
        if let Some(kehe) = kehe {
            grouped_tail_entries.push(TreeEntry {
                label: Some("kehe"),
                value: required_legacy_syntax_subtree_value(kehe, source, options),
            });
        }
        return TreeValue::Node(TreeNode {
            constructor: "SumtiGrouped",
            entries: vec![
                TreeEntry {
                    label: Some("leading_sumti"),
                    value: legacy_as_generated_sumti_afterthought_tree_value(
                        leading_sumti.as_ref(),
                        source,
                        options,
                    ),
                },
                TreeEntry {
                    label: Some("grouped_tail"),
                    value: TreeValue::Node(TreeNode {
                        constructor: "GroupedSumtiTail",
                        entries: grouped_tail_entries,
                    }),
                },
            ],
        });
    }

    TreeValue::Node(TreeNode {
        constructor: "SumtiGrouped",
        entries: vec![TreeEntry {
            label: Some("leading_sumti"),
            value: legacy_as_generated_sumti_afterthought_tree_value(sumti, source, options),
        }],
    })
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_sumti_afterthought_tree_value(
    sumti: &jbotci_syntax::ast::SumtiSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    let (leading_sumti, continuations) = legacy_sumti_afterthought_parts(sumti, source, options);
    let mut entries = vec![TreeEntry {
        label: Some("leading_sumti"),
        value: legacy_as_generated_sumti_bound_tree_value(leading_sumti, source, options),
    }];
    if let Some(entry) = labelled_tree_collection_entry_from_values("continuations", continuations)
    {
        entries.push(entry);
    }
    TreeValue::Node(TreeNode {
        constructor: "SumtiAfterthought",
        entries,
    })
}

#[requires(true)]
#[ensures(true)]
fn legacy_sumti_afterthought_parts<'tree>(
    sumti: &'tree jbotci_syntax::ast::SumtiSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> (&'tree jbotci_syntax::ast::SumtiSyntax, Vec<TreeValue>) {
    match sumti.as_data() {
        bityzba::data!(jbotci_syntax::ast::SumtiSyntax::SumtiConnection {
            leading_sumti,
            connective,
            trailing_sumti,
        }) if !matches!(
            trailing_sumti.as_data(),
            bityzba::data!(jbotci_syntax::ast::SumtiSyntax::GroupedSumti { .. })
        ) =>
        {
            let (leading, mut continuations) =
                legacy_sumti_afterthought_parts(leading_sumti.as_ref(), source, options);
            continuations.push(TreeValue::Node(TreeNode {
                constructor: "SumtiConnection",
                entries: vec![
                    TreeEntry {
                        label: Some("connective"),
                        value: required_legacy_syntax_subtree_value(connective, source, options),
                    },
                    TreeEntry {
                        label: Some("sumti"),
                        value: legacy_as_generated_sumti_bound_tree_value(
                            trailing_sumti.as_ref(),
                            source,
                            options,
                        ),
                    },
                ],
            }));
            (leading, continuations)
        }
        _ => (sumti, Vec::new()),
    }
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_sumti_bound_tree_value(
    sumti: &jbotci_syntax::ast::SumtiSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    if let bityzba::data!(jbotci_syntax::ast::SumtiSyntax::BoundSumtiConnection {
        leading_sumti,
        bo_connective,
        bo_tense_modal,
        bo,
        trailing_sumti,
    }) = sumti.as_data()
    {
        let mut tail_entries = Vec::new();
        if let Some(connective) = bo_connective {
            tail_entries.push(TreeEntry {
                label: Some("connective"),
                value: required_legacy_syntax_subtree_value(connective.as_ref(), source, options),
            });
        }
        if let Some(tense_modal) = bo_tense_modal {
            tail_entries.push(TreeEntry {
                label: Some("tense_modal"),
                value: legacy_as_generated_tense_modal_tree_value(
                    tense_modal.as_ref(),
                    source,
                    options,
                ),
            });
        }
        tail_entries.push(TreeEntry {
            label: Some("bo"),
            value: required_legacy_syntax_subtree_value(bo, source, options),
        });
        tail_entries.push(TreeEntry {
            label: Some("trailing_sumti"),
            value: legacy_as_generated_sumti_bound_tree_value(
                trailing_sumti.as_ref(),
                source,
                options,
            ),
        });
        return TreeValue::Node(TreeNode {
            constructor: "SumtiBound",
            entries: vec![
                TreeEntry {
                    label: Some("leading_sumti"),
                    value: legacy_as_generated_sumti_forethought_tree_value(
                        leading_sumti.as_ref(),
                        source,
                        options,
                    ),
                },
                TreeEntry {
                    label: Some("bound_tail"),
                    value: TreeValue::Node(TreeNode {
                        constructor: "BoundSumtiTail",
                        entries: tail_entries,
                    }),
                },
            ],
        });
    }

    TreeValue::Node(TreeNode {
        constructor: "SumtiBound",
        entries: vec![TreeEntry {
            label: Some("leading_sumti"),
            value: legacy_as_generated_sumti_forethought_tree_value(sumti, source, options),
        }],
    })
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_sumti_forethought_tree_value(
    sumti: &jbotci_syntax::ast::SumtiSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    match sumti.as_data() {
        bityzba::data!(
            jbotci_syntax::ast::SumtiSyntax::ForethoughtSumtiConnection {
                gek,
                leading_sumti,
                gik,
                trailing_sumti,
                gihi,
            }
        ) => {
            let mut entries = vec![
                TreeEntry {
                    label: Some("gek"),
                    value: legacy_as_generated_connective_tree_value(gek, source, options),
                },
                TreeEntry {
                    label: Some("leading_sumti"),
                    value: legacy_as_generated_sumti_tree_value(
                        leading_sumti.as_ref(),
                        source,
                        options,
                    ),
                },
                TreeEntry {
                    label: Some("gik"),
                    value: required_legacy_syntax_subtree_value(gik, source, options),
                },
                TreeEntry {
                    label: Some("trailing_sumti"),
                    value: legacy_as_generated_sumti_forethought_tree_value(
                        trailing_sumti.as_ref(),
                        source,
                        options,
                    ),
                },
            ];
            if let Some(gihi) = gihi {
                entries.push(TreeEntry {
                    label: Some("gihi"),
                    value: generated_token_tree_value(gihi, source, options),
                });
            }
            TreeValue::Node(TreeNode {
                constructor: "ForethoughtSumtiConnection",
                entries,
            })
        }
        _ => legacy_as_generated_simple_sumti_tree_value(sumti, source, options),
    }
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_simple_sumti_tree_value(
    sumti: &jbotci_syntax::ast::SumtiSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    if let bityzba::data!(jbotci_syntax::ast::SumtiSyntax::SumtiWithRelativeClauses {
        base_sumti,
        vuho,
        relative_clauses,
    }) = sumti.as_data()
    {
        let mut entries = vec![TreeEntry {
            label: Some("base_sumti"),
            value: legacy_as_generated_sumti_base_tree_value(base_sumti.as_ref(), source, options),
        }];
        if let Some(vuho) = vuho {
            entries.push(TreeEntry {
                label: Some("vuho"),
                value: required_legacy_syntax_subtree_value(vuho, source, options),
            });
        }
        if let Some(entry) = labelled_tree_collection_entry_from_values(
            "relative_clauses",
            relative_clauses
                .iter()
                .map(|relative_clause| {
                    legacy_as_generated_relative_clause_tree_value(relative_clause, source, options)
                })
                .collect(),
        ) {
            entries.push(entry);
        }
        return TreeValue::Node(TreeNode {
            constructor: "SimpleSumti",
            entries,
        });
    }

    TreeValue::Node(TreeNode {
        constructor: "SimpleSumti",
        entries: vec![TreeEntry {
            label: Some("base_sumti"),
            value: legacy_as_generated_sumti_base_tree_value(sumti, source, options),
        }],
    })
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_sumti_base_tree_value(
    sumti: &jbotci_syntax::ast::SumtiSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    match sumti.as_data() {
        bityzba::data!(jbotci_syntax::ast::SumtiSyntax::QuantifiedSumti {
            quantifier,
            inner_sumti,
        }) => TreeValue::Node(TreeNode {
            constructor: "QuantifiedSumti",
            entries: vec![
                TreeEntry {
                    label: Some("quantifier"),
                    value: legacy_as_generated_quantifier_tree_value(quantifier, source, options),
                },
                TreeEntry {
                    label: Some("inner_sumti"),
                    value: legacy_as_generated_sumti_base_tree_value(
                        inner_sumti.as_ref(),
                        source,
                        options,
                    ),
                },
            ],
        }),
        bityzba::data!(jbotci_syntax::ast::SumtiSyntax::ReferentSumti {
            lahe,
            relative_clauses,
            inner_sumti,
            luhu,
        }) => {
            let mut entries = vec![TreeEntry {
                label: Some("lahe"),
                value: required_legacy_syntax_subtree_value(lahe, source, options),
            }];
            if let Some(entry) = labelled_tree_collection_entry_from_values(
                "relative_clauses",
                relative_clauses
                    .iter()
                    .map(|relative_clause| {
                        legacy_as_generated_relative_clause_tree_value(
                            relative_clause,
                            source,
                            options,
                        )
                    })
                    .collect(),
            ) {
                entries.push(entry);
            }
            entries.push(TreeEntry {
                label: Some("inner_sumti"),
                value: legacy_as_generated_sumti_tree_value(inner_sumti.as_ref(), source, options),
            });
            if let Some(luhu) = luhu {
                entries.push(TreeEntry {
                    label: Some("luhu"),
                    value: required_legacy_syntax_subtree_value(luhu, source, options),
                });
            }
            TreeValue::Node(TreeNode {
                constructor: "ReferentSumti",
                entries,
            })
        }
        bityzba::data!(jbotci_syntax::ast::SumtiSyntax::QualifiedTerm {
            term_wrapper_kind,
            wrapper,
            wrapper_bo,
            inner_term,
            luhu,
        }) => {
            let mut entries = Vec::new();
            let constructor = match term_wrapper_kind {
                jbotci_syntax::ast::SumtiWrapperKindSyntax::Referent => {
                    entries.extend(legacy_token_field_entries("lahe", wrapper, source, options));
                    "ReferentTermWrapper"
                }
                jbotci_syntax::ast::SumtiWrapperKindSyntax::ScalarNegationWithBo => {
                    entries.push(TreeEntry {
                        label: Some("nahe"),
                        value: generated_token_tree_value(&wrapper.value, source, options),
                    });
                    let wrapper_bo = wrapper_bo
                        .as_ref()
                        .expect("scalar NAhE BO term wrapper has BO");
                    entries.extend(legacy_token_field_entries(
                        "bo", wrapper_bo, source, options,
                    ));
                    "ScalarNegatedTermWrapperWithBo"
                }
                jbotci_syntax::ast::SumtiWrapperKindSyntax::ScalarNegation => {
                    entries.extend(legacy_token_field_entries("nahe", wrapper, source, options));
                    "ScalarNegatedTermWrapper"
                }
            };
            entries.push(TreeEntry {
                label: Some("inner_term"),
                value: legacy_as_generated_term_tree_value(inner_term.as_ref(), source, options),
            });
            if let Some(luhu) = luhu {
                entries.push(TreeEntry {
                    label: Some("luhu"),
                    value: required_legacy_syntax_subtree_value(luhu, source, options),
                });
            }
            TreeValue::Node(TreeNode {
                constructor,
                entries,
            })
        }
        bityzba::data!(jbotci_syntax::ast::SumtiSyntax::ScalarNegatedSumtiWithBo {
            nahe,
            bo,
            inner_sumti,
            luhu,
        }) => {
            let mut entries = vec![
                TreeEntry {
                    label: Some("nahe"),
                    value: generated_token_tree_value(nahe, source, options),
                },
                TreeEntry {
                    label: Some("bo"),
                    value: required_legacy_syntax_subtree_value(bo, source, options),
                },
                TreeEntry {
                    label: Some("inner_sumti"),
                    value: legacy_as_generated_sumti_tree_value(
                        inner_sumti.as_ref(),
                        source,
                        options,
                    ),
                },
            ];
            if let Some(luhu) = luhu {
                entries.push(TreeEntry {
                    label: Some("luhu"),
                    value: required_legacy_syntax_subtree_value(luhu, source, options),
                });
            }
            TreeValue::Node(TreeNode {
                constructor: "ScalarNegatedSumtiWithBo",
                entries,
            })
        }
        bityzba::data!(jbotci_syntax::ast::SumtiSyntax::ScalarNegatedSumti {
            nahe,
            inner_sumti,
            luhu,
        }) => {
            let mut entries = vec![
                TreeEntry {
                    label: Some("nahe"),
                    value: required_legacy_syntax_subtree_value(nahe, source, options),
                },
                TreeEntry {
                    label: Some("inner_sumti"),
                    value: legacy_as_generated_sumti_tree_value(
                        inner_sumti.as_ref(),
                        source,
                        options,
                    ),
                },
            ];
            if let Some(luhu) = luhu {
                entries.push(TreeEntry {
                    label: Some("luhu"),
                    value: required_legacy_syntax_subtree_value(luhu, source, options),
                });
            }
            TreeValue::Node(TreeNode {
                constructor: "ScalarNegatedSumti",
                entries,
            })
        }
        bityzba::data!(jbotci_syntax::ast::SumtiSyntax::ProSumti(token)) => {
            let mut entries = vec![TreeEntry {
                label: None,
                value: generated_token_tree_value(&token.value, source, options),
            }];
            if let Some(entry) = labelled_tree_collection_entry_from_values(
                "free_modifiers",
                token
                    .free_modifiers
                    .iter()
                    .map(|free_modifier| {
                        legacy_as_generated_free_modifier_tree_value(free_modifier, source, options)
                    })
                    .collect(),
            ) {
                entries.push(entry);
            }
            TreeValue::Node(TreeNode {
                constructor: "ProSumti",
                entries,
            })
        }
        bityzba::data!(jbotci_syntax::ast::SumtiSyntax::NameDescription { la, names }) => {
            let mut entries = legacy_token_field_entries("la", la, source, options);
            entries.push(TreeEntry {
                label: Some("names"),
                value: legacy_word_run_tree_value(&names.value, source, options),
            });
            if let Some(entry) = labelled_tree_collection_entry_from_values(
                "free_modifiers",
                names
                    .free_modifiers
                    .iter()
                    .map(|free_modifier| {
                        legacy_as_generated_free_modifier_tree_value(free_modifier, source, options)
                    })
                    .collect(),
            ) {
                entries.push(entry);
            }
            TreeValue::Node(TreeNode {
                constructor: "NameSumti",
                entries,
            })
        }
        bityzba::data!(jbotci_syntax::ast::SumtiSyntax::Description(description)) => {
            legacy_as_generated_description_sumti_tree_value(description.as_ref(), source, options)
        }
        bityzba::data!(jbotci_syntax::ast::SumtiSyntax::DescriptionConnection(
            description
        )) => legacy_as_generated_description_connection_sumti_tree_value(
            description.as_ref(),
            source,
            options,
        ),
        bityzba::data!(jbotci_syntax::ast::SumtiSyntax::QuotedSumti(quote)) => {
            legacy_as_generated_quoted_sumti_tree_value(quote.as_ref(), source, options)
        }
        bityzba::data!(jbotci_syntax::ast::SumtiSyntax::NumberSumti {
            li,
            expression,
            loho,
        }) => {
            let mut entries = vec![
                TreeEntry {
                    label: Some("li"),
                    value: required_legacy_syntax_subtree_value(li, source, options),
                },
                TreeEntry {
                    label: Some("expression"),
                    value: legacy_as_generated_mekso_tree_value(
                        expression.as_ref(),
                        source,
                        options,
                    ),
                },
            ];
            if let Some(loho) = loho {
                entries.push(TreeEntry {
                    label: Some("loho"),
                    value: required_legacy_syntax_subtree_value(loho, source, options),
                });
            }
            TreeValue::Node(TreeNode {
                constructor: "NumberSumti",
                entries,
            })
        }
        bityzba::data!(jbotci_syntax::ast::SumtiSyntax::LerfuStringSumti { letter, boi }) => {
            let mut entries = vec![TreeEntry {
                label: Some("words"),
                value: legacy_word_run_tree_value(&letter.value, source, options),
            }];
            if let Some(entry) = labelled_tree_collection_entry_from_values(
                "free_modifiers",
                letter
                    .free_modifiers
                    .iter()
                    .map(|free_modifier| {
                        legacy_as_generated_free_modifier_tree_value(free_modifier, source, options)
                    })
                    .collect(),
            ) {
                entries.push(entry);
            }
            if let Some(boi) = boi {
                entries.extend(legacy_token_field_entries("boi", boi, source, options));
            }
            TreeValue::Node(TreeNode {
                constructor: "LerfuStringSumti",
                entries,
            })
        }
        bityzba::data!(jbotci_syntax::ast::SumtiSyntax::BridiDescription {
            lohoi,
            subbridi,
            kuhau,
        }) => {
            let mut entries = legacy_token_field_entries("lohoi", lohoi, source, options);
            entries.push(TreeEntry {
                label: Some("subbridi"),
                value: legacy_as_generated_subbridi_tree_value(subbridi.as_ref(), source, options),
            });
            if let Some(kuhau) = kuhau {
                entries.extend(legacy_token_field_entries("kuhau", kuhau, source, options));
            }
            TreeValue::Node(TreeNode {
                constructor: "BridiDescription",
                entries,
            })
        }
        bityzba::data!(jbotci_syntax::ast::SumtiSyntax::ElidedSumti {
            tag,
            maybe_ku,
            free_modifiers,
        }) => {
            assert!(
                tag.is_none(),
                "tagged legacy elided sumti reached untagged generated ElidedSumti adapter"
            );
            let mut entries = Vec::new();
            if let Some(maybe_ku) = maybe_ku {
                entries.extend(legacy_token_field_entries(
                    "maybe_ku", maybe_ku, source, options,
                ));
            }
            if let Some(entry) = labelled_tree_collection_entry_from_values(
                "free_modifiers",
                free_modifiers
                    .iter()
                    .map(|free_modifier| {
                        legacy_as_generated_free_modifier_tree_value(free_modifier, source, options)
                    })
                    .collect(),
            ) {
                entries.push(entry);
            }
            TreeValue::Node(TreeNode {
                constructor: "ElidedSumti",
                entries,
            })
        }
        _ => required_legacy_syntax_subtree_value(sumti, source, options),
    }
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_quoted_sumti_tree_value(
    quote: &jbotci_syntax::ast::QuoteSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    match quote.as_data() {
        bityzba::data!(jbotci_syntax::ast::QuoteSyntax::TextQuote { lu, text, lihu }) => {
            let mut entries = legacy_token_field_entries("lu", lu, source, options);
            entries.push(TreeEntry {
                label: Some("text"),
                value: legacy_as_generated_text_tree_value(text.as_ref(), source, options),
            });
            if let Some(lihu) = lihu {
                entries.extend(legacy_token_field_entries("lihu", lihu, source, options));
            }
            TreeValue::Node(TreeNode {
                constructor: "TextQuote",
                entries: vec![TreeEntry {
                    label: Some("text_quote"),
                    value: TreeValue::Node(TreeNode {
                        constructor: "TextQuote",
                        entries,
                    }),
                }],
            })
        }
        bityzba::data!(jbotci_syntax::ast::QuoteSyntax::WordQuote(quote))
        | bityzba::data!(jbotci_syntax::ast::QuoteSyntax::WordsQuote(quote))
        | bityzba::data!(jbotci_syntax::ast::QuoteSyntax::DelimitedNonLojbanQuote(
            quote
        )) => legacy_as_generated_compound_quote_enum_tree_value(
            "GenericCompoundQuote",
            "generic_compound_quote",
            quote,
            source,
            options,
        ),
        bityzba::data!(jbotci_syntax::ast::QuoteSyntax::DelimitedWordQuote(quote)) => {
            let (constructor, field_label) =
                legacy_generated_delimited_word_quote_branch(&quote.value);
            legacy_as_generated_compound_quote_enum_tree_value(
                constructor,
                field_label,
                quote,
                source,
                options,
            )
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn legacy_generated_delimited_word_quote_branch(
    quote_marker: &Token,
) -> (&'static str, &'static str) {
    let marker_cmavo = quote_marker.quote_marker_cmavo();
    if marker_cmavo == Some(Cmavo::Mehoi) {
        return (
            "ExperimentalMehoiCompoundQuote",
            "experimental_mehoi_compound_quote",
        );
    }
    if marker_cmavo == Some(Cmavo::Zohoi) || marker_cmavo == Some(Cmavo::Lahoi) {
        return (
            "ExperimentalZohoiCompoundQuote",
            "experimental_zohoi_compound_quote",
        );
    }
    if marker_cmavo == Some(Cmavo::Rahoi) {
        return (
            "ExperimentalRahoiCompoundQuote",
            "experimental_rahoi_compound_quote",
        );
    }
    if marker_cmavo == Some(Cmavo::Gohoi)
        || marker_cmavo == Some(Cmavo::Zehoi)
        || marker_cmavo == Some(Cmavo::Tahai)
        || marker_cmavo == Some(Cmavo::Bohei)
    {
        return (
            "ExperimentalGohoiCompoundQuote",
            "experimental_gohoi_compound_quote",
        );
    }
    panic!(
        "legacy delimited word quote marker was not represented in generated compound_quote: {}",
        quote_marker.core_word()
    );
}

#[requires(!constructor.is_empty() && !field_label.is_empty())]
#[ensures(true)]
fn legacy_as_generated_compound_quote_enum_tree_value(
    constructor: &'static str,
    field_label: &'static str,
    quote: &WithFreeModifiers<Token>,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    TreeValue::Node(TreeNode {
        constructor,
        entries: vec![TreeEntry {
            label: Some(field_label),
            value: TreeValue::Node(TreeNode {
                constructor,
                entries: legacy_token_field_entries("quote", quote, source, options),
            }),
        }],
    })
}

#[requires(true)]
#[ensures(true)]
fn legacy_token_field_entries(
    label: &'static str,
    token: &WithFreeModifiers<Token>,
    source: &str,
    options: TreeRenderOptions,
) -> Vec<TreeEntry> {
    let mut entries = vec![TreeEntry {
        label: Some(label),
        value: generated_token_tree_value(&token.value, source, options),
    }];
    if let Some(entry) = labelled_tree_collection_entry_from_values(
        "free_modifiers",
        token
            .free_modifiers
            .iter()
            .map(|free_modifier| {
                legacy_as_generated_free_modifier_tree_value(free_modifier, source, options)
            })
            .collect(),
    ) {
        entries.push(entry);
    }
    entries
}

#[requires(true)]
#[ensures(true)]
fn legacy_token_tree_value_with_extra_free_modifiers(
    token: &Token,
    free_modifiers: &[jbotci_syntax::ast::FreeModifierSyntax],
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    let value = generated_token_tree_value(token, source, options);
    if free_modifiers.is_empty() {
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
                value: TreeValue::Collection(
                    free_modifiers
                        .iter()
                        .map(|free_modifier| {
                            legacy_as_generated_free_modifier_tree_value(
                                free_modifier,
                                source,
                                options,
                            )
                        })
                        .collect(),
                ),
            },
        ],
    })
}

#[requires(true)]
#[ensures(true)]
fn legacy_attach_free_modifiers_to_rightmost_tense_leaf(
    value: TreeValue,
    free_modifiers: &[jbotci_syntax::ast::FreeModifierSyntax],
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    if free_modifiers.is_empty() {
        return value;
    }

    match value {
        TreeValue::Node(mut node) => {
            if legacy_is_tense_leaf_constructor(node.constructor) {
                if let Some(entry) = legacy_free_modifiers_tree_entry(
                    "free_modifiers",
                    free_modifiers,
                    source,
                    options,
                ) {
                    node.entries.push(entry);
                }
                return TreeValue::Node(node);
            }
            for entry in node.entries.iter_mut().rev() {
                if legacy_tree_value_contains_tense_leaf(&entry.value) {
                    let value = std::mem::replace(&mut entry.value, TreeValue::Collection(vec![]));
                    entry.value = legacy_attach_free_modifiers_to_rightmost_tense_leaf(
                        value,
                        free_modifiers,
                        source,
                        options,
                    );
                    return TreeValue::Node(node);
                }
            }
            TreeValue::Node(node)
        }
        TreeValue::Collection(mut values) => {
            for value in values.iter_mut().rev() {
                if legacy_tree_value_contains_tense_leaf(value) {
                    let old_value = std::mem::replace(value, TreeValue::Collection(vec![]));
                    *value = legacy_attach_free_modifiers_to_rightmost_tense_leaf(
                        old_value,
                        free_modifiers,
                        source,
                        options,
                    );
                    return TreeValue::Collection(values);
                }
            }
            TreeValue::Collection(values)
        }
        TreeValue::Syntax { syntax_ids, value } => TreeValue::Syntax {
            syntax_ids,
            value: Box::new(legacy_attach_free_modifiers_to_rightmost_tense_leaf(
                *value,
                free_modifiers,
                source,
                options,
            )),
        },
        value => value,
    }
}

#[requires(true)]
#[ensures(true)]
fn legacy_tree_value_contains_tense_leaf(value: &TreeValue) -> bool {
    match value {
        TreeValue::Node(node) => {
            legacy_is_tense_leaf_constructor(node.constructor)
                || node
                    .entries
                    .iter()
                    .any(|entry| legacy_tree_value_contains_tense_leaf(&entry.value))
        }
        TreeValue::Collection(values) => values.iter().any(legacy_tree_value_contains_tense_leaf),
        TreeValue::Syntax { value, .. } => legacy_tree_value_contains_tense_leaf(value),
        _ => false,
    }
}

#[requires(true)]
#[ensures(true)]
fn legacy_is_tense_leaf_constructor(constructor: &str) -> bool {
    matches!(
        constructor,
        "CahaTense"
            | "CuheTense"
            | "FaFlatTagTense"
            | "FahaIntervalDirectionTense"
            | "FahaSpaceOffsetTense"
            | "FeheIntervalPropertyTense"
            | "KiCompositeTense"
            | "ModalTense"
            | "MohiSpaceOffsetTense"
            | "NumberedIntervalPropertyTense"
            | "PuTimeOffsetTense"
            | "TaheIntervalPropertyTense"
            | "VaSpaceDistanceTense"
            | "VehaSpaceIntervalTense"
            | "VihaSpaceIntervalTense"
            | "ZahoIntervalPropertyTense"
            | "ZehaTimeIntervalTense"
            | "ZiTimeDistanceTense"
    )
}

#[requires(!label.is_empty())]
#[ensures(true)]
fn legacy_free_modifiers_tree_entry(
    label: &'static str,
    free_modifiers: &[jbotci_syntax::ast::FreeModifierSyntax],
    source: &str,
    options: TreeRenderOptions,
) -> Option<TreeEntry> {
    labelled_tree_collection_entry_from_values(
        label,
        free_modifiers
            .iter()
            .map(|free_modifier| {
                legacy_as_generated_free_modifier_tree_value(free_modifier, source, options)
            })
            .collect(),
    )
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_description_connection_sumti_tree_value(
    description: &jbotci_syntax::ast::DescriptionConnectionSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    let mut entries = vec![
        TreeEntry {
            label: Some("leading_description_head"),
            value: legacy_as_generated_description_head_tree_value(
                &description.leading_description_head.description,
                source,
                options,
            ),
        },
        TreeEntry {
            label: Some("connective"),
            value: TreeValue::Node(TreeNode {
                constructor: "Afterthought",
                entries: vec![TreeEntry {
                    label: Some("connective"),
                    value: required_legacy_syntax_subtree_value(
                        &description.connective,
                        source,
                        options,
                    ),
                }],
            }),
        },
        TreeEntry {
            label: Some("trailing_description_head"),
            value: legacy_as_generated_description_head_tree_value(
                &description.trailing_description_head.description,
                source,
                options,
            ),
        },
        TreeEntry {
            label: Some("tail"),
            value: legacy_as_generated_description_tail_tree_value(
                &description.tail_elements,
                description.selbri.as_deref(),
                &description.relative_clauses,
                source,
                options,
            ),
        },
    ];
    if let Some(ku) = &description.ku {
        entries.push(TreeEntry {
            label: Some("ku"),
            value: required_legacy_syntax_subtree_value(ku, source, options),
        });
    }
    TreeValue::Node(TreeNode {
        constructor: "DescriptionConnectionSumti",
        entries,
    })
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_description_sumti_tree_value(
    description: &jbotci_syntax::ast::DescriptionSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    let Some(description_marker) = &description.description else {
        return legacy_as_generated_gadri_elided_description_sumti_tree_value(
            description,
            source,
            options,
        );
    };
    let tail = legacy_as_generated_description_tail_tree_value(
        &description.tail_elements,
        description.selbri.as_deref(),
        &description.relative_clauses,
        source,
        options,
    );

    let mut entries = Vec::new();
    let constructor = if let Some(outer_quantifier) = &description.outer_quantifier {
        entries.push(TreeEntry {
            label: Some("outer_quantifier"),
            value: legacy_as_generated_quantifier_tree_value(outer_quantifier, source, options),
        });
        "DescriptorWithOuterQuantifierSumti"
    } else {
        "DescriptorWithGadriSumti"
    };
    entries.push(TreeEntry {
        label: Some("description"),
        value: legacy_as_generated_description_head_tree_value(description_marker, source, options),
    });
    entries.push(TreeEntry {
        label: Some("tail"),
        value: tail,
    });
    if let Some(ku) = &description.ku {
        entries.push(TreeEntry {
            label: Some("ku"),
            value: required_legacy_syntax_subtree_value(ku, source, options),
        });
    }
    TreeValue::Node(TreeNode {
        constructor,
        entries,
    })
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_gadri_elided_description_sumti_tree_value(
    description: &jbotci_syntax::ast::DescriptionSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    if let (Some(quantifier), [], Some(selbri)) = (
        description
            .tail_elements
            .first()
            .and_then(legacy_description_tail_quantifier),
        &description.tail_elements[description.tail_elements.len().min(1)..],
        description.selbri.as_deref(),
    ) {
        let mut entries = vec![
            TreeEntry {
                label: Some("quantifier"),
                value: legacy_as_generated_quantifier_tree_value(quantifier, source, options),
            },
            TreeEntry {
                label: Some("selbri"),
                value: legacy_as_generated_selbri_tree_value(selbri, source, options),
            },
        ];
        if let Some(entry) = labelled_tree_collection_entry_from_values(
            "relative_clauses",
            description
                .relative_clauses
                .iter()
                .map(|relative_clause| {
                    legacy_as_generated_relative_clause_tree_value(relative_clause, source, options)
                })
                .collect(),
        ) {
            entries.push(entry);
        }
        return TreeValue::Node(TreeNode {
            constructor: "DescriptorWithoutGadriSumti",
            entries,
        });
    }
    required_legacy_syntax_subtree_value(description, source, options)
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_description_head_tree_value(
    description_marker: &WithFreeModifiers<Token>,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    TreeValue::Node(TreeNode {
        constructor: "DescriptionHead",
        entries: legacy_token_field_entries("description", description_marker, source, options),
    })
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_description_tail_tree_value(
    tail_elements: &[jbotci_syntax::ast::DescriptionTailElementSyntax],
    selbri: Option<&jbotci_syntax::ast::SelbriSyntax>,
    relative_clauses: &[jbotci_syntax::ast::RelativeClauseSyntax],
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    if let Some((quantifier, rest)) = legacy_description_tail_initial_quantifier(tail_elements) {
        if let Some(sumti) = legacy_single_description_tail_sumti(rest) {
            return TreeValue::Node(TreeNode {
                constructor: "DescriptionTail",
                entries: vec![
                    TreeEntry {
                        label: Some("leading_tail_elements"),
                        value: legacy_empty_leading_description_tail_elements_tree_value(),
                    },
                    TreeEntry {
                        label: Some("tail"),
                        value: TreeValue::Node(TreeNode {
                            constructor: "QuantifierSumtiDescriptionTail",
                            entries: vec![
                                TreeEntry {
                                    label: Some("quantifier"),
                                    value: legacy_as_generated_quantifier_tree_value(
                                        quantifier, source, options,
                                    ),
                                },
                                TreeEntry {
                                    label: Some("sumti"),
                                    value: legacy_as_generated_sumti_tree_value(
                                        sumti, source, options,
                                    ),
                                },
                            ],
                        }),
                    },
                ],
            });
        }

        if let Some(selbri) = selbri {
            let mut tail_entries = vec![
                TreeEntry {
                    label: Some("quantifier"),
                    value: legacy_as_generated_quantifier_tree_value(quantifier, source, options),
                },
                TreeEntry {
                    label: Some("selbri"),
                    value: legacy_as_generated_selbri_tree_value(selbri, source, options),
                },
            ];
            if let Some(entry) = labelled_tree_collection_entry_from_values(
                "relative_clauses",
                legacy_as_generated_relative_clause_list_tree_values(
                    relative_clauses,
                    source,
                    options,
                ),
            ) {
                tail_entries.push(entry);
            }
            return TreeValue::Node(TreeNode {
                constructor: "DescriptionTail",
                entries: vec![
                    TreeEntry {
                        label: Some("leading_tail_elements"),
                        value: legacy_empty_leading_description_tail_elements_tree_value(),
                    },
                    TreeEntry {
                        label: Some("tail"),
                        value: TreeValue::Node(TreeNode {
                            constructor: "QuantifierRelationDescriptionTail",
                            entries: tail_entries,
                        }),
                    },
                ],
            });
        }
    }

    let leading_tail_elements = legacy_as_generated_leading_description_tail_elements_tree_value(
        tail_elements,
        source,
        options,
    );
    let mut entries = vec![TreeEntry {
        label: Some("leading_tail_elements"),
        value: leading_tail_elements,
    }];
    if let Some(selbri) = selbri {
        let mut tail_entries = Vec::new();
        let tail_constructor =
            if let Some(quantifier) = legacy_description_tail_any_quantifier(tail_elements) {
                tail_entries.push(TreeEntry {
                    label: Some("quantifier"),
                    value: legacy_as_generated_quantifier_tree_value(quantifier, source, options),
                });
                "QuantifierRelationDescriptionTail"
            } else {
                "RelationDescriptionTail"
            };
        tail_entries.push(TreeEntry {
            label: Some("selbri"),
            value: legacy_as_generated_selbri_tree_value(selbri, source, options),
        });
        if let Some(entry) = labelled_tree_collection_entry_from_values(
            "relative_clauses",
            legacy_as_generated_relative_clause_list_tree_values(relative_clauses, source, options),
        ) {
            tail_entries.push(entry);
        }
        entries.push(TreeEntry {
            label: Some("tail"),
            value: TreeValue::Node(TreeNode {
                constructor: tail_constructor,
                entries: tail_entries,
            }),
        });
    }
    TreeValue::Node(TreeNode {
        constructor: "DescriptionTail",
        entries,
    })
}

#[requires(true)]
#[ensures(true)]
fn legacy_empty_leading_description_tail_elements_tree_value() -> TreeValue {
    TreeValue::Node(TreeNode {
        constructor: "LeadingDescriptionTailElements",
        entries: Vec::new(),
    })
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_leading_description_tail_elements_tree_value(
    tail_elements: &[jbotci_syntax::ast::DescriptionTailElementSyntax],
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    let mut entries = Vec::new();
    for tail_element in tail_elements {
        match tail_element.as_data() {
            bityzba::data!(
                jbotci_syntax::ast::DescriptionTailElementSyntax::DescriptionTailSumti(sumti)
            ) => {
                entries.push(TreeEntry {
                    label: Some("tail_sumti"),
                    value: TreeValue::Node(TreeNode {
                        constructor: "DescriptionTailSumti",
                        entries: vec![TreeEntry {
                            label: Some("sumti"),
                            value: legacy_as_generated_sumti_base_tree_value(
                                sumti.as_ref(),
                                source,
                                options,
                            ),
                        }],
                    }),
                });
            }
            bityzba::data!(
                jbotci_syntax::ast::DescriptionTailElementSyntax::DescriptionTailRelativeClauses(
                    relative_clauses,
                )
            ) => {
                entries.push(TreeEntry {
                    label: Some("relative_clauses"),
                    value: TreeValue::Collection(
                        legacy_as_generated_relative_clause_list_tree_values(
                            relative_clauses,
                            source,
                            options,
                        ),
                    ),
                });
            }
            bityzba::data!(
                jbotci_syntax::ast::DescriptionTailElementSyntax::DescriptionTailQuantifier(_,)
            ) => {}
        }
    }
    TreeValue::Node(TreeNode {
        constructor: "LeadingDescriptionTailElements",
        entries,
    })
}

#[requires(true)]
#[ensures(true)]
fn legacy_description_tail_initial_quantifier(
    tail_elements: &[jbotci_syntax::ast::DescriptionTailElementSyntax],
) -> Option<(
    &jbotci_syntax::ast::QuantifierSyntax,
    &[jbotci_syntax::ast::DescriptionTailElementSyntax],
)> {
    let (first, rest) = tail_elements.split_first()?;
    legacy_description_tail_quantifier(first).map(|quantifier| (quantifier, rest))
}

#[requires(true)]
#[ensures(true)]
fn legacy_description_tail_quantifier(
    tail_element: &jbotci_syntax::ast::DescriptionTailElementSyntax,
) -> Option<&jbotci_syntax::ast::QuantifierSyntax> {
    match tail_element.as_data() {
        bityzba::data!(
            jbotci_syntax::ast::DescriptionTailElementSyntax::DescriptionTailQuantifier(quantifier,)
        ) => Some(quantifier),
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn legacy_description_tail_any_quantifier(
    tail_elements: &[jbotci_syntax::ast::DescriptionTailElementSyntax],
) -> Option<&jbotci_syntax::ast::QuantifierSyntax> {
    tail_elements
        .iter()
        .find_map(legacy_description_tail_quantifier)
}

#[requires(true)]
#[ensures(true)]
fn legacy_single_description_tail_sumti(
    tail_elements: &[jbotci_syntax::ast::DescriptionTailElementSyntax],
) -> Option<&jbotci_syntax::ast::SumtiSyntax> {
    let [tail_element] = tail_elements else {
        return None;
    };
    match tail_element.as_data() {
        bityzba::data!(
            jbotci_syntax::ast::DescriptionTailElementSyntax::DescriptionTailSumti(sumti)
        ) => Some(sumti.as_ref()),
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_quantifier_tree_value(
    quantifier: &jbotci_syntax::ast::QuantifierSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    match quantifier.as_data() {
        bityzba::data!(jbotci_syntax::ast::QuantifierSyntax::NumberQuantifier { number, boi }) => {
            let mut entries = vec![TreeEntry {
                label: Some("number"),
                value: legacy_word_run_tree_value(&number.value, source, options),
            }];
            if let Some(entry) = labelled_tree_collection_entry_from_values(
                "free_modifiers",
                number
                    .free_modifiers
                    .iter()
                    .map(|free_modifier| {
                        legacy_as_generated_free_modifier_tree_value(free_modifier, source, options)
                    })
                    .collect(),
            ) {
                entries.push(entry);
            }
            if let Some(boi) = boi {
                entries.extend(legacy_token_field_entries("boi", boi, source, options));
            }
            TreeValue::Node(TreeNode {
                constructor: "NumberQuantifier",
                entries,
            })
        }
        bityzba::data!(jbotci_syntax::ast::QuantifierSyntax::MeksoQuantifier {
            vei,
            mekso,
            veho,
        }) => {
            let mut entries = vec![
                TreeEntry {
                    label: Some("vei"),
                    value: required_legacy_syntax_subtree_value(vei, source, options),
                },
                TreeEntry {
                    label: Some("mekso"),
                    value: legacy_as_generated_mekso_tree_value(mekso.as_ref(), source, options),
                },
            ];
            if let Some(veho) = veho {
                entries.push(TreeEntry {
                    label: Some("veho"),
                    value: required_legacy_syntax_subtree_value(veho, source, options),
                });
            }
            TreeValue::Node(TreeNode {
                constructor: "MeksoQuantifier",
                entries,
            })
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_relative_clause_tree_value(
    relative_clause: &jbotci_syntax::ast::RelativeClauseSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    match relative_clause.as_data() {
        bityzba::data!(
            jbotci_syntax::ast::RelativeClauseSyntax::SumtiAssociationPhrase(phrase,)
        ) => {
            let mut entries = vec![
                TreeEntry {
                    label: Some("association_marker"),
                    value: required_legacy_syntax_subtree_value(
                        &phrase.association_marker,
                        source,
                        options,
                    ),
                },
                TreeEntry {
                    label: Some("sumti"),
                    value: legacy_as_generated_relative_sumti_tree_value(
                        phrase.sumti.as_ref(),
                        source,
                        options,
                    ),
                },
            ];
            if let Some(gehu) = &phrase.gehu {
                entries.push(TreeEntry {
                    label: Some("gehu"),
                    value: required_legacy_syntax_subtree_value(gehu, source, options),
                });
            }
            TreeValue::Node(TreeNode {
                constructor: "SumtiAssociationPhrase",
                entries,
            })
        }
        bityzba::data!(
            jbotci_syntax::ast::RelativeClauseSyntax::IncidentalRelativeBridi {
                noi,
                subbridi,
                kuho,
            }
        ) => {
            let mut entries = vec![
                TreeEntry {
                    label: Some("noi"),
                    value: required_legacy_syntax_subtree_value(noi, source, options),
                },
                TreeEntry {
                    label: Some("subbridi"),
                    value: legacy_as_generated_subbridi_tree_value(
                        subbridi.as_ref(),
                        source,
                        options,
                    ),
                },
            ];
            if let Some(kuho) = kuho {
                entries.push(TreeEntry {
                    label: Some("kuho"),
                    value: required_legacy_syntax_subtree_value(kuho, source, options),
                });
            }
            TreeValue::Node(TreeNode {
                constructor: "IncidentalRelativeBridi",
                entries,
            })
        }
        bityzba::data!(
            jbotci_syntax::ast::RelativeClauseSyntax::RestrictiveRelativeBridi {
                poi,
                subbridi,
                kuho,
            }
        ) => {
            let mut entries = vec![
                TreeEntry {
                    label: Some("poi"),
                    value: required_legacy_syntax_subtree_value(poi, source, options),
                },
                TreeEntry {
                    label: Some("subbridi"),
                    value: legacy_as_generated_subbridi_tree_value(
                        subbridi.as_ref(),
                        source,
                        options,
                    ),
                },
            ];
            if let Some(kuho) = kuho {
                entries.push(TreeEntry {
                    label: Some("kuho"),
                    value: required_legacy_syntax_subtree_value(kuho, source, options),
                });
            }
            TreeValue::Node(TreeNode {
                constructor: "RestrictiveRelativeBridi",
                entries,
            })
        }
        bityzba::data!(
            jbotci_syntax::ast::RelativeClauseSyntax::JoinedRelativeClauses { zihe, inner }
        ) => TreeValue::Collection(vec![TreeValue::Node(TreeNode {
            constructor: "JoinedRelativeClauses",
            entries: vec![
                TreeEntry {
                    label: Some("zihe"),
                    value: required_legacy_syntax_subtree_value(zihe, source, options),
                },
                TreeEntry {
                    label: Some("inner"),
                    value: legacy_as_generated_relative_clause_tree_value(
                        inner.as_ref(),
                        source,
                        options,
                    ),
                },
            ],
        })]),
        bityzba::data!(
            jbotci_syntax::ast::RelativeClauseSyntax::RelativeClauseConnection {
                connective,
                inner,
            }
        ) => TreeValue::Collection(vec![TreeValue::Node(TreeNode {
            constructor: "RelativeClauseConnection",
            entries: vec![
                TreeEntry {
                    label: Some("connective"),
                    value: required_legacy_syntax_subtree_value(connective, source, options),
                },
                TreeEntry {
                    label: Some("inner"),
                    value: legacy_as_generated_relative_clause_tree_value(
                        inner.as_ref(),
                        source,
                        options,
                    ),
                },
            ],
        })]),
    }
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_relative_sumti_tree_value(
    sumti: &jbotci_syntax::ast::SumtiSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    if matches!(
        sumti.as_data(),
        bityzba::data!(jbotci_syntax::ast::SumtiSyntax::NegatedSumti { .. })
    ) {
        return required_legacy_syntax_subtree_value(sumti, source, options);
    }
    if let Some((tense_modal, maybe_ku, free_modifiers)) =
        legacy_elided_tense_tagged_sumti_parts(sumti)
    {
        return TreeValue::Node(TreeNode {
            constructor: "TenseTaggedRelativeSumti",
            entries: vec![
                TreeEntry {
                    label: Some("tense_modal"),
                    value: legacy_as_generated_tense_modal_tree_value(tense_modal, source, options),
                },
                TreeEntry {
                    label: Some("sumti"),
                    value: legacy_as_generated_elided_sumti_without_tag_tree_value(
                        maybe_ku,
                        free_modifiers,
                        source,
                        options,
                    ),
                },
            ],
        });
    }
    if let Some((tense_modal, inner_sumti)) = legacy_tense_tagged_linked_sumti_parts(sumti) {
        return TreeValue::Node(TreeNode {
            constructor: "TenseTaggedRelativeSumti",
            entries: vec![
                TreeEntry {
                    label: Some("tense_modal"),
                    value: legacy_as_generated_tense_modal_tree_value(tense_modal, source, options),
                },
                TreeEntry {
                    label: Some("sumti"),
                    value: legacy_as_generated_tagged_or_elided_sumti_tree_value(
                        inner_sumti,
                        source,
                        options,
                    ),
                },
            ],
        });
    }
    legacy_as_generated_sumti_tree_value(sumti, source, options)
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_relative_clause_list_tree_values(
    relative_clauses: &[jbotci_syntax::ast::RelativeClauseSyntax],
    source: &str,
    options: TreeRenderOptions,
) -> Vec<TreeValue> {
    let Some((first, additional)) = relative_clauses.split_first() else {
        return Vec::new();
    };
    let mut values = vec![legacy_as_generated_relative_clause_tree_value(
        first, source, options,
    )];
    let additional_values = additional
        .iter()
        .map(|relative_clause| {
            legacy_as_generated_relative_clause_tail_tree_value(relative_clause, source, options)
        })
        .collect::<Vec<_>>();
    if !additional_values.is_empty() {
        values.push(TreeValue::Collection(additional_values));
    }
    values
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_relative_clause_tail_tree_value(
    relative_clause: &jbotci_syntax::ast::RelativeClauseSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    match legacy_as_generated_relative_clause_tree_value(relative_clause, source, options) {
        TreeValue::Collection(mut values) if values.len() == 1 => values.remove(0),
        value => value,
    }
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_mekso_tree_value(
    mekso: &jbotci_syntax::ast::MeksoSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    if let bityzba::data!(jbotci_syntax::ast::MeksoSyntax::ReversePolish {
        fuha,
        operands,
        operators,
    }) = mekso.as_data()
        && let Some(parts) = legacy_as_generated_reverse_polish_parts_tree_value(
            operands, operators, source, options,
        )
    {
        return TreeValue::Node(TreeNode {
            constructor: "ReversePolish",
            entries: vec![
                TreeEntry {
                    label: Some("fuha"),
                    value: required_legacy_syntax_subtree_value(fuha, source, options),
                },
                TreeEntry {
                    label: Some("parts"),
                    value: parts,
                },
            ],
        });
    }

    let (first_expression, continuations) = legacy_mekso_infix_parts(mekso, source, options);
    let mut entries = vec![TreeEntry {
        label: Some("first_expression"),
        value: legacy_as_generated_mekso_precedence_tree_value(first_expression, source, options),
    }];
    if let Some(entry) = labelled_tree_collection_entry_from_values("continuations", continuations)
    {
        entries.push(entry);
    }
    TreeValue::Node(TreeNode {
        constructor: "InfixMekso",
        entries,
    })
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_reverse_polish_parts_tree_value(
    operands: &[jbotci_syntax::ast::MeksoSyntax],
    operators: &[jbotci_syntax::ast::MeksoOperatorSyntax],
    source: &str,
    options: TreeRenderOptions,
) -> Option<TreeValue> {
    let expression = legacy_reverse_polish_expression_tree(operands, operators)?;
    Some(
        legacy_as_generated_reverse_polish_expression_parts_tree_value(
            &expression,
            source,
            options,
        ),
    )
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_reverse_polish_expression_parts_tree_value(
    expression: &LegacyReversePolishExpr,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    let (first_operand, tails) =
        legacy_reverse_polish_expression_parts(expression, source, options);
    let mut entries = vec![TreeEntry {
        label: Some("first_operand"),
        value: first_operand,
    }];
    if let Some(entry) = labelled_tree_collection_entry_from_values("tails", tails) {
        entries.push(entry);
    }

    TreeValue::Node(TreeNode {
        constructor: "ReversePolishParts",
        entries,
    })
}

#[requires(true)]
#[ensures(true)]
fn legacy_reverse_polish_expression_parts(
    expression: &LegacyReversePolishExpr,
    source: &str,
    options: TreeRenderOptions,
) -> (TreeValue, Vec<TreeValue>) {
    match expression.as_data() {
        bityzba::data!(LegacyReversePolishExpr::Operand(operand)) => (
            legacy_as_generated_mekso_operand_tree_value(operand, source, options),
            Vec::new(),
        ),
        bityzba::data!(LegacyReversePolishExpr::Operation {
            left,
            right,
            operator,
        }) => {
            let (first_operand, mut tails) =
                legacy_reverse_polish_expression_parts(left, source, options);
            tails.push(TreeValue::Collection(vec![
                legacy_as_generated_reverse_polish_expression_parts_tree_value(
                    right, source, options,
                ),
                legacy_as_generated_mekso_operator_tree_value(operator, source, options),
            ]));
            (first_operand, tails)
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn legacy_reverse_polish_expression_tree<'tree>(
    operands: &'tree [jbotci_syntax::ast::MeksoSyntax],
    operators: &'tree [jbotci_syntax::ast::MeksoOperatorSyntax],
) -> Option<LegacyReversePolishExpr> {
    if operands.len() != operators.len() + 1 {
        return None;
    }
    let mut items = operands
        .iter()
        .map(|operand| {
            legacy_first_token_byte_start(operand).map(|start| {
                bityzba::new!(LegacyReversePolishItem::Operand {
                    start: start,
                    operand: operand.clone(),
                })
            })
        })
        .chain(operators.iter().map(|operator| {
            legacy_first_token_byte_start(operator).map(|start| {
                bityzba::new!(LegacyReversePolishItem::Operator {
                    start: start,
                    operator: operator.clone(),
                })
            })
        }))
        .collect::<Option<Vec<_>>>()?;
    items.sort_by_key(LegacyReversePolishItem::start);

    let mut stack = Vec::new();
    for item in items {
        match item.into_data() {
            bityzba::data!(LegacyReversePolishItem::Operand { operand, .. }) => {
                stack.push(bityzba::new!(LegacyReversePolishExpr::Operand(operand)));
            }
            bityzba::data!(LegacyReversePolishItem::Operator { operator, .. }) => {
                let right = stack.pop()?;
                let left = stack.pop()?;
                stack.push(bityzba::new!(LegacyReversePolishExpr::Operation {
                    left: Box::new(left),
                    right: Box::new(right),
                    operator: operator,
                }));
            }
        }
    }
    if stack.len() == 1 { stack.pop() } else { None }
}

#[requires(true)]
#[ensures(true)]
fn legacy_first_token_byte_start<T>(node: &T) -> Option<usize>
where
    T: SyntaxAstTreeNode + ?Sized,
{
    let mut visitor = FirstLegacySyntaxTokenStartVisitor { start: None };
    node.visit_in_order(&mut visitor);
    visitor.start
}

#[invariant(true)]
struct FirstLegacySyntaxTokenStartVisitor {
    start: Option<usize>,
}

impl<'tree> TreeVisitor<'tree> for FirstLegacySyntaxTokenStartVisitor {
    type Node = SyntaxNodeRef<'tree>;
    type Atom = SyntaxAtomRef<'tree>;

    #[requires(true)]
    #[ensures(true)]
    fn visit_atom(&mut self, atom: Self::Atom) {
        if self.start.is_some() {
            return;
        }
        self.start = match atom {
            SyntaxAtomRef::Token(token) => token.source_spans().first().map(|span| span.byte_start),
            SyntaxAtomRef::Word(word) => Some(word.span().byte_start),
        };
    }
}

#[requires(true)]
#[ensures(true)]
fn legacy_mekso_infix_parts<'tree>(
    mekso: &'tree jbotci_syntax::ast::MeksoSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> (&'tree jbotci_syntax::ast::MeksoSyntax, Vec<TreeValue>) {
    match mekso.as_data() {
        bityzba::data!(jbotci_syntax::ast::MeksoSyntax::Infix {
            left_expression,
            operator,
            right_expression,
        }) => {
            let (first_expression, mut continuations) =
                legacy_mekso_infix_parts(left_expression.as_ref(), source, options);
            continuations.push(TreeValue::Collection(vec![
                legacy_as_generated_mekso_operator_tree_value(operator.as_ref(), source, options),
                legacy_as_generated_mekso_precedence_tree_value(
                    right_expression.as_ref(),
                    source,
                    options,
                ),
            ]));
            (first_expression, continuations)
        }
        _ => (mekso, Vec::new()),
    }
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_mekso_precedence_tree_value(
    mekso: &jbotci_syntax::ast::MeksoSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    match mekso.as_data() {
        bityzba::data!(jbotci_syntax::ast::MeksoSyntax::PrecedenceInfix {
            left_expression,
            bihe,
            operator,
            right_expression,
        }) => TreeValue::Node(TreeNode {
            constructor: "MeksoPrecedence",
            entries: vec![
                TreeEntry {
                    label: Some("left_expression"),
                    value: legacy_as_generated_mekso_base_tree_value(
                        left_expression.as_ref(),
                        source,
                        options,
                    ),
                },
                TreeEntry {
                    label: Some("tail"),
                    value: TreeValue::Collection(vec![
                        TreeValue::Collection(vec![
                            required_legacy_syntax_subtree_value(bihe, source, options),
                            legacy_as_generated_mekso_operator_tree_value(
                                operator.as_ref(),
                                source,
                                options,
                            ),
                        ]),
                        legacy_as_generated_mekso_precedence_tree_value(
                            right_expression.as_ref(),
                            source,
                            options,
                        ),
                    ]),
                },
            ],
        }),
        _ => TreeValue::Node(TreeNode {
            constructor: "MeksoPrecedence",
            entries: vec![TreeEntry {
                label: Some("left_expression"),
                value: legacy_as_generated_mekso_base_tree_value(mekso, source, options),
            }],
        }),
    }
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_mekso_base_tree_value(
    mekso: &jbotci_syntax::ast::MeksoSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    if matches!(
        mekso.as_data(),
        bityzba::data!(jbotci_syntax::ast::MeksoSyntax::ForethoughtCall { .. })
    ) {
        legacy_as_generated_simple_mekso_operand_tree_value(mekso, source, options)
    } else {
        legacy_as_generated_mekso_operand_tree_value(mekso, source, options)
    }
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_mekso_operand_tree_value(
    mekso: &jbotci_syntax::ast::MeksoSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    if let Some(bound_connection) =
        legacy_as_generated_bound_mekso_operand_connection_tree_value(mekso, source, options)
    {
        return TreeValue::Node(TreeNode {
            constructor: "AfterthoughtMeksoOperand",
            entries: vec![TreeEntry {
                label: Some("leading_expression"),
                value: bound_connection,
            }],
        });
    }
    if let Some((leading_expression, continuations)) = legacy_mekso_connection_parts(mekso) {
        let mut entries = vec![TreeEntry {
            label: Some("leading_expression"),
            value: legacy_as_generated_simple_mekso_operand_tree_value(
                leading_expression,
                source,
                options,
            ),
        }];
        if let Some(entry) = labelled_tree_collection_entry_from_values(
            "continuations",
            continuations
                .iter()
                .map(|(connective, trailing_expression)| {
                    TreeValue::Collection(vec![
                        required_legacy_syntax_subtree_value(*connective, source, options),
                        legacy_as_generated_simple_mekso_operand_tree_value(
                            *trailing_expression,
                            source,
                            options,
                        ),
                    ])
                })
                .collect(),
        ) {
            entries.push(entry);
        }
        return TreeValue::Node(TreeNode {
            constructor: "AfterthoughtMeksoOperand",
            entries,
        });
    }
    TreeValue::Node(TreeNode {
        constructor: "AfterthoughtMeksoOperand",
        entries: vec![TreeEntry {
            label: Some("leading_expression"),
            value: legacy_as_generated_simple_mekso_operand_tree_value(mekso, source, options),
        }],
    })
}

#[requires(true)]
#[ensures(true)]
fn legacy_mekso_connection_parts<'tree>(
    mekso: &'tree jbotci_syntax::ast::MeksoSyntax,
) -> Option<(
    &'tree jbotci_syntax::ast::MeksoSyntax,
    Vec<(
        &'tree jbotci_syntax::ast::ConnectiveSyntax,
        &'tree jbotci_syntax::ast::MeksoSyntax,
    )>,
)> {
    match mekso.as_data() {
        bityzba::data!(jbotci_syntax::ast::MeksoSyntax::MeksoConnection {
            left_expression,
            connective,
            right_expression,
        }) => {
            let (leading, mut continuations) = legacy_mekso_connection_parts(left_expression)
                .unwrap_or((left_expression.as_ref(), Vec::new()));
            if let Some((trailing_leading, mut trailing_continuations)) =
                legacy_mekso_connection_parts(right_expression)
            {
                continuations.push((connective, trailing_leading));
                continuations.append(&mut trailing_continuations);
            } else {
                continuations.push((connective, right_expression.as_ref()));
            }
            Some((leading, continuations))
        }
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_bound_mekso_operand_connection_tree_value(
    mekso: &jbotci_syntax::ast::MeksoSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> Option<TreeValue> {
    let bityzba::data!(jbotci_syntax::ast::MeksoSyntax::MeksoConnection {
        left_expression,
        connective,
        right_expression,
    }) = mekso.as_data()
    else {
        return None;
    };
    let bityzba::data!(jbotci_syntax::ast::ConnectiveSyntax::Afterthought {
        se: None,
        nahe: None,
        na: None,
        cmavo,
        nai: None,
    }) = connective.as_data()
    else {
        return None;
    };
    let words = cmavo.value.as_slice();
    let [operand_connective, tense_words @ .., bo] = words else {
        return None;
    };
    if !bo.is_cmavo(Cmavo::Bo) {
        return None;
    }

    let mut entries = vec![
        TreeEntry {
            label: Some("left_expression"),
            value: legacy_as_generated_simple_mekso_operand_tree_value(
                left_expression.as_ref(),
                source,
                options,
            ),
        },
        TreeEntry {
            label: Some("operand_connective"),
            value: generated_token_tree_value(operand_connective, source, options),
        },
    ];
    if !tense_words.is_empty() {
        entries.push(TreeEntry {
            label: Some("tense_modal"),
            value: legacy_as_generated_tense_modal_words_tree_value(tense_words, source, options)?,
        });
    }
    entries.push(TreeEntry {
        label: Some("bo"),
        value: generated_token_tree_value(bo, source, options),
    });
    entries.push(TreeEntry {
        label: Some("right_expression"),
        value: legacy_as_generated_mekso_operand_tree_value(
            right_expression.as_ref(),
            source,
            options,
        ),
    });

    Some(TreeValue::Node(TreeNode {
        constructor: "BoundMeksoOperandConnection",
        entries,
    }))
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_tense_modal_words_tree_value(
    words: &[Token],
    source: &str,
    options: TreeRenderOptions,
) -> Option<TreeValue> {
    let tokens = words.iter().collect::<Vec<_>>();
    let mut index = 0;
    let value =
        legacy_as_generated_connected_tense_atom_tree_value(&tokens, &mut index, source, options)?;
    if index == tokens.len() {
        Some(value)
    } else {
        None
    }
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_simple_mekso_operand_tree_value(
    mekso: &jbotci_syntax::ast::MeksoSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    match mekso.as_data() {
        bityzba::data!(jbotci_syntax::ast::MeksoSyntax::NumberMekso(quantifier)) => {
            legacy_as_generated_quantifier_tree_value(quantifier.as_ref(), source, options)
        }
        bityzba::data!(jbotci_syntax::ast::MeksoSyntax::LerfuStringMekso { letter, boi }) => {
            let mut entries = vec![TreeEntry {
                label: Some("letters"),
                value: legacy_word_run_tree_value(&letter.value, source, options),
            }];
            if let Some(entry) = labelled_tree_collection_entry_from_values(
                "free_modifiers",
                letter
                    .free_modifiers
                    .iter()
                    .map(|free_modifier| {
                        legacy_as_generated_free_modifier_tree_value(free_modifier, source, options)
                    })
                    .collect(),
            ) {
                entries.push(entry);
            }
            if let Some(boi) = boi {
                entries.extend(legacy_token_field_entries("boi", boi, source, options));
            }
            TreeValue::Node(TreeNode {
                constructor: "LerfuStringMekso",
                entries,
            })
        }
        bityzba::data!(jbotci_syntax::ast::MeksoSyntax::ParenthesizedMekso {
            vei,
            inner_expression,
            veho,
        }) => {
            let mut entries = legacy_token_field_entries("vei", vei, source, options);
            entries.push(TreeEntry {
                label: Some("inner_expression"),
                value: legacy_as_generated_mekso_tree_value(
                    inner_expression.as_ref(),
                    source,
                    options,
                ),
            });
            if let Some(veho) = veho {
                entries.extend(legacy_token_field_entries("veho", veho, source, options));
            }
            TreeValue::Node(TreeNode {
                constructor: "ParenthesizedMekso",
                entries,
            })
        }
        bityzba::data!(jbotci_syntax::ast::MeksoSyntax::ForethoughtCall {
            peho,
            operator,
            operands,
            kuhe,
        }) => {
            let mut entries = Vec::new();
            if let Some(peho) = peho {
                entries.extend(legacy_token_field_entries("peho", peho, source, options));
            }
            entries.push(TreeEntry {
                label: Some("operator"),
                value: legacy_as_generated_mekso_operator_tree_value(
                    operator.as_ref(),
                    source,
                    options,
                ),
            });
            entries.push(TreeEntry {
                label: Some("operands"),
                value: TreeValue::Collection(
                    operands
                        .iter()
                        .map(|operand| {
                            legacy_as_generated_mekso_base_tree_value(operand, source, options)
                        })
                        .collect(),
                ),
            });
            if let Some(kuhe) = kuhe {
                entries.extend(legacy_token_field_entries("kuhe", kuhe, source, options));
            }
            TreeValue::Node(TreeNode {
                constructor: "ForethoughtCall",
                entries,
            })
        }
        bityzba::data!(
            jbotci_syntax::ast::MeksoSyntax::ForethoughtMeksoConnection {
                gek,
                left_expression,
                gik,
                right_expression,
            }
        ) => TreeValue::Node(TreeNode {
            constructor: "ForethoughtMeksoConnection",
            entries: vec![
                TreeEntry {
                    label: Some("gek"),
                    value: legacy_as_generated_connective_tree_value(gek, source, options),
                },
                TreeEntry {
                    label: Some("left_expression"),
                    value: legacy_as_generated_mekso_operand_tree_value(
                        left_expression.as_ref(),
                        source,
                        options,
                    ),
                },
                TreeEntry {
                    label: Some("gik"),
                    value: required_legacy_syntax_subtree_value(gik, source, options),
                },
                TreeEntry {
                    label: Some("right_expression"),
                    value: legacy_as_generated_mekso_operand_tree_value(
                        right_expression.as_ref(),
                        source,
                        options,
                    ),
                },
            ],
        }),
        bityzba::data!(jbotci_syntax::ast::MeksoSyntax::SelbriOperand { nihe, selbri, tehu }) => {
            let mut entries = legacy_token_field_entries("nihe", nihe, source, options);
            entries.push(TreeEntry {
                label: Some("selbri"),
                value: legacy_as_generated_selbri_tree_value(selbri.as_ref(), source, options),
            });
            if let Some(tehu) = tehu {
                entries.extend(legacy_token_field_entries("tehu", tehu, source, options));
            }
            TreeValue::Node(TreeNode {
                constructor: "SelbriOperand",
                entries,
            })
        }
        bityzba::data!(jbotci_syntax::ast::MeksoSyntax::SumtiOperand { mohe, sumti, tehu }) => {
            let mut entries = legacy_token_field_entries("mohe", mohe, source, options);
            entries.push(TreeEntry {
                label: Some("sumti"),
                value: legacy_as_generated_sumti_tree_value(sumti.as_ref(), source, options),
            });
            if let Some(tehu) = tehu {
                entries.extend(legacy_token_field_entries("tehu", tehu, source, options));
            }
            TreeValue::Node(TreeNode {
                constructor: "SumtiOperand",
                entries,
            })
        }
        bityzba::data!(jbotci_syntax::ast::MeksoSyntax::MeksoArray {
            johi,
            expressions,
            tehu,
        }) => {
            let mut entries = legacy_token_field_entries("johi", johi, source, options);
            entries.push(TreeEntry {
                label: Some("expressions"),
                value: TreeValue::Collection(
                    expressions
                        .iter()
                        .map(|expression| {
                            legacy_as_generated_mekso_tree_value(expression, source, options)
                        })
                        .collect(),
                ),
            });
            if let Some(tehu) = tehu {
                entries.extend(legacy_token_field_entries("tehu", tehu, source, options));
            }
            TreeValue::Node(TreeNode {
                constructor: "MeksoArray",
                entries,
            })
        }
        bityzba::data!(jbotci_syntax::ast::MeksoSyntax::QualifiedOperand {
            markers,
            inner_expression,
            luhu,
        }) => {
            let mut entries = vec![
                TreeEntry {
                    label: Some("markers"),
                    value: TreeValue::Collection(
                        markers
                            .value
                            .iter()
                            .map(|token| generated_token_tree_value(token, source, options))
                            .chain(markers.free_modifiers.iter().map(|free_modifier| {
                                legacy_as_generated_free_modifier_tree_value(
                                    free_modifier,
                                    source,
                                    options,
                                )
                            }))
                            .collect(),
                    ),
                },
                TreeEntry {
                    label: Some("inner_expression"),
                    value: legacy_as_generated_mekso_operand_tree_value(
                        inner_expression.as_ref(),
                        source,
                        options,
                    ),
                },
            ];
            if let Some(luhu) = luhu {
                entries.extend(legacy_token_field_entries("luhu", luhu, source, options));
            }
            TreeValue::Node(TreeNode {
                constructor: "QualifiedOperand",
                entries,
            })
        }
        _ => required_legacy_syntax_subtree_value(mekso, source, options),
    }
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_mekso_operator_tree_value(
    operator: &jbotci_syntax::ast::MeksoOperatorSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    if let Some(forethought_operator) =
        legacy_as_generated_forethought_mekso_operator_tree_value(operator, source, options)
    {
        return TreeValue::Node(TreeNode {
            constructor: "AfterthoughtMeksoOperator",
            entries: vec![TreeEntry {
                label: Some("leading_operator"),
                value: forethought_operator,
            }],
        });
    }
    if let Some((leading_operator, continuations)) =
        legacy_mekso_operator_connection_parts(operator)
    {
        let mut entries = vec![TreeEntry {
            label: Some("leading_operator"),
            value: legacy_as_generated_simple_mekso_operator_tree_value(
                leading_operator,
                source,
                options,
            ),
        }];
        if let Some(entry) = labelled_tree_collection_entry_from_values(
            "continuations",
            continuations
                .iter()
                .map(|(connective, trailing_operator)| {
                    TreeValue::Collection(vec![
                        legacy_as_generated_connective_tree_value(connective, source, options),
                        legacy_as_generated_simple_mekso_operator_tree_value(
                            trailing_operator,
                            source,
                            options,
                        ),
                    ])
                })
                .collect(),
        ) {
            entries.push(entry);
        }
        return TreeValue::Node(TreeNode {
            constructor: "AfterthoughtMeksoOperator",
            entries,
        });
    }
    TreeValue::Node(TreeNode {
        constructor: "AfterthoughtMeksoOperator",
        entries: vec![TreeEntry {
            label: Some("leading_operator"),
            value: legacy_as_generated_simple_mekso_operator_tree_value(operator, source, options),
        }],
    })
}

#[requires(true)]
#[ensures(true)]
fn legacy_mekso_operator_connection_parts<'tree>(
    operator: &'tree jbotci_syntax::ast::MeksoOperatorSyntax,
) -> Option<(
    &'tree jbotci_syntax::ast::MeksoOperatorSyntax,
    Vec<(
        &'tree jbotci_syntax::ast::ConnectiveSyntax,
        &'tree jbotci_syntax::ast::MeksoOperatorSyntax,
    )>,
)> {
    match operator.as_data() {
        bityzba::data!(
            jbotci_syntax::ast::MeksoOperatorSyntax::OperatorConnection {
                left_operator,
                connective,
                right_operator,
            }
        ) if legacy_connective_is_guhek_gik_forethought(connective) => None,
        bityzba::data!(
            jbotci_syntax::ast::MeksoOperatorSyntax::OperatorConnection {
                left_operator,
                connective,
                right_operator,
            }
        ) => {
            let (leading, mut continuations) =
                legacy_mekso_operator_connection_parts(left_operator)
                    .unwrap_or((left_operator.as_ref(), Vec::new()));
            if let Some((trailing_leading, mut trailing_continuations)) =
                legacy_mekso_operator_connection_parts(right_operator)
            {
                continuations.push((connective, trailing_leading));
                continuations.append(&mut trailing_continuations);
            } else {
                continuations.push((connective, right_operator.as_ref()));
            }
            Some((leading, continuations))
        }
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_forethought_mekso_operator_tree_value(
    operator: &jbotci_syntax::ast::MeksoOperatorSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> Option<TreeValue> {
    let bityzba::data!(
        jbotci_syntax::ast::MeksoOperatorSyntax::OperatorConnection {
            left_operator,
            connective,
            right_operator,
        }
    ) = operator.as_data()
    else {
        return None;
    };
    let (guhek, gik) = legacy_as_generated_guhek_gik_connective_pair(connective, source, options)?;
    Some(TreeValue::Node(TreeNode {
        constructor: "ForethoughtMeksoOperator",
        entries: vec![
            TreeEntry {
                label: Some("guhek"),
                value: guhek,
            },
            TreeEntry {
                label: Some("left_operator"),
                value: legacy_as_generated_mekso_operator_tree_value(
                    left_operator.as_ref(),
                    source,
                    options,
                ),
            },
            TreeEntry {
                label: Some("gik"),
                value: gik,
            },
            TreeEntry {
                label: Some("right_operator"),
                value: legacy_as_generated_mekso_operator_tree_value(
                    right_operator.as_ref(),
                    source,
                    options,
                ),
            },
        ],
    }))
}

#[requires(true)]
#[ensures(true)]
fn legacy_connective_is_guhek_gik_forethought(
    connective: &jbotci_syntax::ast::ConnectiveSyntax,
) -> bool {
    let bityzba::data!(jbotci_syntax::ast::ConnectiveSyntax::Forethought { cmavo, .. }) =
        connective.as_data()
    else {
        return false;
    };
    matches!(
        cmavo.value.as_slice(),
        [guha, gi] if guha.is_selmaho(Selmaho::Guha) && gi.is_cmavo(Cmavo::Gi)
    )
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_guhek_gik_connective_pair(
    connective: &jbotci_syntax::ast::ConnectiveSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> Option<(TreeValue, TreeValue)> {
    let bityzba::data!(jbotci_syntax::ast::ConnectiveSyntax::Forethought {
        se,
        nahe,
        na: None,
        cmavo,
        nai,
    }) = connective.as_data()
    else {
        return None;
    };
    let [guha, gi] = cmavo.value.as_slice() else {
        return None;
    };
    if !guha.is_selmaho(Selmaho::Guha) || !gi.is_cmavo(Cmavo::Gi) {
        return None;
    }
    let mut guhek_entries = Vec::new();
    if let Some(nahe) = nahe {
        guhek_entries.push(TreeEntry {
            label: Some("nahe"),
            value: generated_token_tree_value(nahe, source, options),
        });
    }
    if let Some(se) = se {
        guhek_entries.push(TreeEntry {
            label: Some("se"),
            value: generated_token_tree_value(se, source, options),
        });
    }
    guhek_entries.push(TreeEntry {
        label: None,
        value: generated_token_tree_value(guha, source, options),
    });
    if let Some(nai) = nai {
        guhek_entries.push(TreeEntry {
            label: Some("nai"),
            value: required_legacy_syntax_subtree_value(nai.as_ref(), source, options),
        });
    }
    let guhek = TreeValue::Node(TreeNode {
        constructor: "Forethought",
        entries: guhek_entries,
    });
    let gik = TreeValue::Node(TreeNode {
        constructor: "Forethought",
        entries: vec![TreeEntry {
            label: None,
            value: generated_token_tree_value(gi, source, options),
        }],
    });
    Some((guhek, gik))
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_simple_mekso_operator_tree_value(
    operator: &jbotci_syntax::ast::MeksoOperatorSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    match operator.as_data() {
        bityzba::data!(jbotci_syntax::ast::MeksoOperatorSyntax::Primitive(vuhu)) => {
            required_legacy_syntax_subtree_value(vuhu, source, options)
        }
        bityzba::data!(jbotci_syntax::ast::MeksoOperatorSyntax::OperandAsOperator {
            maho,
            mekso,
            tehu,
        }) => {
            let mut entries = legacy_token_field_entries("maho", maho, source, options);
            entries.push(TreeEntry {
                label: Some("mekso"),
                value: legacy_as_generated_mekso_tree_value(mekso.as_ref(), source, options),
            });
            if let Some(tehu) = tehu {
                entries.extend(legacy_token_field_entries("tehu", tehu, source, options));
            }
            TreeValue::Node(TreeNode {
                constructor: "OperandAsOperator",
                entries,
            })
        }
        bityzba::data!(jbotci_syntax::ast::MeksoOperatorSyntax::Converted {
            se,
            inner_operator,
        }) => TreeValue::Node(TreeNode {
            constructor: "Converted",
            entries: vec![
                TreeEntry {
                    label: Some("se"),
                    value: required_legacy_syntax_subtree_value(se, source, options),
                },
                TreeEntry {
                    label: Some("inner_operator"),
                    value: legacy_as_generated_mekso_operator_tree_value(
                        inner_operator.as_ref(),
                        source,
                        options,
                    ),
                },
            ],
        }),
        bityzba::data!(jbotci_syntax::ast::MeksoOperatorSyntax::ScalarNegated {
            nahe,
            inner_operator,
        }) => TreeValue::Node(TreeNode {
            constructor: "ScalarNegated",
            entries: vec![
                TreeEntry {
                    label: Some("nahe"),
                    value: required_legacy_syntax_subtree_value(nahe, source, options),
                },
                TreeEntry {
                    label: Some("inner_operator"),
                    value: legacy_as_generated_mekso_operator_tree_value(
                        inner_operator.as_ref(),
                        source,
                        options,
                    ),
                },
            ],
        }),
        bityzba::data!(jbotci_syntax::ast::MeksoOperatorSyntax::SelbriAsOperator {
            nahu,
            selbri,
            tehu,
        }) => {
            let mut entries = legacy_token_field_entries("nahu", nahu, source, options);
            entries.push(TreeEntry {
                label: Some("selbri"),
                value: legacy_as_generated_mekso_operator_selbri_tree_value(
                    selbri.as_ref(),
                    source,
                    options,
                ),
            });
            if let Some(tehu) = tehu {
                entries.extend(legacy_token_field_entries("tehu", tehu, source, options));
            }
            TreeValue::Node(TreeNode {
                constructor: "SelbriAsOperator",
                entries,
            })
        }
        bityzba::data!(jbotci_syntax::ast::MeksoOperatorSyntax::GroupedOperator {
            ke,
            inner_operator,
            kehe,
        }) => {
            let mut entries = legacy_token_field_entries("ke", ke, source, options);
            entries.push(TreeEntry {
                label: Some("inner_operator"),
                value: legacy_as_generated_mekso_operator_tree_value(
                    inner_operator.as_ref(),
                    source,
                    options,
                ),
            });
            if let Some(kehe) = kehe {
                entries.extend(legacy_token_field_entries("kehe", kehe, source, options));
            }
            TreeValue::Node(TreeNode {
                constructor: "GroupedOperator",
                entries,
            })
        }
        bityzba::data!(
            jbotci_syntax::ast::MeksoOperatorSyntax::BoundOperatorConnection {
                left_operator,
                connective,
                bo,
                right_operator,
            }
        ) => TreeValue::Node(TreeNode {
            constructor: "BoundOperatorConnection",
            entries: vec![
                TreeEntry {
                    label: Some("left_operator"),
                    value: legacy_as_generated_simple_mekso_operator_tree_value(
                        left_operator.as_ref(),
                        source,
                        options,
                    ),
                },
                TreeEntry {
                    label: Some("connective"),
                    value: legacy_as_generated_connective_tree_value(connective, source, options),
                },
                TreeEntry {
                    label: Some("bo"),
                    value: required_legacy_syntax_subtree_value(bo, source, options),
                },
                TreeEntry {
                    label: Some("right_operator"),
                    value: legacy_as_generated_mekso_operator_tree_value(
                        right_operator.as_ref(),
                        source,
                        options,
                    ),
                },
            ],
        }),
        _ => required_legacy_syntax_subtree_value(operator, source, options),
    }
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_mekso_operator_selbri_tree_value(
    selbri: &jbotci_syntax::ast::SelbriSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    if let bityzba::data!(jbotci_syntax::ast::SelbriSyntax::SelbriWord(word)) = selbri.as_data()
        && word.is_selmaho(Selmaho::Goha)
    {
        return TreeValue::Node(TreeNode {
            constructor: "CoSelbri",
            entries: vec![TreeEntry {
                label: Some("leading_selbri"),
                value: TreeValue::Node(TreeNode {
                    constructor: "ConnectedSelbri",
                    entries: vec![TreeEntry {
                        label: Some("leading_selbri"),
                        value: TreeValue::Node(TreeNode {
                            constructor: "TanruSelbri",
                            entries: vec![TreeEntry {
                                label: Some("first_unit"),
                                value: TreeValue::Node(TreeNode {
                                    constructor: "ConnectedTanruUnit",
                                    entries: vec![TreeEntry {
                                        label: Some("leading_unit"),
                                        value: TreeValue::Node(TreeNode {
                                            constructor: "LinkedTanruUnit",
                                            entries: vec![TreeEntry {
                                                label: Some("base"),
                                                value: TreeValue::Node(TreeNode {
                                                    constructor: "TanruUnitAtom",
                                                    entries: vec![TreeEntry {
                                                        label: Some("base"),
                                                        value: TreeValue::Node(TreeNode {
                                                            constructor: "ProBridi",
                                                            entries: vec![TreeEntry {
                                                                label: Some("goha"),
                                                                value: generated_token_tree_value(
                                                                    word, source, options,
                                                                ),
                                                            }],
                                                        }),
                                                    }],
                                                }),
                                            }],
                                        }),
                                    }],
                                }),
                            }],
                        }),
                    }],
                }),
            }],
        });
    }
    legacy_as_generated_selbri_tree_value(selbri, source, options)
}

#[requires(true)]
#[ensures(true)]
fn legacy_word_run_with_free_modifiers_tree_value(
    words: &WithFreeModifiers<jbotci_syntax::ast::WordRun>,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    let mut values = words
        .value
        .iter()
        .map(|token| generated_token_tree_value(token, source, options))
        .collect::<Vec<_>>();
    values.extend(words.free_modifiers.iter().map(|free_modifier| {
        legacy_as_generated_free_modifier_tree_value(free_modifier, source, options)
    }));
    TreeValue::Collection(values)
}

#[requires(true)]
#[ensures(true)]
fn legacy_word_run_tree_value(
    words: &jbotci_syntax::ast::WordRun,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    TreeValue::Collection(
        words
            .iter()
            .map(|token| generated_token_tree_value(token, source, options))
            .collect(),
    )
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_subscript_expression_tree_value(
    expression: &jbotci_syntax::ast::MeksoSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    if let bityzba::data!(jbotci_syntax::ast::MeksoSyntax::NumberMekso(quantifier)) =
        expression.as_data()
        && let bityzba::data!(jbotci_syntax::ast::QuantifierSyntax::NumberQuantifier {
            number,
            boi,
        }) = quantifier.as_data()
    {
        let mut entries = vec![TreeEntry {
            label: Some("words"),
            value: legacy_word_run_tree_value(&number.value, source, options),
        }];
        if let Some(boi) = boi {
            entries.extend(legacy_token_field_entries("boi", boi, source, options));
        }
        if let Some(entry) = labelled_tree_collection_entry_from_values(
            "free_modifiers",
            number
                .free_modifiers
                .iter()
                .map(|free_modifier| {
                    legacy_as_generated_free_modifier_tree_value(free_modifier, source, options)
                })
                .collect(),
        ) {
            entries.push(entry);
        }
        return TreeValue::Node(TreeNode {
            constructor: "NumberOrLetterMekso",
            entries,
        });
    }

    legacy_as_generated_mekso_tree_value(expression, source, options)
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_selbri_tree_value(
    selbri: &jbotci_syntax::ast::SelbriSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    match selbri.as_data() {
        bityzba::data!(jbotci_syntax::ast::SelbriSyntax::TaggedSelbri {
            tense_modal,
            inner_selbri,
        }) => TreeValue::Node(TreeNode {
            constructor: "TaggedSelbri",
            entries: vec![
                TreeEntry {
                    label: Some("tense_modal"),
                    value: legacy_as_generated_tense_modal_tree_value(
                        tense_modal.as_ref(),
                        source,
                        options,
                    ),
                },
                TreeEntry {
                    label: Some("inner_selbri"),
                    value: legacy_as_generated_selbri_tree_value(
                        inner_selbri.as_ref(),
                        source,
                        options,
                    ),
                },
            ],
        }),
        bityzba::data!(jbotci_syntax::ast::SelbriSyntax::Negated { na, inner_selbri }) => {
            let mut entries = legacy_token_field_entries("na", na, source, options);
            entries.push(TreeEntry {
                label: Some("inner_selbri"),
                value: legacy_as_generated_selbri_tree_value(
                    inner_selbri.as_ref(),
                    source,
                    options,
                ),
            });
            TreeValue::Node(TreeNode {
                constructor: "Negated",
                entries,
            })
        }
        _ => legacy_as_generated_untagged_selbri_tree_value(selbri, source, options),
    }
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_untagged_selbri_tree_value(
    selbri: &jbotci_syntax::ast::SelbriSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    if let bityzba::data!(jbotci_syntax::ast::SelbriSyntax::InvertedTanru {
        leading_selbri,
        co,
        trailing_selbri,
    }) = selbri.as_data()
    {
        return TreeValue::Node(TreeNode {
            constructor: "CoSelbri",
            entries: vec![
                TreeEntry {
                    label: Some("leading_selbri"),
                    value: legacy_as_generated_connected_selbri_tree_value(
                        leading_selbri.as_ref(),
                        source,
                        options,
                    ),
                },
                TreeEntry {
                    label: Some("co_tail"),
                    value: TreeValue::Collection(vec![
                        required_legacy_syntax_subtree_value(co, source, options),
                        legacy_as_generated_untagged_selbri_tree_value(
                            trailing_selbri.as_ref(),
                            source,
                            options,
                        ),
                    ]),
                },
            ],
        });
    }
    TreeValue::Node(TreeNode {
        constructor: "CoSelbri",
        entries: vec![TreeEntry {
            label: Some("leading_selbri"),
            value: legacy_as_generated_connected_selbri_tree_value(selbri, source, options),
        }],
    })
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_connected_selbri_tree_value(
    selbri: &jbotci_syntax::ast::SelbriSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    TreeValue::Node(TreeNode {
        constructor: "ConnectedSelbri",
        entries: vec![TreeEntry {
            label: Some("leading_selbri"),
            value: legacy_as_generated_tanru_selbri_tree_value(selbri, source, options),
        }],
    })
}

#[requires(true)]
#[ensures(true)]
fn legacy_selbri_connection_parts<'tree>(
    selbri: &'tree jbotci_syntax::ast::SelbriSyntax,
) -> Option<(
    &'tree jbotci_syntax::ast::SelbriSyntax,
    Vec<(
        &'tree jbotci_syntax::ast::ConnectiveSyntax,
        &'tree jbotci_syntax::ast::SelbriSyntax,
    )>,
)> {
    match selbri.as_data() {
        bityzba::data!(jbotci_syntax::ast::SelbriSyntax::SelbriConnection {
            leading_selbri,
            connective,
            trailing_selbri,
        }) => {
            let (leading, mut continuations) = legacy_selbri_connection_parts(leading_selbri)
                .unwrap_or((leading_selbri.as_ref(), Vec::new()));
            if let Some((trailing_leading, mut trailing_continuations)) =
                legacy_selbri_connection_parts(trailing_selbri)
            {
                continuations.push((connective, trailing_leading));
                continuations.append(&mut trailing_continuations);
            } else {
                continuations.push((connective, trailing_selbri.as_ref()));
            }
            Some((leading, continuations))
        }
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_tanru_selbri_tree_value(
    selbri: &jbotci_syntax::ast::SelbriSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    if let Some((leading_selbri, continuations)) = legacy_selbri_connection_parts(selbri) {
        return TreeValue::Node(TreeNode {
            constructor: "TanruSelbri",
            entries: vec![TreeEntry {
                label: Some("first_unit"),
                value: legacy_as_generated_connected_tanru_unit_from_selbri_parts_tree_value(
                    leading_selbri,
                    &continuations,
                    source,
                    options,
                ),
            }],
        });
    }
    let units = legacy_selbri_tanru_units(selbri);
    let mut entries = Vec::new();
    if let Some((first, additional)) = units.split_first() {
        entries.push(TreeEntry {
            label: Some("first_unit"),
            value: legacy_as_generated_connected_tanru_unit_tree_value(*first, source, options),
        });
        if let Some(entry) = labelled_tree_collection_entry_from_values(
            "additional_units",
            additional
                .iter()
                .map(|unit| {
                    legacy_as_generated_connected_tanru_unit_tree_value(*unit, source, options)
                })
                .collect(),
        ) {
            entries.push(entry);
        }
    } else {
        entries.push(TreeEntry {
            label: Some("first_unit"),
            value: legacy_as_generated_connected_tanru_unit_tree_value(selbri, source, options),
        });
    }
    TreeValue::Node(TreeNode {
        constructor: "TanruSelbri",
        entries,
    })
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_connected_tanru_unit_from_selbri_parts_tree_value(
    leading_selbri: &jbotci_syntax::ast::SelbriSyntax,
    continuations: &[(
        &jbotci_syntax::ast::ConnectiveSyntax,
        &jbotci_syntax::ast::SelbriSyntax,
    )],
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    let mut entries = vec![TreeEntry {
        label: Some("leading_unit"),
        value: leading_selbri.bo_or_linked_tanru_unit_tree_value(source, options),
    }];
    if let Some(entry) = labelled_tree_collection_entry_from_values(
        "continuations",
        continuations
            .iter()
            .map(|(connective, trailing_selbri)| {
                TreeValue::Collection(vec![
                    required_legacy_syntax_subtree_value(*connective, source, options),
                    trailing_selbri.bo_or_linked_tanru_unit_tree_value(source, options),
                ])
            })
            .collect(),
    ) {
        entries.push(entry);
    }
    TreeValue::Node(TreeNode {
        constructor: "ConnectedTanruUnit",
        entries,
    })
}

#[requires(true)]
#[ensures(true)]
fn legacy_selbri_tanru_units(
    selbri: &jbotci_syntax::ast::SelbriSyntax,
) -> Vec<&jbotci_syntax::ast::TanruUnitSyntax> {
    match selbri.as_data() {
        bityzba::data!(jbotci_syntax::ast::SelbriSyntax::Tanru(units)) => units.iter().collect(),
        _ => Vec::new(),
    }
}

#[requires(true)]
#[ensures(true)]
fn legacy_single_selbri_tanru_unit(
    selbri: &jbotci_syntax::ast::SelbriSyntax,
) -> Option<&jbotci_syntax::ast::TanruUnitSyntax> {
    match selbri.as_data() {
        bityzba::data!(jbotci_syntax::ast::SelbriSyntax::Tanru(units)) => {
            let mut iter = units.iter();
            let first = iter.next()?;
            if iter.next().is_none() {
                Some(first)
            } else {
                None
            }
        }
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn legacy_flatten_selbri_conversions<'tree>(
    mut selbri: &'tree jbotci_syntax::ast::SelbriSyntax,
) -> (
    Vec<&'tree WithFreeModifiers<Token>>,
    &'tree jbotci_syntax::ast::SelbriSyntax,
) {
    let mut conversions = Vec::new();
    while let bityzba::data!(jbotci_syntax::ast::SelbriSyntax::ConvertedSelbri {
        se,
        inner_selbri,
    }) = selbri.as_data()
    {
        conversions.push(se);
        selbri = inner_selbri.as_ref();
    }
    (conversions, selbri)
}

#[requires(true)]
#[ensures(true)]
fn legacy_flatten_tanru_unit_conversions<'tree>(
    mut unit: &'tree jbotci_syntax::ast::TanruUnitSyntax,
) -> (
    Vec<&'tree WithFreeModifiers<Token>>,
    &'tree jbotci_syntax::ast::TanruUnitSyntax,
) {
    let mut conversions = Vec::new();
    while let bityzba::data!(jbotci_syntax::ast::TanruUnitSyntax::ConvertedTanruUnit {
        se,
        inner_unit,
    }) = unit.as_data()
    {
        conversions.push(se);
        unit = inner_unit.as_ref();
    }
    (conversions, unit)
}

#[requires(true)]
#[ensures(true)]
fn legacy_conversion_tree_parts(
    conversions: Vec<&WithFreeModifiers<Token>>,
    source: &str,
    options: TreeRenderOptions,
) -> (TreeValue, Vec<TreeValue>) {
    let mut free_modifiers = Vec::new();
    let conversions = conversions
        .into_iter()
        .map(|conversion| {
            free_modifiers.extend(conversion.free_modifiers.iter().map(|free_modifier| {
                legacy_as_generated_free_modifier_tree_value(free_modifier, source, options)
            }));
            generated_token_tree_value(&conversion.value, source, options)
        })
        .collect();
    (TreeValue::Collection(conversions), free_modifiers)
}

#[contract_trait]
trait LegacyTanruUnitLike {
    #[requires(true)]
    #[ensures(true)]
    fn bo_or_linked_tanru_unit_tree_value(
        &self,
        source: &str,
        options: TreeRenderOptions,
    ) -> TreeValue;

    #[requires(true)]
    #[ensures(true)]
    fn linked_tanru_unit_tree_value(&self, source: &str, options: TreeRenderOptions) -> TreeValue;

    #[requires(true)]
    #[ensures(true)]
    fn tanru_unit_connection_parts<'tree>(
        &'tree self,
    ) -> Option<(
        &'tree jbotci_syntax::ast::TanruUnitSyntax,
        Vec<(
            &'tree jbotci_syntax::ast::ConnectiveSyntax,
            &'tree jbotci_syntax::ast::TanruUnitSyntax,
        )>,
    )>;

    #[requires(true)]
    #[ensures(true)]
    fn tanru_unit_atom_tree_value(&self, source: &str, options: TreeRenderOptions) -> TreeValue;

    #[requires(true)]
    #[ensures(true)]
    fn tanru_unit_atom_base_tree_value(
        &self,
        source: &str,
        options: TreeRenderOptions,
    ) -> TreeValue;
}

#[contract_trait]
impl LegacyTanruUnitLike for jbotci_syntax::ast::SelbriSyntax {
    #[requires(true)]
    #[ensures(true)]
    fn bo_or_linked_tanru_unit_tree_value(
        &self,
        source: &str,
        options: TreeRenderOptions,
    ) -> TreeValue {
        match self.as_data() {
            bityzba::data!(jbotci_syntax::ast::SelbriSyntax::BoundSelbriConnection {
                leading_selbri,
                bo_connective,
                bo_tense_modal,
                bo,
                trailing_selbri,
            }) => legacy_as_generated_bound_selbri_connection_tree_value(
                leading_selbri.as_ref(),
                bo_connective.as_deref(),
                bo_tense_modal.as_deref(),
                bo,
                trailing_selbri.as_ref(),
                source,
                options,
            ),
            _ if let Some(unit) = legacy_single_selbri_tanru_unit(self) => {
                unit.bo_or_linked_tanru_unit_tree_value(source, options)
            }
            _ => self.linked_tanru_unit_tree_value(source, options),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn linked_tanru_unit_tree_value(&self, source: &str, options: TreeRenderOptions) -> TreeValue {
        if let bityzba::data!(
            jbotci_syntax::ast::SelbriSyntax::ForethoughtSelbriConnection {
                guhek,
                leading_bridi,
                gik,
                trailing_bridi,
                gihi,
            }
        ) = self.as_data()
            && let Some(value) = legacy_as_generated_forethought_selbri_group_tanru_unit_tree_value(
                guhek,
                leading_bridi.as_ref(),
                gik,
                trailing_bridi.as_ref(),
                gihi.as_ref(),
                source,
                options,
            )
        {
            return value;
        }
        if let Some(unit) = legacy_single_selbri_tanru_unit(self) {
            return unit.linked_tanru_unit_tree_value(source, options);
        }
        TreeValue::Node(TreeNode {
            constructor: "LinkedTanruUnit",
            entries: vec![TreeEntry {
                label: Some("base"),
                value: legacy_as_generated_tanru_unit_atom_tree_value(self, source, options),
            }],
        })
    }

    #[requires(true)]
    #[ensures(ret.is_none())]
    fn tanru_unit_connection_parts<'tree>(
        &'tree self,
    ) -> Option<(
        &'tree jbotci_syntax::ast::TanruUnitSyntax,
        Vec<(
            &'tree jbotci_syntax::ast::ConnectiveSyntax,
            &'tree jbotci_syntax::ast::TanruUnitSyntax,
        )>,
    )> {
        None
    }

    #[requires(true)]
    #[ensures(true)]
    fn tanru_unit_atom_tree_value(&self, source: &str, options: TreeRenderOptions) -> TreeValue {
        match self.as_data() {
            bityzba::data!(jbotci_syntax::ast::SelbriSyntax::ConvertedSelbri { .. }) => {
                let (conversions, base) = legacy_flatten_selbri_conversions(self);
                let (conversions, free_modifiers) =
                    legacy_conversion_tree_parts(conversions, source, options);
                let mut entries = vec![TreeEntry {
                    label: Some("conversions"),
                    value: conversions,
                }];
                if let Some(entry) =
                    labelled_tree_collection_entry_from_values("free_modifiers", free_modifiers)
                {
                    entries.push(entry);
                }
                entries.push(TreeEntry {
                    label: Some("base"),
                    value: base.tanru_unit_atom_base_tree_value(source, options),
                });
                TreeValue::Node(TreeNode {
                    constructor: "TanruUnitAtom",
                    entries,
                })
            }
            _ => TreeValue::Node(TreeNode {
                constructor: "TanruUnitAtom",
                entries: vec![TreeEntry {
                    label: Some("base"),
                    value: self.tanru_unit_atom_base_tree_value(source, options),
                }],
            }),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn tanru_unit_atom_base_tree_value(
        &self,
        source: &str,
        options: TreeRenderOptions,
    ) -> TreeValue {
        match self.as_data() {
            bityzba::data!(jbotci_syntax::ast::SelbriSyntax::SelbriWord(word)) => {
                if legacy_goha_token_renders_as_pro_bridi(word, &[]) {
                    return TreeValue::Node(TreeNode {
                        constructor: "ProBridi",
                        entries: vec![TreeEntry {
                            label: Some("goha"),
                            value: generated_token_tree_value(word, source, options),
                        }],
                    });
                }
                generated_token_tree_value(word, source, options)
            }
            bityzba::data!(jbotci_syntax::ast::SelbriSyntax::Abstraction(abstraction)) => {
                legacy_as_generated_abstraction_tanru_unit_tree_value(
                    abstraction.as_ref(),
                    source,
                    options,
                )
            }
            bityzba::data!(jbotci_syntax::ast::SelbriSyntax::GroupedSelbri {
                ke_tense_modal,
                ke,
                selbri,
                kehe,
            }) => legacy_as_generated_grouped_tanru_unit_tree_value(
                ke_tense_modal.as_deref(),
                ke,
                selbri.as_ref(),
                kehe.as_ref(),
                source,
                options,
            ),
            _ if let Some(unit) = legacy_single_selbri_tanru_unit(self) => {
                unit.tanru_unit_atom_base_tree_value(source, options)
            }
            _ => required_legacy_syntax_subtree_value(self, source, options),
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn legacy_simple_bridi_selbri(
    bridi: &jbotci_syntax::ast::BridiSyntax,
) -> Option<&jbotci_syntax::ast::SelbriSyntax> {
    if !bridi.leading_terms.is_empty() || bridi.cu.is_some() || !bridi.free_modifiers.is_empty() {
        return None;
    }
    let bridi_tail = bridi.bridi_tail.as_ref();
    if bridi_tail.ke_continuation.is_some() {
        return None;
    }
    let afterthought = bridi_tail.first.as_ref();
    if !afterthought.continuations.is_empty() {
        return None;
    }
    let bo_grouped = afterthought.first.as_ref();
    if bo_grouped.bo_continuation.is_some() {
        return None;
    }
    match bo_grouped.first.as_data() {
        bityzba::data!(jbotci_syntax::ast::SimpleBridiTailSyntax::SelbriBridiTail {
            selbri,
            terms,
            vau,
            free_modifiers,
        }) if terms.is_empty() && vau.is_none() && free_modifiers.is_empty() => {
            Some(selbri.as_ref())
        }
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_grouped_tanru_unit_tree_value(
    ke_tense_modal: Option<&jbotci_syntax::ast::TenseModalSyntax>,
    ke: &WithFreeModifiers<Token>,
    selbri: &jbotci_syntax::ast::SelbriSyntax,
    kehe: Option<&WithFreeModifiers<Token>>,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    let mut entries = Vec::new();
    if let Some(ke_tense_modal) = ke_tense_modal {
        entries.push(TreeEntry {
            label: Some("ke_tense_modal"),
            value: legacy_as_generated_tense_modal_tree_value(ke_tense_modal, source, options),
        });
    }
    entries.extend(legacy_token_field_entries("ke", ke, source, options));
    entries.push(TreeEntry {
        label: Some("selbri"),
        value: legacy_as_generated_connected_selbri_tree_value(selbri, source, options),
    });
    if let Some(kehe) = kehe {
        entries.extend(legacy_token_field_entries("kehe", kehe, source, options));
    }
    TreeValue::Node(TreeNode {
        constructor: "GroupedTanruUnit",
        entries,
    })
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_bound_tanru_unit_connection_tree_value(
    leading_unit: &jbotci_syntax::ast::TanruUnitSyntax,
    bo_connective: Option<&jbotci_syntax::ast::ConnectiveSyntax>,
    bo_tense_modal: Option<&jbotci_syntax::ast::TenseModalSyntax>,
    bo: &WithFreeModifiers<Token>,
    trailing_unit: &jbotci_syntax::ast::TanruUnitSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    let mut entries = vec![TreeEntry {
        label: Some("leading_unit"),
        value: legacy_as_generated_linked_tanru_unit_tree_value(leading_unit, source, options),
    }];
    if let Some(bo_connective) = bo_connective {
        entries.push(TreeEntry {
            label: Some("bo_connective"),
            value: legacy_as_generated_connective_tree_value(bo_connective, source, options),
        });
    }
    if let Some(bo_tense_modal) = bo_tense_modal {
        entries.push(TreeEntry {
            label: Some("bo_tense_modal"),
            value: legacy_as_generated_tense_modal_tree_value(bo_tense_modal, source, options),
        });
    }
    entries.extend(legacy_token_field_entries("bo", bo, source, options));
    entries.push(TreeEntry {
        label: Some("trailing_unit"),
        value: trailing_unit.bo_or_linked_tanru_unit_tree_value(source, options),
    });
    TreeValue::Node(TreeNode {
        constructor: "BoundTanruUnitConnection",
        entries,
    })
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_bound_selbri_connection_tree_value(
    leading_selbri: &jbotci_syntax::ast::SelbriSyntax,
    bo_connective: Option<&jbotci_syntax::ast::ConnectiveSyntax>,
    bo_tense_modal: Option<&jbotci_syntax::ast::TenseModalSyntax>,
    bo: &WithFreeModifiers<Token>,
    trailing_selbri: &jbotci_syntax::ast::SelbriSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    let mut entries = vec![TreeEntry {
        label: Some("leading_unit"),
        value: legacy_as_generated_linked_tanru_unit_tree_value(leading_selbri, source, options),
    }];
    if let Some(bo_connective) = bo_connective {
        entries.push(TreeEntry {
            label: Some("bo_connective"),
            value: legacy_as_generated_connective_tree_value(bo_connective, source, options),
        });
    }
    if let Some(bo_tense_modal) = bo_tense_modal {
        entries.push(TreeEntry {
            label: Some("bo_tense_modal"),
            value: legacy_as_generated_tense_modal_tree_value(bo_tense_modal, source, options),
        });
    }
    entries.extend(legacy_token_field_entries("bo", bo, source, options));
    entries.push(TreeEntry {
        label: Some("trailing_unit"),
        value: trailing_selbri.bo_or_linked_tanru_unit_tree_value(source, options),
    });
    TreeValue::Node(TreeNode {
        constructor: "BoundTanruUnitConnection",
        entries,
    })
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_forethought_selbri_group_tanru_unit_tree_value(
    guhek: &jbotci_syntax::ast::ConnectiveSyntax,
    leading_bridi: &jbotci_syntax::ast::BridiSyntax,
    gik: &jbotci_syntax::ast::ConnectiveSyntax,
    trailing_bridi: &jbotci_syntax::ast::BridiSyntax,
    gihi: Option<&Token>,
    source: &str,
    options: TreeRenderOptions,
) -> Option<TreeValue> {
    let (Some(leading_selbri), Some(trailing_selbri)) = (
        legacy_simple_bridi_selbri(leading_bridi),
        legacy_simple_bridi_selbri(trailing_bridi),
    ) else {
        return None;
    };
    let mut entries = vec![
        TreeEntry {
            label: Some("guhek"),
            value: required_legacy_syntax_subtree_value(guhek, source, options),
        },
        TreeEntry {
            label: Some("leading_selbri"),
            value: legacy_as_generated_selbri_tree_value(leading_selbri, source, options),
        },
        TreeEntry {
            label: Some("gik"),
            value: required_legacy_syntax_subtree_value(gik, source, options),
        },
        TreeEntry {
            label: Some("trailing_unit"),
            value: trailing_selbri.bo_or_linked_tanru_unit_tree_value(source, options),
        },
    ];
    if let Some(gihi) = gihi {
        entries.push(TreeEntry {
            label: Some("gihi"),
            value: generated_token_tree_value(gihi, source, options),
        });
    }
    Some(TreeValue::Node(TreeNode {
        constructor: "ForethoughtSelbriGroupTanruUnit",
        entries,
    }))
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_assigned_pro_bridi_tanru_unit_tree_value(
    base: &jbotci_syntax::ast::TanruUnitSyntax,
    assignments: &[jbotci_syntax::ast::ProBridiAssignmentSyntax],
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    let mut entries = vec![TreeEntry {
        label: Some("base"),
        value: legacy_as_generated_linked_tanru_unit_for_cei_tree_value(base, source, options),
    }];
    if let Some(entry) = labelled_tree_collection_entry_from_values(
        "assignments",
        assignments
            .iter()
            .map(|assignment| {
                TreeValue::Collection(vec![
                    required_legacy_syntax_subtree_value(&assignment.cei, source, options),
                    legacy_as_generated_linked_tanru_unit_for_cei_tree_value(
                        assignment.tanru_unit.as_ref(),
                        source,
                        options,
                    ),
                ])
            })
            .collect(),
    ) {
        entries.push(entry);
    }
    TreeValue::Node(TreeNode {
        constructor: "AssignedProBridiTanruUnit",
        entries,
    })
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_linked_tanru_unit_for_cei_tree_value(
    unit: &jbotci_syntax::ast::TanruUnitSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    match unit.as_data() {
        bityzba::data!(jbotci_syntax::ast::TanruUnitSyntax::LinkedSumtiTanruUnit {
            base,
            be,
            fa,
            first_sumti,
            bei_links,
            beho,
        }) => TreeValue::Node(TreeNode {
            constructor: "LinkedTanruUnitForCei",
            entries: vec![
                TreeEntry {
                    label: Some("base"),
                    value: legacy_as_generated_tanru_unit_atom_for_cei_tree_value(
                        base.as_ref(),
                        source,
                        options,
                    ),
                },
                TreeEntry {
                    label: Some("linkargs"),
                    value: legacy_as_generated_linked_sumti_list_tree_value(
                        be,
                        fa.as_ref(),
                        first_sumti.as_deref(),
                        bei_links,
                        beho.as_ref(),
                        source,
                        options,
                    ),
                },
            ],
        }),
        _ => TreeValue::Node(TreeNode {
            constructor: "LinkedTanruUnitForCei",
            entries: vec![TreeEntry {
                label: Some("base"),
                value: legacy_as_generated_tanru_unit_atom_for_cei_tree_value(
                    unit, source, options,
                ),
            }],
        }),
    }
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_tanru_unit_atom_for_cei_tree_value(
    unit: &jbotci_syntax::ast::TanruUnitSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    match unit.as_data() {
        bityzba::data!(jbotci_syntax::ast::TanruUnitSyntax::ConvertedTanruUnit { .. }) => {
            let (conversions, base) = legacy_flatten_tanru_unit_conversions(unit);
            let (conversions, free_modifiers) =
                legacy_conversion_tree_parts(conversions, source, options);
            let mut entries = vec![TreeEntry {
                label: Some("conversions"),
                value: conversions,
            }];
            if let Some(entry) =
                labelled_tree_collection_entry_from_values("free_modifiers", free_modifiers)
            {
                entries.push(entry);
            }
            entries.push(TreeEntry {
                label: Some("base"),
                value: legacy_as_generated_tanru_unit_atom_base_for_cei_tree_value(
                    base, source, options,
                ),
            });
            TreeValue::Node(TreeNode {
                constructor: "TanruUnitAtomForCei",
                entries,
            })
        }
        _ => TreeValue::Node(TreeNode {
            constructor: "TanruUnitAtomForCei",
            entries: vec![TreeEntry {
                label: Some("base"),
                value: legacy_as_generated_tanru_unit_atom_base_for_cei_tree_value(
                    unit, source, options,
                ),
            }],
        }),
    }
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_tanru_unit_atom_base_for_cei_tree_value(
    unit: &jbotci_syntax::ast::TanruUnitSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    if let Some(value) =
        legacy_as_generated_pro_bridi_if_goha_tanru_unit_tree_value(unit, source, options)
    {
        return value;
    }
    unit.tanru_unit_atom_base_tree_value(source, options)
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_scalar_negated_tanru_inner_unit_tree_value(
    unit: &jbotci_syntax::ast::TanruUnitSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    if let bityzba::data!(jbotci_syntax::ast::TanruUnitSyntax::SelbriGroupTanruUnit(
        selbri
    )) = unit.as_data()
        && let bityzba::data!(jbotci_syntax::ast::SelbriSyntax::TaggedSelbri {
            tense_modal,
            inner_selbri,
        }) = selbri.as_data()
    {
        return TreeValue::Node(TreeNode {
            constructor: "TaggedSelbriGroupTanruUnit",
            entries: vec![
                TreeEntry {
                    label: Some("tense_modal"),
                    value: legacy_as_generated_tense_modal_tree_value(
                        tense_modal.as_ref(),
                        source,
                        options,
                    ),
                },
                TreeEntry {
                    label: Some("inner_selbri"),
                    value: legacy_as_generated_connected_selbri_tree_value(
                        inner_selbri.as_ref(),
                        source,
                        options,
                    ),
                },
            ],
        });
    }
    if let Some(value) =
        legacy_as_generated_pro_bridi_if_goha_tanru_unit_tree_value(unit, source, options)
    {
        return value;
    }
    unit.tanru_unit_atom_tree_value(source, options)
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_jai_inner_tanru_unit_tree_value(
    unit: &jbotci_syntax::ast::TanruUnitSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    if let Some(value) =
        legacy_as_generated_pro_bridi_if_goha_tanru_unit_tree_value(unit, source, options)
    {
        return value;
    }
    match unit.as_data() {
        bityzba::data!(jbotci_syntax::ast::TanruUnitSyntax::ConvertedTanruUnit {
            se,
            inner_unit,
        }) => {
            let mut entries = legacy_token_field_entries("se", se, source, options);
            entries.push(TreeEntry {
                label: Some("inner_unit"),
                value: legacy_as_generated_jai_inner_tanru_unit_tree_value(
                    inner_unit.as_ref(),
                    source,
                    options,
                ),
            });
            TreeValue::Node(TreeNode {
                constructor: "ConvertedTanruUnit",
                entries,
            })
        }
        bityzba::data!(
            jbotci_syntax::ast::TanruUnitSyntax::ScalarNegatedTanruUnit { nahe, inner_unit }
        ) => {
            let mut entries = legacy_token_field_entries("nahe", nahe, source, options);
            entries.push(TreeEntry {
                label: Some("inner_unit"),
                value: legacy_as_generated_jai_inner_tanru_unit_tree_value(
                    inner_unit.as_ref(),
                    source,
                    options,
                ),
            });
            TreeValue::Node(TreeNode {
                constructor: "ScalarNegatedTanruUnit",
                entries,
            })
        }
        _ => unit.tanru_unit_atom_base_tree_value(source, options),
    }
}

#[contract_trait]
impl LegacyTanruUnitLike for jbotci_syntax::ast::TanruUnitSyntax {
    #[requires(true)]
    #[ensures(true)]
    fn bo_or_linked_tanru_unit_tree_value(
        &self,
        source: &str,
        options: TreeRenderOptions,
    ) -> TreeValue {
        match self.as_data() {
            bityzba::data!(jbotci_syntax::ast::TanruUnitSyntax::SelbriGroupTanruUnit(
                selbri
            )) if let bityzba::data!(
                jbotci_syntax::ast::SelbriSyntax::ForethoughtSelbriConnection {
                    guhek,
                    leading_bridi,
                    gik,
                    trailing_bridi,
                    gihi,
                }
            ) = selbri.as_data()
                && let Some(value) =
                    legacy_as_generated_forethought_selbri_group_tanru_unit_tree_value(
                        guhek,
                        leading_bridi.as_ref(),
                        gik,
                        trailing_bridi.as_ref(),
                        gihi.as_ref(),
                        source,
                        options,
                    ) =>
            {
                value
            }
            bityzba::data!(jbotci_syntax::ast::TanruUnitSyntax::AssignedProBridi {
                base,
                assignments,
            }) => legacy_as_generated_assigned_pro_bridi_tanru_unit_tree_value(
                base.as_ref(),
                assignments,
                source,
                options,
            ),
            bityzba::data!(
                jbotci_syntax::ast::TanruUnitSyntax::BoundTanruUnitConnection {
                    leading_unit,
                    bo_connective,
                    bo_tense_modal,
                    bo,
                    trailing_unit,
                }
            ) => legacy_as_generated_bound_tanru_unit_connection_tree_value(
                leading_unit.as_ref(),
                bo_connective.as_deref(),
                bo_tense_modal.as_deref(),
                bo,
                trailing_unit.as_ref(),
                source,
                options,
            ),
            _ => self.linked_tanru_unit_tree_value(source, options),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn linked_tanru_unit_tree_value(&self, source: &str, options: TreeRenderOptions) -> TreeValue {
        match self.as_data() {
            bityzba::data!(jbotci_syntax::ast::TanruUnitSyntax::LinkedSumtiTanruUnit {
                base,
                be,
                fa,
                first_sumti,
                bei_links,
                beho,
            }) => TreeValue::Node(TreeNode {
                constructor: "LinkedTanruUnit",
                entries: vec![
                    TreeEntry {
                        label: Some("base"),
                        value: legacy_as_generated_tanru_unit_atom_tree_value(
                            base.as_ref(),
                            source,
                            options,
                        ),
                    },
                    TreeEntry {
                        label: Some("linkargs"),
                        value: legacy_as_generated_linked_sumti_list_tree_value(
                            be,
                            fa.as_ref(),
                            first_sumti.as_deref(),
                            bei_links,
                            beho.as_ref(),
                            source,
                            options,
                        ),
                    },
                ],
            }),
            _ => TreeValue::Node(TreeNode {
                constructor: "LinkedTanruUnit",
                entries: vec![TreeEntry {
                    label: Some("base"),
                    value: legacy_as_generated_tanru_unit_atom_tree_value(self, source, options),
                }],
            }),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn tanru_unit_connection_parts<'tree>(
        &'tree self,
    ) -> Option<(
        &'tree jbotci_syntax::ast::TanruUnitSyntax,
        Vec<(
            &'tree jbotci_syntax::ast::ConnectiveSyntax,
            &'tree jbotci_syntax::ast::TanruUnitSyntax,
        )>,
    )> {
        match self.as_data() {
            bityzba::data!(jbotci_syntax::ast::TanruUnitSyntax::TanruUnitConnection {
                leading_unit,
                connective,
                trailing_unit,
            }) => {
                let (leading, mut continuations) = leading_unit
                    .tanru_unit_connection_parts()
                    .unwrap_or((leading_unit.as_ref(), Vec::new()));
                if let Some((trailing_leading, mut trailing_continuations)) =
                    trailing_unit.tanru_unit_connection_parts()
                {
                    continuations.push((connective, trailing_leading));
                    continuations.append(&mut trailing_continuations);
                } else {
                    continuations.push((connective, trailing_unit.as_ref()));
                }
                Some((leading, continuations))
            }
            _ => None,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn tanru_unit_atom_tree_value(&self, source: &str, options: TreeRenderOptions) -> TreeValue {
        match self.as_data() {
            bityzba::data!(jbotci_syntax::ast::TanruUnitSyntax::ConvertedTanruUnit { .. }) => {
                let (conversions, base) = legacy_flatten_tanru_unit_conversions(self);
                let (conversions, free_modifiers) =
                    legacy_conversion_tree_parts(conversions, source, options);
                let mut entries = vec![TreeEntry {
                    label: Some("conversions"),
                    value: conversions,
                }];
                if let Some(entry) =
                    labelled_tree_collection_entry_from_values("free_modifiers", free_modifiers)
                {
                    entries.push(entry);
                }
                entries.push(TreeEntry {
                    label: Some("base"),
                    value: base.tanru_unit_atom_base_tree_value(source, options),
                });
                TreeValue::Node(TreeNode {
                    constructor: "TanruUnitAtom",
                    entries,
                })
            }
            _ => TreeValue::Node(TreeNode {
                constructor: "TanruUnitAtom",
                entries: vec![TreeEntry {
                    label: Some("base"),
                    value: self.tanru_unit_atom_base_tree_value(source, options),
                }],
            }),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn tanru_unit_atom_base_tree_value(
        &self,
        source: &str,
        options: TreeRenderOptions,
    ) -> TreeValue {
        match self.as_data() {
            bityzba::data!(jbotci_syntax::ast::TanruUnitSyntax::ConvertedTanruUnit {
                se,
                inner_unit,
            }) => {
                let mut entries = legacy_token_field_entries("se", se, source, options);
                entries.push(TreeEntry {
                    label: Some("inner_unit"),
                    value: inner_unit.tanru_unit_atom_base_tree_value(source, options),
                });
                TreeValue::Node(TreeNode {
                    constructor: "ConvertedTanruUnit",
                    entries,
                })
            }
            bityzba::data!(jbotci_syntax::ast::TanruUnitSyntax::TanruUnitWord(word)) => {
                if legacy_goha_token_renders_as_pro_bridi(&word.value, &word.free_modifiers) {
                    return legacy_as_generated_pro_bridi_tree_value(word, None, source, options);
                }
                legacy_as_generated_tanru_unit_word_tree_value(word, source, options)
            }
            bityzba::data!(jbotci_syntax::ast::TanruUnitSyntax::ProBridi { goha, raho }) => {
                if raho.is_none()
                    && !legacy_goha_token_renders_as_pro_bridi(&goha.value, &goha.free_modifiers)
                {
                    legacy_as_generated_tanru_unit_word_tree_value(goha, source, options)
                } else {
                    legacy_as_generated_pro_bridi_tree_value(goha, raho.as_ref(), source, options)
                }
            }
            bityzba::data!(jbotci_syntax::ast::TanruUnitSyntax::Abstraction(
                abstraction
            )) => legacy_as_generated_abstraction_tanru_unit_tree_value(
                abstraction.as_ref(),
                source,
                options,
            ),
            bityzba::data!(jbotci_syntax::ast::TanruUnitSyntax::SumtiSelbri {
                me,
                sumti,
                mehu,
                moi_marker,
            }) => {
                let mut entries = legacy_token_field_entries("me", me, source, options);
                entries.push(TreeEntry {
                    label: Some("sumti"),
                    value: legacy_as_generated_sumti_selbri_sumti_tree_value(
                        sumti.as_ref(),
                        moi_marker.is_some(),
                        source,
                        options,
                    ),
                });
                if let Some(mehu) = mehu {
                    entries.extend(legacy_token_field_entries("mehu", mehu, source, options));
                }
                if let Some(moi_marker) = moi_marker {
                    entries.extend(legacy_token_field_entries(
                        "moi_marker",
                        moi_marker,
                        source,
                        options,
                    ));
                }
                TreeValue::Node(TreeNode {
                    constructor: "SumtiSelbri",
                    entries,
                })
            }
            bityzba::data!(jbotci_syntax::ast::TanruUnitSyntax::ModalConversion {
                jai,
                tense_modal,
                inner_unit,
            }) => {
                let mut entries = legacy_token_field_entries("jai", jai, source, options);
                if let Some(tense_modal) = tense_modal {
                    entries.push(TreeEntry {
                        label: Some("tense_modal"),
                        value: legacy_as_generated_tense_modal_tree_value(
                            tense_modal.as_ref(),
                            source,
                            options,
                        ),
                    });
                }
                entries.push(TreeEntry {
                    label: Some("inner_unit"),
                    value: legacy_as_generated_jai_inner_tanru_unit_tree_value(
                        inner_unit.as_ref(),
                        source,
                        options,
                    ),
                });
                TreeValue::Node(TreeNode {
                    constructor: "ModalConversion",
                    entries,
                })
            }
            bityzba::data!(jbotci_syntax::ast::TanruUnitSyntax::GroupedTanruUnit {
                ke_tense_modal,
                ke,
                selbri,
                kehe,
            }) => legacy_as_generated_grouped_tanru_unit_tree_value(
                ke_tense_modal.as_deref(),
                ke,
                selbri.as_ref(),
                kehe.as_ref(),
                source,
                options,
            ),
            bityzba::data!(
                jbotci_syntax::ast::TanruUnitSyntax::ScalarNegatedTanruUnit { nahe, inner_unit }
            ) => {
                let mut entries = legacy_token_field_entries("nahe", nahe, source, options);
                entries.push(TreeEntry {
                    label: Some("inner_unit"),
                    value: legacy_as_generated_scalar_negated_tanru_inner_unit_tree_value(
                        inner_unit.as_ref(),
                        source,
                        options,
                    ),
                });
                TreeValue::Node(TreeNode {
                    constructor: "ScalarNegatedTanruUnit",
                    entries,
                })
            }
            bityzba::data!(
                jbotci_syntax::ast::TanruUnitSyntax::PreposedLinkedSumtiTanruUnit {
                    be,
                    fa,
                    first_sumti,
                    bei_links,
                    beho,
                    base,
                }
            ) => TreeValue::Node(TreeNode {
                constructor: "PreposedLinkargsTanruUnit",
                entries: vec![
                    TreeEntry {
                        label: Some("linkargs"),
                        value: legacy_as_generated_linked_sumti_list_tree_value(
                            be,
                            fa.as_ref(),
                            first_sumti.as_deref(),
                            bei_links,
                            beho.as_ref(),
                            source,
                            options,
                        ),
                    },
                    TreeEntry {
                        label: Some("base"),
                        value: legacy_as_generated_connected_tanru_unit_tree_value(
                            base.as_ref(),
                            source,
                            options,
                        ),
                    },
                ],
            }),
            bityzba::data!(jbotci_syntax::ast::TanruUnitSyntax::TagSelbri { xohi, tag }) => {
                let mut entries = legacy_token_field_entries("xohi", xohi, source, options);
                entries.push(TreeEntry {
                    label: Some("tag"),
                    value: legacy_as_generated_tense_modal_tree_value(
                        tag.as_ref(),
                        source,
                        options,
                    ),
                });
                TreeValue::Node(TreeNode {
                    constructor: "TagSelbri",
                    entries,
                })
            }
            bityzba::data!(jbotci_syntax::ast::TanruUnitSyntax::OrdinalSelbri { number, moi }) => {
                let mut entries = vec![TreeEntry {
                    label: Some("number"),
                    value: legacy_word_run_tree_value(number, source, options),
                }];
                entries.extend(legacy_token_field_entries("moi", moi, source, options));
                TreeValue::Node(TreeNode {
                    constructor: "OrdinalTanruUnit",
                    entries,
                })
            }
            bityzba::data!(jbotci_syntax::ast::TanruUnitSyntax::OperatorSelbri {
                nuha,
                mekso_operator,
            }) => {
                let mut entries = legacy_token_field_entries("nuha", nuha, source, options);
                entries.push(TreeEntry {
                    label: Some("mekso_operator"),
                    value: legacy_as_generated_mekso_operator_tree_value(
                        mekso_operator.as_ref(),
                        source,
                        options,
                    ),
                });
                TreeValue::Node(TreeNode {
                    constructor: "OperatorSelbri",
                    entries,
                })
            }
            bityzba::data!(jbotci_syntax::ast::TanruUnitSyntax::TextSelbri {
                luhei,
                text,
                liau,
            }) => {
                let mut entries = legacy_token_field_entries("luhei", luhei, source, options);
                entries.push(TreeEntry {
                    label: Some("text"),
                    value: legacy_as_generated_text_tree_value(text.as_ref(), source, options),
                });
                if let Some(liau) = liau {
                    entries.extend(legacy_token_field_entries("liau", liau, source, options));
                }
                TreeValue::Node(TreeNode {
                    constructor: "TextSelbri",
                    entries,
                })
            }
            _ => required_legacy_syntax_subtree_value(self, source, options),
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_sumti_selbri_sumti_tree_value(
    sumti: &jbotci_syntax::ast::SumtiSyntax,
    has_moi_marker: bool,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    if has_moi_marker
        && let bityzba::data!(jbotci_syntax::ast::SumtiSyntax::LerfuStringSumti {
            letter,
            boi: None,
        }) = sumti.as_data()
        && letter.free_modifiers.is_empty()
    {
        return TreeValue::Node(TreeNode {
            constructor: "MeLerfuSumti",
            entries: vec![TreeEntry {
                label: Some("words"),
                value: legacy_word_run_tree_value(&letter.value, source, options),
            }],
        });
    }
    legacy_as_generated_sumti_tree_value(sumti, source, options)
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_tanru_unit_word_tree_value(
    word: &WithFreeModifiers<Token>,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    let mut entries = vec![TreeEntry {
        label: None,
        value: generated_token_tree_value(&word.value, source, options),
    }];
    if let Some(entry) = labelled_tree_collection_entry_from_values(
        "free_modifiers",
        word.free_modifiers
            .iter()
            .map(|free_modifier| {
                legacy_as_generated_free_modifier_tree_value(free_modifier, source, options)
            })
            .collect(),
    ) {
        entries.push(entry);
    }
    TreeValue::Node(TreeNode {
        constructor: "TanruUnitWord",
        entries,
    })
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_pro_bridi_tree_value(
    goha: &WithFreeModifiers<Token>,
    raho: Option<&WithFreeModifiers<Token>>,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    let mut entries = legacy_token_field_entries("goha", goha, source, options);
    if let Some(raho) = raho {
        entries.extend(legacy_token_field_entries("raho", raho, source, options));
    }
    TreeValue::Node(TreeNode {
        constructor: "ProBridi",
        entries,
    })
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_pro_bridi_if_goha_tanru_unit_tree_value(
    unit: &jbotci_syntax::ast::TanruUnitSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> Option<TreeValue> {
    match unit.as_data() {
        bityzba::data!(jbotci_syntax::ast::TanruUnitSyntax::TanruUnitWord(word))
            if word.value.is_selmaho(Selmaho::Goha) =>
        {
            Some(legacy_as_generated_pro_bridi_tree_value(
                word, None, source, options,
            ))
        }
        bityzba::data!(jbotci_syntax::ast::TanruUnitSyntax::ProBridi { goha, raho }) => Some(
            legacy_as_generated_pro_bridi_tree_value(goha, raho.as_ref(), source, options),
        ),
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn legacy_goha_token_renders_as_pro_bridi(
    goha: &Token,
    free_modifiers: &[jbotci_syntax::ast::FreeModifierSyntax],
) -> bool {
    if !goha.is_selmaho(Selmaho::Goha) {
        return false;
    }
    if !free_modifiers.is_empty() {
        return true;
    }
    let next = legacy_next_tree_token_after_with_free_modifiers(goha, free_modifiers);
    next.as_ref()
        .is_some_and(legacy_token_blocks_goha_word_tanru_unit)
}

#[requires(true)]
#[ensures(true)]
fn legacy_token_blocks_goha_word_tanru_unit(token: &Token) -> bool {
    token.is_cmavo(Cmavo::Raho)
        || token.is_cmavo(Cmavo::Be)
        || token.is_selmaho(Selmaho::Pa)
        || legacy_token_can_start_free_modifier(token)
}

#[requires(true)]
#[ensures(true)]
fn legacy_token_can_start_free_modifier(token: &Token) -> bool {
    token.is_one_of_selmaho(&[Selmaho::Sei, Selmaho::Soi, Selmaho::Coi, Selmaho::Doi])
        || token.is_one_of_cmavo(&[Cmavo::To, Cmavo::Xi])
        || legacy_number_or_letter_run_is_followed_by_mai(token)
}

#[requires(true)]
#[ensures(true)]
fn legacy_number_or_letter_run_is_followed_by_mai(first: &Token) -> bool {
    if !legacy_token_is_number_or_letter_word(first) {
        return false;
    }
    let mut current = first.clone();
    loop {
        let Some(next) = legacy_next_tree_token_after(&current) else {
            return false;
        };
        if next.is_selmaho(Selmaho::Mai) {
            return true;
        }
        if !legacy_token_is_number_or_letter_word(&next) {
            return false;
        }
        current = next;
    }
}

#[requires(true)]
#[ensures(true)]
fn legacy_token_is_number_or_letter_word(token: &Token) -> bool {
    token.is_selmaho(Selmaho::Pa) || legacy_token_is_letter_word(token)
}

#[requires(true)]
#[ensures(true)]
fn legacy_token_is_letter_word(token: &Token) -> bool {
    match token.core_word().as_data() {
        bityzba::data!(WordLike::LerfuWord { .. }) => true,
        bityzba::data!(WordLike::PlainWord(word)) => {
            word.kind() == WordKind::Cmavo
                && word.cmavo().is_some_and(|cmavo| {
                    (!matches!(cmavo, Cmavo::A | Cmavo::E | Cmavo::I | Cmavo::O | Cmavo::U)
                        && cmavo.is_selmaho(Selmaho::By))
                        || cmavo == Cmavo::Sehe
                        || cmavo == Cmavo::Y
                })
        }
        _ => false,
    }
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_connected_tanru_unit_tree_value<T>(
    unit: &T,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue
where
    T: LegacyTanruUnitLike + ?Sized,
{
    if let Some((leading_unit, continuations)) = unit.tanru_unit_connection_parts() {
        let mut entries = vec![TreeEntry {
            label: Some("leading_unit"),
            value: leading_unit.bo_or_linked_tanru_unit_tree_value(source, options),
        }];
        if let Some(entry) = labelled_tree_collection_entry_from_values(
            "continuations",
            continuations
                .iter()
                .map(|(connective, trailing_unit)| {
                    TreeValue::Collection(vec![
                        required_legacy_syntax_subtree_value(*connective, source, options),
                        trailing_unit.bo_or_linked_tanru_unit_tree_value(source, options),
                    ])
                })
                .collect(),
        ) {
            entries.push(entry);
        }
        return TreeValue::Node(TreeNode {
            constructor: "ConnectedTanruUnit",
            entries,
        });
    }
    TreeValue::Node(TreeNode {
        constructor: "ConnectedTanruUnit",
        entries: vec![TreeEntry {
            label: Some("leading_unit"),
            value: unit.bo_or_linked_tanru_unit_tree_value(source, options),
        }],
    })
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_linked_tanru_unit_tree_value<T>(
    unit: &T,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue
where
    T: LegacyTanruUnitLike + ?Sized,
{
    unit.linked_tanru_unit_tree_value(source, options)
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_tanru_unit_atom_tree_value<T>(
    unit: &T,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue
where
    T: LegacyTanruUnitLike + ?Sized,
{
    unit.tanru_unit_atom_tree_value(source, options)
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_linked_sumti_list_tree_value(
    be: &WithFreeModifiers<Token>,
    fa: Option<&WithFreeModifiers<Token>>,
    first_sumti: Option<&jbotci_syntax::ast::SumtiSyntax>,
    bei_links: &[jbotci_syntax::ast::AdditionalLinkedSumtiSyntax],
    beho: Option<&WithFreeModifiers<Token>>,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    let mut entries = legacy_token_field_entries("be", be, source, options);
    entries.push(TreeEntry {
        label: Some("first_link"),
        value: legacy_as_generated_linked_sumti_tree_value(fa, first_sumti, source, options),
    });
    if let Some(entry) = labelled_tree_collection_entry_from_values(
        "bei_links",
        bei_links
            .iter()
            .map(|link| {
                legacy_as_generated_additional_linked_sumti_tree_value(link, source, options)
            })
            .collect(),
    ) {
        entries.push(entry);
    }
    if let Some(beho) = beho {
        entries.extend(legacy_token_field_entries("beho", beho, source, options));
    }
    TreeValue::Node(TreeNode {
        constructor: "Linkargs",
        entries,
    })
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_additional_linked_sumti_tree_value(
    link: &jbotci_syntax::ast::AdditionalLinkedSumtiSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    let mut entries = legacy_token_field_entries("bei", &link.bei, source, options);
    entries.push(TreeEntry {
        label: Some("link"),
        value: legacy_as_generated_linked_sumti_tree_value(
            link.fa.as_ref(),
            link.sumti.as_deref(),
            source,
            options,
        ),
    });
    TreeValue::Node(TreeNode {
        constructor: "BeiLink",
        entries,
    })
}

#[requires(!constructor.is_empty() && !label.is_empty())]
#[ensures(true)]
fn legacy_as_generated_linked_sumti_variant_tree_value(
    constructor: &'static str,
    label: &'static str,
    entries: Vec<TreeEntry>,
) -> TreeValue {
    let inner = TreeValue::Node(TreeNode {
        constructor,
        entries,
    });
    TreeValue::Node(TreeNode {
        constructor,
        entries: vec![TreeEntry {
            label: Some(label),
            value: inner,
        }],
    })
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_linked_sumti_tree_value(
    fa: Option<&WithFreeModifiers<Token>>,
    sumti: Option<&jbotci_syntax::ast::SumtiSyntax>,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    match (fa, sumti) {
        (Some(fa), Some(sumti)) => legacy_as_generated_linked_sumti_variant_tree_value(
            "PlaceTaggedLinkedSumti",
            "place_tagged_linked_sumti",
            {
                let mut entries = legacy_token_field_entries("fa", fa, source, options);
                entries.push(TreeEntry {
                    label: Some("sumti"),
                    value: legacy_as_generated_tagged_or_elided_sumti_tree_value(
                        sumti, source, options,
                    ),
                });
                entries
            },
        ),
        (Some(fa), None) => {
            let mut entries = legacy_token_field_entries("fa", fa, source, options);
            entries.push(TreeEntry {
                label: Some("sumti"),
                value: TreeValue::Node(TreeNode {
                    constructor: "ElidedSumti",
                    entries: Vec::new(),
                }),
            });
            legacy_as_generated_linked_sumti_variant_tree_value(
                "PlaceTaggedLinkedSumti",
                "place_tagged_linked_sumti",
                entries,
            )
        }
        (None, Some(sumti)) => {
            if let Some((fa, maybe_ku, free_modifiers)) =
                legacy_elided_place_tagged_linked_sumti_parts(sumti)
            {
                let mut entries = legacy_token_field_entries("fa", fa, source, options);
                entries.push(TreeEntry {
                    label: Some("sumti"),
                    value: legacy_as_generated_elided_sumti_without_tag_tree_value(
                        maybe_ku,
                        free_modifiers,
                        source,
                        options,
                    ),
                });
                return legacy_as_generated_linked_sumti_variant_tree_value(
                    "PlaceTaggedLinkedSumti",
                    "place_tagged_linked_sumti",
                    entries,
                );
            }
            if let Some((tense_modal, maybe_ku, free_modifiers)) =
                legacy_elided_tense_tagged_sumti_parts(sumti)
            {
                return legacy_as_generated_linked_sumti_variant_tree_value(
                    "TenseTaggedLinkedSumti",
                    "tense_tagged_linked_sumti",
                    vec![
                        TreeEntry {
                            label: Some("tense_modal"),
                            value: legacy_as_generated_tense_modal_tree_value(
                                tense_modal,
                                source,
                                options,
                            ),
                        },
                        TreeEntry {
                            label: Some("sumti"),
                            value: legacy_as_generated_elided_sumti_without_tag_tree_value(
                                maybe_ku,
                                free_modifiers,
                                source,
                                options,
                            ),
                        },
                    ],
                );
            }
            if let Some((fa, inner_sumti)) = legacy_place_tagged_linked_sumti_parts(sumti) {
                let mut entries = legacy_token_field_entries("fa", fa, source, options);
                entries.push(TreeEntry {
                    label: Some("sumti"),
                    value: legacy_as_generated_tagged_or_elided_sumti_tree_value(
                        inner_sumti,
                        source,
                        options,
                    ),
                });
                return legacy_as_generated_linked_sumti_variant_tree_value(
                    "PlaceTaggedLinkedSumti",
                    "place_tagged_linked_sumti",
                    entries,
                );
            }
            if let Some((tense_modal, inner_sumti)) = legacy_tense_tagged_linked_sumti_parts(sumti)
            {
                return legacy_as_generated_linked_sumti_variant_tree_value(
                    "TenseTaggedLinkedSumti",
                    "tense_tagged_linked_sumti",
                    vec![
                        TreeEntry {
                            label: Some("tense_modal"),
                            value: legacy_as_generated_tense_modal_tree_value(
                                tense_modal,
                                source,
                                options,
                            ),
                        },
                        TreeEntry {
                            label: Some("sumti"),
                            value: legacy_as_generated_tagged_or_elided_sumti_tree_value(
                                inner_sumti,
                                source,
                                options,
                            ),
                        },
                    ],
                );
            }
            legacy_as_generated_linked_sumti_variant_tree_value(
                "PlainLinkedSumti",
                "plain_linked_sumti",
                vec![TreeEntry {
                    label: Some("sumti"),
                    value: legacy_as_generated_tagged_or_elided_sumti_tree_value(
                        sumti, source, options,
                    ),
                }],
            )
        }
        _ => legacy_as_generated_linked_sumti_variant_tree_value(
            "EmptyLinkedSumti",
            "empty_linked_sumti",
            Vec::new(),
        ),
    }
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_elided_sumti_without_tag_tree_value(
    maybe_ku: Option<&WithFreeModifiers<Token>>,
    free_modifiers: &[jbotci_syntax::ast::FreeModifierSyntax],
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    let mut entries = Vec::new();
    if let Some(maybe_ku) = maybe_ku {
        entries.extend(legacy_token_field_entries(
            "maybe_ku", maybe_ku, source, options,
        ));
    }
    if let Some(entry) = labelled_tree_collection_entry_from_values(
        "free_modifiers",
        free_modifiers
            .iter()
            .map(|free_modifier| {
                legacy_as_generated_free_modifier_tree_value(free_modifier, source, options)
            })
            .collect(),
    ) {
        entries.push(entry);
    }
    TreeValue::Node(TreeNode {
        constructor: "ElidedSumti",
        entries,
    })
}

#[requires(true)]
#[ensures(true)]
fn legacy_elided_place_tagged_linked_sumti_parts(
    sumti: &jbotci_syntax::ast::SumtiSyntax,
) -> Option<(
    &WithFreeModifiers<Token>,
    Option<&WithFreeModifiers<Token>>,
    &[jbotci_syntax::ast::FreeModifierSyntax],
)> {
    match sumti.as_data() {
        bityzba::data!(jbotci_syntax::ast::SumtiSyntax::ElidedSumti {
            tag: Some(tag),
            maybe_ku,
            free_modifiers,
        }) => match tag.as_data() {
            bityzba::data!(jbotci_syntax::ast::SumtiTagSyntax::PlaceTag(fa)) => {
                Some((fa, maybe_ku.as_ref(), free_modifiers.as_slice()))
            }
            _ => None,
        },
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn legacy_elided_tense_tagged_sumti_parts(
    sumti: &jbotci_syntax::ast::SumtiSyntax,
) -> Option<(
    &jbotci_syntax::ast::TenseModalSyntax,
    Option<&WithFreeModifiers<Token>>,
    &[jbotci_syntax::ast::FreeModifierSyntax],
)> {
    match sumti.as_data() {
        bityzba::data!(jbotci_syntax::ast::SumtiSyntax::ElidedSumti {
            tag: Some(tag),
            maybe_ku,
            free_modifiers,
        }) => match tag.as_data() {
            bityzba::data!(jbotci_syntax::ast::SumtiTagSyntax::TenseModal(tense_modal)) => {
                Some((tense_modal.as_ref(), maybe_ku.as_ref(), free_modifiers))
            }
            _ => None,
        },
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn legacy_place_tagged_linked_sumti_parts(
    sumti: &jbotci_syntax::ast::SumtiSyntax,
) -> Option<(&WithFreeModifiers<Token>, &jbotci_syntax::ast::SumtiSyntax)> {
    match sumti.as_data() {
        bityzba::data!(jbotci_syntax::ast::SumtiSyntax::TaggedSumti { tag, inner_sumti }) => {
            match tag.as_data() {
                bityzba::data!(jbotci_syntax::ast::SumtiTagSyntax::PlaceTag(fa)) => {
                    Some((fa, inner_sumti.as_ref()))
                }
                _ => None,
            }
        }
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn legacy_tense_tagged_linked_sumti_parts(
    sumti: &jbotci_syntax::ast::SumtiSyntax,
) -> Option<(
    &jbotci_syntax::ast::TenseModalSyntax,
    &jbotci_syntax::ast::SumtiSyntax,
)> {
    match sumti.as_data() {
        bityzba::data!(jbotci_syntax::ast::SumtiSyntax::TaggedSumti { tag, inner_sumti }) => {
            match tag.as_data() {
                bityzba::data!(jbotci_syntax::ast::SumtiTagSyntax::TenseModal(tense_modal)) => {
                    Some((tense_modal.as_ref(), inner_sumti.as_ref()))
                }
                _ => None,
            }
        }
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_abstraction_tanru_unit_tree_value(
    abstraction: &jbotci_syntax::ast::AbstractionSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    let mut entries = legacy_token_field_entries("nu", &abstraction.nu, source, options);
    if let Some(nai) = &abstraction.nai {
        entries.extend(legacy_token_field_entries("nai", nai, source, options));
    }
    if let Some(entry) = labelled_tree_collection_entry_from_values(
        "abstractor_connections",
        abstraction
            .abstractor_connections
            .iter()
            .map(|connection| required_legacy_syntax_subtree_value(connection, source, options))
            .collect(),
    ) {
        entries.push(entry);
    }
    entries.push(TreeEntry {
        label: Some("subbridi"),
        value: legacy_as_generated_subbridi_tree_value(
            abstraction.subbridi.as_ref(),
            source,
            options,
        ),
    });
    if let Some(kei) = &abstraction.kei {
        entries.extend(legacy_token_field_entries("kei", kei, source, options));
    }
    TreeValue::Node(TreeNode {
        constructor: "AbstractionTanruUnit",
        entries,
    })
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_tense_modal_tree_value(
    tense_modal: &jbotci_syntax::ast::TenseModalSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    match tense_modal.as_data() {
        bityzba::data!(jbotci_syntax::ast::TenseModalSyntax::Composite { parts }) => {
            legacy_as_generated_composite_tense_modal_tree_value(parts, source, options)
        }
        bityzba::data!(jbotci_syntax::ast::TenseModalSyntax::AdHocModal { fiho, selbri, fehu }) => {
            let mut entries = legacy_token_field_entries("fiho", fiho, source, options);
            entries.push(TreeEntry {
                label: Some("selbri"),
                value: legacy_as_generated_selbri_tree_value(selbri.as_ref(), source, options),
            });
            if let Some(fehu) = fehu {
                entries.extend(legacy_token_field_entries("fehu", fehu, source, options));
            }
            TreeValue::Node(TreeNode {
                constructor: "FihoTense",
                entries,
            })
        }
        bityzba::data!(jbotci_syntax::ast::TenseModalSyntax::Modal {
            nahe,
            se,
            bai,
            nai,
            ki,
        }) => {
            let mut entries = Vec::new();
            if let Some(nahe) = nahe {
                entries.extend(legacy_token_field_entries("nahe", nahe, source, options));
            }
            if let Some(se) = se {
                entries.extend(legacy_token_field_entries("se", se, source, options));
            }
            entries.extend(legacy_token_field_entries("bai", bai, source, options));
            if let Some(nai) = nai {
                entries.extend(legacy_token_field_entries("nai", nai, source, options));
            }
            if let Some(ki) = ki {
                entries.extend(legacy_token_field_entries("ki", ki, source, options));
            }
            TreeValue::Node(TreeNode {
                constructor: "ModalTense",
                entries,
            })
        }
        bityzba::data!(jbotci_syntax::ast::TenseModalSyntax::Sticky(ki)) => {
            TreeValue::Node(TreeNode {
                constructor: "StickyTense",
                entries: legacy_token_field_entries("ki", ki, source, options),
            })
        }
        _ => required_legacy_syntax_subtree_value(tense_modal, source, options),
    }
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_leading_term_tag_tense_modal_tree_value(
    tense_modal: &jbotci_syntax::ast::TenseModalSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> Option<TreeValue> {
    match tense_modal.as_data() {
        bityzba::data!(jbotci_syntax::ast::TenseModalSyntax::Composite { parts }) => {
            legacy_as_generated_leading_term_tag_composite_tense_value(parts, source, options)
        }
        bityzba::data!(jbotci_syntax::ast::TenseModalSyntax::TimeDirection(_))
        | bityzba::data!(jbotci_syntax::ast::TenseModalSyntax::TimeDirectionDistance { .. })
        | bityzba::data!(jbotci_syntax::ast::TenseModalSyntax::TimeInterval(_))
        | bityzba::data!(jbotci_syntax::ast::TenseModalSyntax::TimeDirectionActuality { .. })
        | bityzba::data!(jbotci_syntax::ast::TenseModalSyntax::SpaceDistance(_))
        | bityzba::data!(jbotci_syntax::ast::TenseModalSyntax::SpaceDirection(_))
        | bityzba::data!(jbotci_syntax::ast::TenseModalSyntax::SpaceMovement { .. })
        | bityzba::data!(jbotci_syntax::ast::TenseModalSyntax::Modal { .. })
        | bityzba::data!(jbotci_syntax::ast::TenseModalSyntax::Sticky(_))
        | bityzba::data!(jbotci_syntax::ast::TenseModalSyntax::AdHocModal { .. }) => {
            if legacy_tense_modal_has_following_tense_modal(tense_modal) {
                Some(legacy_as_generated_tense_modal_tree_value(
                    tense_modal,
                    source,
                    options,
                ))
            } else {
                None
            }
        }
        bityzba::data!(jbotci_syntax::ast::TenseModalSyntax::Actuality(caha)) => {
            let next =
                legacy_next_tree_token_after_with_free_modifiers(&caha.value, &caha.free_modifiers);
            if next
                .as_ref()
                .is_some_and(legacy_token_can_start_tense_modal)
            {
                Some(legacy_as_generated_tense_modal_tree_value(
                    tense_modal,
                    source,
                    options,
                ))
            } else {
                None
            }
        }
        bityzba::data!(jbotci_syntax::ast::TenseModalSyntax::EventContour(words)) => {
            legacy_as_generated_leading_term_tag_event_contour_tense_value(words, source, options)
        }
        bityzba::data!(jbotci_syntax::ast::TenseModalSyntax::IntervalProperty {
            number,
            roi_or_tahe,
            nai,
        }) => legacy_as_generated_leading_term_tag_interval_property_tense_value(
            number.as_ref(),
            roi_or_tahe,
            nai.as_ref(),
            source,
            options,
        ),
    }
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_leading_term_tag_composite_tense_value(
    parts: &WithFreeModifiers<Vec<jbotci_syntax::ast::CompositeTenseModalPartSyntax>>,
    source: &str,
    options: TreeRenderOptions,
) -> Option<TreeValue> {
    let tokens = legacy_composite_tense_modal_part_tokens(&parts.value)?;
    let free_modifiers = parts.free_modifiers.as_slice();
    let next = legacy_next_tree_token_after_sequence(&tokens, free_modifiers);
    if let Some((pu, nai)) = leading_term_tag_pu_before_nahe_parts(&tokens) {
        if next
            .as_ref()
            .is_some_and(|token| token.is_selmaho(Selmaho::Nahe))
        {
            let mut entries = vec![leading_term_tag_token_entry(
                "pu",
                pu,
                nai.is_none(),
                free_modifiers,
                source,
                options,
            )];
            if let Some(nai) = nai {
                entries.push(leading_term_tag_token_entry(
                    "nai",
                    nai,
                    true,
                    free_modifiers,
                    source,
                    options,
                ));
            }
            return Some(TreeValue::Node(TreeNode {
                constructor: "PuBeforeNaheLeadingTermTagTense",
                entries,
            }));
        }
    }

    if let Some((pu, nai, distance)) = leading_term_tag_pu_distance_parts(&tokens) {
        if next
            .as_ref()
            .is_some_and(|token| token.is_selmaho(Selmaho::Zi))
        {
            let mut entries = vec![leading_term_tag_token_entry(
                "pu",
                pu,
                false,
                free_modifiers,
                source,
                options,
            )];
            if let Some(nai) = nai {
                entries.push(leading_term_tag_token_entry(
                    "nai",
                    nai,
                    false,
                    free_modifiers,
                    source,
                    options,
                ));
            }
            entries.push(leading_term_tag_token_entry(
                "distance",
                distance,
                true,
                free_modifiers,
                source,
                options,
            ));
            return Some(TreeValue::Node(TreeNode {
                constructor: "PuDistanceBeforeTagLeadingTermTagTense",
                entries,
            }));
        }
    }

    if let [zi] = tokens.as_slice()
        && zi.is_selmaho(Selmaho::Zi)
    {
        if next
            .as_ref()
            .is_some_and(|token| token.is_selmaho(Selmaho::Zi))
        {
            return Some(TreeValue::Node(TreeNode {
                constructor: "ZiBeforeZiLeadingTermTagTense",
                entries: vec![leading_term_tag_token_entry(
                    "zi",
                    zi,
                    true,
                    free_modifiers,
                    source,
                    options,
                )],
            }));
        }
    }

    if let [va] = tokens.as_slice()
        && va.is_selmaho(Selmaho::Va)
    {
        if next
            .as_ref()
            .is_some_and(|token| token.is_selmaho(Selmaho::Va))
        {
            return Some(TreeValue::Node(TreeNode {
                constructor: "VaBeforeVaLeadingTermTagTense",
                entries: vec![leading_term_tag_token_entry(
                    "va",
                    va,
                    true,
                    free_modifiers,
                    source,
                    options,
                )],
            }));
        }
    }

    if let Some((mohi, direction, nai, distance)) = leading_term_tag_mohi_parts(&tokens) {
        if next
            .as_ref()
            .is_some_and(|token| token.is_selmaho(Selmaho::Mohi))
        {
            let mut entries = vec![
                leading_term_tag_token_entry("mohi", mohi, false, free_modifiers, source, options),
                leading_term_tag_token_entry(
                    "direction",
                    direction,
                    nai.is_none() && distance.is_none(),
                    free_modifiers,
                    source,
                    options,
                ),
            ];
            if let Some(nai) = nai {
                entries.push(leading_term_tag_token_entry(
                    "nai",
                    nai,
                    distance.is_none(),
                    free_modifiers,
                    source,
                    options,
                ));
            }
            if let Some(distance) = distance {
                entries.push(leading_term_tag_token_entry(
                    "distance",
                    distance,
                    true,
                    free_modifiers,
                    source,
                    options,
                ));
            }
            return Some(TreeValue::Node(TreeNode {
                constructor: "MohiBeforeMohiLeadingTermTagTense",
                entries,
            }));
        }
    }

    if let Some(value) =
        legacy_as_generated_leading_term_tag_composite_interval_property_tense_value(
            &tokens,
            free_modifiers,
            source,
            options,
        )
    {
        return Some(value);
    }

    if next
        .as_ref()
        .is_some_and(legacy_token_can_start_tense_modal)
        && let [token] = tokens.as_slice()
        && let Some(value) =
            legacy_as_generated_single_composite_tense_token_tree_value(token, source, options)
    {
        return Some(legacy_attach_free_modifiers_to_rightmost_tense_leaf(
            value,
            free_modifiers,
            source,
            options,
        ));
    }

    if next
        .as_ref()
        .is_some_and(legacy_token_can_start_tense_modal)
        && legacy_next_tag_connective_start(&tokens, 0).is_none()
        && let Some(value) =
            legacy_as_generated_time_space_caha_inner_tense_value(&tokens, source, options)
    {
        return Some(legacy_attach_free_modifiers_to_rightmost_tense_leaf(
            TreeValue::Node(TreeNode {
                constructor: "TimeSpaceCahaKiTense",
                entries: vec![TreeEntry {
                    label: Some("tense"),
                    value,
                }],
            }),
            free_modifiers,
            source,
            options,
        ));
    }

    None
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_leading_term_tag_composite_interval_property_tense_value(
    tokens: &[&Token],
    free_modifiers: &[jbotci_syntax::ast::FreeModifierSyntax],
    source: &str,
    options: TreeRenderOptions,
) -> Option<TreeValue> {
    let last = tokens.last()?;
    if !legacy_interval_property_tense_has_follower(last, free_modifiers) {
        return None;
    }

    let mut index = 0;
    let value =
        legacy_as_generated_interval_property_tense_value(tokens, &mut index, source, options)?;
    if index == tokens.len() {
        Some(value)
    } else {
        None
    }
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_leading_term_tag_event_contour_tense_value(
    words: &WithFreeModifiers<Vec<Token>>,
    source: &str,
    options: TreeRenderOptions,
) -> Option<TreeValue> {
    let [zaho] = words.value.as_slice() else {
        return None;
    };
    if !legacy_interval_property_tense_has_follower(zaho, &words.free_modifiers) {
        return None;
    }
    Some(TreeValue::Node(TreeNode {
        constructor: "ZahoIntervalPropertyTense",
        entries: vec![leading_term_tag_token_entry(
            "zaho",
            zaho,
            true,
            &words.free_modifiers,
            source,
            options,
        )],
    }))
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_leading_term_tag_interval_property_tense_value(
    number: Option<&jbotci_syntax::ast::WordRun>,
    roi_or_tahe: &WithFreeModifiers<Token>,
    nai: Option<&WithFreeModifiers<Token>>,
    source: &str,
    options: TreeRenderOptions,
) -> Option<TreeValue> {
    let last = nai.unwrap_or(roi_or_tahe);
    if !legacy_interval_property_tense_has_follower(&last.value, &last.free_modifiers) {
        return None;
    }

    if roi_or_tahe.value.is_selmaho(Selmaho::Roi) {
        let number = number?;
        let mut entries = vec![
            TreeEntry {
                label: Some("number"),
                value: legacy_word_run_tree_value(number, source, options),
            },
            TreeEntry {
                label: Some("roi"),
                value: legacy_token_tree_value_with_extra_free_modifiers(
                    &roi_or_tahe.value,
                    &roi_or_tahe.free_modifiers,
                    source,
                    options,
                ),
            },
        ];
        if let Some(nai) = nai {
            entries.extend(legacy_token_field_entries("nai", nai, source, options));
        }
        return Some(TreeValue::Node(TreeNode {
            constructor: "NumberedIntervalPropertyTense",
            entries,
        }));
    }

    if roi_or_tahe.value.is_selmaho(Selmaho::Tahe) {
        let mut entries = legacy_token_field_entries("tahe", roi_or_tahe, source, options);
        if let Some(nai) = nai {
            entries.extend(legacy_token_field_entries("nai", nai, source, options));
        }
        return Some(TreeValue::Node(TreeNode {
            constructor: "TaheIntervalPropertyTense",
            entries,
        }));
    }

    None
}

#[requires(true)]
#[ensures(true)]
fn legacy_next_tree_token_after_sequence(
    tokens: &[&Token],
    free_modifiers: &[jbotci_syntax::ast::FreeModifierSyntax],
) -> Option<Token> {
    let last = tokens.last()?;
    legacy_next_tree_token_after_with_free_modifiers(last, free_modifiers)
}

#[requires(true)]
#[ensures(true)]
fn legacy_next_tree_token_after_with_free_modifiers(
    token: &Token,
    free_modifiers: &[jbotci_syntax::ast::FreeModifierSyntax],
) -> Option<Token> {
    let mut last = token.clone();
    for free_modifier in free_modifiers {
        free_modifier.visit_words(&mut |token| last = token.clone());
    }
    legacy_next_tree_token_after(&last)
}

#[requires(true)]
#[ensures(true)]
fn legacy_interval_property_tense_has_follower(
    token: &Token,
    free_modifiers: &[jbotci_syntax::ast::FreeModifierSyntax],
) -> bool {
    let next = legacy_next_tree_token_after_with_free_modifiers(token, free_modifiers);
    let Some(next) = next else {
        return false;
    };
    if next.is_one_of_selmaho(&[Selmaho::Pu, Selmaho::Zi, Selmaho::Zeha, Selmaho::Bai])
        || next.is_cmavo(Cmavo::Fiho)
    {
        return true;
    }
    if next.is_selmaho(Selmaho::Se) {
        return legacy_next_tree_token_after(&next)
            .as_ref()
            .is_some_and(|token| token.is_selmaho(Selmaho::Bai));
    }
    if next.is_selmaho(Selmaho::Nahe) {
        return legacy_next_tree_token_after(&next)
            .as_ref()
            .is_some_and(|token| {
                token.is_selmaho(Selmaho::Caha)
                    || token.is_selmaho(Selmaho::Bai)
                    || token.is_selmaho(Selmaho::Se)
            });
    }
    false
}

#[requires(true)]
#[ensures(true)]
fn legacy_token_can_start_tense_modal(token: &Token) -> bool {
    token.is_one_of_selmaho(&[
        Selmaho::Pu,
        Selmaho::Zi,
        Selmaho::Zeha,
        Selmaho::Va,
        Selmaho::Faha,
        Selmaho::Veha,
        Selmaho::Viha,
        Selmaho::Mohi,
        Selmaho::Caha,
        Selmaho::Zaho,
        Selmaho::Pa,
        Selmaho::Tahe,
        Selmaho::Bai,
        Selmaho::Nahe,
        Selmaho::Se,
        Selmaho::Fa,
    ]) || token.is_one_of_cmavo(&[Cmavo::Fiho, Cmavo::Ki])
}

#[requires(true)]
#[ensures(true)]
fn legacy_next_tree_token_after(token: &Token) -> Option<Token> {
    LEGACY_GENERATED_TOKEN_STREAM.with(|stream| {
        let stream = stream.borrow();
        let tokens = stream.as_ref()?;
        let index = tokens
            .iter()
            .position(|candidate| legacy_tree_tokens_match(candidate, token))?;
        tokens.get(index + 1).cloned()
    })
}

#[requires(true)]
#[ensures(true)]
fn legacy_tree_tokens_match(left: &Token, right: &Token) -> bool {
    Token::ptr_eq(left, right) || legacy_tree_token_spans_match(left, right)
}

#[requires(true)]
#[ensures(true)]
fn legacy_tree_token_spans_match(left: &Token, right: &Token) -> bool {
    let left_spans = left.source_spans();
    let right_spans = right.source_spans();
    !left_spans.is_empty()
        && left_spans.len() == right_spans.len()
        && left_spans
            .iter()
            .zip(right_spans)
            .all(|(left, right)| *left == right)
}

#[requires(true)]
#[ensures(true)]
fn leading_term_tag_token_entry(
    label: &'static str,
    token: &Token,
    attach_free_modifiers: bool,
    free_modifiers: &[jbotci_syntax::ast::FreeModifierSyntax],
    source: &str,
    options: TreeRenderOptions,
) -> TreeEntry {
    TreeEntry {
        label: Some(label),
        value: if attach_free_modifiers {
            legacy_token_tree_value_with_extra_free_modifiers(
                token,
                free_modifiers,
                source,
                options,
            )
        } else {
            generated_token_tree_value(token, source, options)
        },
    }
}

#[requires(true)]
#[ensures(ret.is_some() -> tokens.iter().all(|token| token.is_selmaho(Selmaho::Pu) || token.is_cmavo(Cmavo::Nai)))]
fn leading_term_tag_pu_before_nahe_parts<'a>(
    tokens: &[&'a Token],
) -> Option<(&'a Token, Option<&'a Token>)> {
    match tokens {
        [pu] if pu.is_selmaho(Selmaho::Pu) => Some((pu, None)),
        [pu, nai] if pu.is_selmaho(Selmaho::Pu) && nai.is_cmavo(Cmavo::Nai) => {
            Some((pu, Some(nai)))
        }
        _ => None,
    }
}

#[requires(true)]
#[ensures(ret.is_some() -> tokens.iter().all(|token| token.is_selmaho(Selmaho::Pu) || token.is_selmaho(Selmaho::Zi) || token.is_cmavo(Cmavo::Nai)))]
fn leading_term_tag_pu_distance_parts<'a>(
    tokens: &[&'a Token],
) -> Option<(&'a Token, Option<&'a Token>, &'a Token)> {
    match tokens {
        [pu, distance] if pu.is_selmaho(Selmaho::Pu) && distance.is_selmaho(Selmaho::Zi) => {
            Some((pu, None, distance))
        }
        [pu, nai, distance]
            if pu.is_selmaho(Selmaho::Pu)
                && nai.is_cmavo(Cmavo::Nai)
                && distance.is_selmaho(Selmaho::Zi) =>
        {
            Some((pu, Some(nai), distance))
        }
        _ => None,
    }
}

#[requires(true)]
#[ensures(ret.is_some() -> tokens.iter().all(|token| token.is_selmaho(Selmaho::Mohi) || token.is_selmaho(Selmaho::Faha) || token.is_selmaho(Selmaho::Va) || token.is_cmavo(Cmavo::Nai)))]
fn leading_term_tag_mohi_parts<'a>(
    tokens: &[&'a Token],
) -> Option<(&'a Token, &'a Token, Option<&'a Token>, Option<&'a Token>)> {
    match tokens {
        [mohi, direction]
            if mohi.is_selmaho(Selmaho::Mohi) && direction.is_selmaho(Selmaho::Faha) =>
        {
            Some((mohi, direction, None, None))
        }
        [mohi, direction, nai]
            if mohi.is_selmaho(Selmaho::Mohi)
                && direction.is_selmaho(Selmaho::Faha)
                && nai.is_cmavo(Cmavo::Nai) =>
        {
            Some((mohi, direction, Some(nai), None))
        }
        [mohi, direction, distance]
            if mohi.is_selmaho(Selmaho::Mohi)
                && direction.is_selmaho(Selmaho::Faha)
                && distance.is_selmaho(Selmaho::Va) =>
        {
            Some((mohi, direction, None, Some(distance)))
        }
        [mohi, direction, nai, distance]
            if mohi.is_selmaho(Selmaho::Mohi)
                && direction.is_selmaho(Selmaho::Faha)
                && nai.is_cmavo(Cmavo::Nai)
                && distance.is_selmaho(Selmaho::Va) =>
        {
            Some((mohi, direction, Some(nai), Some(distance)))
        }
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_composite_tense_modal_tree_value(
    parts: &WithFreeModifiers<Vec<jbotci_syntax::ast::CompositeTenseModalPartSyntax>>,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    let attach_free_modifiers = |value| {
        legacy_attach_free_modifiers_to_rightmost_tense_leaf(
            value,
            &parts.free_modifiers,
            source,
            options,
        )
    };
    if let Some(tokens) = legacy_composite_tense_modal_part_tokens(&parts.value) {
        if let Some(value) =
            legacy_as_generated_zantufa_recursive_tag_tense_tree_value(&tokens, source, options)
        {
            return attach_free_modifiers(value);
        }
        if let Some(value) =
            legacy_as_generated_connected_tense_modal_tree_value(&tokens, source, options)
        {
            return attach_free_modifiers(value);
        }
        if let Some(value) =
            legacy_as_generated_time_tense_sequence_tree_value(&tokens, source, options)
        {
            return attach_free_modifiers(value);
        }
        if let Some(value) =
            legacy_as_generated_space_then_time_tense_sequence_tree_value(&tokens, source, options)
        {
            return attach_free_modifiers(value);
        }
        if let [pu, distance] = tokens.as_slice()
            && pu.is_selmaho(Selmaho::Pu)
            && distance.is_selmaho(Selmaho::Zi)
        {
            return attach_free_modifiers(TreeValue::Node(TreeNode {
                constructor: "TimeSpaceCahaKiTense",
                entries: vec![TreeEntry {
                    label: Some("tense"),
                    value: TreeValue::Node(TreeNode {
                        constructor: "TimeThenSpaceCahaTense",
                        entries: vec![TreeEntry {
                            label: Some("time"),
                            value: TreeValue::Node(TreeNode {
                                constructor: "TimeTenseWithOffset",
                                entries: vec![TreeEntry {
                                    label: Some("offsets"),
                                    value: TreeValue::Collection(vec![TreeValue::Node(TreeNode {
                                        constructor: "PuTimeOffsetTense",
                                        entries: vec![
                                            TreeEntry {
                                                label: Some("pu"),
                                                value: generated_token_tree_value(
                                                    pu, source, options,
                                                ),
                                            },
                                            TreeEntry {
                                                label: Some("distance"),
                                                value: generated_token_tree_value(
                                                    distance, source, options,
                                                ),
                                            },
                                        ],
                                    })]),
                                }],
                            }),
                        }],
                    }),
                }],
            }));
        }
        if tokens.len() > 1 && tokens.iter().all(|token| token.is_selmaho(Selmaho::Pu)) {
            return attach_free_modifiers(TreeValue::Node(TreeNode {
                constructor: "TimeSpaceCahaKiTense",
                entries: vec![TreeEntry {
                    label: Some("tense"),
                    value: TreeValue::Node(TreeNode {
                        constructor: "TimeThenSpaceCahaTense",
                        entries: vec![TreeEntry {
                            label: Some("time"),
                            value: TreeValue::Node(TreeNode {
                                constructor: "TimeTenseWithOffset",
                                entries: vec![TreeEntry {
                                    label: Some("offsets"),
                                    value: TreeValue::Collection(
                                        tokens
                                            .iter()
                                            .map(|token| {
                                                TreeValue::Node(TreeNode {
                                                    constructor: "PuTimeOffsetTense",
                                                    entries: vec![TreeEntry {
                                                        label: Some("pu"),
                                                        value: generated_token_tree_value(
                                                            token, source, options,
                                                        ),
                                                    }],
                                                })
                                            })
                                            .collect(),
                                    ),
                                }],
                            }),
                        }],
                    }),
                }],
            }));
        }
        if let [token] = tokens.as_slice()
            && token.is_selmaho(Selmaho::Fa)
        {
            return attach_free_modifiers(TreeValue::Node(TreeNode {
                constructor: "FaFlatTagTense",
                entries: vec![TreeEntry {
                    label: Some("fa"),
                    value: generated_token_tree_value(token, source, options),
                }],
            }));
        }
        if let [token] = tokens.as_slice()
            && let Some(value) =
                legacy_as_generated_single_composite_tense_token_tree_value(token, source, options)
        {
            return attach_free_modifiers(value);
        }
        if let [se, rest @ ..] = tokens.as_slice()
            && se.is_selmaho(Selmaho::Se)
            && !rest.is_empty()
            && let Some(atom_value) =
                legacy_as_generated_composite_flat_tag_atom_tree_value(rest, source, options)
        {
            return attach_free_modifiers(TreeValue::Node(TreeNode {
                constructor: "SeFlatPrefixedTense",
                entries: vec![
                    TreeEntry {
                        label: Some("se"),
                        value: generated_token_tree_value(se, source, options),
                    },
                    TreeEntry {
                        label: Some("atom"),
                        value: atom_value,
                    },
                ],
            }));
        }
        if let [se, atom] = tokens.as_slice()
            && se.is_selmaho(Selmaho::Se)
            && let Some(atom_value) =
                legacy_as_generated_flat_tag_atom_tree_value(atom, source, options)
        {
            return attach_free_modifiers(TreeValue::Node(TreeNode {
                constructor: "SeFlatPrefixedTense",
                entries: vec![
                    TreeEntry {
                        label: Some("se"),
                        value: generated_token_tree_value(se, source, options),
                    },
                    TreeEntry {
                        label: Some("atom"),
                        value: atom_value,
                    },
                ],
            }));
        }
        if let [nahe, rest @ ..] = tokens.as_slice()
            && nahe.is_selmaho(Selmaho::Nahe)
            && let Some(tense) =
                legacy_as_generated_time_space_caha_inner_tense_value(rest, source, options)
        {
            return attach_free_modifiers(TreeValue::Node(TreeNode {
                constructor: "PrefixedTimeSpaceCahaTense",
                entries: vec![
                    TreeEntry {
                        label: Some("nahe"),
                        value: generated_token_tree_value(nahe, source, options),
                    },
                    TreeEntry {
                        label: Some("tense"),
                        value: tense,
                    },
                ],
            }));
        }
        if let [nahe, atom] = tokens.as_slice()
            && nahe.is_selmaho(Selmaho::Nahe)
            && let Some(tense) =
                legacy_as_generated_time_space_caha_inner_tense_value(&[*atom], source, options)
        {
            return attach_free_modifiers(TreeValue::Node(TreeNode {
                constructor: "PrefixedTimeSpaceCahaTense",
                entries: vec![
                    TreeEntry {
                        label: Some("nahe"),
                        value: generated_token_tree_value(nahe, source, options),
                    },
                    TreeEntry {
                        label: Some("tense"),
                        value: tense,
                    },
                ],
            }));
        }
        if let [nahe, atom] = tokens.as_slice()
            && nahe.is_selmaho(Selmaho::Nahe)
            && let Some(atom_value) =
                legacy_as_generated_flat_tag_atom_tree_value(atom, source, options)
        {
            return attach_free_modifiers(TreeValue::Node(TreeNode {
                constructor: "NaheSeFlatPrefixedTense",
                entries: vec![
                    TreeEntry {
                        label: Some("nahe"),
                        value: generated_token_tree_value(nahe, source, options),
                    },
                    TreeEntry {
                        label: Some("atom"),
                        value: atom_value,
                    },
                ],
            }));
        }
        if let [nahe, se, atom] = tokens.as_slice()
            && nahe.is_selmaho(Selmaho::Nahe)
            && se.is_selmaho(Selmaho::Se)
            && let Some(atom_value) =
                legacy_as_generated_flat_tag_atom_tree_value(atom, source, options)
        {
            return attach_free_modifiers(TreeValue::Node(TreeNode {
                constructor: "NaheSeFlatPrefixedTense",
                entries: vec![
                    TreeEntry {
                        label: Some("nahe"),
                        value: generated_token_tree_value(nahe, source, options),
                    },
                    TreeEntry {
                        label: Some("se"),
                        value: generated_token_tree_value(se, source, options),
                    },
                    TreeEntry {
                        label: Some("atom"),
                        value: atom_value,
                    },
                ],
            }));
        }
    }
    if let Some(value) =
        legacy_as_generated_composite_tense_modal_parts_tree_value(&parts.value, source, options)
    {
        return attach_free_modifiers(value);
    }
    if parts.value.len() == 1
        && let bityzba::data!(jbotci_syntax::ast::CompositeTenseModalPartSyntax::Cmavo(
            token
        )) = parts.value[0].as_data()
        && token.is_selmaho(Selmaho::Pu)
    {
        return attach_free_modifiers(TreeValue::Node(TreeNode {
            constructor: "TimeSpaceCahaKiTense",
            entries: vec![TreeEntry {
                label: Some("tense"),
                value: TreeValue::Node(TreeNode {
                    constructor: "TimeThenSpaceCahaTense",
                    entries: vec![TreeEntry {
                        label: Some("time"),
                        value: TreeValue::Node(TreeNode {
                            constructor: "TimeTenseWithOffset",
                            entries: vec![TreeEntry {
                                label: Some("offsets"),
                                value: TreeValue::Collection(vec![TreeValue::Node(TreeNode {
                                    constructor: "PuTimeOffsetTense",
                                    entries: vec![TreeEntry {
                                        label: Some("pu"),
                                        value: generated_token_tree_value(token, source, options),
                                    }],
                                })]),
                            }],
                        }),
                    }],
                }),
            }],
        }));
    }
    if parts.value.len() == 1
        && let bityzba::data!(jbotci_syntax::ast::CompositeTenseModalPartSyntax::Cmavo(
            token
        )) = parts.value[0].as_data()
        && token.is_selmaho(Selmaho::Va)
    {
        return attach_free_modifiers(TreeValue::Node(TreeNode {
            constructor: "TimeSpaceCahaKiTense",
            entries: vec![TreeEntry {
                label: Some("tense"),
                value: TreeValue::Node(TreeNode {
                    constructor: "SpaceThenTimeCahaTense",
                    entries: vec![TreeEntry {
                        label: Some("space"),
                        value: TreeValue::Node(TreeNode {
                            constructor: "SpaceTenseWithVa",
                            entries: vec![TreeEntry {
                                label: Some("va"),
                                value: TreeValue::Node(TreeNode {
                                    constructor: "VaSpaceDistanceTense",
                                    entries: vec![TreeEntry {
                                        label: Some("va"),
                                        value: generated_token_tree_value(token, source, options),
                                    }],
                                }),
                            }],
                        }),
                    }],
                }),
            }],
        }));
    }
    if parts.value.len() == 1
        && let bityzba::data!(jbotci_syntax::ast::CompositeTenseModalPartSyntax::Cmavo(
            token
        )) = parts.value[0].as_data()
        && token.is_selmaho(Selmaho::Faha)
    {
        return attach_free_modifiers(TreeValue::Node(TreeNode {
            constructor: "TimeSpaceCahaKiTense",
            entries: vec![TreeEntry {
                label: Some("tense"),
                value: TreeValue::Node(TreeNode {
                    constructor: "SpaceThenTimeCahaTense",
                    entries: vec![TreeEntry {
                        label: Some("space"),
                        value: TreeValue::Node(TreeNode {
                            constructor: "SpaceTenseWithOffset",
                            entries: vec![TreeEntry {
                                label: Some("offsets"),
                                value: TreeValue::Collection(vec![TreeValue::Node(TreeNode {
                                    constructor: "FahaSpaceOffsetTense",
                                    entries: vec![TreeEntry {
                                        label: Some("faha"),
                                        value: generated_token_tree_value(token, source, options),
                                    }],
                                })]),
                            }],
                        }),
                    }],
                }),
            }],
        }));
    }
    required_legacy_syntax_subtree_value(parts, source, options)
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_composite_tense_modal_parts_tree_value(
    parts: &[jbotci_syntax::ast::CompositeTenseModalPartSyntax],
    source: &str,
    options: TreeRenderOptions,
) -> Option<TreeValue> {
    let mut index = 0;
    let first = legacy_as_generated_composite_tense_modal_part_atom_tree_value(
        parts, &mut index, source, options,
    )?;
    let mut continuations = Vec::new();
    while index < parts.len() {
        let connective = legacy_as_generated_composite_tense_modal_part_connective_tree_value(
            parts, &mut index, source, options,
        )?;
        let atom = legacy_as_generated_composite_tense_modal_part_atom_tree_value(
            parts, &mut index, source, options,
        )?;
        continuations.push(TreeValue::Collection(vec![connective, atom]));
    }
    if continuations.is_empty() {
        return Some(first);
    }
    Some(TreeValue::Node(TreeNode {
        constructor: "ConnectedTenseModal",
        entries: vec![
            TreeEntry {
                label: Some("first"),
                value: first,
            },
            TreeEntry {
                label: Some("continuations"),
                value: TreeValue::Collection(continuations),
            },
        ],
    }))
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_composite_tense_modal_part_atom_tree_value(
    parts: &[jbotci_syntax::ast::CompositeTenseModalPartSyntax],
    index: &mut usize,
    source: &str,
    options: TreeRenderOptions,
) -> Option<TreeValue> {
    let part = parts.get(*index)?;
    match part.as_data() {
        bityzba::data!(jbotci_syntax::ast::CompositeTenseModalPartSyntax::AdHocModal(modal)) => {
            *index += 1;
            Some(legacy_as_generated_ad_hoc_modal_tense_tree_value(
                modal.as_ref(),
                source,
                options,
            ))
        }
        bityzba::data!(jbotci_syntax::ast::CompositeTenseModalPartSyntax::Cmavo(_)) => {
            let tokens = legacy_composite_tense_modal_part_token_run(parts, *index);
            if tokens.is_empty() {
                return None;
            }
            let mut token_index = 0;
            let value = legacy_as_generated_connected_tense_atom_tree_value(
                &tokens,
                &mut token_index,
                source,
                options,
            )?;
            *index += token_index;
            Some(value)
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_composite_tense_modal_part_connective_tree_value(
    parts: &[jbotci_syntax::ast::CompositeTenseModalPartSyntax],
    index: &mut usize,
    source: &str,
    options: TreeRenderOptions,
) -> Option<TreeValue> {
    let tokens = legacy_composite_tense_modal_part_token_run(parts, *index);
    if tokens.is_empty() {
        return None;
    }
    let mut token_index = 0;
    let value =
        legacy_as_generated_tag_connective_tree_value(&tokens, &mut token_index, source, options)?;
    *index += token_index;
    Some(value)
}

#[requires(true)]
#[ensures(true)]
fn legacy_composite_tense_modal_part_token_run(
    parts: &[jbotci_syntax::ast::CompositeTenseModalPartSyntax],
    start: usize,
) -> Vec<&Token> {
    parts
        .iter()
        .skip(start)
        .map_while(|part| match part.as_data() {
            bityzba::data!(jbotci_syntax::ast::CompositeTenseModalPartSyntax::Cmavo(
                token
            )) => Some(token),
            _ => None,
        })
        .collect()
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_ad_hoc_modal_tense_tree_value(
    modal: &jbotci_syntax::ast::AdHocModalSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    let mut entries = Vec::new();
    if let Some(nahe) = &modal.nahe {
        entries.push(TreeEntry {
            label: Some("nahe"),
            value: generated_token_tree_value(nahe, source, options),
        });
    }
    entries.extend(legacy_token_field_entries(
        "fiho",
        &modal.fiho,
        source,
        options,
    ));
    entries.push(TreeEntry {
        label: Some("selbri"),
        value: legacy_as_generated_selbri_tree_value(modal.selbri.as_ref(), source, options),
    });
    if let Some(fehu) = &modal.fehu {
        entries.extend(legacy_token_field_entries("fehu", fehu, source, options));
    }
    TreeValue::Node(TreeNode {
        constructor: "FihoTense",
        entries,
    })
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_zantufa_recursive_tag_tense_tree_value(
    tokens: &[&Token],
    source: &str,
    options: TreeRenderOptions,
) -> Option<TreeValue> {
    if tokens.len() < 4
        || !tokens[..tokens.len() - 1]
            .iter()
            .all(|token| token.is_selmaho(Selmaho::Nahe) || token.is_selmaho(Selmaho::Se))
    {
        return None;
    }
    let atom = tokens[tokens.len() - 1];
    if !(atom.is_selmaho(Selmaho::Fa)
        || atom.is_selmaho(Selmaho::Pu)
        || atom.is_selmaho(Selmaho::Zi)
        || atom.is_selmaho(Selmaho::Zeha)
        || atom.is_selmaho(Selmaho::Va)
        || atom.is_selmaho(Selmaho::Faha)
        || atom.is_selmaho(Selmaho::Veha)
        || atom.is_selmaho(Selmaho::Viha)
        || atom.is_selmaho(Selmaho::Caha)
        || atom.is_selmaho(Selmaho::Zaho)
        || atom.is_selmaho(Selmaho::Cuhe)
        || atom.is_cmavo(Cmavo::Ki))
    {
        return None;
    }

    let mut entries = vec![TreeEntry {
        label: Some("first_prefix"),
        value: generated_token_tree_value(tokens[0], source, options),
    }];
    if let Some(entry) = labelled_tree_collection_entry_from_values(
        "additional_prefixes",
        tokens[1..tokens.len() - 1]
            .iter()
            .map(|token| generated_token_tree_value(token, source, options))
            .collect(),
    ) {
        entries.push(entry);
    }
    entries.push(TreeEntry {
        label: Some("atom"),
        value: generated_token_tree_value(atom, source, options),
    });

    Some(TreeValue::Node(TreeNode {
        constructor: "ZantufaRecursiveTagTense",
        entries,
    }))
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_connected_tense_modal_tree_value(
    tokens: &[&Token],
    source: &str,
    options: TreeRenderOptions,
) -> Option<TreeValue> {
    let mut index = 0;
    let first =
        legacy_as_generated_connected_tense_atom_tree_value(tokens, &mut index, source, options)?;
    let mut continuations = Vec::new();
    while index < tokens.len() {
        let connective =
            legacy_as_generated_tag_connective_tree_value(tokens, &mut index, source, options)?;
        let atom = legacy_as_generated_connected_tense_atom_tree_value(
            tokens, &mut index, source, options,
        )?;
        continuations.push(TreeValue::Collection(vec![connective, atom]));
    }
    if continuations.is_empty() {
        return None;
    }
    Some(TreeValue::Node(TreeNode {
        constructor: "ConnectedTenseModal",
        entries: vec![
            TreeEntry {
                label: Some("first"),
                value: first,
            },
            TreeEntry {
                label: Some("continuations"),
                value: TreeValue::Collection(continuations),
            },
        ],
    }))
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_connected_tense_atom_tree_value(
    tokens: &[&Token],
    index: &mut usize,
    source: &str,
    options: TreeRenderOptions,
) -> Option<TreeValue> {
    let start = *index;
    if let Some(value) =
        legacy_as_generated_flat_tense_modal_atom_tree_value(tokens, index, source, options)
    {
        return Some(value);
    }
    *index = start;
    if *index >= tokens.len() {
        return None;
    }
    let end = legacy_next_tag_connective_start(tokens, *index + 1).unwrap_or(tokens.len());
    if end == *index {
        return None;
    }
    if let [cuhe] = &tokens[*index..end]
        && cuhe.is_selmaho(Selmaho::Cuhe)
    {
        *index = end;
        return Some(TreeValue::Node(TreeNode {
            constructor: "CuheTense",
            entries: vec![TreeEntry {
                label: Some("cuhe"),
                value: generated_token_tree_value(cuhe, source, options),
            }],
        }));
    }
    let value =
        legacy_as_generated_time_tense_sequence_tree_value(&tokens[*index..end], source, options)
            .or_else(|| {
            legacy_as_generated_space_then_time_tense_sequence_tree_value(
                &tokens[*index..end],
                source,
                options,
            )
        })?;
    *index = end;
    Some(value)
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_flat_tense_modal_atom_tree_value(
    tokens: &[&Token],
    index: &mut usize,
    source: &str,
    options: TreeRenderOptions,
) -> Option<TreeValue> {
    let start = *index;
    let nahe = tokens
        .get(*index)
        .filter(|token| token.is_selmaho(Selmaho::Nahe));
    if nahe.is_some() {
        *index += 1;
    }
    let se = tokens
        .get(*index)
        .filter(|token| token.is_selmaho(Selmaho::Se));
    if se.is_some() {
        *index += 1;
    }
    let bai = tokens
        .get(*index)
        .filter(|token| token.is_selmaho(Selmaho::Bai))?;
    *index += 1;
    let nai = tokens
        .get(*index)
        .filter(|token| token.is_cmavo(Cmavo::Nai));
    if nai.is_some() {
        *index += 1;
    }
    let ki = tokens.get(*index).filter(|token| token.is_cmavo(Cmavo::Ki));
    if ki.is_some() {
        *index += 1;
    }
    if *index == start {
        return None;
    }

    let mut entries = Vec::new();
    if let Some(nahe) = nahe {
        entries.push(TreeEntry {
            label: Some("nahe"),
            value: generated_token_tree_value(nahe, source, options),
        });
    }
    if let Some(se) = se {
        entries.push(TreeEntry {
            label: Some("se"),
            value: generated_token_tree_value(se, source, options),
        });
    }
    entries.push(TreeEntry {
        label: Some("bai"),
        value: generated_token_tree_value(bai, source, options),
    });
    if let Some(nai) = nai {
        entries.push(TreeEntry {
            label: Some("nai"),
            value: generated_token_tree_value(nai, source, options),
        });
    }
    if let Some(ki) = ki {
        entries.push(TreeEntry {
            label: Some("ki"),
            value: generated_token_tree_value(ki, source, options),
        });
    }
    Some(TreeValue::Node(TreeNode {
        constructor: "ModalTense",
        entries,
    }))
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_time_tense_sequence_tree_value(
    tokens: &[&Token],
    source: &str,
    options: TreeRenderOptions,
) -> Option<TreeValue> {
    let mut index = 0;
    let zi = if tokens
        .get(index)
        .is_some_and(|token| token.is_selmaho(Selmaho::Zi))
    {
        let token = tokens[index];
        index += 1;
        Some(TreeValue::Node(TreeNode {
            constructor: "ZiTimeDistanceTense",
            entries: vec![TreeEntry {
                label: Some("zi"),
                value: generated_token_tree_value(token, source, options),
            }],
        }))
    } else {
        None
    };

    let mut offsets = Vec::new();
    while tokens
        .get(index)
        .is_some_and(|token| token.is_selmaho(Selmaho::Pu))
    {
        let pu = tokens[index];
        index += 1;
        let mut entries = vec![TreeEntry {
            label: Some("pu"),
            value: generated_token_tree_value(pu, source, options),
        }];
        if tokens
            .get(index)
            .is_some_and(|token| token.is_cmavo(Cmavo::Nai))
        {
            entries.push(TreeEntry {
                label: Some("nai"),
                value: generated_token_tree_value(tokens[index], source, options),
            });
            index += 1;
        }
        if tokens
            .get(index)
            .is_some_and(|token| token.is_selmaho(Selmaho::Zi))
        {
            entries.push(TreeEntry {
                label: Some("distance"),
                value: generated_token_tree_value(tokens[index], source, options),
            });
            index += 1;
        }
        offsets.push(TreeValue::Node(TreeNode {
            constructor: "PuTimeOffsetTense",
            entries,
        }));
    }

    let zeha = if tokens
        .get(index)
        .is_some_and(|token| token.is_selmaho(Selmaho::Zeha))
    {
        let zeha = tokens[index];
        index += 1;
        let mut entries = vec![TreeEntry {
            label: Some("zeha"),
            value: generated_token_tree_value(zeha, source, options),
        }];
        if tokens
            .get(index)
            .is_some_and(|token| token.is_selmaho(Selmaho::Pu))
        {
            let mut direction = vec![generated_token_tree_value(tokens[index], source, options)];
            index += 1;
            if tokens
                .get(index)
                .is_some_and(|token| token.is_cmavo(Cmavo::Nai))
            {
                direction.push(generated_token_tree_value(tokens[index], source, options));
                index += 1;
            }
            entries.push(TreeEntry {
                label: Some("direction"),
                value: TreeValue::Collection(direction),
            });
        }
        Some(TreeValue::Node(TreeNode {
            constructor: "ZehaTimeIntervalTense",
            entries,
        }))
    } else {
        None
    };

    let mut properties = Vec::new();
    loop {
        if index >= tokens.len() {
            break;
        }
        if tokens[index].is_selmaho(Selmaho::Zaho) {
            let zaho = tokens[index];
            index += 1;
            let mut entries = vec![TreeEntry {
                label: Some("zaho"),
                value: generated_token_tree_value(zaho, source, options),
            }];
            if tokens
                .get(index)
                .is_some_and(|token| token.is_cmavo(Cmavo::Nai))
            {
                entries.push(TreeEntry {
                    label: Some("nai"),
                    value: generated_token_tree_value(tokens[index], source, options),
                });
                index += 1;
            }
            properties.push(TreeValue::Node(TreeNode {
                constructor: "ZahoIntervalPropertyTense",
                entries,
            }));
        } else if tokens[index].is_selmaho(Selmaho::Tahe) {
            let tahe = tokens[index];
            index += 1;
            let mut entries = vec![TreeEntry {
                label: Some("tahe"),
                value: generated_token_tree_value(tahe, source, options),
            }];
            if tokens
                .get(index)
                .is_some_and(|token| token.is_cmavo(Cmavo::Nai))
            {
                entries.push(TreeEntry {
                    label: Some("nai"),
                    value: generated_token_tree_value(tokens[index], source, options),
                });
                index += 1;
            }
            properties.push(TreeValue::Node(TreeNode {
                constructor: "TaheIntervalPropertyTense",
                entries,
            }));
        } else if tokens[index].is_selmaho(Selmaho::Pa) {
            let start = index;
            index += 1;
            while tokens
                .get(index)
                .is_some_and(|token| legacy_token_is_number_or_letter_word(token))
            {
                index += 1;
            }
            let roi = tokens
                .get(index)
                .filter(|token| token.is_selmaho(Selmaho::Roi))?;
            index += 1;
            let mut entries = vec![
                TreeEntry {
                    label: Some("number"),
                    value: TreeValue::Collection(
                        tokens[start..index - 1]
                            .iter()
                            .map(|token| generated_token_tree_value(token, source, options))
                            .collect(),
                    ),
                },
                TreeEntry {
                    label: Some("roi"),
                    value: generated_token_tree_value(roi, source, options),
                },
            ];
            if tokens
                .get(index)
                .is_some_and(|token| token.is_cmavo(Cmavo::Nai))
            {
                entries.push(TreeEntry {
                    label: Some("nai"),
                    value: generated_token_tree_value(tokens[index], source, options),
                });
                index += 1;
            }
            properties.push(TreeValue::Node(TreeNode {
                constructor: "NumberedIntervalPropertyTense",
                entries,
            }));
        } else {
            break;
        }
    }
    let space =
        legacy_as_generated_space_tense_sequence_tree_value(tokens, &mut index, source, options)?;
    let caha = if tokens
        .get(index)
        .is_some_and(|token| token.is_selmaho(Selmaho::Caha))
    {
        let caha = tokens[index];
        index += 1;
        Some(TreeValue::Node(TreeNode {
            constructor: "CahaTense",
            entries: vec![TreeEntry {
                label: Some("caha"),
                value: generated_token_tree_value(caha, source, options),
            }],
        }))
    } else {
        None
    };
    let ki = if tokens
        .get(index)
        .is_some_and(|token| token.is_cmavo(Cmavo::Ki))
    {
        let ki = tokens[index];
        index += 1;
        Some(TreeValue::Node(TreeNode {
            constructor: "KiCompositeTense",
            entries: vec![TreeEntry {
                label: Some("ki"),
                value: generated_token_tree_value(ki, source, options),
            }],
        }))
    } else {
        None
    };
    if index != tokens.len() {
        return None;
    }

    let time_constructor = if zi.is_some() {
        Some("TimeTenseWithZi")
    } else if !offsets.is_empty() {
        Some("TimeTenseWithOffset")
    } else if zeha.is_some() {
        Some("TimeTenseWithInterval")
    } else if !properties.is_empty() {
        Some("TimeTenseWithProperties")
    } else {
        None
    };
    let mut tense_entries = Vec::new();
    if let Some(constructor) = time_constructor {
        let mut time_entries = Vec::new();
        if let Some(zi) = zi {
            time_entries.push(TreeEntry {
                label: Some("zi"),
                value: zi,
            });
        }
        if let Some(entry) = labelled_tree_collection_entry_from_values("offsets", offsets) {
            time_entries.push(entry);
        }
        if let Some(zeha) = zeha {
            time_entries.push(TreeEntry {
                label: Some("zeha"),
                value: zeha,
            });
        }
        if let Some(entry) = labelled_tree_collection_entry_from_values("properties", properties) {
            time_entries.push(entry);
        }
        tense_entries.push(TreeEntry {
            label: Some("time"),
            value: TreeValue::Node(TreeNode {
                constructor,
                entries: time_entries,
            }),
        });
        if let Some(space) = space {
            tense_entries.push(TreeEntry {
                label: Some("space"),
                value: space,
            });
        }
        if let Some(caha) = caha {
            tense_entries.push(TreeEntry {
                label: Some("caha"),
                value: caha,
            });
        }
    } else if let Some(space) = space {
        tense_entries.push(TreeEntry {
            label: Some("space"),
            value: space,
        });
        if let Some(caha) = caha {
            tense_entries.push(TreeEntry {
                label: Some("caha"),
                value: caha,
            });
        }
    } else if let Some(caha) = caha {
        let mut entries = vec![TreeEntry {
            label: Some("tense"),
            value: caha,
        }];
        if let Some(ki) = ki {
            entries.push(TreeEntry {
                label: Some("ki"),
                value: ki,
            });
        }
        return Some(TreeValue::Node(TreeNode {
            constructor: "TimeSpaceCahaKiTense",
            entries,
        }));
    } else {
        return None;
    }

    let tense_constructor = if time_constructor.is_some() {
        "TimeThenSpaceCahaTense"
    } else {
        "SpaceThenTimeCahaTense"
    };
    let mut entries = vec![TreeEntry {
        label: Some("tense"),
        value: TreeValue::Node(TreeNode {
            constructor: tense_constructor,
            entries: tense_entries,
        }),
    }];
    if let Some(ki) = ki {
        entries.push(TreeEntry {
            label: Some("ki"),
            value: ki,
        });
    }

    Some(TreeValue::Node(TreeNode {
        constructor: "TimeSpaceCahaKiTense",
        entries,
    }))
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_space_then_time_tense_sequence_tree_value(
    tokens: &[&Token],
    source: &str,
    options: TreeRenderOptions,
) -> Option<TreeValue> {
    let mut index = 0;
    let space =
        legacy_as_generated_space_tense_sequence_tree_value(tokens, &mut index, source, options)??;
    if index >= tokens.len() {
        return None;
    }

    let time_outer =
        legacy_as_generated_time_tense_sequence_tree_value(&tokens[index..], source, options)?;
    let TreeValue::Node(TreeNode {
        constructor: "TimeSpaceCahaKiTense",
        entries,
    }) = time_outer
    else {
        return None;
    };

    let mut time = None;
    let mut caha = None;
    let mut ki = None;
    for entry in entries {
        match entry.label {
            Some("ki") => ki = Some(entry.value),
            Some("tense") => {
                let TreeValue::Node(TreeNode { entries, .. }) = entry.value else {
                    return None;
                };
                for tense_entry in entries {
                    match tense_entry.label {
                        Some("time") => time = Some(tense_entry.value),
                        Some("caha") => caha = Some(tense_entry.value),
                        Some("space") => return None,
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    let mut tense_entries = vec![TreeEntry {
        label: Some("space"),
        value: space,
    }];
    tense_entries.push(TreeEntry {
        label: Some("time"),
        value: time?,
    });
    if let Some(caha) = caha {
        tense_entries.push(TreeEntry {
            label: Some("caha"),
            value: caha,
        });
    }

    let mut entries = vec![TreeEntry {
        label: Some("tense"),
        value: TreeValue::Node(TreeNode {
            constructor: "SpaceThenTimeCahaTense",
            entries: tense_entries,
        }),
    }];
    if let Some(ki) = ki {
        entries.push(TreeEntry {
            label: Some("ki"),
            value: ki,
        });
    }

    Some(TreeValue::Node(TreeNode {
        constructor: "TimeSpaceCahaKiTense",
        entries,
    }))
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_time_space_caha_inner_tense_value(
    tokens: &[&Token],
    source: &str,
    options: TreeRenderOptions,
) -> Option<TreeValue> {
    let outer = legacy_as_generated_time_tense_sequence_tree_value(tokens, source, options)?;
    match outer {
        TreeValue::Node(TreeNode {
            constructor: "TimeSpaceCahaKiTense",
            entries,
        }) => entries
            .into_iter()
            .find(|entry| entry.label == Some("tense"))
            .map(|entry| entry.value),
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_space_tense_sequence_tree_value(
    tokens: &[&Token],
    index: &mut usize,
    source: &str,
    options: TreeRenderOptions,
) -> Option<Option<TreeValue>> {
    let start = *index;
    let va = if tokens
        .get(*index)
        .is_some_and(|token| token.is_selmaho(Selmaho::Va))
    {
        let va = tokens[*index];
        *index += 1;
        Some(TreeValue::Node(TreeNode {
            constructor: "VaSpaceDistanceTense",
            entries: vec![TreeEntry {
                label: Some("va"),
                value: generated_token_tree_value(va, source, options),
            }],
        }))
    } else {
        None
    };

    let mut offsets = Vec::new();
    while let Some(offset) =
        legacy_as_generated_faha_space_offset_tense_value(tokens, index, source, options)
    {
        offsets.push(offset);
    }

    let interval = legacy_as_generated_space_interval_tense_value(tokens, index, source, options)?;
    let mohi = if tokens
        .get(*index)
        .is_some_and(|token| token.is_selmaho(Selmaho::Mohi))
    {
        let mohi = tokens[*index];
        *index += 1;
        let offset =
            legacy_as_generated_faha_space_offset_tense_value(tokens, index, source, options)?;
        Some(TreeValue::Node(TreeNode {
            constructor: "MohiSpaceOffsetTense",
            entries: vec![
                TreeEntry {
                    label: Some("mohi"),
                    value: generated_token_tree_value(mohi, source, options),
                },
                TreeEntry {
                    label: Some("offset"),
                    value: offset,
                },
            ],
        }))
    } else {
        None
    };

    if va.is_none() && offsets.is_empty() && interval.is_none() && mohi.is_none() {
        *index = start;
        return Some(None);
    }
    let constructor = if va.is_some() {
        "SpaceTenseWithVa"
    } else if !offsets.is_empty() {
        "SpaceTenseWithOffset"
    } else if interval.is_some() {
        "SpaceTenseWithInterval"
    } else {
        "SpaceTenseWithMohi"
    };
    let mut entries = Vec::new();
    if let Some(va) = va {
        entries.push(TreeEntry {
            label: Some("va"),
            value: va,
        });
    }
    if let Some(entry) = labelled_tree_collection_entry_from_values("offsets", offsets) {
        entries.push(entry);
    }
    if let Some(interval) = interval {
        entries.push(TreeEntry {
            label: Some("interval"),
            value: interval,
        });
    }
    if let Some(mohi) = mohi {
        entries.push(TreeEntry {
            label: Some("mohi"),
            value: mohi,
        });
    }

    Some(Some(TreeValue::Node(TreeNode {
        constructor,
        entries,
    })))
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_faha_space_offset_tense_value(
    tokens: &[&Token],
    index: &mut usize,
    source: &str,
    options: TreeRenderOptions,
) -> Option<TreeValue> {
    let mut next = *index;
    let faha = tokens
        .get(next)
        .filter(|token| token.is_selmaho(Selmaho::Faha))?;
    next += 1;
    let mut entries = vec![TreeEntry {
        label: Some("faha"),
        value: generated_token_tree_value(faha, source, options),
    }];
    if tokens
        .get(next)
        .is_some_and(|token| token.is_cmavo(Cmavo::Nai))
    {
        entries.push(TreeEntry {
            label: Some("nai"),
            value: generated_token_tree_value(tokens[next], source, options),
        });
        next += 1;
    }
    if tokens
        .get(next)
        .is_some_and(|token| token.is_selmaho(Selmaho::Va))
    {
        entries.push(TreeEntry {
            label: Some("distance"),
            value: generated_token_tree_value(tokens[next], source, options),
        });
        next += 1;
    }
    *index = next;
    Some(TreeValue::Node(TreeNode {
        constructor: "FahaSpaceOffsetTense",
        entries,
    }))
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_space_interval_tense_value(
    tokens: &[&Token],
    index: &mut usize,
    source: &str,
    options: TreeRenderOptions,
) -> Option<Option<TreeValue>> {
    let start = *index;
    if let Some(properties) =
        legacy_as_generated_space_interval_properties_tense_value(tokens, index, source, options)
    {
        return Some(Some(properties));
    }

    let Some(extent) =
        legacy_as_generated_veha_viha_space_interval_tense_value(tokens, index, source, options)
    else {
        *index = start;
        return Some(None);
    };
    let direction =
        legacy_as_generated_faha_interval_direction_tense_value(tokens, index, source, options);
    let properties =
        legacy_as_generated_space_interval_properties_tense_value(tokens, index, source, options);
    let mut entries = vec![TreeEntry {
        label: Some("extent"),
        value: extent,
    }];
    if let Some(direction) = direction {
        entries.push(TreeEntry {
            label: Some("direction"),
            value: direction,
        });
    }
    if let Some(properties) = properties {
        entries.push(TreeEntry {
            label: Some("properties"),
            value: properties,
        });
    }
    Some(Some(TreeValue::Node(TreeNode {
        constructor: "SpaceIntervalWithExtentTense",
        entries,
    })))
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_veha_viha_space_interval_tense_value(
    tokens: &[&Token],
    index: &mut usize,
    source: &str,
    options: TreeRenderOptions,
) -> Option<TreeValue> {
    if tokens
        .get(*index)
        .is_some_and(|token| token.is_selmaho(Selmaho::Veha))
    {
        let veha = tokens[*index];
        *index += 1;
        let mut entries = vec![TreeEntry {
            label: Some("veha"),
            value: generated_token_tree_value(veha, source, options),
        }];
        if tokens
            .get(*index)
            .is_some_and(|token| token.is_selmaho(Selmaho::Viha))
        {
            entries.push(TreeEntry {
                label: Some("viha"),
                value: generated_token_tree_value(tokens[*index], source, options),
            });
            *index += 1;
        }
        return Some(TreeValue::Node(TreeNode {
            constructor: "VehaSpaceIntervalTense",
            entries,
        }));
    }
    if tokens
        .get(*index)
        .is_some_and(|token| token.is_selmaho(Selmaho::Viha))
    {
        let viha = tokens[*index];
        *index += 1;
        return Some(TreeValue::Node(TreeNode {
            constructor: "VihaSpaceIntervalTense",
            entries: vec![TreeEntry {
                label: Some("viha"),
                value: generated_token_tree_value(viha, source, options),
            }],
        }));
    }
    None
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_faha_interval_direction_tense_value(
    tokens: &[&Token],
    index: &mut usize,
    source: &str,
    options: TreeRenderOptions,
) -> Option<TreeValue> {
    let mut next = *index;
    let faha = tokens
        .get(next)
        .filter(|token| token.is_selmaho(Selmaho::Faha))?;
    next += 1;
    let mut entries = vec![TreeEntry {
        label: Some("faha"),
        value: generated_token_tree_value(faha, source, options),
    }];
    if tokens
        .get(next)
        .is_some_and(|token| token.is_cmavo(Cmavo::Nai))
    {
        entries.push(TreeEntry {
            label: Some("nai"),
            value: generated_token_tree_value(tokens[next], source, options),
        });
        next += 1;
    }
    *index = next;
    Some(TreeValue::Node(TreeNode {
        constructor: "FahaIntervalDirectionTense",
        entries,
    }))
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_space_interval_properties_tense_value(
    tokens: &[&Token],
    index: &mut usize,
    source: &str,
    options: TreeRenderOptions,
) -> Option<TreeValue> {
    let first =
        legacy_as_generated_fehe_interval_property_tense_value(tokens, index, source, options)?;
    let mut additional = Vec::new();
    while let Some(property) =
        legacy_as_generated_fehe_interval_property_tense_value(tokens, index, source, options)
    {
        additional.push(property);
    }
    let mut entries = vec![TreeEntry {
        label: Some("first"),
        value: first,
    }];
    if let Some(entry) = labelled_tree_collection_entry_from_values("additional", additional) {
        entries.push(entry);
    }
    Some(TreeValue::Node(TreeNode {
        constructor: "SpaceIntervalPropertiesTense",
        entries,
    }))
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_fehe_interval_property_tense_value(
    tokens: &[&Token],
    index: &mut usize,
    source: &str,
    options: TreeRenderOptions,
) -> Option<TreeValue> {
    let mut next = *index;
    let fehe = tokens
        .get(next)
        .filter(|token| token.is_cmavo(Cmavo::Fehe))?;
    next += 1;
    let property =
        legacy_as_generated_interval_property_tense_value(tokens, &mut next, source, options)?;
    *index = next;
    Some(TreeValue::Node(TreeNode {
        constructor: "FeheIntervalPropertyTense",
        entries: vec![
            TreeEntry {
                label: Some("fehe"),
                value: generated_token_tree_value(fehe, source, options),
            },
            TreeEntry {
                label: Some("property"),
                value: property,
            },
        ],
    }))
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_interval_property_tense_value(
    tokens: &[&Token],
    index: &mut usize,
    source: &str,
    options: TreeRenderOptions,
) -> Option<TreeValue> {
    let mut next = *index;
    if tokens
        .get(next)
        .is_some_and(|token| token.is_selmaho(Selmaho::Zaho))
    {
        let zaho = tokens[next];
        next += 1;
        let mut entries = vec![TreeEntry {
            label: Some("zaho"),
            value: generated_token_tree_value(zaho, source, options),
        }];
        if tokens
            .get(next)
            .is_some_and(|token| token.is_cmavo(Cmavo::Nai))
        {
            entries.push(TreeEntry {
                label: Some("nai"),
                value: generated_token_tree_value(tokens[next], source, options),
            });
            next += 1;
        }
        *index = next;
        return Some(TreeValue::Node(TreeNode {
            constructor: "ZahoIntervalPropertyTense",
            entries,
        }));
    }
    if tokens
        .get(next)
        .is_some_and(|token| token.is_selmaho(Selmaho::Tahe))
    {
        let tahe = tokens[next];
        next += 1;
        let mut entries = vec![TreeEntry {
            label: Some("tahe"),
            value: generated_token_tree_value(tahe, source, options),
        }];
        if tokens
            .get(next)
            .is_some_and(|token| token.is_cmavo(Cmavo::Nai))
        {
            entries.push(TreeEntry {
                label: Some("nai"),
                value: generated_token_tree_value(tokens[next], source, options),
            });
            next += 1;
        }
        *index = next;
        return Some(TreeValue::Node(TreeNode {
            constructor: "TaheIntervalPropertyTense",
            entries,
        }));
    }
    if tokens
        .get(next)
        .is_some_and(|token| token.is_selmaho(Selmaho::Pa))
    {
        let start = next;
        next += 1;
        while tokens
            .get(next)
            .is_some_and(|token| legacy_token_is_number_or_letter_word(token))
        {
            next += 1;
        }
        let roi = tokens
            .get(next)
            .filter(|token| token.is_selmaho(Selmaho::Roi))?;
        next += 1;
        let mut entries = vec![
            TreeEntry {
                label: Some("number"),
                value: TreeValue::Collection(
                    tokens[start..next - 1]
                        .iter()
                        .map(|token| generated_token_tree_value(token, source, options))
                        .collect(),
                ),
            },
            TreeEntry {
                label: Some("roi"),
                value: generated_token_tree_value(roi, source, options),
            },
        ];
        if tokens
            .get(next)
            .is_some_and(|token| token.is_cmavo(Cmavo::Nai))
        {
            entries.push(TreeEntry {
                label: Some("nai"),
                value: generated_token_tree_value(tokens[next], source, options),
            });
            next += 1;
        }
        *index = next;
        return Some(TreeValue::Node(TreeNode {
            constructor: "NumberedIntervalPropertyTense",
            entries,
        }));
    }
    None
}

#[requires(true)]
#[ensures(true)]
fn legacy_next_tag_connective_start(tokens: &[&Token], start: usize) -> Option<usize> {
    (start..tokens.len()).find(|index| {
        let mut probe = *index;
        legacy_as_generated_tag_connective_tree_value(
            tokens,
            &mut probe,
            "",
            TreeRenderOptions::default(),
        )
        .is_some()
    })
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|_| *index > old(*index)))]
fn legacy_as_generated_tag_connective_tree_value(
    tokens: &[&Token],
    index: &mut usize,
    source: &str,
    options: TreeRenderOptions,
) -> Option<TreeValue> {
    let start = *index;
    if let Some(value) =
        legacy_as_generated_jek_tag_connective_tree_value(tokens, index, source, options)
    {
        return Some(value);
    }
    *index = start;
    if let Some(value) =
        legacy_as_generated_joik_tag_connective_tree_value(tokens, index, source, options)
    {
        return Some(value);
    }
    *index = start;
    None
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|_| *index > old(*index)))]
fn legacy_as_generated_jek_tag_connective_tree_value(
    tokens: &[&Token],
    index: &mut usize,
    source: &str,
    options: TreeRenderOptions,
) -> Option<TreeValue> {
    let mut next = *index;
    let na = tokens
        .get(next)
        .filter(|token| token.is_selmaho(Selmaho::Na));
    if na.is_some() {
        next += 1;
    }
    let se = tokens
        .get(next)
        .filter(|token| token.is_selmaho(Selmaho::Se));
    if se.is_some() {
        next += 1;
    }
    let ja = tokens
        .get(next)
        .filter(|token| token.is_selmaho(Selmaho::Ja))?;
    next += 1;
    let nai = tokens.get(next).filter(|token| token.is_cmavo(Cmavo::Nai));
    if nai.is_some() {
        next += 1;
    }
    *index = next;

    let mut entries = Vec::new();
    if let Some(na) = na {
        entries.push(TreeEntry {
            label: Some("na"),
            value: generated_token_tree_value(na, source, options),
        });
    }
    if let Some(se) = se {
        entries.push(TreeEntry {
            label: Some("se"),
            value: generated_token_tree_value(se, source, options),
        });
    }
    entries.push(TreeEntry {
        label: None,
        value: generated_token_tree_value(ja, source, options),
    });
    if let Some(nai) = nai {
        entries.push(TreeEntry {
            label: Some("nai"),
            value: generated_token_tree_value(nai, source, options),
        });
    }
    Some(TreeValue::Node(TreeNode {
        constructor: "Selbri",
        entries,
    }))
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|_| *index > old(*index)))]
fn legacy_as_generated_joik_tag_connective_tree_value(
    tokens: &[&Token],
    index: &mut usize,
    source: &str,
    options: TreeRenderOptions,
) -> Option<TreeValue> {
    let start = *index;
    if let Some(value) =
        legacy_as_generated_joi_tag_connective_tree_value(tokens, index, source, options)
    {
        return Some(value);
    }
    *index = start;
    if let Some(value) = legacy_as_generated_closed_interval_tag_connective_tree_value(
        tokens, index, source, options,
    ) {
        return Some(value);
    }
    *index = start;
    legacy_as_generated_simple_interval_tag_connective_tree_value(tokens, index, source, options)
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|_| *index > old(*index)))]
fn legacy_as_generated_joi_tag_connective_tree_value(
    tokens: &[&Token],
    index: &mut usize,
    source: &str,
    options: TreeRenderOptions,
) -> Option<TreeValue> {
    let mut next = *index;
    let se = tokens
        .get(next)
        .filter(|token| token.is_selmaho(Selmaho::Se));
    if se.is_some() {
        next += 1;
    }
    let joi = tokens
        .get(next)
        .filter(|token| token.is_selmaho(Selmaho::Joi))?;
    next += 1;
    let nai = tokens.get(next).filter(|token| token.is_cmavo(Cmavo::Nai));
    if nai.is_some() {
        next += 1;
    }
    *index = next;
    Some(legacy_as_generated_tag_connective_node(
        "NonLogical",
        se,
        vec![joi],
        nai,
        source,
        options,
    ))
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|_| *index > old(*index)))]
fn legacy_as_generated_simple_interval_tag_connective_tree_value(
    tokens: &[&Token],
    index: &mut usize,
    source: &str,
    options: TreeRenderOptions,
) -> Option<TreeValue> {
    let mut next = *index;
    let se = tokens
        .get(next)
        .filter(|token| token.is_selmaho(Selmaho::Se));
    if se.is_some() {
        next += 1;
    }
    let bihi = tokens
        .get(next)
        .filter(|token| token.is_selmaho(Selmaho::Bihi))?;
    next += 1;
    let nai = tokens.get(next).filter(|token| token.is_cmavo(Cmavo::Nai));
    if nai.is_some() {
        next += 1;
    }
    *index = next;
    Some(legacy_as_generated_tag_connective_node(
        "Interval",
        se,
        vec![bihi],
        nai,
        source,
        options,
    ))
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|_| *index > old(*index)))]
fn legacy_as_generated_closed_interval_tag_connective_tree_value(
    tokens: &[&Token],
    index: &mut usize,
    source: &str,
    options: TreeRenderOptions,
) -> Option<TreeValue> {
    let mut next = *index;
    let left = tokens
        .get(next)
        .filter(|token| token.is_selmaho(Selmaho::Gaho))?;
    next += 1;
    let se = tokens
        .get(next)
        .filter(|token| token.is_selmaho(Selmaho::Se));
    if se.is_some() {
        next += 1;
    }
    let bihi = tokens
        .get(next)
        .filter(|token| token.is_selmaho(Selmaho::Bihi))?;
    next += 1;
    let nai = tokens.get(next).filter(|token| token.is_cmavo(Cmavo::Nai));
    if nai.is_some() {
        next += 1;
    }
    let right = tokens
        .get(next)
        .filter(|token| token.is_selmaho(Selmaho::Gaho))?;
    next += 1;
    *index = next;
    Some(legacy_as_generated_tag_connective_node(
        "Interval",
        se,
        vec![left, bihi, right],
        nai,
        source,
        options,
    ))
}

#[requires(!constructor.is_empty())]
#[ensures(true)]
fn legacy_as_generated_tag_connective_node(
    constructor: &'static str,
    se: Option<&&Token>,
    cmavo: Vec<&&Token>,
    nai: Option<&&Token>,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    let mut entries = Vec::new();
    if let Some(se) = se {
        entries.push(TreeEntry {
            label: Some("se"),
            value: generated_token_tree_value(se, source, options),
        });
    }
    entries.extend(cmavo.into_iter().map(|token| TreeEntry {
        label: None,
        value: generated_token_tree_value(token, source, options),
    }));
    if let Some(nai) = nai {
        entries.push(TreeEntry {
            label: Some("nai"),
            value: generated_token_tree_value(nai, source, options),
        });
    }
    TreeValue::Node(TreeNode {
        constructor,
        entries,
    })
}

#[requires(true)]
#[ensures(true)]
fn legacy_composite_tense_modal_part_tokens(
    parts: &[jbotci_syntax::ast::CompositeTenseModalPartSyntax],
) -> Option<Vec<&Token>> {
    parts
        .iter()
        .map(|part| match part.as_data() {
            bityzba::data!(jbotci_syntax::ast::CompositeTenseModalPartSyntax::Cmavo(
                token
            )) => Some(token),
            _ => None,
        })
        .collect()
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_flat_tag_atom_tree_value(
    token: &Token,
    source: &str,
    options: TreeRenderOptions,
) -> Option<TreeValue> {
    if token.is_selmaho(Selmaho::Fa) {
        return Some(TreeValue::Node(TreeNode {
            constructor: "FaFlatTagAtom",
            entries: vec![TreeEntry {
                label: Some("fa_flat_tag_atom"),
                value: TreeValue::Node(TreeNode {
                    constructor: "FaFlatTagAtom",
                    entries: vec![TreeEntry {
                        label: Some("fa"),
                        value: generated_token_tree_value(token, source, options),
                    }],
                }),
            }],
        }));
    }
    legacy_as_generated_single_composite_tense_token_tree_value(token, source, options).map(
        |composite| {
            TreeValue::Node(TreeNode {
                constructor: "CompositeFlatTagAtom",
                entries: vec![TreeEntry {
                    label: Some("composite_flat_tag_atom"),
                    value: TreeValue::Node(TreeNode {
                        constructor: "CompositeFlatTagAtom",
                        entries: vec![TreeEntry {
                            label: Some("composite"),
                            value: composite,
                        }],
                    }),
                }],
            })
        },
    )
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_composite_flat_tag_atom_tree_value(
    tokens: &[&Token],
    source: &str,
    options: TreeRenderOptions,
) -> Option<TreeValue> {
    let composite = legacy_as_generated_connected_tense_modal_tree_value(tokens, source, options)
        .or_else(|| legacy_as_generated_time_tense_sequence_tree_value(tokens, source, options))
        .or_else(|| {
            legacy_as_generated_space_then_time_tense_sequence_tree_value(tokens, source, options)
        })
        .or_else(|| match tokens {
            [token] => {
                legacy_as_generated_single_composite_tense_token_tree_value(token, source, options)
            }
            _ => None,
        })?;
    Some(TreeValue::Node(TreeNode {
        constructor: "CompositeFlatTagAtom",
        entries: vec![TreeEntry {
            label: Some("composite_flat_tag_atom"),
            value: TreeValue::Node(TreeNode {
                constructor: "CompositeFlatTagAtom",
                entries: vec![TreeEntry {
                    label: Some("composite"),
                    value: composite,
                }],
            }),
        }],
    }))
}

#[requires(true)]
#[ensures(true)]
fn legacy_as_generated_single_composite_tense_token_tree_value(
    token: &Token,
    source: &str,
    options: TreeRenderOptions,
) -> Option<TreeValue> {
    if let Some(value) =
        legacy_as_generated_time_tense_sequence_tree_value(&[token], source, options)
    {
        return Some(value);
    }
    if token.is_selmaho(Selmaho::Pu) {
        return Some(TreeValue::Node(TreeNode {
            constructor: "TimeSpaceCahaKiTense",
            entries: vec![TreeEntry {
                label: Some("tense"),
                value: TreeValue::Node(TreeNode {
                    constructor: "TimeThenSpaceCahaTense",
                    entries: vec![TreeEntry {
                        label: Some("time"),
                        value: TreeValue::Node(TreeNode {
                            constructor: "TimeTenseWithOffset",
                            entries: vec![TreeEntry {
                                label: Some("offsets"),
                                value: TreeValue::Collection(vec![TreeValue::Node(TreeNode {
                                    constructor: "PuTimeOffsetTense",
                                    entries: vec![TreeEntry {
                                        label: Some("pu"),
                                        value: generated_token_tree_value(token, source, options),
                                    }],
                                })]),
                            }],
                        }),
                    }],
                }),
            }],
        }));
    }
    if token.is_selmaho(Selmaho::Caha) {
        return Some(TreeValue::Node(TreeNode {
            constructor: "TimeSpaceCahaKiTense",
            entries: vec![TreeEntry {
                label: Some("tense"),
                value: TreeValue::Node(TreeNode {
                    constructor: "CahaTense",
                    entries: vec![TreeEntry {
                        label: Some("caha"),
                        value: generated_token_tree_value(token, source, options),
                    }],
                }),
            }],
        }));
    }
    if token.is_selmaho(Selmaho::Cuhe) {
        return Some(TreeValue::Node(TreeNode {
            constructor: "CuheTense",
            entries: vec![TreeEntry {
                label: Some("cuhe"),
                value: generated_token_tree_value(token, source, options),
            }],
        }));
    }
    if token.is_selmaho(Selmaho::Va) {
        return Some(TreeValue::Node(TreeNode {
            constructor: "TimeSpaceCahaKiTense",
            entries: vec![TreeEntry {
                label: Some("tense"),
                value: TreeValue::Node(TreeNode {
                    constructor: "SpaceThenTimeCahaTense",
                    entries: vec![TreeEntry {
                        label: Some("space"),
                        value: TreeValue::Node(TreeNode {
                            constructor: "SpaceTenseWithVa",
                            entries: vec![TreeEntry {
                                label: Some("va"),
                                value: TreeValue::Node(TreeNode {
                                    constructor: "VaSpaceDistanceTense",
                                    entries: vec![TreeEntry {
                                        label: Some("va"),
                                        value: generated_token_tree_value(token, source, options),
                                    }],
                                }),
                            }],
                        }),
                    }],
                }),
            }],
        }));
    }
    if token.is_selmaho(Selmaho::Faha) {
        return Some(TreeValue::Node(TreeNode {
            constructor: "TimeSpaceCahaKiTense",
            entries: vec![TreeEntry {
                label: Some("tense"),
                value: TreeValue::Node(TreeNode {
                    constructor: "SpaceThenTimeCahaTense",
                    entries: vec![TreeEntry {
                        label: Some("space"),
                        value: TreeValue::Node(TreeNode {
                            constructor: "SpaceTenseWithOffset",
                            entries: vec![TreeEntry {
                                label: Some("offsets"),
                                value: TreeValue::Collection(vec![TreeValue::Node(TreeNode {
                                    constructor: "FahaSpaceOffsetTense",
                                    entries: vec![TreeEntry {
                                        label: Some("faha"),
                                        value: generated_token_tree_value(token, source, options),
                                    }],
                                })]),
                            }],
                        }),
                    }],
                }),
            }],
        }));
    }
    None
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
            GeneratedSyntaxAtomRef::Token(word) => {
                with_indicators_tree_value(word.as_indicators(), source, options)
            }
        }
    }

    fn atom_end_position<'tree>(atom: Self::Atom<'tree>) -> Option<RenderedPosition> {
        match atom {
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
            GeneratedSyntaxNodeRef::TextSyntaxRegularText(text) => {
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
    let GeneratedTextSyntax::RegularText { regular_text } = text else {
        return None;
    };
    let generated_model::RegularTextSyntax {
        leading_nai,
        leading_cmevla,
        leading_indicators,
        leading_free_modifiers,
        leading_connective,
        leading_i_statements,
        paragraphs,
    } = regular_text;
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
            .map(|indicator| required_generated_syntax_subtree_value(indicator, source, options))
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
        "Linkargs",
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
        generated_model::BridiStatementContinuationSyntax::BoBridiStatementContinuation {
            bo_bridi_statement_continuation,
        } => {
            let continuation = bo_bridi_statement_continuation;
            let mut entries = vec![TreeEntry {
                label: Some("connective"),
                value: generated_statement_connective_tree_value(
                    &continuation.connective,
                    source,
                    options,
                ),
            }];
            if let Some(tense_modal) = &continuation.tense_modal {
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
                value: generated_with_free_modifiers_token_tree_value(
                    &continuation.bo,
                    source,
                    options,
                ),
            });
            entries.push(TreeEntry {
                label: Some("trailing_subbridi"),
                value: required_generated_syntax_subtree_value(
                    continuation.trailing_subbridi.as_ref(),
                    source,
                    options,
                ),
            });
            TreeValue::Node(TreeNode {
                constructor: "BridiStatementContinuation",
                entries,
            })
        }
        generated_model::BridiStatementContinuationSyntax::KeBridiStatementContinuation {
            ke_bridi_statement_continuation,
        } => {
            let continuation = ke_bridi_statement_continuation;
            let mut entries = vec![TreeEntry {
                label: Some("connective"),
                value: generated_statement_connective_tree_value(
                    &continuation.connective,
                    source,
                    options,
                ),
            }];
            if let Some(tense_modal) = &continuation.tense_modal {
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
                value: generated_with_free_modifiers_token_tree_value(
                    &continuation.ke,
                    source,
                    options,
                ),
            }];
            if let Some(kehe) = &continuation.kehe {
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
                    continuation.trailing_subbridi.as_ref(),
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
        GeneratedIStatementConnectionTailSyntax::SimpleIConnectiveStatementTail {
            simple_i_connective_statement_tail,
        } => {
            let tail = simple_i_connective_statement_tail;
            GeneratedStatementConnectionPart {
                i: generated_token_tree_value(&tail.i, source, options),
                connective: generated_statement_connective_tree_value(
                    &tail.connective,
                    source,
                    options,
                ),
                connective_has_bo: generated_connective_has_bo(&tail.connective),
                trailing_statement: required_generated_syntax_subtree_value(
                    tail.trailing_statement.as_ref(),
                    source,
                    options,
                ),
            }
        }
        GeneratedIStatementConnectionTailSyntax::ChainedIConnectiveStatementTail {
            chained_i_connective_statement_tail,
        } => {
            let tail = chained_i_connective_statement_tail;
            let first_pending = tail
                .pending
                .first()
                .expect("chained I statement tails parse pending with many1");
            let mut extra = Vec::new();
            for pending_connective in tail.pending.iter().skip(1) {
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
            extra.push(generated_token_tree_value(&tail.i, source, options));
            extra.push(generated_statement_connective_tree_value(
                &tail.connective,
                source,
                options,
            ));
            GeneratedStatementConnectionPart {
                i: generated_token_tree_value(&first_pending.i, source, options),
                connective: generated_connective_tree_value_with_extra_words(
                    &first_pending.connective,
                    extra,
                    source,
                    options,
                ),
                connective_has_bo: generated_connective_has_bo(&tail.connective),
                trailing_statement: required_generated_syntax_subtree_value(
                    tail.trailing_statement.as_ref(),
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
        let GeneratedSyntaxAtomRef::Token(token) = atom;
        self.values
            .push(generated_token_tree_value(token, self.source, self.options));
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
