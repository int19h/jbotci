use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[allow(unused_imports)]
use bityzba::{ensures, requires};
use xarsnu::{read_transcript, report_file};

const FIXTURE: &str = "tests/fixtures/transcript-all-events.jsonl";

#[test]
#[requires(true)]
#[ensures(true)]
fn golden_transcript_renders_every_event_kind() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let records = read_transcript(&root.join(FIXTURE)).expect("golden transcript validates");
    let kinds = records
        .iter()
        .map(|record| {
            serde_json::to_value(&record.event).expect("event serializes")["kind"]
                .as_str()
                .expect("tag is a string")
                .to_owned()
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(
        kinds,
        [
            "acknowledged",
            "answer-submitted",
            "blind-interpretation-recorded",
            "candidate-accepted",
            "candidate-rejected",
            "candidate-submitted",
            "checker-outcome",
            "intent-registered",
            "meaning-confirmed",
            "message-posted",
            "protocol-error",
            "reference-tool-completed",
            "run-aborted",
            "run-finished",
            "run-started",
            "tersmu-revealed",
            "turn-forfeited",
            "turn-started",
            "usage-recorded",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect()
    );

    let report = report_file(&root.join(FIXTURE)).expect("golden report renders");
    assert_eq!(
        report,
        include_str!("fixtures/transcript-all-events-report.md")
    );
    for anti_no_op in [
        "**Gate result:** rejected (morphology)",
        "Verdict: **mismatch**",
        "### Turn forfeited",
        "Recorded discrepancies:",
        "### Scenario answer",
        "### Scenario checker",
        "80 cached, 40 cache-write tokens",
        "Cache efficiency: 66.67%",
        "Call hit rate: 100.00%",
    ] {
        assert!(report.contains(anti_no_op), "missing {anti_no_op}");
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn schema_v1_accepts_usage_without_additive_cache_fields() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixture = fs::read_to_string(root.join(FIXTURE)).expect("read golden fixture");
    let legacy = fixture.replace(",\"cached_tokens\":80,\"cache_write_tokens\":40", "");
    assert_ne!(legacy, fixture, "fixture must exercise cache fields");
    let path = temp_path("schema-v1-without-cache-fields");
    fs::write(&path, legacy).expect("write legacy-compatible transcript");

    let records = read_transcript(&path).expect("optional cache fields may be absent");
    assert!(!records.is_empty());
    let report = report_file(&path).expect("legacy-compatible transcript renders");
    assert!(report.contains("Cache totals: 0 cached tokens; 0 cache-write tokens"));

    fs::remove_file(path).expect("remove temporary transcript");
}

#[test]
#[requires(true)]
#[ensures(true)]
fn corrupted_transcripts_report_the_exact_line() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixture = fs::read_to_string(root.join(FIXTURE)).expect("read golden fixture");
    let lines = fixture.lines().collect::<Vec<_>>();

    let bad_json = temp_path("bad-json");
    fs::write(&bad_json, format!("{}\n{{bad\n", lines[0])).expect("write bad JSON");
    let error = read_transcript(&bad_json).expect_err("bad JSON must fail");
    assert_eq!(error.line, 2);
    assert!(error.to_string().contains("invalid transcript JSON"));

    let missing_header = temp_path("missing-header");
    fs::write(&missing_header, format!("{}\n", lines[1])).expect("write missing header");
    let error = read_transcript(&missing_header).expect_err("missing header must fail");
    assert_eq!(error.line, 1);
    assert!(error.to_string().contains("missing run header"));

    let sequence_gap = temp_path("sequence-gap");
    let mut second: serde_json::Value = serde_json::from_str(lines[1]).expect("valid line");
    second["sequence-number"] = serde_json::json!(8);
    fs::write(
        &sequence_gap,
        format!(
            "{}\n{}\n",
            lines[0],
            serde_json::to_string(&second).unwrap()
        ),
    )
    .expect("write sequence gap");
    let error = read_transcript(&sequence_gap).expect_err("sequence gap must fail");
    assert_eq!(error.line, 2);
    assert!(error.to_string().contains("expected 1, found 8"));

    let truncated = temp_path("truncated");
    fs::write(&truncated, format!("{}\n{}\n", lines[0], lines[1]))
        .expect("write truncated transcript");
    let error = read_transcript(&truncated).expect_err("truncation must fail");
    assert_eq!(error.line, 2);
    assert!(error.to_string().contains("no terminal event"));

    for path in [bad_json, missing_header, sequence_gap, truncated] {
        fs::remove_file(path).expect("remove temporary transcript");
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn report_subcommand_is_offline_and_matches_the_library() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let output = Command::new(env!("CARGO_BIN_EXE_xarsnu"))
        .arg("report")
        .arg(root.join(FIXTURE))
        .env_remove("OPENROUTER_API_KEY")
        .output()
        .expect("run report command");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).expect("report is UTF-8"),
        include_str!("fixtures/transcript-all-events-report.md")
    );
}

#[requires(!name.trim().is_empty())]
#[ensures(ret.file_name().is_some())]
fn temp_path(name: &str) -> PathBuf {
    let executable = std::env::current_exe().expect("current test executable");
    let target_directory = executable
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .expect("Cargo target directory");
    let directory = target_directory.join("xarsnu-test-tmp");
    fs::create_dir_all(&directory).expect("create target temporary directory");
    directory.join(format!("xarsnu-{name}-{}.jsonl", std::process::id()))
}
