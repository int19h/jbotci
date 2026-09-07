//! The Discord application: modules, the signed endpoint, and registration.
//!
//! Every interaction arrives at one URL. This module checks that Discord
//! really sent it, hands it to [`interaction::DiscordService`], and turns the
//! answer into an HTTP response. The command schema those interactions belong
//! to lives in [`schema`], and registering it is the one operation that
//! changes anything at Discord's end.

pub(crate) mod assemble;
pub(crate) mod codec;
pub(crate) mod components;
pub(crate) mod dedupe;
pub(crate) mod diagram;
pub(crate) mod interaction;
pub(crate) mod links;
pub(crate) mod locks;
pub(crate) mod modal;
pub(crate) mod operations;
pub(crate) mod present;
pub(crate) mod request;
pub(crate) mod schema;
pub(crate) mod transport;
pub(crate) mod work;

use std::sync::Arc;

use axum::body::{Body, Bytes};
use axum::extract::Extension;
use axum::http::header::CONTENT_TYPE;
use axum::http::{HeaderMap, Response, StatusCode};
#[allow(unused_imports)]
use bityzba::{ensures, invariant, new, requires};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde_json::{Value, json};

use crate::AppState;
use interaction::DiscordService;
use request::Snowflake;
use transport::DiscordApi;

const DISCORD_SIGNATURE_HEADER: &str = "x-signature-ed25519";
const DISCORD_TIMESTAMP_HEADER: &str = "x-signature-timestamp";
pub(crate) const DEFAULT_PUBLIC_BASE_URL: &str = "https://jbotci.app";
pub(crate) const DEFAULT_DISCORD_API_BASE: &str = "https://discord.com/api/v10";
pub(crate) const DISCORD_PUBLIC_KEY_ENV: &str = "DISCORD_PUBLIC_KEY";
pub(crate) const DISCORD_API_BASE_ENV: &str = "DISCORD_API_BASE";
pub(crate) const DISCORD_PUBLIC_BASE_URL_ENV: &str = "JBOTCI_PUBLIC_BASE_URL";
const DISCORD_APPLICATION_ID_ENV: &str = "DISCORD_APPLICATION_ID";
const DISCORD_BOT_TOKEN_ENV: &str = "DISCORD_BOT_TOKEN";

/// The interaction endpoint. A request Discord did not sign is refused before
/// anything is parsed, as Discord's own verification check requires.
#[requires(true)]
#[ensures(true)]
pub(crate) async fn discord_post(
    Extension(state): Extension<Arc<AppState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Response<Body> {
    let Some(service) = state.discord_service() else {
        return plain_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "This deployment has no Discord application configured.",
        );
    };
    if let Err(error) = verify_discord_signature(service.public_key(), &headers, &body) {
        return plain_response(StatusCode::UNAUTHORIZED, &error);
    }
    let Ok(value) = serde_json::from_slice::<Value>(&body) else {
        return plain_response(StatusCode::BAD_REQUEST, "Malformed interaction payload.");
    };
    json_response(StatusCode::OK, service.handle(&value).await.to_json())
}

/// The command schema this build registers.
#[requires(true)]
#[ensures(ret.get("name").and_then(Value::as_str) == Some("jbotci"))]
pub fn discord_command_registration() -> Value {
    schema::registration_payload()
}

/// Replace the application's commands with this build's schema. `guild` scopes
/// the registration to one guild, which is how a change is tried before it
/// reaches everyone; `dry_run` prints what would be sent and calls nothing.
#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
pub fn register_discord_commands_from_env(
    guild: Option<&str>,
    dry_run: bool,
) -> anyhow::Result<()> {
    let application_id = required_env(DISCORD_APPLICATION_ID_ENV)?;
    if !discord_snowflake_is_valid(&application_id) {
        anyhow::bail!("{DISCORD_APPLICATION_ID_ENV} must be a Discord snowflake");
    }
    let application_id = Snowflake::parse(&application_id)
        .map_err(|error| anyhow::anyhow!("{DISCORD_APPLICATION_ID_ENV}: {error}"))?;
    let guild = guild
        .map(|guild| Snowflake::parse(guild).map_err(|error| anyhow::anyhow!("--guild: {error}")))
        .transpose()?;
    let payload = discord_command_registration();
    if dry_run {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "application_id": application_id.as_str(),
                "guild_id": guild.as_ref().map(Snowflake::as_str),
                "commands": [payload],
            }))
            .map_err(|error| anyhow::anyhow!("could not render the registration: {error}"))?
        );
        return Ok(());
    }
    let bot_token = required_env(DISCORD_BOT_TOKEN_ENV)?;
    if !discord_authorization_token_is_valid(&bot_token) {
        anyhow::bail!("{DISCORD_BOT_TOKEN_ENV} must be non-empty visible ASCII without spaces");
    }
    let api = DiscordApi::new(&discord_api_base());
    api.overwrite_commands(&application_id, guild.as_ref(), &bot_token, &payload)
        .map(|_| ())
        .map_err(|error| anyhow::anyhow!("registering the jbotci command: {error}"))
}

#[requires(name.starts_with("DISCORD_"))]
#[ensures(ret.as_ref().is_ok_and(|value| !value.trim().is_empty()) || ret.is_err())]
fn required_env(name: &str) -> anyhow::Result<String> {
    let value = std::env::var(name).map_err(|_| anyhow::anyhow!("{name} is required"))?;
    let trimmed = value.trim();
    if trimmed.is_empty() {
        anyhow::bail!("{name} must not be empty");
    }
    Ok(trimmed.to_owned())
}

/// The Discord application this deployment answers for, when it is configured.
#[requires(true)]
#[ensures(true)]
pub(crate) fn configured_service(tools: crate::ToolServices) -> Option<Arc<DiscordService>> {
    let public_key = std::env::var(DISCORD_PUBLIC_KEY_ENV)
        .ok()
        .map(|key| key.trim().to_owned())
        .filter(|key| !key.is_empty())?;
    let config = new!(interaction::DiscordConfig {
        public_key,
        api_base: discord_api_base(),
        public_base_url: public_base_url(),
    });
    Some(Arc::new(DiscordService::new(config, tools)))
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn public_base_url() -> String {
    std::env::var(DISCORD_PUBLIC_BASE_URL_ENV)
        .ok()
        .map(|url| url.trim().trim_end_matches('/').to_owned())
        .filter(|url| !url.is_empty())
        .unwrap_or_else(|| DEFAULT_PUBLIC_BASE_URL.to_owned())
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.is_empty()))]
fn verify_discord_signature(
    public_key: &str,
    headers: &HeaderMap,
    body: &[u8],
) -> Result<(), String> {
    let signature = header_str(headers, DISCORD_SIGNATURE_HEADER)
        .ok_or_else(|| "Missing X-Signature-Ed25519 header.".to_owned())?;
    let timestamp = header_str(headers, DISCORD_TIMESTAMP_HEADER)
        .ok_or_else(|| "Missing X-Signature-Timestamp header.".to_owned())?;
    let key_bytes = decode_hex_fixed::<32>(public_key)
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

/// A Discord id: decimal digits, and short enough to be one.
#[requires(true)]
#[ensures(ret -> !value.is_empty())]
fn discord_snowflake_is_valid(value: &str) -> bool {
    !value.is_empty() && value.len() <= 20 && value.bytes().all(|byte| byte.is_ascii_digit())
}

/// A bot token, as far as an HTTP header is concerned.
#[requires(true)]
#[ensures(ret -> !value.trim().is_empty())]
fn discord_authorization_token_is_valid(value: &str) -> bool {
    !value.trim().is_empty() && value.bytes().all(|byte| byte.is_ascii_graphic())
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

#[requires(true)]
#[ensures(!ret.trim().is_empty())]
fn discord_api_base() -> String {
    std::env::var("DISCORD_API_BASE")
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_DISCORD_API_BASE.to_owned())
}
