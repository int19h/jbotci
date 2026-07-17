use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command as ProcessCommand, Output};

use anyhow::{Context, Result, bail};
#[allow(unused_imports)]
use bityzba::{ensures, invariant, new, requires};
use clap::Args;
use serde::Deserialize;
use serde_json::json;

const DEFAULT_CATALOG: &str = "tools/semantic-model-doc-probes.toml";
const DEFAULT_OUTPUT: &str = ".jbotci-build/semantic-model-doc-audit";

#[invariant(true)]
#[derive(Debug, Args)]
pub(crate) struct SemanticDocAuditArgs {
    /// Source-linked catalog of normative semantic-model examples.
    #[arg(long, default_value = DEFAULT_CATALOG)]
    catalog: PathBuf,
    /// Release jbotci binary used for every probe.
    #[arg(long)]
    binary: PathBuf,
    /// Directory receiving JSON, tree, stderr, and manifest evidence.
    #[arg(long, default_value = DEFAULT_OUTPUT)]
    output: PathBuf,
}

#[invariant(!documents.is_empty() && documents.values().all(|examples| !examples.is_empty()))]
#[derive(Debug, Deserialize)]
struct ProbeCatalog {
    documents: BTreeMap<PathBuf, Vec<String>>,
}

#[invariant(*line > 0)]
#[invariant(!text.is_empty())]
#[derive(Debug)]
struct InlineCodeSpan {
    line: usize,
    text: String,
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
pub(crate) fn run(args: SemanticDocAuditArgs) -> Result<()> {
    if !args.binary.is_file() {
        bail!(
            "semantic-doc audit binary `{}` is not a file",
            args.binary.display()
        );
    }
    let catalog_text = fs::read_to_string(&args.catalog)
        .with_context(|| format!("reading probe catalog `{}`", args.catalog.display()))?;
    let catalog: ProbeCatalog = toml::from_str(&catalog_text)
        .with_context(|| format!("parsing probe catalog `{}`", args.catalog.display()))?;
    fs::create_dir_all(&args.output)
        .with_context(|| format!("creating audit output `{}`", args.output.display()))?;

    let mut records = Vec::new();
    let mut example_count = 0usize;
    let mut succeeded = 0usize;
    let mut failed = 0usize;
    for (document, examples) in &catalog.documents {
        let source = fs::read_to_string(document)
            .with_context(|| format!("reading semantic-model document `{}`", document.display()))?;
        let spans = inline_code_spans(&source);
        let mut catalog_examples = BTreeSet::new();
        for example in examples {
            if example.is_empty() {
                bail!("`{}` contains an empty probe example", document.display());
            }
            if !catalog_examples.insert(example.as_str()) {
                bail!(
                    "`{}` catalogs duplicate example `{example}`",
                    document.display()
                );
            }
            let Some(occurrence) = spans.iter().find(|span| span.text == *example) else {
                bail!(
                    "`{}` no longer contains cataloged inline example `{example}`",
                    document.display()
                );
            };
            example_count += 1;
            let stem = output_stem(example_count, document);
            let json_output = run_probe(&args.binary, "json", example)?;
            let tree_output = run_probe(&args.binary, "tree", example)?;
            write_probe_output(&args.output, &stem, "json", &json_output)?;
            write_probe_output(&args.output, &stem, "tree", &tree_output)?;
            if json_output.status.success() {
                serde_json::from_slice::<serde_json::Value>(&json_output.stdout)
                    .with_context(|| format!("probe `{example}` emitted invalid canonical JSON"))?;
                succeeded += 1;
            } else {
                failed += 1;
            }
            if tree_output.status.success() {
                if tree_output.stdout.is_empty() {
                    bail!("probe `{example}` emitted an empty structural tree");
                }
                succeeded += 1;
            } else {
                failed += 1;
            }
            records.push(json!({
                "source": document,
                "line": occurrence.line,
                "text": example,
                "json": format!("{stem}.json"),
                "jsonStderr": format!("{stem}.json.stderr"),
                "jsonStatus": json_output.status.code(),
                "tree": format!("{stem}.tree"),
                "treeStderr": format!("{stem}.tree.stderr"),
                "treeStatus": tree_output.status.code(),
            }));
        }
    }

    let manifest = json!({
        "catalog": args.catalog,
        "binary": args.binary,
        "documents": catalog.documents.len(),
        "examples": example_count,
        "probes": example_count * 2,
        "succeeded": succeeded,
        "failed": failed,
        "records": records,
    });
    let manifest_path = args.output.join("manifest.json");
    fs::write(
        &manifest_path,
        serde_json::to_vec_pretty(&manifest).context("serializing probe manifest")?,
    )
    .with_context(|| format!("writing probe manifest `{}`", manifest_path.display()))?;

    println!(
        "documents={} examples={} probes={} succeeded={} failed={} output={}",
        catalog.documents.len(),
        example_count,
        example_count * 2,
        succeeded,
        failed,
        args.output.display()
    );
    if failed > 0 {
        bail!("{failed} semantic-model document probe(s) failed");
    }
    Ok(())
}

#[requires(true)]
#[ensures(ret.iter().all(|span| span.line > 0 && !span.text.is_empty()))]
fn inline_code_spans(source: &str) -> Vec<InlineCodeSpan> {
    let mut spans = Vec::new();
    let mut fence = None;
    for (line_index, line) in source.lines().enumerate() {
        let trimmed = line.trim_start();
        let marker = if trimmed.starts_with("```") {
            Some(b'`')
        } else if trimmed.starts_with("~~~") {
            Some(b'~')
        } else {
            None
        };
        if let Some(marker) = marker {
            match fence {
                None => fence = Some(marker),
                Some(open) if open == marker => fence = None,
                Some(_) => {}
            }
            continue;
        }
        if fence.is_some() {
            continue;
        }
        spans.extend(inline_code_spans_on_line(line_index + 1, line));
    }
    spans
}

#[requires(line_number > 0)]
#[ensures(ret.iter().all(|span| span.line == line_number && !span.text.is_empty()))]
fn inline_code_spans_on_line(line_number: usize, line: &str) -> Vec<InlineCodeSpan> {
    let bytes = line.as_bytes();
    let mut spans = Vec::new();
    let mut cursor = 0usize;
    while cursor < bytes.len() {
        if bytes[cursor] != b'`' {
            cursor += 1;
            continue;
        }
        let delimiter_start = cursor;
        while cursor < bytes.len() && bytes[cursor] == b'`' {
            cursor += 1;
        }
        let delimiter_len = cursor - delimiter_start;
        let content_start = cursor;
        let Some(content_end) = matching_backtick_run(bytes, content_start, delimiter_len) else {
            break;
        };
        let text = line[content_start..content_end].trim();
        if !text.is_empty() {
            spans.push(new!(InlineCodeSpan {
                line: line_number,
                text: text.to_owned(),
            }));
        }
        cursor = content_end + delimiter_len;
    }
    spans
}

#[requires(start <= bytes.len())]
#[requires(delimiter_len > 0)]
#[ensures(ret.is_none_or(|offset| offset >= start && offset < bytes.len()))]
fn matching_backtick_run(bytes: &[u8], start: usize, delimiter_len: usize) -> Option<usize> {
    let mut cursor = start;
    while cursor < bytes.len() {
        if bytes[cursor] != b'`' {
            cursor += 1;
            continue;
        }
        let run_start = cursor;
        while cursor < bytes.len() && bytes[cursor] == b'`' {
            cursor += 1;
        }
        if cursor - run_start == delimiter_len {
            return Some(run_start);
        }
    }
    None
}

#[requires(index > 0)]
#[ensures(!ret.is_empty())]
fn output_stem(index: usize, document: &Path) -> String {
    let document_stem = document
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("document");
    let slug = document_stem
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' {
                character
            } else {
                '-'
            }
        })
        .collect::<String>();
    format!("{index:03}-{slug}")
}

#[requires(!format.is_empty())]
#[requires(!text.is_empty())]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn run_probe(binary: &Path, format: &str, text: &str) -> Result<Output> {
    ProcessCommand::new(binary)
        .arg("tersmu")
        .args(["--format", format])
        .arg(text)
        .output()
        .with_context(|| format!("running `{}` tersmu {format} probe", binary.display()))
}

#[requires(!stem.is_empty())]
#[requires(!format.is_empty())]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn write_probe_output(directory: &Path, stem: &str, format: &str, output: &Output) -> Result<()> {
    let stdout_path = directory.join(format!("{stem}.{format}"));
    let stderr_path = directory.join(format!("{stem}.{format}.stderr"));
    fs::write(&stdout_path, &output.stdout)
        .with_context(|| format!("writing probe stdout `{}`", stdout_path.display()))?;
    fs::write(&stderr_path, &output.stderr)
        .with_context(|| format!("writing probe stderr `{}`", stderr_path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn inline_scanner_skips_fences_and_tracks_source_lines() {
        let source = "before `mi klama`\n```json\n`not an example`\n```\nafter ``do cadzu``\n";
        let spans = inline_code_spans(source);
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].line, 1);
        assert_eq!(spans[0].text, "mi klama");
        assert_eq!(spans[1].line, 5);
        assert_eq!(spans[1].text, "do cadzu");
    }
}
