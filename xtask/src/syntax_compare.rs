#[allow(unused_imports)]
use bityzba::{data, ensures, requires};
use jbotci_syntax::{SyntaxNode, SyntaxValue, SyntaxValueData};

#[requires(true)]
#[ensures(!ret.is_empty())]
pub(crate) fn format_syntax_mismatch(expected: &SyntaxValue, actual: &SyntaxValue) -> String {
    let mut path = Vec::new();
    let detail = syntax_difference(expected, actual, &mut path)
        .unwrap_or_else(|| "syntax parse-tree differs".to_owned());
    format!(
        "syntax parse-tree mismatch at {}: {detail}",
        path_text(&path)
    )
}

#[requires(true)]
#[ensures(ret.is_some() || path.len() == old(path.len()))]
fn syntax_difference(
    expected: &SyntaxValue,
    actual: &SyntaxValue,
    path: &mut Vec<String>,
) -> Option<String> {
    match (expected.as_data(), actual.as_data()) {
        (data!(SyntaxValue::Null), data!(SyntaxValue::Null)) => None,
        (data!(SyntaxValue::Bool { value: left }), data!(SyntaxValue::Bool { value: right }))
            if left == right =>
        {
            None
        }
        (
            data!(SyntaxValue::Integer { value: left }),
            data!(SyntaxValue::Integer { value: right }),
        ) if left == right => None,
        (data!(SyntaxValue::Text { value: left }), data!(SyntaxValue::Text { value: right }))
            if left == right =>
        {
            None
        }
        (data!(SyntaxValue::Word { word: left }), data!(SyntaxValue::Word { word: right }))
            if jbotci_morphology::word_with_modifiers_syntax_eq(left, right) =>
        {
            None
        }
        (data!(SyntaxValue::Word { word: left }), data!(SyntaxValue::Word { word: right })) => {
            Some(format!("expected word `{left}`, got `{right}`"))
        }
        (data!(SyntaxValue::Json { value: left }), data!(SyntaxValue::Json { value: right }))
            if left == right =>
        {
            None
        }
        (data!(SyntaxValue::List { items: left }), data!(SyntaxValue::List { items: right })) => {
            compare_syntax_slices(left, right, path)
        }
        (data!(SyntaxValue::Node { node: left }), data!(SyntaxValue::Node { node: right })) => {
            if left.constructor != right.constructor {
                return Some(format!(
                    "expected constructor `{}`, got `{}`; expected preview [{}], actual preview [{}]",
                    left.constructor,
                    right.constructor,
                    syntax_value_preview(expected),
                    syntax_value_preview(actual)
                ));
            }
            if left.fields.len() != right.fields.len() {
                return Some(format!(
                    "expected {} field(s), got {}",
                    left.fields.len(),
                    right.fields.len()
                ));
            }
            for (index, (left_field, right_field)) in
                left.fields.iter().zip(right.fields.iter()).enumerate()
            {
                if left_field.name != right_field.name {
                    return Some(format!(
                        "expected field name {:?}, got {:?}",
                        left_field.name, right_field.name
                    ));
                }
                path.push(
                    left_field
                        .name
                        .clone()
                        .unwrap_or_else(|| format!("field[{index}]")),
                );
                let difference = syntax_difference(&left_field.value, &right_field.value, path);
                if difference.is_some() {
                    return difference;
                }
                path.pop();
            }
            None
        }
        _ => Some(format!(
            "expected {}, got {}",
            syntax_value_kind(expected),
            syntax_value_kind(actual)
        )),
    }
}

#[requires(true)]
#[ensures(ret.is_some() || path.len() == old(path.len()))]
fn compare_syntax_slices(
    expected: &[SyntaxValue],
    actual: &[SyntaxValue],
    path: &mut Vec<String>,
) -> Option<String> {
    if expected.len() != actual.len() {
        return Some(format!(
            "expected {} item(s), got {}; expected preview [{}], actual preview [{}]",
            expected.len(),
            actual.len(),
            syntax_values_preview(expected),
            syntax_values_preview(actual)
        ));
    }
    for (index, (left, right)) in expected.iter().zip(actual.iter()).enumerate() {
        path.push(format!("[{index}]"));
        let difference = syntax_difference(left, right, path);
        if difference.is_some() {
            return difference;
        }
        path.pop();
    }
    None
}

#[requires(true)]
#[ensures(ret.len() <= 240)]
fn syntax_values_preview(values: &[SyntaxValue]) -> String {
    const PREVIEW_LIMIT: usize = 240;
    let mut preview = values
        .iter()
        .take(3)
        .map(syntax_value_preview)
        .collect::<Vec<_>>()
        .join(", ");
    if values.len() > 3 {
        preview.push_str(", ...");
    }
    if preview.len() > PREVIEW_LIMIT {
        preview.truncate(PREVIEW_LIMIT);
    }
    preview
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn syntax_value_preview(value: &SyntaxValue) -> String {
    match value.as_data() {
        data!(SyntaxValue::Null) => "null".to_owned(),
        data!(SyntaxValue::Bool { value }) => value.to_string(),
        data!(SyntaxValue::Integer { value }) => value.to_string(),
        data!(SyntaxValue::Text { value }) => format!("{value:?}"),
        data!(SyntaxValue::Json { .. }) => "json".to_owned(),
        data!(SyntaxValue::Word { word }) => word.to_string(),
        data!(SyntaxValue::List { items }) => {
            format!("list({}: {})", items.len(), syntax_values_preview(items))
        }
        data!(SyntaxValue::Node { node }) => {
            let words = syntax_node_word_preview(node);
            if words.is_empty() {
                node.constructor.clone()
            } else {
                format!("{}({words})", node.constructor)
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn syntax_node_word_preview(node: &SyntaxNode) -> String {
    let mut words = Vec::new();
    collect_syntax_words_from_node(node, &mut words);
    words.truncate(5);
    words.join(" ")
}

#[requires(true)]
#[ensures(words.len() >= old(words.len()))]
fn collect_syntax_words_from_node(node: &SyntaxNode, words: &mut Vec<String>) {
    for field in &node.fields {
        collect_syntax_words(&field.value, words);
        if words.len() >= 5 {
            return;
        }
    }
}

#[requires(true)]
#[ensures(words.len() >= old(words.len()))]
fn collect_syntax_words(value: &SyntaxValue, words: &mut Vec<String>) {
    match value.as_data() {
        data!(SyntaxValue::Word { word }) => words.push(word.to_string()),
        data!(SyntaxValue::List { items }) => {
            for item in items {
                collect_syntax_words(item, words);
                if words.len() >= 5 {
                    return;
                }
            }
        }
        data!(SyntaxValue::Node { node }) => collect_syntax_words_from_node(node, words),
        _ => {}
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn syntax_value_kind(value: &SyntaxValue) -> &'static str {
    match value.as_data() {
        data!(SyntaxValue::Null) => "null",
        data!(SyntaxValue::Bool { .. }) => "bool",
        data!(SyntaxValue::Integer { .. }) => "integer",
        data!(SyntaxValue::Text { .. }) => "text",
        data!(SyntaxValue::List { .. }) => "list",
        data!(SyntaxValue::Node { .. }) => "node",
        data!(SyntaxValue::Word { .. }) => "word",
        data!(SyntaxValue::Json { .. }) => "json",
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn path_text(path: &[String]) -> String {
    if path.is_empty() {
        "<root>".to_owned()
    } else {
        path.join(".")
    }
}
