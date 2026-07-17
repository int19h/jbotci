use std::cell::Cell;
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread::{self, JoinHandle};
use std::time::Duration;

#[allow(unused_imports)]
use bityzba::{ensures, invariant, new, requires};
use serde_json::{Value, json};
use xarsnu::protocol::{ProtocolEventData, RuntimeFailureSite};
use xarsnu::run::RunErrorData;
use xarsnu::{
    OpenRouterClient, OpenRouterClientConfig, PromptCaching, RetryPolicy, TaskStatus,
    read_transcript, report_file, run,
};

static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[invariant((100..=599).contains(status))]
#[derive(Debug)]
struct MockResponse {
    status: u16,
    body: Value,
}

#[invariant(body.is_object())]
#[derive(Debug)]
struct CapturedRequest {
    body: Value,
}

#[invariant(!base_url.trim().is_empty())]
#[derive(Debug)]
struct MockServer {
    base_url: String,
    worker: JoinHandle<Vec<CapturedRequest>>,
}

impl MockServer {
    #[requires(!responses.is_empty())]
    #[ensures(!ret.base_url.is_empty())]
    fn start(responses: Vec<MockResponse>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind local mock server");
        let address = listener.local_addr().expect("mock server address");
        let worker = thread::spawn(move || {
            let mut captured = Vec::with_capacity(responses.len());
            for response in responses {
                let (mut stream, _) = listener.accept().expect("accept mock request");
                captured.push(new!(CapturedRequest {
                    body: read_request_body(&mut stream),
                }));
                write_response(&mut stream, response);
            }
            captured
        });
        new!(MockServer {
            base_url: format!("http://{address}"),
            worker,
        })
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn finish(self) -> Vec<CapturedRequest> {
        let bityzba::data!(MockServer { worker, .. }) = self.into_data();
        worker.join().expect("mock server thread")
    }
}

#[requires(true)]
#[ensures(ret.is_object())]
fn read_request_body(stream: &mut TcpStream) -> Value {
    let mut bytes = Vec::new();
    let mut buffer = [0u8; 4096];
    let header_end = loop {
        let count = stream.read(&mut buffer).expect("read mock request");
        assert!(count > 0, "request ended before headers");
        bytes.extend_from_slice(&buffer[..count]);
        if let Some(offset) = find_subslice(&bytes, b"\r\n\r\n") {
            break offset + 4;
        }
    };
    let headers = std::str::from_utf8(&bytes[..header_end]).expect("ASCII request headers");
    let content_length = headers
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().expect("content length"))
        })
        .expect("content-length header");
    while bytes.len() - header_end < content_length {
        let count = stream.read(&mut buffer).expect("read mock request body");
        assert!(count > 0, "request ended before body");
        bytes.extend_from_slice(&buffer[..count]);
    }
    serde_json::from_slice(&bytes[header_end..header_end + content_length])
        .expect("JSON completion request")
}

#[requires(!needle.is_empty())]
#[ensures(ret.is_none_or(|offset| offset + needle.len() <= haystack.len()))]
fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

#[requires((100..=599).contains(&response.status))]
#[ensures(true)]
fn write_response(stream: &mut TcpStream, response: MockResponse) {
    let body = response.body.to_string();
    let reason = match response.status {
        200 => "OK",
        401 => "Unauthorized",
        429 => "Too Many Requests",
        _ => "Server Error",
    };
    write!(
        stream,
        "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        response.status,
        reason,
        body.len(),
        body,
    )
    .expect("write mock response");
    stream.flush().expect("flush mock response");
}

#[requires(id > 0)]
#[requires(!name.trim().is_empty())]
#[requires(arguments.is_object())]
#[ensures(ret.status == 200)]
fn tool_response(id: usize, name: &str, arguments: Value) -> MockResponse {
    new!(MockResponse {
        status: 200,
        body: json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": null,
                    "tool_calls": [{
                        "id": format!("call-{id}-{name}"),
                        "type": "function",
                        "function": {
                            "name": name,
                            "arguments": arguments.to_string(),
                        },
                    }],
                },
            }],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 2,
                "total_tokens": 12,
                "cost": 0.001,
            },
        }),
    })
}

#[requires(true)]
#[ensures(ret.status == 500)]
fn runtime_failure_response() -> MockResponse {
    new!(MockResponse {
        status: 500,
        body: json!({ "error": "deliberate mid-run failure" }),
    })
}

#[requires(true)]
#[ensures(ret.len() == 16)]
fn complete_dialog_responses() -> Vec<MockResponse> {
    let wrong_answer = || json!({ "day": "tuesday", "start_minute": 600, "duration_minutes": 60 });
    vec![
        tool_response(
            1,
            "register_intent",
            json!({ "meaning_en": "I can meet on Tuesday." }),
        ),
        tool_response(2, "submit_lojban", json!({ "text": "mi cu" })),
        tool_response(3, "submit_lojban", json!({ "text": "mi klama" })),
        tool_response(
            4,
            "confirm_meaning",
            json!({ "matches": true, "paraphrase_en": "I go." }),
        ),
        tool_response(
            5,
            "interpret_blind",
            json!({ "interpretation_en": "Alice goes." }),
        ),
        tool_response(
            6,
            "acknowledge",
            json!({ "final_understanding_en": "Alice goes." }),
        ),
        tool_response(
            7,
            "register_intent",
            json!({ "meaning_en": "I can also meet on Tuesday." }),
        ),
        tool_response(8, "submit_lojban", json!({ "text": "mi klama" })),
        tool_response(
            9,
            "confirm_meaning",
            json!({ "matches": true, "paraphrase_en": "I go." }),
        ),
        tool_response(
            10,
            "interpret_blind",
            json!({ "interpretation_en": "Bob goes." }),
        ),
        tool_response(
            11,
            "acknowledge",
            json!({ "final_understanding_en": "Bob goes." }),
        ),
        tool_response(12, "submit_answer", wrong_answer()),
        tool_response(
            13,
            "register_intent",
            json!({ "meaning_en": "Let us use the proposed time." }),
        ),
        tool_response(14, "submit_lojban", json!({ "text": "mi klama" })),
        tool_response(
            15,
            "confirm_meaning",
            json!({ "matches": true, "paraphrase_en": "I go." }),
        ),
        tool_response(16, "submit_answer", wrong_answer()),
    ]
}

#[requires(!base_url.trim().is_empty())]
#[ensures(true)]
fn client(base_url: String) -> OpenRouterClient {
    let retry_policy = RetryPolicy::new(0, Duration::from_millis(1)).expect("valid retry policy");
    let config =
        OpenRouterClientConfig::new(base_url, None, retry_policy, 0, Duration::from_secs(2))
            .expect("valid mock client config");
    OpenRouterClient::new(config)
}

#[requires(!name.trim().is_empty())]
#[ensures(ret.is_dir())]
fn temp_directory(name: &str) -> PathBuf {
    let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let executable = std::env::current_exe().expect("current test executable");
    let target_directory = executable
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .expect("Cargo target directory");
    let base = target_directory.join("xarsnu-test-tmp");
    fs::create_dir_all(&base).expect("create target temporary directory");
    let path = base.join(format!(
        "xarsnu-run-{name}-{}-{sequence}",
        std::process::id()
    ));
    fs::create_dir(&path).expect("create isolated temporary directory");
    path
}

#[requires(!scenario.trim().is_empty())]
#[requires(!second_participant.trim().is_empty())]
#[ensures(ret.contains("[[participants]]"))]
fn config_source(scenario: &str, second_participant: &str) -> String {
    format!(
        r#"scenario = "{scenario}"

[caps]
max-parse-attempts-per-turn = 3
max-intent-revisions-per-turn = 2
max-turns = 5
max-cost-usd = 1.0

[[participants]]
name = "alice"
model = "mock/alice"
prompt-caching = "off"
temperature = 0.2
system-prompt = "Alice persona."

[[participants]]
name = "{second_participant}"
model = "mock/{second_participant}"
temperature = 0.3
system-prompt = "Second persona."
"#
    )
}

#[requires(directory.is_dir())]
#[requires(!source.trim().is_empty())]
#[ensures(ret.is_file())]
fn write_config(directory: &Path, source: &str) -> PathBuf {
    let path = directory.join("run.toml");
    fs::write(&path, source).expect("write run config");
    path
}

#[test]
#[requires(true)]
#[ensures(true)]
fn real_run_path_composes_mock_runtime_protocol_scenario_transcript_and_report() {
    let directory = temp_directory("complete");
    let scenario_path = directory.join("scenario.toml");
    fs::copy(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("scenarios")
            .join("schedule-negotiation-1.toml"),
        &scenario_path,
    )
    .expect("copy local scenario");
    let config_path = write_config(&directory, &config_source("scenario.toml", "bob"));
    let server = MockServer::start(complete_dialog_responses());

    let summary =
        run(&config_path, || Ok(client(server.base_url.clone()))).expect("complete live run");

    assert_eq!(
        summary
            .task_outcome
            .as_ref()
            .expect("checker outcome")
            .status,
        TaskStatus::Failure,
        "task failure must remain a successful run result"
    );
    assert_eq!(summary.outcome_line(), "task failed after 3 turn(s)");
    assert!(summary.transcript_path.starts_with(&directory));
    assert_eq!(summary.transcript_path.extension().unwrap(), "jsonl");
    let records = read_transcript(&summary.transcript_path).expect("complete transcript validates");
    assert!(records.iter().any(|record| matches!(
        record.event.as_data(),
        ProtocolEventData::CandidateRejected { .. }
    )));
    assert!(records.iter().any(|record| matches!(
        record.event.as_data(),
        ProtocolEventData::BlindInterpretationRecorded { .. }
    )));
    assert_eq!(
        records
            .iter()
            .filter(|record| matches!(
                record.event.as_data(),
                ProtocolEventData::AnswerSubmitted { .. }
            ))
            .count(),
        2
    );
    let report = report_file(&summary.transcript_path).expect("offline report renders");
    assert!(report.contains("Aggregate status: **failure**"));
    assert!(report.contains("### Blind interpretation"));
    assert!(report.contains("**Gate result:** rejected"));

    let captured = server.finish();
    assert_eq!(captured.len(), 16);
    let first_messages = captured[0].body["messages"]
        .as_array()
        .expect("request messages");
    assert_eq!(first_messages[0]["role"], "system");
    assert!(
        first_messages[0]["content"]
            .as_str()
            .expect("system content")
            .contains("Alice persona.")
    );
    assert_eq!(first_messages[1]["role"], "user");
    let scenario_prompt = first_messages[1]["content"]
        .as_str()
        .expect("scenario prompt");
    assert!(scenario_prompt.contains("Public setup:"));
    assert!(scenario_prompt.contains("Your private brief:"));
    assert!(
        scenario_prompt.contains("Tuesday from 09:00 to 12:00"),
        "{scenario_prompt}"
    );
    assert!(!scenario_prompt.contains("Tuesday from 11:00 to 12:30"));

    fs::remove_dir_all(directory).expect("remove complete-run fixtures");
}

#[test]
#[requires(true)]
#[ensures(true)]
fn participant_mismatch_and_missing_scenario_are_typed() {
    let directory = temp_directory("load-errors");
    let fallback = config_source("schedule-negotiation-1.toml", "carol");
    let config_path = write_config(&directory, &fallback);
    let factory_called = Cell::new(false);
    let error = run(&config_path, || {
        factory_called.set(true);
        Ok(client("http://127.0.0.1:1".to_owned()))
    })
    .expect_err("participant mismatch");
    assert!(!factory_called.get());
    assert!(matches!(
        error.as_data(),
        RunErrorData::ParticipantMismatch {
            configured_only,
            scenario_only,
        } if configured_only == &["carol"] && scenario_only == &["bob"]
    ));
    assert!(error.to_string().contains("`carol`"));
    assert!(error.to_string().contains("`bob`"));

    fs::write(&config_path, config_source("does-not-exist.toml", "bob")).expect("replace config");
    let error = run(&config_path, || Ok(client("http://127.0.0.1:1".to_owned())))
        .expect_err("missing scenario");
    assert!(matches!(
        error.as_data(),
        RunErrorData::ScenarioNotFound { reference, searched }
            if reference == "does-not-exist.toml" && searched.len() == 2
    ));

    fs::remove_dir_all(directory).expect("remove load-error fixtures");
}

#[test]
#[requires(true)]
#[ensures(true)]
fn mid_run_model_death_flushes_an_accepted_runtime_failure_transcript() {
    let directory = temp_directory("runtime-failure");
    let config_path = write_config(
        &directory,
        &config_source("schedule-negotiation-1.toml", "bob"),
    );
    let server = MockServer::start(vec![
        tool_response(
            1,
            "register_intent",
            json!({ "meaning_en": "I can meet on Tuesday." }),
        ),
        tool_response(2, "submit_lojban", json!({ "text": "mi cu" })),
        runtime_failure_response(),
    ]);

    let error =
        run(&config_path, || Ok(client(server.base_url.clone()))).expect_err("runtime failure");
    let transcript_path = error
        .transcript_path()
        .expect("runtime error retains transcript path")
        .to_owned();
    assert!(matches!(error.as_data(), RunErrorData::Protocol { .. }));
    let records = read_transcript(&transcript_path).expect("failure transcript is complete");
    assert!(records.iter().any(|record| matches!(
        record.event.as_data(),
        ProtocolEventData::CandidateRejected { .. }
    )));
    assert!(matches!(
        records.last().expect("terminal record").event.as_data(),
        ProtocolEventData::RunFailed { failure }
            if failure.participant.as_deref() == Some("alice")
                && failure.call_site == RuntimeFailureSite::ModelRequest
                && failure.message.contains("deliberate mid-run failure")
    ));
    let report = report_file(&transcript_path).expect("failure report renders");
    assert!(report.contains("## Runtime failure"));
    assert!(report.contains("Call site: **model request**"));
    assert!(report.contains("Participant: `alice`"));
    assert_eq!(server.finish().len(), 3);

    fs::remove_dir_all(directory).expect("remove failure-run fixtures");
}

#[test]
#[requires(true)]
#[ensures(true)]
fn config_private_brief_is_rejected_before_any_model_call() {
    let directory = temp_directory("removed-brief");
    let source = config_source("schedule-negotiation-1.toml", "bob").replacen(
        "system-prompt = \"Alice persona.\"",
        "system-prompt = \"Alice persona.\"\nprivate-brief = \"obsolete\"",
        1,
    );
    let config_path = write_config(&directory, &source);

    let factory_called = Cell::new(false);
    let error = run(&config_path, || {
        factory_called.set(true);
        Ok(client("http://127.0.0.1:1".to_owned()))
    })
    .expect_err("removed participant field");
    assert!(!factory_called.get());
    assert!(matches!(
        error.as_data(),
        RunErrorData::ConfigInvalid { .. }
    ));
    assert!(error.to_string().contains("unknown field `private-brief`"));

    fs::remove_dir_all(directory).expect("remove removed-brief fixtures");
}

#[test]
#[requires(true)]
#[ensures(true)]
fn prompt_caching_defaults_remain_participant_scoped() {
    let config = xarsnu::RunConfig::from_toml(&config_source("schedule-negotiation-1.toml", "bob"))
        .expect("valid config");
    assert_eq!(config.participants[0].prompt_caching, PromptCaching::Off);
    assert_eq!(config.participants[1].prompt_caching, PromptCaching::Auto);
}
