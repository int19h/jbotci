use std::sync::Arc;

use axum::body::{Body, Bytes};
use axum::extract::Extension;
use axum::http::header::CONTENT_TYPE;
use axum::http::{HeaderMap, Response, StatusCode};
use bityzba::{invariant, new, requires};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use jbotci_cli::{
    ToolCollisionScope, ToolCuktaMode, ToolCuktaRequest, ToolGentufaFormat, ToolGentufaRequest,
    ToolGimfihiFormat, ToolGimfihiRequest, ToolGimfihiSource, ToolJvozbaMode, ToolJvozbaPart,
    ToolJvozbaPartKind, ToolJvozbaRequest, ToolRenderedOutput, ToolVlackuMode, ToolVlackuRequest,
    ToolVlaseiFormat, ToolVlaseiRequest, run_tool_gentufa, run_tool_gimfihi, run_tool_jvozba,
    run_tool_vlasei,
};
use jbotci_web_core::{
    CUKTA_WEB_DEFAULT_COUNT, CuktaWebMode, CuktaWebSearchState, CuktaWebState, CuktaWebView,
    GentufaWebState, GentufaWebViewMode, VLACKU_WEB_DEFAULT_COUNT, VlackuWebMode, VlackuWebState,
    WebRoute, web_route_url,
};
use serde_json::{Value, json};

use crate::{AppState, ToolServices};

const DISCORD_SIGNATURE_HEADER: &str = "x-signature-ed25519";
const DISCORD_TIMESTAMP_HEADER: &str = "x-signature-timestamp";
const DISCORD_RESPONSE_LIMIT: usize = 1900;
const DEFAULT_PUBLIC_BASE_URL: &str = "https://jbotci.app";
const DEFAULT_DISCORD_API_BASE: &str = "https://discord.com/api/v10";

#[invariant(payload.is_object(), "Discord command registration payload must be an object")]
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct DiscordCommandRegistration {
    pub payload: Value,
}

#[invariant(::Gentufa(_) => true)]
#[invariant(::Vlasei(_) => true)]
#[invariant(::Vlacku(_) => true)]
#[invariant(::Cukta(_) => true)]
#[invariant(::Jvozba(_) => true)]
#[invariant(::Gimfihi(_) => true)]
#[derive(Debug, Clone)]
enum DiscordCommand {
    Gentufa(ToolGentufaRequest),
    Vlasei(ToolVlaseiRequest),
    Vlacku(ToolVlackuRequest),
    Cukta(ToolCuktaRequest),
    Jvozba(ToolJvozbaRequest),
    Gimfihi(ToolGimfihiRequest),
}

#[requires(true)]
#[ensures(true)]
pub(crate) async fn discord_post(
    Extension(state): Extension<Arc<AppState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Response<Body> {
    if let Err(error) = verify_discord_signature(&headers, &body) {
        return plain_response(StatusCode::UNAUTHORIZED, &error);
    }
    let value = match serde_json::from_slice::<Value>(&body) {
        Ok(value) => value,
        Err(error) => {
            return json_response(
                StatusCode::BAD_REQUEST,
                json!({
                    "type": 4,
                    "data": discord_message_data(&format!("Invalid Discord interaction JSON: {error}"))
                }),
            );
        }
    };
    match value.get("type").and_then(Value::as_i64) {
        Some(1) => json_response(StatusCode::OK, json!({ "type": 1 })),
        Some(2) => handle_application_command(value, state.tool_services()).await,
        _ => json_response(
            StatusCode::OK,
            json!({
                "type": 4,
                "data": discord_message_data("Unsupported Discord interaction type.")
            }),
        ),
    }
}

#[requires(true)]
#[ensures(true)]
async fn handle_application_command(value: Value, tool_services: ToolServices) -> Response<Body> {
    let application_id = value
        .get("application_id")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    let token = value
        .get("token")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    if application_id.is_empty() || token.is_empty() {
        return json_response(
            StatusCode::OK,
            json!({
                "type": 4,
                "data": discord_message_data("Discord interaction is missing application id or token.")
            }),
        );
    }
    let command = match parse_discord_command(&value) {
        Ok(command) => command,
        Err(error) => {
            return json_response(
                StatusCode::OK,
                json!({
                    "type": 4,
                    "data": discord_message_data(&error)
                }),
            );
        }
    };
    if discord_followup_enabled() {
        tokio::spawn(async move {
            let rendered =
                tokio::task::spawn_blocking(move || render_discord_command(command, tool_services))
                    .await;
            let message = match rendered {
                Ok(Ok(message)) => message,
                Ok(Err(error)) => discord_message_data(&format!("jbotci command failed: {error}")),
                Err(error) => discord_message_data(&format!("jbotci command task failed: {error}")),
            };
            if let Err(error) =
                send_discord_original_response_edit(&application_id, &token, message)
            {
                eprintln!("failed to edit Discord interaction response: {error}");
            }
        });
    }
    json_response(
        StatusCode::OK,
        json!({
            "type": 5
        }),
    )
}

#[requires(true)]
#[ensures(ret.payload.get("name").and_then(Value::as_str) == Some("jbotci"))]
pub fn discord_command_registration() -> DiscordCommandRegistration {
    new!(DiscordCommandRegistration {
        payload: json!({
            "name": "jbotci",
            "type": 1,
            "description": "Lojban tools",
            "options": discord_command_options()
        }),
    })
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn discord_command_options() -> Vec<Value> {
    vec![
        json!({
            "name": "gentufa",
            "description": "Parse Lojban syntax",
            "type": 1,
            "options": [
                string_option("text", "Lojban text to parse", true),
                string_choices_option("format", "Output format", false, &["tree", "brackets", "json", "svg", "png"]),
                string_option("dialect", "Dialect formula", false),
                bool_option("show-elided", "Show elided terminators", false)
            ]
        }),
        json!({
            "name": "vlasei",
            "description": "Segment Lojban morphology",
            "type": 1,
            "options": [
                string_option("text", "Lojban text to segment", true),
                string_choices_option("format", "Output format", false, &["tree", "brackets", "ipa", "json"])
            ]
        }),
        json!({
            "name": "vlacku",
            "description": "Search dictionary cards",
            "type": 1,
            "options": [
                string_option("query", "Search query (word/rafsi accept * ? globs and /regex/)", true),
                string_choices_option("mode", "Search mode", false, &["word", "rafsi", "lujvo", "sound", "meaning"]),
                integer_option("count", "Maximum result count", false),
                string_option("word-type", "Word type filter", false)
            ]
        }),
        json!({
            "name": "cukta",
            "description": "Read or search the CLL",
            "type": 1,
            "options": [
                string_choices_option("mode", "Read/search mode", false, &["meaning", "word", "section", "example", "toc"]),
                string_option("query", "Reference or search query", false),
                integer_option("count", "Maximum result count", false),
                string_choices_option("format", "Output format", false, &["markdown", "html", "raw"])
            ]
        }),
        json!({
            "name": "jvozba",
            "description": "Build a lujvo or cmevla",
            "type": 1,
            "options": [
                string_option("parts", "Space-separated source words", true),
                string_option("rafsi", "Space- or comma-separated fixed rafsi appended after words", false),
                string_choices_option("mode", "Output word kind", false, &["lujvo", "cmevla"])
            ]
        }),
        json!({
            "name": "gimfihi",
            "description": "Compose candidate gismu",
            "type": 1,
            "options": [
                string_option("sources", "Comma-separated LANG[:WEIGHT]:WORD source specs", false),
                string_option("preset", "Preset source set", false),
                integer_option("count", "Maximum candidate count", false),
                string_choices_option("format", "Output format", false, &["table", "json"])
            ]
        }),
    ]
}

#[requires(!name.is_empty())]
#[requires(!description.is_empty())]
#[ensures(true)]
fn string_option(name: &str, description: &str, required: bool) -> Value {
    json!({
        "name": name,
        "description": description,
        "type": 3,
        "required": required
    })
}

#[requires(!name.is_empty())]
#[requires(!description.is_empty())]
#[ensures(true)]
fn string_choices_option(name: &str, description: &str, required: bool, choices: &[&str]) -> Value {
    json!({
        "name": name,
        "description": description,
        "type": 3,
        "required": required,
        "choices": choices.iter().map(|choice| json!({
            "name": *choice,
            "value": *choice
        })).collect::<Vec<_>>()
    })
}

#[requires(!name.is_empty())]
#[requires(!description.is_empty())]
#[ensures(true)]
fn bool_option(name: &str, description: &str, required: bool) -> Value {
    json!({
        "name": name,
        "description": description,
        "type": 5,
        "required": required
    })
}

#[requires(!name.is_empty())]
#[requires(!description.is_empty())]
#[ensures(true)]
fn integer_option(name: &str, description: &str, required: bool) -> Value {
    json!({
        "name": name,
        "description": description,
        "type": 4,
        "required": required,
        "min_value": 1
    })
}

#[requires(true)]
#[ensures(true)]
fn parse_discord_command(value: &Value) -> Result<DiscordCommand, String> {
    let options = value
        .get("data")
        .and_then(|data| data.get("options"))
        .and_then(Value::as_array)
        .ok_or_else(|| "Missing Discord subcommand options.".to_owned())?;
    let subcommand = options
        .first()
        .ok_or_else(|| "Missing jbotci subcommand.".to_owned())?;
    let name = subcommand
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| "Discord subcommand is missing a name.".to_owned())?;
    let sub_options = subcommand
        .get("options")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    match name {
        "gentufa" => parse_discord_gentufa(sub_options).map(DiscordCommand::Gentufa),
        "vlasei" => parse_discord_vlasei(sub_options).map(DiscordCommand::Vlasei),
        "vlacku" => parse_discord_vlacku(sub_options).map(DiscordCommand::Vlacku),
        "cukta" => parse_discord_cukta(sub_options).map(DiscordCommand::Cukta),
        "jvozba" => parse_discord_jvozba(sub_options).map(DiscordCommand::Jvozba),
        "gimfihi" => parse_discord_gimfihi(sub_options).map(DiscordCommand::Gimfihi),
        other => Err(format!("Unsupported jbotci subcommand `{other}`.")),
    }
}

#[requires(true)]
#[ensures(true)]
fn parse_discord_gentufa(options: &[Value]) -> Result<ToolGentufaRequest, String> {
    Ok(ToolGentufaRequest {
        text: required_string_option(options, "text")?,
        format: parse_gentufa_format(optional_string_option(options, "format").as_deref())?,
        dialect: optional_string_option(options, "dialect"),
        show_defs: false,
        show_spans: false,
        show_refs: Some(true),
        show_elided: optional_bool_option(options, "show-elided").unwrap_or(false),
        decompose_lujvo: false,
        indent: None,
    })
}

#[requires(true)]
#[ensures(true)]
fn parse_discord_vlasei(options: &[Value]) -> Result<ToolVlaseiRequest, String> {
    Ok(ToolVlaseiRequest {
        text: required_string_option(options, "text")?,
        format: parse_vlasei_format(optional_string_option(options, "format").as_deref())?,
        dialect: None,
        show_spans: false,
        decompose_lujvo: false,
        indent: None,
    })
}

#[requires(true)]
#[ensures(true)]
fn parse_discord_vlacku(options: &[Value]) -> Result<ToolVlackuRequest, String> {
    Ok(ToolVlackuRequest {
        mode: parse_vlacku_mode(optional_string_option(options, "mode").as_deref())?,
        query: required_string_option(options, "query")?,
        count: optional_integer_option(options, "count"),
        word_types: optional_string_option(options, "word-type")
            .into_iter()
            .collect(),
        min_votes: None,
        min_similarity: None,
        decompose_lujvo: true,
        show_etymology: false,
    })
}

#[requires(true)]
#[ensures(true)]
fn parse_discord_cukta(options: &[Value]) -> Result<ToolCuktaRequest, String> {
    Ok(ToolCuktaRequest {
        mode: parse_cukta_mode(optional_string_option(options, "mode").as_deref())?,
        query: optional_string_option(options, "query"),
        count: optional_integer_option(options, "count"),
        targets: Vec::new(),
        format: parse_cukta_format(optional_string_option(options, "format").as_deref())?,
    })
}

#[requires(true)]
#[ensures(true)]
fn parse_discord_jvozba(options: &[Value]) -> Result<ToolJvozbaRequest, String> {
    let mut parts = split_discord_words(&required_string_option(options, "parts")?)
        .into_iter()
        .map(|value| ToolJvozbaPart {
            kind: ToolJvozbaPartKind::Word,
            value,
        })
        .collect::<Vec<_>>();
    for value in optional_string_option(options, "rafsi")
        .as_deref()
        .map(split_discord_words)
        .unwrap_or_default()
    {
        parts.push(ToolJvozbaPart {
            kind: ToolJvozbaPartKind::FixedRafsi,
            value,
        });
    }
    Ok(ToolJvozbaRequest {
        mode: parse_jvozba_mode(optional_string_option(options, "mode").as_deref())?,
        parts,
    })
}

#[requires(true)]
#[ensures(true)]
fn parse_discord_gimfihi(options: &[Value]) -> Result<ToolGimfihiRequest, String> {
    let sources = optional_string_option(options, "sources")
        .as_deref()
        .map(split_discord_sources)
        .unwrap_or_default()
        .iter()
        .map(|spec| ToolGimfihiSource::from_spec(spec))
        .collect::<Result<Vec<_>, String>>()?;
    Ok(ToolGimfihiRequest {
        sources,
        preset: optional_string_option(options, "preset"),
        shapes: Vec::new(),
        check_collisions: ToolCollisionScope::default(),
        all_letters: false,
        show_collisions: false,
        require_free_short_rafsi: false,
        count: optional_integer_option(options, "count"),
        highlight: None,
        format: parse_gimfihi_format(optional_string_option(options, "format").as_deref())?,
    })
}

#[requires(true)]
#[ensures(true)]
fn render_discord_command(
    command: DiscordCommand,
    tool_services: ToolServices,
) -> anyhow::Result<Value> {
    let (output, link) = match command {
        DiscordCommand::Gentufa(request) => {
            let link = gentufa_link(&request);
            (run_tool_gentufa(request)?, link)
        }
        DiscordCommand::Vlasei(request) => (run_tool_vlasei(request)?, None),
        DiscordCommand::Vlacku(request) => {
            let link = vlacku_link(&request);
            (tool_services.run_vlacku(request)?, link)
        }
        DiscordCommand::Cukta(request) => {
            let link = cukta_link(&request);
            (tool_services.run_cukta(request)?, link)
        }
        DiscordCommand::Jvozba(request) => {
            (run_tool_jvozba(request)?, Some(absolute_web_url("/vlacku")))
        }
        DiscordCommand::Gimfihi(request) => {
            let link = Some(absolute_web_url("/gimfihi"));
            (run_tool_gimfihi(request)?, link)
        }
    };
    Ok(discord_message_data(&trim_discord_output(
        &output,
        link.as_deref(),
    )))
}

#[requires(true)]
#[ensures(true)]
fn gentufa_link(request: &ToolGentufaRequest) -> Option<String> {
    Some(absolute_web_url(&web_route_url(
        "",
        &WebRoute::Gentufa(GentufaWebState {
            text: request.text.clone(),
            dialect: request.dialect.clone(),
            view_mode: GentufaWebViewMode::Blocks,
            show_elided: request.show_elided,
            show_glosses: false,
        }),
    )))
}

#[requires(true)]
#[ensures(true)]
fn vlacku_link(request: &ToolVlackuRequest) -> Option<String> {
    let mode = match request.mode {
        ToolVlackuMode::Rafsi => VlackuWebMode::Rafsi,
        ToolVlackuMode::Sound => VlackuWebMode::Sound,
        ToolVlackuMode::Meaning => VlackuWebMode::Meaning,
        ToolVlackuMode::Word | ToolVlackuMode::Lujvo => VlackuWebMode::Word,
    };
    Some(absolute_web_url(&web_route_url(
        "",
        &WebRoute::Vlacku(VlackuWebState {
            mode,
            query: request.query.clone(),
            count: request.count.unwrap_or(VLACKU_WEB_DEFAULT_COUNT),
            word_types: request.word_types.clone(),
        }),
    )))
}

#[requires(true)]
#[ensures(true)]
fn cukta_link(request: &ToolCuktaRequest) -> Option<String> {
    let route = match request.mode {
        ToolCuktaMode::Section => WebRoute::Cukta(CuktaWebState {
            view: CuktaWebView::Section {
                reference: request.query.clone().unwrap_or_default(),
            },
        }),
        ToolCuktaMode::Toc => WebRoute::Cukta(CuktaWebState {
            view: CuktaWebView::Index,
        }),
        ToolCuktaMode::Word | ToolCuktaMode::Meaning => WebRoute::Cukta(CuktaWebState {
            view: CuktaWebView::Search(CuktaWebSearchState {
                mode: if request.mode == ToolCuktaMode::Word {
                    CuktaWebMode::Word
                } else {
                    CuktaWebMode::Meaning
                },
                query: request.query.clone().unwrap_or_default(),
                count: request.count.unwrap_or(CUKTA_WEB_DEFAULT_COUNT),
                targets: request
                    .targets
                    .iter()
                    .map(|target| target.as_str().to_owned())
                    .collect(),
            }),
        }),
        ToolCuktaMode::Example => return Some(absolute_web_url("/cukta")),
    };
    Some(absolute_web_url(&web_route_url("", &route)))
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn trim_discord_output(output: &ToolRenderedOutput, link: Option<&str>) -> String {
    let mut text = String::new();
    if !output.stderr.is_empty() {
        text.push_str(&output.stderr);
    }
    if let Ok(stdout) = output.stdout_text() {
        text.push_str(stdout);
    } else if !output.stdout.is_empty() {
        text.push_str("[binary output]");
    }
    if text.trim().is_empty() {
        text.push_str(&format!("jbotci finished with status {:?}", output.status));
    }
    let suffix = link.map(|link| format!("\n\nFull render: {link}"));
    let suffix_len = suffix.as_ref().map(String::len).unwrap_or(0);
    let limit = DISCORD_RESPONSE_LIMIT.saturating_sub(suffix_len);
    let mut trimmed = truncate_to_char_boundary(&text, limit);
    if trimmed.len() < text.len() {
        trimmed.push_str("\n...");
    }
    if let Some(suffix) = suffix {
        trimmed.push_str(&suffix);
    }
    trimmed
}

#[requires(true)]
#[ensures(ret.len() <= input.len())]
fn truncate_to_char_boundary(input: &str, limit: usize) -> String {
    if limit == 0 {
        return String::new();
    }
    if input.len() <= limit {
        return input.to_owned();
    }
    let mut end = limit;
    while end > 0 && !input.is_char_boundary(end) {
        end -= 1;
    }
    input[..end].to_owned()
}

#[requires(!content.trim().is_empty())]
#[ensures(true)]
fn discord_message_data(content: &str) -> Value {
    json!({
        "content": content,
        "allowed_mentions": {
            "parse": []
        }
    })
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn send_discord_original_response_edit(
    application_id: &str,
    token: &str,
    payload: Value,
) -> anyhow::Result<()> {
    if application_id.is_empty() || token.is_empty() {
        anyhow::bail!("application id and token are required for Discord follow-up edits");
    }
    let base =
        std::env::var("DISCORD_API_BASE").unwrap_or_else(|_| DEFAULT_DISCORD_API_BASE.to_owned());
    let url = format!(
        "{}/webhooks/{}/{}/messages/@original",
        base.trim_end_matches('/'),
        application_id,
        token
    );
    let body = payload.to_string();
    ureq::patch(&url)
        .header(CONTENT_TYPE.as_str(), "application/json")
        .send(body)
        .map(|_| ())
        .map_err(|error| anyhow::anyhow!("PATCH `{url}`: {error}"))
}

#[requires(true)]
#[ensures(true)]
fn discord_followup_enabled() -> bool {
    std::env::var("JBOTCI_DISCORD_DISABLE_FOLLOWUP")
        .ok()
        .as_deref()
        != Some("1")
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.is_empty()))]
fn verify_discord_signature(headers: &HeaderMap, body: &[u8]) -> Result<(), String> {
    let public_key = std::env::var("DISCORD_PUBLIC_KEY")
        .map_err(|_| "Discord public key is not configured.".to_owned())?;
    let signature = header_str(headers, DISCORD_SIGNATURE_HEADER)
        .ok_or_else(|| "Missing X-Signature-Ed25519 header.".to_owned())?;
    let timestamp = header_str(headers, DISCORD_TIMESTAMP_HEADER)
        .ok_or_else(|| "Missing X-Signature-Timestamp header.".to_owned())?;
    let key_bytes = decode_hex_fixed::<32>(&public_key)
        .map_err(|error| format!("Invalid Discord public key: {error}"))?;
    let signature_bytes = decode_hex_fixed::<64>(&signature)
        .map_err(|error| format!("Invalid Discord signature: {error}"))?;
    let key = VerifyingKey::from_bytes(&key_bytes)
        .map_err(|error| format!("Invalid Discord public key: {error}"))?;
    let signature = Signature::from_bytes(&signature_bytes);
    let mut signed = Vec::with_capacity(timestamp.len() + body.len());
    signed.extend_from_slice(timestamp.as_bytes());
    signed.extend_from_slice(body);
    key.verify(&signed, &signature)
        .map_err(|_| "invalid request signature".to_owned())
}

#[requires(true)]
#[ensures(true)]
fn header_str(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|bytes| bytes.len() == N) || ret.is_err())]
fn decode_hex_fixed<const N: usize>(input: &str) -> Result<[u8; N], String> {
    if input.len() != N * 2 {
        return Err(format!("expected {} hex characters", N * 2));
    }
    let mut bytes = [0u8; N];
    for (index, byte) in bytes.iter_mut().enumerate() {
        let start = index * 2;
        *byte = u8::from_str_radix(&input[start..start + 2], 16)
            .map_err(|error| format!("invalid hex at byte {index}: {error}"))?;
    }
    Ok(bytes)
}

#[requires(true)]
#[ensures(true)]
fn required_string_option(options: &[Value], name: &str) -> Result<String, String> {
    optional_string_option(options, name)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("Missing required option `{name}`."))
}

#[requires(true)]
#[ensures(true)]
fn optional_string_option(options: &[Value], name: &str) -> Option<String> {
    option_value(options, name)
        .and_then(Value::as_str)
        .map(str::to_owned)
}

#[requires(true)]
#[ensures(true)]
fn optional_bool_option(options: &[Value], name: &str) -> Option<bool> {
    option_value(options, name).and_then(Value::as_bool)
}

#[requires(true)]
#[ensures(true)]
fn optional_integer_option(options: &[Value], name: &str) -> Option<usize> {
    option_value(options, name)
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
}

#[requires(true)]
#[ensures(true)]
fn option_value<'a>(options: &'a [Value], name: &str) -> Option<&'a Value> {
    options
        .iter()
        .find(|option| option.get("name").and_then(Value::as_str) == Some(name))
        .and_then(|option| option.get("value"))
}

#[requires(true)]
#[ensures(true)]
fn parse_gentufa_format(value: Option<&str>) -> Result<ToolGentufaFormat, String> {
    match value.unwrap_or("tree") {
        "tree" => Ok(ToolGentufaFormat::Tree),
        "brackets" => Ok(ToolGentufaFormat::Brackets),
        "json" => Ok(ToolGentufaFormat::Json),
        "svg" => Ok(ToolGentufaFormat::Svg),
        "png" => Ok(ToolGentufaFormat::Png),
        other => Err(format!("Unknown gentufa format `{other}`.")),
    }
}

#[requires(true)]
#[ensures(true)]
fn parse_vlasei_format(value: Option<&str>) -> Result<ToolVlaseiFormat, String> {
    match value.unwrap_or("tree") {
        "tree" => Ok(ToolVlaseiFormat::Tree),
        "brackets" => Ok(ToolVlaseiFormat::Brackets),
        "ipa" => Ok(ToolVlaseiFormat::Ipa),
        "json" => Ok(ToolVlaseiFormat::Json),
        other => Err(format!("Unknown vlasei format `{other}`.")),
    }
}

#[requires(true)]
#[ensures(true)]
fn parse_vlacku_mode(value: Option<&str>) -> Result<ToolVlackuMode, String> {
    match value.unwrap_or("word") {
        "word" => Ok(ToolVlackuMode::Word),
        "rafsi" => Ok(ToolVlackuMode::Rafsi),
        "lujvo" => Ok(ToolVlackuMode::Lujvo),
        "sound" => Ok(ToolVlackuMode::Sound),
        "meaning" => Ok(ToolVlackuMode::Meaning),
        other => Err(format!("Unknown vlacku mode `{other}`.")),
    }
}

#[requires(true)]
#[ensures(true)]
fn parse_cukta_mode(value: Option<&str>) -> Result<ToolCuktaMode, String> {
    match value.unwrap_or("meaning") {
        "meaning" => Ok(ToolCuktaMode::Meaning),
        "word" => Ok(ToolCuktaMode::Word),
        "section" => Ok(ToolCuktaMode::Section),
        "example" => Ok(ToolCuktaMode::Example),
        "toc" => Ok(ToolCuktaMode::Toc),
        other => Err(format!("Unknown cukta mode `{other}`.")),
    }
}

#[requires(true)]
#[ensures(true)]
fn parse_cukta_format(value: Option<&str>) -> Result<jbotci_cli::ToolCuktaFormat, String> {
    match value.unwrap_or("markdown") {
        "markdown" => Ok(jbotci_cli::ToolCuktaFormat::Markdown),
        "html" => Ok(jbotci_cli::ToolCuktaFormat::Html),
        "raw" => Ok(jbotci_cli::ToolCuktaFormat::Raw),
        other => Err(format!("Unknown cukta format `{other}`.")),
    }
}

#[requires(true)]
#[ensures(true)]
fn parse_jvozba_mode(value: Option<&str>) -> Result<ToolJvozbaMode, String> {
    match value.unwrap_or("lujvo") {
        "lujvo" => Ok(ToolJvozbaMode::Lujvo),
        "cmevla" => Ok(ToolJvozbaMode::Cmevla),
        other => Err(format!("Unknown jvozba mode `{other}`.")),
    }
}

#[requires(true)]
#[ensures(true)]
fn parse_gimfihi_format(value: Option<&str>) -> Result<ToolGimfihiFormat, String> {
    match value.unwrap_or("table") {
        "table" => Ok(ToolGimfihiFormat::Table),
        "json" => Ok(ToolGimfihiFormat::Json),
        other => Err(format!("Unknown gimfi'i format `{other}`.")),
    }
}

#[requires(true)]
#[ensures(true)]
fn split_discord_words(input: &str) -> Vec<String> {
    input
        .split(|character: char| character.is_whitespace() || character == ',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .collect()
}

#[requires(true)]
#[ensures(true)]
fn split_discord_sources(input: &str) -> Vec<String> {
    input
        .split([',', ';'])
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .collect()
}

#[requires(path.starts_with('/'))]
#[ensures(ret.starts_with("http"))]
fn absolute_web_url(path: &str) -> String {
    let base = std::env::var("JBOTCI_PUBLIC_BASE_URL")
        .unwrap_or_else(|_| DEFAULT_PUBLIC_BASE_URL.to_owned());
    format!("{}{}", base.trim_end_matches('/'), path)
}

#[requires(true)]
#[ensures(true)]
fn json_response(status: StatusCode, value: Value) -> Response<Body> {
    Response::builder()
        .status(status)
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(value.to_string()))
        .expect("Discord JSON response builder is valid")
}

#[requires(true)]
#[ensures(true)]
fn plain_response(status: StatusCode, text: &str) -> Response<Body> {
    Response::builder()
        .status(status)
        .header(CONTENT_TYPE, "text/plain; charset=utf-8")
        .body(Body::from(text.to_owned()))
        .expect("Discord plain response builder is valid")
}
