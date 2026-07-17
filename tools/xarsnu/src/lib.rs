//! Runtime foundations for the xarsnu dialog simulation lab.

pub mod config;
pub mod jbotci_tools;
pub mod openrouter;
pub mod protocol;
pub mod report;
pub mod run;
pub mod scenario;
pub mod transcript;

pub use config::{
    CapsConfig, ConfigError, ParticipantConfig, PromptCaching, RunConfig, TersmuFormat,
};
pub use jbotci_tools::{
    DiagnosticCategory, GateError, GateOutcome, ReferenceToolError, ReferenceTools, gate_lojban,
};
pub use openrouter::{
    AbortKind, AbortRecord, ChatMessage, FunctionCall, FunctionToolDefinition, ModelTurn,
    OpenRouterClient, OpenRouterClientConfig, OpenRouterError, ParticipantConversation,
    RetryPolicy, RunAccounting, ToolCall, ToolChoice, ToolDefinition, ToolDefinitionError,
    ToolDispatchError, ToolDispatcher, Usage, UsageTotals,
};
pub use protocol::{
    BlindMessage, ListenerPhase, ListenerState, OpenRouterParticipant, ProtocolEvent,
    ProtocolModel, ProtocolModelError, ProtocolPhase, ProtocolRunError, ProtocolRunOutcome,
    ProtocolRunner, ProtocolTool, ProtocolTools, ReferenceToolDispatcher, RevealedMessage,
    RuntimeFailureRecord, RuntimeFailureSite, SpeakerPhase, SpeakerState, TurnForfeitReason,
    VisibleMessage,
};
pub use report::report_file;
pub use run::{RunError, RunSummary, run};
pub use scenario::{
    Assignment, DeductionAnswer, ParticipantTaskOutcome, ReferentialAnswer, ScenarioAnswer,
    ScenarioAnswerError, ScenarioConfigError, ScenarioInstance, ScenarioKind, ScenarioParticipant,
    ScheduleAnswer, TaskOutcome, TaskStatus, Weekday,
};
pub use transcript::{
    RunHeader, TRANSCRIPT_SCHEMA_VERSION, TranscriptError, TranscriptErrorKind, TranscriptRecord,
    read_transcript,
};
