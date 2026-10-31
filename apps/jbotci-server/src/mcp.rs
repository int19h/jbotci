use crate::execution::{AdmittedWorkContext, ServerExecution};
use crate::policy::ServerRequest;
use std::future::Future;
use std::sync::Arc;

use crate::error::ServerErrorCode;
use axum::body::{Body, Bytes};
use axum::extract::Extension;
use axum::http::header::CONTENT_TYPE;
use axum::http::{Response, StatusCode};
use base64::display::Base64Display;
use bityzba::{data, invariant, new, requires};
use jbotci_cli::{
    GimfihiSourceWordKind, ToolCuktaRequest, ToolGentufaRequest, ToolGimfihiRequest,
    ToolJvozbaRequest, ToolRenderedOutput, ToolStatus, ToolVlackuRequest, ToolVlaseiRequest,
    run_tool_gentufa_with_limits, run_tool_gimfihi_with_limits, run_tool_vlasei_with_limits,
    tool_request_schema,
};
use serde::Deserialize;
use serde::ser::{SerializeMap, SerializeSeq};
use serde::{Serialize, Serializer};
use serde_json::{Value, json};

use crate::{AppState, ToolServices};

const MCP_PROTOCOL_VERSION: &str = "2025-06-18";
const SERVER_NAME: &str = "jbotci";
const SERVER_TITLE: &str = "jbotci";

/// The Lojban formal grammar, embedded so the server is self-contained and the
/// resource works on any transport without a reachable base URL.
const LOJBAN_GRAMMAR_EBNF: &str = include_str!("../resources/lojban-grammar.ebnf");
const LOJBAN_GRAMMAR_URI: &str = "jbotci:///grammar/lojban.ebnf";
const LOJBAN_GRAMMAR_NAME: &str = "lojban-grammar";
const LOJBAN_GRAMMAR_TITLE: &str = "Lojban EBNF grammar";
const LOJBAN_GRAMMAR_MIME: &str = "text/plain; charset=utf-8";
const LOJBAN_GRAMMAR_DESCRIPTION: &str = "The formal EBNF grammar of Lojban — the official machine grammar that `gentufa` implements, \
     prefixed with a guide to its non-standard notation (`&`, `...`, `//`, `#`). Read this to \
     understand or generate Lojban syntax.";

#[invariant(true)]
#[derive(Debug, Deserialize)]
struct JsonRpcMessage {
    jsonrpc: Option<String>,
    #[serde(default)]
    id: Option<Value>,
    method: Option<String>,
    #[serde(default)]
    params: Option<Value>,
}

// Deliberately NOT `deny_unknown_fields`: the MCP spec reserves a sibling
// `_meta` object inside `tools/call` params (e.g. `progressToken`, sent by the
// Claude Agent SDK and other clients) that a conformant server must tolerate,
// and the protocol may add further params fields over time. Rejecting them would
// break interop on every tool call. The tool *arguments* are still validated
// strictly by each tool's request type.
#[invariant(!name.trim().is_empty(), "MCP tool call name must be present")]
#[invariant(arguments.is_object(), "MCP tool call arguments must be an object")]
#[derive(Debug, Deserialize)]
struct ToolCallParams {
    name: String,
    #[serde(default = "empty_json_object")]
    arguments: Value,
}

#[requires(true)]
#[ensures(true)]
pub(crate) async fn mcp_get() -> Response<Body> {
    plain_response(
        StatusCode::METHOD_NOT_ALLOWED,
        "MCP Streamable HTTP SSE is not supported by this stateless endpoint.",
    )
}

#[requires(true)]
#[ensures(true)]
pub(crate) async fn mcp_post(
    Extension(state): Extension<Arc<AppState>>,
    Extension(deadline): Extension<crate::delivery::OperationDeadline>,
    body: Bytes,
) -> Response<Body> {
    let json_response = |status, value: Value| {
        crate::execution::json_response(status, &value, &state.tool_services.execution.policy, None)
    };
    let message = match serde_json::from_slice::<JsonRpcMessage>(&body) {
        Ok(message) => message,
        Err(error) => {
            return formatted_protocol_error(
                StatusCode::BAD_REQUEST,
                Value::Null,
                -32700,
                format_args!("Invalid JSON payload: {error}"),
                &state.tool_services.execution.policy,
            );
        }
    };
    let id = message.id.clone();
    if message.jsonrpc.as_deref() != Some("2.0") {
        return json_response(
            StatusCode::OK,
            json_rpc_error(id.unwrap_or(Value::Null), -32600, "`jsonrpc` must be `2.0`"),
        );
    }
    let Some(method) = message.method.as_deref() else {
        return json_response(
            StatusCode::OK,
            json_rpc_error(id.unwrap_or(Value::Null), -32600, "Missing JSON-RPC method"),
        );
    };
    if id.is_none() {
        return notification_response(method);
    }
    let id = id.unwrap_or(Value::Null);
    let result = match method {
        "initialize" => initialize_result(),
        "ping" => json!({}),
        "tools/list" => json!({ "tools": mcp_tools() }),
        "resources/list" => json!({ "resources": mcp_resources() }),
        "resources/templates/list" => json!({ "resourceTemplates": [] }),
        "resources/read" => {
            let uri = message
                .params
                .as_ref()
                .and_then(|params| params.get("uri"))
                .and_then(Value::as_str);
            match uri {
                Some(LOJBAN_GRAMMAR_URI) => grammar_resource_contents(),
                Some(other) => {
                    return formatted_protocol_error(
                        StatusCode::OK,
                        id,
                        -32602,
                        format_args!("Unknown resource URI: {other}"),
                        &state.tool_services.execution.policy,
                    );
                }
                None => {
                    return json_response(
                        StatusCode::OK,
                        json_rpc_error(id, -32602, "`resources/read` requires a `uri`"),
                    );
                }
            }
        }
        "tools/call" => {
            let params = match message.params {
                Some(params) => params,
                None => {
                    return json_response(
                        StatusCode::OK,
                        json_rpc_error(id, -32602, "`tools/call` requires params"),
                    );
                }
            };
            match serde_json::from_value::<ToolCallParams>(params) {
                Ok(params) => {
                    if !tool_error_envelope_fits(&id, &state.tool_services.execution.policy) {
                        return crate::execution::work_error_response(
                            crate::work::WorkError::OutputLimit,
                        );
                    }
                    let result =
                        call_tool_with_deadline(params, state.tool_services(), Some(deadline.0))
                            .await;
                    return admitted_tool_json_response(
                        id,
                        result,
                        &state.tool_services().execution.policy,
                    )
                    .await;
                }
                Err(error) => {
                    return formatted_protocol_error(
                        StatusCode::OK,
                        id,
                        -32602,
                        format_args!("Invalid tool call params: {error}"),
                        &state.tool_services.execution.policy,
                    );
                }
            }
        }
        other => {
            return formatted_protocol_error(
                StatusCode::OK,
                id,
                -32601,
                format_args!("Unsupported MCP method: {other}"),
                &state.tool_services.execution.policy,
            );
        }
    };
    json_response(
        StatusCode::OK,
        json!({ "jsonrpc": "2.0", "id": id, "result": result }),
    )
}

#[requires(true)]
#[ensures(true)]
fn notification_response(_method: &str) -> Response<Body> {
    // JSON-RPC notifications have no response body; acknowledge supported and
    // ignored notifications alike.
    Response::builder()
        .status(StatusCode::ACCEPTED)
        .body(Body::empty())
        .expect("MCP notification response builder is valid")
}

#[requires(true)]
#[ensures(true)]
fn initialize_result() -> Value {
    json!({
        "protocolVersion": MCP_PROTOCOL_VERSION,
        "capabilities": {
            "tools": {
                "listChanged": false
            },
            "resources": {
                "subscribe": false,
                "listChanged": false
            }
        },
        "serverInfo": {
            "name": SERVER_NAME,
            "title": SERVER_TITLE,
            "version": env!("CARGO_PKG_VERSION")
        },
        "instructions": "jbotci is a Lojban toolkit. Choose a tool by task: `cukta` for the reference grammar (CLL), `vlacku` for dictionary word lookups, `gentufa` to parse a sentence's grammar, `vlasei` for word-level morphology, `jvozba` to build a compound word, `gimfihi` to invent a new root word. Tools default to a readable text (or image) format."
    })
}

#[requires(true)]
#[ensures(ret.is_object())]
fn empty_json_object() -> Value {
    json!({})
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn mcp_tools() -> Vec<Value> {
    // The `cukta` description names the edition this build actually answers
    // from, so a caller can tell which revision of the grammar it is reading.
    let edition = jbotci_cll::cll_edition();
    vec![
        tool_definition(
            "cukta",
            "Lojban reference book (CLL)",
            &format!(
                "Read or search the Lojban reference grammar (CLL). Use it for grammar rules, \
                 explanations of how constructs work, and worked examples — not for plain word \
                 definitions (use `vlacku` for those). Answers come from *{}*, edition `{}` — {}; \
                 it teaches the amendments made since the first edition and labels each of them \
                 with its status. Defaults to semantic search; output is readable Markdown.",
                edition.title, edition.version, edition.publisher,
            ),
            tool_request_schema::<ToolCuktaRequest>(),
        ),
        tool_definition(
            "vlacku",
            "Lojban dictionary lookup",
            "Look up words in the Lojban dictionary (jbovlaste): definitions, place structure, \
             rafsi, glosses, and notes. Use it for \"what does this word mean\" or \"what's the word \
             for X\" — not for grammar rules (use `cukta`). Returns readable cards.",
            tool_request_schema::<ToolVlackuRequest>(),
        ),
        tool_definition(
            "jvozba",
            "Build a compound word",
            "Assemble a lujvo (compound word) or cmevla (name) from source words and/or fixed rafsi. \
             This is the *construction* tool; to take an existing lujvo apart, use `vlacku` with \
             `mode: lujvo`.",
            tool_request_schema::<ToolJvozbaRequest>(),
        ),
        tool_definition(
            "vlasei",
            "Lojban morphology",
            "Split Lojban text into words and classify each one (gismu, cmavo, lujvo, cmevla, \
             fu'ivla, …). This is word-level analysis — for the grammar of a whole sentence use \
             `gentufa`. Recoverable errors return marked partial output plus diagnostics. Defaults \
             to a readable tree.",
            tool_request_schema::<ToolVlaseiRequest>(),
        ),
        tool_definition(
            "gentufa",
            "Parse Lojban grammar",
            "Parse Lojban text into its grammar (syntax) tree — the authoritative way to see how a \
             sentence is structured and which word fills each role. For word-level analysis only use \
             `vlasei`. Recoverable errors return a marked partial tree plus diagnostics. Defaults \
             to a readable tree with place references.",
            tool_request_schema::<ToolGentufaRequest>(),
        ),
        tool_definition(
            "gimfihi",
            "Invent candidate gismu",
            "Propose new candidate gismu (root words) from source-language words using the standard \
             gismu-creation algorithm. This *creates* roots; it does not look up existing words (use \
             `vlacku`). Returns a ranked table.",
            tool_request_schema::<ToolGimfihiRequest>(),
        ),
    ]
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn mcp_resources() -> Vec<Value> {
    vec![json!({
        "uri": LOJBAN_GRAMMAR_URI,
        "name": LOJBAN_GRAMMAR_NAME,
        "title": LOJBAN_GRAMMAR_TITLE,
        "description": LOJBAN_GRAMMAR_DESCRIPTION,
        "mimeType": LOJBAN_GRAMMAR_MIME,
    })]
}

#[requires(true)]
#[ensures(ret.is_object())]
fn grammar_resource_contents() -> Value {
    json!({
        "contents": [{
            "uri": LOJBAN_GRAMMAR_URI,
            "name": LOJBAN_GRAMMAR_NAME,
            "title": LOJBAN_GRAMMAR_TITLE,
            "mimeType": LOJBAN_GRAMMAR_MIME,
            "text": LOJBAN_GRAMMAR_EBNF,
        }]
    })
}

#[requires(!name.trim().is_empty())]
#[requires(!title.trim().is_empty())]
#[requires(!description.trim().is_empty())]
#[ensures(true)]
fn tool_definition(name: &str, title: &str, description: &str, input_schema: Value) -> Value {
    json!({
        "name": name,
        "title": title,
        "description": description,
        "inputSchema": input_schema,
        "annotations": {
            "title": title,
            "readOnlyHint": true,
            "destructiveHint": false,
            "idempotentHint": true,
            "openWorldHint": false
        }
    })
}

#[invariant(::Output(_) => true)]
#[invariant(::Error { .. } => true)]
enum ToolPayload {
    Output(ToolRenderedOutput),
    Error {
        message: String,
        code: Option<ServerErrorCode>,
    },
}

#[invariant(true)]
struct ToolResult {
    payload: ToolPayload,
    context: Option<AdmittedWorkContext>,
}

impl ToolResult {
    #[requires(true)]
    #[ensures(true)]
    fn error(message: String) -> Self {
        Self {
            payload: ToolPayload::Error {
                message,
                code: None,
            },
            context: None,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn error_bounded(arguments: std::fmt::Arguments<'_>, policy: &crate::ServerPolicy) -> Self {
        let limit =
            jbotci_output::budget::ByteLimit::new(policy.text_bytes).expect("validated text limit");
        match jbotci_output::budget::bounded_format(arguments, limit) {
            Ok(message) => Self::error(message),
            Err(_) => Self::work_error(crate::work::WorkError::OutputLimit),
        }
    }
    #[requires(true)]
    #[ensures(true)]
    fn work_error(error: crate::work::WorkError) -> Self {
        Self {
            payload: ToolPayload::Error {
                message: error.to_string(),
                code: Some(crate::execution::work_error_code(&error)),
            },
            context: None,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn failed(error: anyhow::Error, policy: &crate::ServerPolicy) -> Self {
        if let Some(error) = error.downcast_ref::<crate::work::WorkError>() {
            return Self::work_error(error.clone());
        }
        if let Some(error) = error.downcast_ref::<crate::SemanticSearchError>()
            && let bityzba::data!(crate::SemanticSearchError::Lane(error)) = error.as_data()
        {
            return Self::work_error(error.clone());
        }
        if let Some(code) = crate::execution::operation_error_code(&error) {
            return Self {
                payload: ToolPayload::Error {
                    message: crate::execution::operation_error_message(code).to_owned(),
                    code: Some(code),
                },
                context: None,
            };
        }
        Self::error_bounded(format_args!("{error}"), policy)
    }
}

#[invariant(true)]
struct ToolContent<'a>(&'a ToolPayload);

impl Serialize for ToolPayload {
    #[requires(true)]
    #[ensures(true)]
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut result = serializer.serialize_map(None)?;
        result.serialize_entry("content", &ToolContent(self))?;
        match self {
            Self::Error { message, code } => {
                result.serialize_entry("isError", &true)?;
                if let Some(code) = code {
                    result.serialize_entry(
                        "structuredContent",
                        &StructuredToolError {
                            error: ToolErrorDetail {
                                code: code.as_str(),
                                message,
                            },
                        },
                    )?;
                }
            }
            Self::Output(output)
                if matches!(
                    output.status,
                    ToolStatus::Failure | ToolStatus::InvalidInput
                ) =>
            {
                result.serialize_entry("isError", &true)?
            }
            _ => {}
        }
        result.end()
    }
}

#[invariant(::Text { .. } => true)]
#[invariant(::Image { mime_type, .. } => mime_type.starts_with("image/") && *mime_type != "image/svg+xml")]
#[derive(Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum ContentItem<'a> {
    Text {
        text: ToolText<'a>,
    },
    Image {
        #[serde(rename = "mimeType")]
        mime_type: &'a str,
        data: EncodedImage<'a>,
    },
}

#[invariant(true)]
struct EncodedImage<'a>(&'a [u8]);
impl Serialize for EncodedImage<'_> {
    #[requires(true)]
    #[ensures(true)]
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(&Base64Display::new(
            self.0,
            &base64::engine::general_purpose::STANDARD,
        ))
    }
}

#[invariant(::Plain(_) => true)]
#[invariant(::Failure(output) => matches!(output.status, ToolStatus::Failure | ToolStatus::InvalidInput))]
enum ToolText<'a> {
    Plain(&'a str),
    Failure(&'a ToolRenderedOutput),
}
impl Serialize for ToolText<'_> {
    #[requires(true)]
    #[ensures(true)]
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}
impl std::fmt::Display for ToolText<'_> {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.as_data() {
            data!(ToolText::Plain(text)) => formatter.write_str(text),
            data!(ToolText::Failure(output)) => {
                let stdout = output.stdout_text().unwrap_or("");
                if output.stderr.is_empty() && stdout.trim().is_empty() {
                    return write!(formatter, "tool failed with status {:?}", output.status);
                }
                formatter.write_str(&output.stderr)?;
                if !stdout.trim().is_empty() {
                    if !output.stderr.is_empty() && !output.stderr.ends_with('\n') {
                        formatter.write_str("\n")?;
                    }
                    formatter.write_str(stdout)?;
                }
                Ok(())
            }
        }
    }
}

impl Serialize for ToolContent<'_> {
    #[requires(true)]
    #[ensures(true)]
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut sequence = serializer.serialize_seq(None)?;
        match self.0 {
            ToolPayload::Error { message, .. } => {
                sequence.serialize_element(&new!(ContentItem::Text {
                    text: new!(ToolText::Plain(message)),
                }))?
            }
            ToolPayload::Output(output)
                if matches!(
                    output.status,
                    ToolStatus::Failure | ToolStatus::InvalidInput
                ) =>
            {
                sequence.serialize_element(&new!(ContentItem::Text {
                    text: new!(ToolText::Failure(output)),
                }))?
            }
            ToolPayload::Output(output) => {
                if !output.stderr.is_empty() {
                    sequence.serialize_element(&new!(ContentItem::Text {
                        text: new!(ToolText::Plain(&output.stderr)),
                    }))?;
                }
                if content_type_is_raster_image(output.content_type.as_deref()) {
                    sequence.serialize_element(&new!(ContentItem::Image {
                        mime_type: output
                            .content_type
                            .as_deref()
                            .and_then(|content_type| content_type.split(';').next())
                            .map(str::trim)
                            .unwrap_or("application/octet-stream"),
                        data: EncodedImage(&output.stdout),
                    }))?;
                } else {
                    let text = output.stdout_text().map_err(serde::ser::Error::custom)?;
                    sequence.serialize_element(&new!(ContentItem::Text {
                        text: new!(ToolText::Plain(text)),
                    }))?;
                }
            }
        }
        sequence.end()
    }
}

#[invariant(true)]
#[derive(Serialize)]
struct ToolEnvelope<'a> {
    jsonrpc: &'static str,
    id: &'a Value,
    result: &'a ToolPayload,
}

#[requires(true)]
#[ensures(true)]
fn tool_json_response(
    id: Value,
    result: ToolResult,
    policy: &crate::ServerPolicy,
) -> Response<Body> {
    let limit = jbotci_output::budget::ByteLimit::new(policy.response_bytes)
        .expect("validated response limit");
    let mut error_code = match &result.payload {
        ToolPayload::Error { code, .. } => *code,
        _ => None,
    };
    let encoded = jbotci_output::budget::bounded_json(
        &ToolEnvelope {
            jsonrpc: "2.0",
            id: &id,
            result: &result.payload,
        },
        limit,
    );
    let bytes = match encoded {
        Ok(bytes) => bytes,
        Err(error) => {
            let code = if error.io_error_kind() == Some(std::io::ErrorKind::FileTooLarge) {
                ServerErrorCode::OutputLimit
            } else {
                eprintln!("MCP response encoding failed: {error}");
                ServerErrorCode::Failed
            };
            error_code = Some(code);
            let payload = ToolPayload::Error {
                message: canonical_tool_error(code).to_owned(),
                code: Some(code),
            };
            match jbotci_output::budget::bounded_json(
                &ToolEnvelope {
                    jsonrpc: "2.0",
                    id: &id,
                    result: &payload,
                },
                limit,
            ) {
                Ok(bytes) => bytes,
                Err(_) => {
                    return crate::execution::work_error_response(
                        crate::work::WorkError::OutputLimit,
                    );
                }
            }
        }
    };
    let mut response = Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, "application/json")
        .body(crate::execution::owned_body(bytes, result.context))
        .expect("MCP response");
    if let Some(code) = error_code {
        response.extensions_mut().insert(code);
    }
    response
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn canonical_tool_error(code: ServerErrorCode) -> &'static str {
    code.message()
}

/// Reject an identifier that leaves no room for a canonical error envelope.
#[requires(true)]
#[ensures(true)]
fn tool_error_envelope_fits(id: &Value, policy: &crate::ServerPolicy) -> bool {
    let limit = jbotci_output::budget::ByteLimit::new(policy.response_bytes)
        .expect("validated response limit");
    ServerErrorCode::ALL.into_iter().all(|code| {
        let payload = ToolPayload::Error {
            message: canonical_tool_error(code).to_owned(),
            code: Some(code),
        };
        jbotci_output::budget::bounded_json_size(
            &ToolEnvelope {
                jsonrpc: "2.0",
                id,
                result: &payload,
            },
            limit,
        )
        .is_ok()
    })
}

#[cfg(test)]
#[requires(!params.name.trim().is_empty())]
#[ensures(true)]
async fn call_tool(params: ToolCallParams, tool_services: ToolServices) -> ToolResult {
    call_tool_with_deadline(params, tool_services, None).await
}

#[requires(!params.name.trim().is_empty())]
#[ensures(true)]
async fn call_tool_with_deadline(
    params: ToolCallParams,
    tool_services: ToolServices,
    deadline: Option<tokio::time::Instant>,
) -> ToolResult {
    let execution = Arc::clone(&tool_services.execution);
    let limits = execution.policy.tool_limits();
    let params = params.into_data();
    let name = params.name;
    let arguments = params.arguments;
    match name.as_str() {
        "cukta" => {
            call_async_typed_tool(
                arguments,
                execution,
                deadline,
                move |request, context| async move {
                    tool_services.run_cukta_admitted(request, &context).await
                },
            )
            .await
        }
        "vlacku" => {
            call_async_typed_tool(
                arguments,
                execution,
                deadline,
                move |request, context| async move {
                    tool_services.run_vlacku_admitted(request, &context).await
                },
            )
            .await
        }
        "jvozba" => {
            call_typed_tool(arguments, execution, deadline, move |request| {
                jbotci_cli::run_tool_jvozba_with_limits(request, &limits)
            })
            .await
        }
        "vlasei" => {
            call_typed_tool(arguments, execution, deadline, move |request| {
                run_tool_vlasei_with_limits(request, &limits)
            })
            .await
        }
        "gentufa" => {
            call_typed_tool(arguments, execution, deadline, move |request| {
                run_tool_gentufa_with_limits(request, &limits)
            })
            .await
        }
        "gimfihi" => {
            call_typed_tool(arguments, execution, deadline, move |request| {
                run_tool_gimfihi_with_limits(request, GimfihiSourceWordKind::Ipa, &limits)
            })
            .await
        }
        _ => ToolResult::error_bounded(format_args!("Unknown tool: {name}"), &execution.policy),
    }
}

#[requires(true)]
#[ensures(true)]
async fn call_async_typed_tool<T, F, Fut>(
    arguments: Value,
    execution: Arc<ServerExecution>,
    deadline: Option<tokio::time::Instant>,
    runner: F,
) -> ToolResult
where
    T: for<'de> Deserialize<'de> + ServerRequest + Send + 'static,
    F: FnOnce(T, AdmittedWorkContext) -> Fut + Send + 'static,
    Fut: Future<Output = anyhow::Result<ToolRenderedOutput>> + Send + 'static,
{
    let request = match serde_json::from_value::<T>(arguments) {
        Ok(request) => request,
        Err(error) => {
            return ToolResult::error_bounded(
                format_args!("Invalid tool arguments: {error}"),
                &execution.policy,
            );
        }
    };
    if let Err(error) = request.validate(&execution.policy) {
        return ToolResult::failed(anyhow::Error::new(error), &execution.policy);
    }
    let context = match execution.admit(deadline).await {
        Ok(context) => context,
        Err(error) => return ToolResult::work_error(error),
    };
    let payload = match runner(request, context.clone()).await {
        Ok(output) => ToolPayload::Output(output),
        Err(error) => {
            let mut result = ToolResult::failed(error, &execution.policy);
            result.context = Some(context);
            return result;
        }
    };
    ToolResult {
        payload,
        context: Some(context),
    }
}

#[requires(true)]
#[ensures(true)]
async fn call_typed_tool<T, F>(
    arguments: Value,
    execution: Arc<ServerExecution>,
    deadline: Option<tokio::time::Instant>,
    runner: F,
) -> ToolResult
where
    T: for<'de> Deserialize<'de> + ServerRequest + Send + 'static,
    F: FnOnce(T) -> anyhow::Result<ToolRenderedOutput> + Send + 'static,
{
    let request = match serde_json::from_value::<T>(arguments) {
        Ok(request) => request,
        Err(error) => {
            return ToolResult::error_bounded(
                format_args!("Invalid tool arguments: {error}"),
                &execution.policy,
            );
        }
    };
    if let Err(error) = request.validate(&execution.policy) {
        return ToolResult::failed(anyhow::Error::new(error), &execution.policy);
    }
    let context = match execution.admit(deadline).await {
        Ok(context) => context,
        Err(error) => return ToolResult::work_error(error),
    };
    let payload = match context.run(move || runner(request)).await {
        Ok(Ok(output)) => ToolPayload::Output(output),
        Ok(Err(error)) => {
            let mut result = ToolResult::failed(error, &execution.policy);
            result.context = Some(context);
            return result;
        }
        Err(error) => {
            let mut result = ToolResult::work_error(error);
            result.context = Some(context);
            return result;
        }
    };
    ToolResult {
        payload,
        context: Some(context),
    }
}

#[cfg(test)]
#[requires(true)]
#[ensures(true)]
fn tool_output_result(output: ToolRenderedOutput) -> Value {
    serde_json::to_value(ToolPayload::Output(output)).expect("tool result serializes")
}

#[requires(true)]
#[ensures(true)]
fn content_type_is_raster_image(content_type: Option<&str>) -> bool {
    content_type
        .and_then(|value| value.split(';').next())
        .map(str::trim)
        // SVG is XML text, not a raster image, so it is excluded here and served
        // as text instead (the harnesses we target cannot render SVG images).
        .is_some_and(|value| value.starts_with("image/") && value != "image/svg+xml")
}

#[requires(true)]
#[ensures(true)]
fn formatted_protocol_error(
    status: StatusCode,
    id: Value,
    code: i64,
    message: std::fmt::Arguments<'_>,
    policy: &crate::ServerPolicy,
) -> Response<Body> {
    let limit = jbotci_output::budget::ByteLimit::new(policy.text_bytes.min(policy.response_bytes))
        .expect("validated output limits");
    match jbotci_output::budget::bounded_format(message, limit) {
        Ok(message) => crate::execution::json_response(
            status,
            &json_rpc_error(id, code, &message),
            policy,
            None,
        ),
        Err(_) => crate::execution::work_error_response(crate::work::WorkError::OutputLimit),
    }
}

#[requires(true)]
#[ensures(true)]
fn json_rpc_error(id: Value, code: i64, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": code,
            "message": message
        }
    })
}

#[requires(true)]
#[ensures(true)]
fn plain_response(status: StatusCode, text: &str) -> Response<Body> {
    Response::builder()
        .status(status)
        .header(CONTENT_TYPE, "text/plain; charset=utf-8")
        .body(Body::from(text.to_owned()))
        .expect("MCP plain response builder is valid")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn raster_encoding_and_text_escaping_obey_the_wire_byte_limit() {
        use jbotci_output::budget::{ByteLimit, bounded_json};
        let payload = ToolPayload::Output(ToolRenderedOutput {
            status: ToolStatus::Success,
            stdout: vec![0, 1, 2],
            stderr: String::new(),
            content_type: Some("image/png".into()),
        });
        let expected = br#"{"content":[{"type":"image","mimeType":"image/png","data":"AAEC"}]}"#;
        assert_eq!(
            bounded_json(&payload, ByteLimit::new(expected.len()).unwrap()).unwrap(),
            expected
        );
        assert!(bounded_json(&payload, ByteLimit::new(expected.len() - 1).unwrap()).is_err());
        let payload = ToolPayload::Output(ToolRenderedOutput {
            status: ToolStatus::Success,
            stdout: b"\n".to_vec(),
            stderr: String::new(),
            content_type: Some("text/plain".into()),
        });
        let expected = br#"{"content":[{"type":"text","text":"\n"}]}"#;
        assert_eq!(
            bounded_json(&payload, ByteLimit::new(expected.len()).unwrap()).unwrap(),
            expected
        );
        assert!(bounded_json(&payload, ByteLimit::new(expected.len() - 1).unwrap()).is_err());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn failure_text_preserves_diagnostics_and_recovered_output() {
        let output = ToolRenderedOutput {
            status: ToolStatus::Failure,
            stdout: "([mi ‼ku‼] [.i do])\n".as_bytes().to_vec(),
            stderr: "error[syntax.unexpected-cmavo]: unexpected cmavo\n".to_owned(),
            content_type: Some("text/plain; charset=utf-8".to_owned()),
        };

        let result = tool_output_result(output);

        assert_eq!(result["isError"], true);
        assert_eq!(
            result["content"][0]["text"],
            "error[syntax.unexpected-cmavo]: unexpected cmavo\n([mi ‼ku‼] [.i do])\n"
        );
    }
    #[tokio::test]
    #[requires(true)]
    #[ensures(true)]
    async fn mcp_and_discord_use_the_same_compound_limits_in_both_modes() {
        use crate::discord::{
            operations::{OperationContext, ToolOutcome, run_request},
            request::{DiscordRequest, JvozbaOptions, JvozbaRequest, JvozbaTarget, SourceText},
        };
        use jbotci_jvozba::{JvozbaError, JvozbaWorkMeasure};
        let defaults = crate::ServerPolicy::default();
        for (jvozba, measure) in [
            (
                defaults.jvozba.with_data(data! { max_pieces: 2 }),
                JvozbaWorkMeasure::Pieces,
            ),
            (
                defaults.jvozba.with_data(data! { max_placements: 1 }),
                JvozbaWorkMeasure::Placements,
            ),
            (
                defaults.jvozba.with_data(data! { max_spelling_letters: 1 }),
                JvozbaWorkMeasure::SpellingLetters,
            ),
        ] {
            let execution =
                ServerExecution::new(defaults.clone().with_data(data! { jvozba: jvozba }));
            let tools = ToolServices::with_worker_and_execution(execution.clone(), || {
                |_| panic!("compound builds cannot use the embedding worker")
            });
            for (mode, target) in [
                ("lujvo", JvozbaTarget::Lujvo),
                ("cmevla", JvozbaTarget::Cmevla),
            ] {
                let result = call_tool(new!(ToolCallParams { name: "jvozba".to_owned(), arguments: json!({ "mode": mode, "parts": [{ "kind": "word", "value": "jbobau" }, { "kind": "word", "value": "klama" }] }) }), tools.clone()).await;
                assert!(
                    matches!(
                        &result.payload,
                        ToolPayload::Error {
                            code: Some(ServerErrorCode::OperationLimit),
                            ..
                        }
                    ),
                    "{mode} did not use {measure:?}"
                );
                drop(result);
                let completed = run_request(
                    DiscordRequest::Jvozba(JvozbaRequest {
                        parts: SourceText::new("jbobau klama").unwrap(),
                        options: JvozbaOptions { target },
                    }),
                    OperationContext {
                        tools: &tools,
                        deadline: tokio::time::Instant::now() + std::time::Duration::from_secs(20),
                        keepalive: None,
                        attachment_size_limit: None,
                    },
                )
                .await
                .unwrap();
                let Ok(ToolOutcome::Jvozba(outcome)) = &completed.outcome else {
                    panic!("compound outcome");
                };
                assert!(
                    matches!(&outcome.result, Err(JvozbaError::TooMuchWork { measure: actual, .. }) if *actual == measure),
                    "{mode}: {:?}",
                    outcome.result
                );
                drop(completed);
                assert_eq!(execution.governor.snapshot().compute_active, 0);
            }
        }
    }

    #[tokio::test]
    #[requires(true)]
    #[ensures(true)]
    async fn encoded_output_overflow_preserves_the_mcp_error_envelope() {
        let policy = crate::ServerPolicy::default().with_data(data! { response_bytes: 512 });
        let id = json!("byte-boundary");
        assert!(tool_error_envelope_fits(&id, &policy));
        assert!(!tool_error_envelope_fits(&json!("x".repeat(1024)), &policy));
        let response = tool_json_response(
            id.clone(),
            ToolResult {
                payload: ToolPayload::Output(ToolRenderedOutput {
                    status: ToolStatus::Success,
                    stdout: vec![b'x'; 1024],
                    stderr: String::new(),
                    content_type: Some("text/plain".to_owned()),
                }),
                context: None,
            },
            &policy,
        );
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), 512)
            .await
            .unwrap();
        let value: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(value["id"], id);
        assert_eq!(value["result"]["isError"], true);
        assert_eq!(
            value["result"]["structuredContent"]["error"]["code"],
            "output_limit_exceeded"
        );
    }
}

#[invariant(true)]
#[derive(Serialize)]
struct StructuredToolError<'a> {
    error: ToolErrorDetail<'a>,
}
#[invariant(true)]
#[derive(Serialize)]
struct ToolErrorDetail<'a> {
    code: &'a str,
    message: &'a str,
}

#[requires(true)]
#[ensures(true)]
async fn admitted_tool_json_response(
    id: Value,
    result: ToolResult,
    policy: &crate::ServerPolicy,
) -> Response<Body> {
    let Some(context) = result.context.clone() else {
        return tool_json_response(id, result, policy);
    };
    let refusal = matches!(
        &result.payload,
        ToolPayload::Error {
            code: Some(ServerErrorCode::Busy | ServerErrorCode::Deadline),
            ..
        }
    );
    if refusal {
        return coded_error_json_response(id, result, context).await;
    }
    let policy = context.execution.policy.clone();
    let error_id = id.clone();
    match context
        .run(move || tool_json_response(id, result, &policy))
        .await
    {
        Ok(response) => response,
        Err(error) => {
            let mut refusal = ToolResult::work_error(error);
            refusal.context = Some(context.clone());
            coded_error_json_response(error_id, refusal, context).await
        }
    }
}

/// Encode only a canonical error. This never runs a tool or accepts its output.
#[requires(matches!(&result.payload, ToolPayload::Error { code: Some(_), .. }))]
#[ensures(true)]
async fn coded_error_json_response(
    id: Value,
    result: ToolResult,
    context: crate::execution::AdmittedWorkContext,
) -> Response<Body> {
    let policy = context.execution.policy.clone();
    let deadline = if matches!(
        &result.payload,
        ToolPayload::Error {
            code: Some(ServerErrorCode::Busy | ServerErrorCode::Deadline),
            ..
        }
    ) {
        context
            .deadline
            .checked_add(policy.error_delivery)
            .unwrap_or(context.deadline)
    } else {
        context.deadline
    };
    match crate::work::run_admitted(context.lease.clone(), deadline, move || {
        tool_json_response(id, result, &policy)
    })
    .await
    {
        Ok(response) => response,
        Err(error) => crate::execution::work_error_response(error),
    }
}

#[cfg(test)]
mod budget_integration_tests {
    use super::*;

    #[requires(true)]
    #[ensures(true)]
    fn services(policy: crate::ServerPolicy) -> ToolServices {
        ToolServices::with_worker_and_execution(ServerExecution::new(policy), || {
            |_| panic!("these requests cannot use native embedding search")
        })
    }
    #[requires(true)]
    #[ensures(true)]
    fn params(name: &str, arguments: Value) -> ToolCallParams {
        new!(ToolCallParams {
            name: name.to_owned(),
            arguments
        })
    }

    #[tokio::test]
    #[requires(true)]
    #[ensures(true)]
    async fn mcp_admission_keeps_the_deadline_from_the_http_boundary() {
        let tools = services(crate::ServerPolicy::default());
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(10);
        let result = call_tool_with_deadline(
            params("vlasei", json!({ "text": "klama", "format": "ipa" })),
            tools.clone(),
            Some(deadline),
        )
        .await;
        assert!(matches!(result.payload, ToolPayload::Output(_)));
        assert_eq!(result.context.unwrap().deadline, deadline);
        let expired = call_tool_with_deadline(
            params("vlasei", json!({ "text": "klama", "format": "ipa" })),
            tools,
            Some(tokio::time::Instant::now()),
        )
        .await;
        assert!(matches!(
            expired.payload,
            ToolPayload::Error {
                code: Some(ServerErrorCode::Busy),
                ..
            }
        ));
        assert!(expired.context.is_none());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[requires(true)]
    #[ensures(true)]
    async fn an_expired_tool_result_becomes_a_coded_timeout_without_extending_the_operation() {
        let execution = ServerExecution::new(crate::ServerPolicy::default());
        let context = execution
            .admit(None)
            .await
            .unwrap()
            .with_data(data! { deadline: tokio::time::Instant::now() });
        let original_deadline = context.deadline;
        let response = admitted_tool_json_response(
            json!(7),
            ToolResult {
                payload: ToolPayload::Output(ToolRenderedOutput {
                    status: ToolStatus::Success,
                    stdout: b"LATE NORMAL RESULT".to_vec(),
                    stderr: String::new(),
                    content_type: Some("text/plain".to_owned()),
                }),
                context: Some(context.clone()),
            },
            &execution.policy,
        )
        .await;
        assert_eq!(context.deadline, original_deadline);
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.extensions().get::<ServerErrorCode>(),
            Some(&ServerErrorCode::Deadline)
        );
        let bytes = axum::body::to_bytes(response.into_body(), 4096)
            .await
            .unwrap();
        let value: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(value["id"], 7);
        assert_eq!(value["result"]["isError"], true);
        assert_eq!(
            value["result"]["structuredContent"]["error"]["code"],
            "work_deadline_exceeded"
        );
        assert!(
            !String::from_utf8(bytes.to_vec())
                .unwrap()
                .contains("LATE NORMAL RESULT")
        );
    }

    #[tokio::test]
    #[requires(true)]
    #[ensures(true)]
    async fn a_tool_shares_one_text_budget_between_output_and_diagnostics() {
        let arguments = json!({ "text": "mi ku .i do", "format": "brackets" });
        let default_tools = services(crate::ServerPolicy::default());
        let baseline = call_tool(params("gentufa", arguments.clone()), default_tools.clone()).await;
        let ToolPayload::Output(output) = baseline.payload else {
            panic!("baseline tool output");
        };
        assert!(!output.stdout.is_empty());
        assert!(!output.stderr.is_empty());
        let exact = output.stdout.len() + output.stderr.len();
        for bytes in [exact, exact - 1] {
            let policy = crate::ServerPolicy::default().with_data(data! { text_bytes: bytes });
            let tools = services(policy);
            let result = call_tool(params("gentufa", arguments.clone()), tools.clone()).await;
            if bytes == exact {
                let ToolPayload::Output(actual) = result.payload else {
                    panic!("exact budget must fit");
                };
                assert_eq!(actual.stdout, output.stdout);
                assert_eq!(actual.stderr, output.stderr);
            } else {
                assert!(matches!(
                    result.payload,
                    ToolPayload::Error {
                        code: Some(ServerErrorCode::OutputLimit),
                        ..
                    }
                ));
            }
        }
    }

    #[tokio::test]
    #[requires(true)]
    #[ensures(true)]
    async fn book_formats_use_the_server_text_limit() {
        for format in ["markdown", "html", "raw"] {
            let policy = crate::ServerPolicy::default().with_data(data! { text_bytes: 1 });
            let tools = services(policy);
            let result = call_tool(
                params(
                    "cukta",
                    json!({ "mode": "section", "query": "5.7", "format": format }),
                ),
                tools.clone(),
            )
            .await;
            assert!(
                matches!(
                    result.payload,
                    ToolPayload::Error {
                        code: Some(ServerErrorCode::OutputLimit),
                        ..
                    }
                ),
                "{format} bypassed the text budget"
            );
        }
    }

    #[tokio::test]
    #[requires(true)]
    #[ensures(true)]
    async fn every_morphology_tool_format_uses_the_server_text_limit() {
        for format in ["json", "tree", "brackets", "ipa", "raw"] {
            let policy = crate::ServerPolicy::default().with_data(data! { text_bytes: 1 });
            let tools = services(policy);
            let mut arguments = json!({ "text": ".klama.", "format": format });
            if matches!(format, "json" | "tree") {
                arguments["indent"] = json!(8);
            }
            if format == "raw" {
                arguments["indent"] = json!(0);
            }
            let result = call_tool(params("vlasei", arguments), tools.clone()).await;
            assert!(
                matches!(
                    result.payload,
                    ToolPayload::Error {
                        code: Some(ServerErrorCode::OutputLimit),
                        ..
                    }
                ),
                "{format} bypassed the text budget"
            );
        }
    }
    #[tokio::test]
    #[requires(true)]
    #[ensures(true)]
    async fn both_image_tool_formats_use_each_shared_diagram_limit() {
        let defaults = crate::ServerPolicy::default();
        for diagram in [
            defaults.diagram.with_data(data! { max_blocks: 1 }),
            defaults.diagram.with_data(data! { max_columns: 1 }),
            defaults.diagram.with_data(data! { max_pixels: 1 }),
            defaults.diagram.with_data(data! { max_bytes: 1 }),
        ] {
            for format in ["png", "svg"] {
                let tools = services(defaults.clone().with_data(data! { diagram: diagram }));
                let result = call_tool(
                    params(
                        "gentufa",
                        json!({
                            "text": "lo nanmu cu klama", "format": format,
                        }),
                    ),
                    tools,
                )
                .await;
                assert!(
                    matches!(
                        result.payload,
                        ToolPayload::Error {
                            code: Some(ServerErrorCode::OutputLimit),
                            ..
                        }
                    ),
                    "{format} bypassed {diagram:?}"
                );
            }
        }
    }
    #[tokio::test]
    #[requires(true)]
    #[ensures(true)]
    async fn excess_explicit_compound_parts_fail_before_admission_with_the_shared_code() {
        let defaults = crate::ServerPolicy::default();
        let work = defaults.work.with_data(data! { compute_queue: 0 });
        let jvozba = defaults.jvozba.with_data(data! { max_pieces: 2 });
        let tools = services(defaults.with_data(data! { work: work, jvozba: jvozba }));
        let occupied = tools.execution.admit(None).await.unwrap();
        for mode in ["lujvo", "cmevla"] {
            let result = call_tool(
                params(
                    "jvozba",
                    json!({
                        "mode": mode,
                        "parts": [
                            {"kind": "word", "value": "klama"},
                            {"kind": "word", "value": "bajra"},
                            {"kind": "word", "value": "sutra"},
                        ],
                    }),
                ),
                tools.clone(),
            )
            .await;
            assert!(matches!(
                result.payload,
                ToolPayload::Error {
                    code: Some(ServerErrorCode::OperationLimit),
                    ..
                }
            ));
            assert!(result.context.is_none());
            assert_eq!(tools.execution.governor.snapshot().compute_waiting, 0);
        }
        drop(occupied);
    }
}
