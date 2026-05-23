//! Compact JSON DOM builders over generated tree traversal.

#[allow(unused_imports)]
use bityzba::{ensures, invariant, requires};
use jbotci_morphology::{TreeNode as MorphologyTreeNode, Word, WordLike};
use jbotci_source::SourceSpan;
use jbotci_syntax::WithIndicators;
use jbotci_syntax::ast::{
    AtomRef as SyntaxAtomRef, NodeRef as SyntaxNodeRef, TextSyntax, TreeNode as SyntaxTreeNode,
};
use jbotci_tree::{FieldRef, TreeVisitor};
use serde_json::{Map, Value};

#[derive(Debug, Clone, PartialEq)]
#[invariant(true)]
struct JsonEntry {
    label: Option<&'static str>,
    value: Value,
}

#[derive(Debug, Clone, PartialEq)]
#[invariant(true)]
enum JsonFrame<N> {
    Node {
        node: N,
        entries: Vec<JsonEntry>,
    },
    Field {
        name: Option<&'static str>,
        values: Vec<Value>,
        nested_entries: Vec<JsonEntry>,
    },
    Sequence {
        items: Vec<Value>,
    },
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn morphology_json_value(words: &[WordLike]) -> Value {
    Value::Array(words.iter().map(morphology_word_like_value).collect())
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn syntax_json_value(tree: &TextSyntax) -> Value {
    let mut builder = SyntaxJsonBuilder::default();
    tree.visit_in_order(&mut builder);
    builder.finish()
}

#[requires(true)]
#[ensures(true)]
fn morphology_word_like_value(word_like: &WordLike) -> Value {
    let mut builder = MorphologyJsonBuilder::default();
    word_like.visit_in_order(&mut builder);
    builder.finish()
}

#[requires(true)]
#[ensures(true)]
fn morphology_word_value(word: &Word) -> Value {
    let mut builder = MorphologyJsonBuilder::default();
    MorphologyTreeNode::visit_in_order(word, &mut builder);
    builder.finish()
}

#[derive(Debug, Default)]
#[invariant(true)]
struct MorphologyJsonBuilder {
    stack: Vec<JsonFrame<MorphologyNodeInfo>>,
    root: Option<Value>,
}

impl MorphologyJsonBuilder {
    #[requires(true)]
    #[ensures(true)]
    fn finish(self) -> Value {
        self.root.expect("morphology JSON walk produced a root")
    }

    #[requires(true)]
    #[ensures(true)]
    fn push_value(&mut self, value: Value) {
        push_value(&mut self.stack, &mut self.root, value);
    }

    #[requires(true)]
    #[ensures(true)]
    fn push_entry(&mut self, entry: JsonEntry) {
        push_entry(&mut self.stack, entry);
    }
}

impl<'tree> TreeVisitor<'tree> for MorphologyJsonBuilder {
    type Node = jbotci_morphology::NodeRef<'tree>;
    type Atom = jbotci_morphology::AtomRef<'tree>;

    #[requires(true)]
    #[ensures(true)]
    fn enter_node(&mut self, node: Self::Node) {
        self.stack.push(JsonFrame::Node {
            node: MorphologyNodeInfo {
                constructor: node.constructor_name(),
                variant: node.is_variant(),
            },
            entries: Vec::new(),
        });
    }

    #[requires(true)]
    #[ensures(true)]
    fn exit_node(&mut self, _node: Self::Node) {
        let Some(JsonFrame::Node { node, entries }) = self.stack.pop() else {
            panic!("morphology JSON walker exited a node without entering it");
        };
        let value = node_value(node.constructor, node.variant, entries);
        self.push_value(value);
    }

    #[requires(true)]
    #[ensures(true)]
    fn enter_field(&mut self, field: FieldRef) {
        self.stack.push(JsonFrame::Field {
            name: field.name,
            values: Vec::new(),
            nested_entries: Vec::new(),
        });
    }

    #[requires(true)]
    #[ensures(true)]
    fn exit_field(&mut self, _field: FieldRef) {
        let Some(JsonFrame::Field {
            name,
            values,
            nested_entries,
        }) = self.stack.pop()
        else {
            panic!("morphology JSON walker exited a field without entering it");
        };
        let Some(value) = field_value(values, nested_entries) else {
            return;
        };
        if let Some(label) = name {
            self.push_entry(JsonEntry {
                label: Some(label),
                value,
            });
        } else {
            self.push_value(value);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn enter_sequence(&mut self) {
        self.stack.push(JsonFrame::Sequence { items: Vec::new() });
    }

    #[requires(true)]
    #[ensures(true)]
    fn exit_sequence(&mut self) {
        let Some(JsonFrame::Sequence { items }) = self.stack.pop() else {
            panic!("morphology JSON walker exited a sequence without entering it");
        };
        if !items.is_empty() {
            self.push_value(Value::Array(items));
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_atom(&mut self, atom: Self::Atom) {
        self.push_value(match atom {
            jbotci_morphology::AtomRef::Phonemes(phonemes) => {
                Value::String(phonemes.as_str().to_owned())
            }
            jbotci_morphology::AtomRef::String(text) => Value::String(text.clone()),
            jbotci_morphology::AtomRef::SourceSpan(span) => span_value(span),
        });
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
struct MorphologyNodeInfo {
    constructor: &'static str,
    variant: bool,
}

#[derive(Debug, Default)]
#[invariant(true)]
struct SyntaxJsonBuilder {
    stack: Vec<JsonFrame<SyntaxNodeInfo>>,
    root: Option<Value>,
}

impl SyntaxJsonBuilder {
    #[requires(true)]
    #[ensures(true)]
    fn finish(self) -> Value {
        self.root.expect("syntax JSON walk produced a root")
    }

    #[requires(true)]
    #[ensures(true)]
    fn push_value(&mut self, value: Value) {
        push_value(&mut self.stack, &mut self.root, value);
    }

    #[requires(true)]
    #[ensures(true)]
    fn push_entry(&mut self, entry: JsonEntry) {
        push_entry(&mut self.stack, entry);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
struct SyntaxNodeInfo {
    constructor: &'static str,
    variant: bool,
}

impl<'tree> TreeVisitor<'tree> for SyntaxJsonBuilder {
    type Node = SyntaxNodeRef<'tree>;
    type Atom = SyntaxAtomRef<'tree>;

    #[requires(true)]
    #[ensures(true)]
    fn enter_node(&mut self, node: Self::Node) {
        self.stack.push(JsonFrame::Node {
            node: SyntaxNodeInfo {
                constructor: syntax_constructor_name(node.constructor_name()),
                variant: node.is_variant(),
            },
            entries: Vec::new(),
        });
    }

    #[requires(true)]
    #[ensures(true)]
    fn exit_node(&mut self, _node: Self::Node) {
        let Some(JsonFrame::Node { node, entries }) = self.stack.pop() else {
            panic!("syntax JSON walker exited a node without entering it");
        };
        let value = node_value(node.constructor, node.variant, entries);
        self.push_value(value);
    }

    #[requires(true)]
    #[ensures(true)]
    fn enter_field(&mut self, field: FieldRef) {
        self.stack.push(JsonFrame::Field {
            name: field.name,
            values: Vec::new(),
            nested_entries: Vec::new(),
        });
    }

    #[requires(true)]
    #[ensures(true)]
    fn exit_field(&mut self, _field: FieldRef) {
        let Some(JsonFrame::Field {
            name,
            values,
            nested_entries,
        }) = self.stack.pop()
        else {
            panic!("syntax JSON walker exited a field without entering it");
        };
        let Some(value) = field_value(values, nested_entries) else {
            return;
        };
        if let Some(label) = name {
            self.push_entry(JsonEntry {
                label: Some(label),
                value,
            });
        } else {
            self.push_value(value);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn enter_sequence(&mut self) {
        self.stack.push(JsonFrame::Sequence { items: Vec::new() });
    }

    #[requires(true)]
    #[ensures(true)]
    fn exit_sequence(&mut self) {
        let Some(JsonFrame::Sequence { items }) = self.stack.pop() else {
            panic!("syntax JSON walker exited a sequence without entering it");
        };
        if !items.is_empty() {
            self.push_value(Value::Array(items));
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_atom(&mut self, atom: Self::Atom) {
        self.push_value(match atom {
            SyntaxAtomRef::WithIndicatorsWordLike(word) => with_indicators_value(word),
            SyntaxAtomRef::Word(word) => morphology_word_value(word),
        });
    }
}

#[requires(true)]
#[ensures(true)]
fn push_value<N>(stack: &mut [JsonFrame<N>], root: &mut Option<Value>, value: Value) {
    match stack.last_mut() {
        Some(JsonFrame::Node { entries, .. }) => entries.push(JsonEntry { label: None, value }),
        Some(JsonFrame::Field { values, .. }) => values.push(value),
        Some(JsonFrame::Sequence { items }) => items.push(value),
        None => *root = Some(value),
    }
}

#[requires(true)]
#[ensures(true)]
fn push_entry<N>(stack: &mut [JsonFrame<N>], entry: JsonEntry) {
    match stack.last_mut() {
        Some(JsonFrame::Node { entries, .. }) => entries.push(entry),
        Some(JsonFrame::Field { nested_entries, .. }) => nested_entries.push(entry),
        Some(JsonFrame::Sequence { .. }) | None => {
            panic!("JSON walker produced a labelled field outside a node or field")
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn field_value(values: Vec<Value>, nested_entries: Vec<JsonEntry>) -> Option<Value> {
    if nested_entries.is_empty() {
        return values_to_value(values);
    }
    if nested_entries
        .iter()
        .all(|entry| entry.label == Some("free_modifiers"))
    {
        let mut items = values;
        for entry in nested_entries {
            match entry.value {
                Value::Array(values) => items.extend(values),
                value => items.push(value),
            }
        }
        return (!items.is_empty()).then_some(Value::Array(items));
    }
    let mut object = Map::new();
    if let Some(value) = values_to_value(values) {
        object.insert("value".to_owned(), value);
    }
    for entry in nested_entries {
        if let Some(label) = entry.label {
            object.insert(label.to_owned(), entry.value);
        }
    }
    (!object.is_empty()).then_some(Value::Object(object))
}

#[requires(true)]
#[ensures(true)]
fn values_to_value(values: Vec<Value>) -> Option<Value> {
    match values.len() {
        0 => None,
        1 => values.into_iter().next(),
        _ => Some(Value::Array(values)),
    }
}

#[requires(true)]
#[ensures(true)]
fn node_value(constructor: &'static str, variant: bool, entries: Vec<JsonEntry>) -> Value {
    if variant {
        constructor_value(constructor, variant_payload(constructor, entries))
    } else {
        Value::Object(struct_fields(constructor, entries))
    }
}

#[requires(true)]
#[ensures(true)]
fn variant_payload(constructor: &'static str, entries: Vec<JsonEntry>) -> Value {
    if entries.is_empty() {
        return Value::Object(Map::new());
    }
    if let Some(value) = compact_single_payload(constructor, &entries) {
        return value;
    }
    if entries.len() == 1 && entries[0].label.is_none() {
        return entries.into_iter().next().expect("length checked").value;
    }
    if entries.iter().all(|entry| entry.label.is_some()) {
        return Value::Object(entries_to_object(entries));
    }
    Value::Array(entries.into_iter().map(|entry| entry.value).collect())
}

#[requires(true)]
#[ensures(true)]
fn compact_single_payload(constructor: &str, entries: &[JsonEntry]) -> Option<Value> {
    let field = match constructor {
        "Bare" => "word",
        "GekSentence" => "gek_sentence",
        "Argument" => "argument",
        "BeiLink" => "bei_only_links",
        "RelativeClause" => "relative_clauses",
        "MathExpression" => "math_expression",
        "Relation" => "relation",
        "Descriptor" => "descriptor",
        "ConnectedDescriptor" => "connected_descriptor",
        "Base" => "word",
        "Abstraction" => "abstraction",
        "Compound" => "units",
        "Wrapped" => "relation",
        _ => return None,
    };
    if entries.len() == 1 && entries[0].label == Some(field) {
        return Some(entries[0].value.clone());
    }
    None
}

#[requires(true)]
#[ensures(true)]
fn struct_fields(constructor: &'static str, entries: Vec<JsonEntry>) -> Map<String, Value> {
    if constructor == "Word" {
        return word_fields(entries);
    }
    entries_to_object(entries)
}

#[requires(true)]
#[ensures(true)]
fn entries_to_object(entries: Vec<JsonEntry>) -> Map<String, Value> {
    let mut object: Map<String, Value> = entries
        .into_iter()
        .filter_map(|entry| entry.label.map(|label| (label.to_owned(), entry.value)))
        .collect();
    if let Some(leading_indicators) = object.remove("leading_indicators") {
        object.insert("leading_indicators".to_owned(), leading_indicators);
    }
    object
}

#[requires(true)]
#[ensures(true)]
fn word_fields(entries: Vec<JsonEntry>) -> Map<String, Value> {
    let mut fields = entries_to_object(entries);
    let mut ordered = Map::new();
    for label in ["span", "phonemes", "kind"] {
        if let Some(value) = fields.remove(label) {
            ordered.insert(label.to_owned(), value);
        }
    }
    ordered.extend(fields);
    ordered
}

#[requires(true)]
#[ensures(true)]
fn constructor_value(constructor: &str, payload: Value) -> Value {
    Value::Object([(constructor.to_owned(), payload)].into_iter().collect())
}

#[requires(true)]
#[ensures(true)]
fn with_indicators_value(word: &WithIndicators<WordLike>) -> Value {
    match word {
        WithIndicators::Bare(word_like) => {
            constructor_value("Bare", morphology_word_like_value(word_like))
        }
        WithIndicators::Emphasized { bahe, word_like } => constructor_value(
            "Emphasized",
            Value::Object(
                [
                    ("bahe".to_owned(), morphology_word_value(bahe)),
                    (
                        "word_like".to_owned(),
                        morphology_word_like_value(word_like),
                    ),
                ]
                .into_iter()
                .collect(),
            ),
        ),
        WithIndicators::WithIndicator {
            base,
            indicator,
            nai,
        } => {
            let mut payload = Map::new();
            payload.insert("base".to_owned(), with_indicators_value(base));
            payload.insert("indicator".to_owned(), morphology_word_value(indicator));
            if let Some(nai) = nai {
                payload.insert("nai".to_owned(), morphology_word_value(nai));
            }
            constructor_value("WithIndicator", Value::Object(payload))
        }
    }
}

#[requires(span.char_start <= span.char_end)]
#[ensures(true)]
fn span_value(span: &SourceSpan) -> Value {
    Value::Array(vec![span.char_start.into(), span.char_end.into()])
}

#[requires(true)]
#[ensures(!ret.ends_with("Syntax"))]
fn syntax_constructor_name(constructor: &'static str) -> &'static str {
    constructor.strip_suffix("Syntax").unwrap_or(constructor)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compact_json_value;
    #[allow(unused_imports)]
    use bityzba::{ensures, requires};
    use jbotci_morphology::segment_words_with_modifiers;
    use jbotci_syntax::parse_syntax_tree;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn morphology_tree_json_matches_existing_compact_shape() {
        for text in [
            "mi klama",
            "zo broda cu melbi",
            "zoi gy hello gy",
            "mi broda zei brode",
        ] {
            let words = segment_words_with_modifiers(text).expect("morphology");
            assert_eq!(
                morphology_json_value(&words),
                compact_json_value(&words).expect("serde compact JSON")
            );
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn syntax_tree_json_matches_existing_compact_shape() {
        for text in [
            "mi klama",
            ".ui mi klama",
            "mi klama to coi toi",
            "ba'e mi ui nai klama",
        ] {
            let words = segment_words_with_modifiers(text).expect("morphology");
            let parsed = parse_syntax_tree(&words).expect("syntax");
            assert_eq!(
                syntax_json_value(&parsed.parse_tree),
                compact_json_value(&parsed.parse_tree).expect("serde compact JSON")
            );
        }
    }
}
