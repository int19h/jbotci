use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[allow(unused_imports)]
use bityzba::{ensures, requires};
use xarsnu::protocol::ProtocolEventData;
use xarsnu::{ReasoningConfig, dialog_file, read_transcript, report_file};

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
            "dialog-closed-for-answers",
            "intent-registered",
            "listener-flow-abandoned",
            "meaning-confirmed",
            "message-posted",
            "prose-rejected",
            "protocol-error",
            "reference-call-budget-exhausted",
            "reference-lookup-repeated",
            "reference-research-nudge",
            "reference-tool-completed",
            "run-aborted",
            "run-finished",
            "run-started",
            "tersmu-revealed",
            "thinking-recorded",
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
        "## Dialog",
        "**alice:** mi klama",
        "*(alice forfeited turn 1)*",
        "*(alice submitted an answer)*",
        "*(checker: partial)*",
        "*(visible dialog closed for independent answers after round 1)*",
        "*(run aborted: cost budget exceeded)*",
        "**Gate result:** rejected (morphology)",
        "Verdict: **mismatch**",
        "### Turn forfeited",
        "Recorded discrepancies:",
        "### Scenario answer",
        "### Scenario checker",
        "### Repeated reference lookup",
        "### Reference-call budget exhausted",
        "### Reference-research nudge",
        "### Auto-mode prose rejected",
        "### Listener flow abandoned",
        "### Visible dialog closed for independent answers",
        "Auto-mode prose rejections: 1",
        "Listener flows abandoned: 1",
        "80 cached, 40 cache-write tokens",
        "Cache efficiency: 66.67%",
        "Call hit rate: 100.00%",
        "Reasoning field present: true; reasoning tokens: 20",
        "Reasoning totals: 20 tokens across 1 provider calls",
        "Serving provider: `xiaomi/fp8`",
        "Provider mix: `xiaomi/fp8`: 1",
        "### Thinking — `alice`",
        "> First private line.\n> Second private line.",
        "fixture-signature",
    ] {
        assert!(report.contains(anti_no_op), "missing {anti_no_op}");
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn standalone_dialog_matches_golden_and_excludes_private_scaffolding() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let dialog = dialog_file(&root.join(FIXTURE)).expect("standalone dialog renders");

    assert_eq!(
        dialog,
        include_str!("fixtures/transcript-all-events-dialog.md")
    );
    for private_scaffolding in [
        "tersmu",
        "Diagnostics",
        "diagnostics",
        "Intent",
        "intent",
        "(klama mi)",
        "start_minute",
        "Reasoning",
        "reasoning",
        "Thinking",
        "First private line",
        "fixture-signature",
    ] {
        assert!(
            !dialog.contains(private_scaffolding),
            "standalone dialog leaked {private_scaffolding}"
        );
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn standalone_dialog_preserves_interleaved_event_order() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixture = fs::read_to_string(root.join(FIXTURE)).expect("read golden fixture");
    let records = fixture
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).expect("valid fixture record"))
        .collect::<Vec<_>>();
    let event = |kind: &str| {
        records
            .iter()
            .find(|record| record["event"]["kind"] == kind)
            .expect("fixture contains requested event")
            .clone()
    };
    let mut interleaved = vec![
        event("run-started"),
        event("turn-started"),
        event("message-posted"),
        event("answer-submitted"),
        event("message-posted"),
        event("turn-forfeited"),
        event("checker-outcome"),
        event("run-finished"),
    ];
    interleaved[4]["participant"] = serde_json::json!("bob");
    interleaved[4]["event"]["speaker"] = serde_json::json!("bob");
    interleaved[4]["event"]["message"]["text"] = serde_json::json!("do tavla");
    interleaved[5]["participant"] = serde_json::json!("bob");
    interleaved[5]["event"]["speaker"] = serde_json::json!("bob");
    for (sequence_number, record) in interleaved.iter_mut().enumerate() {
        record["sequence-number"] = serde_json::json!(sequence_number);
    }
    let path = temp_path("interleaved-dialog-order");
    let source = interleaved
        .iter()
        .map(|record| serde_json::to_string(record).expect("record serializes"))
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    fs::write(&path, source).expect("write interleaved transcript");

    let dialog = dialog_file(&path).expect("interleaved dialog renders");
    let expected_entries = [
        "**alice:** mi klama",
        "*(alice submitted an answer)*",
        "**bob:** do tavla",
        "*(bob forfeited turn 1)*",
        "*(checker: partial)*",
    ];
    let mut previous = 0;
    for entry in expected_entries {
        let position = dialog.find(entry).expect("dialog contains expected entry");
        assert!(position >= previous, "{entry} rendered out of event order");
        previous = position;
    }

    fs::remove_file(path).expect("remove temporary transcript");
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
fn schema_v1_accepts_usage_without_additive_provider_field() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixture = fs::read_to_string(root.join(FIXTURE)).expect("read golden fixture");
    let legacy = fixture.replace("\"provider\":\"xiaomi/fp8\",", "");
    assert_ne!(legacy, fixture, "fixture must exercise provider capture");
    let path = temp_path("schema-v1-without-provider");
    fs::write(&path, legacy).expect("write legacy-compatible transcript");

    let records = read_transcript(&path).expect("optional provider field may be absent");
    assert!(!records.is_empty());
    let report = report_file(&path).expect("legacy-compatible transcript renders");
    assert!(report.contains("Provider mix: unknown: 1"));

    fs::remove_file(path).expect("remove temporary transcript");
}

#[test]
#[requires(true)]
#[ensures(true)]
fn schema_v1_accepts_transcripts_without_additive_thinking_events() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixture = fs::read_to_string(root.join(FIXTURE)).expect("read golden fixture");
    let mut legacy = fixture
        .lines()
        .filter_map(|line| {
            let record: serde_json::Value = serde_json::from_str(line).expect("valid fixture");
            (record["event"]["kind"] != "thinking-recorded").then_some(record)
        })
        .collect::<Vec<_>>();
    for (sequence_number, record) in legacy.iter_mut().enumerate() {
        record["sequence-number"] = serde_json::json!(sequence_number);
    }
    let source = legacy
        .iter()
        .map(|record| serde_json::to_string(record).expect("legacy record serializes"))
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    let path = temp_path("schema-v1-without-thinking-events");
    fs::write(&path, source).expect("write legacy-compatible transcript");

    let records = read_transcript(&path).expect("thinking events are additive");
    assert!(!records.is_empty());
    let report = report_file(&path).expect("legacy-compatible transcript renders");
    assert!(!report.contains("### Thinking"));

    fs::remove_file(path).expect("remove temporary transcript");
}

#[test]
#[requires(true)]
#[ensures(true)]
fn schema_v1_accepts_the_pre_unification_reasoning_config_field() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixture = fs::read_to_string(root.join(FIXTURE)).expect("read golden fixture");
    let legacy = fixture.replacen(
        "\"model\":\"example/alice\",\"temperature\"",
        "\"model\":\"example/alice\",\"disable-reasoning\":true,\"temperature\"",
        1,
    );
    assert_ne!(legacy, fixture, "fixture must gain the legacy field");
    let path = temp_path("schema-v1-legacy-disable-reasoning");
    fs::write(&path, legacy).expect("write legacy-compatible transcript");

    let records = read_transcript(&path).expect("legacy reasoning config remains readable");
    let ProtocolEventData::RunStarted { header } = records[0].event.as_data() else {
        panic!("validated transcript starts with the run header");
    };
    assert_eq!(
        header.config.participants[0].reasoning,
        Some(ReasoningConfig::Off)
    );

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

    let dialog_output = Command::new(env!("CARGO_BIN_EXE_xarsnu"))
        .arg("report")
        .arg("--dialog")
        .arg(root.join(FIXTURE))
        .env_remove("OPENROUTER_API_KEY")
        .output()
        .expect("run dialog report command");
    assert!(
        dialog_output.status.success(),
        "{}",
        String::from_utf8_lossy(&dialog_output.stderr)
    );
    assert_eq!(
        String::from_utf8(dialog_output.stdout).expect("dialog is UTF-8"),
        include_str!("fixtures/transcript-all-events-dialog.md")
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
