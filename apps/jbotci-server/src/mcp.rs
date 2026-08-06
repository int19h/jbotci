use std::future::Future;
use std::sync::Arc;

use axum::body::{Body, Bytes};
use axum::extract::Extension;
use axum::http::header::CONTENT_TYPE;
use axum::http::{Response, StatusCode};
use base64::Engine;
use bityzba::{invariant, requires};
use jbotci_cli::projection::ProjectionFailureEnvelope;
use jbotci_cli::{
    GimfihiSourceWordKind, ToolCuktaRequest, ToolGentufaRequest, ToolGimfihiRequest,
    ToolJvozbaRequest, ToolRenderedOutput, ToolStatus, ToolTersmuRequest, ToolVlackuRequest,
    ToolVlaseiRequest, run_tool_gentufa, run_tool_gimfihi, run_tool_jvozba, run_tool_tersmu,
    run_tool_vlasei, tool_request_schema,
};
use serde::Deserialize;
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

/// The SFN-XML tersmu document schema, embedded from `jbotci-semantics` so
/// the server stays self-contained and the resource works on any transport.
const SFN_XML_SCHEMA_URI: &str = "jbotci:///tersmu/sfn-xml-v0.xsd";
const SFN_XML_SCHEMA_NAME: &str = "sfn-xml-schema";
const SFN_XML_SCHEMA_TITLE: &str = "SFN-XML v0 document schema";
const SFN_XML_SCHEMA_MIME: &str = "application/xml";
const SFN_XML_SCHEMA_DESCRIPTION: &str = "The XSD 1.0 schema of the complete SFN-XML tersmu document that `tersmu --format xml` \
     emits — the compact scoped form, the `WORDS` word-card section, and the `TYPED-GRAPH` fallback form. \
     Every element and attribute carries reference documentation in `xs:annotation`, including the \
     ID/IDREF discipline and the constructs that resist XSD 1.0 typing. Read this to interpret or \
     validate tersmu XML output.";

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
    body: Bytes,
) -> Response<Body> {
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
                Some(SFN_XML_SCHEMA_URI) => sfn_xml_schema_resource_contents(),
                Some(other) => {
                    return json_response(
                        StatusCode::OK,
                        json_rpc_error(id, -32602, &format!("Unknown resource URI: {other}")),
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
        "instructions": "jbotci is a Lojban toolkit. Choose a tool by task: `cukta` for the reference grammar (CLL), `vlacku` for dictionary word lookups, `gentufa` to parse a sentence's grammar, `vlasei` for word-level morphology, `tersmu` for deep logical meaning, `jvozba` to build a compound word, `gimfihi` to invent a new root word. Tools default to a readable text (or image) format; `tersmu` defaults to canonical, self-describing SFN-XML. Request `smusni` for the experimental human-readable typed S-expression notation or `json` for the canonical interchange graph."
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
            "Lojban reference book (CLL)",
            "Read or search *The Complete Lojban Language* (CLL), the canonical reference grammar. \
             Use it for grammar rules, explanations of how constructs work, and worked examples — \
             not for plain word definitions (use `vlacku` for those). Defaults to semantic search; \
             output is readable Markdown.",
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
             `gentufa`, and for its meaning use `tersmu`. Recoverable errors return marked partial \
             output plus diagnostics. Defaults to a readable tree.",
            tool_request_schema::<ToolVlaseiRequest>(),
        ),
        tool_definition(
            "gentufa",
            "Parse Lojban grammar",
            "Parse Lojban text into its grammar (syntax) tree — the authoritative way to see how a \
             sentence is structured and which word fills each role. For word-level analysis only use \
             `vlasei`; for logical meaning use `tersmu`. Recoverable errors return a marked partial \
             tree plus diagnostics. Defaults to a readable tree with place references.",
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
        tool_definition(
            "tersmu",
            "Lojban semantics",
            "Compute the deep semantic/logical meaning of Lojban text. The default is canonical \
             scoped SFN-XML: the `SFN` root begins with an embedded, authoritative `KEY`; the \
             ordinary scoped form places shared graph definitions in scoped `DEFS` before the \
             semantic body. UPPERCASE names are structural elements and attributes, PascalCase \
             values are sorts, and lowercase content words occur only as data values. Simple ID \
             and number lists are space-separated attributes; semantic structure remains \
             element-valued, child order is fixed, and childless elements self-close. `ID=` \
             defines a shared graph node (except that `DEICTIC-GROUND` role \
             references define speech-situation referents); `REF=` and named `*-REF=` attributes \
             point to discourse referents, later references reuse the exact node, and `GROUND=` \
             points to a deictic-ground unit. IDs are opaque; distinct IDs assert neither identity \
             nor non-identity. `EXISTS`, `FORALL`, and `CARDINALITY` bind a `VARIABLE`; use sites \
             carry `REF=`, while `RESTRICTION` and `BODY` are explicit siblings. `EXISTS` has no \
             `RESTRICTION`; the other binders always write one, even when empty. `ADJUNCT` adds a \
             predicate-keyed optional participant: compact lexical adjuncts use `PREDICATE=` and \
             flat `ARG` children, composite adjuncts use `BODY`, `FILL=\"true\"` marks the unique \
             explicit non-host filled place, and `APPLIES-TO` links the host. `REF=\"SOME\"` is a \
             distinct elided node per occurrence, without asserting non-identity. `DEICTIC-GROUND`, \
             selected by `UTTERANCE GROUND=`, is the shared speech situation identified by its \
             speaker, audience, time, and place referents; grounds share one definition exactly \
             when those four referents are pairwise identical. Silence is noncommittal: references \
             are number-neutral unless an explicit description quantity, cardinality binder, or \
             mass restriction commits number; an unmarked referent inside a quantifier may be \
             shared or vary with the bound variable, while `SAME-FOR-ALL` marks known sharing and \
             `POSSIBLY-DIFFERENT-PER=` names a strict subset of enclosing binders on which it may \
             depend; and an absent facet attribute means `UNSPECIFIED` (no commitment). That facet default is \
             distinct from an absent XML structure: do not invent an `UNSPECIFIED` value or a \
             negative claim for arbitrary absent elements or attributes. With definitions on (the \
             default), a `WORDS` section follows the `KEY`: one structured `WORD` card per content \
             word, whose `DEF` and `NOTES` dictionary prose uses `ARG INDEX=\"n\"` place markup in \
             the same argument vocabulary as predications; `KNOWN=\"false\"` marks a \
             dictionary-absent word, and `COMPOSITE-APPROX` shows the mechanical, suggestive-only \
             composition of a dictionary-absent compound through the same `KIND-COMPOSITION` idiom \
             as the body. The `SFN FORM=\"TYPED-GRAPH\"` \
             fallback uses its own embedded `KEY` and `OBJECT`/`FIELD`/`RECORD`/`LIST`/`ITEM`/`REFERENCE` \
             typed vocabulary; follow that key instead of the ordinary scoped vocabulary. The \
             complete document schema (XSD 1.0, with per-element reference documentation) is \
             queryable as the `jbotci:///tersmu/sfn-xml-v0.xsd` resource. Request \
             `smusni` for the experimental human-readable typed S-expression notation or `json` for the canonical interchange graph. \
             For grammar use `gentufa`, for morphology use `vlasei`.",
            tool_request_schema::<ToolTersmuRequest>(),
        ),
    ]
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn mcp_resources() -> Vec<Value> {
    vec![
        json!({
            "uri": LOJBAN_GRAMMAR_URI,
            "name": LOJBAN_GRAMMAR_NAME,
            "title": LOJBAN_GRAMMAR_TITLE,
            "description": LOJBAN_GRAMMAR_DESCRIPTION,
            "mimeType": LOJBAN_GRAMMAR_MIME,
        }),
        json!({
            "uri": SFN_XML_SCHEMA_URI,
            "name": SFN_XML_SCHEMA_NAME,
            "title": SFN_XML_SCHEMA_TITLE,
            "description": SFN_XML_SCHEMA_DESCRIPTION,
            "mimeType": SFN_XML_SCHEMA_MIME,
        }),
    ]
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

#[requires(true)]
#[ensures(ret.is_object())]
fn sfn_xml_schema_resource_contents() -> Value {
    json!({
        "contents": [{
            "uri": SFN_XML_SCHEMA_URI,
            "name": SFN_XML_SCHEMA_NAME,
            "title": SFN_XML_SCHEMA_TITLE,
            "mimeType": SFN_XML_SCHEMA_MIME,
            "text": jbotci_semantics::SFN_XML_SCHEMA_XSD,
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

#[requires(!params.name.trim().is_empty())]
#[ensures(true)]
async fn call_tool(params: ToolCallParams, tool_services: ToolServices) -> Value {
    let params = params.into_data();
    let name = params.name;
    let arguments = params.arguments;
    match name.as_str() {
        "cukta" => {
            call_async_typed_tool(arguments, move |request| async move {
                tool_services.run_cukta(request).await
            })
            .await
        }
        "vlacku" => {
            call_async_typed_tool(arguments, move |request| async move {
                tool_services.run_vlacku(request).await
            })
            .await
        }
        "jvozba" => call_typed_tool(arguments, run_tool_jvozba).await,
        "vlasei" => call_typed_tool(arguments, run_tool_vlasei).await,
        "gentufa" => call_typed_tool(arguments, run_tool_gentufa).await,
        "gimfihi" => {
            call_typed_tool(arguments, |request| {
                run_tool_gimfihi(request, GimfihiSourceWordKind::Ipa)
            })
            .await
        }
        "tersmu" => call_typed_tool(arguments, run_tool_tersmu).await,
        _ => tool_error_result(format!("Unknown tool: {name}")),
    }
}

#[requires(true)]
#[ensures(true)]
async fn call_async_typed_tool<T, F, Fut>(arguments: Value, runner: F) -> Value
where
    T: for<'de> Deserialize<'de> + Send + 'static,
    F: FnOnce(T) -> Fut + Send + 'static,
    Fut: Future<Output = anyhow::Result<ToolRenderedOutput>> + Send + 'static,
{
    let request = match serde_json::from_value::<T>(arguments) {
        Ok(request) => request,
        Err(error) => return tool_error_result(format!("Invalid tool arguments: {error}")),
    };
    match runner(request).await {
        Ok(output) => tool_output_result(output),
        Err(error) => tool_error_result(error.to_string()),
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
        // A smusni projection failure carries the same structured envelope the
        // HTTP profile returns: a concise readable summary first, then the
        // envelope as one JSON text item. Nothing here reparses the rendered
        // diagnostics (specification section 16.3).
        if let Some(envelope) = &output.projection_failure {
            return projection_failure_error_result(envelope);
        }
        return tool_error_result(tool_error_text(&output));
    }
    let mut content = Vec::new();
    if !output.stderr.is_empty() {
        content.push(json!({ "type": "text", "text": output.stderr }));
    }
    // Raster images (PNG, …) are returned as image content for direct display.
    // SVG is deliberately NOT treated as an image here: it is XML text, and the
    // chatbot harnesses that consume this server cannot render an SVG *image*,
    // so it falls through to the text branch below — the model receives the SVG
    // source and can read or reuse it (e.g. embed it in a page). Use the `png`
    // format when a displayable raster image is wanted.
    if content_type_is_raster_image(output.content_type.as_deref()) {
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
    // A single readable text representation (also the SVG source, per above). For
    // JSON formats the text is itself valid JSON, so we deliberately do not also
    // emit a duplicate `structuredContent` block: this server is consumed by
    // models that read the text content, and no tool declares an `outputSchema`,
    // so a structured copy would only cost tokens without adding value.
    let text = output
        .stdout_text()
        .map(str::to_owned)
        .unwrap_or_else(|_| String::from_utf8_lossy(&output.stdout).into_owned());
    content.push(json!({ "type": "text", "text": text }));
    json!({ "content": content })
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn tool_error_text(output: &ToolRenderedOutput) -> String {
    let mut text = output.stderr.clone();
    if let Ok(stdout) = output.stdout_text()
        && !stdout.trim().is_empty()
    {
        if !text.is_empty() && !text.ends_with('\n') {
            text.push('\n');
        }
        text.push_str(stdout);
    }
    if !text.is_empty() {
        return text;
    }
    format!("tool failed with status {:?}", output.status)
}

/// The MCP presentation of one structured projection failure.
#[requires(true)]
#[ensures(ret["isError"] == true)]
fn projection_failure_error_result(envelope: &ProjectionFailureEnvelope) -> Value {
    let mut summary = format!(
        "{}: the {} projection of this input failed with {} registered projection error(s)",
        envelope.code, envelope.format, envelope.total
    );
    if envelope.truncated {
        summary.push_str(&format!("; {} of them are carried here", envelope.returned));
    }
    if let Some(first) = envelope.diagnostics.first() {
        summary.push_str(&format!(
            "\nfirst error {}: {}",
            first.reason_id, first.message
        ));
    }
    let serialized = serde_json::to_string_pretty(envelope)
        .unwrap_or_else(|error| format!("{{\"serializationError\":{error:?}}}"));
    json!({
        "content": [
            { "type": "text", "text": summary },
            { "type": "text", "text": serialized }
        ],
        "isError": true
    })
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn failure_text_preserves_diagnostics_and_recovered_output() {
        let output = ToolRenderedOutput {
            status: ToolStatus::Failure,
            stdout: "([mi ‼ku‼] [.i do])\n".as_bytes().to_vec(),
            stderr: "error[syntax.unexpected-cmavo]: unexpected cmavo\n".to_owned(),
            content_type: Some("text/plain; charset=utf-8".to_owned()),
            projection_failure: None,
        };

        let result = tool_output_result(output);

        assert_eq!(result["isError"], true);
        assert_eq!(
            result["content"][0]["text"],
            "error[syntax.unexpected-cmavo]: unexpected cmavo\n([mi ‼ku‼] [.i do])\n"
        );
    }
}
