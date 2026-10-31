use std::fs;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use bityzba::requires;

#[test]
#[requires(true)]
#[ensures(true)]
fn fixture_tools_report_an_eleven_word_replacement() {
    assert_fixture_dialect_failure(
        "((mi -> mi mi mi mi mi mi mi mi mi mi mi))",
        "The limit is 10",
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn fixture_tools_report_an_invalid_swap_word() {
    assert_fixture_dialect_failure("((aaa <-> eee))", "not morphologically valid: aaa");
}

#[requires(!formula.is_empty() && !reason.is_empty())]
#[ensures(true)]
fn assert_fixture_dialect_failure(formula: &str, reason: &str) {
    let root = std::env::temp_dir().join(format!(
        "jbotci-fixture-dialect-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&root).unwrap();
    let fixture = root.join("one.toml");
    let parity = root.join("one.json");
    let normalized = jbotci_dialect::dialect_definition_to_text(
        &jbotci_dialect::parse_dialect_definition(formula).unwrap(),
    );
    fs::write(
        &parity,
        serde_json::to_vec(&serde_json::json!({
            "schema-version": 1,
            "cases": [{
                "id": "cll.fixture-dialect",
                "lojban": "mi",
                "dialect": formula,
                "syntax-refs": {
                    "pro-argument-targets": [{
                        "node": {"offset": 0, "length": 2},
                        "label": "mi"
                    }]
                }
            }]
        }))
        .unwrap(),
    )
    .unwrap();
    for status in ["success", "failure"] {
        let text = format!(
            "id = 'cll.fixture-dialect'\nlojban = 'mi'\ndialect = '{formula}'\n\
             [expectations.morphology]\nstatus = '{status}'\nraw = '[]'\n\
             [expectations.syntax]\nstatus = 'success'\nraw = '[]'\n"
        );
        fs::write(&fixture, &text).unwrap();
        let root_arg = root.to_str().unwrap();
        let fixture_arg = fixture.to_str().unwrap();
        let parity_arg = parity.to_str().unwrap();
        for args in [
            vec![
                "fixture-test",
                "--root",
                root_arg,
                "--facet",
                "morphology",
                "--jobs",
                "1",
            ],
            vec![
                "fixture-test",
                "--root",
                root_arg,
                "--facet",
                "syntax",
                "--jobs",
                "1",
            ],
            vec![
                "fixture-rewrite",
                "--path",
                fixture_arg,
                "--dry-run",
                "--syntax-only",
            ],
            vec!["fixture-vector-stats", "--root", root_arg, "--jobs", "1"],
            vec![
                "syntax-parser-benchmark",
                "--root",
                root_arg,
                "--worker",
                "--parser",
                "generated",
            ],
            vec!["refs-v0-parity", "--input", parity_arg],
        ] {
            let output = Command::new(env!("CARGO_BIN_EXE_xtask-full"))
                .args(&args)
                .output()
                .unwrap();
            let report = format!(
                "{}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            assert_eq!(output.status.code(), Some(1), "{args:?}: {report}");
            assert!(report.contains(&normalized), "{args:?}: {report}");
            assert!(report.contains(reason), "{args:?}: {report}");
            assert!(!report.contains("panicked"), "{args:?}: {report}");
            assert!(
                !report.contains("dialect definition must compile"),
                "{args:?}: {report}"
            );
            assert_eq!(fs::read_to_string(&fixture).unwrap(), text);
        }
    }
    fs::remove_dir_all(root).unwrap();
}
