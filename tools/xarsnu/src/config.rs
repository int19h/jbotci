//! Typed TOML configuration for xarsnu runs.

use std::time::Duration;

#[allow(unused_imports)]
use bityzba::{data, ensures, invariant, requires};
use serde::{Deserialize, Deserializer, Serialize};
use thiserror::Error;

const DEFAULT_MAX_REFERENCE_CALLS_PER_PHASE: usize = 30;
const DEFAULT_REFERENCE_NUDGE_AFTER: usize = 10;
const DEFAULT_HTTP_TIMEOUT_SECONDS: u64 = 60;
const DEFAULT_MAX_COMPLETION_TOKENS: u32 = 16_384;

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

#[requires(true)]
#[ensures(ret == DEFAULT_MAX_COMPLETION_TOKENS)]
const fn default_max_completion_tokens() -> u32 {
    DEFAULT_MAX_COMPLETION_TOKENS
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
#[invariant(::Metadata => true)]
#[invariant(::Required => true)]
#[invariant(::Auto => true)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolChoice {
    /// Select required calls or bounded railroading from vendored model metadata.
    #[default]
    Metadata,
    /// Require a tool call structurally at the provider boundary.
    Required,
    /// Allow prose so thinking-mode providers can be corrected by the protocol.
    Auto,
}

/// Reasoning effort requested from OpenRouter for one participant.
#[invariant(::Off => true)]
#[invariant(::Default => true)]
#[invariant(::Low => true)]
#[invariant(::Medium => true)]
#[invariant(::High => true)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReasoningConfig {
    /// Disable model reasoning.
    Off,
    /// Let OpenRouter select the provider's default reasoning effort.
    #[default]
    Default,
    /// Request low reasoning effort.
    Low,
    /// Request medium reasoning effort.
    Medium,
    /// Request high reasoning effort.
    High,
}

#[invariant(::Named(_) => true)]
#[invariant(::LegacyDisabled(_) => true)]
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum ReasoningConfigWire {
    Named(ReasoningConfig),
    LegacyDisabled(bool),
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn deserialize_reasoning_config<'de, D>(
    deserializer: D,
) -> Result<Option<ReasoningConfig>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(
        Option::<ReasoningConfigWire>::deserialize(deserializer)?.map(|wire| match wire {
            ReasoningConfigWire::Named(reasoning) => reasoning,
            ReasoningConfigWire::LegacyDisabled(true) => ReasoningConfig::Off,
            ReasoningConfigWire::LegacyDisabled(false) => ReasoningConfig::Default,
        }),
    )
}

/// One model participating in the private side of a simulated discussion.
#[invariant(!name.trim().is_empty(), "participant names cannot be empty")]
#[invariant(!model.trim().is_empty(), "participant model ids cannot be empty")]
#[invariant(temperature.is_finite() && (0.0..=2.0).contains(temperature), "temperature must be finite and between 0 and 2")]
#[invariant(!system_prompt.trim().is_empty(), "participant system prompts cannot be empty")]
#[invariant(max_completion_tokens.is_none_or(|value| value > 0), "participant completion token limit must be positive")]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct ParticipantConfig {
    pub name: String,
    pub model: String,
    /// Opaque OpenRouter provider-routing options, serialized without schema modeling.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<toml::Table>,
    #[serde(default)]
    pub prompt_caching: PromptCaching,
    #[serde(default)]
    pub tool_choice: ToolChoice,
    /// Explicit reasoning policy; absent values follow model capability metadata.
    #[serde(
        default,
        alias = "disable-reasoning",
        deserialize_with = "deserialize_reasoning_config"
    )]
    pub reasoning: Option<ReasoningConfig>,
    pub temperature: f64,
    pub system_prompt: String,
    /// Per-participant completion token limit override; the `[client]` default
    /// when absent (issue #726).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_completion_tokens: Option<u32>,
}

impl ParticipantConfig {
    /// This participant's completion token limit: its own override when set,
    /// otherwise the run-wide `[client]` default (issue #726).
    #[requires(client_default > 0)]
    #[ensures(ret > 0)]
    #[ensures(ret == self.max_completion_tokens.unwrap_or(client_default))]
    pub fn effective_max_completion_tokens(&self, client_default: u32) -> u32 {
        self.max_completion_tokens.unwrap_or(client_default)
    }
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
#[invariant(base_url.as_ref().is_none_or(|value| !value.trim().is_empty()), "client base URL cannot be empty")]
#[invariant(*http_timeout_seconds > 0, "HTTP timeout must be positive")]
#[invariant(*max_completion_tokens > 0, "completion token limit must be positive")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct ClientConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    #[serde(default = "default_http_timeout_seconds")]
    pub http_timeout_seconds: u64,
    /// Run-wide completion token limit; per-participant overrides win (issue #726).
    #[serde(default = "default_max_completion_tokens")]
    pub max_completion_tokens: u32,
}

impl Default for ClientConfig {
    #[requires(true)]
    #[ensures(ret.base_url.is_none())]
    #[ensures(ret.http_timeout_seconds == DEFAULT_HTTP_TIMEOUT_SECONDS)]
    #[ensures(ret.max_completion_tokens == DEFAULT_MAX_COMPLETION_TOKENS)]
    fn default() -> Self {
        Self::from_data(bityzba::data!(ClientConfig {
            base_url: None,
            http_timeout_seconds: DEFAULT_HTTP_TIMEOUT_SECONDS,
            max_completion_tokens: DEFAULT_MAX_COMPLETION_TOKENS,
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

/// One argv-style external command invocation used to render tersmu JSON.
///
/// The first element is the executable and every remaining element is passed
/// to it literally, without invoking a shell. The command receives the
/// production tersmu JSON graph on stdin and must write its rendering to
/// stdout.
#[invariant(self.first().is_some_and(|program| !program.trim().is_empty()), "external renderer command must name a non-empty executable")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ExternalRendererCommand(Vec<String>);

impl ExternalRendererCommand {
    /// The executable to spawn.
    #[requires(true)]
    #[ensures(!ret.trim().is_empty())]
    pub fn program(&self) -> &str {
        self.first()
            .expect("invariant guarantees a nonempty command")
    }

    /// Literal arguments passed to the executable in configured order.
    #[requires(true)]
    #[ensures(true)]
    pub fn args(&self) -> &[String] {
        &self.as_slice()[1..]
    }
}

/// Semantic rendering selected for the jbotci gate.
#[invariant(::Json => true)]
#[invariant(::Smusni => true)]
#[invariant(::Xml => true)]
#[invariant(::External(_) => true)]
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TersmuFormat {
    /// Model-facing `smusni` notation: a flat, self-describing declaration
    /// listing of the semantic graph (the default).
    #[default]
    Smusni,
    Json,
    /// Canonical scoped SFN-XML rendering produced by the in-product renderer.
    Xml,
    /// Pipe the JSON graph through a caller-configured renderer.
    External(ExternalRendererCommand),
}

/// Adversarial fresh-session meaning review of accepted candidates (issue #723).
///
/// When enabled, the speaker's self-confirmation is replaced by a separate
/// reviewer session on the same model that adversarially verifies the tersmu
/// rendering against the registered intent. The reviewer inherits the
/// participant's temperature and reasoning policy unless overridden here.
#[invariant(temperature.is_none_or(|value| value.is_finite() && (0.0..=2.0).contains(&value)), "reviewer temperature must be finite and between 0 and 2")]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct MeaningReviewConfig {
    /// Replace speaker self-confirmation with the adversarial reviewer session.
    #[serde(default)]
    pub enabled: bool,
    /// Reviewer temperature override; the participant's temperature when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    /// Reviewer reasoning override; the participant's reasoning policy when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<ReasoningConfig>,
}

impl Default for MeaningReviewConfig {
    #[requires(true)]
    #[ensures(!ret.enabled && ret.temperature.is_none() && ret.reasoning.is_none())]
    fn default() -> Self {
        Self::from_data(bityzba::data!(MeaningReviewConfig {
            enabled: false,
            temperature: None,
            reasoning: None,
        }))
    }
}

/// Information available when a listener first interprets a posted message.
#[invariant(::Informed => true)]
#[invariant(::BlindThenReveal => true)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ListenerMode {
    /// Present the Lojban, tersmu rendering, and embedded definitions together.
    #[default]
    Informed,
    /// Commit an interpretation from Lojban alone before revealing tersmu.
    BlindThenReveal,
}

/// Historical schema-v1 transcripts predate informed listeners and were blind.
#[requires(true)]
#[ensures(ret == ListenerMode::BlindThenReveal)]
fn historical_transcript_listener_mode() -> ListenerMode {
    ListenerMode::BlindThenReveal
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
    #[serde(default = "historical_transcript_listener_mode")]
    pub listener_mode: ListenerMode,
    /// Continue after a failed semantic-search preflight, with a warning event.
    #[serde(default)]
    pub allow_degraded_search: bool,
    /// Adversarial fresh-session meaning review; disabled unless explicitly enabled.
    #[serde(default)]
    pub meaning_review: MeaningReviewConfig,
}

impl RunConfig {
    /// Parse and validate one TOML configuration document.
    #[requires(true)]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    pub fn from_toml(source: &str) -> Result<Self, ConfigError> {
        let table = toml::from_str::<toml::Table>(source).map_err(ConfigError::Toml)?;
        let listener_mode_was_explicit = table.contains_key("listener-mode");
        let config = toml::from_str::<Self>(source).map_err(ConfigError::Toml)?;
        Ok(if listener_mode_was_explicit {
            config
        } else {
            config.with_data(data! {
                listener_mode: ListenerMode::Informed,
            })
        })
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
    fn config_defaults_to_smusni() {
        let config = RunConfig::from_toml(VALID_CONFIG).expect("valid config");
        assert_eq!(config.tersmu_format, TersmuFormat::Smusni);
        assert_eq!(config.listener_mode, ListenerMode::Informed);
        assert!(!config.allow_degraded_search);
        assert_eq!(config.meaning_review, MeaningReviewConfig::default());
        assert!(!config.meaning_review.enabled);
        assert_eq!(config.caps.max_reference_calls_per_phase, 30);
        assert!(config.caps.reference_dedupe);
        assert_eq!(config.caps.reference_nudge_after, 10);
        assert_eq!(config.client.base_url, None);
        assert_eq!(config.client.http_timeout(), Duration::from_secs(60));
        assert_eq!(config.client.max_completion_tokens, 16_384);
        assert!(
            config
                .participants
                .iter()
                .all(|participant| participant.max_completion_tokens.is_none())
        );
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
                .all(|participant| participant.provider.is_none())
        );
        assert!(
            config
                .participants
                .iter()
                .all(|participant| participant.tool_choice == ToolChoice::Metadata)
        );
        assert!(
            config
                .participants
                .iter()
                .all(|participant| participant.reasoning.is_none())
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn tersmu_format_is_explicitly_selectable() {
        for (configured, expected) in [
            ("json", TersmuFormat::Json),
            ("smusni", TersmuFormat::Smusni),
            ("xml", TersmuFormat::Xml),
        ] {
            let source = VALID_CONFIG.replace(
                "scenario = \"schedule-negotiation\"",
                &format!("scenario = \"schedule-negotiation\"\ntersmu-format = \"{configured}\""),
            );
            let config = RunConfig::from_toml(&source).expect("valid tersmu format");
            assert_eq!(config.tersmu_format, expected);
        }

        let invalid = VALID_CONFIG.replace(
            "scenario = \"schedule-negotiation\"",
            "scenario = \"schedule-negotiation\"\ntersmu-format = \"lean4\"",
        );
        assert!(RunConfig::from_toml(&invalid).is_err());

        // The `smusni` format renamed the earlier `lean3` working name, and the
        // legacy `tree` / `tree+proj` renderers were removed, all with no
        // deprecated alias, so none of the retired values may deserialize.
        for retired in ["lean3", "tree", "tree+proj"] {
            let source = VALID_CONFIG.replace(
                "scenario = \"schedule-negotiation\"",
                &format!("scenario = \"schedule-negotiation\"\ntersmu-format = \"{retired}\""),
            );
            assert!(
                RunConfig::from_toml(&source).is_err(),
                "retired tersmu format `{retired}` must not deserialize"
            );
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn listener_mode_and_degraded_search_override_are_explicitly_configurable() {
        let source = VALID_CONFIG.replace(
            "scenario = \"schedule-negotiation\"",
            "scenario = \"schedule-negotiation\"\nlistener-mode = \"blind-then-reveal\"\nallow-degraded-search = true",
        );
        let config = RunConfig::from_toml(&source).expect("valid listener settings");
        assert_eq!(config.listener_mode, ListenerMode::BlindThenReveal);
        assert!(config.allow_degraded_search);

        let invalid = source.replace("blind-then-reveal", "sometimes-blind");
        assert!(RunConfig::from_toml(&invalid).is_err());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn meaning_review_is_opt_in_and_inherits_unless_overridden() {
        let config = RunConfig::from_toml(VALID_CONFIG).expect("valid config");
        assert!(!config.meaning_review.enabled);
        assert_eq!(config.meaning_review.temperature, None);
        assert_eq!(config.meaning_review.reasoning, None);

        let enabled = VALID_CONFIG.replace(
            "scenario = \"schedule-negotiation\"",
            "scenario = \"schedule-negotiation\"\n\n[meaning-review]\nenabled = true",
        );
        let config = RunConfig::from_toml(&enabled).expect("valid meaning-review config");
        assert!(config.meaning_review.enabled);
        assert_eq!(config.meaning_review.temperature, None);
        assert_eq!(config.meaning_review.reasoning, None);

        let overridden = enabled.replace(
            "enabled = true",
            "enabled = true\ntemperature = 0.2\nreasoning = \"high\"",
        );
        let config = RunConfig::from_toml(&overridden).expect("valid meaning-review overrides");
        assert!(config.meaning_review.enabled);
        assert_eq!(config.meaning_review.temperature, Some(0.2));
        assert_eq!(config.meaning_review.reasoning, Some(ReasoningConfig::High));

        for value in ["-0.1", "2.1"] {
            let invalid = enabled.replace(
                "enabled = true",
                &format!("enabled = true\ntemperature = {value}"),
            );
            let error =
                RunConfig::from_toml(&invalid).expect_err("out-of-range temperature must fail");
            assert!(
                error
                    .to_string()
                    .contains("reviewer temperature must be finite and between 0 and 2"),
                "{error}"
            );
        }

        let unknown = enabled.replace("enabled = true", "enabled = true\nmodel = \"other/model\"");
        let error = RunConfig::from_toml(&unknown).expect_err("reviewer model is not configurable");
        assert!(error.to_string().contains("unknown field `model`"), "{error}");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn completion_token_limits_are_configurable_and_validated() {
        // Issue #726: the completion token limit is a `[client]` default with
        // an optional per-participant override; both must be positive.
        let configured = VALID_CONFIG.replace(
            "scenario = \"schedule-negotiation\"",
            "scenario = \"schedule-negotiation\"\n\n[client]\nmax-completion-tokens = 32768",
        );
        let config = RunConfig::from_toml(&configured).expect("valid client token limit");
        assert_eq!(config.client.max_completion_tokens, 32_768);
        assert!(
            config
                .participants
                .iter()
                .all(|participant| participant.max_completion_tokens.is_none())
        );
        assert_eq!(
            config.participants[0].effective_max_completion_tokens(config.client.max_completion_tokens),
            32_768,
            "participants without an override inherit the client default"
        );

        let overridden = configured.replace(
            "model = \"example/alice\"",
            "model = \"example/alice\"\nmax-completion-tokens = 8192",
        );
        let config = RunConfig::from_toml(&overridden).expect("valid participant override");
        assert_eq!(config.participants[0].max_completion_tokens, Some(8_192));
        assert_eq!(config.participants[1].max_completion_tokens, None);
        assert_eq!(
            config.participants[0].effective_max_completion_tokens(config.client.max_completion_tokens),
            8_192,
            "the per-participant override wins over the client default"
        );
        assert_eq!(
            config.participants[1].effective_max_completion_tokens(config.client.max_completion_tokens),
            32_768
        );

        let invalid = configured.replace("max-completion-tokens = 32768", "max-completion-tokens = 0");
        let error = RunConfig::from_toml(&invalid).expect_err("zero client limit must be rejected");
        assert!(
            error
                .to_string()
                .contains("completion token limit must be positive"),
            "{error}"
        );

        let invalid = VALID_CONFIG.replace(
            "model = \"example/alice\"",
            "model = \"example/alice\"\nmax-completion-tokens = 0",
        );
        let error =
            RunConfig::from_toml(&invalid).expect_err("zero participant limit must be rejected");
        assert!(
            error
                .to_string()
                .contains("participant completion token limit must be positive"),
            "{error}"
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
        assert_eq!(config.participants[1].tool_choice, ToolChoice::Metadata);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn participant_can_override_reasoning_metadata() {
        let source = VALID_CONFIG.replace(
            "model = \"example/alice\"",
            "model = \"example/alice\"\nreasoning = \"low\"",
        );
        let config = RunConfig::from_toml(&source).expect("valid config");
        assert_eq!(config.participants[0].reasoning, Some(ReasoningConfig::Low));
        assert_eq!(config.participants[1].reasoning, None);

        for (configured, expected) in [
            ("off", ReasoningConfig::Off),
            ("default", ReasoningConfig::Default),
            ("low", ReasoningConfig::Low),
            ("medium", ReasoningConfig::Medium),
            ("high", ReasoningConfig::High),
        ] {
            let source = VALID_CONFIG.replace(
                "model = \"example/alice\"",
                &format!("model = \"example/alice\"\nreasoning = \"{configured}\""),
            );
            let config = RunConfig::from_toml(&source).expect("valid reasoning config");
            assert_eq!(config.participants[0].reasoning, Some(expected));
        }

        let invalid = VALID_CONFIG.replace(
            "model = \"example/alice\"",
            "model = \"example/alice\"\nreasoning = \"extreme\"",
        );
        assert!(RunConfig::from_toml(&invalid).is_err());

        let legacy_disabled = VALID_CONFIG.replace(
            "model = \"example/alice\"",
            "model = \"example/alice\"\ndisable-reasoning = true",
        );
        let config =
            RunConfig::from_toml(&legacy_disabled).expect("legacy config remains readable");
        assert_eq!(config.participants[0].reasoning, Some(ReasoningConfig::Off));

        let legacy_enabled =
            legacy_disabled.replace("disable-reasoning = true", "disable-reasoning = false");
        let config = RunConfig::from_toml(&legacy_enabled).expect("legacy config remains readable");
        assert_eq!(
            config.participants[0].reasoning,
            Some(ReasoningConfig::Default)
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn client_settings_are_configurable_and_validated() {
        let configured = VALID_CONFIG.replace(
            "scenario = \"schedule-negotiation\"",
            "scenario = \"schedule-negotiation\"\n\n[client]\nbase-url = \"http://127.0.0.1:1234/v1\"\nhttp-timeout-seconds = 180",
        );
        let config = RunConfig::from_toml(&configured).expect("valid client settings");
        assert_eq!(
            config.client.base_url.as_deref(),
            Some("http://127.0.0.1:1234/v1")
        );
        assert_eq!(config.client.http_timeout(), Duration::from_secs(180));

        let invalid = configured.replace("http-timeout-seconds = 180", "http-timeout-seconds = 0");
        let error = RunConfig::from_toml(&invalid).expect_err("zero timeout must be rejected");
        assert!(error.to_string().contains("HTTP timeout must be positive"));

        for empty in ["", "   "] {
            let invalid = configured.replace(
                "base-url = \"http://127.0.0.1:1234/v1\"",
                &format!("base-url = \"{empty}\""),
            );
            let error =
                RunConfig::from_toml(&invalid).expect_err("empty base URL must be rejected");
            assert!(
                error
                    .to_string()
                    .contains("client base URL cannot be empty")
            );
        }
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
                "30",
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

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn external_tersmu_format_deserializes_and_rejects_empty_commands() {
        let configured = VALID_CONFIG.replace(
            "scenario = \"schedule-negotiation\"",
            "scenario = \"schedule-negotiation\"\ntersmu-format = { external = [\"renderer\", \"--notation\", \"lean2\"] }",
        );
        let config = RunConfig::from_toml(&configured).expect("valid external renderer config");
        let TersmuFormat::External(command) = &config.tersmu_format else {
            panic!("expected external tersmu format");
        };
        assert_eq!(command.program(), "renderer");
        assert_eq!(command.args(), ["--notation", "lean2"]);

        for command in ["[]", "[\"\"]", "[\"   \"]"] {
            let invalid = VALID_CONFIG.replace(
                "scenario = \"schedule-negotiation\"",
                &format!(
                    "scenario = \"schedule-negotiation\"\ntersmu-format = {{ external = {command} }}"
                ),
            );
            let error =
                RunConfig::from_toml(&invalid).expect_err("empty renderer command must fail");
            assert!(
                error
                    .to_string()
                    .contains("external renderer command must name a non-empty executable"),
                "{error}"
            );
        }
    }
}
