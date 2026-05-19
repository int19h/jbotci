use bityzba::requires;

#[requires(true)]
#[ensures(!ret.is_empty())]
pub(crate) fn format_syntax_mismatch(
    expected: &serde_json::Value,
    actual: &serde_json::Value,
) -> String {
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
    expected: &serde_json::Value,
    actual: &serde_json::Value,
    path: &mut Vec<String>,
) -> Option<String> {
    match (expected, actual) {
        (serde_json::Value::Object(left), serde_json::Value::Object(right)) => {
            if left.len() != right.len() {
                return Some(format!(
                    "expected {} object field(s), got {}",
                    left.len(),
                    right.len()
                ));
            }
            for (key, left_value) in left {
                let Some(right_value) = right.get(key) else {
                    return Some(format!("missing field `{key}`"));
                };
                path.push(key.clone());
                let difference = syntax_difference(left_value, right_value, path);
                if difference.is_some() {
                    return difference;
                }
                path.pop();
            }
            None
        }
        (serde_json::Value::Array(left), serde_json::Value::Array(right)) => {
            if left.len() != right.len() {
                return Some(format!(
                    "expected {} item(s), got {}",
                    left.len(),
                    right.len()
                ));
            }
            for (index, (left_value, right_value)) in left.iter().zip(right.iter()).enumerate() {
                path.push(format!("[{index}]"));
                let difference = syntax_difference(left_value, right_value, path);
                if difference.is_some() {
                    return difference;
                }
                path.pop();
            }
            None
        }
        _ if expected == actual => None,
        _ => Some(format!("expected {expected}, got {actual}")),
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn path_text(path: &[String]) -> String {
    if path.is_empty() {
        return "<root>".to_owned();
    }
    path.join(".")
}
