//! Dev-time per-document census for issue #761 step 4b/5.
//!
//! Emits one tab-separated record per corpus input so two runs can be diffed
//! document by document. This is a temporary measurement harness and is deleted
//! before the branch is committed; it is never an expectation oracle.

use std::collections::BTreeMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};

#[allow(unused_imports)]
use bityzba::{data, ensures, invariant, new, requires};
use jbotci_dialect::{DialectDefinition, parse_dialect_definition};
use jbotci_morphology::{
    MorphologyOptions, segment_words_with_modifiers_with_options_and_source_id,
};
use jbotci_semantics::completeness::corpus::CORPUS_DOCS;
use jbotci_semantics::{
    SemanticBuildOptions, SemanticGraph,
    build_generated_semantic_graph_with_dictionary_and_options, render_smusni,
};
use jbotci_source::SourceId;
use jbotci_syntax::{ParseOptions, parse_syntax_tree_generated_model_with_source_and_options};

/// One named source unit and the dialect required to build it.
#[invariant(!name.is_empty())]
#[invariant(!text.trim().is_empty())]
#[derive(Debug, Clone)]
struct CorpusInput {
    name: String,
    text: String,
    dialect: DialectDefinition,
}

/// Stable 64-bit content digest, used only to detect changed rendered bytes.
#[requires(true)]
#[ensures(true)]
fn digest(text: &str) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in text.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// Build a validated semantic graph through the production pipeline.
#[requires(!input.name.is_empty() && !input.text.trim().is_empty())]
#[ensures(ret.as_ref().is_ok_and(|graph| graph.objects.contains_key(&graph.root)) || ret.is_err())]
fn build_graph(input: &CorpusInput) -> Result<SemanticGraph, &'static str> {
    let text = input.text.trim();
    let morphology_options = MorphologyOptions::default().with_dialect_definition(&input.dialect);
    let syntax_options = ParseOptions::default().with_dialect_definition(&input.dialect);
    let source_id = Some(SourceId(format!("<smusni-census:{}>", input.name)));
    let words = segment_words_with_modifiers_with_options_and_source_id(
        text,
        &morphology_options,
        source_id,
    )
    .map_err(|_| "morphology")?;
    let parsed =
        parse_syntax_tree_generated_model_with_source_and_options(&words, text, &syntax_options)
            .map_err(|_| "syntax")?;
    build_generated_semantic_graph_with_dictionary_and_options(
        &parsed,
        SemanticBuildOptions {
            source_text: Some(text),
            story_time: false,
        },
        jbotci_dictionary_data::english(),
    )
    .map_err(|_| "semantics")
}

/// Emit one `DOC` record per input, plus the per-reason edge tallies.
#[requires(!corpus.is_empty())]
#[ensures(true)]
fn census(corpus: &str, inputs: &[CorpusInput], dump: Option<&str>) {
    for input in inputs {
        let graph = match catch_unwind(AssertUnwindSafe(|| build_graph(input))) {
            Ok(Ok(graph)) => graph,
            Ok(Err(stage)) => {
                println!("DOC\t{corpus}\t{}\tbuild-{stage}\t0\t0\t", input.name);
                continue;
            }
            Err(_) => {
                println!("DOC\t{corpus}\t{}\tbuild-panic\t0\t0\t", input.name);
                continue;
            }
        };
        match catch_unwind(AssertUnwindSafe(|| render_smusni(&graph))) {
            Ok(Ok(rendered)) => {
                println!(
                    "DOC\t{corpus}\t{}\tok\t{:016x}\t0\t",
                    input.name,
                    digest(&rendered.text)
                );
                if dump == Some(input.name.as_str()) {
                    println!("----- {} -----", input.name);
                    print!("{}", rendered.text);
                    println!("----- end -----");
                }
            }
            Ok(Err(failed)) => {
                let mut reasons = BTreeMap::<&str, usize>::new();
                for failure in &failed.failures {
                    *reasons.entry(failure.reason_id).or_default() += 1;
                }
                let spelled = reasons
                    .iter()
                    .map(|(reason, count)| {
                        format!(
                            "{}={count}",
                            reason.trim_start_matches("smusni.projection.")
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(",");
                println!(
                    "DOC\t{corpus}\t{}\tfail\t0\t{}\t{spelled}",
                    input.name,
                    failed.failures.len()
                );
                for failure in &failed.failures {
                    if failure.reason_id != "smusni.projection.scope-dependency-without-binder" {
                        continue;
                    }
                    let Some(definition) = failure.use_site else {
                        continue;
                    };
                    let is_restriction = graph
                        .objects
                        .values()
                        .any(|object| object.formula_restriction() == Some(definition));
                    let is_argument_clause = graph.objects.values().any(|object| {
                        object.predication_arguments().is_some_and(|arguments| {
                            arguments.values().any(|argument| {
                                argument
                                    .relative_clauses
                                    .iter()
                                    .any(|clause| clause.body == definition)
                            })
                        })
                    });
                    println!(
                        "SDWB\t{corpus}\t{}\trestriction={is_restriction}\targclause={is_argument_clause}",
                        input.name
                    );
                }
                if dump == Some(input.name.as_str()) {
                    println!("----- {} -----", input.name);
                    println!("text: {}", input.text.trim());
                    for failure in &failed.failures {
                        println!(
                            "  {} owner={:?} use_site={:?} span={}..{} src={:?}",
                            failure.reason_id,
                            failure.owner,
                            failure.use_site,
                            failure.span.byte_start,
                            failure.span.byte_end,
                            input
                                .text
                                .trim()
                                .get(failure.span.byte_start..failure.span.byte_end)
                        );
                    }
                    dump_structure(&graph);
                    println!("----- end -----");
                }
            }
            Err(_) => println!("DOC\t{corpus}\t{}\trender-panic\t0\t0\t", input.name),
        }
    }
}

/// Print a compact object/region/occurrence table for one graph.
#[requires(true)]
#[ensures(true)]
fn dump_structure(graph: &SemanticGraph) {
    println!("objects:");
    for (id, object) in &graph.objects {
        let mut references = Vec::new();
        object.references_into(&mut references);
        println!(
            "  {id} {:?} deps={:?} -> {}",
            object.object_kind(),
            object
                .scope_dependence()
                .and_then(|dependence| dependence.may_depend_on().cloned()),
            references
                .iter()
                .map(|target| format!("{target}"))
                .collect::<Vec<_>>()
                .join(" ")
        );
    }
    println!("regions:");
    for (id, region) in &graph.scope.regions {
        println!(
            "  {id:?} parent={:?} owner={:?}/{:?} boundary={:?} binders={:?}",
            region.parent,
            region.owner.object.map(|owner| format!("{owner}")),
            region.owner.site,
            region.boundary,
            region
                .binders
                .iter()
                .map(|binder| format!("{binder}"))
                .collect::<Vec<_>>()
        );
    }
    println!("origins:");
    for (id, region) in &graph.scope.object_origins {
        println!("  {id} in {region:?}");
    }
    println!("uses:");
    for occurrence in &graph.scope.uses {
        println!(
            "  {} -> {} role={:?} region={:?}",
            occurrence.owner, occurrence.target, occurrence.role, occurrence.region
        );
    }
}

/// Load the retained Phase-B sources.
#[requires(directory.is_dir())]
#[ensures(ret.len() == CORPUS_DOCS.len())]
fn phaseb_inputs(directory: &Path) -> Vec<CorpusInput> {
    CORPUS_DOCS
        .iter()
        .map(|document| {
            let path = directory.join(format!("{document}.lojban"));
            new!(CorpusInput {
                name: format!("phaseb:{document}"),
                text: std::fs::read_to_string(&path)
                    .unwrap_or_else(|error| panic!("read {}: {error}", path.display())),
                dialect: DialectDefinition::default()
            })
        })
        .collect()
}

/// Load all TOML fixtures in one directory that carry an inline `lojban` field.
#[requires(directory.is_dir())]
#[ensures(true)]
fn toml_fixture_inputs(directory: &Path, corpus: &str) -> Vec<CorpusInput> {
    let mut paths = Vec::new();
    toml_fixture_paths_into(directory, &mut paths);
    paths.sort();
    paths
        .into_iter()
        .filter_map(|path| {
            let source = std::fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
            let document = toml::from_str::<toml::Value>(&source)
                .unwrap_or_else(|error| panic!("parse {}: {error}", path.display()));
            let text = document.get("lojban")?.as_str()?.to_owned();
            (!text.trim().is_empty()).then(|| {
                new!(CorpusInput {
                    name: format!(
                        "{corpus}:{}",
                        path.file_stem().expect("fixture stem").to_string_lossy()
                    ),
                    text: text,
                    dialect: DialectDefinition::default()
                })
            })
        })
        .collect()
}

/// Recursively collect TOML fixture paths.
#[requires(directory.is_dir())]
#[ensures(out.len() >= old(out.len()))]
fn toml_fixture_paths_into(directory: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(directory)
        .unwrap_or_else(|error| panic!("read {}: {error}", directory.display()))
    {
        let path = entry.expect("fixture directory entry").path();
        if path.is_dir() {
            toml_fixture_paths_into(&path, out);
        } else if path.extension().and_then(|value| value.to_str()) == Some("toml") {
            out.push(path);
        }
    }
}

/// Load Alice as nonempty physical line units and as one whole document.
#[requires(path.is_file())]
#[ensures(!ret.0.is_empty() && ret.1.len() == 1)]
fn alice_inputs(path: &Path) -> (Vec<CorpusInput>, Vec<CorpusInput>) {
    let text = std::fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
    let dialect = parse_dialect_definition("(case-insensitive zantufa)")
        .expect("Alice fixture dialect is valid");
    let lines = text
        .lines()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty())
        .map(|(index, line)| {
            new!(CorpusInput {
                name: format!("alice-line:{}", index + 1),
                text: line.to_owned(),
                dialect: dialect.clone()
            })
        })
        .collect();
    let whole = vec![new!(CorpusInput {
        name: "alice-whole".to_owned(),
        text: text,
        dialect: dialect
    })];
    (lines, whole)
}

/// Load one xarsnu dialog transcript: each speaker turn is one document.
#[requires(path.is_file())]
#[ensures(true)]
fn frontier_inputs(path: &Path) -> Vec<CorpusInput> {
    let text = std::fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
    let mut turns = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let Some(rest) = line.strip_prefix("**") else {
            continue;
        };
        let Some((speaker, body)) = rest.split_once(":**") else {
            continue;
        };
        if body.trim().is_empty() {
            continue;
        }
        turns.push(new!(CorpusInput {
            name: format!("frontier:{}:{}", index + 1, speaker),
            text: body.trim().to_owned(),
            dialect: DialectDefinition::default()
        }));
    }
    let whole = turns
        .iter()
        .map(|turn| turn.text.clone())
        .collect::<Vec<_>>()
        .join(" .i ");
    if !whole.trim().is_empty() {
        turns.push(new!(CorpusInput {
            name: "frontier:whole".to_owned(),
            text: whole,
            dialect: DialectDefinition::default()
        }));
    }
    turns
}

/// Resolve the repository root from this package's manifest location.
#[requires(true)]
#[ensures(ret.is_dir())]
fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root exists")
}

#[requires(true)]
#[ensures(true)]
fn main() {
    let root = repository_root();
    let requested = std::env::args().nth(1).unwrap_or_else(|| "all".to_owned());
    let dump = std::env::args().nth(2);
    let dump = dump.as_deref();
    if matches!(requested.as_str(), "all" | "phaseb") {
        census(
            "phaseb",
            &phaseb_inputs(&root.join("crates/jbotci-semantics/tests/phaseb_corpus")),
            dump,
        );
    }
    if matches!(requested.as_str(), "all" | "cll") {
        census(
            "cll",
            &toml_fixture_inputs(&root.join("tests/fixtures/cll"), "cll"),
            dump,
        );
    }
    if requested.as_str() == "probe" {
        let text = std::env::args().nth(2).expect("probe text");
        let probe = vec![new!(CorpusInput {
            name: "probe".to_owned(),
            text: text,
            dialect: DialectDefinition::default()
        })];
        census("probe", &probe, Some("probe"));
        return;
    }
    if matches!(requested.as_str(), "fixtures") {
        census(
            "fixtures",
            &toml_fixture_inputs(&root.join("tests/fixtures"), "fixtures"),
            dump,
        );
    }
    if matches!(requested.as_str(), "all" | "frontier") {
        census(
            "frontier",
            &frontier_inputs(Path::new(
                "/home/int19h.linux/git/tersmu-dsl-research/reports/secondchance-2026-08/frontier-r3.dialog.md",
            )),
            dump,
        );
    }
    if matches!(requested.as_str(), "all" | "alice-lines") {
        let (lines, _) =
            alice_inputs(&root.join("tests/fixtures/corpus/alis/texts/full-alice.lojban"));
        census("alice-lines", &lines, dump);
    }
    if matches!(requested.as_str(), "all" | "alice-whole") {
        let (_, whole) =
            alice_inputs(&root.join("tests/fixtures/corpus/alis/texts/full-alice.lojban"));
        census("alice-whole", &whole, dump);
    }
    assert!(
        matches!(
            requested.as_str(),
            "all" | "phaseb" | "cll" | "fixtures" | "frontier" | "alice-lines" | "alice-whole"
        ),
        "unknown corpus slice {requested:?}"
    );
}
