//! Typed TOML configuration for xarsnu runs.

use std::time::Duration;

#[allow(unused_imports)]
use bityzba::{ensures, invariant, requires};
use serde::{Deserialize, Serialize};
use thiserror::Error;

const DEFAULT_MAX_REFERENCE_CALLS_PER_PHASE: usize = 16;
const DEFAULT_REFERENCE_NUDGE_AFTER: usize = 6;
const DEFAULT_HTTP_TIMEOUT_SECONDS: u64 = 60;

#[requires(true)]
#[ensures(ret == DEFAULT_MAX_REFERENCE_CALLS_PER_PHASE)]
const fn default_max_reference_calls_per_phase() -> usize {
    DEFAULT_MAX_REFERENCE_CALLS_PER_PHASE
}

#[requires(true)]
#[ensures(ret)]
const fn default_reference_dedupe() -> bool {
    true
}

#[requires(true)]
#[ensures(ret == DEFAULT_REFERENCE_NUDGE_AFTER)]
const fn default_reference_nudge_after() -> usize {
    DEFAULT_REFERENCE_NUDGE_AFTER
}

#[requires(true)]
#[ensures(ret == DEFAULT_HTTP_TIMEOUT_SECONDS)]
const fn default_http_timeout_seconds() -> u64 {
    DEFAULT_HTTP_TIMEOUT_SECONDS
}

/// Whether xarsnu should manage provider prompt-cache breakpoints.
#[invariant(::Auto => true)]
#[invariant(::Off => true)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PromptCaching {
    /// Enable explicit breakpoints only for models known to require them.
    #[default]
    Auto,
    /// Never add prompt-cache breakpoints for this participant.
    Off,
}

/// How the provider should enforce selection from the offered tool set.
#[invariant(::Required => true)]
#[invariant(::Auto => true)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolChoice {
    /// Require a tool call structurally at the provider boundary.
    #[default]
    Required,
    /// Allow prose so thinking-mode providers can be corrected by the protocol.
    Auto,
}

/// One model participating in the private side of a simulated discussion.
#[invariant(!name.trim().is_empty(), "participant names cannot be empty")]
#[invariant(!model.trim().is_empty(), "participant model ids cannot be empty")]
#[invariant(temperature.is_finite() && (0.0..=2.0).contains(temperature), "temperature must be finite and between 0 and 2")]
#[invariant(!system_prompt.trim().is_empty(), "participant system prompts cannot be empty")]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct ParticipantConfig {
    pub name: String,
    pub model: String,
    #[serde(default)]
    pub prompt_caching: PromptCaching,
    #[serde(default)]
    pub tool_choice: ToolChoice,
    pub temperature: f64,
    pub system_prompt: String,
}

/// Hard limits that make a run bounded and reviewable.
#[invariant(*max_parse_attempts_per_turn > 0, "parse-attempt cap must be positive")]
#[invariant(*max_intent_revisions_per_turn > 0, "intent-revision cap must be positive")]
#[invariant(*max_turns > 0, "turn cap must be positive")]
#[invariant(max_cost_usd.is_finite() && *max_cost_usd > 0.0, "cost cap must be finite and positive")]
#[invariant(*max_reference_calls_per_phase > 0, "reference-call cap must be positive")]
#[invariant(*reference_nudge_after > 0, "reference nudge threshold must be positive")]
#[invariant(*reference_nudge_after < *max_reference_calls_per_phase, "reference nudge threshold must be less than the reference-call cap")]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct CapsConfig {
    pub max_parse_attempts_per_turn: usize,
    pub max_intent_revisions_per_turn: usize,
    pub max_turns: usize,
    pub max_cost_usd: f64,
    #[serde(default = "default_max_reference_calls_per_phase")]
    pub max_reference_calls_per_phase: usize,
    #[serde(default = "default_reference_dedupe")]
    pub reference_dedupe: bool,
    #[serde(default = "default_reference_nudge_after")]
    pub reference_nudge_after: usize,
}

/// HTTP settings for OpenRouter requests made during one run.
#[invariant(*http_timeout_seconds > 0, "HTTP timeout must be positive")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct ClientConfig {
    #[serde(default = "default_http_timeout_seconds")]
    pub http_timeout_seconds: u64,
}

impl Default for ClientConfig {
    #[requires(true)]
    #[ensures(ret.http_timeout_seconds == DEFAULT_HTTP_TIMEOUT_SECONDS)]
    fn default() -> Self {
        Self::from_data(bityzba::data!(ClientConfig {
            http_timeout_seconds: DEFAULT_HTTP_TIMEOUT_SECONDS,
        }))
    }
}

impl ClientConfig {
    /// Convert the serialized whole-second timeout into the HTTP client's type.
    #[requires(true)]
    #[ensures(ret > Duration::ZERO)]
    pub fn http_timeout(&self) -> Duration {
        Duration::from_secs(self.http_timeout_seconds)
    }
}

/// Semantic rendering selected for the jbotci gate.
#[invariant(::TreeProj => true)]
#[invariant(::Tree => true)]
#[invariant(::Json => true)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TersmuFormat {
    #[default]
    #[serde(rename = "tree+proj")]
    TreeProj,
    Tree,
    Json,
}

/// Complete configuration for one xarsnu run.
#[invariant(participants.len() >= 2, "a discussion requires at least two participants")]
#[invariant(!scenario.trim().is_empty(), "scenario reference cannot be empty")]
#[invariant(participants.iter().enumerate().all(|(index, participant)| participants[..index].iter().all(|earlier| earlier.name != participant.name)), "participant names must be unique")]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct RunConfig {
    pub participants: Vec<ParticipantConfig>,
    pub scenario: String,
    pub caps: CapsConfig,
    #[serde(default)]
    pub client: ClientConfig,
    #[serde(default)]
    pub tersmu_format: TersmuFormat,
}

impl RunConfig {
    /// Parse and validate one TOML configuration document.
    #[requires(true)]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    pub fn from_toml(source: &str) -> Result<Self, ConfigError> {
        toml::from_str(source).map_err(ConfigError::Toml)
    }
}

/// A configuration document could not be parsed or violated a type invariant.
#[derive(Debug, Error)]
#[invariant(true)]
#[invariant(::Toml(_) => true)]
pub enum ConfigError {
    #[error("invalid xarsnu configuration: {0}")]
    Toml(#[source] toml::de::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_CONFIG: &str = r#"
scenario = "schedule-negotiation"

[caps]
max-parse-attempts-per-turn = 3
max-intent-revisions-per-turn = 2
max-turns = 8
max-cost-usd = 1.25

[[participants]]
name = "alice"
model = "example/alice"
temperature = 0.4
system-prompt = "Speak only Lojban."

[[participants]]
name = "bob"
model = "example/bob"
temperature = 0.6
system-prompt = "Speak only Lojban."
"#;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn config_defaults_to_tree_proj() {
        let config = RunConfig::from_toml(VALID_CONFIG).expect("valid config");
        assert_eq!(config.tersmu_format, TersmuFormat::TreeProj);
        assert_eq!(config.caps.max_reference_calls_per_phase, 16);
        assert!(config.caps.reference_dedupe);
        assert_eq!(config.caps.reference_nudge_after, 6);
        assert_eq!(config.client.http_timeout(), Duration::from_secs(60));
        assert!(
            config
                .participants
                .iter()
                .all(|participant| participant.prompt_caching == PromptCaching::Auto)
        );
        assert!(
            config
                .participants
                .iter()
                .all(|participant| participant.tool_choice == ToolChoice::Required)
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn participant_can_disable_prompt_caching() {
        let source = VALID_CONFIG.replace(
            "model = \"example/alice\"",
            "model = \"example/alice\"\nprompt-caching = \"off\"",
        );
        let config = RunConfig::from_toml(&source).expect("valid config");
        assert_eq!(config.participants[0].prompt_caching, PromptCaching::Off);
        assert_eq!(config.participants[1].prompt_caching, PromptCaching::Auto);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn participant_can_select_automatic_tool_choice() {
        let source = VALID_CONFIG.replace(
            "model = \"example/alice\"",
            "model = \"example/alice\"\ntool-choice = \"auto\"",
        );
        let config = RunConfig::from_toml(&source).expect("valid config");
        assert_eq!(config.participants[0].tool_choice, ToolChoice::Auto);
        assert_eq!(config.participants[1].tool_choice, ToolChoice::Required);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn client_http_timeout_is_configurable_and_positive() {
        let configured = VALID_CONFIG.replace(
            "scenario = \"schedule-negotiation\"",
            "scenario = \"schedule-negotiation\"\n\n[client]\nhttp-timeout-seconds = 180",
        );
        let config = RunConfig::from_toml(&configured).expect("valid client timeout");
        assert_eq!(config.client.http_timeout(), Duration::from_secs(180));

        let invalid = configured.replace("http-timeout-seconds = 180", "http-timeout-seconds = 0");
        let error = RunConfig::from_toml(&invalid).expect_err("zero timeout must be rejected");
        assert!(error.to_string().contains("HTTP timeout must be positive"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn participant_private_brief_is_rejected_as_an_unknown_field() {
        let invalid = VALID_CONFIG.replacen(
            "system-prompt = \"Speak only Lojban.\"",
            "system-prompt = \"Speak only Lojban.\"\nprivate-brief = \"obsolete\"",
            1,
        );
        let error = RunConfig::from_toml(&invalid).expect_err("removed field must be rejected");
        assert!(matches!(error, ConfigError::Toml(_)));
        assert!(error.to_string().contains("unknown field `private-brief`"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn invalid_config_is_a_typed_error() {
        let invalid = VALID_CONFIG.replace("max-turns = 8", "max-turns = 0");
        let error = RunConfig::from_toml(&invalid).expect_err("zero cap must be rejected");
        assert!(matches!(error, ConfigError::Toml(_)));
        assert!(error.to_string().contains("turn cap must be positive"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn reference_loop_knobs_are_typed_and_validated_together() {
        let configured = VALID_CONFIG.replace(
            "max-cost-usd = 1.25",
            "max-cost-usd = 1.25\nmax-reference-calls-per-phase = 9\nreference-dedupe = false\nreference-nudge-after = 4",
        );
        let config = RunConfig::from_toml(&configured).expect("valid reference caps");
        assert_eq!(config.caps.max_reference_calls_per_phase, 9);
        assert!(!config.caps.reference_dedupe);
        assert_eq!(config.caps.reference_nudge_after, 4);

        for (field, value, expected) in [
            (
                "max-reference-calls-per-phase",
                "0",
                "reference-call cap must be positive",
            ),
            (
                "reference-nudge-after",
                "0",
                "reference nudge threshold must be positive",
            ),
            (
                "reference-nudge-after",
                "16",
                "reference nudge threshold must be less than the reference-call cap",
            ),
        ] {
            let invalid = VALID_CONFIG.replace(
                "max-cost-usd = 1.25",
                &format!("max-cost-usd = 1.25\n{field} = {value}"),
            );
            let error = RunConfig::from_toml(&invalid).expect_err("invalid reference cap");
            assert!(error.to_string().contains(expected), "{error}");
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn duplicate_participant_names_are_rejected() {
        let invalid = VALID_CONFIG.replace("name = \"bob\"", "name = \"alice\"");
        let error = RunConfig::from_toml(&invalid).expect_err("duplicate names must be rejected");
        assert!(
            error
                .to_string()
                .contains("participant names must be unique")
        );
    }
}
