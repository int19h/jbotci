//! Discord REST transport for interaction follow-ups and registration.
//!
//! Everything here is blocking (ureq) and is only ever called from the
//! blocking pool through the work governor, never on a Tokio worker.
//!
//! Failures are classified by what Discord may have done: [`TransportError::NotSent`]
//! and [`TransportError::Rejected`] mean the message is known unchanged;
//! [`TransportError::Ambiguous`] means the request may have been applied and
//! the caller must re-read the message before deciding anything.

use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

#[allow(unused_imports)]
use bityzba::{ensures, invariant, new, requires, try_new};
use serde_json::{Value, json};

use super::components::{FLAG_EPHEMERAL, MessagePayload};
use super::request::Snowflake;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(3);
const SEND_TIMEOUT: Duration = Duration::from_secs(10);
const RECEIVE_TIMEOUT: Duration = Duration::from_secs(15);
const GLOBAL_TIMEOUT: Duration = Duration::from_secs(30);
/// Discord's documented request ceiling; uploads are bounded well below it.
pub(crate) const MAX_REQUEST_BYTES: usize = 25 * 1024 * 1024;

static MULTIPART_COUNTER: AtomicU64 = AtomicU64::new(0);

/// The continuation token of one interaction. Discord tokens are opaque
/// URL-safe strings; the check keeps them out of path injection.
#[invariant(!value.is_empty() && value.len() <= 1024 && value.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-' || byte == b'.'))]
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct InteractionToken {
    value: String,
}

impl InteractionToken {
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|token| token.as_str() == value) || ret.is_err())]
    pub(crate) fn parse(value: &str) -> Result<Self, InvalidInteractionToken> {
        try_new!(InteractionToken {
            value: value.to_owned()
        })
        .map_err(|_| InvalidInteractionToken)
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub(crate) fn as_str(&self) -> &str {
        &self.value
    }
}

/// Tokens never appear in logs or error text.
impl fmt::Debug for InteractionToken {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("InteractionToken(<redacted>)")
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct InvalidInteractionToken;

impl fmt::Display for InvalidInteractionToken {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("interaction token is not URL-safe")
    }
}

/// How a Discord request failed, in terms of what the caller may assume.
#[invariant(::NotSent { .. } => true)]
#[invariant(::Rejected { .. } => true)]
#[invariant(::Ambiguous { .. } => true)]
#[invariant(::BadResponse { .. } => true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TransportError {
    /// Discord never received the request; nothing changed.
    NotSent { reason: String },
    /// Discord answered with an error status; nothing changed.
    Rejected { status: u16, body: String },
    /// The request was (possibly) delivered but no verdict arrived; the
    /// message may or may not have been updated.
    Ambiguous { reason: String },
    /// A response arrived but could not be understood.
    BadResponse { reason: String },
}

impl TransportError {
    /// Whether the request may have taken effect.
    #[requires(true)]
    #[ensures(ret == matches!(self, Self::Ambiguous { .. } | Self::BadResponse { .. }))]
    pub(crate) fn may_have_applied(&self) -> bool {
        matches!(self, Self::Ambiguous { .. } | Self::BadResponse { .. })
    }
}

impl fmt::Display for TransportError {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotSent { reason } => write!(formatter, "Discord was not reachable ({reason})"),
            Self::Rejected { status, body } => {
                write!(
                    formatter,
                    "Discord rejected the request (HTTP {status}: {body})"
                )
            }
            Self::Ambiguous { reason } => {
                write!(formatter, "no confirmation from Discord ({reason})")
            }
            Self::BadResponse { reason } => {
                write!(formatter, "unexpected Discord response ({reason})")
            }
        }
    }
}

impl std::error::Error for TransportError {}

/// Classify a ureq failure. Anything after the request may have left the
/// process is ambiguous; only resolution/connect-stage failures and explicit
/// rejections are known to have changed nothing.
#[requires(true)]
#[ensures(true)]
fn classify(error: ureq::Error) -> TransportError {
    use ureq::Timeout;
    match error {
        ureq::Error::Timeout(Timeout::Resolve) | ureq::Error::Timeout(Timeout::Connect) => {
            TransportError::NotSent {
                reason: "connection timed out".to_owned(),
            }
        }
        ureq::Error::Timeout(timeout) => TransportError::Ambiguous {
            reason: format!("timed out ({timeout:?})"),
        },
        ureq::Error::HostNotFound
        | ureq::Error::ConnectionFailed
        | ureq::Error::BadUri(_)
        | ureq::Error::Http(_)
        | ureq::Error::InvalidProxyUrl
        | ureq::Error::BodyExceedsLimit(_) => TransportError::NotSent {
            reason: error.to_string(),
        },
        ureq::Error::Io(io) if io.kind() == std::io::ErrorKind::ConnectionRefused => {
            TransportError::NotSent {
                reason: io.to_string(),
            }
        }
        ureq::Error::StatusCode(status) => TransportError::Rejected {
            status,
            body: String::new(),
        },
        other => TransportError::Ambiguous {
            reason: other.to_string(),
        },
    }
}

/// The response side of a Discord call once a status arrived.
#[requires(true)]
#[ensures(true)]
fn read_json_response(
    mut response: ureq::http::Response<ureq::Body>,
) -> Result<Value, TransportError> {
    let status = response.status().as_u16();
    let body = response
        .body_mut()
        .with_config()
        .limit(4 * 1024 * 1024)
        .read_to_string()
        .map_err(|error| TransportError::BadResponse {
            reason: format!("could not read the response body: {error}"),
        })?;
    if !(200..300).contains(&status) {
        return Err(TransportError::Rejected { status, body });
    }
    if body.trim().is_empty() {
        return Ok(Value::Null);
    }
    serde_json::from_str(&body).map_err(|error| TransportError::BadResponse {
        reason: format!("response is not JSON: {error}"),
    })
}

/// A multipart/form-data body carrying `payload_json` plus `files[n]` parts.
#[invariant(!boundary.is_empty() && boundary.is_ascii())]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MultipartBody {
    pub(crate) boundary: String,
    pub(crate) bytes: Vec<u8>,
}

impl MultipartBody {
    /// Build the body. The boundary is regenerated until it occurs in no part,
    /// so arbitrary upload bytes can never terminate the form early.
    #[requires(true)]
    #[ensures(ret.bytes.ends_with(b"--\r\n"))]
    pub(crate) fn build(payload_json: &str, uploads: &[(usize, &str, &str, &[u8])]) -> Self {
        let mut attempt = 0u32;
        loop {
            let counter = MULTIPART_COUNTER.fetch_add(1, Ordering::Relaxed);
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|elapsed| elapsed.as_nanos())
                .unwrap_or(0);
            let boundary = format!(
                "----jbotci{:x}{:x}{:x}{:x}",
                std::process::id(),
                counter,
                nanos,
                attempt
            );
            let collides = payload_json
                .as_bytes()
                .windows(boundary.len())
                .any(|window| window == boundary.as_bytes())
                || uploads.iter().any(|(_, name, content_type, bytes)| {
                    name.contains(&boundary)
                        || content_type.contains(&boundary)
                        || bytes
                            .windows(boundary.len())
                            .any(|window| window == boundary.as_bytes())
                });
            if collides {
                attempt += 1;
                continue;
            }
            let mut bytes = Vec::new();
            bytes.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
            bytes.extend_from_slice(
                b"Content-Disposition: form-data; name=\"payload_json\"\r\nContent-Type: application/json\r\n\r\n",
            );
            bytes.extend_from_slice(payload_json.as_bytes());
            bytes.extend_from_slice(b"\r\n");
            for (index, name, content_type, data) in uploads {
                bytes.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
                bytes.extend_from_slice(
                    format!(
                        "Content-Disposition: form-data; name=\"files[{index}]\"; filename=\"{name}\"\r\nContent-Type: {content_type}\r\n\r\n"
                    )
                    .as_bytes(),
                );
                bytes.extend_from_slice(data);
                bytes.extend_from_slice(b"\r\n");
            }
            bytes.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
            return new!(MultipartBody { boundary, bytes });
        }
    }

    #[requires(true)]
    #[ensures(ret.starts_with("multipart/form-data; boundary="))]
    pub(crate) fn content_type(&self) -> String {
        format!("multipart/form-data; boundary={}", self.boundary)
    }
}

/// Blocking Discord API client bound to one API base URL.
#[invariant(!base_url.is_empty() && !base_url.ends_with('/'))]
#[derive(Debug, Clone)]
pub(crate) struct DiscordApi {
    base_url: String,
    agent: ureq::Agent,
}

impl DiscordApi {
    #[requires(!base_url.trim().is_empty())]
    #[ensures(true)]
    pub(crate) fn new(base_url: &str) -> Self {
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .timeout_connect(Some(CONNECT_TIMEOUT))
            .timeout_send_request(Some(SEND_TIMEOUT))
            .timeout_send_body(Some(SEND_TIMEOUT))
            .timeout_recv_response(Some(RECEIVE_TIMEOUT))
            .timeout_recv_body(Some(RECEIVE_TIMEOUT))
            .timeout_global(Some(GLOBAL_TIMEOUT))
            .max_redirects(0)
            .build()
            .into();
        new!(DiscordApi {
            base_url: base_url.trim().trim_end_matches('/').to_owned(),
            agent,
        })
    }

    #[requires(true)]
    #[ensures(ret.starts_with(&self.base_url))]
    fn original_message_url(&self, application_id: &Snowflake, token: &InteractionToken) -> String {
        format!(
            "{}/webhooks/{}/{}/messages/@original",
            self.base_url,
            application_id,
            token.as_str()
        )
    }

    /// `GET /webhooks/{app}/{token}/messages/@original`: the message the
    /// interaction belongs to, as Discord currently has it.
    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn get_original(
        &self,
        application_id: &Snowflake,
        token: &InteractionToken,
    ) -> Result<Value, TransportError> {
        let url = self.original_message_url(application_id, token);
        let response = self.agent.get(&url).call().map_err(classify)?;
        read_json_response(response)
    }

    /// `PATCH /webhooks/{app}/{token}/messages/@original` with the full
    /// payload; multipart when files are uploaded. Returns the edited message.
    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn edit_original(
        &self,
        application_id: &Snowflake,
        token: &InteractionToken,
        payload: &MessagePayload,
    ) -> Result<Value, TransportError> {
        let url = self.original_message_url(application_id, token);
        let json = payload.to_json().to_string();
        let uploads = payload.uploads();
        let response = if uploads.is_empty() {
            self.agent
                .patch(&url)
                .header("content-type", "application/json")
                .send(json.as_bytes())
        } else {
            let parts = uploads
                .iter()
                .map(|(index, name, content_type, bytes)| {
                    (*index, name.as_str(), *content_type, *bytes)
                })
                .collect::<Vec<_>>();
            let body = MultipartBody::build(&json, &parts);
            if body.bytes.len() > MAX_REQUEST_BYTES {
                return Err(TransportError::NotSent {
                    reason: format!(
                        "request body of {} bytes exceeds Discord's limit",
                        body.bytes.len()
                    ),
                });
            }
            self.agent
                .patch(&url)
                .header("content-type", &body.content_type())
                .send(body.bytes.as_slice())
        }
        .map_err(classify)?;
        read_json_response(response)
    }

    /// `POST /webhooks/{app}/{token}`: a private follow-up to the acting user.
    #[requires(!content.trim().is_empty())]
    #[ensures(true)]
    pub(crate) fn create_ephemeral_followup(
        &self,
        application_id: &Snowflake,
        token: &InteractionToken,
        content: &str,
    ) -> Result<(), TransportError> {
        let url = format!(
            "{}/webhooks/{}/{}",
            self.base_url,
            application_id,
            token.as_str()
        );
        let body = json!({
            "content": content,
            "flags": FLAG_EPHEMERAL,
            "allowed_mentions": { "parse": [] },
        })
        .to_string();
        let response = self
            .agent
            .post(&url)
            .header("content-type", "application/json")
            .send(body.as_bytes())
            .map_err(classify)?;
        read_json_response(response).map(|_| ())
    }

    /// Download an attachment from a URL Discord supplied in this interaction,
    /// bounded by `byte_cap` and `timeout` (which also covers body transfer).
    #[requires(byte_cap > 0)]
    #[ensures(ret.as_ref().is_ok_and(|bytes| bytes.len() <= byte_cap) || ret.is_err())]
    pub(crate) fn fetch_attachment(
        &self,
        url: &str,
        byte_cap: usize,
        timeout: Duration,
    ) -> Result<Vec<u8>, TransportError> {
        let mut response = self
            .agent
            .get(url)
            .config()
            .timeout_global(Some(timeout))
            .build()
            .call()
            .map_err(classify)?;
        let status = response.status().as_u16();
        if !(200..300).contains(&status) {
            return Err(TransportError::Rejected {
                status,
                body: String::new(),
            });
        }
        let bytes = response
            .body_mut()
            .with_config()
            .limit(byte_cap as u64 + 1)
            .read_to_vec()
            .map_err(|error| TransportError::BadResponse {
                reason: format!("could not read the attachment: {error}"),
            })?;
        if bytes.len() > byte_cap {
            return Err(TransportError::BadResponse {
                reason: format!("attachment exceeds {byte_cap} bytes"),
            });
        }
        Ok(bytes)
    }

    /// `PUT /applications/{app}/commands` (or the guild variant): bulk
    /// overwrite the application's commands with `payload`.
    #[requires(!bot_token.trim().is_empty())]
    #[ensures(true)]
    pub(crate) fn overwrite_commands(
        &self,
        application_id: &Snowflake,
        guild_id: Option<&Snowflake>,
        bot_token: &str,
        payload: &Value,
    ) -> Result<Value, TransportError> {
        let url = match guild_id {
            Some(guild) => format!(
                "{}/applications/{}/guilds/{}/commands",
                self.base_url, application_id, guild
            ),
            None => format!("{}/applications/{}/commands", self.base_url, application_id),
        };
        let body = json!([payload]).to_string();
        let response = self
            .agent
            .put(&url)
            .header("content-type", "application/json")
            .header("authorization", &format!("Bot {bot_token}"))
            .send(body.as_bytes())
            .map_err(classify)?;
        read_json_response(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn timeouts_before_sending_are_not_sent_and_later_ones_are_ambiguous() {
        assert!(matches!(
            classify(ureq::Error::Timeout(ureq::Timeout::Connect)),
            TransportError::NotSent { .. }
        ));
        assert!(matches!(
            classify(ureq::Error::Timeout(ureq::Timeout::Resolve)),
            TransportError::NotSent { .. }
        ));
        for timeout in [
            ureq::Timeout::SendRequest,
            ureq::Timeout::SendBody,
            ureq::Timeout::RecvResponse,
            ureq::Timeout::RecvBody,
            ureq::Timeout::Global,
        ] {
            let error = classify(ureq::Error::Timeout(timeout));
            assert!(error.may_have_applied(), "{timeout:?}: {error}");
        }
        assert!(!classify(ureq::Error::HostNotFound).may_have_applied());
        assert!(!classify(ureq::Error::ConnectionFailed).may_have_applied());
        assert!(
            !classify(ureq::Error::Io(std::io::Error::from(
                std::io::ErrorKind::ConnectionRefused
            )))
            .may_have_applied()
        );
        assert!(
            classify(ureq::Error::Io(std::io::Error::from(
                std::io::ErrorKind::ConnectionReset
            )))
            .may_have_applied()
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn multipart_body_is_well_formed_and_avoids_boundary_collisions() {
        let payload = r#"{"attachments":[{"id":0,"filename":"a.png"}]}"#;
        let image = vec![0x89, b'P', b'N', b'G', b'\r', b'\n'];
        let body = MultipartBody::build(payload, &[(0, "a.png", "image/png", &image)]);
        let text = String::from_utf8_lossy(&body.bytes);
        assert!(text.starts_with(&format!(
            "--{}\r\nContent-Disposition: form-data; name=\"payload_json\"",
            body.boundary
        )));
        assert!(
            text.contains(
                "name=\"files[0]\"; filename=\"a.png\"\r\nContent-Type: image/png\r\n\r\n"
            )
        );
        assert!(text.ends_with(&format!("--{}--\r\n", body.boundary)));
        assert_eq!(
            body.content_type(),
            format!("multipart/form-data; boundary={}", body.boundary)
        );
        // The payload and the file must not contain the boundary.
        assert_eq!(payload.matches(&body.boundary).count(), 0);
        let first = MultipartBody::build(payload, &[]);
        let second = MultipartBody::build(payload, &[]);
        assert_ne!(
            first.boundary, second.boundary,
            "boundaries are unique per request"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn tokens_are_validated_and_redacted() {
        assert!(InteractionToken::parse("aW50ZXJhY3Rpb24.abc-DEF_123").is_ok());
        assert!(InteractionToken::parse("bad/token").is_err());
        assert!(InteractionToken::parse("").is_err());
        let token = InteractionToken::parse("secret-token").expect("token");
        assert_eq!(format!("{token:?}"), "InteractionToken(<redacted>)");
        let api = DiscordApi::new("https://discord.com/api/v10/");
        assert_eq!(
            api.original_message_url(&Snowflake::parse("1").expect("id"), &token),
            "https://discord.com/api/v10/webhooks/1/secret-token/messages/@original"
        );
    }
}
