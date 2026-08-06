//! Reproduce the non-golden corpus measurements for smusni S-expressions.
//!
//! This reports only aggregate structural outcomes. It never writes output
//! expectations or treats a rendered sample as semantic authority.

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
use jbotci_semantics::model::SemanticObjectId;
use jbotci_semantics::{
    SemanticBuildOptions, SemanticGraph, SmusniProjectionFailed, SmusniRender,
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

/// Aggregate outcomes for one independently named corpus slice.
#[invariant(true)]
#[derive(Debug, Default)]
struct CorpusReport {
    inputs: usize,
    successes: usize,
    build_failures: BTreeMap<&'static str, usize>,
    build_panics: usize,
    render_panics: usize,
    /// Inputs that produced one complete document.
    compact_documents: usize,
    /// Inputs whose projection failed and therefore produced no document.
    failed_projections: usize,
    compact_objects: usize,
    /// The renderer's edge statistic. Reporting edges and owners separately is
    /// what makes the per-edge channel observable in a corpus: an owner that
    /// declines at two different boundaries contributes two edges and one
    /// owner.
    failed_projection_edges: usize,
    /// Distinct owners named by those edges.
    failing_owners: usize,
    multi_edge_owners: usize,
    semantic_diagnostics: usize,
    failure_reasons: BTreeMap<String, usize>,
}

/// Count failed projection edges per named owner in one failed projection.
///
/// Owner evidence is optional, so a record that carries none simply does not
/// contribute to this observation.
#[requires(true)]
#[ensures(ret.values().all(|total| *total > 0))]
fn edges_per_owner(failed: &SmusniProjectionFailed) -> BTreeMap<SemanticObjectId, usize> {
    let mut owners = BTreeMap::<SemanticObjectId, usize>::new();
    for failure in &failed.failures {
        if let Some(owner) = failure.owner {
            *owners.entry(owner).or_default() += 1;
        }
    }
    owners
}

impl CorpusReport {
    /// Record one complete document.
    #[requires(true)]
    #[ensures(self.compact_documents == old(self.compact_documents) + 1)]
    fn record_document(&mut self, rendered: &SmusniRender) {
        self.successes += 1;
        self.compact_documents += 1;
        self.compact_objects += rendered.stats.compact_objects;
        self.semantic_diagnostics += rendered.stats.semantic_diagnostic_count;
    }

    /// Record one projection failure and its per-edge measurements.
    #[requires(!failed.failures.is_empty())]
    #[ensures(self.failed_projections == old(self.failed_projections) + 1)]
    fn record_failure(&mut self, failed: &SmusniProjectionFailed) {
        let owners = edges_per_owner(failed);
        self.failing_owners += owners.len();
        self.multi_edge_owners += owners.values().filter(|total| **total > 1).count();
        self.failed_projections += 1;
        self.failed_projection_edges += failed.stats.failed_projection_edges;
        self.semantic_diagnostics += failed.stats.semantic_diagnostic_count;
        for (reason, count) in &failed.stats.failure_reasons {
            *self
                .failure_reasons
                .entry((*reason).to_owned())
                .or_default() += count;
        }
    }

    /// Print stable tab-separated aggregate and reason records.
    #[requires(!corpus.is_empty())]
    #[ensures(true)]
    fn print(&self, corpus: &str) {
        println!(
            "SUMMARY\t{corpus}\tinputs={}\tsuccesses={}\tbuild_failures={}\tbuild_panics={}\trender_panics={}\tcompact_documents={}\tfailed_projections={}\tcompact_objects={}\tfailed_projection_edges={}\tfailing_owners={}\tmulti_edge_owners={}\tsemantic_diagnostics={}",
            self.inputs,
            self.successes,
            self.build_failures.values().sum::<usize>(),
            self.build_panics,
            self.render_panics,
            self.compact_documents,
            self.failed_projections,
            self.compact_objects,
            self.failed_projection_edges,
            self.failing_owners,
            self.multi_edge_owners,
            self.semantic_diagnostics,
        );
        for (stage, count) in &self.build_failures {
            println!("BUILD_FAILURE\t{corpus}\t{stage}\t{count}");
        }
        for (reason, count) in &self.failure_reasons {
            println!("PROJECTION_FAILURE_REASON\t{corpus}\t{reason}\t{count}");
        }
    }
}

/// Read one exact Linux process-memory counter for the corpus diagnostics.
/// Other hosts simply omit the optional measurement.
#[requires(!field.is_empty())]
#[ensures(true)]
fn process_status_kib(field: &str) -> Option<usize> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    let line = status.lines().find(|line| line.starts_with(field))?;
    let mut fields = line.split_ascii_whitespace();
    let label = fields.next()?;
    let value = fields.next()?.parse::<usize>().ok()?;
    let unit = fields.next()?;
    (label == field && unit == "kB" && fields.next().is_none()).then_some(value)
}

/// Build a validated semantic graph through the production morphology, syntax,
/// and semantic builder pipeline.
#[requires(!input.name.is_empty() && !input.text.trim().is_empty())]
#[ensures(ret.as_ref().is_ok_and(|graph| graph.objects.contains_key(&graph.root)) || ret.is_err())]
fn build_graph(input: &CorpusInput) -> Result<SemanticGraph, &'static str> {
    let text = input.text.trim();
    let morphology_options = MorphologyOptions::default().with_dialect_definition(&input.dialect);
    let syntax_options = ParseOptions::default().with_dialect_definition(&input.dialect);
    let source_id = Some(SourceId(format!("<smusni-report:{}>", input.name)));
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

/// Measure one corpus while distinguishing returned build failures from panics.
#[requires(true)]
#[ensures(ret.inputs == inputs.len())]
fn measure(inputs: &[CorpusInput]) -> CorpusReport {
    let mut report = CorpusReport::default();
    for input in inputs {
        report.inputs += 1;
        let graph = match catch_unwind(AssertUnwindSafe(|| build_graph(input))) {
            Ok(Ok(graph)) => graph,
            Ok(Err(stage)) => {
                *report.build_failures.entry(stage).or_default() += 1;
                continue;
            }
            Err(_) => {
                report.build_panics += 1;
                continue;
            }
        };
        if input.name == "alice-whole" {
            eprintln!(
                "ALICE_WHOLE\tbuilt\tobjects={}\trss_kib={}\tpeak_kib={}",
                graph.objects.len(),
                process_status_kib("VmRSS:").unwrap_or(0),
                process_status_kib("VmHWM:").unwrap_or(0),
            );
        }
        match catch_unwind(AssertUnwindSafe(|| render_smusni(&graph))) {
            Ok(projection) => {
                if input.name == "alice-whole" {
                    eprintln!(
                        "ALICE_WHOLE\tprojected\trss_kib={}\tpeak_kib={}",
                        process_status_kib("VmRSS:").unwrap_or(0),
                        process_status_kib("VmHWM:").unwrap_or(0),
                    );
                }
                match projection {
                    Ok(rendered) => report.record_document(&rendered),
                    Err(failed) => report.record_failure(&failed),
                }
            }
            Err(_) => report.render_panics += 1,
        }
    }
    report
}

/// Load the retained Phase-B sources used by structural renderer tests.
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

/// Recursively collect TOML fixture paths without interpreting directory names.
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

/// Focused exact surfaces required by the renderer design review.
#[requires(true)]
#[ensures(ret.len() == 16)]
fn focused_inputs() -> Vec<CorpusInput> {
    [
        ("deleted-place", "mi dunda zi'o ti"),
        ("paragraph", "ni'o mi klama"),
        ("relation-question", "ti mo zdani"),
        ("quotation", "mi cusku lu mi prami do li'u"),
        ("hostile-quotation", r#"mi cusku zoi gy. a"b\c(d);e .gy"#),
        ("relative-poi", "mi klama lo zarci poi do nelci ke'a"),
        ("relative-noi", "mi klama lo zarci noi do nelci ke'a"),
        ("connective", "ganai broda gi brode .a brodi"),
        ("abstraction", "mi djuno lo du'u do klama"),
        ("termset", "ci gerku ce'e re nanmu cu batci"),
        (
            "respectively-distribution",
            "la djan fa'u la frank cusku nu'i fa'ugi bau la lojban nu'u gi bai tu'a la djordj",
        ),
        (
            "respectively-values",
            "la djeimyz fa'u la djordj prami la meris fa'u la martas",
        ),
        ("modal", "mi klama fi'o pilno lo karce"),
        ("tense", "mi pu klama lo zarci"),
        ("math", "li pa su'i re du li ci"),
        ("displayed", "mi klama sei do cusku"),
    ]
    .into_iter()
    .map(|(name, text)| {
        new!(CorpusInput {
            name: format!("focused:{name}"),
            text: text.to_owned(),
            dialect: DialectDefinition::default()
        })
    })
    .collect()
}

/// Load Alice as both nonempty physical line units and one whole document.
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
    if matches!(requested.as_str(), "all" | "phaseb") {
        let phaseb = phaseb_inputs(&root.join("crates/jbotci-semantics/tests/phaseb_corpus"));
        measure(&phaseb).print("phaseb");
    }
    if matches!(requested.as_str(), "all" | "cll") {
        let cll = toml_fixture_inputs(&root.join("tests/fixtures/cll"), "cll");
        measure(&cll).print("cll");
    }
    if matches!(requested.as_str(), "all" | "focused") {
        let focused = focused_inputs();
        measure(&focused).print("focused");
    }
    if matches!(requested.as_str(), "all" | "alice-lines") {
        let (alice_lines, _) =
            alice_inputs(&root.join("tests/fixtures/corpus/alis/texts/full-alice.lojban"));
        measure(&alice_lines).print("alice-lines");
    }
    if matches!(requested.as_str(), "all" | "alice-whole") {
        let (_, alice_whole) =
            alice_inputs(&root.join("tests/fixtures/corpus/alis/texts/full-alice.lojban"));
        eprintln!("ALICE_WHOLE\tbuilding");
        let report = measure(&alice_whole);
        eprintln!("ALICE_WHOLE\tfinished");
        report.print("alice-whole");
    }
    assert!(
        matches!(
            requested.as_str(),
            "all" | "phaseb" | "cll" | "focused" | "alice-lines" | "alice-whole"
        ),
        "unknown corpus slice {requested:?}"
    );
}
