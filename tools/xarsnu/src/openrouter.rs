//! Sequential OpenRouter chat-completions runtime with dynamic tool sets.

use std::collections::BTreeMap;
use std::fmt;
use std::thread;
use std::time::Duration;

#[allow(unused_imports)]
use bityzba::{contract_trait, ensures, invariant, new, requires};
use serde::ser::{Error as _, SerializeSeq};
use serde::{Deserialize, Serialize, Serializer};
use serde_json::Value;
use thiserror::Error;

use crate::model_capabilities::ParticipantModelPolicy;
use crate::{PromptCaching, ProviderToolChoice, ReasoningConfig};

const DEFAULT_OPENROUTER_BASE_URL: &str = "https://openrouter.ai/api/v1";
pub(crate) const REQUIRED_TOOL_CORRECTION: &str =
    "You must respond by calling one of the provided tools. Do not answer with prose.";
const EMPTY_RESPONSE_CORRECTION: &str = "Your previous response supplied no visible content or tool call. Private reasoning, if any, is not received as a reply. Respond with visible content or call one of the provided tools.";
const SKIPPED_INVALID_BATCH_CALL: &str = "This tool call was not executed because another tool call in the same response had invalid arguments.";

#[requires(true)]
#[ensures(ret == !*value)]
fn is_false(value: &bool) -> bool {
    !*value
}

/// Whether a request or response-body failure is an explicit transport timeout.
#[requires(true)]
#[ensures(!ret || matches!(error, ureq::Error::Timeout(_) | ureq::Error::Io(_)))]
fn is_retriable_transport_timeout(error: &ureq::Error) -> bool {
    match error {
        ureq::Error::Timeout(_) => true,
        ureq::Error::Io(error) => error.kind() == std::io::ErrorKind::TimedOut,
        _ => false,
    }
}

/// Whether reading a response body failed at the transport boundary.
#[requires(true)]
#[ensures(ret == matches!(error, ureq::Error::Timeout(_) | ureq::Error::Io(_)))]
fn is_retriable_response_body_failure(error: &ureq::Error) -> bool {
    matches!(error, ureq::Error::Timeout(_) | ureq::Error::Io(_))
}

/// Whether JSON parsing stopped because the provider response ended early.
#[requires(true)]
#[ensures(ret == (error.classify() == serde_json::error::Category::Eof))]
fn is_truncated_json_response(error: &serde_json::Error) -> bool {
    error.classify() == serde_json::error::Category::Eof
}

/// Whether an OpenRouter error code denotes a transient provider condition.
#[requires(true)]
#[ensures(ret == (code == 408 || code == 429 || (500..=599).contains(&code)))]
fn is_transient_provider_code(code: u16) -> bool {
    code == 408 || code == 429 || (500..=599).contains(&code)
}

/// Whether a model currently requires explicit provider prompt-cache breakpoints.
///
/// Keep provider policy centralized here: OpenRouter models under `anthropic/`
/// require explicit management today, while other configured models retain the
/// legacy request shape.
#[requires(true)]
#[ensures(ret == model.starts_with("anthropic/"))]
fn model_requires_explicit_prompt_caching(model: &str) -> bool {
    model.starts_with("anthropic/")
}

/// The OpenAI-compatible function portion of a model-facing tool definition.
#[invariant(!name.trim().is_empty(), "tool names cannot be empty")]
#[invariant(!description.trim().is_empty(), "tool descriptions cannot be empty")]
#[invariant(parameters.is_object(), "tool parameters must be a JSON Schema object")]
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FunctionToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

/// One OpenAI-compatible function tool offered to a model.
#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ToolDefinition {
    #[serde(rename = "type")]
    kind: ToolKind,
    pub function: FunctionToolDefinition,
}

impl ToolDefinition {
    /// Construct a validated function tool definition.
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|tool| tool.function.name == name) || ret.is_err())]
    pub fn new(
        name: String,
        description: String,
        parameters: Value,
    ) -> Result<Self, ToolDefinitionError> {
        let function =
            FunctionToolDefinition::try_from_data(bityzba::data!(FunctionToolDefinition {
                name: name.clone(),
                description,
                parameters,
            }))
            .map_err(|error| ToolDefinitionError::Invalid {
                message: error.to_string(),
            })?;
        Ok(Self {
            kind: ToolKind::Function,
            function,
        })
    }

    /// Return the function name used for dispatch and validation.
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn name(&self) -> &str {
        &self.function.name
    }
}

/// A caller supplied an invalid model-facing tool definition.
#[derive(Debug, Error, PartialEq, Eq)]
#[invariant(true)]
#[invariant(::Invalid { .. } => true)]
pub enum ToolDefinitionError {
    #[error("invalid tool definition: {message}")]
    Invalid { message: String },
}

#[invariant(::Function => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
enum ToolKind {
    Function,
}

/// The model-selected function and its JSON-encoded arguments.
#[invariant(!name.trim().is_empty(), "tool-call function names cannot be empty")]
#[invariant(!arguments.trim().is_empty(), "tool-call arguments cannot be empty")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

/// One tool call returned by OpenRouter.
#[invariant(!id.trim().is_empty(), "tool-call ids cannot be empty")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    kind: ToolCallKind,
    pub function: FunctionCall,
}

impl ToolCall {
    /// Return arguments as a JSON object, rejecting malformed provider output.
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(Value::is_object) || ret.is_err())]
    pub fn arguments(&self) -> Result<Value, OpenRouterError> {
        let arguments: Value = serde_json::from_str(&self.function.arguments).map_err(|error| {
            OpenRouterError::InvalidToolCall {
                tool_name: self.function.name.clone(),
                message: error.to_string(),
            }
        })?;
        if !arguments.is_object() {
            return Err(OpenRouterError::InvalidToolCall {
                tool_name: self.function.name.clone(),
                message: "tool arguments must encode a JSON object".to_owned(),
            });
        }
        Ok(arguments)
    }
}

#[invariant(::Function => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum ToolCallKind {
    Function,
}

/// A message retained in one participant's private conversation.
#[invariant(::System { content } => !content.trim().is_empty())]
#[invariant(::User { content } => !content.trim().is_empty())]
#[invariant(::Assistant { content, tool_calls } => content.as_ref().is_some_and(|text| !text.is_empty()) || !tool_calls.is_empty())]
#[invariant(::Tool { tool_call_id, name, .. } => !tool_call_id.trim().is_empty() && !name.trim().is_empty())]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "role", rename_all = "lowercase")]
pub enum ChatMessage {
    System {
        content: String,
    },
    User {
        content: String,
    },
    Assistant {
        content: Option<String>,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        tool_calls: Vec<ToolCall>,
    },
    Tool {
        tool_call_id: String,
        name: String,
        content: String,
    },
}

impl ChatMessage {
    /// Construct a system message.
    #[requires(!content.trim().is_empty())]
    #[ensures(matches!(ret.as_data(), bityzba::data!(ChatMessage::System { .. })))]
    fn system(content: String) -> Self {
        new!(ChatMessage::System { content })
    }

    /// Construct a user message.
    #[requires(!content.trim().is_empty())]
    #[ensures(matches!(ret.as_data(), bityzba::data!(ChatMessage::User { .. })))]
    fn user(content: String) -> Self {
        new!(ChatMessage::User { content })
    }

    /// Construct an assistant message from one completion response.
    #[requires(content.as_ref().is_some_and(|text| !text.is_empty()) || !tool_calls.is_empty())]
    #[ensures(matches!(ret.as_data(), bityzba::data!(ChatMessage::Assistant { .. })))]
    fn assistant(content: Option<String>, tool_calls: Vec<ToolCall>) -> Self {
        new!(ChatMessage::Assistant {
            content,
            tool_calls,
        })
    }

    /// Construct an assistant-voice continuation used as provider prefill.
    #[requires(!content.trim().is_empty())]
    #[ensures(matches!(ret.as_data(), bityzba::data!(ChatMessage::Assistant { .. })))]
    fn assistant_prefill(content: String) -> Self {
        Self::assistant(Some(content), Vec::new())
    }

    /// Construct a tool-result message threaded to its originating call.
    #[requires(!tool_call_id.trim().is_empty())]
    #[requires(!name.trim().is_empty())]
    #[ensures(matches!(ret.as_data(), bityzba::data!(ChatMessage::Tool { .. })))]
    fn tool(tool_call_id: String, name: String, content: String) -> Self {
        new!(ChatMessage::Tool {
            tool_call_id,
            name,
            content,
        })
    }
}

/// Usage reported for one non-streaming completion.
///
/// Token counts preserve provider accounting verbatim. Their unsigned types
/// enforce the only portable structural constraint; relationships between
/// counters are intentionally unconstrained because providers account for
/// cached, reasoning, completion, and total tokens differently.
#[invariant(provider.as_ref().is_none_or(|provider| !provider.trim().is_empty()), "serving provider must not be empty when reported")]
#[invariant(cost.is_finite() && *cost >= 0.0, "reported cost must be finite and nonnegative")]
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Usage {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(default)]
    pub prompt_tokens: u64,
    #[serde(default)]
    pub completion_tokens: u64,
    #[serde(default)]
    pub total_tokens: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cached_tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_write_tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub reasoning_present: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning_tokens: Option<u64>,
    #[serde(default)]
    pub cost: f64,
}

/// Private reasoning payload returned by one provider call.
///
/// The structured details remain provider-shaped JSON because OpenRouter
/// requires them to be replayed byte-for-byte at the semantic JSON boundary.
#[invariant(reasoning.is_some() || reasoning_details.is_some(), "thinking traces must contain at least one provider field")]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThinkingTrace {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning_details: Option<Vec<Value>>,
}

impl ThinkingTrace {
    #[requires(true)]
    #[ensures(ret.as_ref().is_none_or(|trace| trace.reasoning.is_some() || trace.reasoning_details.is_some()))]
    fn from_provider_fields(
        reasoning: Option<String>,
        reasoning_details: Option<Vec<Value>>,
    ) -> Option<Self> {
        if reasoning.is_none() && reasoning_details.is_none() {
            None
        } else {
            Some(new!(ThinkingTrace {
                reasoning,
                reasoning_details,
            }))
        }
    }
}

/// Usage and private observability captured from one provider call.
#[invariant(true, "usage-only and usage-plus-thinking calls are both valid")]
#[derive(Debug, Clone, PartialEq)]
pub struct ProviderCallObservation {
    pub usage: Usage,
    pub thinking: Option<ThinkingTrace>,
}

/// Request-scoped reasoning details attached to their originating assistant message.
#[invariant(!reasoning_details.is_empty(), "empty details do not require replay")]
#[derive(Debug, Clone, PartialEq)]
struct ReasoningDetailsReplay {
    assistant_message_index: usize,
    reasoning_details: Vec<Value>,
}

/// Accumulated token and cost totals.
#[invariant(
    true,
    "mutated only by UsageTotals::record, which preserves nonnegative totals"
)]
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct UsageTotals {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
    pub cached_tokens: u64,
    pub cache_write_tokens: u64,
    pub reasoning_tokens: u64,
    pub provider_calls: u64,
    pub cache_hit_calls: u64,
    pub reasoning_calls: u64,
    pub provider_calls_by_name: BTreeMap<String, u64>,
    pub cost_usd: f64,
}

impl UsageTotals {
    /// Add one response's usage using saturating token counters.
    #[requires(usage.cost.is_finite() && usage.cost >= 0.0)]
    #[ensures(self.cost_usd >= old(self.cost_usd))]
    pub fn record(&mut self, usage: &Usage) {
        self.prompt_tokens = self.prompt_tokens.saturating_add(usage.prompt_tokens);
        self.completion_tokens = self
            .completion_tokens
            .saturating_add(usage.completion_tokens);
        self.total_tokens = self.total_tokens.saturating_add(usage.total_tokens);
        self.cached_tokens = self
            .cached_tokens
            .saturating_add(usage.cached_tokens.unwrap_or(0));
        self.cache_write_tokens = self
            .cache_write_tokens
            .saturating_add(usage.cache_write_tokens.unwrap_or(0));
        self.reasoning_tokens = self
            .reasoning_tokens
            .saturating_add(usage.reasoning_tokens.unwrap_or(0));
        self.provider_calls = self.provider_calls.saturating_add(1);
        if usage.cached_tokens.unwrap_or(0) > 0 {
            self.cache_hit_calls = self.cache_hit_calls.saturating_add(1);
        }
        if usage.reasoning_present || usage.reasoning_tokens.is_some() {
            self.reasoning_calls = self.reasoning_calls.saturating_add(1);
        }
        if let Some(provider) = &usage.provider {
            let calls = self
                .provider_calls_by_name
                .entry(provider.clone())
                .or_default();
            *calls = calls.saturating_add(1);
        }
        self.cost_usd += usage.cost;
    }

    /// Fraction of prompt tokens served from cache, when prompt usage exists.
    #[requires(true)]
    #[ensures(ret.is_none() == (self.prompt_tokens == 0))]
    #[ensures(ret.is_none_or(|rate| rate.is_finite() && rate >= 0.0))]
    pub fn cache_efficiency(&self) -> Option<f64> {
        (self.prompt_tokens > 0).then(|| self.cached_tokens as f64 / self.prompt_tokens as f64)
    }

    /// Fraction of provider calls reporting at least one cached prompt token.
    #[requires(true)]
    #[ensures(ret.is_none() == (self.provider_calls == 0))]
    #[ensures(ret.is_none_or(|rate| (0.0..=1.0).contains(&rate)))]
    pub fn cache_hit_rate(&self) -> Option<f64> {
        (self.provider_calls > 0).then(|| self.cache_hit_calls as f64 / self.provider_calls as f64)
    }
}

/// Why a bounded run stopped without a runtime failure.
#[invariant(::CostBudgetExceeded => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AbortKind {
    CostBudgetExceeded,
}

/// Explicit record surfaced when a graceful run cap is hit.
#[invariant(max_cost_usd.is_finite() && *max_cost_usd > 0.0)]
#[invariant(actual_cost_usd.is_finite() && *actual_cost_usd >= *max_cost_usd)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct AbortRecord {
    pub kind: AbortKind,
    pub max_cost_usd: f64,
    pub actual_cost_usd: f64,
}

/// Run-wide accounting shared by sequential participant calls.
#[invariant(
    true,
    "constructed with a positive budget and mutated only through record"
)]
#[derive(Debug, Clone, PartialEq)]
pub struct RunAccounting {
    max_cost_usd: f64,
    usage: UsageTotals,
    abort: Option<AbortRecord>,
}

impl RunAccounting {
    /// Create accounting for one run-wide dollar cap.
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|value| value.usage.cost_usd == 0.0) || ret.is_err())]
    pub fn new(max_cost_usd: f64) -> Result<Self, OpenRouterError> {
        if !max_cost_usd.is_finite() || max_cost_usd <= 0.0 {
            return Err(OpenRouterError::InvalidConfiguration {
                message: "maximum run cost must be finite and positive".to_owned(),
            });
        }
        Ok(Self {
            max_cost_usd,
            usage: UsageTotals::default(),
            abort: None,
        })
    }

    /// Current run-wide usage.
    #[requires(true)]
    #[ensures(ret.cost_usd >= 0.0)]
    pub fn usage(&self) -> &UsageTotals {
        &self.usage
    }

    /// Existing graceful-abort record, if any.
    #[requires(true)]
    #[ensures(ret.is_some() == self.abort.is_some())]
    pub fn abort(&self) -> Option<&AbortRecord> {
        self.abort.as_ref()
    }

    /// Account one response and return the first budget-abort record.
    #[requires(usage.cost.is_finite() && usage.cost >= 0.0)]
    #[ensures(ret.is_some() == self.abort.is_some())]
    fn record(&mut self, usage: &Usage) -> Option<AbortRecord> {
        if let Some(abort) = &self.abort {
            return Some(abort.clone());
        }
        self.usage.record(usage);
        if self.usage.cost_usd >= self.max_cost_usd {
            let abort = new!(AbortRecord {
                kind: AbortKind::CostBudgetExceeded,
                max_cost_usd: self.max_cost_usd,
                actual_cost_usd: self.usage.cost_usd,
            });
            self.abort = Some(abort.clone());
            Some(abort)
        } else {
            None
        }
    }
}

/// Transient retry settings for OpenRouter HTTP responses.
#[invariant(*initial_backoff > Duration::ZERO, "retry backoff must be positive")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryPolicy {
    pub max_retries: usize,
    pub initial_backoff: Duration,
}

impl Default for RetryPolicy {
    #[requires(true)]
    #[ensures(ret.initial_backoff > Duration::ZERO)]
    fn default() -> Self {
        Self::from_data(bityzba::data!(RetryPolicy {
            max_retries: 3,
            initial_backoff: Duration::from_millis(250),
        }))
    }
}

impl RetryPolicy {
    /// Create a retry policy with a nonzero initial delay.
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|value| value.max_retries == max_retries) || ret.is_err())]
    pub fn new(max_retries: usize, initial_backoff: Duration) -> Result<Self, OpenRouterError> {
        Self::try_from_data(bityzba::data!(RetryPolicy {
            max_retries,
            initial_backoff,
        }))
        .map_err(|error| OpenRouterError::InvalidConfiguration {
            message: error.to_string(),
        })
    }
}

/// Configuration used to construct an HTTP client.
#[invariant(!base_url.trim().is_empty(), "OpenRouter base URL cannot be empty")]
#[invariant(*timeout > Duration::ZERO, "HTTP timeout must be positive")]
#[derive(Clone)]
pub struct OpenRouterClientConfig {
    pub base_url: String,
    pub api_key: Option<String>,
    pub retry_policy: RetryPolicy,
    pub max_required_tool_reprompts: usize,
    pub timeout: Duration,
}

impl fmt::Debug for OpenRouterClientConfig {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OpenRouterClientConfig")
            .field("base_url", &self.base_url)
            .field("api_key", &self.api_key.as_ref().map(|_| "[redacted]"))
            .field("retry_policy", &self.retry_policy)
            .field(
                "max_required_tool_reprompts",
                &self.max_required_tool_reprompts,
            )
            .field("timeout", &self.timeout)
            .finish()
    }
}

impl Default for OpenRouterClientConfig {
    #[requires(true)]
    #[ensures(ret.base_url == DEFAULT_OPENROUTER_BASE_URL)]
    fn default() -> Self {
        Self::from_data(bityzba::data!(OpenRouterClientConfig {
            base_url: DEFAULT_OPENROUTER_BASE_URL.to_owned(),
            api_key: None,
            retry_policy: RetryPolicy::default(),
            max_required_tool_reprompts: 2,
            timeout: Duration::from_secs(60),
        }))
    }
}

impl OpenRouterClientConfig {
    /// Create configuration for an explicit base URL without consulting the environment.
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|value| !value.base_url.is_empty()) || ret.is_err())]
    pub fn new(
        base_url: String,
        api_key: Option<String>,
        retry_policy: RetryPolicy,
        max_required_tool_reprompts: usize,
        timeout: Duration,
    ) -> Result<Self, OpenRouterError> {
        Self::try_from_data(bityzba::data!(OpenRouterClientConfig {
            base_url,
            api_key,
            retry_policy,
            max_required_tool_reprompts,
            timeout,
        }))
        .map_err(|error| OpenRouterError::InvalidConfiguration {
            message: error.to_string(),
        })
    }

    /// Load the real OpenRouter API key at runtime.
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|value| value.api_key.is_some()) || ret.is_err())]
    pub fn from_env() -> Result<Self, OpenRouterError> {
        let api_key = std::env::var("OPENROUTER_API_KEY")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .ok_or(OpenRouterError::MissingApiKey)?;
        let mut data = Self::default().into_data();
        data.api_key = Some(api_key);
        Ok(Self::from_data(data))
    }
}

/// Synchronous OpenRouter client for the sequential xarsnu turn loop.
#[invariant(true)]
#[derive(Debug, Clone)]
pub struct OpenRouterClient {
    config: OpenRouterClientConfig,
    agent: ureq::Agent,
}

impl OpenRouterClient {
    /// Construct a client without reading process environment.
    #[requires(true)]
    #[ensures(!ret.config.base_url.is_empty())]
    pub fn new(config: OpenRouterClientConfig) -> Self {
        let agent_config = ureq::Agent::config_builder()
            .timeout_global(Some(config.timeout))
            .http_status_as_error(false)
            .build();
        Self {
            config,
            agent: ureq::Agent::new_with_config(agent_config),
        }
    }

    /// Construct the real OpenRouter client from runtime environment.
    #[requires(true)]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    pub fn from_env() -> Result<Self, OpenRouterError> {
        Ok(Self::new(OpenRouterClientConfig::from_env()?))
    }

    /// Construct the real OpenRouter client with a run-configured timeout.
    #[requires(timeout > Duration::ZERO)]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    pub fn from_env_with_timeout(timeout: Duration) -> Result<Self, OpenRouterError> {
        let config = OpenRouterClientConfig::from_env()?.with_data(bityzba::data! {
            timeout: timeout,
        });
        Ok(Self::new(config))
    }

    /// Protocol-level corrective reprompts allowed after an automatic prose response.
    #[requires(true)]
    #[ensures(ret == self.config.max_required_tool_reprompts)]
    pub(crate) fn max_required_tool_reprompts(&self) -> usize {
        self.config.max_required_tool_reprompts
    }

    /// Sleep for the next exponential-backoff slot when retry capacity remains.
    #[requires(true)]
    #[ensures(ret == (old(*retries) < self.config.retry_policy.max_retries))]
    #[ensures(ret -> *retries == old(*retries) + 1)]
    #[ensures(!ret -> *retries == old(*retries))]
    fn back_off_before_retry(&self, retries: &mut usize) -> bool {
        if *retries >= self.config.retry_policy.max_retries {
            return false;
        }
        let factor = 1u32
            .checked_shl((*retries).min(31) as u32)
            .unwrap_or(u32::MAX);
        let delay = self
            .config
            .retry_policy
            .initial_backoff
            .saturating_mul(factor);
        thread::sleep(delay);
        *retries += 1;
        true
    }

    /// Back off after a timeout or return the typed exhaustion error.
    #[requires(is_retriable_transport_timeout(error))]
    #[ensures(ret.is_ok() == (old(*retries) < self.config.retry_policy.max_retries))]
    fn back_off_after_transport_timeout(
        &self,
        retries: &mut usize,
        error: &ureq::Error,
    ) -> Result<(), OpenRouterError> {
        let message = error.to_string();
        if self.back_off_before_retry(retries) {
            Ok(())
        } else {
            Err(OpenRouterError::TransportRetriesExhausted {
                attempts: *retries + 1,
                message,
            })
        }
    }

    /// Back off after a response-body transport failure or truncated JSON.
    #[requires(!message.trim().is_empty())]
    #[ensures(ret.is_ok() == (old(*retries) < self.config.retry_policy.max_retries))]
    fn back_off_after_response_body_failure(
        &self,
        retries: &mut usize,
        message: String,
    ) -> Result<(), OpenRouterError> {
        if self.back_off_before_retry(retries) {
            Ok(())
        } else {
            Err(OpenRouterError::TransportRetriesExhausted {
                attempts: *retries + 1,
                message,
            })
        }
    }

    /// Issue one completion with the caller's exact tool list.
    #[requires(!model.trim().is_empty())]
    #[requires(temperature.is_finite() && (0.0..=2.0).contains(&temperature))]
    #[requires(!messages.is_empty())]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    fn complete(
        &self,
        model: &str,
        provider: Option<&toml::Table>,
        prompt_caching: PromptCaching,
        temperature: f64,
        messages: &[ChatMessage],
        tools: &[ToolDefinition],
        tool_choice: ProviderToolChoice,
        reasoning: ReasoningConfig,
        reasoning_details_replay: &[ReasoningDetailsReplay],
    ) -> Result<Completion, OpenRouterError> {
        let explicit_prompt_caching =
            prompt_caching == PromptCaching::Auto && model_requires_explicit_prompt_caching(model);

        // OpenRouter's Anthropic-compatible prompt prefix places tool definitions
        // before message content. Xarsnu deliberately keeps tools dynamic by phase,
        // so a phase transition invalidates the cache even with these breakpoints.
        // Within-phase loops carry the expected call volume; do not stabilize the
        // tool array here because that would weaken protocol enforcement.
        let request = CompletionRequest {
            model,
            provider,
            temperature,
            messages: new!(CompletionMessages {
                messages,
                explicit_prompt_caching,
                reasoning_details_replay,
            }),
            tools,
            tool_choice,
            reasoning: CompletionReasoningRequest::from_config(reasoning),
            usage: new!(CompletionUsageRequest { include: true }),
        };
        let url = format!(
            "{}/chat/completions",
            self.config.base_url.trim_end_matches('/')
        );
        let mut retries = 0usize;
        loop {
            let mut builder = self
                .agent
                .post(&url)
                .header("Content-Type", "application/json");
            if let Some(api_key) = self.config.api_key.as_deref() {
                builder = builder.header("Authorization", &format!("Bearer {api_key}"));
            }
            let request_body = serde_json::to_string(&request).map_err(|error| {
                OpenRouterError::InvalidConfiguration {
                    message: format!("completion request did not serialize: {error}"),
                }
            })?;
            let mut response = match builder.send(request_body) {
                Ok(response) => response,
                Err(error) if is_retriable_transport_timeout(&error) => {
                    self.back_off_after_transport_timeout(&mut retries, &error)?;
                    continue;
                }
                Err(error) => {
                    return Err(OpenRouterError::Transport {
                        message: error.to_string(),
                    });
                }
            };
            let status = response.status().as_u16();
            if is_transient_provider_code(status) {
                if self.back_off_before_retry(&mut retries) {
                    continue;
                }
                let body = match response.body_mut().read_to_string() {
                    Ok(body) => body,
                    Err(error) if is_retriable_response_body_failure(&error) => {
                        return Err(OpenRouterError::TransportRetriesExhausted {
                            attempts: retries + 1,
                            message: error.to_string(),
                        });
                    }
                    Err(error) => format!("unable to read response body: {error}"),
                };
                return Err(OpenRouterError::TransientRetriesExhausted {
                    code: status,
                    attempts: retries + 1,
                    message: body,
                });
            }
            if !(200..=299).contains(&status) {
                let body = match response.body_mut().read_to_string() {
                    Ok(body) => body,
                    Err(error) if is_retriable_response_body_failure(&error) => {
                        self.back_off_after_response_body_failure(&mut retries, error.to_string())?;
                        continue;
                    }
                    Err(error) => format!("unable to read response body: {error}"),
                };
                return Err(OpenRouterError::HttpStatus { status, body });
            }
            let response_body = match response.body_mut().read_to_string() {
                Ok(body) => body,
                Err(error) if is_retriable_response_body_failure(&error) => {
                    self.back_off_after_response_body_failure(&mut retries, error.to_string())?;
                    continue;
                }
                Err(error) => {
                    return Err(OpenRouterError::InvalidResponse {
                        message: error.to_string(),
                    });
                }
            };
            let mut wire: CompletionResponse = match serde_json::from_str(&response_body) {
                Ok(wire) => wire,
                Err(error) if is_truncated_json_response(&error) => {
                    self.back_off_after_response_body_failure(
                        &mut retries,
                        format!("provider response body ended before JSON was complete: {error}"),
                    )?;
                    continue;
                }
                Err(error) => {
                    return Err(OpenRouterError::InvalidResponse {
                        message: error.to_string(),
                    });
                }
            };
            let choices = match wire.choices.take() {
                Some(choices) => choices,
                None => {
                    let Some(error) = wire.error.take() else {
                        return Err(OpenRouterError::InvalidResponse {
                            message: "OpenRouter response contained neither choices nor an error"
                                .to_owned(),
                        });
                    };
                    if error.message.trim().is_empty() {
                        return Err(OpenRouterError::InvalidResponse {
                            message: "OpenRouter error envelope contained an empty message"
                                .to_owned(),
                        });
                    }
                    if is_transient_provider_code(error.code) {
                        if self.back_off_before_retry(&mut retries) {
                            continue;
                        }
                        return Err(OpenRouterError::TransientRetriesExhausted {
                            code: error.code,
                            attempts: retries + 1,
                            message: error.message,
                        });
                    }
                    return Err(OpenRouterError::Provider {
                        code: error.code,
                        message: error.message,
                    });
                }
            };
            let choice =
                choices
                    .into_iter()
                    .next()
                    .ok_or_else(|| OpenRouterError::InvalidResponse {
                        message: "OpenRouter returned no completion choices".to_owned(),
                    })?;
            let CompletionMessage {
                content,
                reasoning,
                reasoning_details,
                tool_calls,
            } = choice.message;
            let thinking = ThinkingTrace::from_provider_fields(reasoning, reasoning_details);
            let reasoning_present = thinking.is_some();
            let usage = wire
                .usage
                .into_usage(reasoning_present, wire.provider.take())?;
            let content = content.filter(|content| !content.is_empty());
            return Ok(Completion {
                content,
                tool_calls,
                thinking,
                usage,
            });
        }
    }
}

/// Harness-side execution hook used by later protocol layers.
#[contract_trait]
pub trait ToolDispatcher {
    /// Execute one model-requested tool and return its unmodified textual payload.
    #[requires(true)]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    fn dispatch(&mut self, call: &ToolCall) -> Result<String, ToolDispatchError>;
}

/// A harness-side tool could not be dispatched.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
#[error("tool `{tool_name}` failed: {message}")]
#[invariant(true)]
pub struct ToolDispatchError {
    pub tool_name: String,
    pub message: String,
}

impl ToolDispatchError {
    /// Construct a typed dispatch error.
    #[requires(!tool_name.trim().is_empty())]
    #[requires(!message.trim().is_empty())]
    #[ensures(!ret.tool_name.is_empty())]
    pub fn new(tool_name: String, message: String) -> Self {
        Self { tool_name, message }
    }
}

/// Result of one model call after required-tool fallback and budget checks.
#[invariant(::ToolCalls { calls } => !calls.is_empty())]
#[invariant(::Message { content } => !content.is_empty())]
#[invariant(::Aborted { .. } => true)]
#[derive(Debug, Clone, PartialEq)]
pub enum ModelTurn {
    ToolCalls { calls: Vec<ToolCall> },
    Message { content: String },
    Aborted { record: AbortRecord },
}

impl ModelTurn {
    /// Tool calls selected by the model, when this is a tool-call turn.
    #[requires(true)]
    #[ensures(ret.is_some() == matches!(self.as_data(), bityzba::data!(ModelTurn::ToolCalls { .. })))]
    pub fn tool_calls(&self) -> Option<&[ToolCall]> {
        match self.as_data() {
            bityzba::data!(ModelTurn::ToolCalls { calls }) => Some(calls),
            _ => None,
        }
    }

    /// Prose content, when automatic tool choice allowed a normal message.
    #[requires(true)]
    #[ensures(ret.is_some() == matches!(self.as_data(), bityzba::data!(ModelTurn::Message { .. })))]
    pub fn content(&self) -> Option<&str> {
        match self.as_data() {
            bityzba::data!(ModelTurn::Message { content }) => Some(content),
            _ => None,
        }
    }

    /// Graceful run-abort record, when a cap stopped inference.
    #[requires(true)]
    #[ensures(ret.is_some() == matches!(self.as_data(), bityzba::data!(ModelTurn::Aborted { .. })))]
    pub fn abort_record(&self) -> Option<&AbortRecord> {
        match self.as_data() {
            bityzba::data!(ModelTurn::Aborted { record }) => Some(record),
            _ => None,
        }
    }
}

/// Private message history and usage for one participant.
#[invariant(
    true,
    "initialized from validated ParticipantConfig and mutated only by its methods"
)]
#[derive(Debug, Clone, PartialEq)]
pub struct ParticipantConversation {
    participant_name: String,
    model: String,
    provider: Option<toml::Table>,
    prompt_caching: PromptCaching,
    reasoning: ReasoningConfig,
    temperature: f64,
    messages: Vec<ChatMessage>,
    usage: UsageTotals,
    pending_observations: Vec<ProviderCallObservation>,
    reasoning_details_replay: Vec<ReasoningDetailsReplay>,
}

impl ParticipantConversation {
    /// Seed a participant's private channel with its persona.
    ///
    /// The scenario runner appends the public setup and participant-scoped
    /// scenario brief before the first model call.
    #[requires(true)]
    #[ensures(ret.messages.len() == 1)]
    pub fn new(participant: &crate::ParticipantConfig) -> Self {
        let policy = ParticipantModelPolicy::resolve(
            &participant.model,
            participant.tool_choice,
            participant.reasoning,
        );
        Self::from_system_prompt(
            participant.name.clone(),
            participant.model.clone(),
            participant.provider.clone(),
            participant.prompt_caching,
            policy.reasoning,
            participant.temperature,
            participant.system_prompt.clone(),
        )
    }

    /// Seed a participant with an explicitly composed system prompt.
    #[requires(!participant_name.trim().is_empty())]
    #[requires(!model.trim().is_empty())]
    #[requires(temperature.is_finite() && (0.0..=2.0).contains(&temperature))]
    #[requires(!system_prompt.trim().is_empty())]
    #[ensures(ret.messages.len() == 1)]
    pub fn from_system_prompt(
        participant_name: String,
        model: String,
        provider: Option<toml::Table>,
        prompt_caching: PromptCaching,
        reasoning: ReasoningConfig,
        temperature: f64,
        system_prompt: String,
    ) -> Self {
        Self {
            participant_name,
            model,
            provider,
            prompt_caching,
            reasoning,
            temperature,
            messages: vec![ChatMessage::system(system_prompt)],
            usage: UsageTotals::default(),
            pending_observations: Vec::new(),
            reasoning_details_replay: Vec::new(),
        }
    }

    /// Seed a conversation directly, primarily for lower-level runtime tests.
    #[requires(!participant_name.trim().is_empty())]
    #[requires(!model.trim().is_empty())]
    #[requires(temperature.is_finite() && (0.0..=2.0).contains(&temperature))]
    #[requires(!system_prompt.trim().is_empty())]
    #[requires(!initial_user_prompt.trim().is_empty())]
    #[ensures(ret.messages.len() == 2)]
    pub fn from_parts(
        participant_name: String,
        model: String,
        provider: Option<toml::Table>,
        prompt_caching: PromptCaching,
        reasoning: ReasoningConfig,
        temperature: f64,
        system_prompt: String,
        initial_user_prompt: String,
    ) -> Self {
        Self {
            participant_name,
            model,
            provider,
            prompt_caching,
            reasoning,
            temperature,
            messages: vec![
                ChatMessage::system(system_prompt),
                ChatMessage::user(initial_user_prompt),
            ],
            usage: UsageTotals::default(),
            pending_observations: Vec::new(),
            reasoning_details_replay: Vec::new(),
        }
    }

    /// Participant name associated with this private channel.
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn participant_name(&self) -> &str {
        &self.participant_name
    }

    /// Current private message history.
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn messages(&self) -> &[ChatMessage] {
        &self.messages
    }

    /// Current per-participant usage totals.
    #[requires(true)]
    #[ensures(ret.cost_usd >= 0.0)]
    pub fn usage(&self) -> &UsageTotals {
        &self.usage
    }

    /// Drain provider-call observations accumulated since the previous drain.
    #[requires(true)]
    #[ensures(self.pending_observations.is_empty())]
    pub fn take_pending_observations(&mut self) -> Vec<ProviderCallObservation> {
        std::mem::take(&mut self.pending_observations)
    }

    /// Start a distinct provider tool loop without carrying stale details into it.
    #[requires(true)]
    #[ensures(self.reasoning_details_replay.is_empty())]
    pub fn begin_tool_loop(&mut self) {
        self.reasoning_details_replay.clear();
    }

    /// Add a private user/protocol instruction before the next inference.
    #[requires(!content.trim().is_empty())]
    #[ensures(self.messages.len() == old(self.messages.len()) + 1)]
    pub fn push_user(&mut self, content: String) {
        self.messages.push(ChatMessage::user(content));
    }

    /// Add an assistant-voice self-correction as the next request's prefill.
    #[requires(!content.trim().is_empty())]
    #[ensures(self.messages.len() == old(self.messages.len()) + 1)]
    pub fn push_assistant_prefill(&mut self, content: String) {
        self.messages.push(ChatMessage::assistant_prefill(content));
    }

    /// Append one already-dispatched tool result to this private conversation.
    ///
    /// Protocol orchestration uses this narrow hook so scripted and live model
    /// boundaries can share the same tool-gating logic without duplicating the
    /// OpenRouter request implementation.
    #[requires(!call.id.trim().is_empty())]
    #[requires(!call.function.name.trim().is_empty())]
    #[ensures(self.messages.len() == old(self.messages.len()) + 1)]
    pub fn push_tool_result(&mut self, call: &ToolCall, content: String) {
        self.messages.push(ChatMessage::tool(
            call.id.clone(),
            call.function.name.clone(),
            content,
        ));
    }

    /// Request one model turn using exactly the supplied tool list.
    #[requires(true)]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    pub fn request(
        &mut self,
        client: &OpenRouterClient,
        tools: &[ToolDefinition],
        tool_choice: ProviderToolChoice,
        accounting: &mut RunAccounting,
    ) -> Result<ModelTurn, OpenRouterError> {
        if let Some(record) = accounting.abort() {
            return Ok(new!(ModelTurn::Aborted {
                record: record.clone(),
            }));
        }
        let mut reprompts = 0usize;
        loop {
            let completion = client.complete(
                &self.model,
                self.provider.as_ref(),
                self.prompt_caching,
                self.temperature,
                &self.messages,
                tools,
                tool_choice,
                self.reasoning,
                &self.reasoning_details_replay,
            )?;
            let Completion {
                content,
                tool_calls,
                thinking,
                usage,
            } = completion;
            self.usage.record(&usage);
            let abort = accounting.record(&usage);
            if content.is_none() && tool_calls.is_empty() {
                self.pending_observations
                    .push(ProviderCallObservation { usage, thinking });
                if let Some(record) = abort {
                    return Ok(new!(ModelTurn::Aborted { record }));
                }
                if reprompts >= client.config.max_required_tool_reprompts {
                    return Err(OpenRouterError::RequiredToolCallExhausted {
                        attempts: reprompts + 1,
                    });
                }
                self.messages
                    .push(ChatMessage::user(EMPTY_RESPONSE_CORRECTION.to_owned()));
                reprompts += 1;
                continue;
            }
            self.messages
                .push(ChatMessage::assistant(content.clone(), tool_calls.clone()));
            if !tool_calls.is_empty() {
                if let Some(reasoning_details) = thinking
                    .as_ref()
                    .and_then(|trace| trace.reasoning_details.as_ref())
                    .filter(|details| !details.is_empty())
                {
                    // Presence in the response is the capability gate. Both
                    // Anthropic signatures and Gemini thought signatures need
                    // this continuity, so no provider-family allowlist belongs
                    // here.
                    self.reasoning_details_replay
                        .push(new!(ReasoningDetailsReplay {
                            assistant_message_index: self.messages.len() - 1,
                            reasoning_details: reasoning_details.clone(),
                        }));
                }
            }
            self.pending_observations
                .push(ProviderCallObservation { usage, thinking });
            if let Some(record) = abort {
                return Ok(new!(ModelTurn::Aborted { record }));
            }
            let invalid_call = tool_calls.iter().find_map(|call| call.arguments().err());
            if invalid_call.is_some() {
                for call in &tool_calls {
                    let content = call.arguments().map_or_else(
                        |error| error.to_string(),
                        |_| SKIPPED_INVALID_BATCH_CALL.to_owned(),
                    );
                    self.messages.push(ChatMessage::tool(
                        call.id.clone(),
                        call.function.name.clone(),
                        content,
                    ));
                }
            }
            if !tool_calls.is_empty() && invalid_call.is_none() {
                return Ok(new!(ModelTurn::ToolCalls { calls: tool_calls }));
            }
            if tool_choice == ProviderToolChoice::Auto {
                if let Some(content) = content {
                    return Ok(new!(ModelTurn::Message { content }));
                }
                return Err(
                    invalid_call.unwrap_or_else(|| OpenRouterError::InvalidResponse {
                        message: "automatic tool call response was unusable".to_owned(),
                    }),
                );
            }
            if reprompts >= client.config.max_required_tool_reprompts {
                return Err(OpenRouterError::RequiredToolCallExhausted {
                    attempts: reprompts + 1,
                });
            }
            if tool_calls.is_empty() {
                self.messages
                    .push(ChatMessage::user(REQUIRED_TOOL_CORRECTION.to_owned()));
            }
            reprompts += 1;
        }
    }

    /// Execute calls and append correctly threaded tool-result messages.
    #[requires(!calls.is_empty())]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    pub fn dispatch_tool_calls(
        &mut self,
        calls: &[ToolCall],
        dispatcher: &mut impl ToolDispatcher,
    ) -> Result<(), ToolDispatchError> {
        for (index, call) in calls.iter().enumerate() {
            match dispatcher.dispatch(call) {
                Ok(content) => self.messages.push(ChatMessage::tool(
                    call.id.clone(),
                    call.function.name.clone(),
                    content,
                )),
                Err(error) => {
                    let failure = error.to_string();
                    self.messages.push(ChatMessage::tool(
                        call.id.clone(),
                        call.function.name.clone(),
                        failure.clone(),
                    ));
                    for remaining in &calls[index + 1..] {
                        self.messages.push(ChatMessage::tool(
                            remaining.id.clone(),
                            remaining.function.name.clone(),
                            format!(
                                "This tool call was not executed because an earlier tool call failed: {failure}"
                            ),
                        ));
                    }
                    return Err(error);
                }
            }
        }
        Ok(())
    }
}

/// Typed runtime failures; graceful budget stops use [`ModelTurn::Aborted`].
#[invariant(true)]
#[invariant(::MissingApiKey => true)]
#[invariant(::InvalidConfiguration { .. } => true)]
#[invariant(::Transport { .. } => true)]
#[invariant(::TransportRetriesExhausted { .. } => true)]
#[invariant(::HttpStatus { .. } => true)]
#[invariant(::TransientRetriesExhausted { .. } => true)]
#[invariant(::Provider { .. } => true)]
#[invariant(::InvalidProviderUsage { .. } => true)]
#[invariant(::InvalidResponse { .. } => true)]
#[invariant(::InvalidToolCall { .. } => true)]
#[invariant(::RequiredToolCallExhausted { .. } => true)]
#[derive(Debug, Error, PartialEq, Eq)]
pub enum OpenRouterError {
    #[error("OPENROUTER_API_KEY is not set")]
    MissingApiKey,
    #[error("invalid OpenRouter configuration: {message}")]
    InvalidConfiguration { message: String },
    #[error("OpenRouter transport failed: {message}")]
    Transport { message: String },
    #[error("OpenRouter transport timeout exhausted after {attempts} attempts: {message}")]
    TransportRetriesExhausted { attempts: usize, message: String },
    #[error("OpenRouter returned HTTP {status}: {body}")]
    HttpStatus { status: u16, body: String },
    #[error("OpenRouter transient error {code} exhausted after {attempts} attempts: {message}")]
    TransientRetriesExhausted {
        code: u16,
        attempts: usize,
        message: String,
    },
    #[error("OpenRouter provider error {code}: {message}")]
    Provider { code: u16, message: String },
    #[error("invalid OpenRouter provider usage: {reason}")]
    InvalidProviderUsage {
        reason: ProviderUsageValidationError,
    },
    #[error("invalid OpenRouter response: {message}")]
    InvalidResponse { message: String },
    #[error("invalid call to tool `{tool_name}`: {message}")]
    InvalidToolCall { tool_name: String, message: String },
    #[error("required tool call was not produced after {attempts} attempts")]
    RequiredToolCallExhausted { attempts: usize },
}

/// A provider usage payload violated a provider-independent structural clause.
#[invariant(::CostMustBeFiniteAndNonnegative => true)]
#[invariant(::ProviderMustNotBeEmpty => true)]
#[derive(Debug, Clone, Copy, Error, PartialEq, Eq)]
pub enum ProviderUsageValidationError {
    #[error("reported cost must be finite and nonnegative")]
    CostMustBeFiniteAndNonnegative,
    #[error("serving provider must not be empty when reported")]
    ProviderMustNotBeEmpty,
}

#[invariant(true)]
#[derive(Debug, Serialize)]
struct CompletionRequest<'a> {
    model: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    provider: Option<&'a toml::Table>,
    temperature: f64,
    messages: CompletionMessages<'a>,
    tools: &'a [ToolDefinition],
    tool_choice: ProviderToolChoice,
    reasoning: CompletionReasoningRequest,
    usage: CompletionUsageRequest,
}

#[invariant(::Enabled { enabled, exclude, summary } => *enabled && !*exclude && *summary == ReasoningSummaryVerbosity::Detailed)]
#[invariant(::Effort { exclude, summary, .. } => !*exclude && *summary == ReasoningSummaryVerbosity::Detailed)]
#[derive(Debug, Serialize)]
#[serde(untagged)]
enum CompletionReasoningRequest {
    Enabled {
        enabled: bool,
        exclude: bool,
        summary: ReasoningSummaryVerbosity,
    },
    Effort {
        effort: ReasoningEffort,
        exclude: bool,
        summary: ReasoningSummaryVerbosity,
    },
}

impl CompletionReasoningRequest {
    #[requires(true)]
    #[ensures(true)]
    fn from_config(reasoning: ReasoningConfig) -> Self {
        match reasoning {
            ReasoningConfig::Default => new!(CompletionReasoningRequest::Enabled {
                enabled: true,
                exclude: false,
                summary: ReasoningSummaryVerbosity::Detailed,
            }),
            ReasoningConfig::Off => new!(CompletionReasoningRequest::Effort {
                effort: ReasoningEffort::None,
                exclude: false,
                summary: ReasoningSummaryVerbosity::Detailed,
            }),
            ReasoningConfig::Low => new!(CompletionReasoningRequest::Effort {
                effort: ReasoningEffort::Low,
                exclude: false,
                summary: ReasoningSummaryVerbosity::Detailed,
            }),
            ReasoningConfig::Medium => new!(CompletionReasoningRequest::Effort {
                effort: ReasoningEffort::Medium,
                exclude: false,
                summary: ReasoningSummaryVerbosity::Detailed,
            }),
            ReasoningConfig::High => new!(CompletionReasoningRequest::Effort {
                effort: ReasoningEffort::High,
                exclude: false,
                summary: ReasoningSummaryVerbosity::Detailed,
            }),
        }
    }
}

/// OpenRouter's requested verbosity for provider-generated reasoning summaries.
#[invariant(::Detailed => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
enum ReasoningSummaryVerbosity {
    /// Request the most detailed reasoning summary the provider supports.
    Detailed,
}

#[invariant(::None => true)]
#[invariant(::Low => true)]
#[invariant(::Medium => true)]
#[invariant(::High => true)]
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
enum ReasoningEffort {
    None,
    Low,
    Medium,
    High,
}

/// Request-scoped view over stored history.
///
/// The plain path delegates to `ChatMessage` serialization byte-for-byte. The
/// explicit path wraps only the two messages selected for this request; it does
/// not mutate history, so the moving final breakpoint naturally advances.
#[invariant(!messages.is_empty(), "completion requests require message history")]
#[invariant(reasoning_details_replay.iter().all(|replay| replay.assistant_message_index < messages.len()), "reasoning replay indices must address request history")]
#[derive(Debug)]
struct CompletionMessages<'a> {
    messages: &'a [ChatMessage],
    explicit_prompt_caching: bool,
    reasoning_details_replay: &'a [ReasoningDetailsReplay],
}

impl Serialize for CompletionMessages<'_> {
    #[requires(true)]
    #[ensures(true)]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if !self.explicit_prompt_caching && self.reasoning_details_replay.is_empty() {
            return self.messages.serialize(serializer);
        }

        let final_index = self.messages.len().saturating_sub(1);
        let mut sequence = serializer.serialize_seq(Some(self.messages.len()))?;
        for (index, message) in self.messages.iter().enumerate() {
            let cache_breakpoint = index == 0 || index == final_index;
            let reasoning_details = self
                .reasoning_details_replay
                .iter()
                .find(|replay| replay.assistant_message_index == index)
                .map(|replay| replay.reasoning_details.as_slice());
            let message =
                RequestChatMessage::from_message(message, cache_breakpoint, reasoning_details)
                    .map_err(S::Error::custom)?;
            sequence.serialize_element(&message)?;
        }
        sequence.end()
    }
}

/// Borrowed message representation used only when a request has breakpoints.
#[invariant(::System { .. } => true, "data is borrowed from an invariant-bearing ChatMessage")]
#[invariant(::User { .. } => true, "data is borrowed from an invariant-bearing ChatMessage")]
#[invariant(::Assistant { .. } => true, "data is borrowed from an invariant-bearing ChatMessage")]
#[invariant(::Tool { .. } => true, "data is borrowed from an invariant-bearing ChatMessage")]
#[derive(Debug, Serialize)]
#[serde(tag = "role", rename_all = "lowercase")]
enum RequestChatMessage<'a> {
    System {
        content: RequestMessageContent<'a>,
    },
    User {
        content: RequestMessageContent<'a>,
    },
    Assistant {
        content: Option<RequestMessageContent<'a>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        tool_calls: Option<&'a [ToolCall]>,
        #[serde(skip_serializing_if = "Option::is_none")]
        reasoning_details: Option<&'a [Value]>,
    },
    Tool {
        tool_call_id: &'a str,
        name: &'a str,
        content: RequestMessageContent<'a>,
    },
}

impl<'a> RequestChatMessage<'a> {
    #[requires(true)]
    #[ensures(ret.is_ok() || cache_breakpoint)]
    fn from_message(
        message: &'a ChatMessage,
        cache_breakpoint: bool,
        reasoning_details: Option<&'a [Value]>,
    ) -> Result<Self, &'static str> {
        if reasoning_details.is_some()
            && !matches!(
                message.as_data(),
                bityzba::data!(ChatMessage::Assistant { .. })
            )
        {
            return Err("reasoning details can be replayed only on an assistant message");
        }
        match message.as_data() {
            bityzba::data!(ChatMessage::System { content }) => Ok(Self::System {
                content: RequestMessageContent::new(content, cache_breakpoint),
            }),
            bityzba::data!(ChatMessage::User { content }) => Ok(Self::User {
                content: RequestMessageContent::new(content, cache_breakpoint),
            }),
            bityzba::data!(ChatMessage::Assistant {
                content,
                tool_calls,
            }) => {
                if cache_breakpoint && content.is_none() {
                    return Err(
                        "the final prompt-cache breakpoint requires textual message content",
                    );
                }
                Ok(Self::Assistant {
                    content: content
                        .as_deref()
                        .map(|content| RequestMessageContent::new(content, cache_breakpoint)),
                    tool_calls: (!tool_calls.is_empty()).then_some(tool_calls),
                    reasoning_details,
                })
            }
            bityzba::data!(ChatMessage::Tool {
                tool_call_id,
                name,
                content,
            }) => Ok(Self::Tool {
                tool_call_id,
                name,
                content: RequestMessageContent::new(content, cache_breakpoint),
            }),
        }
    }
}

/// A string on the legacy path or one cacheable text part on the explicit path.
#[invariant(true, "text is borrowed exactly from the stored ChatMessage")]
#[derive(Debug)]
struct RequestMessageContent<'a> {
    text: &'a str,
    cache_breakpoint: bool,
}

impl<'a> RequestMessageContent<'a> {
    #[requires(true)]
    #[ensures(ret.text == text && ret.cache_breakpoint == cache_breakpoint)]
    fn new(text: &'a str, cache_breakpoint: bool) -> Self {
        Self {
            text,
            cache_breakpoint,
        }
    }
}

impl Serialize for RequestMessageContent<'_> {
    #[requires(true)]
    #[ensures(true)]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if self.cache_breakpoint {
            [CacheableTextPart {
                kind: ContentPartKind::Text,
                text: self.text,
                cache_control: CacheControl {
                    kind: CacheControlKind::Ephemeral,
                },
            }]
            .serialize(serializer)
        } else {
            self.text.serialize(serializer)
        }
    }
}

#[invariant(
    true,
    "all fields are fixed protocol constants or borrowed message text"
)]
#[derive(Debug, Serialize)]
struct CacheableTextPart<'a> {
    #[serde(rename = "type")]
    kind: ContentPartKind,
    text: &'a str,
    cache_control: CacheControl,
}

#[invariant(::Text => true)]
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
enum ContentPartKind {
    Text,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, Serialize)]
struct CacheControl {
    #[serde(rename = "type")]
    kind: CacheControlKind,
}

#[invariant(::Ephemeral => true)]
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
enum CacheControlKind {
    Ephemeral,
}

#[invariant(*include, "usage accounting must be explicitly requested")]
#[derive(Debug, Serialize)]
struct CompletionUsageRequest {
    include: bool,
}

#[invariant(true)]
#[derive(Debug, Deserialize)]
struct CompletionResponse {
    #[serde(default)]
    choices: Option<Vec<CompletionChoice>>,
    #[serde(default)]
    usage: ProviderUsage,
    #[serde(default)]
    provider: Option<String>,
    #[serde(default)]
    error: Option<ProviderError>,
}

/// Error body returned after OpenRouter has already committed HTTP success.
#[invariant(true, "provider fields are validated before classification")]
#[derive(Debug, Deserialize)]
struct ProviderError {
    code: u16,
    message: String,
    #[serde(default, rename = "metadata")]
    _metadata: Option<Value>,
}

/// OpenRouter's provider response shape before transcript normalization.
#[invariant(true, "provider data is validated while converting into Usage")]
#[derive(Debug, Default, Deserialize)]
struct ProviderUsage {
    #[serde(default)]
    prompt_tokens: u64,
    #[serde(default)]
    completion_tokens: u64,
    #[serde(default)]
    total_tokens: u64,
    #[serde(default)]
    prompt_tokens_details: Option<PromptTokensDetails>,
    #[serde(default)]
    completion_tokens_details: Option<CompletionTokensDetails>,
    #[serde(default)]
    cache_write_tokens: Option<u64>,
    #[serde(default)]
    cost: f64,
}

impl ProviderUsage {
    #[requires(true)]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    fn into_usage(
        self,
        reasoning_present: bool,
        provider: Option<String>,
    ) -> Result<Usage, OpenRouterError> {
        if !self.cost.is_finite() || self.cost < 0.0 {
            return Err(OpenRouterError::InvalidProviderUsage {
                reason: ProviderUsageValidationError::CostMustBeFiniteAndNonnegative,
            });
        }
        if provider
            .as_ref()
            .is_some_and(|provider| provider.trim().is_empty())
        {
            return Err(OpenRouterError::InvalidProviderUsage {
                reason: ProviderUsageValidationError::ProviderMustNotBeEmpty,
            });
        }

        Ok(new!(Usage {
            provider,
            prompt_tokens: self.prompt_tokens,
            completion_tokens: self.completion_tokens,
            total_tokens: self.total_tokens,
            cached_tokens: self
                .prompt_tokens_details
                .and_then(|details| details.cached_tokens),
            cache_write_tokens: self.cache_write_tokens,
            reasoning_present,
            reasoning_tokens: self
                .completion_tokens_details
                .and_then(|details| details.reasoning_tokens),
            cost: self.cost,
        }))
    }
}

#[invariant(true, "provider data is validated while converting into Usage")]
#[derive(Debug, Default, Deserialize)]
struct PromptTokensDetails {
    #[serde(default)]
    cached_tokens: Option<u64>,
}

#[invariant(true, "provider data is validated while converting into Usage")]
#[derive(Debug, Default, Deserialize)]
struct CompletionTokensDetails {
    #[serde(default)]
    reasoning_tokens: Option<u64>,
}

#[invariant(true)]
#[derive(Debug, Deserialize)]
struct CompletionChoice {
    message: CompletionMessage,
}

#[invariant(true)]
#[derive(Debug, Deserialize)]
struct CompletionMessage {
    content: Option<String>,
    #[serde(default)]
    reasoning: Option<String>,
    #[serde(default)]
    reasoning_details: Option<Vec<Value>>,
    #[serde(default)]
    tool_calls: Vec<ToolCall>,
}

#[invariant(true)]
#[derive(Debug)]
struct Completion {
    content: Option<String>,
    tool_calls: Vec<ToolCall>,
    thinking: Option<ThinkingTrace>,
    usage: Usage,
}
