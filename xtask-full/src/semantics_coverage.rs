use std::any::Any;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::num::NonZeroUsize;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::{Context, Result, bail};
#[allow(unused_imports)]
use bityzba::{data, ensures, invariant, new, requires};
use clap::Args;
use jbotci_morphology::{
    MorphologyOptions, segment_words_with_modifiers_with_options_and_source_id,
};
use jbotci_semantics::{
    SemanticBuildOptions, build_generated_semantic_graph_with_dictionary_and_options,
};
use jbotci_source::SourceId;
use jbotci_syntax::{ParseOptions, parse_syntax_tree_generated_model_with_source_and_options};
use rayon::prelude::*;
use xtask_common::fixtures::{ExpectationStatus, LoadedTestCase, fixture_paths, load_fixture_path};

const DEFAULT_JOBS: usize = 16;
const DEFAULT_FAILURE_SAMPLES: usize = 20;
const UNSUPPORTED_ERROR_MARKER: &str = "generated semantic builder does not yet support ";

#[invariant(true)]
#[derive(Debug, Args)]
pub(crate) struct SemanticsCoverageArgs {
    /// Fixture corpus root.
    #[arg(long, default_value = "tests/fixtures")]
    root: PathBuf,
    /// Shrink-only set of fixture IDs that currently panic or report unsupported syntax.
    #[arg(long, default_value = "tests/semantics-coverage-allowlist.txt")]
    allowlist: PathBuf,
    /// Parallel semantic-analysis jobs.
    #[arg(long, default_value_t = NonZeroUsize::new(DEFAULT_JOBS).expect("nonzero default"))]
    jobs: NonZeroUsize,
    /// Maximum details printed for each ratchet failure direction; zero prints none.
    #[arg(long, default_value_t = DEFAULT_FAILURE_SAMPLES)]
    failure_samples: usize,
}

#[invariant(::Success => true)]
#[invariant(::OtherError { message } => !message.is_empty())]
#[invariant(::Unsupported { class } => !class.is_empty())]
#[invariant(::Panic { message } => !message.is_empty())]
#[derive(Debug)]
enum SemanticsCoverageClassification {
    Success,
    OtherError { message: String },
    Unsupported { class: String },
    Panic { message: String },
}

impl SemanticsCoverageClassification {
    #[requires(true)]
    #[ensures(true)]
    fn is_ratchet_failure(&self) -> bool {
        matches!(
            self.as_data(),
            data!(SemanticsCoverageClassification::Unsupported { .. })
                | data!(SemanticsCoverageClassification::Panic { .. })
        )
    }

    #[requires(self.is_ratchet_failure())]
    #[ensures(!ret.is_empty())]
    fn unexpected_description(&self) -> String {
        match self.as_data() {
            data!(SemanticsCoverageClassification::Unsupported { class }) => {
                format!("reported unsupported construct class `{class}`")
            }
            data!(SemanticsCoverageClassification::Panic { message }) => {
                format!("panicked: {}", one_line_excerpt(message, 240))
            }
            _ => unreachable!("precondition requires a ratchet failure"),
        }
    }

    #[requires(!self.is_ratchet_failure())]
    #[ensures(!ret.is_empty())]
    fn stale_description(&self) -> String {
        match self.as_data() {
            data!(SemanticsCoverageClassification::Success) => {
                "semantic analysis now succeeds".to_owned()
            }
            data!(SemanticsCoverageClassification::OtherError { message }) => format!(
                "semantic analysis now reports an allowed other error: {}",
                one_line_excerpt(message, 240)
            ),
            _ => unreachable!("precondition excludes ratchet failures"),
        }
    }
}

#[invariant(!fixture_id.is_empty())]
#[derive(Debug)]
struct SemanticsCoverageOutcome {
    fixture_id: String,
    classification: SemanticsCoverageClassification,
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
pub(crate) fn run(args: SemanticsCoverageArgs) -> Result<()> {
    let started = Instant::now();
    let allowlist = load_allowlist(&args.allowlist)?;
    let paths = fixture_paths(&args.root)
        .with_context(|| format!("listing fixtures under `{}`", args.root.display()))?;
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(args.jobs.get())
        .build()
        .context("creating semantics-coverage thread pool")?;

    // The semantic builder owns all mutable analysis state for each call and only shares the
    // immutable embedded dictionary. Catching an unwind per fixture therefore cannot poison
    // another fixture's state. Silence the process-global panic hook while all such panics are
    // deliberately captured; the panic payload remains part of the fixture classification.
    let previous_panic_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let loaded_outcomes = catch_unwind(AssertUnwindSafe(|| {
        pool.install(|| {
            paths
                .par_iter()
                .map(|path| run_fixture_path(path))
                .collect::<Vec<_>>()
        })
    }));
    let _coverage_panic_hook = std::panic::take_hook();
    std::panic::set_hook(previous_panic_hook);
    let loaded_outcomes = match loaded_outcomes {
        Ok(outcomes) => outcomes,
        Err(payload) => bail!(
            "semantics-coverage harness panicked outside per-fixture isolation: {}",
            panic_payload_message(payload.as_ref())
        ),
    };

    let mut outcomes = BTreeMap::new();
    for outcome in loaded_outcomes {
        let Some(outcome) = outcome? else {
            continue;
        };
        let outcome = outcome.into_data();
        if outcomes
            .insert(outcome.fixture_id.clone(), outcome.classification)
            .is_some()
        {
            bail!(
                "duplicate syntax-success fixture id `{}` encountered by semantics coverage",
                outcome.fixture_id
            );
        }
    }

    print_summary(&outcomes, allowlist.len(), started.elapsed().as_secs_f64());
    enforce_ratchet(&outcomes, &allowlist, &args.allowlist, args.failure_samples)
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn run_fixture_path(path: &Path) -> Result<Option<SemanticsCoverageOutcome>> {
    let fixture =
        load_fixture_path(path).with_context(|| format!("loading fixture `{}`", path.display()))?;
    if !fixture.test_case.is_valid_fixture_metadata() {
        bail!(
            "fixture `{}` has invalid metadata; run `cargo xtask fixture-check` for details",
            path.display()
        );
    }
    if !fixture
        .test_case
        .expectations
        .syntax
        .as_ref()
        .is_some_and(|syntax| syntax.status == ExpectationStatus::Success)
    {
        return Ok(None);
    }
    let classification = match catch_unwind(AssertUnwindSafe(|| analyze_fixture(&fixture))) {
        Ok(Ok(())) => new!(SemanticsCoverageClassification::Success),
        Ok(Err(message)) => match unsupported_construct_class(&message) {
            Some(class) => new!(SemanticsCoverageClassification::Unsupported { class }),
            None => new!(SemanticsCoverageClassification::OtherError { message }),
        },
        Err(payload) => new!(SemanticsCoverageClassification::Panic {
            message: panic_payload_message(payload.as_ref()),
        }),
    };
    Ok(Some(new!(SemanticsCoverageOutcome {
        fixture_id: fixture.test_case.id,
        classification,
    })))
}

#[requires(fixture.test_case.is_valid_fixture_metadata())]
#[ensures(ret.as_ref().err().is_none_or(|message| !message.is_empty()))]
fn analyze_fixture(fixture: &LoadedTestCase) -> std::result::Result<(), String> {
    let dialect = fixture
        .test_case
        .dialect_definition()
        .map_err(|error| format!("dialect error: {error}"))?;
    let morphology_options = MorphologyOptions::default().with_dialect_definition(&dialect);
    let syntax_options = ParseOptions::default().with_dialect_definition(&dialect);
    let source_id = Some(SourceId(format!("<fixture:{}>", fixture.test_case.id)));
    let words = segment_words_with_modifiers_with_options_and_source_id(
        &fixture.test_case.lojban,
        &morphology_options,
        source_id,
    )
    .map_err(|error| format!("morphology error: {error}"))?;
    let parsed = parse_syntax_tree_generated_model_with_source_and_options(
        &words,
        &fixture.test_case.lojban,
        &syntax_options,
    )
    .map_err(|error| format!("syntax error: {error}"))?;
    build_generated_semantic_graph_with_dictionary_and_options(
        &parsed,
        SemanticBuildOptions {
            source_text: Some(&fixture.test_case.lojban),
            story_time: false,
        },
        jbotci_dictionary_data::english(),
    )
    .map(|_| ())
    .map_err(|error| format!("semantic error: {error}"))
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|class| !class.is_empty()))]
fn unsupported_construct_class(message: &str) -> Option<String> {
    let marker_offset = message.find(UNSUPPORTED_ERROR_MARKER)?;
    let class = message[marker_offset + UNSUPPORTED_ERROR_MARKER.len()..]
        .lines()
        .next()
        .unwrap_or_default()
        .trim();
    (!class.is_empty()).then(|| class.to_owned())
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn panic_payload_message(payload: &(dyn Any + Send)) -> String {
    if let Some(message) = payload.downcast_ref::<String>() {
        return if message.is_empty() {
            "empty String panic payload".to_owned()
        } else {
            message.clone()
        };
    }
    if let Some(message) = payload.downcast_ref::<&str>() {
        return if message.is_empty() {
            "empty str panic payload".to_owned()
        } else {
            (*message).to_owned()
        };
    }
    "non-string panic payload".to_owned()
}

#[requires(limit > 0)]
#[ensures(!ret.contains('\n') && !ret.contains('\r'))]
fn one_line_excerpt(text: &str, limit: usize) -> String {
    let one_line = text.replace(['\n', '\r'], " ");
    let mut characters = one_line.chars();
    let excerpt = characters.by_ref().take(limit).collect::<String>();
    if characters.next().is_some() {
        format!("{excerpt}…")
    } else {
        excerpt
    }
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn load_allowlist(path: &Path) -> Result<BTreeSet<String>> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("reading semantics coverage allowlist `{}`", path.display()))?;
    parse_allowlist(&text, path)
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn parse_allowlist(text: &str, path: &Path) -> Result<BTreeSet<String>> {
    let mut entries = Vec::<String>::new();
    let mut saw_header_comment = false;
    let mut saw_entry = false;
    for (line_index, line) in text.lines().enumerate() {
        let line_number = line_index + 1;
        if line.is_empty() {
            continue;
        }
        if line.starts_with('#') {
            if saw_entry {
                bail!(
                    "allowlist `{}` line {line_number}: comments are only permitted in the header",
                    path.display()
                );
            }
            saw_header_comment = true;
            continue;
        }
        if !saw_header_comment {
            bail!(
                "allowlist `{}` must begin with a header comment explaining the shrink-only rule",
                path.display()
            );
        }
        if line.trim() != line {
            bail!(
                "allowlist `{}` line {line_number}: fixture ids must not have surrounding whitespace",
                path.display()
            );
        }
        if let Some(previous) = entries.last()
            && previous.as_str() >= line
        {
            bail!(
                "allowlist `{}` line {line_number}: fixture ids must be unique and sorted; `{line}` must follow `{previous}`",
                path.display()
            );
        }
        entries.push(line.to_owned());
        saw_entry = true;
    }
    if !saw_header_comment {
        bail!(
            "allowlist `{}` must contain a header comment explaining the shrink-only rule",
            path.display()
        );
    }
    Ok(entries.into_iter().collect())
}

#[requires(elapsed_seconds >= 0.0)]
#[ensures(true)]
fn print_summary(
    outcomes: &BTreeMap<String, SemanticsCoverageClassification>,
    allowlist_size: usize,
    elapsed_seconds: f64,
) {
    let mut success = 0usize;
    let mut other_error = 0usize;
    let mut panic = 0usize;
    let mut unsupported = 0usize;
    let mut unsupported_classes = BTreeMap::<&str, usize>::new();
    for classification in outcomes.values() {
        match classification.as_data() {
            data!(SemanticsCoverageClassification::Success) => success += 1,
            data!(SemanticsCoverageClassification::OtherError { .. }) => other_error += 1,
            data!(SemanticsCoverageClassification::Panic { .. }) => panic += 1,
            data!(SemanticsCoverageClassification::Unsupported { class }) => {
                unsupported += 1;
                *unsupported_classes.entry(class).or_default() += 1;
            }
        }
    }
    println!(
        "semantics coverage: checked={} success={success} other-error={other_error} panic={panic} unsupported={unsupported} allowlist={allowlist_size} elapsed={elapsed_seconds:.3}s",
        outcomes.len()
    );
    println!("unsupported classes (alphabetical):");
    for (class, count) in unsupported_classes {
        println!("  {count:>5}  {class}");
    }
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn enforce_ratchet(
    outcomes: &BTreeMap<String, SemanticsCoverageClassification>,
    allowlist: &BTreeSet<String>,
    allowlist_path: &Path,
    failure_samples: usize,
) -> Result<()> {
    let unexpected_count = outcomes
        .iter()
        .filter(|(fixture_id, classification)| {
            classification.is_ratchet_failure() && !allowlist.contains(*fixture_id)
        })
        .count();
    let stale_count = allowlist
        .iter()
        .filter(|fixture_id| {
            outcomes
                .get(*fixture_id)
                .is_none_or(|classification| !classification.is_ratchet_failure())
        })
        .count();

    if unexpected_count > 0 {
        eprintln!(
            "semantics coverage found {unexpected_count} non-allowlisted panic/unsupported failure(s):"
        );
        let mut printed = 0usize;
        for (fixture_id, classification) in outcomes {
            if printed >= failure_samples {
                break;
            }
            if classification.is_ratchet_failure() && !allowlist.contains(fixture_id) {
                eprintln!(
                    "  UNEXPECTED `{fixture_id}`: {}. Fix the semantic-analysis regression; do not add this fixture to `{}` because the allowlist is shrink-only.",
                    classification.unexpected_description(),
                    allowlist_path.display()
                );
                printed += 1;
            }
        }
        if unexpected_count > printed {
            eprintln!(
                "  ... {} more (rerun with `--failure-samples {unexpected_count}` to show all)",
                unexpected_count - printed
            );
        }
    }

    if stale_count > 0 {
        eprintln!("semantics coverage found {stale_count} stale allowlist entry/entries:");
        let mut printed = 0usize;
        for fixture_id in allowlist {
            if printed >= failure_samples {
                break;
            }
            match outcomes.get(fixture_id) {
                Some(classification) if classification.is_ratchet_failure() => {}
                Some(classification) => {
                    eprintln!(
                        "  STALE `{fixture_id}`: {}. Remove this fixture id from `{}`.",
                        classification.stale_description(),
                        allowlist_path.display()
                    );
                    printed += 1;
                }
                None => {
                    eprintln!(
                        "  STALE `{fixture_id}`: this id is not a fixture whose own syntax expectation is `success`. Remove this fixture id from `{}`.",
                        allowlist_path.display()
                    );
                    printed += 1;
                }
            }
        }
        if stale_count > printed {
            eprintln!(
                "  ... {} more (rerun with `--failure-samples {stale_count}` to show all)",
                stale_count - printed
            );
        }
    }

    if unexpected_count > 0 || stale_count > 0 {
        bail!(
            "semantics coverage ratchet failed: {unexpected_count} unexpected failure(s), {stale_count} stale allowlist entry/entries"
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn unsupported_class_matching_accepts_context_without_full_string_equality() {
        assert_eq!(
            unsupported_construct_class(
                "semantic error: generated semantic builder does not yet support quantified sumti"
            )
            .as_deref(),
            Some("quantified sumti")
        );
        assert_eq!(
            unsupported_construct_class("semantic error: a principled failure"),
            None
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn allowlist_parser_requires_a_sorted_unique_headered_list() {
        let path = Path::new("allowlist.txt");
        let parsed = parse_allowlist("# Shrink only.\na\nb\n", path).unwrap();
        assert_eq!(parsed.into_iter().collect::<Vec<_>>(), ["a", "b"]);
        assert!(parse_allowlist("a\n", path).is_err());
        assert!(parse_allowlist("# Shrink only.\nb\na\n", path).is_err());
        assert!(parse_allowlist("# Shrink only.\na\na\n", path).is_err());
    }
}
