mod classification;
mod git;
mod ledger;
mod model;
mod output;
mod python;
mod rust;
mod source;

use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use bityzba::{ensures, invariant, new, requires};
use clap::Args;
use classification::{
    CLASSIFICATION_CATALOG_PATH, ClassificationCatalog, ExecutableConsumerRule,
    LoadedClassificationCatalog, RecordFamily, load_catalog,
};
use git::GitTree;
use ledger::{ClassifiableRecord, build_migration_ledger};
use model::{
    ARTIFACT_DIRECTORY, ConsumerLanguage, ExecutableConsumerRecord, FileClass, FileRecord,
    Manifest, ManifestClassificationCatalog, ManifestSourceFile, PythonCensusRecord,
    TestFixtureKind, TestFixtureRecord, EXTRACTOR_VERSION, JSONL_ARTIFACTS,
};
use output::ArtifactSet;
use python::extract_python_file;
use rust::{RustInventory, extract_rust_file, validate_extraction_completeness};
use source::{SourceMap, record_id, sha256_hex};

const SEMANTIC_DOC_PATHS: &[&str] = &[
    "docs/semantic-model-cll-encodings.md",
    "docs/semantic-model-design.md",
    "docs/semantic-model-spec.md",
    "review/tersmu_schema_primer.md",
];
const RUST_CONSUMER_PATHS: &[&str] = &[
    "apps/jbotci/src/commands/tersmu.rs",
    "apps/jbotci/src/tool.rs",
    "apps/jbotci-server/src/lib.rs",
    "apps/jbotci-server/src/mcp.rs",
];
const RUST_VALIDATION_PATHS: &[&str] = &["xtask-full/src/semantics_coverage.rs"];
const SEMANTICS_COVERAGE_ALLOWLIST: &str = "tests/semantics-coverage-allowlist.txt";
const VENDORED_PYTHON_PREFIXES: &[&str] = &["crates/vendor/"];
const FIXTURE_BATCH_SIZE: usize = 256;
const PINNED_SOURCE_COMMIT: &str = "68a9950e4959accf818485a2bc09381d7e35f427";
const PINNED_SOURCE_TREE: &str = "3ada614819ea1e37215303c5261420bd0b1abe5d";

const PLANNING_REFERENCE_PYTHON_PATHS: &[&str] = &[
    "scripts/verify-domain-import-fixtures.py",
    "scripts/verify-recovery-marker-fixture-migration.py",
    "scripts/verify_issue_368_cli_compatibility.py",
    "scripts/verify_issue_368_fixture_migration.py",
    "scripts/verify_issue_373_fixture_migration.py",
    "scripts/verify_issue_374_fixture_migration.py",
    "scripts/verify_issue_376_fixture_migration.py",
    "scripts/verify_issue_377_fixture_migration.py",
    "scripts/verify_issue_378_fixture_migration.py",
    "scripts/verify_issue_379_fixture_migration.py",
    "scripts/verify_issue_394_fixture_migration.py",
    "tools/compare-recovery-fixture-expectations.py",
    "tools/embedding-pack/f2llm/build-vector-pack.py",
    "tools/embedding-pack/f2llm/export-webgpu-from-onnx-q4.py",
    "tools/embedding-pack/f2llm/validate-vector-pack.py",
    "tools/embedding-pack/f2llm/validate-webgpu-artifact.py",
    "tools/verify-discourse-fixture-updates.py",
    "tools/xarsnu/scripts/bisect-openrouter-request.py",
    "tools/xarsnu/scripts/import-bickr-openrouter-capabilities.py",
];

#[invariant(*check || commit.as_ref().is_some_and(|commit| !commit.is_empty()))]
#[derive(Debug, Args)]
pub(crate) struct SemanticSourceInventoryArgs {
    /// Exact pre-#568 source commit to inventory.
    #[arg(long, required_unless_present = "check", conflicts_with = "check")]
    commit: Option<String>,
    /// Regenerate from the commit/tree in the committed manifest and compare every byte.
    #[arg(long)]
    check: bool,
}

#[invariant(true)]
#[derive(Debug, Default)]
struct Inventory {
    files: Vec<FileRecord>,
    rust: RustInventory,
    tests: Vec<TestFixtureRecord>,
    python: Vec<PythonCensusRecord>,
    executable_consumers: Vec<ExecutableConsumerRecord>,
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
pub(crate) fn run(args: SemanticSourceInventoryArgs) -> Result<()> {
    let repository_root = repository_root()?;
    let loaded_catalog = load_catalog(&repository_root)?;
    if args.check {
        check(&repository_root, &loaded_catalog)
    } else {
        let commit = args
            .commit
            .as_deref()
            .context("generation requires `--commit`")?;
        let tree = GitTree::load(&repository_root, commit)?;
        validate_expected_pin(&tree)?;
        let generated = generate(&tree, &loaded_catalog)?;
        let output = repository_root.join(ARTIFACT_DIRECTORY);
        generated.artifacts.write(&output, &generated.manifest)?;
        println!(
            "generated semantic source inventory: commit={} tree={} files={} ledger={} output={}",
            generated.manifest.commit,
            generated.manifest.tree,
            generated.manifest.source_files.len(),
            generated
                .manifest
                .record_counts
                .get("migration-ledger.jsonl")
                .copied()
                .unwrap_or_default(),
            output.display()
        );
        Ok(())
    }
}

#[invariant(true)]
struct GeneratedInventory {
    artifacts: ArtifactSet,
    manifest: Manifest,
}

#[requires(repository_root.is_absolute())]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn check(
    repository_root: &Path,
    loaded_catalog: &LoadedClassificationCatalog,
) -> Result<()> {
    let committed = repository_root.join(ARTIFACT_DIRECTORY);
    let manifest_path = committed.join("manifest.json");
    let manifest_bytes = fs::read(&manifest_path).with_context(|| {
        format!(
            "reading committed semantic source inventory `{}`",
            manifest_path.display()
        )
    })?;
    let committed_manifest: Manifest = serde_json::from_slice(&manifest_bytes)
        .context("parsing committed semantic source inventory manifest")?;
    if committed_manifest.extractor_version != EXTRACTOR_VERSION {
        bail!(
            "committed extractor version `{}` differs from supported `{EXTRACTOR_VERSION}`",
            committed_manifest.extractor_version
        );
    }
    if committed_manifest.classification_catalog.path != CLASSIFICATION_CATALOG_PATH
        || committed_manifest.classification_catalog.sha256 != loaded_catalog.sha256
        || committed_manifest.classification_catalog.byte_length != loaded_catalog.byte_length
    {
        bail!("classification catalog identity differs from the committed manifest");
    }
    let tree = GitTree::load(repository_root, &committed_manifest.commit)?;
    validate_expected_pin(&tree)?;
    if tree.tree != committed_manifest.tree {
        bail!(
            "manifest tree `{}` does not match commit `{}` tree `{}`",
            committed_manifest.tree,
            committed_manifest.commit,
            tree.tree
        );
    }
    tree.ensure_worktree_clean()?;
    let regenerated = generate(&tree, loaded_catalog)?;
    let temporary = tempfile::tempdir().context("creating inventory check temporary directory")?;
    let temporary_output = temporary.path().join("source-inventory-v1");
    regenerated
        .artifacts
        .write(&temporary_output, &regenerated.manifest)?;
    compare_directories(&temporary_output, &committed)?;
    println!(
        "semantic source inventory check passed: commit={} tree={} files={} artifacts=16",
        regenerated.manifest.commit,
        regenerated.manifest.tree,
        regenerated.manifest.source_files.len()
    );
    Ok(())
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn validate_expected_pin(tree: &GitTree) -> Result<()> {
    if tree.commit != PINNED_SOURCE_COMMIT || tree.tree != PINNED_SOURCE_TREE {
        bail!(
            "semantic source inventory requires pinned commit `{PINNED_SOURCE_COMMIT}` and tree `{PINNED_SOURCE_TREE}`, resolved commit `{}` and tree `{}`",
            tree.commit,
            tree.tree
        );
    }
    Ok(())
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn generate(
    tree: &GitTree,
    loaded_catalog: &LoadedClassificationCatalog,
) -> Result<GeneratedInventory> {
    let mut inventory = Inventory::default();
    let rust_paths = discover_rust_paths(tree)?;
    let python_paths = discover_python_paths(tree);
    validate_python_catalog(&python_paths, &loaded_catalog.catalog)?;

    let mut primary_paths = rust_paths.keys().cloned().collect::<BTreeSet<_>>();
    primary_paths.extend(python_paths.iter().cloned());
    primary_paths.extend(SEMANTIC_DOC_PATHS.iter().map(|path| (*path).to_owned()));
    primary_paths.insert(SEMANTICS_COVERAGE_ALLOWLIST.to_owned());
    require_paths(tree, &primary_paths)?;
    let primary_blobs = tree.read_blobs(&primary_paths)?;

    for (path, class) in &rust_paths {
        let bytes = primary_blobs
            .get(path)
            .with_context(|| format!("missing loaded Rust source `{path}`"))?;
        let source = utf8_source(path, bytes)?;
        add_file_record(&mut inventory.files, tree, path, bytes, *class)?;
        extract_rust_file(path, source, *class, &mut inventory.rust)?;
    }
    for path in SEMANTIC_DOC_PATHS {
        let bytes = primary_blobs
            .get(*path)
            .with_context(|| format!("missing loaded semantic document `{path}`"))?;
        utf8_source(path, bytes)?;
        add_file_record(
            &mut inventory.files,
            tree,
            path,
            bytes,
            FileClass::SemanticDocumentation,
        )?;
    }
    let python_rules = loaded_catalog
        .catalog
        .python_files
        .iter()
        .map(|rule| (rule.path.as_str(), rule))
        .collect::<BTreeMap<_, _>>();
    for path in &python_paths {
        let bytes = primary_blobs
            .get(path)
            .with_context(|| format!("missing loaded Python source `{path}`"))?;
        let rule = python_rules
            .get(path.as_str())
            .with_context(|| format!("Python path `{path}` lacks an exact catalog row"))?;
        add_file_record(
            &mut inventory.files,
            tree,
            path,
            bytes,
            FileClass::Python,
        )?;
        inventory.python.push(extract_python_file(
            path,
            &tree.entries[path].object_id,
            bytes,
            rule,
        )?);
    }
    let allowlist_bytes = &primary_blobs[SEMANTICS_COVERAGE_ALLOWLIST];
    let allowlist_source = utf8_source(SEMANTICS_COVERAGE_ALLOWLIST, allowlist_bytes)?;
    add_file_record(
        &mut inventory.files,
        tree,
        SEMANTICS_COVERAGE_ALLOWLIST,
        allowlist_bytes,
        FileClass::SemanticFixtureConfiguration,
    )?;
    let allowlist_identity = SourceMap::new(SEMANTICS_COVERAGE_ALLOWLIST, allowlist_source).whole_file();
    inventory.tests.push(new!(TestFixtureRecord {
        id: record_id("fixture-configuration", &allowlist_identity),
        source: allowlist_identity,
        name: "semantics-coverage-allowlist".to_owned(),
        kind: TestFixtureKind::CoverageAllowlist,
        owner_id: None,
        semantic_reference_expectation: true,
        tersmu_output_expectation: false,
    }));

    extract_fixtures(tree, &mut inventory)?;
    validate_extraction_completeness(&inventory.rust)?;
    inventory.executable_consumers = executable_consumers(
        &inventory,
        &loaded_catalog.catalog.executable_consumers,
    )?;
    validate_planning_reference_python(&inventory.python)?;
    sort_inventory(&mut inventory);
    validate_inventory(&inventory)?;

    let classifiable = classifiable_records(&inventory)?;
    let (ledger, rule_hits) = build_migration_ledger(&classifiable, &loaded_catalog.catalog)?;
    if ledger.iter().any(|row| row.primary_owner == 580)
        || ledger
            .iter()
            .any(|row| row.additional_owners.contains(&580))
    {
        bail!("#580 must own zero current-source migration ledger rows");
    }

    let mut artifacts = ArtifactSet::default();
    artifacts.insert_jsonl("files.jsonl", &inventory.files)?;
    artifacts.insert_jsonl("modules.jsonl", &inventory.rust.modules)?;
    artifacts.insert_jsonl("types.jsonl", &inventory.rust.types)?;
    artifacts.insert_jsonl("variants.jsonl", &inventory.rust.variants)?;
    artifacts.insert_jsonl("fields.jsonl", &inventory.rust.fields)?;
    artifacts.insert_jsonl("functions.jsonl", &inventory.rust.functions)?;
    artifacts.insert_jsonl("contracts.jsonl", &inventory.rust.contracts)?;
    artifacts.insert_jsonl("serialization.jsonl", &inventory.rust.serialization)?;
    artifacts.insert_jsonl(
        "traversal-and-reference-edges.jsonl",
        &inventory.rust.edges,
    )?;
    artifacts.insert_jsonl("lowering-sites.jsonl", &inventory.rust.lowering_sites)?;
    artifacts.insert_jsonl(
        "renderer-parser-consumers.jsonl",
        &inventory.rust.consumers,
    )?;
    artifacts.insert_jsonl("tests-and-fixtures.jsonl", &inventory.tests)?;
    artifacts.insert_jsonl("python-source-census.jsonl", &inventory.python)?;
    artifacts.insert_jsonl(
        "executable-consumers.jsonl",
        &inventory.executable_consumers,
    )?;
    artifacts.insert_jsonl("migration-ledger.jsonl", &ledger)?;
    let artifact_names = artifacts
        .artifacts
        .keys()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let required_names = JSONL_ARTIFACTS.iter().copied().collect::<BTreeSet<_>>();
    if artifact_names != required_names {
        bail!("generated artifact names do not match the 15 required JSONL artifacts");
    }
    let manifest_artifacts = artifacts.manifest_artifacts();
    let record_counts = manifest_artifacts
        .iter()
        .map(|artifact| (artifact.name.clone(), artifact.record_count))
        .collect::<BTreeMap<_, _>>();
    let manifest = new!(Manifest {
        commit: tree.commit.clone(),
        tree: tree.tree.clone(),
        extractor_version: EXTRACTOR_VERSION.to_owned(),
        generation_command: format!(
            "cargo run -r -p xtask-full -- semantic-source-inventory --commit {}",
            tree.commit
        ),
        classification_catalog: new!(ManifestClassificationCatalog {
            path: CLASSIFICATION_CATALOG_PATH.to_owned(),
            sha256: loaded_catalog.sha256.clone(),
            byte_length: loaded_catalog.byte_length,
            rule_hits,
            issue_580_current_source_rows: 0,
        }),
        source_files: inventory
            .files
            .iter()
            .map(|file| {
                new!(ManifestSourceFile {
                    path: file.source.path.clone(),
                    git_blob: file.git_blob.clone(),
                    sha256: file.sha256.clone(),
                    byte_length: file.byte_length,
                })
            })
            .collect(),
        artifacts: manifest_artifacts,
        record_counts,
        legacy_haskell_file_hashes: BTreeMap::from([
            (
                "WordSignatures.hs".to_owned(),
                "fdb4457bcec2b6b216be39a59ef8b6e9515662dfe76a5e3495b47f3adc3fed0c"
                    .to_owned(),
            ),
            (
                "SlotType.hs".to_owned(),
                "e8372edb3884de8505fc7e4b525cb00c4383b347e95a5db762dc7fcd86d44079"
                    .to_owned(),
            ),
        ]),
        legacy_provenance_facts: vec![
            "old pilno SlotEntity is not v5 authority".to_owned(),
            "old mensi does not justify narrowing TOP".to_owned(),
            "SlotEntity had a nominal rendering".to_owned(),
            "legacy Eventuality-to-Entity compatibility explains the old coarse mapping"
                .to_owned(),
        ],
    });
    Ok(new!(GeneratedInventory { artifacts, manifest }))
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|paths| !paths.is_empty()))]
fn discover_rust_paths(tree: &GitTree) -> Result<BTreeMap<String, FileClass>> {
    let mut paths = BTreeMap::new();
    for path in tree.entries.keys() {
        let class = if path.starts_with("crates/jbotci-semantics/src/")
            && path.ends_with(".rs")
        {
            if path.starts_with("crates/jbotci-semantics/src/generated_builder/") {
                FileClass::RustLowering
            } else if path == "crates/jbotci-semantics/src/render.rs" {
                FileClass::RustRenderer
            } else {
                FileClass::RustSemanticModel
            }
        } else if RUST_CONSUMER_PATHS.contains(&path.as_str()) {
            FileClass::RustConsumer
        } else if RUST_VALIDATION_PATHS.contains(&path.as_str()) {
            FileClass::RustValidationTool
        } else {
            continue;
        };
        paths.insert(path.clone(), class);
    }
    for required in RUST_CONSUMER_PATHS.iter().chain(RUST_VALIDATION_PATHS) {
        if !paths.contains_key(*required) {
            bail!("pinned tree is missing required Rust inventory source `{required}`");
        }
    }
    Ok(paths)
}

#[requires(true)]
#[ensures(ret.iter().all(|path| path.ends_with(".py") || path.ends_with(".pyi")))]
fn discover_python_paths(tree: &GitTree) -> BTreeSet<String> {
    tree.entries
        .keys()
        .filter(|path| path.ends_with(".py") || path.ends_with(".pyi"))
        .filter(|path| {
            !VENDORED_PYTHON_PREFIXES
                .iter()
                .any(|prefix| path.starts_with(prefix))
        })
        .cloned()
        .collect()
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn validate_python_catalog(
    discovered: &BTreeSet<String>,
    catalog: &ClassificationCatalog,
) -> Result<()> {
    let configured = catalog
        .python_files
        .iter()
        .map(|rule| rule.path.clone())
        .collect::<BTreeSet<_>>();
    if discovered != &configured {
        let missing = discovered.difference(&configured).collect::<Vec<_>>();
        let unused = configured.difference(discovered).collect::<Vec<_>>();
        bail!(
            "Python classification catalog is not one-to-one with pinned discovery; missing={missing:?}, unused={unused:?}"
        );
    }
    Ok(())
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn extract_fixtures(tree: &GitTree, inventory: &mut Inventory) -> Result<()> {
    let fixture_paths = tree
        .entries
        .keys()
        .filter(|path| path.starts_with("tests/fixtures/") && path.ends_with(".toml"))
        .cloned()
        .collect::<Vec<_>>();
    for batch in fixture_paths.chunks(FIXTURE_BATCH_SIZE) {
        let batch_paths = batch.iter().cloned().collect::<BTreeSet<_>>();
        let blobs = tree.read_blobs(&batch_paths)?;
        for path in batch {
            let bytes = &blobs[path];
            let source = utf8_source(path, bytes)?;
            let value: toml::Value = toml::from_str(source)
                .with_context(|| format!("parsing pinned fixture `{path}`"))?;
            let profile = path.starts_with("tests/fixtures/profiles/");
            let semantic_references = value
                .get("expectations")
                .and_then(|value| value.get("semantics"))
                .and_then(|value| value.get("refs"))
                .is_some();
            let tersmu_output = value
                .get("expectations")
                .and_then(|value| value.get("output"))
                .and_then(|value| value.get("tersmu"))
                .is_some();
            if !profile && !semantic_references && !tersmu_output {
                continue;
            }
            add_file_record(
                &mut inventory.files,
                tree,
                path,
                bytes,
                if profile {
                    FileClass::SemanticFixtureConfiguration
                } else {
                    FileClass::SemanticFixture
                },
            )?;
            let identity = SourceMap::new(path, source).whole_file();
            let kind = if profile {
                TestFixtureKind::FixtureProfile
            } else if semantic_references && tersmu_output {
                TestFixtureKind::CombinedSemanticFixture
            } else if semantic_references {
                TestFixtureKind::SemanticReferenceFixture
            } else {
                TestFixtureKind::TersmuOutputFixture
            };
            let name = value
                .get("id")
                .and_then(toml::Value::as_str)
                .unwrap_or(path)
                .to_owned();
            inventory.tests.push(new!(TestFixtureRecord {
                id: record_id("fixture", &identity),
                source: identity,
                name,
                kind,
                owner_id: None,
                semantic_reference_expectation: semantic_references,
                tersmu_output_expectation: tersmu_output,
            }));
        }
    }
    Ok(())
}

#[requires(!path.is_empty())]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn add_file_record(
    records: &mut Vec<FileRecord>,
    tree: &GitTree,
    path: &str,
    bytes: &[u8],
    class: FileClass,
) -> Result<()> {
    if bytes.is_empty() {
        bail!("inventoried pinned source `{path}` is empty");
    }
    let source = utf8_source(path, bytes)?;
    let identity = SourceMap::new(path, source).whole_file();
    let entry = tree
        .entries
        .get(path)
        .with_context(|| format!("pinned tree is missing inventoried path `{path}`"))?;
    records.push(new!(FileRecord {
        id: record_id("file", &identity),
        source: identity,
        class,
        git_blob: entry.object_id.clone(),
        sha256: sha256_hex(bytes),
        byte_length: bytes.len(),
    }));
    Ok(())
}

#[requires(!path.is_empty())]
#[ensures(ret.as_ref().is_ok_and(|source| source.len() == bytes.len()))]
fn utf8_source<'bytes>(path: &str, bytes: &'bytes [u8]) -> Result<&'bytes str> {
    std::str::from_utf8(bytes).with_context(|| format!("inventoried source `{path}` is not UTF-8"))
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn require_paths(tree: &GitTree, paths: &BTreeSet<String>) -> Result<()> {
    let missing = paths
        .iter()
        .filter(|path| !tree.entries.contains_key(*path))
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        bail!("pinned tree is missing required inventory sources: {missing:?}");
    }
    Ok(())
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn executable_consumers(
    inventory: &Inventory,
    rules: &[ExecutableConsumerRule],
) -> Result<Vec<ExecutableConsumerRecord>> {
    let files = inventory
        .files
        .iter()
        .map(|file| (file.source.path.as_str(), file))
        .collect::<BTreeMap<_, _>>();
    let discovered_python_consumers = inventory
        .python
        .iter()
        .filter(|record| record.semantic_consumer)
        .map(|record| record.source.path.as_str())
        .collect::<BTreeSet<_>>();
    let discovered_rust_consumers = inventory
        .files
        .iter()
        .filter(|record| {
            matches!(
                record.class,
                FileClass::RustConsumer | FileClass::RustValidationTool
            )
        })
        .map(|record| record.source.path.as_str())
        .collect::<BTreeSet<_>>();
    let configured_rust_consumers = rules
        .iter()
        .filter(|rule| rule.language == ConsumerLanguage::Rust)
        .map(|rule| rule.path.as_str())
        .collect::<BTreeSet<_>>();
    if discovered_rust_consumers != configured_rust_consumers {
        bail!(
            "Rust executable-consumer catalog is not one-to-one with pinned consumer discovery; discovered={discovered_rust_consumers:?}, configured={configured_rust_consumers:?}"
        );
    }
    let configured_python_consumers = rules
        .iter()
        .filter(|rule| rule.language == ConsumerLanguage::Python)
        .map(|rule| rule.path.as_str())
        .collect::<BTreeSet<_>>();
    if discovered_python_consumers != configured_python_consumers {
        bail!(
            "Python executable-consumer catalog is not one-to-one with syntax-aware consumer discovery; discovered={discovered_python_consumers:?}, configured={configured_python_consumers:?}"
        );
    }
    let mut records = Vec::new();
    for rule in rules {
        let file = files.get(rule.path.as_str()).with_context(|| {
            format!(
                "executable-consumer catalog path `{}` is not an inventoried source",
                rule.path
            )
        })?;
        records.push(new!(ExecutableConsumerRecord {
            id: record_id("executable-consumer", &file.source),
            source: file.source.clone(),
            language: rule.language,
            consumer_kind: rule.consumer_kind.clone(),
            cli_formats: rule.cli_formats.clone(),
            cli_default: rule.cli_default.clone(),
            error_surfaces: rule.error_surfaces.clone(),
            fixture_reads: rule.fixture_reads.clone(),
            fixture_writes: rule.fixture_writes.clone(),
            hard_coded_tokens: rule.hard_coded_tokens.clone(),
            base_revision_assumptions: rule.base_revision_assumptions.clone(),
            current_revision_assumptions: rule.current_revision_assumptions.clone(),
            binary_assumptions: rule.binary_assumptions.clone(),
            exact_invocation: rule.exact_invocation.clone(),
            allowlist_count_hash_pins: rule.allowlist_count_hash_pins.clone(),
            downstream_migration_owners: rule.downstream_migration_owners.clone(),
            lifecycle_disposition: rule.lifecycle_disposition,
            replacement_gate: rule.replacement_gate.clone(),
            preserved_witnesses: rule.preserved_witnesses.clone(),
        }));
    }
    Ok(records)
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn validate_planning_reference_python(records: &[PythonCensusRecord]) -> Result<()> {
    let by_path = records
        .iter()
        .map(|record| (record.source.path.as_str(), record))
        .collect::<BTreeMap<_, _>>();
    let planning = PLANNING_REFERENCE_PYTHON_PATHS
        .iter()
        .map(|path| {
            by_path
                .get(path)
                .copied()
                .with_context(|| format!("planning-reference Python path `{path}` is absent"))
        })
        .collect::<Result<Vec<_>>>()?;
    let consumers = planning
        .iter()
        .filter(|record| record.semantic_consumer)
        .count();
    let tersmu_sites = planning
        .iter()
        .flat_map(|record| &record.occurrences)
        .filter(|occurrence| occurrence.token == "tersmu")
        .count();
    let tanru_links = planning
        .iter()
        .flat_map(|record| &record.occurrences)
        .filter(|occurrence| occurrence.token == "TanruLink")
        .collect::<Vec<_>>();
    if planning.len() != 19 || consumers != 11 || tersmu_sites != 207 {
        bail!(
            "planning-reference Python witness drifted: files={}, consumers={consumers}, tersmu_sites={tersmu_sites}; expected 19/11/207",
            planning.len()
        );
    }
    let [tanru_link] = tanru_links.as_slice() else {
        bail!(
            "planning-reference Python witness has {} TanruLink sites; expected exactly one",
            tanru_links.len()
        );
    };
    if tanru_link.source.path != "scripts/verify_issue_374_fixture_migration.py" {
        bail!("TanruLink planning-reference witness moved to an unexpected path");
    }
    Ok(())
}

#[requires(true)]
#[ensures(true)]
fn sort_inventory(inventory: &mut Inventory) {
    macro_rules! sort_by_source {
        ($records:expr) => {
            $records.sort_by(|left, right| {
                left.source
                    .cmp(&right.source)
                    .then_with(|| left.id.cmp(&right.id))
            });
        };
    }
    sort_by_source!(inventory.files);
    sort_by_source!(inventory.rust.modules);
    sort_by_source!(inventory.rust.types);
    sort_by_source!(inventory.rust.variants);
    sort_by_source!(inventory.rust.fields);
    sort_by_source!(inventory.rust.functions);
    sort_by_source!(inventory.rust.contracts);
    sort_by_source!(inventory.rust.serialization);
    sort_by_source!(inventory.rust.edges);
    sort_by_source!(inventory.rust.lowering_sites);
    sort_by_source!(inventory.rust.consumers);
    sort_by_source!(inventory.tests);
    sort_by_source!(inventory.python);
    sort_by_source!(inventory.executable_consumers);
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn validate_inventory(inventory: &Inventory) -> Result<()> {
    unique_records("files", inventory.files.iter().map(|record| record.id.as_str()))?;
    unique_records(
        "modules",
        inventory.rust.modules.iter().map(|record| record.id.as_str()),
    )?;
    unique_records(
        "types",
        inventory.rust.types.iter().map(|record| record.id.as_str()),
    )?;
    unique_records(
        "variants",
        inventory.rust.variants.iter().map(|record| record.id.as_str()),
    )?;
    unique_records(
        "fields",
        inventory.rust.fields.iter().map(|record| record.id.as_str()),
    )?;
    unique_records(
        "functions",
        inventory.rust.functions.iter().map(|record| record.id.as_str()),
    )?;
    unique_records(
        "contracts",
        inventory.rust.contracts.iter().map(|record| record.id.as_str()),
    )?;
    unique_records(
        "serialization",
        inventory
            .rust
            .serialization
            .iter()
            .map(|record| record.id.as_str()),
    )?;
    unique_records(
        "edges",
        inventory.rust.edges.iter().map(|record| record.id.as_str()),
    )?;
    unique_records(
        "lowering sites",
        inventory
            .rust
            .lowering_sites
            .iter()
            .map(|record| record.id.as_str()),
    )?;
    unique_records(
        "renderer/parser consumers",
        inventory.rust.consumers.iter().map(|record| record.id.as_str()),
    )?;
    unique_records("tests", inventory.tests.iter().map(|record| record.id.as_str()))?;
    unique_records(
        "Python census",
        inventory.python.iter().map(|record| record.id.as_str()),
    )?;
    unique_records(
        "executable consumers",
        inventory
            .executable_consumers
            .iter()
            .map(|record| record.id.as_str()),
    )?;
    let type_ids = inventory
        .rust
        .types
        .iter()
        .map(|record| record.id.as_str())
        .collect::<BTreeSet<_>>();
    let module_ids = inventory
        .rust
        .modules
        .iter()
        .map(|record| record.id.as_str())
        .collect::<BTreeSet<_>>();
    let function_ids = inventory
        .rust
        .functions
        .iter()
        .map(|record| record.id.as_str())
        .collect::<BTreeSet<_>>();
    for module in &inventory.rust.modules {
        if module
            .parent
            .as_deref()
            .is_some_and(|parent| !module_ids.contains(parent))
        {
            bail!("module `{}` references absent parent {:?}", module.id, module.parent);
        }
    }
    for semantic_type in &inventory.rust.types {
        if !module_ids.contains(semantic_type.module_id.as_str()) {
            bail!(
                "type `{}` references absent module `{}`",
                semantic_type.id,
                semantic_type.module_id
            );
        }
    }
    let variant_ids = inventory
        .rust
        .variants
        .iter()
        .map(|record| record.id.as_str())
        .collect::<BTreeSet<_>>();
    for variant in &inventory.rust.variants {
        if !type_ids.contains(variant.type_id.as_str()) {
            bail!("variant `{}` references absent type `{}`", variant.id, variant.type_id);
        }
    }
    for field in &inventory.rust.fields {
        if !type_ids.contains(field.owner_id.as_str())
            && !variant_ids.contains(field.owner_id.as_str())
        {
            bail!("field `{}` references absent owner `{}`", field.id, field.owner_id);
        }
    }
    for function in &inventory.rust.functions {
        if !module_ids.contains(function.module_id.as_str()) {
            bail!(
                "function `{}` references absent module `{}`",
                function.id,
                function.module_id
            );
        }
    }
    for contract in &inventory.rust.contracts {
        require_declaration_owner(
            &inventory.rust,
            &contract.id,
            &contract.owner_id,
            "contract",
        )?;
    }
    for serialization in &inventory.rust.serialization {
        require_declaration_owner(
            &inventory.rust,
            &serialization.id,
            &serialization.owner_id,
            "serialization record",
        )?;
    }
    for edge in &inventory.rust.edges {
        require_declaration_owner(&inventory.rust, &edge.id, &edge.owner_id, "edge")?;
    }
    for lowering in &inventory.rust.lowering_sites {
        if !function_ids.contains(lowering.function_id.as_str()) {
            bail!(
                "lowering site `{}` references absent function `{}`",
                lowering.id,
                lowering.function_id
            );
        }
    }
    for consumer in &inventory.rust.consumers {
        if !function_ids.contains(consumer.owner_id.as_str()) {
            bail!(
                "renderer/parser consumer `{}` references absent function `{}`",
                consumer.id,
                consumer.owner_id
            );
        }
    }
    for test in &inventory.tests {
        if test
            .owner_id
            .as_deref()
            .is_some_and(|owner| !function_ids.contains(owner))
        {
            bail!("test `{}` references absent function owner {:?}", test.id, test.owner_id);
        }
    }
    let file_paths = inventory
        .files
        .iter()
        .map(|record| record.source.path.as_str())
        .collect::<BTreeSet<_>>();
    let file_lengths = inventory
        .files
        .iter()
        .map(|record| (record.source.path.as_str(), record.byte_length))
        .collect::<BTreeMap<_, _>>();
    for file in &inventory.files {
        if file.source.byte_start != 0 || file.source.byte_end != file.byte_length {
            bail!("file `{}` does not span its complete pinned bytes", file.id);
        }
    }
    macro_rules! require_file_records {
        ($records:expr, $kind:literal) => {
            for record in $records {
                if !file_paths.contains(record.source.path.as_str()) {
                    bail!(
                        "{} `{}` references absent file `{}`",
                        $kind,
                        record.id,
                        record.source.path
                    );
                }
                if record.source.byte_end > file_lengths[record.source.path.as_str()] {
                    bail!(
                        "{} `{}` has a source span outside file `{}`",
                        $kind,
                        record.id,
                        record.source.path
                    );
                }
            }
        };
    }
    require_file_records!(&inventory.rust.modules, "module");
    require_file_records!(&inventory.rust.types, "type");
    require_file_records!(&inventory.rust.variants, "variant");
    require_file_records!(&inventory.rust.fields, "field");
    require_file_records!(&inventory.rust.functions, "function");
    require_file_records!(&inventory.rust.contracts, "contract");
    require_file_records!(&inventory.rust.serialization, "serialization record");
    require_file_records!(&inventory.rust.edges, "edge");
    require_file_records!(&inventory.rust.lowering_sites, "lowering site");
    require_file_records!(&inventory.rust.consumers, "renderer/parser consumer");
    require_file_records!(&inventory.tests, "test/fixture");
    require_file_records!(&inventory.python, "Python census record");
    require_file_records!(&inventory.executable_consumers, "executable consumer");
    for record in &inventory.python {
        if !file_paths.contains(record.source.path.as_str())
            || record.semantic_consumer
                != record
                    .occurrences
                    .iter()
                    .any(|occurrence| occurrence.token == "tersmu")
        {
            bail!("Python census record `{}` fails its file/occurrence join", record.id);
        }
        for occurrence in &record.occurrences {
            if occurrence.source.byte_end > file_lengths[record.source.path.as_str()] {
                bail!(
                    "Python occurrence `{}` in `{}` is outside its pinned source",
                    occurrence.token,
                    record.source.path
                );
            }
        }
    }
    Ok(())
}

#[requires(!record_id.is_empty() && !owner_id.is_empty() && !kind.is_empty())]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn require_declaration_owner(
    inventory: &RustInventory,
    record_id: &str,
    owner_id: &str,
    kind: &str,
) -> Result<()> {
    if !inventory.declaration_owners.contains(owner_id) {
        bail!("{kind} `{record_id}` references absent declaration owner `{owner_id}`");
    }
    Ok(())
}

#[requires(!category.is_empty())]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn unique_records<'id>(category: &str, ids: impl Iterator<Item = &'id str>) -> Result<()> {
    let mut seen = BTreeSet::new();
    for id in ids {
        if !seen.insert(id) {
            bail!("{category} contains duplicate stable id `{id}`");
        }
    }
    if seen.is_empty() {
        bail!("required inventory category `{category}` is empty");
    }
    Ok(())
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|records| !records.is_empty()))]
fn classifiable_records(inventory: &Inventory) -> Result<Vec<ClassifiableRecord>> {
    let mut records = Vec::new();
    let mut owner_types = BTreeMap::<String, String>::new();
    for record in &inventory.rust.types {
        owner_types.insert(record.id.clone(), record.name.clone());
    }
    for record in &inventory.rust.variants {
        if let Some(owner) = owner_types.get(&record.type_id).cloned() {
            owner_types.insert(record.id.clone(), owner);
        }
    }
    for record in &inventory.rust.fields {
        if let Some(owner) = owner_types.get(&record.owner_id).cloned() {
            owner_types.insert(record.id.clone(), owner);
        }
    }
    for record in &inventory.rust.functions {
        if let Some(owner) = &record.owner_type {
            owner_types.insert(record.id.clone(), owner.clone());
        }
    }
    for record in &inventory.rust.contracts {
        if let Some(owner) = owner_types.get(&record.owner_id).cloned() {
            owner_types.insert(record.id.clone(), owner);
        }
    }
    for record in &inventory.rust.serialization {
        if let Some(owner) = owner_types.get(&record.owner_id).cloned() {
            owner_types.insert(record.id.clone(), owner);
        }
    }
    for record in &inventory.rust.edges {
        if let Some(owner) = owner_types.get(&record.owner_id).cloned() {
            owner_types.insert(record.id.clone(), owner);
        }
    }
    for record in &inventory.rust.lowering_sites {
        if let Some(owner) = owner_types.get(&record.function_id).cloned() {
            owner_types.insert(record.id.clone(), owner);
        }
    }
    for record in &inventory.rust.consumers {
        if let Some(owner) = owner_types.get(&record.owner_id).cloned() {
            owner_types.insert(record.id.clone(), owner);
        }
    }
    for record in &inventory.tests {
        if let Some(owner_id) = &record.owner_id {
            if let Some(owner) = owner_types.get(owner_id).cloned() {
                owner_types.insert(record.id.clone(), owner);
            }
        }
    }
    macro_rules! append_records {
        ($items:expr, $family:expr) => {
            records.extend($items.iter().map(|record| {
                new!(ClassifiableRecord {
                    id: record.id.clone(),
                    family: $family,
                    path: record.source.path.clone(),
                    owner_type: owner_types.get(&record.id).cloned(),
                })
            }));
        };
    }
    append_records!(inventory.files, RecordFamily::File);
    append_records!(inventory.rust.modules, RecordFamily::Module);
    append_records!(inventory.rust.types, RecordFamily::Type);
    append_records!(inventory.rust.variants, RecordFamily::Variant);
    append_records!(inventory.rust.fields, RecordFamily::Field);
    append_records!(inventory.rust.functions, RecordFamily::Function);
    append_records!(inventory.rust.contracts, RecordFamily::Contract);
    append_records!(inventory.rust.serialization, RecordFamily::Serialization);
    append_records!(
        inventory.rust.edges,
        RecordFamily::TraversalAndReferenceEdge
    );
    append_records!(inventory.rust.lowering_sites, RecordFamily::LoweringSite);
    append_records!(
        inventory.rust.consumers,
        RecordFamily::RendererParserConsumer
    );
    append_records!(inventory.tests, RecordFamily::TestAndFixture);
    append_records!(inventory.python, RecordFamily::PythonSource);
    append_records!(
        inventory.executable_consumers,
        RecordFamily::ExecutableConsumer
    );
    unique_records("classifiable records", records.iter().map(|record| record.id.as_str()))?;
    Ok(records)
}

#[requires(expected.is_absolute() && actual.is_absolute())]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn compare_directories(expected: &Path, actual: &Path) -> Result<()> {
    let mut names = vec!["manifest.json"];
    names.extend(JSONL_ARTIFACTS.iter().copied());
    for name in names {
        let expected_bytes = fs::read(expected.join(name))
            .with_context(|| format!("reading regenerated `{name}`"))?;
        let actual_bytes = fs::read(actual.join(name))
            .with_context(|| format!("reading committed `{name}`"))?;
        if expected_bytes != actual_bytes {
            bail!("committed inventory artifact `{name}` differs from regeneration");
        }
    }
    let actual_names = fs::read_dir(actual)
        .with_context(|| format!("listing committed inventory `{}`", actual.display()))?
        .map(|entry| {
            entry
                .map(|entry| entry.file_name().to_string_lossy().into_owned())
                .context("reading committed inventory directory entry")
        })
        .collect::<Result<BTreeSet<_>>>()?;
    let mut required_names = JSONL_ARTIFACTS
        .iter()
        .map(|name| (*name).to_owned())
        .collect::<BTreeSet<_>>();
    required_names.insert("manifest.json".to_owned());
    if actual_names != required_names {
        bail!(
            "committed inventory directory contains a missing or extra artifact: actual={actual_names:?} required={required_names:?}"
        );
    }
    Ok(())
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|path| path.is_absolute()))]
fn repository_root() -> Result<PathBuf> {
    let current = env::current_dir().context("reading current directory")?;
    let output = Command::new("git")
        .current_dir(&current)
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .context("resolving repository root")?;
    if !output.status.success() {
        bail!(
            "`git rev-parse --show-toplevel` failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    let path = std::str::from_utf8(&output.stdout)
        .context("repository root path is not UTF-8")?
        .trim();
    let path = PathBuf::from(path)
        .canonicalize()
        .with_context(|| format!("canonicalizing repository root `{path}`"))?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[requires(directory.is_absolute())]
    #[ensures(true)]
    fn write_complete_artifact_directory(directory: &Path) {
        fs::create_dir_all(directory).expect("create artifact test directory");
        fs::write(directory.join("manifest.json"), b"manifest\n")
            .expect("write test manifest");
        for name in JSONL_ARTIFACTS {
            fs::write(directory.join(name), format!("{name}\n"))
                .expect("write test JSONL artifact");
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn directory_comparison_rejects_drift_and_extra_artifacts() {
        let temporary = tempfile::tempdir().expect("artifact comparison temporary directory");
        let expected = temporary.path().join("expected");
        let actual = temporary.path().join("actual");
        write_complete_artifact_directory(&expected);
        write_complete_artifact_directory(&actual);
        compare_directories(&expected, &actual).expect("identical directories compare equal");

        fs::write(actual.join(JSONL_ARTIFACTS[0]), b"drift\n")
            .expect("write deterministic drift witness");
        let drift = compare_directories(&expected, &actual)
            .expect_err("changed artifact bytes must fail check behavior");
        assert!(drift.to_string().contains("differs from regeneration"));

        fs::write(
            actual.join(JSONL_ARTIFACTS[0]),
            format!("{}\n", JSONL_ARTIFACTS[0]),
        )
        .expect("restore artifact bytes");
        fs::write(actual.join("unexpected.jsonl"), b"extra\n")
            .expect("write extra artifact witness");
        let extra = compare_directories(&expected, &actual)
            .expect_err("extra artifacts must fail check behavior");
        assert!(extra.to_string().contains("missing or extra artifact"));
    }
}
