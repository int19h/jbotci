//! Runtime foundations for the xarsnu dialog simulation lab.

pub mod config;
pub mod openrouter;

pub use config::{CapsConfig, ConfigError, ParticipantConfig, RunConfig, TersmuFormat};
pub use openrouter::{
    AbortKind, AbortRecord, ChatMessage, FunctionCall, FunctionToolDefinition, ModelTurn,
    OpenRouterClient, OpenRouterClientConfig, OpenRouterError, ParticipantConversation,
    RetryPolicy, RunAccounting, ToolCall, ToolChoice, ToolDefinition, ToolDefinitionError,
    ToolDispatchError, ToolDispatcher, Usage, UsageTotals,
};
