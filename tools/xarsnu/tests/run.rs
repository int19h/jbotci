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
use xarsnu::protocol::{ProtocolEventData, ProtocolRunOutcomeData, RuntimeFailureSite};
use xarsnu::run::RunErrorData;
use xarsnu::{
    EmbeddingSearchPreflightError, OpenRouterClient, OpenRouterClientConfig, PromptCaching,
    ProtocolRunOutcome, RetryPolicy, RunSummary, TaskStatus, dialog_file, read_transcript,
    report_file, run_with_preflight,
};

static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[invariant((100..=599).contains(status))]
#[invariant(required_message_substring.as_ref().is_none_or(|value| !value.trim().is_empty()))]
#[derive(Debug)]
struct MockResponse {
    status: u16,
    body: Value,
    required_message_substring: Option<String>,
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
                let body = read_request_body(&mut stream);
                if let Some(required) = &response.required_message_substring {
                    assert!(
                        request_has_message(&body, required),
                        "scripted model must not submit before sighting `{required}`"
                    );
                }
                captured.push(new!(CapturedRequest { body }));
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

#[requires(body.is_object())]
#[requires(!needle.trim().is_empty())]
#[ensures(true)]
fn request_has_message(body: &Value, needle: &str) -> bool {
    body["messages"].as_array().is_some_and(|messages| {
        messages.iter().any(|message| {
            message["content"]
                .as_str()
                .is_some_and(|content| content.contains(needle))
        })
    })
}

#[requires(body["messages"].as_array().is_some_and(|messages| !messages.is_empty()))]
#[ensures(!ret.is_empty())]
fn latest_message_content(body: &Value) -> &str {
    body["messages"]
        .as_array()
        .and_then(|messages| messages.last())
        .and_then(|message| message["content"].as_str())
        .expect("latest request message has textual content")
}

#[requires(body.is_object())]
#[requires(!tool_name.trim().is_empty())]
#[ensures(true)]
fn request_offers_tool(body: &Value, tool_name: &str) -> bool {
    body["tools"].as_array().is_some_and(|tools| {
        tools.iter().any(|tool| {
            tool["function"]["name"]
                .as_str()
                .is_some_and(|name| name == tool_name)
        })
    })
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
        required_message_substring: None,
    })
}

#[requires(id > 0)]
#[requires(!name.trim().is_empty())]
#[requires(arguments.is_object())]
#[ensures(ret.status == 200)]
fn reasoning_tool_response(
    id: usize,
    name: &str,
    arguments: Value,
    reasoning_tokens: u64,
) -> MockResponse {
    let response = tool_response(id, name, arguments);
    let mut body = response.body.clone();
    body["choices"][0]["message"]["reasoning"] = json!("private mock reasoning");
    body["choices"][0]["message"]["reasoning_details"] = json!([
        {
            "type": "reasoning.text",
            "text": "private mock detail",
            "signature": "mock-signature"
        }
    ]);
    body["usage"]["completion_tokens_details"] = json!({
        "reasoning_tokens": reasoning_tokens,
    });
    body["usage"]["total_tokens"] = json!(37);
    response.with_data(bityzba::data! { body: body })
}

#[requires(id > 0)]
#[requires(arguments.is_object())]
#[requires(!required_message_substring.trim().is_empty())]
#[ensures(ret.status == 200)]
fn submit_answer_after_sighting(
    id: usize,
    arguments: Value,
    required_message_substring: &str,
) -> MockResponse {
    tool_response(id, "submit_answer", arguments).with_data(bityzba::data! {
        required_message_substring: Some(required_message_substring.to_owned()),
    })
}

#[requires(true)]
#[ensures(ret.status == 500)]
fn runtime_failure_response() -> MockResponse {
    new!(MockResponse {
        status: 500,
        body: json!({ "error": "deliberate mid-run failure" }),
        required_message_substring: None,
    })
}

#[requires(true)]
#[ensures(ret.len() == 17)]
fn complete_dialog_responses() -> Vec<MockResponse> {
    let wrong_answer = || json!({ "day": "tuesday", "start_minute": 600, "duration_minutes": 60 });
    vec![
        reasoning_tool_response(
            1,
            "register_intent",
            json!({ "meaning_en": "I can meet on Tuesday." }),
            7,
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
        tool_response(
            12,
            "register_intent",
            json!({ "meaning_en": "Let us use the proposed time." }),
        ),
        submit_answer_after_sighting(13, wrong_answer(), "in-dialog agreement does not count"),
        tool_response(14, "submit_lojban", json!({ "text": "mi klama" })),
        tool_response(
            15,
            "confirm_meaning",
            json!({ "matches": true, "paraphrase_en": "I go." }),
        ),
        tool_response(
            16,
            "interpret_blind",
            json!({ "interpretation_en": "Alice goes." }),
        ),
        submit_answer_after_sighting(17, wrong_answer(), "in-dialog agreement does not count"),
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
listener-mode = "blind-then-reveal"

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
    let config_source = config_source("scenario.toml", "bob")
        .replace(
            "listener-mode = \"blind-then-reveal\"",
            "listener-mode = \"blind-then-reveal\"\nallow-degraded-search = true",
        )
        .replace("[caps]", "[client]\nhttp-timeout-seconds = 90\n\n[caps]")
        .replace(
            "model = \"mock/alice\"",
            "model = \"mock/alice\"\ntool-choice = \"auto\"",
        )
        .replace(
            "model = \"mock/bob\"",
            "model = \"mock/bob\"\ntool-choice = \"required\"",
        );
    let config_path = write_config(&directory, &config_source);
    let server = MockServer::start(complete_dialog_responses());
    let warning_seen = Cell::new(false);

    let summary = run_with_preflight(
        &config_path,
        |timeout| {
            assert!(
                warning_seen.get(),
                "the degraded-search warning must be surfaced before client initialization"
            );
            assert_eq!(
                timeout,
                Duration::from_secs(90),
                "the run-configured timeout must reach the client factory"
            );
            Ok(client(server.base_url.clone()))
        },
        || {
            Err(EmbeddingSearchPreflightError::unavailable(
                "embedding model is missing; run jbotci setup --embedding".to_owned(),
            ))
        },
        |warning| {
            assert!(warning.to_string().contains("embedding model is missing"));
            warning_seen.set(true);
        },
    )
    .expect("complete live run");

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
    assert_eq!(summary.warnings.len(), 1);
    assert!(
        summary.warnings[0]
            .to_string()
            .contains("embedding model is missing")
    );
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
    assert!(records.iter().any(|record| matches!(
        record.event.as_data(),
        ProtocolEventData::EmbeddingSearchDegraded { message }
            if message.contains("embedding model is missing")
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
    let reasoning_usage = records
        .iter()
        .find_map(|record| match record.event.as_data() {
            ProtocolEventData::UsageRecorded { usage, .. } if usage.reasoning_present => {
                Some(usage)
            }
            _ => None,
        })
        .expect("reasoning usage reaches the transcript");
    assert_eq!(reasoning_usage.prompt_tokens, 10);
    assert_eq!(reasoning_usage.completion_tokens, 2);
    assert_eq!(reasoning_usage.total_tokens, 37);
    assert_eq!(reasoning_usage.reasoning_tokens, Some(7));
    let thinking_trace = records
        .iter()
        .find_map(|record| match record.event.as_data() {
            ProtocolEventData::ThinkingRecorded {
                participant, trace, ..
            } => Some((participant, trace)),
            _ => None,
        })
        .expect("thinking trace reaches the transcript");
    assert_eq!(thinking_trace.0, "alice");
    assert_eq!(
        thinking_trace.1.reasoning.as_deref(),
        Some("private mock reasoning")
    );
    assert_eq!(
        thinking_trace.1.reasoning_details.as_ref().unwrap()[0]["signature"],
        "mock-signature"
    );
    let transcript = fs::read_to_string(&summary.transcript_path).expect("read transcript JSONL");
    assert!(transcript.contains("\"reasoning_present\":true"));
    assert!(transcript.contains("\"total_tokens\":37"));
    assert!(transcript.contains("\"reasoning_tokens\":7"));
    assert!(transcript.contains("private mock reasoning"));
    assert!(transcript.contains("mock-signature"));
    let report = report_file(&summary.transcript_path).expect("offline report renders");
    assert!(report.contains("Aggregate status: **failure**"));
    assert!(report.contains("### Blind interpretation"));
    assert!(report.contains("**Gate result:** rejected"));
    assert!(report.contains("10 prompt + 2 completion = 37 tokens"));
    assert!(report.contains("Reasoning field present: true; reasoning tokens: 7"));
    assert!(report.contains("Reasoning totals: 7 tokens across 1 provider calls"));
    assert!(report.contains("### Thinking — `alice`"));
    assert!(report.contains("> private mock reasoning"));
    assert!(report.contains(">     \"signature\": \"mock-signature\""));
    let dialog = dialog_file(&summary.transcript_path).expect("standalone dialog renders");
    for private_trace in [
        "Thinking",
        "private mock reasoning",
        "private mock detail",
        "mock-signature",
    ] {
        assert!(
            !dialog.contains(private_trace),
            "standalone dialog leaked {private_trace}"
        );
    }

    let captured = server.finish();
    assert_eq!(captured.len(), 17);
    let replayed = captured[1].body["messages"]
        .as_array()
        .expect("continuing-loop messages")
        .iter()
        .find(|message| message["tool_calls"][0]["id"] == "call-1-register_intent")
        .expect("originating assistant tool call");
    assert_eq!(
        replayed["reasoning_details"][0]["signature"], "mock-signature",
        "the continuing speaker tool loop must replay observed details"
    );
    assert!(
        captured
            .iter()
            .all(|request| !request.body.to_string().contains("private mock reasoning")),
        "unstructured reasoning must never enter re-sent history"
    );
    assert!(
        captured[11].body["messages"]
            .as_array()
            .expect("new-turn messages")
            .iter()
            .all(|message| message.get("reasoning_details").is_none()),
        "reasoning details must not cross the next tool-loop boundary"
    );
    for index in [0, 1, 2, 3, 9, 10, 11, 12, 13, 14] {
        assert_eq!(captured[index].body["tool_choice"], "auto");
    }
    for index in [4, 5, 6, 7, 8, 15, 16] {
        assert_eq!(captured[index].body["tool_choice"], "required");
    }
    let confirm_request = &captured[3].body;
    assert!(request_offers_tool(confirm_request, "confirm_meaning"));
    for required_precision_language in [
        "matches=true only when the tersmu rendering precisely expresses your currently intended message",
        "every predicate relation as rendered, under its dictionary place structure",
        "Calques or idioms from other languages (malgli) are mismatches",
        "physically chases another",
        "call matches=false and recompose",
        "the gist is right are not the standard",
        "the rendering is the meaning that will be scored",
        "you MUST call register_intent again with the revised meaning BEFORE confirming",
        "re-declaring is cheap and encouraged",
        "a target you would have been willing to commit to before you saw this rendering",
        "Deriving the new intent FROM the tersmu output so that it will match is not a revision, it is a mismatch",
        "never a way to make a wrong rendering count as right",
        "Confirming while noting a discrepancy between your intent and the rendering is a contradiction",
        "never for waiving a known mismatch",
    ] {
        assert!(
            request_has_message(confirm_request, required_precision_language),
            "confirm-phase request omitted `{required_precision_language}`"
        );
    }
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
    let standing_system_prompt = first_messages[0]["content"]
        .as_str()
        .expect("system content");
    for standing_doctrine in [
        "built-in knowledge of Lojban vocabulary is flawed",
        "Before choosing any content word you are not certain of",
        "search vlacku BY MEANING",
        "semantic or definition search",
        "compare candidates",
        "do not merely look up a word you already picked and rationalize its definition",
        "when the problem is HOW TO EXPRESS something grammatically (not which word), query cukta with the concept.",
        "You may freely test candidate Lojban with the reference tersmu tool while composing (drafting draws on your reference budget); submit_lojban is the act of commitment and its attempts are limited — submit when you believe the candidate is right.",
        "When composing or checking any bridi, attend closely to argument assignment: verify from the definition the precise meaning AND TYPE of every place of the selbri you use (object, agent, event, property, proposition, quantity), and ensure each sumti matches its place's expected type — a place expecting conduct-as-event or a property cannot take the entity affected. A type-mismatched place is a mismatch at confirm time even if a reader would understand.",
    ] {
        assert!(
            standing_system_prompt.contains(standing_doctrine),
            "wire system prompt omitted `{standing_doctrine}`"
        );
    }
    let cukta_description = captured[0].body["tools"]
        .as_array()
        .expect("request tools")
        .iter()
        .find(|tool| tool["function"]["name"] == "cukta")
        .and_then(|tool| tool["function"]["description"].as_str())
        .expect("wire cukta description");
    assert!(cukta_description.contains("Concept and meaning queries are supported"));
    assert!(cukta_description.contains("often the right choice for grammar questions"));
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

    let availability = "You may now submit your scenario answer with `submit_answer`. The task is scored only from formal submissions; in-dialog agreement does not count.";
    for request in &captured[..11] {
        assert!(
            !request_has_message(&request.body, availability),
            "answer announcement appeared before minimum-round availability"
        );
    }

    let alice_turn_three_opening = latest_message_content(&captured[11].body);
    assert!(alice_turn_three_opening.contains("This is turn 3 of at most 5."));
    assert!(alice_turn_three_opening.contains(availability));
    assert!(request_offers_tool(&captured[11].body, "submit_answer"));

    let alice_composing = latest_message_content(&captured[12].body);
    assert!(alice_composing.starts_with("Intent registered. Compose Lojban"));
    assert!(alice_composing.contains(availability));
    assert!(request_offers_tool(&captured[12].body, "submit_answer"));

    let alice_recorded = latest_message_content(&captured[13].body);
    assert!(
        alice_recorded.contains("Your answer is recorded; 1 of 2 participants have submitted.")
    );
    assert!(!alice_recorded.contains(availability));
    assert!(!request_offers_tool(&captured[13].body, "submit_answer"));

    let bob_turn_three_blind = latest_message_content(&captured[15].body);
    assert!(bob_turn_three_blind.contains("This is turn 3 of at most 5."));
    assert!(!bob_turn_three_blind.contains(availability));
    assert!(
        request_offers_tool(&captured[15].body, "submit_answer"),
        "blind-phase tool legality must remain unchanged"
    );

    let bob_revealed = latest_message_content(&captured[16].body);
    assert!(bob_revealed.starts_with("The tersmu rendering is now revealed."));
    assert!(bob_revealed.contains(availability));
    assert!(!bob_revealed.contains("This is turn 3 of at most 5."));
    assert!(request_offers_tool(&captured[16].body, "submit_answer"));

    assert!(latest_message_content(&captured[0].body).contains("This is turn 1 of at most 5."));
    assert!(latest_message_content(&captured[4].body).contains("This is turn 1 of at most 5."));
    assert!(latest_message_content(&captured[6].body).contains("This is turn 2 of at most 5."));
    assert!(latest_message_content(&captured[9].body).contains("This is turn 2 of at most 5."));

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
    let error = run_with_preflight(
        &config_path,
        |_| {
            factory_called.set(true);
            Ok(client("http://127.0.0.1:1".to_owned()))
        },
        || Ok(()),
        |_| {},
    )
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
    let error = run_with_preflight(
        &config_path,
        |_| Ok(client("http://127.0.0.1:1".to_owned())),
        || Ok(()),
        |_| {},
    )
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

    let error = run_with_preflight(
        &config_path,
        |_| Ok(client(server.base_url.clone())),
        || Ok(()),
        |_| {},
    )
    .expect_err("runtime failure");
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
    let error = run_with_preflight(
        &config_path,
        |_| {
            factory_called.set(true);
            Ok(client("http://127.0.0.1:1".to_owned()))
        },
        || Ok(()),
        |_| {},
    )
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
fn embedding_preflight_fails_before_client_initialization_by_default() {
    let directory = temp_directory("embedding-preflight");
    let config_path = write_config(
        &directory,
        &config_source("schedule-negotiation-1.toml", "bob"),
    );
    let factory_called = Cell::new(false);

    let error = run_with_preflight(
        &config_path,
        |_| {
            factory_called.set(true);
            Ok(client("http://127.0.0.1:1".to_owned()))
        },
        || {
            Err(EmbeddingSearchPreflightError::unavailable(
                "embedding model is missing; run jbotci setup --embedding".to_owned(),
            ))
        },
        |_| panic!("fatal preflight failures must not emit override warnings"),
    )
    .expect_err("missing embedding assets must stop the run");

    assert!(!factory_called.get());
    assert!(matches!(
        error.as_data(),
        RunErrorData::EmbeddingSearchUnavailable { message }
            if message.contains("embedding model is missing")
    ));
    assert!(error.to_string().contains("allow-degraded-search = true"));
    assert!(error.to_string().contains("jbotci setup --embedding"));

    fs::remove_dir_all(directory).expect("remove embedding-preflight fixtures");
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

#[test]
#[requires(true)]
#[ensures(true)]
fn unscored_completion_has_a_dialog_outcome_line() {
    let summary = new!(RunSummary {
        transcript_path: PathBuf::from("debate.jsonl"),
        outcome: new!(ProtocolRunOutcome::Completed { turns: 10 }),
        task_outcome: None,
        warnings: Vec::new(),
    });

    assert_eq!(summary.outcome_line(), "dialog completed after 10 turns");
}
