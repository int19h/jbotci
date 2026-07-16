use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, ChildStderr, ChildStdin, ChildStdout, Command, Stdio};

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
    child: Child,
    stdin: Option<ChildStdin>,
    stdout: BufReader<ChildStdout>,
    stderr: ChildStderr,
    next_id: u64,
}

impl LspClient {
    #[requires(true)]
    #[ensures(true)]
    fn spawn() -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_jbotci"))
            .arg("lsp")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn jbotci lsp");
        let stdin = child.stdin.take().expect("child stdin");
        let stdout = child.stdout.take().expect("child stdout");
        let stderr = child.stderr.take().expect("child stderr");
        Self {
            child,
            stdin: Some(stdin),
            stdout: BufReader::new(stdout),
            stderr,
            next_id: 1,
        }
    }

    #[requires(!method.is_empty())]
    #[ensures(ret.is_null() || ret.is_object())]
    fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.next_id;
        self.next_id += 1;
        self.send(json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        }));
        loop {
            let message = self.read();
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
            if message.get("method").and_then(Value::as_str) == Some(method) {
                return message;
            }
        }
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
        self.notify("exit", Value::Null);
        drop(self.stdin.take());
        let status = self.child.wait().expect("wait for LSP server");
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
    let result = client.request(
        "initialize",
        json!({
            "processId": null,
            "capabilities": {
                "general": {
                    "positionEncodings": [position_encoding]
                },
                "textDocument": text_document
            }
        }),
    );
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
    assert!(result["capabilities"].get("hoverProvider").is_none());
    assert!(result["capabilities"].get("completionProvider").is_none());
    assert!(
        result["capabilities"]
            .get("semanticTokensProvider")
            .is_none()
    );
    client.notify("initialized", json!({}));
    result
}

#[requires(version >= 0)]
#[ensures(true)]
fn open_document(client: &mut LspClient, version: i32) {
    client.notify(
        "textDocument/didOpen",
        json!({
            "textDocument": {
                "uri": DOCUMENT_URI,
                "languageId": "lojban",
                "version": version,
                "text": INVALID_TEXT
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

#[requires(error_start < error_end)]
#[ensures(true)]
fn assert_pull_round_trip(position_encoding: &str, error_start: u64, error_end: u64) {
    let mut client = LspClient::spawn();
    initialize(&mut client, position_encoding, true);
    open_document(&mut client, 1);

    let first = pull_diagnostics(&mut client, None);
    assert_eq!(first["kind"], "full");
    assert_eq!(first["resultId"], "1");
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
    let clean = pull_diagnostics(&mut client, Some("1"));
    assert_eq!(clean["kind"], "full");
    assert_eq!(clean["resultId"], "2");
    assert_eq!(clean["items"], json!([]));

    let unchanged = pull_diagnostics(&mut client, Some("2"));
    assert_eq!(unchanged, json!({"kind": "unchanged", "resultId": "2"}));
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
    assert_eq!(stale["resultId"], "10");
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
    let full_sync = pull_diagnostics(&mut client, Some("10"));
    assert_eq!(full_sync["resultId"], "11");
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
    client.notify("workspace/didChangeConfiguration", json!({ "settings": {} }));
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
