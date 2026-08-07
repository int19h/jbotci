//! Structural and reproducibility tests for the current smusni-v0 candidate bundle.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

#[allow(unused_imports)]
use bityzba::{ensures, requires};
use sha2::{Digest, Sha256};

#[path = "../codegen/smusni_v0_bundle.rs"]
mod smusni_v0_bundle;
#[path = "../codegen/smusni_v0_completeness.rs"]
mod smusni_v0_completeness;
#[path = "../codegen/smusni_v0_dispositions.rs"]
mod smusni_v0_dispositions;
#[path = "../codegen/smusni_v0_kernel.rs"]
mod smusni_v0_kernel;
#[path = "../codegen/smusni_v0_surface.rs"]
mod smusni_v0_surface;

use smusni_v0_bundle::{BundleErrorKind, BundleMode, BundlePaths, BundleSnapshot, DispositionSeed};
use smusni_v0_kernel::sexpr::datum::{Datum, parse_document};

const OBLIQUE: &[u8] = include_bytes!("../data/smusni-v0/sources/lojban-org/oblique_keywords.txt");
const WITNESSES: &str = include_str!("../data/smusni-v0/sources/must-compact-witnesses.txt");
const REGISTRY_SOURCE: &str = include_str!("../data/smusni-v0/sources/registry-source.toml");
const SPEC: &[u8] = include_bytes!("../../../docs/smusni/spec.md");
const RETAINED_SPEC: &[u8] = include_bytes!("../data/smusni-v0/sources/smusni/spec.md");

#[requires(true)]
#[ensures(ret.ends_with("crates/jbotci-semantics"))]
fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[requires(true)]
#[ensures(ret.join("crates/jbotci-semantics").is_dir())]
fn repository_root() -> PathBuf {
    manifest_dir()
        .parent()
        .and_then(Path::parent)
        .expect("semantics crate has a workspace root")
        .to_path_buf()
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn dispositions() -> Vec<DispositionSeed> {
    smusni_v0_dispositions::projected_dispositions()
}

#[requires(path.starts_with("registry/") && path.ends_with(".jsonl"))]
#[ensures(!ret.is_empty())]
fn jsonl_rows(path: &str) -> Vec<serde_json::Value> {
    let bytes = fs::read(
        manifest_dir()
            .join(smusni_v0_bundle::BUNDLE_ROOT)
            .join(path),
    )
    .expect("generated bundle table");
    assert!(bytes.ends_with(b"\n"));
    assert!(!bytes.contains(&b'\r'));
    bytes
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_slice(line).expect("one JCS object per line"))
        .collect()
}

#[requires(true)]
#[ensures(ret.root.ends_with(smusni_v0_bundle::BUNDLE_ROOT))]
fn bundle_paths() -> BundlePaths {
    BundlePaths::for_manifest_dir(
        &manifest_dir(),
        smusni_v0_bundle::scratch_dir("bundle-tests").join("unused-policies.rs"),
    )
}

#[requires(true)]
#[ensures(ret.artifacts.len() == 12)]
fn snapshot() -> BundleSnapshot {
    smusni_v0_bundle::mint_snapshot(&bundle_paths(), &dispositions()).expect("current inputs mint")
}

#[requires(snapshot.artifacts.contains_key(path))]
#[ensures(snapshot.artifacts.contains_key(path))]
fn mutate_first_row(
    snapshot: &mut BundleSnapshot,
    path: &str,
    mutation: impl FnOnce(&mut serde_json::Map<String, serde_json::Value>),
) {
    let bytes = &snapshot.artifacts[path];
    let mut rows = bytes
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_slice::<serde_json::Value>(line).unwrap())
        .collect::<Vec<_>>();
    mutation(
        rows[0]
            .as_object_mut()
            .expect("every top-level registry row is an object"),
    );
    let mut replacement = Vec::new();
    for row in rows {
        replacement.extend(serde_json::to_vec(&row).unwrap());
        replacement.push(b'\n');
    }
    snapshot.artifacts.insert(path.to_owned(), replacement);
}

#[requires(!before.is_empty() && source.matches(before).count() == 1)]
#[ensures(ret.matches(after).count() >= 1)]
fn replace_once(source: &str, before: &str, after: &str) -> String {
    source.replacen(before, after, 1)
}

#[requires(!source.is_empty())]
#[ensures(true)]
fn registry_type_parameter_names(source: &str) -> BTreeSet<String> {
    #[requires(true)]
    #[ensures(true)]
    fn visit(datum: &Datum, names: &mut BTreeSet<String>) {
        if datum.form_head() == Some("TypeParam") {
            let items = datum.as_list().expect("TypeParam form is a list");
            assert_eq!(items.len(), 2);
            names.insert(
                items[1]
                    .as_string()
                    .expect("TypeParam name is a string")
                    .to_owned(),
            );
            return;
        }
        if let Some(items) = datum.as_list() {
            for item in items {
                visit(item, names);
            }
        }
    }

    let datum = parse_document(source).expect("registry datum is canonical");
    let mut names = BTreeSet::new();
    visit(&datum, &mut names);
    names
}

#[requires(!source.is_empty())]
#[ensures(true)]
fn registry_datum_contains_bare_t(source: &str) -> bool {
    #[requires(true)]
    #[ensures(true)]
    fn contains(datum: &Datum) -> bool {
        datum.as_atom() == Some("T")
            || datum
                .as_list()
                .is_some_and(|items| items.iter().any(contains))
    }

    contains(&parse_document(source).expect("registry datum is canonical"))
}

#[test]
#[requires(true)]
#[ensures(true)]
fn pinned_sources_and_candidate_witness_registry_are_exact() {
    assert_eq!(OBLIQUE.len(), 79_293);
    assert_eq!(OBLIQUE.iter().filter(|byte| **byte == b'\n').count(), 3_542);
    assert_eq!(
        format!("{:x}", Sha256::digest(OBLIQUE)),
        "355786cfd049063c92514fac2d417fc4966df7749dc17d7cfb49bd903fb6a2cb"
    );
    assert_eq!(WITNESSES.lines().count(), 18);
    assert!(WITNESSES.ends_with('\n'));
    assert!(!REGISTRY_SOURCE.contains("canonical_definition"));
    // The specification is dual-homed: a source distribution carries no `docs/`
    // tree, so the bundle keeps its own copy for prelude extraction. State the
    // two copies' equality directly rather than pinning either one to a digest.
    assert_eq!(SPEC, RETAINED_SPEC);
}

/// Every byte this build compares must survive checkout unchanged.
///
/// Git decides end-of-line conversion per path, so two copies of one document
/// that disagree about the `text` attribute drift apart on a Windows checkout
/// and nowhere else — which is exactly how the pinned dictionary snapshot broke
/// while every Linux job stayed green. The bundle's own inputs are in the same
/// position, because the build recomputes their digests and compares its
/// generated tables byte for byte. Deriving the inventory from the generator
/// rather than from `.gitattributes` keeps the policy closed: a new generator
/// input that nothing marks fails here.
#[test]
#[requires(true)]
#[ensures(true)]
fn every_compared_byte_is_checked_out_verbatim() {
    let root = repository_root();
    let bundle_dir = Path::new("crates/jbotci-semantics").join(smusni_v0_bundle::BUNDLE_ROOT);
    let mut paths = smusni_v0_bundle::bundle_rerun_paths()
        .into_iter()
        .map(|relative| {
            bundle_dir
                .join(relative)
                .to_str()
                .expect("repository paths are UTF-8")
                .to_owned()
        })
        .collect::<BTreeSet<_>>();
    paths.extend(smusni_v0_bundle::repository_rerun_paths());
    paths.insert("docs/smusni/spec.md".to_owned());
    assert!(paths.len() >= 21, "inventory shrank unexpectedly");

    let output = std::process::Command::new("git")
        .current_dir(&root)
        .args(["check-attr", "text", "--"])
        .args(&paths)
        .output()
        .expect("run git check-attr");
    assert!(
        output.status.success(),
        "git check-attr failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let reported = String::from_utf8(output.stdout).expect("git reports UTF-8 paths");
    let mut checked = 0;
    for line in reported.lines() {
        // `-text` is the only setting that survives every platform's checkout
        // verbatim; `git check-attr` spells it `unset`.
        let (path, value) = line
            .rsplit_once(": ")
            .expect("git check-attr reports `<path>: text: <value>`");
        assert_eq!(value, "unset", "{path} is not checked out verbatim");
        checked += 1;
    }
    assert_eq!(checked, paths.len());
}

/// The package that owns `relative`: the nearest ancestor directory holding a
/// `Cargo.toml`, which is exactly the unit `cargo package` operates on.
#[requires(!relative.is_empty())]
#[ensures(repository_root().join(&ret).join("Cargo.toml").is_file())]
fn owning_package_dir(relative: &str) -> String {
    let root = repository_root();
    let mut candidate = PathBuf::from(relative);
    while candidate.pop() {
        if root.join(&candidate).join("Cargo.toml").is_file() {
            return candidate
                .to_str()
                .expect("repository paths are UTF-8")
                .to_owned();
        }
    }
    String::new()
}

/// Files listed by `cargo package` for one package, relative to that package.
#[requires(true)]
#[ensures(!ret.is_empty())]
fn cargo_package_list(package_dir: &str) -> BTreeSet<String> {
    let root = repository_root();
    let manifest = root.join(package_dir).join("Cargo.toml");
    let output = std::process::Command::new(env!("CARGO"))
        .current_dir(&root)
        // A private target directory keeps this nested invocation off the
        // outer test run's build lock.
        .env(
            "CARGO_TARGET_DIR",
            smusni_v0_bundle::scratch_dir("package-list"),
        )
        .args([
            "package",
            "--list",
            "--locked",
            "--offline",
            "--allow-dirty",
        ])
        .arg("--manifest-path")
        .arg(&manifest)
        .output()
        .expect("run cargo package --list");
    assert!(
        output.status.success(),
        "cargo package --list failed for {}: {}",
        manifest.display(),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .expect("cargo package --list emits UTF-8")
        .lines()
        .map(str::to_owned)
        .collect()
}

/// Every file `build.rs` reads must survive `cargo package`, because the Python
/// source distribution rebuilds the workspace from the extracted archive alone
/// with the original checkout made unreadable.
///
/// Cargo prunes any subdirectory containing a `Cargo.toml` as a nested package,
/// and honours each package's own `exclude`/`include` patterns. Either rule can
/// drop a retained generator input while every in-tree check stays green; both
/// did, and the failure only surfaced as a `build.rs` canonicalize error inside
/// CI's isolated round trip. Ask cargo itself rather than restating its rules,
/// and derive the file set from the bundle's own inventory rather than naming
/// individual files.
#[test]
#[requires(true)]
#[ensures(true)]
fn every_build_time_generator_input_survives_cargo_packaging() {
    let bundle_root = Path::new("crates/jbotci-semantics")
        .join(smusni_v0_bundle::BUNDLE_ROOT)
        .to_str()
        .expect("bundle root is UTF-8")
        .to_owned();
    let required = smusni_v0_bundle::bundle_rerun_paths()
        .into_iter()
        .map(|relative| format!("{bundle_root}/{relative}"))
        .chain(smusni_v0_bundle::repository_rerun_paths())
        .collect::<BTreeSet<_>>();
    assert!(required.len() >= 20, "inventory shrank unexpectedly");

    let mut by_package: std::collections::BTreeMap<String, Vec<String>> = Default::default();
    for relative in &required {
        assert!(
            repository_root().join(relative).is_file(),
            "declared generator input is absent: {relative}"
        );
        by_package
            .entry(owning_package_dir(relative))
            .or_default()
            .push(relative.clone());
    }

    for (package_dir, paths) in by_package {
        assert!(
            !package_dir.is_empty(),
            "a workspace-root generator input cannot reach any package archive: {paths:?}"
        );
        let listed = cargo_package_list(&package_dir);
        let prefix = format!("{package_dir}/");
        for relative in paths {
            let in_package = relative
                .strip_prefix(&prefix)
                .expect("path lies inside its owning package");
            assert!(
                listed.contains(in_package),
                "cargo package drops required generator input {relative}; \
                 a source distribution built from it cannot run build.rs"
            );
        }
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn checked_bundle_is_current_and_generation_is_reproducible() {
    let scratch = smusni_v0_bundle::scratch_dir("bundle-tests");
    fs::create_dir_all(&scratch).expect("create bundle test scratch");
    let first = scratch.join("policies-first.rs");
    let second = scratch.join("policies-second.rs");
    let seeds = dispositions();
    smusni_v0_bundle::run(
        &BundlePaths::for_manifest_dir(&manifest_dir(), first.clone()),
        &seeds,
        BundleMode::Check,
    )
    .expect("checked-in bundle is the deterministic mint");
    smusni_v0_bundle::run(
        &BundlePaths::for_manifest_dir(&manifest_dir(), second.clone()),
        &seeds,
        BundleMode::Check,
    )
    .expect("second clean check succeeds");
    assert_eq!(fs::read(first).unwrap(), fs::read(second).unwrap());
}

#[test]
#[requires(true)]
#[ensures(true)]
fn generated_table_counts_and_candidate_policy_keys_are_closed() {
    let lexical = jsonl_rows("registry/lexical.jsonl");
    assert_eq!(lexical.len(), 44);
    assert!(lexical.iter().all(|row| {
        row.get("optional-event-slot-row").is_some()
            && row["ordered-numbered-slot-rows"]
                .as_array()
                .is_some_and(|slots| {
                    !slots.is_empty() && slots.iter().all(|slot| slot.get("close-policy").is_some())
                })
    }));
    assert_eq!(jsonl_rows("registry/dispositions.jsonl").len(), 951);
    let projection_failure_reasons = jsonl_rows("registry/projection-failure-reasons.jsonl");
    assert_eq!(projection_failure_reasons.len(), 61);
    assert_eq!(
        projection_failure_reasons
            .iter()
            .filter(|row| {
                row["failure-site"] == "TypedPosition"
                    && row["expected-type-schema"] == "Performable"
                    && row["minimum-raw-owner-type"] == "SemanticGraph"
            })
            .count(),
        54,
    );
    // Every row carries a reviewed section-16.2 class, and exactly the two ids
    // section 14.4 names use the `WholeGraph` site with no expected type.
    assert!(projection_failure_reasons.iter().all(|row| {
        matches!(
            row["failure-class"].as_str(),
            Some(
                "InvalidGraph" | "RouteUnavailable" | "TrackedSpecGap" | "ImplementationInvariant"
            )
        )
    }));
    let whole_graph = projection_failure_reasons
        .iter()
        .filter(|row| row["failure-site"] == "WholeGraph")
        .collect::<Vec<_>>();
    assert_eq!(
        whole_graph
            .iter()
            .map(|row| row["reason-id"].as_str().unwrap())
            .collect::<Vec<_>>(),
        [
            "smusni.projection.graph.root-not-performable",
            "smusni.projection.graph.unbound-variable",
        ]
    );
    assert!(whole_graph.iter().all(|row| {
        row["raw-root-type"] == "SemanticGraph"
            && row.get("expected-type-schema").is_none()
            && row.get("minimum-raw-owner-type").is_none()
            && row["failure-class"] == "InvalidGraph"
    }));
    assert!(projection_failure_reasons.iter().any(|row| {
        row["reason-id"] == "smusni.projection.lexical-policy.entity"
            && row["failure-site"] == "TypedPosition"
            && row["expected-type-schema"] == "(Referents Entity)"
            && row["minimum-raw-owner-type"] == "Referent"
    }));
    assert!(projection_failure_reasons.iter().any(|row| {
        row["reason-id"] == "smusni.projection.lexical-policy.eventuality"
            && row["failure-site"] == "TypedPosition"
            && row["expected-type-schema"] == "(Referents Eventuality)"
            && row["minimum-raw-owner-type"] == "Referent"
    }));
    let policies = jsonl_rows("registry/scope-policies.jsonl");
    assert_eq!(policies.len(), 8);
    assert_eq!(
        policies
            .iter()
            .map(|row| (
                row["normalized-root"].as_str().unwrap(),
                row["original-ordinal"].as_u64().unwrap(),
                row["scope-policy"].as_str().unwrap(),
            ))
            .collect::<Vec<_>>(),
        vec![
            ("blabi", 1, "Extensional"),
            ("djica", 2, "Intensional"),
            ("kakne", 2, "Intensional"),
            ("klama", 2, "Extensional"),
            ("klama", 5, "Extensional"),
            ("melbi", 1, "Extensional"),
            ("pilno", 1, "Extensional"),
            ("pilno", 2, "Extensional"),
        ]
    );
    assert_eq!(
        policies
            .iter()
            .filter(|row| row["scope-policy"] == "Extensional")
            .count(),
        6
    );
    assert_eq!(
        policies
            .iter()
            .filter(|row| row["scope-policy"] == "Intensional")
            .count(),
        2
    );
    assert!(
        policies
            .iter()
            .all(|row| row.get("dynamic-family").is_none())
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn prelude_rows_declare_the_exact_registry_type_parameter_domain() {
    let rows = jsonl_rows("registry/prelude.jsonl");
    assert_eq!(rows.len(), 20);
    assert_eq!(
        rows.iter()
            .filter(|row| row["type-parameters"] == serde_json::json!(["T"]))
            .count(),
        13,
    );
    assert_eq!(
        rows.iter()
            .filter(|row| row["type-parameters"] == serde_json::json!([]))
            .count(),
        7,
    );
    for row in rows {
        let declared = row["type-parameters"]
            .as_array()
            .expect("PreludeRow type-parameters array");
        let signature = row["complete-signature-schema"]
            .as_str()
            .expect("PreludeRow signature");
        let definition = row["canonical-definition"]
            .as_str()
            .expect("PreludeRow definition");
        assert!(!registry_datum_contains_bare_t(signature));
        assert!(!registry_datum_contains_bare_t(definition));
        let used = registry_type_parameter_names(signature)
            .into_iter()
            .chain(registry_type_parameter_names(definition))
            .collect::<BTreeSet<_>>();
        assert_eq!(
            used,
            declared
                .iter()
                .map(|name| name.as_str().unwrap().to_owned())
                .collect::<BTreeSet<_>>(),
        );
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn every_row_schema_rejects_a_missing_required_field() {
    let tables = [
        "registry/evidence.jsonl",
        "registry/lexical.jsonl",
        "registry/scope-policies.jsonl",
        "registry/place-deletions.jsonl",
        "registry/tag-reductions.jsonl",
        "registry/relation-formers.jsonl",
        "registry/generated-relations.jsonl",
        "registry/scale-literals.jsonl",
        "registry/projection-failure-reasons.jsonl",
        "registry/dispositions.jsonl",
        "registry/prelude.jsonl",
    ];
    let paths = bundle_paths();
    for path in tables {
        let mut mutated = snapshot();
        mutate_first_row(&mut mutated, path, |row| {
            let key = row.keys().next().cloned().expect("schema has fields");
            row.remove(&key);
        });
        assert!(
            smusni_v0_bundle::verify_snapshot(&paths, &mutated).is_err(),
            "{path} accepted a row missing one required schema field"
        );
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn validator_rejects_byte_and_order_mutations() {
    let paths = bundle_paths();

    let mut crlf = snapshot();
    crlf.artifacts
        .get_mut("registry/scale-literals.jsonl")
        .unwrap()
        .push(b'\r');
    assert_eq!(
        smusni_v0_bundle::verify_snapshot(&paths, &crlf)
            .unwrap_err()
            .kind,
        BundleErrorKind::ByteDomain
    );

    let mut order = snapshot();
    let table = order
        .artifacts
        .get_mut("registry/scope-policies.jsonl")
        .unwrap();
    let mut lines = table
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .map(<[u8]>::to_vec)
        .collect::<Vec<_>>();
    lines.swap(0, 1);
    table.clear();
    for line in lines {
        table.extend(line);
        table.push(b'\n');
    }
    assert_eq!(
        smusni_v0_bundle::verify_snapshot(&paths, &order)
            .unwrap_err()
            .kind,
        BundleErrorKind::NonCanonicalOrder
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn validator_rejects_foreign_key_evidence_template_and_summary_mutations() {
    let paths = bundle_paths();

    let mut foreign = snapshot();
    mutate_first_row(&mut foreign, "registry/scope-policies.jsonl", |row| {
        row.insert("normalized-root".to_owned(), serde_json::json!("aaaaa"));
    });
    assert_eq!(
        smusni_v0_bundle::verify_snapshot(&paths, &foreign)
            .unwrap_err()
            .kind,
        BundleErrorKind::ForeignKey
    );

    let mut evidence = snapshot();
    mutate_first_row(&mut evidence, "registry/scale-literals.jsonl", |row| {
        row.insert(
            "evidence-id".to_owned(),
            serde_json::json!("smusni.missing-evidence"),
        );
    });
    assert_eq!(
        smusni_v0_bundle::verify_snapshot(&paths, &evidence)
            .unwrap_err()
            .kind,
        BundleErrorKind::Evidence
    );

    let mut template = snapshot();
    mutate_first_row(&mut template, "registry/tag-reductions.jsonl", |row| {
        row.insert(
            "typed-expansion-template".to_owned(),
            serde_json::json!("(Hole \"UPPER\" Content)"),
        );
    });
    assert_eq!(
        smusni_v0_bundle::verify_snapshot(&paths, &template)
            .unwrap_err()
            .kind,
        BundleErrorKind::Template
    );

    let mut summary = snapshot();
    mutate_first_row(&mut summary, "registry/generated-relations.jsonl", |row| {
        row.insert(
            "stability-summary".to_owned(),
            serde_json::json!("unstable"),
        );
    });
    assert!(smusni_v0_bundle::verify_snapshot(&paths, &summary).is_err());

    let mut projection_failure_boundary = snapshot();
    mutate_first_row(
        &mut projection_failure_boundary,
        "registry/projection-failure-reasons.jsonl",
        |row| {
            let replacement = if row["expected-type-schema"] == "Performable" {
                "Content"
            } else {
                "Performable"
            };
            row.insert(
                "expected-type-schema".to_owned(),
                serde_json::json!(replacement),
            );
        },
    );
    assert_eq!(
        smusni_v0_bundle::verify_snapshot(&paths, &projection_failure_boundary)
            .unwrap_err()
            .kind,
        BundleErrorKind::Type,
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn validator_rejects_a_type_correct_prelude_that_diverges_from_frozen_spec() {
    let paths = bundle_paths();
    let mut divergent = snapshot();
    mutate_first_row(&mut divergent, "registry/prelude.jsonl", |row| {
        assert_eq!(row["name"], ">");
        let definition = row["canonical-definition"].as_str().unwrap();
        let replacement = definition.replace("(< $b $a)", "(≤ $b $a)");
        assert_ne!(replacement, definition);
        row.insert(
            "canonical-definition".to_owned(),
            serde_json::Value::String(replacement),
        );
    });
    assert_eq!(
        smusni_v0_bundle::verify_snapshot(&paths, &divergent)
            .unwrap_err()
            .kind,
        BundleErrorKind::Template,
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn source_generator_rejects_closed_value_key_deletion_and_hole_mutations() {
    let paths = bundle_paths();
    let seeds = dispositions();
    let cases = [
        replace_once(REGISTRY_SOURCE, "format_version = 0", "format_version = 1"),
        replace_once(
            REGISTRY_SOURCE,
            "normalized_root = \"blabi\"\noriginal_ordinal = 1",
            "normalized_root = \"nonce\"\noriginal_ordinal = 1",
        ),
        replace_once(
            REGISTRY_SOURCE,
            "surviving_slot_map = [\"1->1\", \"2->2\", \"4->4\", \"Eventuality->Eventuality\"]",
            "surviving_slot_map = [\"1->1\", \"2->2\", \"4->3\", \"Eventuality->Eventuality\"]",
        ),
        replace_once(
            REGISTRY_SOURCE,
            "surviving_slot_map = [\"1->1\", \"2->2\", \"4->4\", \"Eventuality->Eventuality\"]",
            "surviving_slot_map = [\"9->1\", \"2->2\", \"4->4\", \"Eventuality->Eventuality\"]",
        ),
        replace_once(
            REGISTRY_SOURCE,
            "typed_expansion_template = \"(Joi (Hole \\\"host\\\" Content) (pilno :2 (Hole \\\"filler\\\" (Referents Entity)) :Eventuality (Hole \\\"event\\\" Eventuality)))\"",
            "typed_expansion_template = \"(Joi (Hole \\\"host\\\" Content) (pilno :2 (Hole \\\"host\\\" (Referents Entity)) :Eventuality (Hole \\\"event\\\" Eventuality)))\"",
        ),
    ];
    for mutated in cases {
        assert!(
            smusni_v0_bundle::validate_registry_source(&paths, &seeds, mutated.as_bytes()).is_err()
        );
    }

    let ill_typed_template = replace_once(
        REGISTRY_SOURCE,
        "typed_expansion_template = \"(Joi (Hole \\\"host\\\" Content) (pilno :2 (Hole \\\"filler\\\" (Referents Entity)) :Eventuality (Hole \\\"event\\\" Eventuality)))\"",
        "typed_expansion_template = \"(Joi (Hole \\\"host\\\" Content) (pilno :2 (Hole \\\"filler\\\" Content) :Eventuality (Hole \\\"event\\\" Eventuality)))\"",
    );
    assert_eq!(
        smusni_v0_bundle::validate_registry_source(&paths, &seeds, ill_typed_template.as_bytes(),)
            .unwrap_err()
            .kind,
        BundleErrorKind::Type
    );

    let ill_typed_prelude_signature = replace_once(
        REGISTRY_SOURCE,
        "name = \"Named\"\ntype_parameters = []\ncomplete_signature_schema = \"(Fn (Text (Referents Entity)) Content)\"",
        "name = \"Named\"\ntype_parameters = []\ncomplete_signature_schema = \"(Fn ((Referents Entity) (Referents Entity)) Content)\"",
    );
    assert_eq!(
        smusni_v0_bundle::validate_registry_source(
            &paths,
            &seeds,
            ill_typed_prelude_signature.as_bytes(),
        )
        .unwrap_err()
        .kind,
        BundleErrorKind::Type
    );

    let ill_typed_relation_former = replace_once(
        REGISTRY_SOURCE,
        "result_row_schema = \"(Row (1 (Referents Entity)) (2 (Referents Entity)) (3 (Referents Entity)) (4 (Referents Entity)) (5 (Referents Entity)) (Eventuality (Referents Eventuality)))\"",
        "result_row_schema = \"(Row (1 Content) (2 (Referents Entity)) (3 (Referents Entity)) (4 (Referents Entity)) (5 (Referents Entity)) (Eventuality (Referents Eventuality)))\"",
    );
    assert_eq!(
        smusni_v0_bundle::validate_registry_source(
            &paths,
            &seeds,
            ill_typed_relation_former.as_bytes(),
        )
        .unwrap_err()
        .kind,
        BundleErrorKind::Type
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn source_generator_rejects_invalid_type_parameter_declarations_and_uses() {
    let paths = bundle_paths();
    let seeds = dispositions();
    let at_least = concat!(
        "name = \"AtLeast\"\n",
        "type_parameters = [\"T\"]\n",
        "complete_signature_schema = \"(Fn (Natural (Fn ((TypeParam \\\"T\\\")) Content)) (GQ (TypeParam \\\"T\\\")))\"",
    );
    let mutations = [
        replace_once(
            REGISTRY_SOURCE,
            at_least,
            &at_least.replace("[\"T\"]", "[\"U\"]"),
        ),
        replace_once(
            REGISTRY_SOURCE,
            at_least,
            &at_least.replace("[\"T\"]", "[\"T\", \"T\"]"),
        ),
        replace_once(
            REGISTRY_SOURCE,
            at_least,
            &at_least.replace("[\"T\"]", "[\"T\", \"U\"]"),
        ),
        replace_once(
            REGISTRY_SOURCE,
            at_least,
            &at_least.replace("(TypeParam \\\"T\\\")", "T"),
        ),
        replace_once(
            REGISTRY_SOURCE,
            "raw_value_type = \"Scale\"",
            "raw_value_type = \"(TypeParam \\\"T\\\")\"",
        ),
    ];
    for (index, mutated) in mutations.into_iter().enumerate() {
        assert!(
            smusni_v0_bundle::validate_registry_source(&paths, &seeds, mutated.as_bytes()).is_err(),
            "type-parameter mutation {index} was accepted",
        );
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn source_generator_rederives_prelude_tag_and_relation_dynamic_summaries() {
    let paths = bundle_paths();
    let seeds = dispositions();
    let mutations = [
        replace_once(
            REGISTRY_SOURCE,
            "name = \"MotionVector\"\ntype_parameters = []\ncomplete_signature_schema = \"(Fn ((Referents Eventuality) (Referents Entity) (Fn ((Referents Entity) (Referents Entity)) Content)) Content)\"\ndirect_dependencies = []\nexpected_dynamic_summary = { context_flow = \"parameterized\", parameter_evaluations = [\"displacement\"], ordered_effects = [], stability = \"parameterized\" }",
            "name = \"MotionVector\"\ntype_parameters = []\ncomplete_signature_schema = \"(Fn ((Referents Eventuality) (Referents Entity) (Fn ((Referents Entity) (Referents Entity)) Content)) Content)\"\ndirect_dependencies = []\nexpected_dynamic_summary = { context_flow = \"identity\", parameter_evaluations = [], ordered_effects = [], stability = \"stable\" }",
        ),
        replace_once(
            REGISTRY_SOURCE,
            "typed_expansion_template = \"(Joi (Hole \\\"host\\\" Content) (pilno :2 (Hole \\\"filler\\\" (Referents Entity)) :Eventuality (Hole \\\"event\\\" Eventuality)))\"\nresulting_type_schema = \"Content\"\nexpected_dynamic_summary = { context_flow = \"parameterized\", parameter_evaluations = [\"host\"], ordered_effects = [], stability = \"parameterized\" }",
            "typed_expansion_template = \"(Joi (Hole \\\"host\\\" Content) (pilno :2 (Hole \\\"filler\\\" (Referents Entity)) :Eventuality (Hole \\\"event\\\" Eventuality)))\"\nresulting_type_schema = \"Content\"\nexpected_dynamic_summary = { context_flow = \"identity\", parameter_evaluations = [], ordered_effects = [], stability = \"stable\" }",
        ),
        replace_once(
            REGISTRY_SOURCE,
            "typed_link_or_expansion_contract = \"(Hole \\\"relation\\\" (PredTerm (Row (1 (Referents Entity)) (2 (Referents Entity)) (3 (Referents Entity)) (4 (Referents Entity)) (5 (Referents Entity)) (Eventuality (Referents Eventuality)))))\"\nexpected_dynamic_summary = { context_flow = \"identity\", parameter_evaluations = [], ordered_effects = [], stability = \"stable\" }",
            "typed_link_or_expansion_contract = \"(Hole \\\"relation\\\" (PredTerm (Row (1 (Referents Entity)) (2 (Referents Entity)) (3 (Referents Entity)) (4 (Referents Entity)) (5 (Referents Entity)) (Eventuality (Referents Eventuality)))))\"\nexpected_dynamic_summary = { context_flow = \"parameterized\", parameter_evaluations = [\"relation\"], ordered_effects = [], stability = \"parameterized\" }",
        ),
    ];
    for (index, mutated) in mutations.into_iter().enumerate() {
        assert_eq!(
            smusni_v0_bundle::validate_registry_source(&paths, &seeds, mutated.as_bytes())
                .unwrap_err()
                .kind,
            BundleErrorKind::Summary,
            "summary mutation {index} was not rejected at the summary boundary",
        );
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn source_generator_resolves_every_tag_hole_map_event_and_identity() {
    let paths = bundle_paths();
    let seeds = dispositions();
    let mutations = [
        replace_once(REGISTRY_SOURCE, "tag-sumti->pilno-x2", "nonce->pilno-x2"),
        replace_once(
            REGISTRY_SOURCE,
            "tag-sumti->pilno-x2",
            "tag-sumti->pilno-x1",
        ),
        replace_once(
            REGISTRY_SOURCE,
            "typed_expansion_template = \"(Joi (Hole \\\"host\\\" Content) (pilno :2 (Hole \\\"filler\\\" (Referents Entity)) :Eventuality (Hole \\\"event\\\" Eventuality)))\"",
            "typed_expansion_template = \"(Joi (Hole \\\"host\\\" Content) (pilno :2 (Hole \\\"sumti\\\" (Referents Entity)) :Eventuality (Hole \\\"event\\\" Eventuality)))\"",
        ),
        replace_once(
            REGISTRY_SOURCE,
            "source_member = \"sepi'o\"\napplicability_guard = \"(Hole \\\"host\\\" Content)\"\noperand_types = [\"Content\", \"(Referents Entity)\", \"Eventuality\"]",
            "source_member = \"sepi'o\"\napplicability_guard = \"(Hole \\\"host\\\" Content)\"\noperand_types = [\"Content\", \"Eventuality\", \"(Referents Entity)\"]",
        ),
        replace_once(
            REGISTRY_SOURCE,
            "source_place_map = [\"tag-sumti->pilno-x2\", \"host-event->pilno-event\"]\nhost_event_map = \"shared\"",
            "source_place_map = [\"tag-sumti->pilno-x2\", \"host-event->pilno-event\"]\nhost_event_map = \"local\"",
        ),
        replace_once(
            REGISTRY_SOURCE,
            "source_place_map = [\"tag-sumti->pilno-x2\", \"host-event->pilno-event\"]",
            "source_place_map = [\"host-event->pilno-event\"]",
        ),
        replace_once(
            REGISTRY_SOURCE,
            "source_place_map = [\"tag-sumti->pilno-x2\", \"host-event->pilno-event\"]\nhost_event_map = \"shared\"\nrequired_graph_identities = [\"host-event\"]",
            "source_place_map = [\"tag-sumti->pilno-x2\", \"host-event->pilno-event\"]\nhost_event_map = \"shared\"\nrequired_graph_identities = [\"host-x1\"]",
        ),
        replace_once(
            REGISTRY_SOURCE,
            "typed_expansion_template = \"(DescribedAs (Hole \\\"describer\\\" (Referents Entity)) (Hole \\\"described\\\" (Referents Entity)) (Hole \\\"property\\\" (Fn ((Referents Entity)) Content)))\"",
            "typed_expansion_template = \"(DescribedAs (Hole \\\"described\\\" (Referents Entity)) (Hole \\\"describer\\\" (Referents Entity)) (Hole \\\"property\\\" (Fn ((Referents Entity)) Content)))\"",
        ),
        replace_once(
            &replace_once(
                REGISTRY_SOURCE,
                "typed_expansion_template = \"(DescribedAs (Hole \\\"describer\\\" (Referents Entity)) (Hole \\\"described\\\" (Referents Entity)) (Hole \\\"property\\\" (Fn ((Referents Entity)) Content)))\"",
                "typed_expansion_template = \"(DescribedAs (Hole \\\"described\\\" (Referents Entity)) (Hole \\\"describer\\\" (Referents Entity)) (Hole \\\"property\\\" (Fn ((Referents Entity)) Content)))\"",
            ),
            "source_place_map = [\"speaker->describer\", \"relative-head->described\", \"clause->property\"]",
            "source_place_map = [\"speaker->described\", \"relative-head->describer\", \"clause->property\"]",
        ),
    ];
    for (index, mutated) in mutations.into_iter().enumerate() {
        assert!(
            smusni_v0_bundle::validate_registry_source(&paths, &seeds, mutated.as_bytes()).is_err(),
            "tag correspondence mutation {index} was accepted",
        );
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn source_generator_requires_explicit_closure_and_checks_polymorphic_deictic_types() {
    let paths = bundle_paths();
    let seeds = dispositions();
    let cases = [
        replace_once(
            REGISTRY_SOURCE,
            "root = \"bajra\"\nslot_types = [\"(Referents Entity)\", \"(Referents Entity)\", \"(Referents Entity)\", \"(Referents Entity)\"]\nslot_close_policies = [\"Contextual\", \"Contextual\", \"Contextual\", \"Contextual\"]\nevent_slot = \"LocalExistential\"",
            "root = \"bajra\"\nslot_types = [\"(Referents Entity)\", \"(Referents Entity)\", \"(Referents Entity)\", \"(Referents Entity)\"]\nevent_slot = \"LocalExistential\"",
        ),
        replace_once(
            REGISTRY_SOURCE,
            "root = \"bajra\"\nslot_types = [\"(Referents Entity)\", \"(Referents Entity)\", \"(Referents Entity)\", \"(Referents Entity)\"]\nslot_close_policies = [\"Contextual\", \"Contextual\", \"Contextual\", \"Contextual\"]\nevent_slot = \"LocalExistential\"",
            "root = \"bajra\"\nslot_types = [\"(Referents Entity)\", \"(Referents Entity)\", \"(Referents Entity)\", \"(Referents Entity)\"]\nslot_close_policies = [\"Contextual\", \"Contextual\", \"Contextual\", \"Contextual\"]\n# event slot deliberately omitted",
        ),
        replace_once(
            REGISTRY_SOURCE,
            "root = \"bajra\"\nslot_types = [\"(Referents Entity)\", \"(Referents Entity)\", \"(Referents Entity)\", \"(Referents Entity)\"]\nslot_close_policies = [\"Contextual\", \"Contextual\", \"Contextual\", \"Contextual\"]\nevent_slot = \"LocalExistential\"",
            "root = \"bajra\"\nslot_types = [\"(Referents Entity)\", \"(Referents Entity)\", \"(Referents Entity)\", \"(Referents Entity)\"]\nslot_close_policies = [\"LocalExistential\", \"Contextual\", \"Contextual\", \"Contextual\"]\nevent_slot = \"LocalExistential\"",
        ),
        replace_once(
            REGISTRY_SOURCE,
            "slot_close_policies = [\"Required\", \"Contextual\", \"Contextual\"]",
            "slot_close_policies = [\"Contextual\", \"Contextual\", \"Contextual\"]",
        ),
        replace_once(
            REGISTRY_SOURCE,
            "root = \"bajra\"\nslot_types = [\"(Referents Entity)\", \"(Referents Entity)\", \"(Referents Entity)\", \"(Referents Entity)\"]\nslot_close_policies = [\"Contextual\", \"Contextual\", \"Contextual\", \"Contextual\"]\nevent_slot = \"LocalExistential\"",
            "root = \"bajra\"\nslot_types = [\"(Referents Entity)\", \"(Referents Entity)\", \"(Referents Entity)\", \"(Referents Entity)\"]\nslot_close_policies = [\"Contextual\", \"Contextual\", \"Contextual\", \"Contextual\"]\nevent_slot = \"ImplicitDefault\"",
        ),
        replace_once(
            REGISTRY_SOURCE,
            "name = \"Named\"\ntype_parameters = []\ncomplete_signature_schema = \"(Fn (Text (Referents Entity)) Content)\"",
            "name = \"Named\"\ntype_parameters = []\ncomplete_signature_schema = \"(Fn (Text Text) Content)\"",
        ),
        replace_once(
            REGISTRY_SOURCE,
            "name = \"AtLeast\"\ntype_parameters = [\"T\"]\ncomplete_signature_schema = \"(Fn (Natural (Fn ((TypeParam \\\"T\\\")) Content)) (GQ (TypeParam \\\"T\\\")))\"",
            "name = \"AtLeast\"\ntype_parameters = [\"T\"]\ncomplete_signature_schema = \"(Fn (Scale (Fn ((TypeParam \\\"T\\\")) Content)) (GQ (TypeParam \\\"T\\\")))\"",
        ),
        replace_once(
            REGISTRY_SOURCE,
            "name = \"This\"\ntype_parameters = []\ncomplete_signature_schema = \"(Referents Entity)\"",
            "name = \"This\"\ntype_parameters = []\ncomplete_signature_schema = \"(Referents Eventuality)\"",
        ),
    ];
    for (index, mutated) in cases.into_iter().enumerate() {
        assert!(
            smusni_v0_bundle::validate_registry_source(&paths, &seeds, mutated.as_bytes()).is_err(),
            "invalid explicit closure/type mutation {index} was accepted"
        );
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn every_curated_policy_row_rejects_each_other_closed_policy() {
    let paths = bundle_paths();
    let seeds = dispositions();
    let source =
        toml::from_str::<toml::Value>(REGISTRY_SOURCE).expect("registry source is valid TOML");
    let policies = source["scope_policy"]
        .as_array()
        .expect("scope-policy array");
    let closed_policies = ["Extensional", "Intensional", "Opaque"];

    for index in 0..policies.len() {
        let current = policies[index]["scope_policy"]
            .as_str()
            .expect("scope-policy string");
        for replacement in closed_policies {
            if replacement == current {
                continue;
            }
            let mut mutated = source.clone();
            mutated["scope_policy"][index]["scope_policy"] =
                toml::Value::String(replacement.to_owned());
            let bytes = toml::to_string(&mutated).unwrap();
            assert_eq!(
                smusni_v0_bundle::validate_registry_source(&paths, &seeds, bytes.as_bytes())
                    .unwrap_err()
                    .kind,
                BundleErrorKind::Evidence,
                "scope-policy row {index} accepted {replacement} instead of {current}"
            );
        }
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn every_failure_seed_requires_one_complete_typed_boundary() {
    let paths = bundle_paths();
    let mut seeds = dispositions();
    let failure = seeds
        .iter_mut()
        .find(|seed| {
            seed.disposition == "Failure" && seed.failure_site.as_deref() == Some("TypedPosition")
        })
        .expect("inventory has typed-position failure dispositions");
    failure.expected_type_schema = None;
    assert_eq!(
        smusni_v0_bundle::mint_snapshot(&paths, &seeds)
            .unwrap_err()
            .kind,
        BundleErrorKind::Type,
    );
}
