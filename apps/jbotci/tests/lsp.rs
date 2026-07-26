use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, ChildStderr, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

#[allow(unused_imports)]
use bityzba::{ensures, invariant, requires};
use serde_json::{Value, json};

const DOCUMENT_URI: &str = "file:///encoding.jbo";
const INVALID_TEXT: &str = "zoi gy 𝙰«a gy ku";
const UTF8_ERROR_START: u64 = 18;
const UTF8_ERROR_END: u64 = 20;
const UTF16_ERROR_START: u64 = 15;
const UTF16_ERROR_END: u64 = 17;

#[invariant(true)]
struct LspClient {
    child: Option<Child>,
    stdin: Option<ChildStdin>,
    stdout: BufReader<ChildStdout>,
    stderr: ChildStderr,
    next_id: u64,
}

impl LspClient {
    #[requires(true)]
    #[ensures(true)]
    fn spawn() -> Self {
        Self::spawn_with_stdio_flag(false)
    }

    #[requires(true)]
    #[ensures(true)]
    fn spawn_with_stdio_flag(stdio: bool) -> Self {
        let mut command = Command::new(env!("CARGO_BIN_EXE_jbotci"));
        command.arg("lsp");
        if stdio {
            command.arg("--stdio");
        }
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn jbotci lsp");
        let stdin = child.stdin.take().expect("child stdin");
        let stdout = child.stdout.take().expect("child stdout");
        let stderr = child.stderr.take().expect("child stderr");
        Self {
            child: Some(child),
            stdin: Some(stdin),
            stdout: BufReader::new(stdout),
            stderr,
            next_id: 1,
        }
    }

    #[requires(!method.is_empty())]
    #[ensures(ret.is_null() || ret.is_object() || ret.is_array())]
    fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.start_request(method, params);
        self.response(id)
    }

    #[requires(!method.is_empty())]
    #[ensures(ret > 0 && ret < self.next_id)]
    fn start_request(&mut self, method: &str, params: Value) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.send(json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        }));
        id
    }

    #[requires(id > 0 && id < self.next_id)]
    #[ensures(ret.is_null() || ret.is_object() || ret.is_array())]
    fn response(&mut self, id: u64) -> Value {
        loop {
            let message = self.read();
            if self.respond_to_server_request(&message) {
                continue;
            }
            if message.get("id") == Some(&json!(id)) {
                assert!(
                    message.get("error").is_none(),
                    "LSP error response: {message}"
                );
                return message.get("result").cloned().expect("response result");
            }
        }
    }

    #[requires(!method.is_empty())]
    #[ensures(true)]
    fn notify(&mut self, method: &str, params: Value) {
        self.send(json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        }));
    }

    #[requires(!method.is_empty())]
    #[ensures(ret.is_object())]
    fn next_notification(&mut self, method: &str) -> Value {
        loop {
            let message = self.read();
            if self.respond_to_server_request(&message) {
                continue;
            }
            if message.get("method").and_then(Value::as_str) == Some(method) {
                return message;
            }
        }
    }

    #[requires(message.is_object())]
    #[ensures(true)]
    fn respond_to_server_request(&mut self, message: &Value) -> bool {
        let Some(id) = message.get("id") else {
            return false;
        };
        if message.get("method").and_then(Value::as_str).is_none() {
            return false;
        }
        self.send(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": null,
        }));
        true
    }

    #[requires(message.is_object())]
    #[ensures(true)]
    fn send(&mut self, message: Value) {
        let body = serde_json::to_vec(&message).expect("serialize LSP message");
        let stdin = self.stdin.as_mut().expect("LSP stdin remains open");
        write!(stdin, "Content-Length: {}\r\n\r\n", body.len()).expect("write LSP header");
        stdin.write_all(&body).expect("write LSP body");
        stdin.flush().expect("flush LSP request");
    }

    #[requires(true)]
    #[ensures(ret.is_object())]
    fn read(&mut self) -> Value {
        let mut content_length = None;
        loop {
            let mut header = String::new();
            let read = self.stdout.read_line(&mut header).expect("read LSP header");
            assert_ne!(read, 0, "LSP server closed stdout before a response");
            if header == "\r\n" || header == "\n" {
                break;
            }
            if let Some(value) = header
                .strip_prefix("Content-Length:")
                .or_else(|| header.strip_prefix("content-length:"))
            {
                content_length = Some(
                    value
                        .trim()
                        .parse::<usize>()
                        .expect("numeric LSP content length"),
                );
            }
        }
        let mut body = vec![0; content_length.expect("LSP Content-Length header")];
        self.stdout
            .read_exact(&mut body)
            .expect("read complete LSP body");
        serde_json::from_slice(&body).expect("valid JSON-RPC body")
    }

    #[requires(true)]
    #[ensures(true)]
    fn shutdown(mut self) {
        assert_eq!(self.request("shutdown", Value::Null), Value::Null);
        self.exit_after_shutdown();
    }

    #[requires(self.child.is_some())]
    #[ensures(true)]
    fn exit_after_shutdown(mut self) {
        self.notify("exit", Value::Null);
        drop(self.stdin.take());
        let status = self
            .child
            .take()
            .expect("LSP child remains owned until exit")
            .wait()
            .expect("wait for LSP server");
        let mut stderr = String::new();
        self.stderr
            .read_to_string(&mut stderr)
            .expect("read LSP stderr");
        assert!(status.success(), "LSP exit {status}: {stderr}");
        assert!(stderr.is_empty(), "unexpected LSP stderr: {stderr}");
    }
}

#[requires(matches!(position_encoding, "utf-8" | "utf-16"))]
#[ensures(true)]
fn initialize(client: &mut LspClient, position_encoding: &str, pull_diagnostics: bool) -> Value {
    initialize_with_options(client, position_encoding, pull_diagnostics, None)
}

#[requires(matches!(position_encoding, "utf-8" | "utf-16"))]
#[ensures(true)]
fn initialize_with_options(
    client: &mut LspClient,
    position_encoding: &str,
    pull_diagnostics: bool,
    initialization_options: Option<Value>,
) -> Value {
    let text_document = if pull_diagnostics {
        json!({
            "diagnostic": {
                "dynamicRegistration": false,
                "relatedDocumentSupport": true
            },
            "publishDiagnostics": {
                "relatedInformation": true
            }
        })
    } else {
        json!({
            "publishDiagnostics": {
                "relatedInformation": true
            }
        })
    };
    let mut params = json!({
        "processId": null,
        "capabilities": {
            "general": {
                "positionEncodings": [position_encoding]
            },
            "workspace": {
                "diagnostic": {
                    "refreshSupport": pull_diagnostics
                }
            },
            "textDocument": text_document
        }
    });
    if let Some(initialization_options) = initialization_options {
        params["initializationOptions"] = initialization_options;
    }
    let result = client.request("initialize", params);
    assert_eq!(
        result["capabilities"]["positionEncoding"],
        position_encoding
    );
    assert_eq!(
        result["capabilities"]["textDocumentSync"]["openClose"],
        true
    );
    assert_eq!(result["capabilities"]["textDocumentSync"]["change"], 2);
    assert!(result["capabilities"]["diagnosticProvider"].is_object());
    assert_eq!(result["capabilities"]["hoverProvider"], true);
    assert_eq!(
        result["capabilities"]["completionProvider"]["resolveProvider"],
        true
    );
    assert!(
        result["capabilities"]["completionProvider"]
            .get("triggerCharacters")
            .is_none(),
        "Lojban completion has no punctuation trigger characters",
    );
    assert_eq!(
        result["capabilities"]["semanticTokensProvider"]["legend"]["tokenTypes"],
        json!([
            "gismu",
            "lujvo",
            "fuhivla",
            "cmevla",
            "sumtiWord",
            "selbriWord",
            "connective",
            "terminator",
            "quotationMarker",
            "number",
            "letteral",
            "attitudinal",
            "tenseModal",
            "cmavo",
            "string"
        ])
    );
    assert_eq!(
        result["capabilities"]["semanticTokensProvider"]["legend"]["tokenModifiers"],
        json!([])
    );
    assert_eq!(
        result["capabilities"]["semanticTokensProvider"]["full"],
        true
    );
    assert!(
        result["capabilities"]["semanticTokensProvider"]
            .get("range")
            .is_none()
    );
    assert_eq!(
        result["capabilities"]["inlayHintProvider"]["resolveProvider"],
        false,
    );
    assert_eq!(result["capabilities"]["selectionRangeProvider"], true);
    assert_eq!(result["capabilities"]["foldingRangeProvider"], true);
    client.notify("initialized", json!({}));
    result
}

#[requires(version >= 0)]
#[ensures(true)]
fn open_document(client: &mut LspClient, version: i32) {
    open_document_text(client, DOCUMENT_URI, version, INVALID_TEXT);
}

#[requires(version >= 0)]
#[requires(!uri.is_empty())]
#[ensures(true)]
fn open_document_text(client: &mut LspClient, uri: &str, version: i32, text: &str) {
    client.notify(
        "textDocument/didOpen",
        json!({
            "textDocument": {
                "uri": uri,
                "languageId": "lojban",
                "version": version,
                "text": text
            }
        }),
    );
}

#[requires(true)]
#[ensures(ret.is_object())]
fn pull_diagnostics(client: &mut LspClient, previous_result_id: Option<&str>) -> Value {
    client.request(
        "textDocument/diagnostic",
        json!({
            "textDocument": { "uri": DOCUMENT_URI },
            "identifier": "jbotci",
            "previousResultId": previous_result_id
        }),
    )
}

#[requires(!uri.is_empty())]
#[ensures(ret.is_null() || ret.is_object())]
fn hover(client: &mut LspClient, uri: &str, character: u64) -> Value {
    client.request(
        "textDocument/hover",
        json!({
            "textDocument": { "uri": uri },
            "position": { "line": 0, "character": character }
        }),
    )
}

#[requires(!uri.is_empty() && positions.is_array())]
#[ensures(ret.is_array())]
fn selection_ranges(client: &mut LspClient, uri: &str, positions: Value) -> Value {
    client.request(
        "textDocument/selectionRange",
        json!({
            "textDocument": { "uri": uri },
            "positions": positions,
        }),
    )
}

#[requires(!uri.is_empty())]
#[ensures(ret.is_array())]
fn folding_ranges(client: &mut LspClient, uri: &str) -> Value {
    client.request(
        "textDocument/foldingRange",
        json!({ "textDocument": { "uri": uri } }),
    )
}

#[requires(!uri.is_empty())]
#[ensures(ret.is_array())]
fn completion(client: &mut LspClient, uri: &str, character: u64) -> Value {
    client.request(
        "textDocument/completion",
        json!({
            "textDocument": { "uri": uri },
            "position": { "line": 0, "character": character }
        }),
    )
}

#[requires(item.is_object())]
#[ensures(ret.is_object())]
fn resolve_completion(client: &mut LspClient, item: Value) -> Value {
    client.request("completionItem/resolve", item)
}

#[requires(!uri.is_empty())]
#[ensures(ret.is_null() || ret.is_object())]
fn semantic_tokens(client: &mut LspClient, uri: &str) -> Value {
    client.request(
        "textDocument/semanticTokens/full",
        json!({ "textDocument": { "uri": uri } }),
    )
}

#[requires(!uri.is_empty())]
#[ensures(ret.is_null() || ret.is_array())]
fn inlay_hints(client: &mut LspClient, uri: &str, start: Value, end: Value) -> Value {
    client.request(
        "textDocument/inlayHint",
        json!({
            "textDocument": { "uri": uri },
            "range": { "start": start, "end": end }
        }),
    )
}

#[requires(hints.is_array())]
#[ensures(ret.iter().all(|(_, label)| !label.is_empty()))]
fn inlay_positions_and_labels(hints: &Value) -> Vec<(u64, String)> {
    hints
        .as_array()
        .expect("inlay array required by precondition")
        .iter()
        .map(|hint| {
            (
                hint["position"]["character"]
                    .as_u64()
                    .expect("inlay column"),
                hint["label"].as_str().expect("inlay label").to_owned(),
            )
        })
        .collect()
}

#[requires(true)]
#[ensures(true)]
fn decode_semantic_tokens(result: &Value, legend: &[Value]) -> Vec<Value> {
    let data = result["data"]
        .as_array()
        .expect("semantic token data array");
    assert_eq!(data.len() % 5, 0, "semantic token data uses 5-tuples");
    let mut line = 0_u64;
    let mut start = 0_u64;
    data.chunks_exact(5)
        .map(|token| {
            let delta_line = token[0].as_u64().expect("deltaLine");
            let delta_start = token[1].as_u64().expect("deltaStart");
            if delta_line == 0 {
                start += delta_start;
            } else {
                line += delta_line;
                start = delta_start;
            }
            let token_type = token[3].as_u64().expect("tokenType") as usize;
            json!({
                "line": line,
                "start": start,
                "length": token[2],
                "kind": legend.get(token_type).expect("tokenType is in the legend"),
                "modifiers": token[4],
            })
        })
        .collect()
}

#[requires(matches!(position_encoding, "utf-8" | "utf-16"))]
#[requires(le_start < nu_start)]
#[ensures(true)]
fn assert_hover_ranges(position_encoding: &str, le_start: u64, nu_start: u64) {
    const URI: &str = "file:///hover-encoding.jbo";
    const TEXT: &str = "zoi gy 𝙰«a gy lenu";

    let mut client = LspClient::spawn();
    initialize(&mut client, position_encoding, true);
    open_document_text(&mut client, URI, 1, TEXT);

    let le = hover(&mut client, URI, le_start);
    assert_eq!(le["contents"]["kind"], "markdown");
    assert_eq!(
        le["contents"]["value"],
        concat!(
            "### `lenu` — *cmavo sequence* · **LE\\***\n\n",
            "specific event descriptor: contraction of {le nu} and identical in meaning.\n\n",
            "**Glosses:** `the specific event of`",
        ),
    );
    assert_eq!(
        le["range"],
        json!({
            "start": { "line": 0, "character": le_start },
            "end": { "line": 0, "character": nu_start + 2 }
        })
    );

    let nu = hover(&mut client, URI, nu_start);
    assert_eq!(nu["contents"]["kind"], "markdown");
    assert_eq!(nu["contents"]["value"], le["contents"]["value"]);
    assert_eq!(
        nu["range"],
        json!({
            "start": { "line": 0, "character": le_start },
            "end": { "line": 0, "character": nu_start + 2 }
        })
    );
    client.shutdown();
}

#[requires(matches!(position_encoding, "utf-8" | "utf-16"))]
#[ensures(true)]
fn assert_selection_range_encoding(position_encoding: &str) {
    const URI: &str = "file:///selection-encoding.jbo";
    const TEXT: &str = "mi cusku zoi gy 𝙰«a gy .i do tavla";

    let tavla_byte_start = TEXT.find("tavla").expect("fixture contains selection word");
    let prefix = &TEXT[..tavla_byte_start];
    let tavla_start = match position_encoding {
        "utf-8" => prefix.len(),
        "utf-16" => prefix.encode_utf16().count(),
        _ => unreachable!("precondition limits position encodings"),
    } as u64;
    let document_end = match position_encoding {
        "utf-8" => TEXT.len(),
        "utf-16" => TEXT.encode_utf16().count(),
        _ => unreachable!("precondition limits position encodings"),
    } as u64;

    let mut client = LspClient::spawn();
    initialize(&mut client, position_encoding, true);
    open_document_text(&mut client, URI, 1, TEXT);
    let result = selection_ranges(
        &mut client,
        URI,
        json!([
            { "line": 0, "character": tavla_start + 1 },
            { "line": 0, "character": 1 },
        ]),
    );
    let ranges = result.as_array().expect("selection range array");

    assert_eq!(
        ranges.len(),
        2,
        "request order and cardinality are preserved"
    );
    assert_eq!(
        ranges[0]["range"],
        json!({
            "start": { "line": 0, "character": tavla_start },
            "end": { "line": 0, "character": tavla_start + 5 },
        }),
    );
    assert_eq!(
        ranges[1]["range"],
        json!({
            "start": { "line": 0, "character": 0 },
            "end": { "line": 0, "character": 2 },
        }),
    );

    for range in ranges {
        let mut outermost = range;
        while let Some(parent) = outermost.get("parent") {
            outermost = parent;
        }
        assert_eq!(
            outermost["range"],
            json!({
                "start": { "line": 0, "character": 0 },
                "end": { "line": 0, "character": document_end },
            }),
            "every selection chain must end at the completed document snapshot",
        );
    }
    client.shutdown();
}

#[requires(matches!(position_encoding, "utf-8" | "utf-16"))]
#[ensures(true)]
fn assert_completion_encoding(position_encoding: &str) {
    const URI: &str = "file:///completion-encoding.jbo";
    const TEXT: &str = "mí klama le bar";
    const SEED: &str = "bar";

    let seed_byte_start = TEXT.rfind(SEED).expect("fixture contains completion seed");
    let prefix = &TEXT[..seed_byte_start];
    let seed_start = match position_encoding {
        "utf-8" => prefix.len(),
        "utf-16" => prefix.encode_utf16().count(),
        _ => unreachable!("precondition limits position encodings"),
    } as u64;
    let cursor = seed_start + SEED.len() as u64;

    let mut client = LspClient::spawn();
    initialize(&mut client, position_encoding, true);
    open_document_text(&mut client, URI, 1, TEXT);

    let result = completion(&mut client, URI, cursor);
    let item = result
        .as_array()
        .expect("completion array")
        .iter()
        .find(|item| item["label"] == "barda")
        .expect("bar extends to dictionary brivla barda")
        .clone();
    assert_eq!(item["kind"], 3, "brivla map to Function");
    assert_eq!(item["labelDetails"]["description"], "big");
    assert_eq!(item["detail"], "starts tanru unit");
    assert_eq!(item["sortText"], "1·barda");
    assert!(item.get("preselect").is_none());
    assert!(
        item.get("documentation").is_none(),
        "completion lists defer documentation",
    );
    assert_eq!(item["data"]["jbotciWord"], "barda");
    assert_eq!(
        item["textEdit"],
        json!({
            "range": {
                "start": { "line": 0, "character": seed_start },
                "end": { "line": 0, "character": cursor }
            },
            "newText": "barda"
        })
    );

    let resolved = resolve_completion(&mut client, item);
    assert_eq!(resolved["documentation"]["kind"], "markdown");
    assert!(
        resolved["documentation"]["value"]
            .as_str()
            .is_some_and(|markdown| markdown.contains("### `barda`")),
        "completion resolution uses the shared dictionary Markdown renderer",
    );
    client.shutdown();
}

#[requires(matches!(position_encoding, "utf-8" | "utf-16"))]
#[requires(payload_length > 0 && closing_start < le_start)]
#[ensures(true)]
fn assert_semantic_token_encoding(
    position_encoding: &str,
    payload_length: u64,
    closing_start: u64,
    le_start: u64,
) {
    const URI: &str = "file:///semantic-encoding.jbo";
    const TEXT: &str = "klama cu .alis. zoi gy 𝙰«a gy lenu";

    let mut client = LspClient::spawn();
    let initialize_result = initialize(&mut client, position_encoding, true);
    let legend =
        initialize_result["capabilities"]["semanticTokensProvider"]["legend"]["tokenTypes"]
            .as_array()
            .expect("semantic token legend");
    open_document_text(&mut client, URI, 1, TEXT);
    let result = semantic_tokens(&mut client, URI);
    assert!(
        result.get("resultId").is_none(),
        "M1 does not advertise delta"
    );
    assert_eq!(
        decode_semantic_tokens(&result, legend),
        vec![
            json!({"line": 0, "start": 0, "length": 5, "kind": "gismu", "modifiers": 0}),
            json!({"line": 0, "start": 6, "length": 2, "kind": "cmavo", "modifiers": 0}),
            json!({"line": 0, "start": 10, "length": 4, "kind": "cmevla", "modifiers": 0}),
            json!({"line": 0, "start": 16, "length": 3, "kind": "quotationMarker", "modifiers": 0}),
            json!({"line": 0, "start": 20, "length": 2, "kind": "quotationMarker", "modifiers": 0}),
            json!({"line": 0, "start": 23, "length": payload_length, "kind": "string", "modifiers": 0}),
            json!({"line": 0, "start": closing_start, "length": 2, "kind": "quotationMarker", "modifiers": 0}),
            json!({"line": 0, "start": le_start, "length": 2, "kind": "sumtiWord", "modifiers": 0}),
            json!({"line": 0, "start": le_start + 2, "length": 2, "kind": "selbriWord", "modifiers": 0}),
        ]
    );
    client.shutdown();
}

#[requires(error_start < error_end)]
#[ensures(true)]
fn assert_pull_round_trip(position_encoding: &str, error_start: u64, error_end: u64) {
    let mut client = LspClient::spawn();
    initialize(&mut client, position_encoding, true);
    open_document(&mut client, 1);

    let first = pull_diagnostics(&mut client, None);
    assert_eq!(first["kind"], "full");
    let first_result_id = first["resultId"]
        .as_str()
        .expect("diagnostic result id")
        .to_owned();
    assert!(first_result_id.ends_with(":1:confirmed"));
    let diagnostics = first["items"].as_array().expect("diagnostic items");
    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic["code"] == "syntax.unexpected-cmavo")
        .expect("syntax error diagnostic");
    assert_eq!(
        diagnostic["range"]["start"],
        json!({"line": 0, "character": error_start})
    );
    assert_eq!(
        diagnostic["range"]["end"],
        json!({"line": 0, "character": error_end})
    );
    assert_eq!(diagnostic["severity"], 1);
    assert_eq!(diagnostic["source"], "jbotci/syntax");
    assert!(
        diagnostic["message"]
            .as_str()
            .is_some_and(|message| message.contains('\n'))
    );
    for related in diagnostic["relatedInformation"]
        .as_array()
        .expect("secondary labels become related information")
    {
        assert_eq!(related["location"]["uri"], DOCUMENT_URI);
    }

    client.notify(
        "textDocument/didChange",
        json!({
            "textDocument": { "uri": DOCUMENT_URI, "version": 2 },
            "contentChanges": [{
                "range": {
                    "start": { "line": 0, "character": error_start },
                    "end": { "line": 0, "character": error_end }
                },
                "text": ""
            }]
        }),
    );
    let clean = pull_diagnostics(&mut client, Some(&first_result_id));
    assert_eq!(clean["kind"], "full");
    let clean_result_id = clean["resultId"]
        .as_str()
        .expect("diagnostic result id")
        .to_owned();
    assert!(clean_result_id.contains(":2:"));
    assert_eq!(clean["items"], json!([]));

    let unchanged = pull_diagnostics(&mut client, Some(&clean_result_id));
    assert_eq!(unchanged["kind"], "unchanged");
    assert_eq!(unchanged["resultId"], clean_result_id);
    client.shutdown();
}

#[test]
#[requires(true)]
#[ensures(true)]
fn pull_diagnostics_negotiate_utf8_and_apply_utf8_edits() {
    assert_ne!(UTF8_ERROR_START, UTF16_ERROR_START);
    assert_ne!(UTF8_ERROR_START, 14, "UTF-8 must not use scalar columns");
    assert_pull_round_trip("utf-8", UTF8_ERROR_START, UTF8_ERROR_END);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn pull_diagnostics_fall_back_to_utf16_and_apply_utf16_edits() {
    assert_ne!(UTF16_ERROR_START, 14, "UTF-16 must not use scalar columns");
    assert_pull_round_trip("utf-16", UTF16_ERROR_START, UTF16_ERROR_END);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn pull_diagnostics_expose_provisional_then_confirmed_generations() {
    const URI: &str = "file:///incremental-diagnostics.jbo";
    const OLD_TEXT: &str = "mi klama\nni'o\ndo cadzu\nni'o\nmi ku i do";
    const NEW_TEXT: &str = "mi klama\nni'o\ndo cadzu le zarci\nni'o\nmi ku i do";

    let mut client = LspClient::spawn();
    initialize(&mut client, "utf-16", true);
    open_document_text(&mut client, URI, 1, OLD_TEXT);
    let initial = client.request(
        "textDocument/diagnostic",
        json!({
            "textDocument": { "uri": URI },
            "identifier": "jbotci",
            "previousResultId": null
        }),
    );
    let initial_id = initial["resultId"]
        .as_str()
        .expect("initial result id")
        .to_owned();
    assert!(initial_id.ends_with(":1:confirmed"));

    client.notify(
        "textDocument/didChange",
        json!({
            "textDocument": { "uri": URI, "version": 2 },
            "contentChanges": [{ "text": NEW_TEXT }]
        }),
    );
    let provisional = client.request(
        "textDocument/diagnostic",
        json!({
            "textDocument": { "uri": URI },
            "identifier": "jbotci",
            "previousResultId": initial_id
        }),
    );
    let provisional_id = provisional["resultId"]
        .as_str()
        .expect("provisional result id")
        .to_owned();
    assert!(provisional_id.ends_with(":2:provisional"));

    let confirmation_deadline = Instant::now() + Duration::from_secs(30);
    let confirmed = loop {
        let report = client.request(
            "textDocument/diagnostic",
            json!({
                "textDocument": { "uri": URI },
                "identifier": "jbotci",
                "previousResultId": &provisional_id
            }),
        );
        if report["resultId"]
            .as_str()
            .is_some_and(|result_id| result_id.ends_with(":2:confirmed"))
        {
            break report;
        }
        assert!(
            Instant::now() < confirmation_deadline,
            "confirmation must complete within the CI-twin boundary; last report: {report}",
        );
        thread::sleep(Duration::from_millis(10));
    };
    assert_eq!(provisional["items"], confirmed["items"]);
    client.shutdown();
}

#[test]
#[requires(true)]
#[ensures(true)]
fn shutdown_is_orderly_during_background_confirmation() {
    const URI: &str = "file:///confirmation-shutdown.jbo";
    const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);

    let old_text = format!(
        "{}do cadzu\nni'o\nmi ku i do",
        "mi klama\nni'o\n".repeat(1_200),
    );
    let new_text = old_text.replacen("do cadzu", "do cadzu le zarci", 1);
    let mut client = LspClient::spawn();
    initialize(&mut client, "utf-16", true);
    open_document_text(&mut client, URI, 1, &old_text);
    let initial = client.request(
        "textDocument/diagnostic",
        json!({
            "textDocument": { "uri": URI },
            "identifier": "jbotci",
            "previousResultId": null
        }),
    );
    let initial_id = initial["resultId"]
        .as_str()
        .expect("initial result id")
        .to_owned();
    client.notify(
        "textDocument/didChange",
        json!({
            "textDocument": { "uri": URI, "version": 2 },
            "contentChanges": [{ "text": new_text }]
        }),
    );
    let provisional = client.request(
        "textDocument/diagnostic",
        json!({
            "textDocument": { "uri": URI },
            "identifier": "jbotci",
            "previousResultId": initial_id
        }),
    );
    assert!(
        provisional["resultId"]
            .as_str()
            .is_some_and(|result_id| result_id.ends_with(":2:provisional"))
    );

    // The confirming phase starts at the 200 ms debounce deadline. Keep
    // process ownership in a watchdog so a shutdown regression is bounded.
    thread::sleep(Duration::from_millis(205));
    let child = client
        .child
        .take()
        .expect("watchdog temporarily owns the LSP child");
    let (disarm_sender, disarm_receiver) = mpsc::sync_channel(1);
    let (child_sender, child_receiver) = mpsc::sync_channel(1);
    let watchdog = thread::spawn(move || {
        let mut child = child;
        let timed_out = disarm_receiver.recv_timeout(SHUTDOWN_TIMEOUT).is_err();
        if timed_out {
            let _ = child.kill();
            let _ = child.wait();
        }
        let _ = child_sender.send((child, timed_out));
    });

    assert_eq!(client.request("shutdown", Value::Null), Value::Null);
    disarm_sender
        .send(())
        .expect("disarm confirmation shutdown watchdog");
    let (child, timed_out) = child_receiver.recv().expect("recover LSP child");
    watchdog.join().expect("confirmation watchdog joins");
    assert!(!timed_out, "shutdown timed out during confirming parse");
    client.child = Some(child);
    client.exit_after_shutdown();
}

#[test]
#[requires(true)]
#[ensures(true)]
fn hover_uses_utf8_ranges_for_a_spaceless_sequence() {
    assert_hover_ranges("utf-8", 18, 20);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn hover_uses_utf16_ranges_for_a_spaceless_sequence() {
    assert_hover_ranges("utf-16", 15, 17);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn selection_ranges_round_trip_multibyte_utf8_positions() {
    assert_selection_range_encoding("utf-8");
}

#[test]
#[requires(true)]
#[ensures(true)]
fn selection_ranges_round_trip_multibyte_utf16_positions() {
    assert_selection_range_encoding("utf-16");
}

#[test]
#[requires(true)]
#[ensures(true)]
fn folding_ranges_are_plain_encoding_independent_lines_on_recovered_text() {
    const URI: &str = "file:///folding-encoding.jbo";
    const TEXT: &str = concat!(
        "mi cusku zoi gy 𝙰«a gy .i mi cusku lu\n",
        "do tavla\n",
        "li'u\n",
        ".i ku cu klama",
    );

    let mut results = Vec::new();
    for position_encoding in ["utf-8", "utf-16"] {
        let mut client = LspClient::spawn();
        initialize(&mut client, position_encoding, true);
        open_document_text(&mut client, URI, 1, TEXT);
        results.push(folding_ranges(&mut client, URI));
        client.shutdown();
    }

    assert_eq!(results[0], results[1]);
    let ranges = results[0].as_array().expect("folding range array");
    assert!(
        ranges
            .iter()
            .any(|range| range["startLine"] == 0 && range["endLine"] == 2),
        "the real LU quotation must remain foldable despite later recovery: {ranges:?}",
    );
    assert!(
        ranges.iter().all(|range| {
            range.get("startCharacter").is_none()
                && range.get("endCharacter").is_none()
                && range.get("kind").is_none()
                && range.get("collapsedText").is_none()
        }),
        "folds must remain plain line ranges without invented semantic kinds",
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn hover_round_trip_exposes_compact_zei_compound_markdown() {
    const URI: &str = "file:///hover-compact.jbo";
    const TEXT: &str = "gleki zei py.";

    let mut client = LspClient::spawn();
    initialize(&mut client, "utf-16", true);
    open_document_text(&mut client, URI, 1, TEXT);

    let result = hover(&mut client, URI, 1);
    assert_eq!(result["contents"]["kind"], "markdown");
    let markdown = result["contents"]["value"]
        .as_str()
        .expect("hover contents are Markdown text");
    assert!(markdown.starts_with("### `gleki zei py` — *ZEI compound*\n\n---\n\n"));
    assert!(markdown.contains("### `gleki` — *gismu*"));
    assert!(markdown.contains("\n\n---\n\n### `py` — *cmavo* · **BY2**"));
    assert!(!markdown.contains("**Word type:**"));
    assert!(!markdown.contains("Component definitions"));
    assert_eq!(
        result["range"],
        json!({
            "start": { "line": 0, "character": 0 },
            "end": { "line": 0, "character": 12 }
        }),
    );
    client.shutdown();
}

#[test]
#[requires(true)]
#[ensures(true)]
fn completion_uses_utf8_seed_edit_ranges_and_resolves_markdown() {
    assert_completion_encoding("utf-8");
}

#[test]
#[requires(true)]
#[ensures(true)]
fn completion_uses_utf16_seed_edit_ranges_and_resolves_markdown() {
    assert_completion_encoding("utf-16");
}

#[test]
#[requires(true)]
#[ensures(true)]
fn completion_maps_preselection_and_two_document_local_sort_blocks() {
    const ORDER_URI: &str = "file:///completion-order.jbo";
    const ORDER_TEXT: &str = "mi barda gi'e cadzu le ";
    const PRESELECT_URI: &str = "file:///completion-preselect.jbo";
    const PRESELECT_TEXT: &str = "mi klama le zarci";

    let mut client = LspClient::spawn();
    initialize(&mut client, "utf-16", true);
    open_document_text(&mut client, ORDER_URI, 1, ORDER_TEXT);
    let ordered = completion(&mut client, ORDER_URI, ORDER_TEXT.chars().count() as u64);
    let ordered = ordered.as_array().expect("completion array");
    let local = ordered
        .iter()
        .filter(|item| {
            item["sortText"]
                .as_str()
                .is_some_and(|sort_text| sort_text.starts_with("0·"))
        })
        .map(|item| item["label"].as_str().expect("completion label"))
        .collect::<Vec<_>>();
    let remainder = ordered
        .iter()
        .filter(|item| {
            item["sortText"]
                .as_str()
                .is_some_and(|sort_text| sort_text.starts_with("1·"))
        })
        .map(|item| item["label"].as_str().expect("completion label"))
        .collect::<Vec<_>>();
    assert!(local.contains(&"barda") && local.contains(&"cadzu"));
    assert!(!remainder.is_empty());
    assert!(local.windows(2).all(|pair| pair[0] <= pair[1]));
    assert!(remainder.windows(2).all(|pair| pair[0] <= pair[1]));

    open_document_text(&mut client, PRESELECT_URI, 1, PRESELECT_TEXT);
    let cursor = PRESELECT_TEXT.find("zarci").expect("fixture word") + "zar".len();
    let items = completion(&mut client, PRESELECT_URI, cursor as u64);
    let items = items.as_array().expect("completion array");
    let preselected = items
        .iter()
        .filter(|item| item["preselect"] == true)
        .collect::<Vec<_>>();
    assert_eq!(preselected.len(), 1);
    assert_eq!(preselected[0]["label"], "zarci");
    assert_eq!(preselected[0]["sortText"], "0·zarci");
    client.shutdown();
}

#[test]
#[requires(true)]
#[ensures(true)]
fn error_heavy_completion_returns_before_followup_diagnostics() {
    let text = format!("{}mukti lo nu", "mi ku .i ".repeat(14));
    let cursor = text
        .find("mukti lo nu")
        .expect("fixture contains the completion phrase")
        + "mukti l".len();
    let mut client = LspClient::spawn();
    initialize(&mut client, "utf-16", true);
    open_document_text(&mut client, DOCUMENT_URI, 1, &text);

    let started = Instant::now();
    let completions = completion(&mut client, DOCUMENT_URI, cursor as u64);
    assert!(
        started.elapsed() < Duration::from_secs(3),
        "error-heavy LSP completion unexpectedly took {:?}",
        started.elapsed(),
    );
    assert!(completions.is_array(), "completion must return a response");

    let diagnostics = pull_diagnostics(&mut client, None);
    assert_eq!(diagnostics["kind"], "full");
    assert!(
        diagnostics["items"]
            .as_array()
            .is_some_and(|items| !items.is_empty()),
        "the server must answer diagnostics after error-heavy completion",
    );
    client.shutdown();
}

#[test]
#[requires(true)]
#[ensures(true)]
fn shutdown_completes_during_an_in_flight_completion() {
    const URI: &str = "file:///completion-shutdown.jbo";
    const TEXT: &str = ".i la prux. ba'o sruma lo du'u le ckule cipra ku frili ra";
    const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);

    let cursor = TEXT.find("sruma").expect("fixture contains sruma") as u64;
    let mut client = LspClient::spawn();
    initialize(&mut client, "utf-16", true);
    open_document_text(&mut client, URI, 1, TEXT);
    let diagnostics = client.request(
        "textDocument/diagnostic",
        json!({
            "textDocument": { "uri": URI },
            "identifier": "jbotci",
            "previousResultId": null
        }),
    );
    assert_eq!(diagnostics["kind"], "full");

    // Keep process ownership in a watchdog while the main test thread blocks
    // on protocol input. A regression that wedges the protocol loop therefore
    // fails in bounded time instead of hanging the entire test process.
    let child = client
        .child
        .take()
        .expect("watchdog temporarily owns the LSP child");
    let (disarm_sender, disarm_receiver) = mpsc::sync_channel(1);
    let (child_sender, child_receiver) = mpsc::sync_channel(1);
    let watchdog = thread::spawn(move || {
        let mut child = child;
        let timed_out = disarm_receiver.recv_timeout(SHUTDOWN_TIMEOUT).is_err();
        if timed_out {
            let _ = child.kill();
            let _ = child.wait();
        }
        let _ = child_sender.send((child, timed_out));
    });

    let _completion_id = client.start_request(
        "textDocument/completion",
        json!({
            "textDocument": { "uri": URI },
            "position": { "line": 0, "character": cursor }
        }),
    );
    let shutdown_id = client.start_request("shutdown", Value::Null);
    assert_eq!(client.response(shutdown_id), Value::Null);

    disarm_sender
        .send(())
        .expect("disarm the shutdown watchdog");
    let (child, timed_out) = child_receiver
        .recv()
        .expect("watchdog returns the LSP child");
    watchdog.join().expect("shutdown watchdog must not panic");
    assert!(
        !timed_out,
        "shutdown response exceeded {SHUTDOWN_TIMEOUT:?}"
    );
    client.child = Some(child);
    client.exit_after_shutdown();
}

#[test]
#[requires(true)]
#[ensures(true)]
fn semantic_tokens_full_uses_utf8_units_and_morphology_boundaries() {
    assert_semantic_token_encoding("utf-8", 7, 31, 34);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn semantic_tokens_full_uses_utf16_units_and_morphology_boundaries() {
    assert_semantic_token_encoding("utf-16", 4, 28, 31);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn structure_inlays_are_range_scoped_on_a_recovered_document() {
    const URI: &str = "file:///structure-recovered.jbo";
    const TEXT: &str = "mi ku i do viska le mlatu\n";

    let mut client = LspClient::spawn();
    initialize(&mut client, "utf-16", true);
    open_document_text(&mut client, URI, 1, TEXT);

    let full = inlay_hints(
        &mut client,
        URI,
        json!({"line": 0, "character": 0}),
        json!({"line": 1, "character": 0}),
    );
    let subset = inlay_hints(
        &mut client,
        URI,
        json!({"line": 0, "character": 6}),
        json!({"line": 0, "character": 25}),
    );
    let full = full.as_array().expect("full inlay array");
    let subset = subset.as_array().expect("subset inlay array");
    assert!(subset.len() < full.len());
    assert_eq!(
        subset
            .iter()
            .map(|inlay| inlay["position"]["character"].as_u64())
            .collect::<Vec<_>>(),
        vec![Some(6), Some(8), Some(11), Some(17)],
    );
    assert!(subset.iter().all(|inlay| {
        inlay["position"]["line"] == 0
            && inlay["position"]["character"]
                .as_u64()
                .is_some_and(|character| (6..25).contains(&character))
            && inlay["label"]
                .as_str()
                .is_some_and(|label| !label.is_empty())
            && inlay.get("kind").is_none()
            && inlay.get("textEdits").is_none()
    }));
    assert!(
        subset.iter().any(|inlay| inlay["position"]["character"]
            .as_u64()
            .is_some_and(|character| character > "mi ku".len() as u64)),
        "structure hints must continue after the recovered error",
    );
    client.shutdown();
}

#[test]
#[requires(true)]
#[ensures(true)]
fn structure_inlay_initialization_options_select_construct_profile() {
    const URI: &str = "file:///structure-profile.jbo";
    const TEXT: &str = "mi viska le mlatu\n";

    let mut client = LspClient::spawn();
    initialize_with_options(
        &mut client,
        "utf-16",
        true,
        Some(json!({
            "inlays": {
                "structureBrackets": {
                    "profile": "raw-brackets",
                    "constructs": "sumti-boundaries"
                }
            }
        })),
    );
    open_document_text(&mut client, URI, 1, TEXT);
    let hints = inlay_hints(
        &mut client,
        URI,
        json!({"line": 0, "character": 0}),
        json!({"line": 1, "character": 0}),
    );
    let hints = hints.as_array().expect("profile inlay array");
    assert_eq!(hints.len(), 2, "only the multiword sumti boundary remains");
    assert_eq!(hints[0]["position"], json!({"line": 0, "character": 9}));
    assert_eq!(hints[1]["position"], json!({"line": 0, "character": 17}));
    client.shutdown();
}

#[test]
#[requires(true)]
#[ensures(true)]
fn legacy_structure_inlay_options_emit_a_deprecation_warning() {
    let mut client = LspClient::spawn();
    initialize_with_options(
        &mut client,
        "utf-16",
        true,
        Some(json!({
            "structureInlays": {
                "profile": "raw-brackets",
                "constructs": "sumti-boundaries"
            }
        })),
    );
    let warning = client.next_notification("window/logMessage");
    assert_eq!(warning["params"]["type"], 2);
    assert_eq!(
        warning["params"]["message"],
        "initializationOptions.structureInlays is deprecated; use initializationOptions.inlays.structureBrackets instead",
    );
    client.shutdown();
}

#[test]
#[requires(true)]
#[ensures(true)]
fn word_and_rafsi_inlay_flags_and_ranges_are_independent() {
    const URI: &str = "file:///word-stream-inlays.jbo";
    const TEXT: &str = "uanaisaidai ua nai sai dai lenkymipri\n";

    let hints_for = |word_boundaries: bool, rafsi_boundaries: bool, start: u64, end: u64| {
        let mut client = LspClient::spawn();
        initialize_with_options(
            &mut client,
            "utf-16",
            true,
            Some(json!({
                "inlays": {
                    "structureBrackets": false,
                    "wordBoundaries": word_boundaries,
                    "rafsiBoundaries": rafsi_boundaries
                }
            })),
        );
        open_document_text(&mut client, URI, 1, TEXT);
        let hints = inlay_hints(
            &mut client,
            URI,
            json!({"line": 0, "character": start}),
            json!({"line": 0, "character": end}),
        );
        client.shutdown();
        hints
    };

    let word_only = hints_for(true, false, 0, TEXT.len() as u64);
    assert_eq!(
        inlay_positions_and_labels(&word_only),
        vec![
            (2, "-".to_owned()),
            (5, "-".to_owned()),
            (8, "-".to_owned()),
        ],
    );

    let rafsi_only = hints_for(false, true, 0, TEXT.len() as u64);
    assert_eq!(
        inlay_positions_and_labels(&rafsi_only),
        vec![(31, "·".to_owned()), (32, "·".to_owned())],
    );

    let scoped = hints_for(true, true, 5, 32);
    assert_eq!(
        inlay_positions_and_labels(&scoped),
        vec![
            (5, "-".to_owned()),
            (8, "-".to_owned()),
            (31, "·".to_owned()),
        ],
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn word_stream_inlay_positions_round_trip_on_a_multibyte_line() {
    const URI: &str = "file:///word-stream-encoding.jbo";
    const TEXT: &str = "mí uanaisaidai lenkymipri\n";

    let hints_for_encoding = |encoding: &str| {
        let mut client = LspClient::spawn();
        initialize_with_options(
            &mut client,
            encoding,
            true,
            Some(json!({
                "inlays": {
                    "structureBrackets": false,
                    "wordBoundaries": true,
                    "rafsiBoundaries": true
                }
            })),
        );
        open_document_text(&mut client, URI, 1, TEXT);
        let hints = inlay_hints(
            &mut client,
            URI,
            json!({"line": 0, "character": 0}),
            json!({"line": 1, "character": 0}),
        );
        client.shutdown();
        hints
    };
    assert_eq!(
        inlay_positions_and_labels(&hints_for_encoding("utf-8")),
        vec![
            (6, "-".to_owned()),
            (9, "-".to_owned()),
            (12, "-".to_owned()),
            (20, "·".to_owned()),
            (21, "·".to_owned()),
        ],
    );
    assert_eq!(
        inlay_positions_and_labels(&hints_for_encoding("utf-16")),
        vec![
            (5, "-".to_owned()),
            (8, "-".to_owned()),
            (11, "-".to_owned()),
            (19, "·".to_owned()),
            (20, "·".to_owned()),
        ],
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn structure_inlay_positions_round_trip_on_a_multibyte_line() {
    const URI: &str = "file:///structure-encoding.jbo";
    const TEXT: &str = "mí klama le zarci\n";

    let hints_for_encoding = |encoding: &str| {
        let mut client = LspClient::spawn();
        initialize(&mut client, encoding, true);
        open_document_text(&mut client, URI, 1, TEXT);
        let hints = inlay_hints(
            &mut client,
            URI,
            json!({"line": 0, "character": 0}),
            json!({"line": 1, "character": 0}),
        );
        client.shutdown();
        hints
    };
    let utf8 = hints_for_encoding("utf-8");
    let utf16 = hints_for_encoding("utf-16");
    let utf8 = utf8.as_array().expect("UTF-8 inlay array");
    let utf16 = utf16.as_array().expect("UTF-16 inlay array");
    assert_eq!(utf8.len(), utf16.len());
    assert!(!utf8.is_empty());

    let mut observed_distinct_column = false;
    for (utf8_hint, utf16_hint) in utf8.iter().zip(utf16) {
        assert_eq!(utf8_hint["label"], utf16_hint["label"]);
        let byte_column = utf8_hint["position"]["character"]
            .as_u64()
            .expect("UTF-8 byte column") as usize;
        let utf16_column = utf16_hint["position"]["character"]
            .as_u64()
            .expect("UTF-16 column") as usize;
        assert!(TEXT.is_char_boundary(byte_column));
        assert_eq!(utf16_column, TEXT[..byte_column].encode_utf16().count());
        observed_distinct_column |= byte_column != utf16_column;
    }
    assert!(
        observed_distinct_column,
        "the multibyte prefix must distinguish UTF-8 and UTF-16 columns",
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn stdio_compatibility_flag_runs_the_same_lsp_transport() {
    let mut client = LspClient::spawn_with_stdio_flag(true);
    initialize(&mut client, "utf-16", true);
    client.shutdown();
}

#[test]
#[requires(true)]
#[ensures(true)]
fn clients_without_pull_support_receive_push_diagnostics_and_close_clears_them() {
    let mut client = LspClient::spawn();
    initialize(&mut client, "utf-8", false);
    open_document(&mut client, 5);

    let first = client.next_notification("textDocument/publishDiagnostics");
    assert_eq!(first["params"]["uri"], DOCUMENT_URI);
    assert_eq!(first["params"]["version"], 5);
    assert!(
        first["params"]["diagnostics"]
            .as_array()
            .is_some_and(|diagnostics| !diagnostics.is_empty())
    );

    client.notify(
        "textDocument/didChange",
        json!({
            "textDocument": { "uri": DOCUMENT_URI, "version": 6 },
            "contentChanges": [{
                "range": {
                    "start": { "line": 0, "character": UTF8_ERROR_START },
                    "end": { "line": 0, "character": UTF8_ERROR_END }
                },
                "text": ""
            }]
        }),
    );
    let clean = client.next_notification("textDocument/publishDiagnostics");
    assert_eq!(clean["params"]["version"], 6);
    assert_eq!(clean["params"]["diagnostics"], json!([]));

    client.notify(
        "textDocument/didClose",
        json!({ "textDocument": { "uri": DOCUMENT_URI } }),
    );
    let closed = client.next_notification("textDocument/publishDiagnostics");
    assert_eq!(closed["params"]["uri"], DOCUMENT_URI);
    assert!(closed["params"].get("version").is_none());
    assert_eq!(closed["params"]["diagnostics"], json!([]));
    client.shutdown();
}

#[test]
#[requires(true)]
#[ensures(true)]
fn stale_versions_are_rejected_full_sync_is_accepted_and_close_drops_state() {
    let mut client = LspClient::spawn();
    initialize(&mut client, "utf-16", true);
    open_document(&mut client, 10);

    client.notify(
        "textDocument/didChange",
        json!({
            "textDocument": { "uri": DOCUMENT_URI, "version": 9 },
            "contentChanges": [{ "text": "mi klama" }]
        }),
    );
    let stale = pull_diagnostics(&mut client, None);
    let stale_result_id = stale["resultId"]
        .as_str()
        .expect("diagnostic result id")
        .to_owned();
    assert!(stale_result_id.ends_with(":10:confirmed"));
    assert!(
        stale["items"]
            .as_array()
            .is_some_and(|diagnostics| !diagnostics.is_empty())
    );

    client.notify(
        "textDocument/didChange",
        json!({
            "textDocument": { "uri": DOCUMENT_URI, "version": 11 },
            "contentChanges": [{ "text": "mi klama" }]
        }),
    );
    let full_sync = pull_diagnostics(&mut client, Some(&stale_result_id));
    assert!(
        full_sync["resultId"]
            .as_str()
            .is_some_and(|result_id| result_id.contains(":11:"))
    );
    assert_eq!(full_sync["items"], json!([]));

    client.notify(
        "textDocument/didClose",
        json!({ "textDocument": { "uri": DOCUMENT_URI } }),
    );
    let closed = pull_diagnostics(&mut client, None);
    assert_eq!(closed["kind"], "full");
    assert_eq!(closed["items"], json!([]));
    assert!(closed.get("resultId").is_none());
    client.shutdown();
}

#[test]
#[requires(true)]
#[ensures(true)]
fn save_and_unknown_notifications_do_not_kill_the_server() {
    let mut client = LspClient::spawn();
    initialize(&mut client, "utf-8", true);
    open_document(&mut client, 1);

    // Editors freely send save and workspace notifications the server does not
    // handle; LSP requires ignoring them. Before the unhandled-notification
    // fallback was installed, the first didSave killed the main loop and the
    // editor observed an EOF on stdout.
    client.notify(
        "textDocument/didSave",
        json!({ "textDocument": { "uri": DOCUMENT_URI } }),
    );
    client.notify(
        "textDocument/willSave",
        json!({ "textDocument": { "uri": DOCUMENT_URI }, "reason": 1 }),
    );
    client.notify(
        "workspace/didChangeConfiguration",
        json!({ "settings": {} }),
    );
    client.notify("custom/experimental", json!({ "payload": true }));

    let report = pull_diagnostics(&mut client, None);
    assert_eq!(report["kind"], "full");
    assert!(
        report["items"]
            .as_array()
            .is_some_and(|diagnostics| !diagnostics.is_empty()),
        "server must still answer diagnostics after ignoring unknown notifications"
    );
    client.shutdown();
}

/// Editors also spawn LSP servers over socket pairs or ptys rather than
/// pipes; the transport must fall back from the pipe-only fast path instead
/// of dying at startup.
#[cfg(unix)]
#[test]
#[requires(true)]
#[ensures(true)]
fn server_survives_socketpair_stdio() {
    use std::os::fd::OwnedFd;
    use std::os::unix::net::UnixStream;

    let (mut parent, child_end) = UnixStream::pair().expect("socketpair");
    let child_stdin = child_end.try_clone().expect("clone socket for child stdin");
    let mut child = Command::new(env!("CARGO_BIN_EXE_jbotci"))
        .arg("lsp")
        .stdin(Stdio::from(OwnedFd::from(child_stdin)))
        .stdout(Stdio::from(OwnedFd::from(child_end)))
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn jbotci lsp over socketpair");

    let body = serde_json::to_vec(&json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "processId": null,
            "capabilities": {
                "general": { "positionEncodings": ["utf-8"] }
            }
        }
    }))
    .expect("serialize initialize");
    write!(parent, "Content-Length: {}\r\n\r\n", body.len()).expect("write header");
    parent.write_all(&body).expect("write body");
    parent.flush().expect("flush initialize");

    let mut reader = BufReader::new(parent.try_clone().expect("clone socket for reads"));
    let mut content_length = None;
    loop {
        let mut header = String::new();
        let read = reader.read_line(&mut header).expect("read response header");
        assert_ne!(read, 0, "server closed the socket before responding");
        if header == "\r\n" || header == "\n" {
            break;
        }
        if let Some(value) = header
            .strip_prefix("Content-Length:")
            .map(str::trim)
            .and_then(|value| value.parse::<usize>().ok())
        {
            content_length = Some(value);
        }
    }
    let mut body = vec![0_u8; content_length.expect("Content-Length header")];
    reader.read_exact(&mut body).expect("read response body");
    let response: Value = serde_json::from_slice(&body).expect("parse response");
    assert_eq!(response["id"], 1);
    assert_eq!(
        response["result"]["capabilities"]["positionEncoding"],
        "utf-8"
    );

    child.kill().expect("kill server");
    let _ = child.wait();
}
