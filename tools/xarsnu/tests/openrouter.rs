use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

#[allow(unused_imports)]
use bityzba::{contract_trait, ensures, invariant, requires};
use serde_json::{Value, json};
use xarsnu::{
    AbortKind, OpenRouterClient, OpenRouterClientConfig, OpenRouterError, ParticipantConversation,
    RetryPolicy, RunAccounting, ToolCall, ToolChoice, ToolDefinition, ToolDispatchError,
    ToolDispatcher,
};

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
        let expected_requests = responses.len();
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind local mock server");
        let address = listener.local_addr().expect("mock server address");
        let worker = thread::spawn(move || {
            let mut captured = Vec::with_capacity(responses.len());
            for response in responses {
                let (mut stream, _) = listener.accept().expect("accept mock request");
                let body = read_json_request(&mut stream);
                captured.push(CapturedRequest {
                    body,
                    received_at: Instant::now(),
                });
                write_json_response(&mut stream, response);
            }
            captured
        });
        Self {
            base_url: format!("http://{address}"),
            expected_requests,
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
#[ensures(ret.is_object())]
fn read_json_request(stream: &mut TcpStream) -> Value {
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
fn write_json_response(stream: &mut TcpStream, response: MockResponse) {
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
        body
    )
    .expect("write mock response");
    stream.flush().expect("flush mock response");
}

#[requires(!name.trim().is_empty())]
#[requires(cost.is_finite() && cost >= 0.0)]
#[ensures(ret.status == 200)]
fn tool_call_response(name: &str, cost: f64) -> MockResponse {
    MockResponse {
        status: 200,
        body: json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": null,
                    "tool_calls": [{
                        "id": format!("call-{name}"),
                        "type": "function",
                        "function": {
                            "name": name,
                            "arguments": "{\"value\":1}"
                        }
                    }]
                }
            }],
            "usage": {
                "prompt_tokens": 11,
                "completion_tokens": 7,
                "total_tokens": 18,
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

#[requires(!base_url.trim().is_empty())]
#[ensures(true)]
fn client(
    base_url: String,
    max_retries: usize,
    backoff: Duration,
    max_reprompts: usize,
) -> OpenRouterClient {
    let retry_policy = RetryPolicy::new(max_retries, backoff).expect("valid retry policy");
    let config = OpenRouterClientConfig::new(
        base_url,
        None,
        retry_policy,
        max_reprompts,
        Duration::from_secs(2),
    )
    .expect("valid mock client config");
    OpenRouterClient::new(config)
}

#[requires(true)]
#[ensures(ret.messages().len() == 2)]
fn conversation() -> ParticipantConversation {
    ParticipantConversation::from_parts(
        "tester".to_owned(),
        "mock/model".to_owned(),
        0.3,
        "Use tools.".to_owned(),
        "Private task.".to_owned(),
    )
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
            ToolChoice::Required,
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
    let captured = server.finish();
    assert_eq!(captured[0].body["tool_choice"], "required");
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
            ToolChoice::Required,
            &mut accounting,
        )
        .expect("reprompt reaches tool call");
    assert!(turn.tool_calls().is_some());

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
            ToolChoice::Required,
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
            ToolChoice::Required,
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
            ToolChoice::Required,
            &mut accounting,
        )
        .expect("500 retry succeeds");
    assert!(turn.tool_calls().is_some());
    assert_eq!(server.finish().len(), 2);
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
        .request(&client, &tools, ToolChoice::Required, &mut accounting)
        .expect("budget is a graceful outcome");
    let record = first.abort_record().expect("expected budget abort").clone();
    assert_eq!(record.kind, AbortKind::CostBudgetExceeded);
    assert_eq!(record.max_cost_usd, 0.5);
    assert_eq!(record.actual_cost_usd, 0.75);
    let second = conversation
        .request(&client, &tools, ToolChoice::Required, &mut accounting)
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
            ToolChoice::Required,
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
            ToolChoice::Required,
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
            ToolChoice::Required,
            &mut accounting,
        )
        .expect_err("401 must fail fast");
    assert!(matches!(
        error,
        OpenRouterError::HttpStatus { status: 401, .. }
    ));
    assert_eq!(server.finish().len(), 1);
}
