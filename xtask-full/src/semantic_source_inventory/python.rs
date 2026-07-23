use anyhow::{Context, Result, bail};
use bityzba::{ensures, new, requires};
use tree_sitter::{Node, Parser};

use super::classification::PythonFileRule;
use super::model::{PythonCensusRecord, PythonOccurrence, PythonOccurrenceKind};
use super::source::{SourceMap, record_id, sha256_hex};

const STRING_TOKENS: &[&str] = &[
    "tersmu",
    "TanruLink",
    "tree+proj",
    "lojban-semantics-json-1",
];

#[requires(!path.is_empty() && !git_blob.is_empty())]
#[ensures(ret.as_ref().is_ok_and(|record| record.source.path == path))]
pub(crate) fn extract_python_file(
    path: &str,
    git_blob: &str,
    bytes: &[u8],
    classification: &PythonFileRule,
) -> Result<PythonCensusRecord> {
    if classification.path != path {
        bail!(
            "Python classification path `{}` does not match discovered path `{path}`",
            classification.path
        );
    }
    let source = std::str::from_utf8(bytes)
        .with_context(|| format!("pinned Python source `{path}` is not UTF-8"))?;
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_python::LANGUAGE.into())
        .context("loading the workspace-locked tree-sitter Python grammar")?;
    let tree = parser
        .parse(source, None)
        .with_context(|| format!("tree-sitter did not produce a syntax tree for `{path}`"))?;
    let root = tree.root_node();
    if root.kind() != "module" {
        bail!(
            "Python source `{path}` parsed to unsupported root kind `{}`",
            root.kind()
        );
    }
    reject_parse_errors(path, root)?;
    let source_map = SourceMap::new(path, source);
    let mut occurrences = Vec::new();
    let mut pending = vec![root];
    while let Some(node) = pending.pop() {
        if node.kind() == "identifier" {
            let text = source
                .get(node.byte_range())
                .with_context(|| format!("Python identifier in `{path}` is not on UTF-8 boundaries"))?;
            if text == "tersmu" {
                occurrences.push(new!(PythonOccurrence {
                    source: source_map.byte_range(node.start_byte(), node.end_byte()),
                    kind: PythonOccurrenceKind::Identifier,
                    token: text.to_owned(),
                }));
            }
        } else if node.kind() == "string" {
            append_string_occurrences(&source_map, source, node, &mut occurrences)?;
            continue;
        }
        let mut cursor = node.walk();
        let mut children = node.children(&mut cursor).collect::<Vec<_>>();
        children.reverse();
        pending.extend(children);
    }
    occurrences.sort_by(|left, right| {
        left.source
            .cmp(&right.source)
            .then_with(|| left.token.cmp(&right.token))
    });
    let whole_file = source_map.whole_file();
    let semantic_consumer = occurrences
        .iter()
        .any(|occurrence| occurrence.token == "tersmu");
    Ok(new!(PythonCensusRecord {
        id: record_id("python", &whole_file),
        source: whole_file,
        git_blob: git_blob.to_owned(),
        sha256: sha256_hex(bytes),
        coverage_class: classification.class,
        coverage_reason: classification.reason.clone(),
        semantic_consumer,
        occurrences,
    }))
}

#[requires(!path.is_empty())]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn reject_parse_errors(path: &str, root: Node<'_>) -> Result<()> {
    let mut pending = vec![root];
    while let Some(node) = pending.pop() {
        if node.is_error() || node.is_missing() {
            let position = node.start_position();
            bail!(
                "unsupported or invalid Python syntax in `{path}` at {}:{} (`{}`)",
                position.row + 1,
                position.column,
                node.kind()
            );
        }
        let mut cursor = node.walk();
        pending.extend(node.children(&mut cursor));
    }
    Ok(())
}

#[requires(node.kind() == "string")]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn append_string_occurrences(
    source_map: &SourceMap<'_>,
    source: &str,
    node: Node<'_>,
    occurrences: &mut Vec<PythonOccurrence>,
) -> Result<()> {
    let text = source
        .get(node.byte_range())
        .context("Python string node is not on UTF-8 boundaries")?;
    for token in STRING_TOKENS {
        let mut searched = 0;
        while let Some(relative) = text[searched..].find(token) {
            let relative = searched + relative;
            let end = relative + token.len();
            if token_boundary(text.as_bytes(), relative, end, token) {
                let start_byte = node.start_byte() + relative;
                let end_byte = start_byte + token.len();
                occurrences.push(new!(PythonOccurrence {
                    source: source_map.byte_range(start_byte, end_byte),
                    kind: PythonOccurrenceKind::StringLiteral,
                    token: (*token).to_owned(),
                }));
            }
            searched = end;
        }
    }
    Ok(())
}

#[requires(start <= end && end <= text.len() && !token.is_empty())]
#[ensures(true)]
fn token_boundary(text: &[u8], start: usize, end: usize, token: &str) -> bool {
    if !token.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'_') {
        return true;
    }
    let identifier_byte = |byte: u8| byte.is_ascii_alphanumeric() || byte == b'_';
    text.get(start.wrapping_sub(1))
        .is_none_or(|byte| !identifier_byte(*byte))
        && text.get(end).is_none_or(|byte| !identifier_byte(*byte))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn comments_do_not_count_as_semantic_occurrences() {
        let source = b"# tersmu\nvalue = 'ordinary'\n";
        let classification = new!(PythonFileRule {
            path: "unclassified.py".to_owned(),
            class: super::super::model::PythonCoverageClass::BindingTest,
            reason: "test classification".to_owned(),
        });
        let record = extract_python_file(
            "unclassified.py",
            "0123456789abcdef0123456789abcdef01234567",
            source,
            &classification,
        )
        .expect("valid Python parses");
        assert!(record.occurrences.is_empty());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn malformed_python_fails_closed() {
        let classification = new!(PythonFileRule {
            path: "unclassified.py".to_owned(),
            class: super::super::model::PythonCoverageClass::BindingTest,
            reason: "test classification".to_owned(),
        });
        let error = extract_python_file(
            "unclassified.py",
            "0123456789abcdef0123456789abcdef01234567",
            b"def broken(:\n",
            &classification,
        )
        .expect_err("malformed syntax must fail before classification");
        assert!(error.to_string().contains("invalid Python syntax"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn identifier_boundaries_are_exact() {
        assert!(token_boundary(b" tersmu ", 1, 7, "tersmu"));
        assert!(!token_boundary(b"xtersmu ", 1, 7, "tersmu"));
        assert!(!token_boundary(b" tersmux", 1, 7, "tersmu"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn string_tokens_are_syntax_aware_and_do_not_alone_define_consumers() {
        let classification = new!(PythonFileRule {
            path: "unclassified.py".to_owned(),
            class: super::super::model::PythonCoverageClass::BindingTest,
            reason: "test classification".to_owned(),
        });
        let record = extract_python_file(
            "unclassified.py",
            "0123456789abcdef0123456789abcdef01234567",
            b"FORMAT = 'tree+proj'\n# tersmu\n",
            &classification,
        )
        .expect("valid Python parses");
        assert!(!record.semantic_consumer);
        assert_eq!(record.occurrences.len(), 1);
        assert_eq!(record.occurrences[0].token, "tree+proj");
    }
}
