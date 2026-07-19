//! Runtime foundations for the xarsnu dialog simulation lab.

pub mod config;
pub mod jbotci_tools;
mod model_capabilities;
pub mod openrouter;
pub mod protocol;
pub mod report;
pub mod run;
pub mod scenario;
pub mod transcript;

pub use config::{
    CapsConfig, ClientConfig, ConfigError, ListenerMode, ParticipantConfig, PromptCaching,
    ReasoningConfig, RunConfig, TersmuFormat, ToolChoice,
};
pub use jbotci_tools::{
    DiagnosticCategory, EmbeddingSearchPreflightError, GateError, GateOutcome, ReferenceToolError,
    ReferenceTools, gate_lojban, preflight_embedding_search,
};
pub use model_capabilities::ProviderToolChoice;
pub use openrouter::{
    AbortKind, AbortRecord, ChatMessage, FunctionCall, FunctionToolDefinition, ModelTurn,
    OpenRouterClient, OpenRouterClientConfig, OpenRouterError, ParticipantConversation,
    ProviderCallObservation, ProviderUsageValidationError, RetryPolicy, RunAccounting,
    ThinkingTrace, ToolCall, ToolDefinition, ToolDefinitionError, ToolDispatchError,
    ToolDispatcher, Usage, UsageTotals,
};
pub use protocol::{
    BlindMessage, ListenerFlowAbandonReason, ListenerPhase, ListenerState, OpenRouterParticipant,
    ProtocolEvent, ProtocolModel, ProtocolModelError, ProtocolPhase, ProtocolRunError,
    ProtocolRunOutcome, ProtocolRunner, ProtocolTool, ProtocolTools, ReferenceToolDispatcher,
    RevealedMessage, RuntimeFailureRecord, RuntimeFailureSite, SpeakerPhase, SpeakerState,
    TurnForfeitReason, VisibleMessage,
};
pub use report::{DialogReportError, community_file, dialog_file, report_file};
pub use run::{
    RunError, RunSummary, RunWarning, run, run_with_preflight, run_with_warning_handler,
};
pub use scenario::{
    Assignment, DeductionAnswer, ParticipantTaskOutcome, ReferentialAnswer, ScenarioAnswer,
    ScenarioAnswerError, ScenarioConfigError, ScenarioInstance, ScenarioKind, ScenarioParticipant,
    ScheduleAnswer, TaskOutcome, TaskStatus, Weekday,
};
pub use transcript::{
    RunHeader, TRANSCRIPT_SCHEMA_VERSION, TranscriptError, TranscriptErrorKind, TranscriptRecord,
    read_transcript,
};
