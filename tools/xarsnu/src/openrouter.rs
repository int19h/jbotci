//! Sequential OpenRouter chat-completions runtime with dynamic tool sets.

use std::fmt;
use std::thread;
use std::time::Duration;

#[allow(unused_imports)]
use bityzba::{contract_trait, ensures, invariant, new, requires};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

const DEFAULT_OPENROUTER_BASE_URL: &str = "https://openrouter.ai/api/v1";
const REQUIRED_TOOL_CORRECTION: &str =
    "You must respond by calling one of the provided tools. Do not answer with prose.";
const SKIPPED_INVALID_BATCH_CALL: &str = "This tool call was not executed because another tool call in the same response had invalid arguments.";

/// Whether the model may answer with prose or must select a tool.
#[invariant(::Auto => true)]
#[invariant(::Required => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolChoice {
    Auto,
    Required,
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
#[invariant(cost.is_finite() && *cost >= 0.0, "reported cost must be finite and nonnegative")]
#[invariant(prompt_tokens.checked_add(*completion_tokens) == Some(*total_tokens), "total tokens must equal prompt plus completion tokens")]
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct Usage {
    #[serde(default)]
    pub prompt_tokens: u64,
    #[serde(default)]
    pub completion_tokens: u64,
    #[serde(default)]
    pub total_tokens: u64,
    #[serde(default)]
    pub cost: f64,
}

/// Accumulated token and cost totals.
#[invariant(
    true,
    "mutated only by UsageTotals::record, which preserves nonnegative totals"
)]
#[derive(Debug, Clone, Default, PartialEq)]
pub struct UsageTotals {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
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
        self.cost_usd += usage.cost;
    }
}

/// Why a bounded run stopped without a runtime failure.
#[invariant(::CostBudgetExceeded => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AbortKind {
    CostBudgetExceeded,
}

/// Explicit record surfaced when a graceful run cap is hit.
#[invariant(max_cost_usd.is_finite() && *max_cost_usd > 0.0)]
#[invariant(actual_cost_usd.is_finite() && *actual_cost_usd >= *max_cost_usd)]
#[derive(Debug, Clone, PartialEq)]
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

    /// Issue one completion with the caller's exact tool list.
    #[requires(!model.trim().is_empty())]
    #[requires(temperature.is_finite() && (0.0..=2.0).contains(&temperature))]
    #[requires(!messages.is_empty())]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    fn complete(
        &self,
        model: &str,
        temperature: f64,
        messages: &[ChatMessage],
        tools: &[ToolDefinition],
        tool_choice: ToolChoice,
    ) -> Result<Completion, OpenRouterError> {
        let request = CompletionRequest {
            model,
            temperature,
            messages,
            tools,
            tool_choice,
            usage: new!(CompletionUsageRequest { include: true }),
        };
        let url = format!(
            "{}/chat/completions",
            self.config.base_url.trim_end_matches('/')
        );
        let mut retry = 0usize;
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
            let mut response =
                builder
                    .send(request_body)
                    .map_err(|error| OpenRouterError::Transport {
                        message: error.to_string(),
                    })?;
            let status = response.status().as_u16();
            if status == 429 || (500..=599).contains(&status) {
                if retry < self.config.retry_policy.max_retries {
                    let factor = 1u32.checked_shl(retry.min(31) as u32).unwrap_or(u32::MAX);
                    let delay = self
                        .config
                        .retry_policy
                        .initial_backoff
                        .saturating_mul(factor);
                    thread::sleep(delay);
                    retry += 1;
                    continue;
                }
                let body = response
                    .body_mut()
                    .read_to_string()
                    .unwrap_or_else(|error| format!("unable to read response body: {error}"));
                return Err(OpenRouterError::TransientRetriesExhausted {
                    status,
                    attempts: retry + 1,
                    body,
                });
            }
            if !(200..=299).contains(&status) {
                let body = response
                    .body_mut()
                    .read_to_string()
                    .unwrap_or_else(|error| format!("unable to read response body: {error}"));
                return Err(OpenRouterError::HttpStatus { status, body });
            }
            let response_body = response.body_mut().read_to_string().map_err(|error| {
                OpenRouterError::InvalidResponse {
                    message: error.to_string(),
                }
            })?;
            let wire: CompletionResponse =
                serde_json::from_str(&response_body).map_err(|error| {
                    OpenRouterError::InvalidResponse {
                        message: error.to_string(),
                    }
                })?;
            let choice = wire.choices.into_iter().next().ok_or_else(|| {
                OpenRouterError::InvalidResponse {
                    message: "OpenRouter returned no completion choices".to_owned(),
                }
            })?;
            let content = choice.message.content.filter(|content| !content.is_empty());
            if content.is_none() && choice.message.tool_calls.is_empty() {
                return Err(OpenRouterError::InvalidResponse {
                    message: "assistant response contained neither content nor tool calls"
                        .to_owned(),
                });
            }
            return Ok(Completion {
                content,
                tool_calls: choice.message.tool_calls,
                usage: wire.usage,
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
    temperature: f64,
    messages: Vec<ChatMessage>,
    usage: UsageTotals,
}

impl ParticipantConversation {
    /// Seed a participant's private channel with its persona and private brief.
    #[requires(true)]
    #[ensures(ret.messages.len() == 2)]
    pub fn new(participant: &crate::ParticipantConfig) -> Self {
        Self {
            participant_name: participant.name.clone(),
            model: participant.model.clone(),
            temperature: participant.temperature,
            messages: vec![
                ChatMessage::system(participant.system_prompt.clone()),
                ChatMessage::user(participant.private_brief.clone()),
            ],
            usage: UsageTotals::default(),
        }
    }

    /// Seed a conversation directly, primarily for protocol composition and offline tests.
    #[requires(!participant_name.trim().is_empty())]
    #[requires(!model.trim().is_empty())]
    #[requires(temperature.is_finite() && (0.0..=2.0).contains(&temperature))]
    #[requires(!system_prompt.trim().is_empty())]
    #[requires(!private_brief.trim().is_empty())]
    #[ensures(ret.messages.len() == 2)]
    pub fn from_parts(
        participant_name: String,
        model: String,
        temperature: f64,
        system_prompt: String,
        private_brief: String,
    ) -> Self {
        Self {
            participant_name,
            model,
            temperature,
            messages: vec![
                ChatMessage::system(system_prompt),
                ChatMessage::user(private_brief),
            ],
            usage: UsageTotals::default(),
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

    /// Add a private user/protocol instruction before the next inference.
    #[requires(!content.trim().is_empty())]
    #[ensures(self.messages.len() == old(self.messages.len()) + 1)]
    pub fn push_user(&mut self, content: String) {
        self.messages.push(ChatMessage::user(content));
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
        tool_choice: ToolChoice,
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
                self.temperature,
                &self.messages,
                tools,
                tool_choice,
            )?;
            self.usage.record(&completion.usage);
            let abort = accounting.record(&completion.usage);
            let Completion {
                content,
                tool_calls,
                ..
            } = completion;
            self.messages
                .push(ChatMessage::assistant(content.clone(), tool_calls.clone()));
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
            if tool_choice == ToolChoice::Auto {
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
#[derive(Debug, Error, PartialEq, Eq)]
#[invariant(true)]
#[invariant(::MissingApiKey => true)]
#[invariant(::InvalidConfiguration { .. } => true)]
#[invariant(::Transport { .. } => true)]
#[invariant(::HttpStatus { .. } => true)]
#[invariant(::TransientRetriesExhausted { .. } => true)]
#[invariant(::InvalidResponse { .. } => true)]
#[invariant(::InvalidToolCall { .. } => true)]
#[invariant(::RequiredToolCallExhausted { .. } => true)]
pub enum OpenRouterError {
    #[error("OPENROUTER_API_KEY is not set")]
    MissingApiKey,
    #[error("invalid OpenRouter configuration: {message}")]
    InvalidConfiguration { message: String },
    #[error("OpenRouter transport failed: {message}")]
    Transport { message: String },
    #[error("OpenRouter returned HTTP {status}: {body}")]
    HttpStatus { status: u16, body: String },
    #[error("OpenRouter transient HTTP {status} exhausted after {attempts} attempts: {body}")]
    TransientRetriesExhausted {
        status: u16,
        attempts: usize,
        body: String,
    },
    #[error("invalid OpenRouter response: {message}")]
    InvalidResponse { message: String },
    #[error("invalid call to tool `{tool_name}`: {message}")]
    InvalidToolCall { tool_name: String, message: String },
    #[error("required tool call was not produced after {attempts} attempts")]
    RequiredToolCallExhausted { attempts: usize },
}

#[invariant(true)]
#[derive(Debug, Serialize)]
struct CompletionRequest<'a> {
    model: &'a str,
    temperature: f64,
    messages: &'a [ChatMessage],
    tools: &'a [ToolDefinition],
    tool_choice: ToolChoice,
    usage: CompletionUsageRequest,
}

#[invariant(*include, "usage accounting must be explicitly requested")]
#[derive(Debug, Serialize)]
struct CompletionUsageRequest {
    include: bool,
}

#[invariant(true)]
#[derive(Debug, Deserialize)]
struct CompletionResponse {
    choices: Vec<CompletionChoice>,
    #[serde(default)]
    usage: Usage,
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
    tool_calls: Vec<ToolCall>,
}

#[invariant(true)]
#[derive(Debug)]
struct Completion {
    content: Option<String>,
    tool_calls: Vec<ToolCall>,
    usage: Usage,
}
