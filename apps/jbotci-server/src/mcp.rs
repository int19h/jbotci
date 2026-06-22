use std::sync::Arc;

use axum::body::{Body, Bytes};
use axum::extract::Extension;
use axum::http::header::{CONTENT_TYPE, HOST, ORIGIN};
use axum::http::{HeaderMap, Response, StatusCode};
use base64::Engine;
use bityzba::{invariant, requires};
use jbotci_cli::{
    ToolCuktaRequest, ToolGentufaRequest, ToolGimfihiRequest, ToolJvozbaRequest,
    ToolRenderedOutput, ToolStatus, ToolTersmuRequest, ToolVlackuRequest, ToolVlaseiRequest,
    run_tool_gentufa, run_tool_gimfihi, run_tool_jvozba, run_tool_tersmu, run_tool_vlasei,
};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{AppState, ToolServices};

const MCP_PROTOCOL_VERSION: &str = "2025-06-18";
const SERVER_NAME: &str = "jbotci";
const SERVER_TITLE: &str = "jbotci";

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

#[invariant(!name.trim().is_empty(), "MCP tool call name must be present")]
#[invariant(arguments.is_object(), "MCP tool call arguments must be an object")]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
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
    headers: HeaderMap,
    body: Bytes,
) -> Response<Body> {
    if !origin_is_allowed(&headers) {
        return plain_response(StatusCode::FORBIDDEN, "invalid Origin for MCP request");
    }
    let message = match serde_json::from_slice::<JsonRpcMessage>(&body) {
        Ok(message) => message,
        Err(error) => {
            return json_response(
                StatusCode::BAD_REQUEST,
                json_rpc_error(
                    Value::Null,
                    -32700,
                    &format!("Invalid JSON payload: {error}"),
                ),
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
                Ok(params) => call_tool(params, state.tool_services()).await,
                Err(error) => {
                    return json_response(
                        StatusCode::OK,
                        json_rpc_error(id, -32602, &format!("Invalid tool call params: {error}")),
                    );
                }
            }
        }
        other => {
            return json_response(
                StatusCode::OK,
                json_rpc_error(id, -32601, &format!("Unsupported MCP method: {other}")),
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
fn notification_response(method: &str) -> Response<Body> {
    match method {
        "notifications/initialized" | "notifications/cancelled" => Response::builder()
            .status(StatusCode::ACCEPTED)
            .body(Body::empty())
            .expect("MCP notification response builder is valid"),
        _ => Response::builder()
            .status(StatusCode::ACCEPTED)
            .body(Body::empty())
            .expect("MCP ignored notification response builder is valid"),
    }
}

#[requires(true)]
#[ensures(true)]
fn initialize_result() -> Value {
    json!({
        "protocolVersion": MCP_PROTOCOL_VERSION,
        "capabilities": {
            "tools": {
                "listChanged": false
            }
        },
        "serverInfo": {
            "name": SERVER_NAME,
            "title": SERVER_TITLE,
            "version": env!("CARGO_PKG_VERSION")
        },
        "instructions": "Use the default non-JSON formats unless a programmatic JSON result is specifically needed."
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
    vec![
        tool_definition(
            "cukta",
            "Read or search the CLL. Defaults to Markdown; prefer default text output for token efficiency unless HTML/raw is needed.",
            tool_request_schema::<ToolCuktaRequest>(),
        ),
        tool_definition(
            "vlacku",
            "Search dictionary cards by word, rafsi, lujvo, meaning, sound, regex, or glob. Default CLI-style cards are usually more token efficient than JSON-like alternatives.",
            tool_request_schema::<ToolVlackuRequest>(),
        ),
        tool_definition(
            "jvozba",
            "Build a lujvo or cmevla from source words and fixed rafsi.",
            tool_request_schema::<ToolJvozbaRequest>(),
        ),
        tool_definition(
            "vlasei",
            "Run Lojban morphology. Defaults to format=tree with refs; use JSON only for programmatic consumers.",
            tool_request_schema::<ToolVlaseiRequest>(),
        ),
        tool_definition(
            "gentufa",
            "Parse Lojban syntax. Defaults to format=tree with refs and detailed errors; use JSON only for programmatic consumers.",
            tool_request_schema::<ToolGentufaRequest>(),
        ),
        tool_definition(
            "gimfihi",
            "Compose candidate gismu from source words. Defaults to the compact table output; JSON is available for programmatic use.",
            tool_request_schema::<ToolGimfihiRequest>(),
        ),
        tool_definition(
            "tersmu",
            "Build Lojban semantic JSON. Defaults to JSON for programmatic use; prefer other tools' default text formats when semantics JSON is not specifically needed.",
            tool_request_schema::<ToolTersmuRequest>(),
        ),
    ]
}

#[requires(!name.trim().is_empty())]
#[requires(!description.trim().is_empty())]
#[ensures(true)]
fn tool_definition(name: &str, description: &str, input_schema: Value) -> Value {
    json!({
        "name": name,
        "title": name,
        "description": description,
        "inputSchema": input_schema,
        "annotations": {
            "readOnlyHint": true,
            "destructiveHint": false,
            "idempotentHint": true,
            "openWorldHint": false
        }
    })
}

#[requires(true)]
#[ensures(ret.is_object())]
fn tool_request_schema<T>() -> Value
where
    T: JsonSchema,
{
    serde_json::to_value(schemars::schema_for!(T))
        .expect("generated MCP tool schema serializes to JSON")
}

#[requires(!params.name.trim().is_empty())]
#[ensures(true)]
async fn call_tool(params: ToolCallParams, tool_services: ToolServices) -> Value {
    let params = params.into_data();
    let name = params.name;
    let arguments = params.arguments;
    match name.as_str() {
        "cukta" => {
            call_typed_tool(arguments, move |request| tool_services.run_cukta(request)).await
        }
        "vlacku" => {
            call_typed_tool(arguments, move |request| tool_services.run_vlacku(request)).await
        }
        "jvozba" => call_typed_tool(arguments, run_tool_jvozba).await,
        "vlasei" => call_typed_tool(arguments, run_tool_vlasei).await,
        "gentufa" => call_typed_tool(arguments, run_tool_gentufa).await,
        "gimfihi" => call_typed_tool(arguments, run_tool_gimfihi).await,
        "tersmu" => call_typed_tool(arguments, run_tool_tersmu).await,
        _ => tool_error_result(format!("Unknown tool: {name}")),
    }
}

#[requires(true)]
#[ensures(true)]
async fn call_typed_tool<T, F>(arguments: Value, runner: F) -> Value
where
    T: for<'de> Deserialize<'de> + Send + 'static,
    F: FnOnce(T) -> anyhow::Result<ToolRenderedOutput> + Send + 'static,
{
    let request = match serde_json::from_value::<T>(arguments) {
        Ok(request) => request,
        Err(error) => return tool_error_result(format!("Invalid tool arguments: {error}")),
    };
    match tokio::task::spawn_blocking(move || runner(request)).await {
        Ok(Ok(output)) => tool_output_result(output),
        Ok(Err(error)) => tool_error_result(error.to_string()),
        Err(error) => tool_error_result(format!("tool task failed: {error}")),
    }
}

#[requires(true)]
#[ensures(true)]
fn tool_output_result(output: ToolRenderedOutput) -> Value {
    if matches!(
        output.status,
        ToolStatus::Failure | ToolStatus::InvalidInput
    ) {
        return tool_error_result(tool_error_text(&output));
    }
    let mut content = Vec::new();
    if !output.stderr.is_empty() {
        content.push(json!({ "type": "text", "text": output.stderr }));
    }
    if content_type_is_image(output.content_type.as_deref()) {
        let mime_type = output
            .content_type
            .as_deref()
            .and_then(|value| value.split(';').next())
            .unwrap_or("application/octet-stream");
        content.push(json!({
            "type": "image",
            "mimeType": mime_type,
            "data": base64::engine::general_purpose::STANDARD.encode(&output.stdout),
        }));
        return json!({ "content": content });
    }
    let text = output
        .stdout_text()
        .map(str::to_owned)
        .unwrap_or_else(|_| String::from_utf8_lossy(&output.stdout).into_owned());
    content.push(json!({ "type": "text", "text": text }));
    if content_type_is_json(output.content_type.as_deref()) {
        if let Ok(structured) = serde_json::from_slice::<Value>(&output.stdout) {
            let structured = structured_content_value(structured);
            return json!({
                "content": content,
                "structuredContent": structured
            });
        }
    }
    json!({ "content": content })
}

#[requires(true)]
#[ensures(ret.is_object())]
fn structured_content_value(value: Value) -> Value {
    if value.is_object() {
        value
    } else {
        json!({ "result": value })
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn tool_error_text(output: &ToolRenderedOutput) -> String {
    if !output.stderr.is_empty() {
        return output.stderr.clone();
    }
    if let Ok(stdout) = output.stdout_text()
        && !stdout.trim().is_empty()
    {
        return stdout.to_owned();
    }
    format!("tool failed with status {:?}", output.status)
}

#[requires(true)]
#[ensures(true)]
fn tool_error_result(message: String) -> Value {
    json!({
        "content": [
            {
                "type": "text",
                "text": message
            }
        ],
        "isError": true
    })
}

#[requires(true)]
#[ensures(true)]
fn content_type_is_json(content_type: Option<&str>) -> bool {
    content_type
        .and_then(|value| value.split(';').next())
        .is_some_and(|value| value.trim().eq_ignore_ascii_case("application/json"))
}

#[requires(true)]
#[ensures(true)]
fn content_type_is_image(content_type: Option<&str>) -> bool {
    content_type
        .and_then(|value| value.split(';').next())
        .is_some_and(|value| value.trim().starts_with("image/"))
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
fn json_response(status: StatusCode, value: Value) -> Response<Body> {
    Response::builder()
        .status(status)
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(value.to_string()))
        .expect("MCP JSON response builder is valid")
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

#[requires(true)]
#[ensures(true)]
fn origin_is_allowed(headers: &HeaderMap) -> bool {
    let Some(origin) = headers
        .get(ORIGIN)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return true;
    };
    let Some(host) = headers
        .get(HOST)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return false;
    };
    origin == format!("https://{host}") || origin == format!("http://{host}")
}
