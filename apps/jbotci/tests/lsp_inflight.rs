//! The stdio server must answer a burst larger than its worker pool.
//!
//! A hover on a confirmed document returns before waits on a new document.
//! A cancel of a waiting folding range returns code -32800.
//! A timeout message includes stderr text that has already arrived.
//!
//! The burst is larger than the worker pool.
//! Linux pins the child to one CPU in the current mask.
//! The pool size is then two.
//! The burst length is 32.
//! Another host uses the real pool size plus eight.
//! The length is at least 32.
//!
//! The pass condition is the response set.
//! Every burst id arrives once.
//! The next diagnostic then carries document version 2.
//! Shutdown then returns a null result.
//! The 120 second deadline is only a hang bound.
//! A full pool that blocks the reader writes no burst response.
//! Completion requests use a grammar limit of one second.
//! The Linux burst finishes inside the bound.

use std::collections::{HashMap, HashSet};
use std::io::{BufRead, BufReader, Read, Write};
use std::num::NonZeroUsize;
#[cfg(target_os = "linux")]
use std::os::unix::process::CommandExt;
use std::panic::{AssertUnwindSafe, resume_unwind};
use std::process::{Child, ChildStdin, ChildStdout, Command, ExitStatus, Stdio};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use std::{thread, time};

#[allow(unused_imports)]
use bityzba::{ensures, invariant, new, requires};
use serde_json::{Value, json};

const DOCUMENT_URI: &str = "untitled:inflight-repro.jbo";
const DOCUMENT_TEXT: &str = "mi klama .i do klama";
const EDITED_TEXT: &str = "do klama";
const RESPONSE_DEADLINE: Duration = Duration::from_secs(120);
const EXIT_DEADLINE: Duration = Duration::from_secs(10);

#[requires(true)]
#[ensures(ret >= 32)]
fn burst_len() -> u64 {
    // Linux pins the child to one CPU.
    // The worker pool size is then two.
    if cfg!(target_os = "linux") {
        return 32;
    }
    let count = std::thread::available_parallelism()
        .map(NonZeroUsize::get)
        .unwrap_or(2)
        .max(2)
        .saturating_add(8)
        .max(32);
    u64::try_from(count).unwrap_or(u64::MAX)
}

#[invariant(*pid > 0)]
struct RunningServer {
    pid: u32,
    child: Mutex<Child>,
    stderr: Arc<Mutex<String>>,
    stderr_reader: Mutex<Option<JoinHandle<()>>>,
    stdout_reader: Mutex<Option<JoinHandle<()>>>,
}

impl RunningServer {
    #[requires(true)]
    #[ensures(ret.0.pid > 0)]
    fn spawn() -> (Self, ChildStdin, std::sync::mpsc::Receiver<Value>) {
        let mut command = Command::new(env!("CARGO_BIN_EXE_jbotci"));
        command
            .arg("lsp")
            .arg("--stdio")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        // Linux runs this hook in the child after fork and before exec.
        // The parent keeps its own CPU set.
        #[cfg(target_os = "linux")]
        unsafe {
            command.pre_exec(restrict_child_to_one_cpu);
        }
        let mut child = command.spawn().expect("spawn jbotci lsp");
        let stdin = child.stdin.take().expect("child stdin");
        let stdout = child.stdout.take().expect("child stdout");
        let stderr = child.stderr.take().expect("child stderr");
        let pid = child.id();
        let stderr_log = Arc::new(Mutex::new(String::new()));
        let logged = Arc::clone(&stderr_log);
        let stderr_reader = thread::spawn(move || {
            let mut reader = BufReader::new(stderr);
            let mut chunk = [0_u8; 1024];
            loop {
                match Read::read(&mut reader, &mut chunk) {
                    Ok(0) => break,
                    Ok(size) => {
                        let text = String::from_utf8_lossy(&chunk[..size]);
                        logged.lock().expect("stderr log poisoned").push_str(&text);
                    }
                    Err(_) => break,
                }
            }
        });
        let (messages, stdout_reader) = spawn_reader(stdout);
        (
            new!(RunningServer {
                pid,
                child: Mutex::new(child),
                stderr: stderr_log,
                stderr_reader: Mutex::new(Some(stderr_reader)),
                stdout_reader: Mutex::new(Some(stdout_reader)),
            }),
            stdin,
            messages,
        )
    }

    #[requires(true)]
    #[ensures(true)]
    fn try_wait(&self) -> Option<ExitStatus> {
        self.child
            .lock()
            .expect("server process lock poisoned")
            .try_wait()
            .expect("poll LSP server")
    }

    #[requires(true)]
    #[ensures(true)]
    fn stderr_text(&self) -> String {
        self.stderr.lock().expect("stderr log poisoned").clone()
    }

    #[requires(true)]
    #[ensures(true)]
    fn stop(&self) {
        let mut child = self.child.lock().expect("server process lock poisoned");
        // try_wait stores the status after the first reap.
        // A kill after that reap can hit a pid that the OS reused.
        if matches!(child.try_wait(), Ok(None)) {
            let _ = child.kill();
            let _ = child.wait();
        }
        drop(child);
        self.join_readers();
    }

    #[requires(true)]
    #[ensures(true)]
    fn join_readers(&self) {
        if let Some(reader) = self
            .stdout_reader
            .lock()
            .expect("stdout reader lock poisoned")
            .take()
        {
            let _ = reader.join();
        }
        if let Some(reader) = self
            .stderr_reader
            .lock()
            .expect("stderr reader lock poisoned")
            .take()
        {
            let _ = reader.join();
        }
    }
}

#[cfg(target_os = "linux")]
#[requires(true)]
#[ensures(ret.is_none_or(|cpu| cpu < libc::CPU_SETSIZE as usize))]
fn first_allowed_cpu(set: &libc::cpu_set_t) -> Option<usize> {
    (0..libc::CPU_SETSIZE as usize).find(|&cpu| unsafe { libc::CPU_ISSET(cpu, set) })
}

/// The child uses the first CPU in the current mask.
///
/// An empty mask returns an error.
/// Spawn fails after that error.
#[cfg(target_os = "linux")]
#[requires(true)]
#[ensures(true)]
fn restrict_child_to_one_cpu() -> std::io::Result<()> {
    unsafe {
        let mut current = std::mem::zeroed::<libc::cpu_set_t>();
        let got = libc::sched_getaffinity(0, std::mem::size_of::<libc::cpu_set_t>(), &mut current);
        if got != 0 {
            return Err(std::io::Error::last_os_error());
        }
        let Some(cpu) = first_allowed_cpu(&current) else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "the current CPU mask is empty",
            ));
        };
        let mut set = std::mem::zeroed::<libc::cpu_set_t>();
        libc::CPU_ZERO(&mut set);
        libc::CPU_SET(cpu, &mut set);
        let result = libc::sched_setaffinity(0, std::mem::size_of::<libc::cpu_set_t>(), &set);
        if result == 0 {
            Ok(())
        } else {
            Err(std::io::Error::last_os_error())
        }
    }
}

#[cfg(target_os = "linux")]
#[test]
#[requires(true)]
#[ensures(true)]
fn first_allowed_cpu_skips_a_gap_at_zero() {
    let mut set = unsafe { std::mem::zeroed::<libc::cpu_set_t>() };
    unsafe {
        libc::CPU_ZERO(&mut set);
        libc::CPU_SET(3, &mut set);
    }
    assert_eq!(first_allowed_cpu(&set), Some(3));
}

#[cfg(target_os = "linux")]
#[test]
#[requires(true)]
#[ensures(true)]
fn first_allowed_cpu_rejects_an_empty_mask() {
    let mut set = unsafe { std::mem::zeroed::<libc::cpu_set_t>() };
    unsafe {
        libc::CPU_ZERO(&mut set);
    }
    assert!(first_allowed_cpu(&set).is_none());
}

#[requires(true)]
#[ensures(true)]
fn spawn_reader(stdout: ChildStdout) -> (std::sync::mpsc::Receiver<Value>, JoinHandle<()>) {
    let (sender, receiver) = std::sync::mpsc::channel();
    let reader = thread::spawn(move || {
        let mut stdout = BufReader::new(stdout);
        while let Some(message) = read_message(&mut stdout) {
            if sender.send(message).is_err() {
                break;
            }
        }
    });
    (receiver, reader)
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|message| message.is_object()))]
fn read_message(stdout: &mut impl BufRead) -> Option<Value> {
    let mut content_length = None;
    loop {
        let mut header = String::new();
        let read = stdout.read_line(&mut header).ok()?;
        if read == 0 {
            return None;
        }
        if header == "\r\n" || header == "\n" {
            break;
        }
        if let Some(value) = header
            .strip_prefix("Content-Length:")
            .or_else(|| header.strip_prefix("content-length:"))
        {
            content_length = Some(value.trim().parse::<usize>().ok()?);
        }
    }
    let mut body = vec![0; content_length?];
    if stdout.read_exact(&mut body).is_err() {
        return None;
    }
    serde_json::from_slice(&body).ok()
}

#[requires(message.is_object())]
#[ensures(true)]
fn write_message(stdin: &mut ChildStdin, message: &Value) {
    let body = serde_json::to_vec(message).expect("serialize LSP message");
    write!(stdin, "Content-Length: {}\r\n\r\n", body.len()).expect("write LSP header");
    stdin.write_all(&body).expect("write LSP body");
    stdin.flush().expect("flush LSP message");
}

#[requires(message.is_object())]
#[ensures(true)]
fn push_message(buffer: &mut Vec<u8>, message: &Value) {
    let body = serde_json::to_vec(message).expect("serialize LSP message");
    buffer.extend(format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes());
    buffer.extend(body);
}

#[requires(!method.is_empty())]
#[ensures(ret.is_object())]
fn request_message(id: u64, method: &str, params: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": method,
        "params": params,
    })
}

#[requires(!method.is_empty())]
#[ensures(ret.is_object())]
fn notification_message(method: &str, params: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": params,
    })
}

#[requires(true)]
#[ensures(ret.is_object())]
fn initialize_params() -> Value {
    json!({
        "processId": null,
        "capabilities": {
            "general": { "positionEncodings": ["utf-16"] },
            "workspace": { "diagnostic": { "refreshSupport": true } },
            "textDocument": {
                "diagnostic": { "dynamicRegistration": false },
                "publishDiagnostics": { "relatedInformation": true }
            }
        }
    })
}

#[requires(!method.is_empty())]
#[ensures(ret.is_object())]
fn params_for(method: &str, uri: &str) -> Value {
    match method {
        "textDocument/diagnostic" => json!({ "textDocument": { "uri": uri } }),
        "textDocument/hover" => json!({
            "textDocument": { "uri": uri },
            "position": { "line": 0, "character": 0 }
        }),
        "textDocument/foldingRange" => json!({ "textDocument": { "uri": uri } }),
        "textDocument/semanticTokens/full" => json!({ "textDocument": { "uri": uri } }),
        "textDocument/inlayHint" => json!({
            "textDocument": { "uri": uri },
            "range": {
                "start": { "line": 0, "character": 0 },
                "end": { "line": 0, "character": 19 }
            }
        }),
        "textDocument/completion" => json!({
            "textDocument": { "uri": uri },
            "position": { "line": 0, "character": 2 }
        }),
        "textDocument/selectionRange" => json!({
            "textDocument": { "uri": uri },
            "positions": [{ "line": 0, "character": 0 }]
        }),
        "completionItem/resolve" => json!({ "label": "mi" }),
        other => panic!("unexpected burst method {other}"),
    }
}

#[requires(true)]
#[ensures(ret.len() == usize::try_from(burst_len()).expect("burst length fits"))]
fn burst_methods() -> Vec<&'static str> {
    const HEAVY: [&str; 7] = [
        "textDocument/diagnostic",
        "textDocument/hover",
        "textDocument/foldingRange",
        "textDocument/semanticTokens/full",
        "textDocument/inlayHint",
        "textDocument/completion",
        "textDocument/selectionRange",
    ];
    (0..burst_len())
        .map(|index| {
            if index == 8 {
                "completionItem/resolve"
            } else {
                HEAVY[usize::try_from(index).expect("index fits") % HEAVY.len()]
            }
        })
        .collect()
}

#[requires(message.is_object())]
#[ensures(true)]
fn reply_to_server_request(stdin: &mut ChildStdin, message: &Value) -> bool {
    let Some(id) = message.get("id") else {
        return false;
    };
    if message.get("method").and_then(Value::as_str).is_none() {
        return false;
    }
    write_message(
        stdin,
        &json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": null,
        }),
    );
    true
}

#[requires(true)]
#[ensures(ret.is_object())]
fn wait_for_id(
    messages: &std::sync::mpsc::Receiver<Value>,
    stdin: &mut ChildStdin,
    id: u64,
    deadline: Instant,
    server: &RunningServer,
) -> Value {
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        assert!(
            !remaining.is_zero(),
            "timed out waiting for LSP id {id}. stderr: {}",
            server.stderr_text(),
        );
        let message = messages.recv_timeout(remaining).unwrap_or_else(|_| {
            panic!(
                "timed out waiting for LSP id {id}. stderr: {}",
                server.stderr_text(),
            )
        });
        if reply_to_server_request(stdin, &message) {
            continue;
        }
        if message.get("method").is_some() {
            continue;
        }
        if message.get("id") == Some(&json!(id)) {
            return message;
        }
        panic!(
            "unexpected LSP response while waiting for {id}: {message}. stderr: {}",
            server.stderr_text(),
        );
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn stdio_server_answers_a_burst_larger_than_the_worker_pool() {
    let (server, stdin, messages) = RunningServer::spawn();
    let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
        drive_stdio_server(&server, stdin, messages);
    }));
    server.stop();
    if let Err(payload) = result {
        resume_unwind(payload);
    }
}

#[requires(true)]
#[ensures(true)]
fn drive_stdio_server(
    server: &RunningServer,
    mut stdin: ChildStdin,
    messages: std::sync::mpsc::Receiver<Value>,
) {
    let deadline = Instant::now() + RESPONSE_DEADLINE;
    write_message(
        &mut stdin,
        &request_message(1, "initialize", initialize_params()),
    );
    let initialized = wait_for_id(&messages, &mut stdin, 1, deadline, server);
    assert!(initialized.get("error").is_none(), "{initialized}");
    assert!(initialized["result"]["capabilities"]["diagnosticProvider"].is_object());
    write_message(&mut stdin, &notification_message("initialized", json!({})));

    let methods = burst_methods();
    let mut expected = HashSet::new();
    let mut buffer = Vec::new();
    push_message(
        &mut buffer,
        &notification_message(
            "textDocument/didOpen",
            json!({
                "textDocument": {
                    "uri": DOCUMENT_URI,
                    "languageId": "lojban",
                    "version": 1,
                    "text": DOCUMENT_TEXT,
                }
            }),
        ),
    );
    for (offset, method) in methods.iter().enumerate() {
        let id = 2 + u64::try_from(offset).expect("offset fits");
        expected.insert(id);
        push_message(
            &mut buffer,
            &request_message(id, method, params_for(method, DOCUMENT_URI)),
        );
    }
    // Id 2 is the first heavy request.
    // Id 10 is the cheap resolve request.
    let cancel_id = 2;
    let resolve_id = 2 + 8;
    push_message(
        &mut buffer,
        &notification_message("$/cancelRequest", json!({ "id": cancel_id })),
    );
    stdin.write_all(&buffer).expect("write the in-flight burst");
    stdin.flush().expect("flush the in-flight burst");

    let mut responses = HashMap::new();
    let mut arrival = Vec::new();
    while responses.len() < expected.len() {
        let remaining = deadline.saturating_duration_since(Instant::now());
        assert!(
            !remaining.is_zero(),
            "timed out with {} of {} burst responses. stderr: {}",
            responses.len(),
            expected.len(),
            server.stderr_text(),
        );
        let message = messages.recv_timeout(remaining).unwrap_or_else(|_| {
            panic!(
                "timed out with {} of {} burst responses. stderr: {}",
                responses.len(),
                expected.len(),
                server.stderr_text(),
            )
        });
        if reply_to_server_request(&mut stdin, &message) || message.get("method").is_some() {
            continue;
        }
        let id = message
            .get("id")
            .and_then(Value::as_u64)
            .unwrap_or_else(|| panic!("burst response has no numeric id: {message}"));
        assert!(
            expected.contains(&id),
            "unexpected burst id {id}: {message}"
        );
        assert!(
            responses.insert(id, message.clone()).is_none(),
            "duplicate response for {id}"
        );
        arrival.push(id);
        assert_response(&message, id, cancel_id, resolve_id);
    }
    assert_eq!(HashSet::from_iter(arrival.iter().copied()), expected);

    write_message(
        &mut stdin,
        &notification_message(
            "textDocument/didChange",
            json!({
                "textDocument": { "uri": DOCUMENT_URI, "version": 2 },
                "contentChanges": [{ "text": EDITED_TEXT }]
            }),
        ),
    );
    let edited_id = 2 + burst_len();
    write_message(
        &mut stdin,
        &request_message(
            edited_id,
            "textDocument/diagnostic",
            params_for("textDocument/diagnostic", DOCUMENT_URI),
        ),
    );
    let edited = wait_for_id(&messages, &mut stdin, edited_id, deadline, server);
    assert!(edited.get("error").is_none(), "{edited}");
    let result_id = edited["result"]["resultId"]
        .as_str()
        .unwrap_or_else(|| panic!("edited diagnostic has no result id: {edited}"));
    let parts = result_id.split(':').collect::<Vec<_>>();
    assert_eq!(parts.len(), 4, "{result_id}");
    assert_eq!(parts[2], "2", "{result_id}");

    let shutdown_id = edited_id + 1;
    write_message(
        &mut stdin,
        &request_message(shutdown_id, "shutdown", Value::Null),
    );
    let shutdown = wait_for_id(&messages, &mut stdin, shutdown_id, deadline, server);
    assert_eq!(shutdown["result"], Value::Null, "{shutdown}");
    write_message(&mut stdin, &notification_message("exit", Value::Null));
    drop(stdin);

    let started = Instant::now();
    let status = loop {
        if let Some(status) = server.try_wait() {
            break status;
        }
        assert!(
            started.elapsed() < EXIT_DEADLINE,
            "LSP server did not exit. stderr: {}",
            server.stderr_text(),
        );
        thread::sleep(time::Duration::from_millis(20));
    };
    assert!(
        status.success(),
        "LSP exit status {status}. stderr: {}",
        server.stderr_text()
    );
    server.join_readers();
    assert!(
        server.stderr_text().is_empty(),
        "unexpected LSP stderr: {}",
        server.stderr_text()
    );
}

#[requires(true)]
#[ensures(ret >= 2)]
fn pinned_worker_pool_size() -> usize {
    // Linux pins the child to one CPU.
    // The server pool is then two.
    if cfg!(target_os = "linux") {
        return 2;
    }
    std::thread::available_parallelism()
        .map(NonZeroUsize::get)
        .unwrap_or(2)
        .max(2)
}

#[requires(true)]
#[ensures(ret.contains("mi klama lo zarci"))]
fn large_document_text() -> String {
    "mi klama lo zarci .i ".repeat(600)
}

#[requires(true)]
#[ensures(true)]
fn initialize_server(
    server: &RunningServer,
    stdin: &mut ChildStdin,
    messages: &std::sync::mpsc::Receiver<Value>,
    deadline: Instant,
) {
    write_message(
        stdin,
        &request_message(1, "initialize", initialize_params()),
    );
    let initialized = wait_for_id(messages, stdin, 1, deadline, server);
    assert!(initialized.get("error").is_none(), "{initialized}");
    write_message(stdin, &notification_message("initialized", json!({})));
}

#[requires(true)]
#[ensures(true)]
fn open_document(stdin: &mut ChildStdin, uri: &str, version: i32, text: &str) {
    write_message(
        stdin,
        &notification_message(
            "textDocument/didOpen",
            json!({
                "textDocument": {
                    "uri": uri,
                    "languageId": "lojban",
                    "version": version,
                    "text": text,
                }
            }),
        ),
    );
}

#[requires(true)]
#[ensures(true)]
fn shutdown_server(
    server: &RunningServer,
    stdin: &mut ChildStdin,
    messages: &std::sync::mpsc::Receiver<Value>,
    id: u64,
    deadline: Instant,
) {
    write_message(stdin, &request_message(id, "shutdown", Value::Null));
    let shutdown = wait_for_id(messages, stdin, id, deadline, server);
    assert_eq!(shutdown["result"], Value::Null, "{shutdown}");
    write_message(stdin, &notification_message("exit", Value::Null));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn confirmed_hover_returns_before_waits_on_a_new_document() {
    let (server, mut stdin, messages) = RunningServer::spawn();
    let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
        drive_confirmed_hover(&server, &mut stdin, &messages);
    }));
    server.stop();
    if let Err(payload) = result {
        resume_unwind(payload);
    }
}

#[requires(true)]
#[ensures(true)]
fn drive_confirmed_hover(
    server: &RunningServer,
    stdin: &mut ChildStdin,
    messages: &std::sync::mpsc::Receiver<Value>,
) {
    let deadline = Instant::now() + RESPONSE_DEADLINE;
    initialize_server(server, stdin, messages, deadline);
    let confirmed = "file:///confirmed.jbo";
    let fresh = "file:///fresh.jbo";
    open_document(stdin, confirmed, 1, DOCUMENT_TEXT);
    write_message(
        stdin,
        &request_message(
            2,
            "textDocument/foldingRange",
            params_for("textDocument/foldingRange", confirmed),
        ),
    );
    let confirmed_folding = wait_for_id(messages, stdin, 2, deadline, server);
    assert!(
        confirmed_folding.get("error").is_none(),
        "{confirmed_folding}"
    );

    let pool = pinned_worker_pool_size();
    let hover_id = 50_u64;
    let mut buffer = Vec::new();
    push_message(
        &mut buffer,
        &notification_message(
            "textDocument/didOpen",
            json!({
                "textDocument": {
                    "uri": fresh,
                    "languageId": "lojban",
                    "version": 1,
                    "text": large_document_text(),
                }
            }),
        ),
    );
    let mut folding_ids = Vec::new();
    for offset in 0..pool {
        let id = 100 + u64::try_from(offset).expect("pool offset fits");
        folding_ids.push(id);
        push_message(
            &mut buffer,
            &request_message(
                id,
                "textDocument/foldingRange",
                params_for("textDocument/foldingRange", fresh),
            ),
        );
    }
    push_message(
        &mut buffer,
        &request_message(
            hover_id,
            "textDocument/hover",
            params_for("textDocument/hover", confirmed),
        ),
    );
    stdin
        .write_all(&buffer)
        .expect("write the waiting requests");
    stdin.flush().expect("flush the waiting requests");

    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        assert!(
            !remaining.is_zero(),
            "timed out waiting for the confirmed hover. stderr: {}",
            server.stderr_text(),
        );
        let message = messages.recv_timeout(remaining).unwrap_or_else(|_| {
            panic!(
                "timed out waiting for the confirmed hover. stderr: {}",
                server.stderr_text(),
            )
        });
        if reply_to_server_request(stdin, &message) || message.get("method").is_some() {
            continue;
        }
        let id = message
            .get("id")
            .and_then(Value::as_u64)
            .unwrap_or_else(|| panic!("response has no numeric id: {message}"));
        if folding_ids.contains(&id) {
            panic!(
                "foldingRange on the new document answered before hover: {message}. stderr: {}",
                server.stderr_text(),
            );
        }
        if id == hover_id {
            assert!(message.get("error").is_none(), "{message}");
            assert!(message.get("result").is_some(), "{message}");
            break;
        }
        panic!(
            "unexpected response before the confirmed hover: {message}. stderr: {}",
            server.stderr_text(),
        );
    }
    shutdown_server(server, stdin, messages, 900, deadline);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn cancel_of_a_waiting_folding_range_returns_cancelled() {
    let (server, mut stdin, messages) = RunningServer::spawn();
    let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
        drive_cancelled_folding_range(&server, &mut stdin, &messages);
    }));
    server.stop();
    if let Err(payload) = result {
        resume_unwind(payload);
    }
}

#[requires(true)]
#[ensures(true)]
fn drive_cancelled_folding_range(
    server: &RunningServer,
    stdin: &mut ChildStdin,
    messages: &std::sync::mpsc::Receiver<Value>,
) {
    let deadline = Instant::now() + RESPONSE_DEADLINE;
    initialize_server(server, stdin, messages, deadline);
    let fresh = "file:///cancelled.jbo";
    let mut buffer = Vec::new();
    push_message(
        &mut buffer,
        &notification_message(
            "textDocument/didOpen",
            json!({
                "textDocument": {
                    "uri": fresh,
                    "languageId": "lojban",
                    "version": 1,
                    "text": large_document_text(),
                }
            }),
        ),
    );
    push_message(
        &mut buffer,
        &request_message(
            2,
            "textDocument/foldingRange",
            params_for("textDocument/foldingRange", fresh),
        ),
    );
    push_message(
        &mut buffer,
        &notification_message("$/cancelRequest", json!({ "id": 2 })),
    );
    stdin
        .write_all(&buffer)
        .expect("write the cancelled folding range");
    stdin.flush().expect("flush the cancelled folding range");
    let response = wait_for_id(messages, stdin, 2, deadline, server);
    assert_eq!(
        response["error"]["code"].as_i64(),
        Some(-32800),
        "{response}. stderr: {}",
        server.stderr_text(),
    );
    assert!(response.get("result").is_none(), "{response}");
    shutdown_server(server, stdin, messages, 3, deadline);
}

#[requires(message.is_object())]
#[ensures(true)]
fn assert_response(message: &Value, id: u64, cancel_id: u64, resolve_id: u64) {
    if id == cancel_id {
        if let Some(error) = message.get("error") {
            assert_eq!(
                error["code"].as_i64(),
                Some(-32800),
                "cancel response: {message}"
            );
            return;
        }
    } else {
        assert!(
            message.get("error").is_none(),
            "request {id} failed: {message}"
        );
    }
    assert!(message.get("result").is_some(), "request {id}: {message}");
    if id == resolve_id {
        assert_eq!(message["result"]["label"], "mi", "{message}");
    }
}
