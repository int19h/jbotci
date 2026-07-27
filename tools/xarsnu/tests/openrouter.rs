use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

#[allow(unused_imports)]
use bityzba::{contract_trait, ensures, invariant, requires};
use serde_json::{Value, json};
use xarsnu::{
    AbortKind, OpenRouterClient, OpenRouterClientConfig, OpenRouterError, ParticipantConversation,
    PromptCaching, ProviderToolChoice, ProviderUsageValidationError, ReasoningConfig, RetryPolicy,
    RunAccounting, RunConfig, ToolCall, ToolDefinition, ToolDispatchError, ToolDispatcher, Usage,
};

static NEXT_DUMP_DIRECTORY: AtomicU64 = AtomicU64::new(1);

#[requires(true)]
#[ensures(!ret.as_os_str().is_empty())]
fn request_dump_directory() -> PathBuf {
    let number = NEXT_DUMP_DIRECTORY.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "xarsnu-openrouter-dump-test-{}-{number}",
        std::process::id()
    ))
}

#[invariant(true)]
#[derive(Debug)]
struct MockResponse {
    status: u16,
    body: Value,
}

#[invariant(true)]
#[derive(Debug)]
struct CapturedRequest {
    body: Value,
    body_bytes: Vec<u8>,
    received_at: Instant,
}

#[invariant(true)]
#[derive(Debug)]
struct MockServer {
    base_url: String,
    expected_requests: usize,
    worker: JoinHandle<Vec<CapturedRequest>>,
}

impl MockServer {
    #[requires(!responses.is_empty())]
    #[ensures(!ret.base_url.is_empty())]
    fn start(responses: Vec<MockResponse>) -> Self {
        Self::start_scheduled(
            responses
                .into_iter()
                .map(|response| (response, Duration::ZERO))
                .collect(),
        )
    }

    #[requires(!responses.is_empty())]
    #[ensures(!ret.base_url.is_empty())]
    fn start_scheduled(responses: Vec<(MockResponse, Duration)>) -> Self {
        let expected_requests = responses.len();
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind local mock server");
        let address = listener.local_addr().expect("mock server address");
        let worker = thread::spawn(move || {
            let mut captured = Vec::with_capacity(responses.len());
            for (response, body_delay) in responses {
                let (mut stream, _) = listener.accept().expect("accept mock request");
                let body_bytes = read_request_body(&mut stream);
                let body = serde_json::from_slice(&body_bytes).expect("JSON completion request");
                captured.push(CapturedRequest {
                    body,
                    body_bytes,
                    received_at: Instant::now(),
                });
                write_json_response(&mut stream, response, body_delay);
            }
            captured
        });
        Self {
            base_url: format!("http://{address}"),
            expected_requests,
            worker,
        }
    }

    #[requires(!truncated_body.is_empty())]
    #[ensures(ret.expected_requests == 2)]
    fn start_truncated_then_complete(
        truncated_body: Vec<u8>,
        complete_response: MockResponse,
    ) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind local mock server");
        let address = listener.local_addr().expect("mock server address");
        let worker = thread::spawn(move || {
            let mut captured = Vec::with_capacity(2);
            for response in [None, Some(complete_response)] {
                let (mut stream, _) = listener.accept().expect("accept mock request");
                let body_bytes = read_request_body(&mut stream);
                let body = serde_json::from_slice(&body_bytes).expect("JSON completion request");
                captured.push(CapturedRequest {
                    body,
                    body_bytes,
                    received_at: Instant::now(),
                });
                if let Some(response) = response {
                    write_json_response(&mut stream, response, Duration::ZERO);
                } else {
                    write_raw_json_response(&mut stream, &truncated_body);
                }
            }
            captured
        });
        Self {
            base_url: format!("http://{address}"),
            expected_requests: 2,
            worker,
        }
    }

    #[requires(true)]
    #[ensures(ret.len() == self.expected_requests)]
    fn finish(self) -> Vec<CapturedRequest> {
        self.worker.join().expect("mock server thread")
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn read_request_body(stream: &mut TcpStream) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut buffer = [0u8; 4096];
    let header_end = loop {
        let count = stream.read(&mut buffer).expect("read mock request");
        assert!(count > 0, "request ended before its headers");
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
            name.eq_ignore_ascii_case("content-length").then(|| {
                value
                    .trim()
                    .parse::<usize>()
                    .expect("numeric content length")
            })
        })
        .expect("content-length header");
    while bytes.len() - header_end < content_length {
        let count = stream.read(&mut buffer).expect("read mock request body");
        assert!(count > 0, "request ended before its body");
        bytes.extend_from_slice(&buffer[..count]);
    }
    bytes[header_end..header_end + content_length].to_vec()
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
fn write_json_response(stream: &mut TcpStream, response: MockResponse, body_delay: Duration) {
    let body = response.body.to_string();
    let reason = match response.status {
        200 => "OK",
        401 => "Unauthorized",
        429 => "Too Many Requests",
        _ => "Server Error",
    };
    write!(
        stream,
        "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        response.status,
        reason,
        body.len(),
    )
    .expect("write mock response headers");
    stream.flush().expect("flush mock response headers");
    thread::sleep(body_delay);
    if body_delay.is_zero() {
        stream
            .write_all(body.as_bytes())
            .expect("write mock response body");
        stream.flush().expect("flush mock response body");
    } else {
        // A timeout test deliberately lets the client close this socket first.
        let _ = stream.write_all(body.as_bytes());
        let _ = stream.flush();
    }
}

#[requires(!body.is_empty())]
#[ensures(true)]
fn write_raw_json_response(stream: &mut TcpStream, body: &[u8]) {
    write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len(),
    )
    .expect("write raw mock response headers");
    stream
        .write_all(body)
        .expect("write raw mock response body");
    stream.flush().expect("flush raw mock response");
}

#[requires(!name.trim().is_empty())]
#[requires(cost.is_finite() && cost >= 0.0)]
#[ensures(ret.status == 200)]
fn tool_call_response(name: &str, cost: f64) -> MockResponse {
    tool_calls_response(&[name], cost)
}

#[requires(!name.trim().is_empty())]
#[requires(cost.is_finite() && cost >= 0.0)]
#[requires(cached_tokens <= 11)]
#[ensures(ret.status == 200)]
fn cached_tool_call_response(
    name: &str,
    cost: f64,
    cached_tokens: u64,
    cache_write_tokens: u64,
) -> MockResponse {
    let mut response = tool_call_response(name, cost);
    response.body["usage"]["prompt_tokens_details"] = json!({
        "cached_tokens": cached_tokens,
    });
    response.body["usage"]["cache_write_tokens"] = json!(cache_write_tokens);
    response
}

#[requires(!names.is_empty() && names.iter().all(|name| !name.trim().is_empty()))]
#[requires(cost.is_finite() && cost >= 0.0)]
#[ensures(ret.status == 200)]
fn tool_calls_response(names: &[&str], cost: f64) -> MockResponse {
    let tool_calls = names
        .iter()
        .map(|name| {
            json!({
                "id": format!("call-{name}"),
                "type": "function",
                "function": {
                    "name": name,
                    "arguments": "{\"value\":1}"
                }
            })
        })
        .collect::<Vec<_>>();
    MockResponse {
        status: 200,
        body: json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": null,
                    "tool_calls": tool_calls
                }
            }],
            "usage": {
                "prompt_tokens": 11,
                "completion_tokens": 7,
                "total_tokens": 18,
                "prompt_tokens_details": null,
                "cache_write_tokens": null,
                "cost": cost
            }
        }),
    }
}

#[requires(!content.is_empty())]
#[requires(cost.is_finite() && cost >= 0.0)]
#[ensures(ret.status == 200)]
fn prose_response(content: &str, cost: f64) -> MockResponse {
    MockResponse {
        status: 200,
        body: json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": content
                }
            }],
            "usage": {
                "prompt_tokens": 3,
                "completion_tokens": 2,
                "total_tokens": 5,
                "cost": cost
            }
        }),
    }
}

#[requires(!message.trim().is_empty())]
#[ensures(ret.status == 200)]
fn provider_error_response(code: u16, message: &str) -> MockResponse {
    MockResponse {
        status: 200,
        body: json!({
            "error": {
                "code": code,
                "message": message,
                "metadata": {
                    "provider_name": "mock-provider"
                }
            }
        }),
    }
}

#[requires(!reasoning.trim().is_empty())]
#[requires(reasoning_tokens <= 4)]
#[ensures(ret.status == 200)]
fn reasoning_only_response(reasoning: &str, reasoning_tokens: u64) -> MockResponse {
    MockResponse {
        status: 200,
        body: json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": null,
                    "reasoning": reasoning
                }
            }],
            "usage": {
                "prompt_tokens": 3,
                "completion_tokens": 4,
                "total_tokens": 7,
                "completion_tokens_details": {
                    "reasoning_tokens": reasoning_tokens
                },
                "cost": 0.01
            }
        }),
    }
}

#[requires(!base_url.trim().is_empty())]
#[ensures(true)]
fn client(
    base_url: String,
    max_retries: usize,
    backoff: Duration,
    max_reprompts: usize,
) -> OpenRouterClient {
    client_with_timeout(
        base_url,
        max_retries,
        backoff,
        max_reprompts,
        Duration::from_secs(2),
    )
}

#[requires(!base_url.trim().is_empty())]
#[requires(timeout > Duration::ZERO)]
#[ensures(true)]
fn client_with_timeout(
    base_url: String,
    max_retries: usize,
    backoff: Duration,
    max_reprompts: usize,
    timeout: Duration,
) -> OpenRouterClient {
    let retry_policy = RetryPolicy::new(max_retries, backoff).expect("valid retry policy");
    let config = OpenRouterClientConfig::new(base_url, None, retry_policy, max_reprompts, timeout)
        .expect("valid mock client config");
    OpenRouterClient::new(config)
}

#[requires(true)]
#[ensures(ret.messages().len() == 2)]
fn conversation() -> ParticipantConversation {
    conversation_for_model("mock/model", PromptCaching::Auto)
}

#[requires(!model.trim().is_empty())]
#[ensures(ret.messages().len() == 2)]
fn conversation_for_model(model: &str, prompt_caching: PromptCaching) -> ParticipantConversation {
    conversation_for_model_with_reasoning(model, prompt_caching, ReasoningConfig::Default)
}

#[requires(!model.trim().is_empty())]
#[ensures(ret.messages().len() == 2)]
fn conversation_for_model_with_reasoning(
    model: &str,
    prompt_caching: PromptCaching,
    reasoning: ReasoningConfig,
) -> ParticipantConversation {
    ParticipantConversation::from_parts(
        "tester".to_owned(),
        model.to_owned(),
        None,
        prompt_caching,
        reasoning,
        0.3,
        "Use tools.".to_owned(),
        "Private task.".to_owned(),
    )
}

#[requires(true)]
#[ensures(true)]
fn take_pending_usage(conversation: &mut ParticipantConversation) -> Vec<Usage> {
    conversation
        .take_pending_observations()
        .into_iter()
        .map(|observation| observation.usage)
        .collect()
}

#[requires(!name.trim().is_empty())]
#[ensures(ret.as_ref().is_ok_and(|tool| tool.name() == name) || ret.is_err())]
fn tool(name: &str) -> Result<ToolDefinition, xarsnu::ToolDefinitionError> {
    ToolDefinition::new(
        name.to_owned(),
        format!("Call {name}"),
        json!({
            "type": "object",
            "properties": { "value": { "type": "integer" } },
            "required": ["value"],
            "additionalProperties": false
        }),
    )
}

#[invariant(true)]
#[derive(Debug, Default)]
struct ExactDispatcher;

#[contract_trait]
impl ToolDispatcher for ExactDispatcher {
    fn dispatch(&mut self, _call: &ToolCall) -> Result<String, ToolDispatchError> {
        Ok("  exact tool payload\n".to_owned())
    }
}

#[invariant(true)]
#[derive(Debug, Default)]
struct FirstCallFails {
    attempts: usize,
}

#[contract_trait]
impl ToolDispatcher for FirstCallFails {
    fn dispatch(&mut self, call: &ToolCall) -> Result<String, ToolDispatchError> {
        self.attempts += 1;
        Err(ToolDispatchError::new(
            call.function.name.clone(),
            "deliberate test failure".to_owned(),
        ))
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn base_url_override_routes_completion_to_configured_server() {
    let server = MockServer::start(vec![tool_call_response("alpha", 0.01)]);
    let retry_policy = RetryPolicy::new(0, Duration::from_millis(1)).expect("valid retry policy");
    let config = OpenRouterClientConfig::new(
        "http://127.0.0.1:1".to_owned(),
        None,
        retry_policy,
        0,
        Duration::from_secs(2),
    )
    .expect("valid initial client config")
    .with_base_url(&server.base_url)
    .expect("valid base URL override");
    let client = OpenRouterClient::new(config);
    let mut conversation = conversation();
    let mut accounting = RunAccounting::new(1.0).expect("valid budget");

    conversation
        .request(
            &client,
            &[tool("alpha").expect("valid tool")],
            ProviderToolChoice::Required,
            &mut accounting,
        )
        .expect("completion reaches overridden base URL");

    assert_eq!(server.finish().len(), 1);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn happy_tool_call_accounts_usage_and_threads_exact_result() {
    let server = MockServer::start(vec![tool_call_response("alpha", 0.125)]);
    let client = client(server.base_url.clone(), 0, Duration::from_millis(1), 0);
    let mut conversation = conversation();
    let mut accounting = RunAccounting::new(1.0).expect("valid budget");
    let turn = conversation
        .request(
            &client,
            &[tool("alpha").expect("valid tool")],
            ProviderToolChoice::Required,
            &mut accounting,
        )
        .expect("happy tool call");
    let calls = turn.tool_calls().expect("expected tool calls");
    assert_eq!(calls[0].function.name, "alpha");
    let mut dispatcher = ExactDispatcher;
    conversation
        .dispatch_tool_calls(calls, &mut dispatcher)
        .expect("dispatch result");
    let last = serde_json::to_value(conversation.messages().last().expect("tool result"))
        .expect("serialize message");
    assert_eq!(last["role"], "tool");
    assert_eq!(last["tool_call_id"], "call-alpha");
    assert_eq!(last["content"], "  exact tool payload\n");
    assert_eq!(conversation.usage().total_tokens, 18);
    assert_eq!(accounting.usage().cost_usd, 0.125);
    let usage = take_pending_usage(&mut conversation);
    assert_eq!(usage.len(), 1);
    assert_eq!(usage[0].cost, 0.125);
    assert_eq!(usage[0].cached_tokens, None);
    assert_eq!(usage[0].cache_write_tokens, None);
    assert!(!usage[0].reasoning_present);
    assert_eq!(usage[0].reasoning_tokens, None);
    assert!(take_pending_usage(&mut conversation).is_empty());
    let captured = server.finish();
    assert_eq!(captured[0].body["tool_choice"], "required");
    assert_eq!(captured[0].body["usage"], json!({ "include": true }));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn provider_cache_usage_normalizes_and_round_trips_through_record_json() {
    let server = MockServer::start(vec![cached_tool_call_response("alpha", 0.01, 8, 3)]);
    let client = client(server.base_url.clone(), 0, Duration::from_millis(1), 0);
    let mut conversation = conversation();
    let mut accounting = RunAccounting::new(1.0).expect("valid budget");

    conversation
        .request(
            &client,
            &[tool("alpha").expect("valid tool")],
            ProviderToolChoice::Required,
            &mut accounting,
        )
        .expect("mock completion succeeds");

    let usage = take_pending_usage(&mut conversation);
    assert_eq!(usage.len(), 1);
    assert_eq!(usage[0].cached_tokens, Some(8));
    assert_eq!(usage[0].cache_write_tokens, Some(3));
    assert_eq!(conversation.usage().cached_tokens, 8);
    assert_eq!(conversation.usage().cache_write_tokens, 3);
    assert_eq!(conversation.usage().provider_calls, 1);
    assert_eq!(conversation.usage().cache_hit_calls, 1);
    assert_eq!(conversation.usage().cache_efficiency(), Some(8.0 / 11.0));
    assert_eq!(conversation.usage().cache_hit_rate(), Some(1.0));

    let record_json = serde_json::to_value(&usage[0]).expect("usage record serializes");
    assert_eq!(record_json["cached_tokens"], 8);
    assert_eq!(record_json["cache_write_tokens"], 3);
    assert!(record_json.get("prompt_tokens_details").is_none());
    assert!(record_json.get("reasoning_present").is_none());
    assert!(record_json.get("reasoning_tokens").is_none());
    let round_tripped: xarsnu::Usage =
        serde_json::from_value(record_json).expect("usage record deserializes");
    assert_eq!(round_tripped, usage[0]);

    assert_eq!(server.finish().len(), 1);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn provider_accounting_variance_is_recorded_verbatim() {
    let mut response = tool_call_response("alpha", 0.01);
    response.body["usage"] = json!({
        "prompt_tokens": 3,
        "completion_tokens": 2,
        "total_tokens": 41,
        "prompt_tokens_details": {
            "cached_tokens": 5
        },
        "completion_tokens_details": {
            "reasoning_tokens": 17
        },
        "cache_write_tokens": 7,
        "cost": 0.01
    });
    let server = MockServer::start(vec![response]);
    let client = client(server.base_url.clone(), 0, Duration::from_millis(1), 0);
    let mut conversation = conversation();
    let mut accounting = RunAccounting::new(1.0).expect("valid budget");

    conversation
        .request(
            &client,
            &[tool("alpha").expect("valid tool")],
            ProviderToolChoice::Required,
            &mut accounting,
        )
        .expect("provider accounting conventions are accepted");

    let usage = take_pending_usage(&mut conversation);
    assert_eq!(usage.len(), 1);
    assert_eq!(usage[0].prompt_tokens, 3);
    assert_eq!(usage[0].completion_tokens, 2);
    assert_eq!(usage[0].total_tokens, 41);
    assert_eq!(usage[0].cached_tokens, Some(5));
    assert_eq!(usage[0].cache_write_tokens, Some(7));
    assert_eq!(usage[0].reasoning_tokens, Some(17));
    assert_eq!(conversation.usage().cache_efficiency(), Some(5.0 / 3.0));

    let record_json = serde_json::to_value(&usage[0]).expect("usage record serializes");
    let round_tripped: xarsnu::Usage =
        serde_json::from_value(record_json).expect("usage record deserializes");
    assert_eq!(round_tripped, usage[0]);
    assert_eq!(server.finish().len(), 1);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn invalid_provider_cost_names_only_the_failing_usage_clause() {
    let mut response = tool_call_response("alpha", 0.01);
    response.body["usage"]["cost"] = json!(-0.01);
    let server = MockServer::start(vec![response]);
    let client = client(server.base_url.clone(), 0, Duration::from_millis(1), 0);
    let mut conversation = conversation();
    let mut accounting = RunAccounting::new(1.0).expect("valid budget");

    let error = conversation
        .request(
            &client,
            &[tool("alpha").expect("valid tool")],
            ProviderToolChoice::Required,
            &mut accounting,
        )
        .expect_err("negative provider cost must fail structural validation");

    assert_eq!(
        error,
        OpenRouterError::InvalidProviderUsage {
            reason: ProviderUsageValidationError::CostMustBeFiniteAndNonnegative,
        }
    );
    assert_eq!(
        error.to_string(),
        "invalid OpenRouter provider usage: reported cost must be finite and nonnegative"
    );
    assert_eq!(server.finish().len(), 1);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn default_reasoning_request_has_a_stable_wire_shape() {
    let server = MockServer::start(vec![tool_call_response("alpha", 0.01)]);
    let client = client(server.base_url.clone(), 0, Duration::from_millis(1), 0);
    let mut conversation = conversation();
    let mut accounting = RunAccounting::new(1.0).expect("valid budget");

    conversation
        .request(
            &client,
            &[tool("alpha").expect("valid tool")],
            ProviderToolChoice::Required,
            &mut accounting,
        )
        .expect("mock completion succeeds");

    let captured = server.finish();
    assert_eq!(
        captured[0].body_bytes,
        br#"{"model":"mock/model","temperature":0.3,"max_tokens":16384,"messages":[{"role":"system","content":"Use tools."},{"role":"user","content":"Private task."}],"tools":[{"type":"function","function":{"name":"alpha","description":"Call alpha","parameters":{"type":"object","properties":{"value":{"type":"integer"}},"required":["value"],"additionalProperties":false}}}],"tool_choice":"required","reasoning":{"enabled":true,"exclude":false,"summary":"detailed"},"usage":{"include":true}}"#
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn request_dump_records_exact_retry_bodies_and_statuses_without_overwriting() {
    let server = MockServer::start(vec![
        MockResponse {
            status: 429,
            body: json!({ "error": "retry" }),
        },
        tool_call_response("alpha", 0.01),
    ]);
    let directory = request_dump_directory();
    fs::create_dir_all(&directory).expect("create test dump directory");
    fs::write(directory.join("000001-request.json"), b"existing request\n")
        .expect("seed existing request dump");
    fs::write(
        directory.join("000001-response.json"),
        b"existing response\n",
    )
    .expect("seed existing response dump");

    let retry_policy = RetryPolicy::new(1, Duration::from_millis(1)).expect("valid retry policy");
    let config = OpenRouterClientConfig::new(
        server.base_url.clone(),
        None,
        retry_policy,
        0,
        Duration::from_secs(2),
    )
    .expect("valid mock client config")
    .with_request_dump_directory(directory.clone())
    .expect("valid request dump directory");
    let client = OpenRouterClient::new(config);
    let mut conversation = conversation();
    let mut accounting = RunAccounting::new(1.0).expect("valid budget");

    conversation
        .request(
            &client,
            &[tool("alpha").expect("valid tool")],
            ProviderToolChoice::Required,
            &mut accounting,
        )
        .expect("retry succeeds");

    let captured = server.finish();
    assert_eq!(captured.len(), 2);
    assert_eq!(
        fs::read(directory.join("000001-request.json")).expect("read existing request"),
        b"existing request\n"
    );
    assert_eq!(
        fs::read(directory.join("000001-response.json")).expect("read existing response"),
        b"existing response\n"
    );
    assert_eq!(
        fs::read(directory.join("000002-request.json")).expect("read first dumped request"),
        captured[0].body_bytes
    );
    assert_eq!(
        fs::read(directory.join("000003-request.json")).expect("read retried request"),
        captured[1].body_bytes
    );
    let first_response: Value = serde_json::from_slice(
        &fs::read(directory.join("000002-response.json")).expect("read first response status"),
    )
    .expect("first response status JSON");
    let second_response: Value = serde_json::from_slice(
        &fs::read(directory.join("000003-response.json")).expect("read second response status"),
    )
    .expect("second response status JSON");
    assert_eq!(first_response["status"], 429);
    assert_eq!(
        serde_json::from_str::<Value>(
            first_response["body"]
                .as_str()
                .expect("first response body text")
        )
        .expect("first response body JSON"),
        json!({ "error": "retry" })
    );
    assert_eq!(second_response["status"], 200);
    assert_eq!(
        serde_json::from_str::<Value>(
            second_response["body"]
                .as_str()
                .expect("second response body text")
        )
        .expect("second response body JSON")["choices"][0]["message"]["tool_calls"][0]["function"]
            ["arguments"],
        "{\"value\":1}"
    );
    assert_eq!(
        fs::read_dir(&directory)
            .expect("list request dumps")
            .count(),
        6
    );

    fs::remove_dir_all(directory).expect("remove test dump directory");
}

#[test]
#[requires(true)]
#[ensures(true)]
fn participant_provider_table_reaches_the_wire_byte_exact() {
    let config = RunConfig::from_toml(
        r#"
scenario = "fixture.toml"

[caps]
max-parse-attempts-per-turn = 3
max-intent-revisions-per-turn = 2
max-turns = 4
max-cost-usd = 1.0

[[participants]]
name = "tester"
model = "mock/model"
tool-choice = "required"
reasoning = "default"
temperature = 0.3
system-prompt = "Use tools."

[participants.provider]
only = ["xiaomi/fp8"]
order = ["xiaomi/fp8", "fallback/example"]
ignore = ["broken/intermediary"]

[[participants]]
name = "observer"
model = "mock/observer"
temperature = 0.4
system-prompt = "Observe."
"#,
    )
    .expect("provider routing config parses");
    let server = MockServer::start(vec![tool_call_response("alpha", 0.01)]);
    let client = client(server.base_url.clone(), 0, Duration::from_millis(1), 0);
    let mut conversation = ParticipantConversation::new(&config.participants[0]);
    conversation.push_user("Private task.".to_owned());
    let mut accounting = RunAccounting::new(1.0).expect("valid budget");

    conversation
        .request(
            &client,
            &[tool("alpha").expect("valid tool")],
            ProviderToolChoice::Required,
            &mut accounting,
        )
        .expect("mock completion succeeds");

    let captured = server.finish();
    assert_eq!(
        captured[0].body_bytes,
        br#"{"model":"mock/model","provider":{"ignore":["broken/intermediary"],"only":["xiaomi/fp8"],"order":["xiaomi/fp8","fallback/example"]},"temperature":0.3,"max_tokens":16384,"messages":[{"role":"system","content":"Use tools."},{"role":"user","content":"Private task."}],"tools":[{"type":"function","function":{"name":"alpha","description":"Call alpha","parameters":{"type":"object","properties":{"value":{"type":"integer"}},"required":["value"],"additionalProperties":false}}}],"tool_choice":"required","reasoning":{"enabled":true,"exclude":false,"summary":"detailed"},"usage":{"include":true}}"#
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn serving_provider_is_captured_and_round_trips_with_usage() {
    let mut response = tool_call_response("alpha", 0.01);
    response.body["provider"] = json!("xiaomi/fp8");
    let server = MockServer::start(vec![response]);
    let client = client(server.base_url.clone(), 0, Duration::from_millis(1), 0);
    let mut conversation = conversation();
    let mut accounting = RunAccounting::new(1.0).expect("valid budget");

    conversation
        .request(
            &client,
            &[tool("alpha").expect("valid tool")],
            ProviderToolChoice::Required,
            &mut accounting,
        )
        .expect("mock completion succeeds");

    let usage = take_pending_usage(&mut conversation);
    assert_eq!(usage[0].provider.as_deref(), Some("xiaomi/fp8"));
    assert_eq!(
        conversation
            .usage()
            .provider_calls_by_name
            .get("xiaomi/fp8"),
        Some(&1)
    );
    let record_json = serde_json::to_value(&usage[0]).expect("usage record serializes");
    assert_eq!(record_json["provider"], "xiaomi/fp8");
    let round_tripped: Usage =
        serde_json::from_value(record_json).expect("usage record deserializes");
    assert_eq!(round_tripped, usage[0]);
    assert_eq!(server.finish().len(), 1);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn automatic_tool_choice_reaches_the_request_wire() {
    let server = MockServer::start(vec![tool_call_response("alpha", 0.01)]);
    let client = client(server.base_url.clone(), 0, Duration::from_millis(1), 0);
    let mut conversation = conversation();
    let mut accounting = RunAccounting::new(1.0).expect("valid budget");

    conversation
        .request(
            &client,
            &[tool("alpha").expect("valid tool")],
            ProviderToolChoice::Auto,
            &mut accounting,
        )
        .expect("mock completion succeeds");

    let captured = server.finish();
    assert_eq!(captured[0].body["tool_choice"], "auto");
}

#[test]
#[requires(true)]
#[ensures(true)]
fn every_reasoning_mode_reaches_the_wire_with_the_openrouter_shape() {
    for (reasoning, expected) in [
        (
            ReasoningConfig::Off,
            json!({ "effort": "none", "exclude": false, "summary": "detailed" }),
        ),
        (
            ReasoningConfig::Default,
            json!({ "enabled": true, "exclude": false, "summary": "detailed" }),
        ),
        (
            ReasoningConfig::Low,
            json!({ "effort": "low", "exclude": false, "summary": "detailed" }),
        ),
        (
            ReasoningConfig::Medium,
            json!({ "effort": "medium", "exclude": false, "summary": "detailed" }),
        ),
        (
            ReasoningConfig::High,
            json!({ "effort": "high", "exclude": false, "summary": "detailed" }),
        ),
    ] {
        let server = MockServer::start(vec![tool_call_response("alpha", 0.01)]);
        let client = client(server.base_url.clone(), 0, Duration::from_millis(1), 0);
        let mut conversation = conversation_for_model_with_reasoning(
            "xiaomi/mimo-v2.5",
            PromptCaching::Auto,
            reasoning,
        );
        let mut accounting = RunAccounting::new(1.0).expect("valid budget");

        conversation
            .request(
                &client,
                &[tool("alpha").expect("valid tool")],
                ProviderToolChoice::Required,
                &mut accounting,
            )
            .expect("mock completion succeeds");

        let captured = server.finish();
        assert_eq!(captured[0].body["tool_choice"], "required");
        assert_eq!(captured[0].body["reasoning"], expected);
        assert_ne!(captured[0].body["reasoning"]["exclude"], true);
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn truncated_json_response_retries_then_recovers() {
    let server = MockServer::start_truncated_then_complete(
        br#"{"choices":[{"message":{"role":"assistant""#.to_vec(),
        tool_call_response("alpha", 0.01),
    );
    let client = client(server.base_url.clone(), 1, Duration::from_millis(1), 0);
    let mut conversation = conversation();
    let mut accounting = RunAccounting::new(1.0).expect("valid budget");

    let turn = conversation
        .request(
            &client,
            &[tool("alpha").expect("valid tool")],
            ProviderToolChoice::Required,
            &mut accounting,
        )
        .expect("truncated response is retried");

    assert_eq!(
        turn.tool_calls().expect("tool call")[0].function.name,
        "alpha"
    );
    let captured = server.finish();
    assert_eq!(captured.len(), 2);
    assert_eq!(captured[0].body_bytes, captured[1].body_bytes);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn anthropic_breakpoints_cover_system_and_move_to_each_request_tail() {
    let server = MockServer::start(vec![
        tool_call_response("alpha", 0.01),
        tool_call_response("beta", 0.01),
    ]);
    let client = client(server.base_url.clone(), 0, Duration::from_millis(1), 0);
    let mut conversation = conversation_for_model("anthropic/claude-test", PromptCaching::Auto);
    let mut accounting = RunAccounting::new(1.0).expect("valid budget");

    let first = conversation
        .request(
            &client,
            &[tool("alpha").expect("valid tool")],
            ProviderToolChoice::Required,
            &mut accounting,
        )
        .expect("first request");
    let mut dispatcher = ExactDispatcher;
    conversation
        .dispatch_tool_calls(
            first.tool_calls().expect("first request tool call"),
            &mut dispatcher,
        )
        .expect("thread first tool result");
    conversation
        .request(
            &client,
            &[tool("beta").expect("valid tool")],
            ProviderToolChoice::Required,
            &mut accounting,
        )
        .expect("second request");

    let captured = server.finish();
    for request in &captured {
        let messages = request.body["messages"]
            .as_array()
            .expect("request messages");
        assert_eq!(cache_breakpoint_count(messages), 2);
        assert!(has_cache_breakpoint(&messages[0]));
        assert!(has_cache_breakpoint(
            messages.last().expect("final message")
        ));
    }

    let first_messages = captured[0].body["messages"]
        .as_array()
        .expect("first request messages");
    assert_eq!(first_messages.last().expect("first tail")["role"], "user");

    let second_messages = captured[1].body["messages"]
        .as_array()
        .expect("second request messages");
    assert_eq!(
        second_messages[1]["content"], "Private task.",
        "the prior request tail must return to plain-string form"
    );
    assert_eq!(second_messages.last().expect("second tail")["role"], "tool");
}

#[test]
#[requires(true)]
#[ensures(true)]
fn prompt_caching_off_suppresses_anthropic_breakpoints() {
    let server = MockServer::start(vec![tool_call_response("alpha", 0.01)]);
    let client = client(server.base_url.clone(), 0, Duration::from_millis(1), 0);
    let mut conversation = conversation_for_model("anthropic/claude-test", PromptCaching::Off);
    let mut accounting = RunAccounting::new(1.0).expect("valid budget");

    conversation
        .request(
            &client,
            &[tool("alpha").expect("valid tool")],
            ProviderToolChoice::Required,
            &mut accounting,
        )
        .expect("mock completion succeeds");

    let captured = server.finish();
    let messages = captured[0].body["messages"]
        .as_array()
        .expect("request messages");
    assert_eq!(cache_breakpoint_count(messages), 0);
    assert!(
        messages
            .iter()
            .all(|message| message["content"].is_string())
    );
}

#[requires(!model.trim().is_empty())]
#[ensures(true)]
fn assert_reasoning_details_round_trip(model: &str) {
    let reasoning = "private summary that must never enter canonical history";
    let reasoning_details = json!([
        {
            "type": "reasoning.text",
            "text": "first private block",
            "signature": "signature-one"
        },
        {
            "type": "reasoning.encrypted",
            "data": "encrypted-two"
        }
    ]);
    let mut first = tool_call_response("alpha", 0.01);
    first.body["choices"][0]["message"]["reasoning"] = json!(reasoning);
    first.body["choices"][0]["message"]["reasoning_details"] = reasoning_details.clone();
    let server = MockServer::start(vec![
        first,
        tool_call_response("beta", 0.01),
        tool_call_response("gamma", 0.01),
    ]);
    let client = client(server.base_url.clone(), 0, Duration::from_millis(1), 0);
    let mut conversation = conversation_for_model(model, PromptCaching::Auto);
    let mut accounting = RunAccounting::new(1.0).expect("valid budget");
    let mut dispatcher = ExactDispatcher;

    let first = conversation
        .request(
            &client,
            &[tool("alpha").expect("valid tool")],
            ProviderToolChoice::Required,
            &mut accounting,
        )
        .expect("first tool call");
    conversation
        .dispatch_tool_calls(first.tool_calls().expect("alpha call"), &mut dispatcher)
        .expect("alpha result");
    let second = conversation
        .request(
            &client,
            &[tool("beta").expect("valid tool")],
            ProviderToolChoice::Required,
            &mut accounting,
        )
        .expect("second tool call");
    conversation
        .dispatch_tool_calls(second.tool_calls().expect("beta call"), &mut dispatcher)
        .expect("beta result");
    conversation
        .request(
            &client,
            &[tool("gamma").expect("valid tool")],
            ProviderToolChoice::Required,
            &mut accounting,
        )
        .expect("third tool call");

    let observations = conversation.take_pending_observations();
    assert_eq!(observations.len(), 3);
    let trace = observations[0]
        .thinking
        .as_ref()
        .expect("first call thinking trace");
    assert_eq!(trace.reasoning.as_deref(), Some(reasoning));
    assert_eq!(
        trace.reasoning_details.as_ref(),
        reasoning_details.as_array()
    );
    assert!(observations[1].thinking.is_none());
    assert!(observations[2].thinking.is_none());

    let canonical_history =
        serde_json::to_string(conversation.messages()).expect("canonical history serializes");
    for private_value in [reasoning, "signature-one", "encrypted-two"] {
        assert!(
            !canonical_history.contains(private_value),
            "private reasoning leaked into canonical history: {private_value}"
        );
    }

    let captured = server.finish();
    let second_messages = captured[1].body["messages"]
        .as_array()
        .expect("second request messages");
    let alpha = second_messages
        .iter()
        .find(|message| message["tool_calls"][0]["id"] == "call-alpha")
        .expect("originating alpha assistant message");
    assert_eq!(alpha["reasoning_details"], reasoning_details);
    assert!(alpha.get("reasoning").is_none());
    assert!(
        !String::from_utf8_lossy(&captured[1].body_bytes).contains(reasoning),
        "unstructured private reasoning must never be replayed"
    );

    let third_messages = captured[2].body["messages"]
        .as_array()
        .expect("third request messages");
    let alpha = third_messages
        .iter()
        .find(|message| message["tool_calls"][0]["id"] == "call-alpha")
        .expect("alpha assistant remains in the same loop");
    assert_eq!(alpha["reasoning_details"], reasoning_details);
    let beta = third_messages
        .iter()
        .find(|message| message["tool_calls"][0]["id"] == "call-beta")
        .expect("beta assistant remains in the same loop");
    assert!(beta.get("reasoning_details").is_none());
}

#[test]
#[requires(true)]
#[ensures(true)]
fn observed_reasoning_details_round_trip_for_anthropic_and_gemini_tool_loops() {
    assert_reasoning_details_round_trip("anthropic/claude-test");
    assert_reasoning_details_round_trip("google/gemini-3-test");
}

#[test]
#[requires(true)]
#[ensures(true)]
fn reasoning_details_never_cross_a_provider_tool_loop_boundary() {
    let mut first = tool_call_response("alpha", 0.01);
    first.body["choices"][0]["message"]["reasoning_details"] = json!([{
        "type": "reasoning.text",
        "text": "loop-local",
        "signature": "loop-signature"
    }]);
    let server = MockServer::start(vec![first, tool_call_response("beta", 0.01)]);
    let client = client(server.base_url.clone(), 0, Duration::from_millis(1), 0);
    let mut conversation = conversation();
    let mut accounting = RunAccounting::new(1.0).expect("valid budget");
    let first = conversation
        .request(
            &client,
            &[tool("alpha").expect("valid tool")],
            ProviderToolChoice::Required,
            &mut accounting,
        )
        .expect("first tool call");
    conversation.begin_tool_loop();
    conversation
        .dispatch_tool_calls(
            first.tool_calls().expect("alpha call"),
            &mut ExactDispatcher,
        )
        .expect("alpha result");
    conversation
        .request(
            &client,
            &[tool("beta").expect("valid tool")],
            ProviderToolChoice::Required,
            &mut accounting,
        )
        .expect("second tool call");

    let captured = server.finish();
    assert!(
        captured[1].body["messages"]
            .as_array()
            .expect("second request messages")
            .iter()
            .all(|message| message.get("reasoning_details").is_none())
    );
}

#[requires(true)]
#[ensures(ret <= messages.len())]
fn cache_breakpoint_count(messages: &[Value]) -> usize {
    messages
        .iter()
        .filter(|message| has_cache_breakpoint(message))
        .count()
}

#[requires(true)]
#[ensures(true)]
fn has_cache_breakpoint(message: &Value) -> bool {
    message["content"].as_array().is_some_and(|parts| {
        parts.iter().any(|part| {
            part["cache_control"]["type"]
                .as_str()
                .is_some_and(|kind| kind == "ephemeral")
        })
    })
}

#[test]
#[requires(true)]
#[ensures(true)]
fn required_prose_is_correctively_reprompted_before_success() {
    let server = MockServer::start(vec![
        prose_response("I will answer directly.", 0.01),
        tool_call_response("alpha", 0.02),
    ]);
    let client = client(server.base_url.clone(), 0, Duration::from_millis(1), 1);
    let mut conversation = conversation();
    let mut accounting = RunAccounting::new(1.0).expect("valid budget");
    let turn = conversation
        .request(
            &client,
            &[tool("alpha").expect("valid tool")],
            ProviderToolChoice::Required,
            &mut accounting,
        )
        .expect("reprompt reaches tool call");
    assert!(turn.tool_calls().is_some());
    let usage = take_pending_usage(&mut conversation);
    assert_eq!(usage.len(), 2, "each provider call must remain distinct");
    assert_eq!(usage.iter().map(|record| record.cost).sum::<f64>(), 0.03);

    // These are the messages received by the mock server, so the assertion
    // fails if the fallback loop retries without actually sending correction.
    let captured = server.finish();
    assert_eq!(captured.len(), 2);
    let second_messages = captured[1].body["messages"]
        .as_array()
        .expect("second request messages");
    assert!(second_messages.iter().any(|message| {
        message["role"] == "user"
            && message["content"]
                .as_str()
                .is_some_and(|content| content.contains("must respond by calling"))
    }));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn reasoning_only_completion_is_correctively_reprompted_before_success() {
    let private_reasoning = "secret chain of thought that must not enter history";
    let server = MockServer::start(vec![
        reasoning_only_response(private_reasoning, 4),
        tool_call_response("alpha", 0.02),
    ]);
    let client = client(server.base_url.clone(), 0, Duration::from_millis(1), 1);
    let mut conversation = conversation();
    let mut accounting = RunAccounting::new(1.0).expect("valid budget");

    let turn = conversation
        .request(
            &client,
            &[tool("alpha").expect("valid tool")],
            ProviderToolChoice::Required,
            &mut accounting,
        )
        .expect("reasoning-only response is correctable");

    assert!(turn.tool_calls().is_some());
    let usage = take_pending_usage(&mut conversation);
    assert_eq!(usage.len(), 2, "the corrective call must remain accounted");
    assert!(usage[0].reasoning_present);
    assert_eq!(usage[0].reasoning_tokens, Some(4));
    assert_eq!(conversation.usage().reasoning_tokens, 4);
    assert_eq!(conversation.usage().reasoning_calls, 1);

    let captured = server.finish();
    assert_eq!(captured.len(), 2);
    let messages = captured[1].body["messages"]
        .as_array()
        .expect("corrective request messages");
    assert_eq!(
        messages.len(),
        3,
        "empty assistant history must not be stored"
    );
    assert_eq!(messages[2]["role"], "user");
    assert!(
        messages[2]["content"]
            .as_str()
            .is_some_and(|content| content.contains("no visible content or tool call"))
    );
    assert!(
        !captured[1]
            .body_bytes
            .windows(private_reasoning.len())
            .any(|window| window == private_reasoning.as_bytes()),
        "private reasoning must never be replayed in model history"
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn reasoning_only_reprompt_exhaustion_keeps_the_existing_typed_error() {
    let server = MockServer::start(vec![
        reasoning_only_response("first private reasoning", 3),
        reasoning_only_response("second private reasoning", 2),
    ]);
    let client = client(server.base_url.clone(), 0, Duration::from_millis(1), 1);
    let mut conversation = conversation();
    let mut accounting = RunAccounting::new(1.0).expect("valid budget");

    let error = conversation
        .request(
            &client,
            &[tool("alpha").expect("valid tool")],
            ProviderToolChoice::Required,
            &mut accounting,
        )
        .expect_err("bounded reasoning-only reprompts must exhaust");

    assert_eq!(
        error,
        OpenRouterError::RequiredToolCallExhausted { attempts: 2 }
    );
    assert_eq!(take_pending_usage(&mut conversation).len(), 2);
    let captured = server.finish();
    assert_eq!(captured.len(), 2);
    assert!(
        captured[1].body["messages"]
            .as_array()
            .expect("corrective messages")
            .iter()
            .any(|message| {
                message["role"] == "user"
                    && message["content"]
                        .as_str()
                        .is_some_and(|content| content.contains("Private reasoning"))
            })
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn invalid_arguments_history_is_valid_json_and_reprompt_answers_every_tool_call_id() {
    let server = MockServer::start(vec![
        MockResponse {
            status: 200,
            body: json!({
                "choices": [{
                    "message": {
                        "role": "assistant",
                        "content": null,
                        "tool_calls": [
                            {
                                "id": "call-malformed",
                                "type": "function",
                                "function": {
                                    "name": "alpha",
                                    "arguments": "{"
                                }
                            },
                            {
                                "id": "call-not-object",
                                "type": "function",
                                "function": {
                                    "name": "beta",
                                    "arguments": "[]"
                                }
                            }
                        ]
                    }
                }],
                "usage": {
                    "prompt_tokens": 3,
                    "completion_tokens": 2,
                    "total_tokens": 5,
                    "cost": 0.01
                }
            }),
        },
        tool_call_response("alpha", 0.01),
    ]);
    let client = client(server.base_url.clone(), 0, Duration::from_millis(1), 1);
    let mut conversation = conversation();
    let mut accounting = RunAccounting::new(1.0).expect("valid budget");
    let tools = [tool("alpha").expect("alpha"), tool("beta").expect("beta")];
    let turn = conversation
        .request(
            &client,
            &tools,
            ProviderToolChoice::Required,
            &mut accounting,
        )
        .expect("invalid arguments are corrected by tool results");
    assert!(turn.tool_calls().is_some());

    // Assert against the reprompt body received by the server. Retrying with
    // bare user prose or omitting either call id necessarily fails this test.
    let captured = server.finish();
    let messages = captured[1].body["messages"]
        .as_array()
        .expect("reprompt messages");
    let assistant_tool_calls = messages
        .iter()
        .find(|message| message["role"] == "assistant")
        .expect("assistant tool-call history")["tool_calls"]
        .as_array()
        .expect("assistant tool calls");
    let malformed_history = assistant_tool_calls
        .iter()
        .find(|call| call["id"] == "call-malformed")
        .expect("malformed call history")["function"]["arguments"]
        .as_str()
        .expect("malformed history arguments string");
    let malformed_history: Value =
        serde_json::from_str(malformed_history).expect("history arguments must be valid JSON");
    assert_eq!(malformed_history, json!({ "malformed_arguments": "{" }));
    let non_object_history = assistant_tool_calls
        .iter()
        .find(|call| call["id"] == "call-not-object")
        .expect("non-object call history")["function"]["arguments"]
        .as_str()
        .expect("non-object history arguments string");
    assert_eq!(
        serde_json::from_str::<Value>(non_object_history)
            .expect("non-object history arguments remain valid JSON"),
        json!([])
    );
    let tool_messages = messages
        .iter()
        .filter(|message| message["role"] == "tool")
        .collect::<Vec<_>>();
    assert_eq!(tool_messages.len(), 2);
    let malformed = tool_messages
        .iter()
        .find(|message| message["tool_call_id"] == "call-malformed")
        .expect("malformed call result");
    assert!(
        malformed["content"]
            .as_str()
            .is_some_and(|content| content.contains("invalid call to tool `alpha`"))
    );
    let not_object = tool_messages
        .iter()
        .find(|message| message["tool_call_id"] == "call-not-object")
        .expect("non-object call result");
    assert!(
        not_object["content"]
            .as_str()
            .is_some_and(|content| content.contains("must encode a JSON object"))
    );
    assert!(!messages.iter().any(|message| {
        message["role"] == "user"
            && message["content"]
                .as_str()
                .is_some_and(|content| content.contains("must respond by calling"))
    }));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn dispatcher_failure_answers_failed_and_remaining_call_ids() {
    let server = MockServer::start(vec![
        tool_calls_response(&["alpha", "beta", "gamma"], 0.01),
        tool_call_response("alpha", 0.01),
    ]);
    let client = client(server.base_url.clone(), 0, Duration::from_millis(1), 0);
    let mut conversation = conversation();
    let mut accounting = RunAccounting::new(1.0).expect("valid budget");
    let tools = [
        tool("alpha").expect("alpha"),
        tool("beta").expect("beta"),
        tool("gamma").expect("gamma"),
    ];
    let first = conversation
        .request(
            &client,
            &tools,
            ProviderToolChoice::Required,
            &mut accounting,
        )
        .expect("first tool-call turn");
    let mut dispatcher = FirstCallFails::default();
    let error = conversation
        .dispatch_tool_calls(first.tool_calls().expect("tool calls"), &mut dispatcher)
        .expect_err("first dispatch must fail");
    assert_eq!(dispatcher.attempts, 1, "remaining tools are not executed");
    assert_eq!(error.tool_name, "alpha");

    conversation
        .request(
            &client,
            &[tool("alpha").expect("alpha")],
            ProviderToolChoice::Required,
            &mut accounting,
        )
        .expect("answered call ids leave the conversation reusable");
    let captured = server.finish();
    let messages = captured[1].body["messages"]
        .as_array()
        .expect("second request messages");
    let tool_messages = messages
        .iter()
        .filter(|message| message["role"] == "tool")
        .collect::<Vec<_>>();
    assert_eq!(tool_messages.len(), 3);
    assert_eq!(tool_messages[0]["tool_call_id"], "call-alpha");
    assert_eq!(tool_messages[1]["tool_call_id"], "call-beta");
    assert_eq!(tool_messages[2]["tool_call_id"], "call-gamma");
    assert!(
        tool_messages[0]["content"]
            .as_str()
            .is_some_and(|content| content.contains("deliberate test failure"))
    );
    assert!(tool_messages[1..].iter().all(|message| {
        message["content"]
            .as_str()
            .is_some_and(|content| content.contains("earlier tool call failed"))
    }));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn required_tool_reprompt_exhaustion_is_typed() {
    let server = MockServer::start(vec![
        prose_response("first prose", 0.01),
        prose_response("second prose", 0.01),
    ]);
    let client = client(server.base_url.clone(), 0, Duration::from_millis(1), 1);
    let mut conversation = conversation();
    let mut accounting = RunAccounting::new(1.0).expect("valid budget");
    let error = conversation
        .request(
            &client,
            &[tool("alpha").expect("valid tool")],
            ProviderToolChoice::Required,
            &mut accounting,
        )
        .expect_err("bounded reprompt must exhaust");
    assert_eq!(
        error,
        OpenRouterError::RequiredToolCallExhausted { attempts: 2 }
    );
    let captured = server.finish();
    assert_eq!(captured.len(), 2);
    assert_eq!(captured[1].body["tool_choice"], "required");
}

#[test]
#[requires(true)]
#[ensures(true)]
fn transient_429_backs_off_then_retries() {
    let server = MockServer::start(vec![
        MockResponse {
            status: 429,
            body: json!({ "error": "slow down" }),
        },
        tool_call_response("alpha", 0.01),
    ]);
    let backoff = Duration::from_millis(30);
    let client = client(server.base_url.clone(), 1, backoff, 0);
    let mut conversation = conversation();
    let mut accounting = RunAccounting::new(1.0).expect("valid budget");
    let turn = conversation
        .request(
            &client,
            &[tool("alpha").expect("valid tool")],
            ProviderToolChoice::Required,
            &mut accounting,
        )
        .expect("retry succeeds");
    assert!(turn.tool_calls().is_some());
    let captured = server.finish();
    assert_eq!(captured.len(), 2);
    assert!(
        captured[1]
            .received_at
            .duration_since(captured[0].received_at)
            >= backoff
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn transient_500_retries_then_succeeds() {
    let server = MockServer::start(vec![
        MockResponse {
            status: 500,
            body: json!({ "error": "upstream unavailable" }),
        },
        tool_call_response("alpha", 0.01),
    ]);
    let client = client(server.base_url.clone(), 1, Duration::from_millis(1), 0);
    let mut conversation = conversation();
    let mut accounting = RunAccounting::new(1.0).expect("valid budget");
    let turn = conversation
        .request(
            &client,
            &[tool("alpha").expect("valid tool")],
            ProviderToolChoice::Required,
            &mut accounting,
        )
        .expect("500 retry succeeds");
    assert!(turn.tool_calls().is_some());
    assert_eq!(server.finish().len(), 2);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn response_body_timeout_retries_then_succeeds() {
    let body_delay = Duration::from_millis(100);
    let backoff = Duration::from_millis(75);
    let server = MockServer::start_scheduled(vec![
        (tool_call_response("alpha", 0.01), body_delay),
        (tool_call_response("alpha", 0.01), Duration::ZERO),
    ]);
    let client = client_with_timeout(
        server.base_url.clone(),
        1,
        backoff,
        0,
        Duration::from_millis(50),
    );
    let mut conversation = conversation();
    let mut accounting = RunAccounting::new(1.0).expect("valid budget");

    let turn = conversation
        .request(
            &client,
            &[tool("alpha").expect("valid tool")],
            ProviderToolChoice::Required,
            &mut accounting,
        )
        .expect("body timeout retry succeeds");

    assert!(turn.tool_calls().is_some());
    let captured = server.finish();
    assert_eq!(
        captured.len(),
        2,
        "timeout must issue a second HTTP request"
    );
    assert!(
        captured[1]
            .received_at
            .duration_since(captured[0].received_at)
            >= backoff,
        "retry must pass through the configured backoff"
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn response_body_timeout_exhaustion_is_typed() {
    let server = MockServer::start_scheduled(vec![
        (
            tool_call_response("alpha", 0.01),
            Duration::from_millis(100),
        ),
        (
            tool_call_response("alpha", 0.01),
            Duration::from_millis(100),
        ),
    ]);
    let client = client_with_timeout(
        server.base_url.clone(),
        1,
        Duration::from_millis(75),
        0,
        Duration::from_millis(50),
    );
    let mut conversation = conversation();
    let mut accounting = RunAccounting::new(1.0).expect("valid budget");

    let error = conversation
        .request(
            &client,
            &[tool("alpha").expect("valid tool")],
            ProviderToolChoice::Required,
            &mut accounting,
        )
        .expect_err("bounded timeout retries must exhaust");

    assert!(matches!(
        error,
        OpenRouterError::TransportRetriesExhausted { attempts: 2, ref message }
            if message.contains("timeout")
    ));
    assert_eq!(
        server.finish().len(),
        2,
        "timeout exhaustion must consume exactly the initial call and one retry"
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn choices_less_transient_provider_error_retries_then_succeeds() {
    let server = MockServer::start(vec![
        provider_error_response(503, "mock provider overloaded"),
        tool_call_response("alpha", 0.01),
    ]);
    let client = client(server.base_url.clone(), 1, Duration::from_millis(1), 0);
    let mut conversation = conversation();
    let mut accounting = RunAccounting::new(1.0).expect("valid budget");

    let turn = conversation
        .request(
            &client,
            &[tool("alpha").expect("valid tool")],
            ProviderToolChoice::Required,
            &mut accounting,
        )
        .expect("transient provider envelope retry succeeds");

    assert!(turn.tool_calls().is_some());
    assert_eq!(
        server.finish().len(),
        2,
        "provider envelope must issue exactly one bounded retry"
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn choices_less_permanent_provider_error_preserves_message_without_retry() {
    let server = MockServer::start(vec![provider_error_response(
        400,
        "model rejected the request payload",
    )]);
    let client = client(server.base_url.clone(), 3, Duration::from_millis(1), 0);
    let mut conversation = conversation();
    let mut accounting = RunAccounting::new(1.0).expect("valid budget");

    let error = conversation
        .request(
            &client,
            &[tool("alpha").expect("valid tool")],
            ProviderToolChoice::Required,
            &mut accounting,
        )
        .expect_err("permanent provider envelope must fail fast");

    assert_eq!(
        error,
        OpenRouterError::Provider {
            code: 400,
            message: "model rejected the request payload".to_owned(),
        }
    );
    assert_eq!(server.finish().len(), 1, "permanent error must not retry");
}

#[test]
#[requires(true)]
#[ensures(true)]
fn budget_cap_returns_explicit_abort_and_prevents_more_http_calls() {
    let server = MockServer::start(vec![tool_call_response("alpha", 0.75)]);
    let client = client(server.base_url.clone(), 0, Duration::from_millis(1), 0);
    let mut conversation = conversation();
    let tools = [tool("alpha").expect("valid tool")];
    let mut accounting = RunAccounting::new(0.5).expect("valid budget");
    let first = conversation
        .request(
            &client,
            &tools,
            ProviderToolChoice::Required,
            &mut accounting,
        )
        .expect("budget is a graceful outcome");
    let record = first.abort_record().expect("expected budget abort").clone();
    assert_eq!(record.kind, AbortKind::CostBudgetExceeded);
    assert_eq!(record.max_cost_usd, 0.5);
    assert_eq!(record.actual_cost_usd, 0.75);
    let second = conversation
        .request(
            &client,
            &tools,
            ProviderToolChoice::Required,
            &mut accounting,
        )
        .expect("existing abort is surfaced without HTTP");
    assert_eq!(second.abort_record(), Some(&record));
    assert_eq!(server.finish().len(), 1);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn tools_are_dynamic_per_request_on_the_mock_server() {
    let server = MockServer::start(vec![
        tool_call_response("alpha", 0.01),
        tool_call_response("beta", 0.01),
    ]);
    let client = client(server.base_url.clone(), 0, Duration::from_millis(1), 0);
    let mut conversation = conversation();
    let mut accounting = RunAccounting::new(1.0).expect("valid budget");
    let first = conversation
        .request(
            &client,
            &[tool("alpha").expect("valid tool")],
            ProviderToolChoice::Required,
            &mut accounting,
        )
        .expect("first request");
    let mut dispatcher = ExactDispatcher;
    conversation
        .dispatch_tool_calls(
            first.tool_calls().expect("first request tool call"),
            &mut dispatcher,
        )
        .expect("thread first tool result");
    conversation
        .request(
            &client,
            &[tool("beta").expect("valid tool")],
            ProviderToolChoice::Required,
            &mut accounting,
        )
        .expect("second request");

    // Assert against the two actual request bodies received by the server. A
    // cached or unified tool list necessarily fails these exact comparisons.
    let captured = server.finish();
    let first_tools = captured[0].body["tools"].clone();
    let second_tools = captured[1].body["tools"].clone();
    assert_ne!(first_tools, second_tools);
    assert_eq!(first_tools[0]["function"]["name"], "alpha");
    assert_eq!(second_tools[0]["function"]["name"], "beta");
    assert_eq!(first_tools.as_array().expect("first tools").len(), 1);
    assert_eq!(second_tools.as_array().expect("second tools").len(), 1);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn permanent_http_errors_fail_without_retry() {
    let server = MockServer::start(vec![MockResponse {
        status: 401,
        body: json!({ "error": "bad key" }),
    }]);
    let client = client(server.base_url.clone(), 3, Duration::from_millis(1), 0);
    let mut conversation = conversation();
    let mut accounting = RunAccounting::new(1.0).expect("valid budget");
    let error = conversation
        .request(
            &client,
            &[tool("alpha").expect("valid tool")],
            ProviderToolChoice::Required,
            &mut accounting,
        )
        .expect_err("401 must fail fast");
    assert!(matches!(
        error,
        OpenRouterError::HttpStatus { status: 401, .. }
    ));
    assert_eq!(server.finish().len(), 1);
}
