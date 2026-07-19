//! Tool-gated speaker and listener protocol for xarsnu runs.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Write as _};
use std::path::Path;

#[allow(unused_imports)]
use bityzba::{contract_trait, ensures, invariant, new, requires};
use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::model_capabilities::ParticipantModelPolicy;
use crate::openrouter::REQUIRED_TOOL_CORRECTION;
use crate::transcript::TranscriptWriter;
use crate::{
    AbortRecord, CapsConfig, DiagnosticCategory, ListenerMode, OpenRouterClient, ParticipantConfig,
    ParticipantConversation, ProviderCallObservation, ProviderToolChoice, ReferenceTools,
    RunAccounting, RunHeader, TersmuFormat, ThinkingTrace, ToolCall, ToolDefinition,
    ToolDefinitionError, ToolDispatchError, ToolDispatcher, TranscriptError, Usage,
};
use crate::{ScenarioAnswer, ScenarioInstance, TaskOutcome};

const STANDING_PROTOCOL_RULES: &str = "You are participating in xarsnu's tool-gated Lojban discussion protocol. Think out loud about meaning in this private conversation before acting. English intents, paraphrases, interpretations, and reasoning are private and must never be written into the visible chat. The visible chat is constructed only from confirmed Lojban text and its jbotci tersmu rendering. Your built-in knowledge of Lojban vocabulary is flawed. Before choosing any content word you are not certain of, search vlacku BY MEANING (semantic or definition search) and compare candidates; do not merely look up a word you already picked and rationalize its definition; when the problem is HOW TO EXPRESS something grammatically (not which word), query cukta with the concept. Use the reference tools whenever they help, and finish each phase by calling the protocol tool currently offered.";
const SPEAKER_TURN_INSTRUCTION: &str = "You are the speaker for this turn. First register your intended meaning in English. Then submit candidate Lojban until jbotci accepts one. Finally compare the returned tersmu rendering with your intent and call confirm_meaning with a mandatory English paraphrase. Call confirm_meaning with matches=true only when the tersmu rendering captures the registered intent precisely: every predicate relation as rendered, under its dictionary place structure, must be the intended relation. Calques or idioms from other languages (malgli) are mismatches even when a listener would get the gist. For example, if the rendering literally says that one person physically chases another, it does not express the colloquial English sense of following or agreeing with someone: call matches=false and recompose. Clear enough in context and the gist is right are not the standard; the rendering is the meaning that will be scored. The discrepancies field is only for renderings that match precisely but have caveats worth recording, not for waiving a known mismatch. A mismatch requires revision; a match posts the Lojban and tersmu rendering.";
const LISTENER_BLIND_INSTRUCTION: &str = "Interpret the following visible Lojban message without access to its tersmu rendering. Think privately, then call interpret_blind with your English interpretation.";
const LISTENER_REVEAL_INSTRUCTION: &str = "The tersmu rendering is now revealed. Compare it with your blind reading, then call acknowledge with your final English understanding and any discrepancies.";
const LISTENER_INFORMED_INSTRUCTION: &str = "Interpret the following visible Lojban message with its tersmu rendering and definitions available from the start. Think privately, then call acknowledge with your final English understanding and any discrepancies.";
const ANSWER_AVAILABLE_INSTRUCTION: &str = "You may now submit your scenario answer with `submit_answer`. The task is scored only from formal submissions; in-dialog agreement does not count.";
const CLOSED_DIALOG_ANSWER_INSTRUCTION: &str = "The visible-channel dialog is now closed. No participant can post or see any further dialog. Submit your scenario answer independently with `submit_answer`, based only on the dialog through the final posted description. The task is scored only from formal submissions; in-dialog agreement does not count.";
const REFERENCE_TOOL_NAMES: [&str; 5] = ["vlacku", "gentufa", "tersmu", "jvozba", "cukta"];

/// One of the six state-changing protocol tools.
#[invariant(::RegisterIntent => true)]
#[invariant(::SubmitLojban => true)]
#[invariant(::ConfirmMeaning => true)]
#[invariant(::InterpretBlind => true)]
#[invariant(::Acknowledge => true)]
#[invariant(::SubmitAnswer => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProtocolTool {
    RegisterIntent,
    SubmitLojban,
    ConfirmMeaning,
    InterpretBlind,
    Acknowledge,
    SubmitAnswer,
}

impl ProtocolTool {
    /// Exhaustive protocol-tool list, used by legality checks and tests.
    pub const ALL: [Self; 6] = [
        Self::RegisterIntent,
        Self::SubmitLojban,
        Self::ConfirmMeaning,
        Self::InterpretBlind,
        Self::Acknowledge,
        Self::SubmitAnswer,
    ];

    /// Stable model-facing tool name.
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub const fn name(self) -> &'static str {
        match self {
            Self::RegisterIntent => "register_intent",
            Self::SubmitLojban => "submit_lojban",
            Self::ConfirmMeaning => "confirm_meaning",
            Self::InterpretBlind => "interpret_blind",
            Self::Acknowledge => "acknowledge",
            Self::SubmitAnswer => "submit_answer",
        }
    }

    /// Classify a model-facing name as a protocol tool.
    #[requires(true)]
    #[ensures(ret.is_some_and(|tool| tool.name() == name) || ret.is_none())]
    fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|tool| tool.name() == name)
    }
}

/// Speaker-side state name.
#[invariant(::AwaitingIntent => true)]
#[invariant(::Composing => true)]
#[invariant(::AwaitingConfirmation => true)]
#[invariant(::Posted => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SpeakerPhase {
    AwaitingIntent,
    Composing,
    AwaitingConfirmation,
    Posted,
}

impl SpeakerPhase {
    /// Exhaustive speaker-state list.
    pub const ALL: [Self; 4] = [
        Self::AwaitingIntent,
        Self::Composing,
        Self::AwaitingConfirmation,
        Self::Posted,
    ];

    /// Whether this state admits the requested protocol tool.
    #[requires(true)]
    #[ensures(true)]
    pub const fn allows(self, tool: ProtocolTool, submit_answer_available: bool) -> bool {
        (submit_answer_available && matches!(tool, ProtocolTool::SubmitAnswer))
            || matches!(
                (self, tool),
                (Self::AwaitingIntent, ProtocolTool::RegisterIntent)
                    | (Self::Composing, ProtocolTool::RegisterIntent)
                    | (Self::Composing, ProtocolTool::SubmitLojban)
                    | (Self::AwaitingConfirmation, ProtocolTool::ConfirmMeaning)
            )
    }
}

/// Listener-side state name.
#[invariant(::InformedInterpretation => true)]
#[invariant(::BlindInterpretation => true)]
#[invariant(::TersmuRevealed => true)]
#[invariant(::Acknowledged => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ListenerPhase {
    InformedInterpretation,
    BlindInterpretation,
    TersmuRevealed,
    Acknowledged,
}

impl ListenerPhase {
    /// Exhaustive listener-state list.
    pub const ALL: [Self; 4] = [
        Self::InformedInterpretation,
        Self::BlindInterpretation,
        Self::TersmuRevealed,
        Self::Acknowledged,
    ];

    /// Whether this state admits the requested protocol tool.
    #[requires(true)]
    #[ensures(true)]
    pub const fn allows(self, tool: ProtocolTool, submit_answer_available: bool) -> bool {
        (submit_answer_available && matches!(tool, ProtocolTool::SubmitAnswer))
            || matches!(
                (self, tool),
                (Self::InformedInterpretation, ProtocolTool::Acknowledge)
                    | (Self::BlindInterpretation, ProtocolTool::InterpretBlind)
                    | (Self::TersmuRevealed, ProtocolTool::Acknowledge)
            )
    }
}

/// Unified state name stored in events and used by the tool gate.
#[invariant(::Speaker { .. } => true)]
#[invariant(::Listener { .. } => true)]
#[invariant(::Answer => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "role", content = "phase", rename_all = "kebab-case")]
pub enum ProtocolPhase {
    Speaker { phase: SpeakerPhase },
    Listener { phase: ListenerPhase },
    Answer,
}

impl ProtocolPhase {
    /// Exhaustive protocol-state list.
    pub const ALL: [Self; 9] = [
        Self::Speaker {
            phase: SpeakerPhase::AwaitingIntent,
        },
        Self::Speaker {
            phase: SpeakerPhase::Composing,
        },
        Self::Speaker {
            phase: SpeakerPhase::AwaitingConfirmation,
        },
        Self::Speaker {
            phase: SpeakerPhase::Posted,
        },
        Self::Listener {
            phase: ListenerPhase::InformedInterpretation,
        },
        Self::Listener {
            phase: ListenerPhase::BlindInterpretation,
        },
        Self::Listener {
            phase: ListenerPhase::TersmuRevealed,
        },
        Self::Listener {
            phase: ListenerPhase::Acknowledged,
        },
        Self::Answer,
    ];

    /// Whether this state admits the requested protocol tool.
    #[requires(true)]
    #[ensures(true)]
    pub const fn allows(self, tool: ProtocolTool, submit_answer_available: bool) -> bool {
        match self {
            Self::Speaker { phase } => phase.allows(tool, submit_answer_available),
            Self::Listener { phase } => phase.allows(tool, submit_answer_available),
            Self::Answer => submit_answer_available && matches!(tool, ProtocolTool::SubmitAnswer),
        }
    }

    /// Human-readable goal of the current protocol state for corrective nudges.
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    const fn intent(self) -> &'static str {
        match self {
            Self::Speaker {
                phase: SpeakerPhase::AwaitingIntent,
            } => "register the intended meaning before composing Lojban",
            Self::Speaker {
                phase: SpeakerPhase::Composing,
            } => "compose or revise Lojban for the registered intent and submit it",
            Self::Speaker {
                phase: SpeakerPhase::AwaitingConfirmation,
            } => "compare the accepted tersmu rendering with the registered intent",
            Self::Speaker {
                phase: SpeakerPhase::Posted,
            } => "complete any available scenario answer",
            Self::Listener {
                phase: ListenerPhase::InformedInterpretation,
            } => "record an informed understanding from the Lojban and tersmu rendering",
            Self::Listener {
                phase: ListenerPhase::BlindInterpretation,
            } => "commit an interpretation of the visible Lojban before seeing tersmu",
            Self::Listener {
                phase: ListenerPhase::TersmuRevealed,
            } => "record final understanding after comparing the tersmu rendering",
            Self::Listener {
                phase: ListenerPhase::Acknowledged,
            } => "complete any available scenario answer",
            Self::Answer => "submit the independently scored scenario answer",
        }
    }
}

/// A confirmed visible-channel message. It cannot carry English private data.
#[invariant(!text.trim().is_empty(), "visible Lojban text cannot be empty")]
#[invariant(!tersmu_rendering.is_empty(), "visible tersmu rendering cannot be empty")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct VisibleMessage {
    pub text: String,
    pub tersmu_rendering: Vec<u8>,
}

impl VisibleMessage {
    /// Strip a visible message down to the structurally blind listener view.
    #[requires(true)]
    #[ensures(ret.text == self.text)]
    pub fn blind(&self) -> BlindMessage {
        new!(BlindMessage {
            text: self.text.clone(),
        })
    }

    /// Construct the post-interpretation listener view.
    #[requires(true)]
    #[ensures(ret.text == self.text)]
    pub fn revealed(&self) -> RevealedMessage {
        new!(RevealedMessage {
            text: self.text.clone(),
            tersmu_rendering: self.tersmu_rendering.clone(),
        })
    }
}

/// Listener-visible content before interpretation; no tersmu field exists.
#[invariant(!text.trim().is_empty(), "blind listener text cannot be empty")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct BlindMessage {
    pub text: String,
}

impl BlindMessage {
    /// Build the blind-phase prompt from the blind-only type.
    #[requires(true)]
    #[ensures(ret.contains(&self.text))]
    pub fn prompt(&self) -> String {
        format!(
            "{LISTENER_BLIND_INSTRUCTION}\n\nLojban message:\n{}",
            self.text
        )
    }
}

/// Listener-visible content after its blind interpretation is committed.
#[invariant(!text.trim().is_empty(), "revealed listener text cannot be empty")]
#[invariant(!tersmu_rendering.is_empty(), "revealed tersmu rendering cannot be empty")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct RevealedMessage {
    pub text: String,
    pub tersmu_rendering: Vec<u8>,
}

impl RevealedMessage {
    /// Build the one-step informed-listener prompt.
    #[requires(true)]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    pub fn informed_prompt(&self) -> Result<String, ProtocolRunError> {
        let rendering = self.rendering()?;
        Ok(format!(
            "{LISTENER_INFORMED_INSTRUCTION}\n\nLojban message:\n{}\n\ntersmu rendering and definitions:\n{rendering}",
            self.text
        ))
    }

    /// Build the reveal prompt, rejecting an impossible non-UTF-8 rendering.
    #[requires(true)]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    pub fn prompt(&self) -> Result<String, ProtocolRunError> {
        let rendering = self.rendering()?;
        Ok(format!(
            "{LISTENER_REVEAL_INSTRUCTION}\n\nLojban message:\n{}\n\ntersmu rendering:\n{rendering}",
            self.text
        ))
    }

    #[requires(true)]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    fn rendering(&self) -> Result<&str, ProtocolRunError> {
        std::str::from_utf8(&self.tersmu_rendering).map_err(|error| {
            new!(ProtocolRunError::InvalidTersmuEncoding {
                message: error.to_string(),
            })
        })
    }
}

/// Typed speaker machine; each variant owns exactly the data its phase permits.
#[invariant(::AwaitingIntent => true)]
#[invariant(::Composing { meaning_en, .. } => !meaning_en.trim().is_empty())]
#[invariant(::AwaitingConfirmation { meaning_en, candidate, parse_attempts, .. } => !meaning_en.trim().is_empty() && !candidate.text.trim().is_empty() && !candidate.tersmu_rendering.is_empty() && *parse_attempts > 0)]
#[invariant(::Posted { message } => !message.text.trim().is_empty() && !message.tersmu_rendering.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpeakerState {
    AwaitingIntent,
    Composing {
        meaning_en: String,
        intent_revisions: usize,
        parse_attempts: usize,
    },
    AwaitingConfirmation {
        meaning_en: String,
        intent_revisions: usize,
        parse_attempts: usize,
        candidate: VisibleMessage,
    },
    Posted {
        message: VisibleMessage,
    },
}

impl SpeakerState {
    /// Initial speaker state.
    #[requires(true)]
    #[ensures(ret.phase() == SpeakerPhase::AwaitingIntent)]
    pub fn awaiting_intent() -> Self {
        new!(SpeakerState::AwaitingIntent)
    }

    /// Current speaker phase.
    #[requires(true)]
    #[ensures(true)]
    pub fn phase(&self) -> SpeakerPhase {
        match self.as_data() {
            bityzba::data!(SpeakerState::AwaitingIntent) => SpeakerPhase::AwaitingIntent,
            bityzba::data!(SpeakerState::Composing { .. }) => SpeakerPhase::Composing,
            bityzba::data!(SpeakerState::AwaitingConfirmation { .. }) => {
                SpeakerPhase::AwaitingConfirmation
            }
            bityzba::data!(SpeakerState::Posted { .. }) => SpeakerPhase::Posted,
        }
    }
}

/// Typed listener machine. Only its measurement-arm blind variant lacks tersmu.
#[invariant(::InformedInterpretation { message } => !message.text.trim().is_empty() && !message.tersmu_rendering.is_empty())]
#[invariant(::BlindInterpretation { message } => !message.text.trim().is_empty())]
#[invariant(::TersmuRevealed { message, interpretation_en } => !message.text.trim().is_empty() && !message.tersmu_rendering.is_empty() && !interpretation_en.trim().is_empty())]
#[invariant(::Acknowledged { final_understanding_en, discrepancies } => !final_understanding_en.trim().is_empty() && discrepancies.as_ref().is_none_or(|value| !value.trim().is_empty()))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ListenerState {
    InformedInterpretation {
        message: RevealedMessage,
    },
    BlindInterpretation {
        message: BlindMessage,
    },
    TersmuRevealed {
        message: RevealedMessage,
        interpretation_en: String,
    },
    Acknowledged {
        final_understanding_en: String,
        discrepancies: Option<String>,
    },
}

impl ListenerState {
    /// Initial informed state, with the parser rendering available immediately.
    #[requires(true)]
    #[ensures(ret.phase() == ListenerPhase::InformedInterpretation)]
    pub fn informed(message: RevealedMessage) -> Self {
        new!(ListenerState::InformedInterpretation { message })
    }

    /// Initial listener state, constructed only from blind-visible content.
    #[requires(true)]
    #[ensures(ret.phase() == ListenerPhase::BlindInterpretation)]
    pub fn blind(message: BlindMessage) -> Self {
        new!(ListenerState::BlindInterpretation { message })
    }

    /// Current listener phase.
    #[requires(true)]
    #[ensures(true)]
    pub fn phase(&self) -> ListenerPhase {
        match self.as_data() {
            bityzba::data!(ListenerState::InformedInterpretation { .. }) => {
                ListenerPhase::InformedInterpretation
            }
            bityzba::data!(ListenerState::BlindInterpretation { .. }) => {
                ListenerPhase::BlindInterpretation
            }
            bityzba::data!(ListenerState::TersmuRevealed { .. }) => ListenerPhase::TersmuRevealed,
            bityzba::data!(ListenerState::Acknowledged { .. }) => ListenerPhase::Acknowledged,
        }
    }
}

/// Why a bounded speaker turn was forfeited.
#[invariant(::ParseAttempts { maximum } => *maximum > 0)]
#[invariant(::IntentRevisions { maximum } => *maximum > 0)]
#[invariant(::ProtocolProseResponses { maximum_attempts } => *maximum_attempts > 0)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TurnForfeitReason {
    ParseAttempts { maximum: usize },
    IntentRevisions { maximum: usize },
    ProtocolProseResponses { maximum_attempts: usize },
}

/// Why one listener's bounded private flow was abandoned.
#[invariant(::ProtocolProseResponses { maximum_attempts } => *maximum_attempts > 0)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ListenerFlowAbandonReason {
    ProtocolProseResponses { maximum_attempts: usize },
}

/// In-process operation that failed after a run had started.
#[invariant(::ProtocolConfiguration => true)]
#[invariant(::ToolDefinitions => true)]
#[invariant(::ModelRequest => true)]
#[invariant(::JbotciGate => true)]
#[invariant(::TersmuRendering => true)]
#[invariant(::TranscriptWrite => true)]
#[invariant(::RunnerReuse => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RuntimeFailureSite {
    ProtocolConfiguration,
    ToolDefinitions,
    ModelRequest,
    JbotciGate,
    TersmuRendering,
    TranscriptWrite,
    RunnerReuse,
}

impl RuntimeFailureSite {
    /// Stable human-readable call-site name.
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ProtocolConfiguration => "protocol configuration",
            Self::ToolDefinitions => "tool definition construction",
            Self::ModelRequest => "model request",
            Self::JbotciGate => "jbotci gate",
            Self::TersmuRendering => "tersmu rendering",
            Self::TranscriptWrite => "transcript write",
            Self::RunnerReuse => "runner reuse",
        }
    }
}

/// Terminal record for an orderly run that stopped on a runtime error.
#[invariant(*turn_number > 0)]
#[invariant(participant.as_ref().is_none_or(|name| !name.trim().is_empty()))]
#[invariant(!message.trim().is_empty())]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct RuntimeFailureRecord {
    pub turn_number: usize,
    pub participant: Option<String>,
    pub call_site: RuntimeFailureSite,
    pub message: String,
}

/// Typed private event stream consumed by the later transcript layer.
#[invariant(::RunStarted { .. } => true)]
#[invariant(::TurnStarted { turn_number, speaker } => *turn_number > 0 && !speaker.trim().is_empty())]
#[invariant(::IntentRegistered { turn_number, speaker, meaning_en, .. } => *turn_number > 0 && !speaker.trim().is_empty() && !meaning_en.trim().is_empty())]
#[invariant(::CandidateSubmitted { turn_number, speaker, text, attempt } => *turn_number > 0 && !speaker.trim().is_empty() && !text.trim().is_empty() && *attempt > 0)]
#[invariant(::CandidateRejected { turn_number, speaker, text, diagnostics, attempt, .. } => *turn_number > 0 && !speaker.trim().is_empty() && !text.trim().is_empty() && !diagnostics.is_empty() && *attempt > 0)]
#[invariant(::CandidateAccepted { turn_number, speaker, message, attempt } => *turn_number > 0 && !speaker.trim().is_empty() && !message.text.trim().is_empty() && !message.tersmu_rendering.is_empty() && *attempt > 0)]
#[invariant(::MeaningConfirmed { turn_number, speaker, paraphrase_en, discrepancies, .. } => *turn_number > 0 && !speaker.trim().is_empty() && !paraphrase_en.trim().is_empty() && discrepancies.as_ref().is_none_or(|value| !value.trim().is_empty()))]
#[invariant(::MessagePosted { turn_number, speaker, message } => *turn_number > 0 && !speaker.trim().is_empty() && !message.text.trim().is_empty() && !message.tersmu_rendering.is_empty())]
#[invariant(::ListenerFlowStarted { turn_number, speaker, listener, message, .. } => *turn_number > 0 && !speaker.trim().is_empty() && !listener.trim().is_empty() && !message.text.trim().is_empty() && !message.tersmu_rendering.is_empty())]
#[invariant(::BlindInterpretationRecorded { turn_number, speaker, listener, interpretation_en } => *turn_number > 0 && !speaker.trim().is_empty() && !listener.trim().is_empty() && !interpretation_en.trim().is_empty())]
#[invariant(::TersmuRevealed { turn_number, speaker, listener, message } => *turn_number > 0 && !speaker.trim().is_empty() && !listener.trim().is_empty() && !message.text.trim().is_empty() && !message.tersmu_rendering.is_empty())]
#[invariant(::Acknowledged { turn_number, speaker, listener, final_understanding_en, discrepancies } => *turn_number > 0 && !speaker.trim().is_empty() && !listener.trim().is_empty() && !final_understanding_en.trim().is_empty() && discrepancies.as_ref().is_none_or(|value| !value.trim().is_empty()))]
#[invariant(::ReferenceToolCompleted { participant, tool_name, arguments, result, .. } => !participant.trim().is_empty() && !tool_name.trim().is_empty() && !arguments.trim().is_empty() && !result.is_empty())]
#[invariant(::ReferenceLookupRepeated { participant, tool_name, arguments, repeat_number, .. } => !participant.trim().is_empty() && !tool_name.trim().is_empty() && !arguments.trim().is_empty() && *repeat_number >= 2)]
#[invariant(::ReferenceCallBudgetExhausted { participant, maximum, .. } => !participant.trim().is_empty() && *maximum > 0)]
#[invariant(::ReferenceResearchNudge { participant, consecutive_calls, message, .. } => !participant.trim().is_empty() && *consecutive_calls > 0 && !message.trim().is_empty())]
#[invariant(::EmbeddingSearchDegraded { message } => !message.trim().is_empty())]
#[invariant(::ProseRejected { turn_number, participant, attempt, maximum_attempts, .. } => *turn_number > 0 && !participant.trim().is_empty() && *attempt > 0 && *maximum_attempts > 0 && attempt <= maximum_attempts)]
#[invariant(::ListenerFlowAbandoned { turn_number, listener, .. } => *turn_number > 0 && !listener.trim().is_empty())]
#[invariant(::ProtocolError { participant, tool_name, message, .. } => !participant.trim().is_empty() && !tool_name.trim().is_empty() && !message.trim().is_empty())]
#[invariant(::TurnForfeited { turn_number, speaker, .. } => *turn_number > 0 && !speaker.trim().is_empty())]
#[invariant(::DialogClosedForAnswers { turn_number, round_number } => *turn_number > 0 && *round_number > 0)]
#[invariant(::AnswerSubmitted { turn_number, participant, .. } => *turn_number > 0 && !participant.trim().is_empty())]
#[invariant(::CheckerOutcome { turn_number, .. } => *turn_number > 0)]
#[invariant(::UsageRecorded { turn_number, participant, .. } => *turn_number > 0 && !participant.trim().is_empty())]
#[invariant(::ThinkingRecorded { turn_number, participant, .. } => *turn_number > 0 && !participant.trim().is_empty())]
#[invariant(::RunAborted { .. } => true)]
#[invariant(::RunFinished { .. } => true)]
#[invariant(::RunFailed { failure } => failure.turn_number > 0 && !failure.message.trim().is_empty())]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum ProtocolEvent {
    RunStarted {
        header: RunHeader,
    },
    TurnStarted {
        turn_number: usize,
        speaker: String,
    },
    IntentRegistered {
        turn_number: usize,
        speaker: String,
        meaning_en: String,
        revision: bool,
        revision_number: usize,
    },
    CandidateSubmitted {
        turn_number: usize,
        speaker: String,
        text: String,
        attempt: usize,
    },
    CandidateRejected {
        turn_number: usize,
        speaker: String,
        text: String,
        diagnostics: String,
        diagnostic_category: DiagnosticCategory,
        attempt: usize,
    },
    CandidateAccepted {
        turn_number: usize,
        speaker: String,
        message: VisibleMessage,
        attempt: usize,
    },
    MeaningConfirmed {
        turn_number: usize,
        speaker: String,
        matches: bool,
        paraphrase_en: String,
        discrepancies: Option<String>,
    },
    MessagePosted {
        turn_number: usize,
        speaker: String,
        message: VisibleMessage,
    },
    ListenerFlowStarted {
        turn_number: usize,
        speaker: String,
        listener: String,
        mode: ListenerMode,
        message: VisibleMessage,
    },
    BlindInterpretationRecorded {
        turn_number: usize,
        speaker: String,
        listener: String,
        interpretation_en: String,
    },
    TersmuRevealed {
        turn_number: usize,
        speaker: String,
        listener: String,
        message: VisibleMessage,
    },
    Acknowledged {
        turn_number: usize,
        speaker: String,
        listener: String,
        final_understanding_en: String,
        discrepancies: Option<String>,
    },
    ReferenceToolCompleted {
        participant: String,
        phase: ProtocolPhase,
        tool_name: String,
        arguments: String,
        result: String,
        succeeded: bool,
    },
    ReferenceLookupRepeated {
        participant: String,
        phase: ProtocolPhase,
        tool_name: String,
        arguments: String,
        repeat_number: usize,
        remaining_calls: usize,
    },
    ReferenceCallBudgetExhausted {
        participant: String,
        phase: ProtocolPhase,
        maximum: usize,
    },
    ReferenceResearchNudge {
        participant: String,
        phase: ProtocolPhase,
        consecutive_calls: usize,
        message: String,
    },
    EmbeddingSearchDegraded {
        message: String,
    },
    ProseRejected {
        turn_number: usize,
        participant: String,
        phase: ProtocolPhase,
        attempt: usize,
        maximum_attempts: usize,
    },
    ListenerFlowAbandoned {
        turn_number: usize,
        listener: String,
        phase: ProtocolPhase,
        reason: ListenerFlowAbandonReason,
    },
    ProtocolError {
        participant: String,
        phase: ProtocolPhase,
        tool_name: String,
        message: String,
    },
    TurnForfeited {
        turn_number: usize,
        speaker: String,
        reason: TurnForfeitReason,
    },
    DialogClosedForAnswers {
        turn_number: usize,
        round_number: usize,
    },
    AnswerSubmitted {
        turn_number: usize,
        participant: String,
        answer: ScenarioAnswer,
    },
    CheckerOutcome {
        turn_number: usize,
        outcome: TaskOutcome,
    },
    UsageRecorded {
        turn_number: usize,
        participant: String,
        usage: Usage,
    },
    ThinkingRecorded {
        turn_number: usize,
        participant: String,
        trace: ThinkingTrace,
    },
    RunAborted {
        record: AbortRecord,
    },
    RunFinished {
        outcome: ProtocolRunOutcome,
    },
    RunFailed {
        failure: RuntimeFailureRecord,
    },
}

impl ProtocolEvent {
    /// Whether this event is the unique transcript header.
    #[requires(true)]
    #[ensures(ret == matches!(self.as_data(), bityzba::data!(ProtocolEvent::RunStarted { .. })))]
    pub(crate) fn is_run_started(&self) -> bool {
        matches!(
            self.as_data(),
            bityzba::data!(ProtocolEvent::RunStarted { .. })
        )
    }

    /// Whether this event closes an orderly-shutdown transcript.
    #[requires(true)]
    #[ensures(ret == matches!(self.as_data(), bityzba::data!(ProtocolEvent::RunFinished { .. }) | bityzba::data!(ProtocolEvent::RunFailed { .. })))]
    pub(crate) fn is_terminal(&self) -> bool {
        matches!(
            self.as_data(),
            bityzba::data!(ProtocolEvent::RunFinished { .. })
                | bityzba::data!(ProtocolEvent::RunFailed { .. })
        )
    }

    /// Turn number introduced by this event, if it starts a turn.
    #[requires(true)]
    #[ensures(ret.is_some() == matches!(self.as_data(), bityzba::data!(ProtocolEvent::TurnStarted { .. })))]
    pub(crate) fn started_turn_number(&self) -> Option<usize> {
        match self.as_data() {
            bityzba::data!(ProtocolEvent::TurnStarted { turn_number, .. }) => Some(*turn_number),
            _ => None,
        }
    }

    /// Payload turn when this event kind stores one directly.
    #[requires(true)]
    #[ensures(matches!(self.as_data(), bityzba::data!(ProtocolEvent::RunStarted { .. })) == (ret == Some(0)))]
    pub(crate) fn explicit_turn_number(&self) -> Option<usize> {
        match self.as_data() {
            bityzba::data!(ProtocolEvent::RunStarted { .. }) => Some(0),
            bityzba::data!(ProtocolEvent::TurnStarted { turn_number, .. })
            | bityzba::data!(ProtocolEvent::IntentRegistered { turn_number, .. })
            | bityzba::data!(ProtocolEvent::CandidateSubmitted { turn_number, .. })
            | bityzba::data!(ProtocolEvent::CandidateRejected { turn_number, .. })
            | bityzba::data!(ProtocolEvent::CandidateAccepted { turn_number, .. })
            | bityzba::data!(ProtocolEvent::MeaningConfirmed { turn_number, .. })
            | bityzba::data!(ProtocolEvent::MessagePosted { turn_number, .. })
            | bityzba::data!(ProtocolEvent::ListenerFlowStarted { turn_number, .. })
            | bityzba::data!(ProtocolEvent::BlindInterpretationRecorded { turn_number, .. })
            | bityzba::data!(ProtocolEvent::TersmuRevealed { turn_number, .. })
            | bityzba::data!(ProtocolEvent::Acknowledged { turn_number, .. })
            | bityzba::data!(ProtocolEvent::ProseRejected { turn_number, .. })
            | bityzba::data!(ProtocolEvent::ListenerFlowAbandoned { turn_number, .. })
            | bityzba::data!(ProtocolEvent::TurnForfeited { turn_number, .. })
            | bityzba::data!(ProtocolEvent::DialogClosedForAnswers { turn_number, .. })
            | bityzba::data!(ProtocolEvent::AnswerSubmitted { turn_number, .. })
            | bityzba::data!(ProtocolEvent::CheckerOutcome { turn_number, .. })
            | bityzba::data!(ProtocolEvent::UsageRecorded { turn_number, .. })
            | bityzba::data!(ProtocolEvent::ThinkingRecorded { turn_number, .. }) => {
                Some(*turn_number)
            }
            bityzba::data!(ProtocolEvent::ReferenceToolCompleted { .. })
            | bityzba::data!(ProtocolEvent::ReferenceLookupRepeated { .. })
            | bityzba::data!(ProtocolEvent::ReferenceCallBudgetExhausted { .. })
            | bityzba::data!(ProtocolEvent::ReferenceResearchNudge { .. })
            | bityzba::data!(ProtocolEvent::EmbeddingSearchDegraded { .. })
            | bityzba::data!(ProtocolEvent::ProtocolError { .. })
            | bityzba::data!(ProtocolEvent::RunAborted { .. }) => None,
            bityzba::data!(ProtocolEvent::RunFinished { outcome }) => Some(outcome.turns().max(1)),
            bityzba::data!(ProtocolEvent::RunFailed { failure }) => Some(failure.turn_number),
        }
    }

    /// Principal actor serialized in the transcript envelope.
    #[requires(true)]
    #[ensures(!ret.trim().is_empty())]
    pub(crate) fn transcript_participant(&self) -> &str {
        match self.as_data() {
            bityzba::data!(ProtocolEvent::TurnStarted { speaker, .. })
            | bityzba::data!(ProtocolEvent::IntentRegistered { speaker, .. })
            | bityzba::data!(ProtocolEvent::CandidateSubmitted { speaker, .. })
            | bityzba::data!(ProtocolEvent::CandidateRejected { speaker, .. })
            | bityzba::data!(ProtocolEvent::CandidateAccepted { speaker, .. })
            | bityzba::data!(ProtocolEvent::MeaningConfirmed { speaker, .. })
            | bityzba::data!(ProtocolEvent::MessagePosted { speaker, .. })
            | bityzba::data!(ProtocolEvent::TurnForfeited { speaker, .. }) => speaker,
            bityzba::data!(ProtocolEvent::ListenerFlowStarted { listener, .. })
            | bityzba::data!(ProtocolEvent::BlindInterpretationRecorded { listener, .. })
            | bityzba::data!(ProtocolEvent::TersmuRevealed { listener, .. })
            | bityzba::data!(ProtocolEvent::Acknowledged { listener, .. })
            | bityzba::data!(ProtocolEvent::ListenerFlowAbandoned { listener, .. }) => listener,
            bityzba::data!(ProtocolEvent::ReferenceToolCompleted { participant, .. })
            | bityzba::data!(ProtocolEvent::ReferenceLookupRepeated { participant, .. })
            | bityzba::data!(ProtocolEvent::ReferenceCallBudgetExhausted { participant, .. })
            | bityzba::data!(ProtocolEvent::ReferenceResearchNudge { participant, .. })
            | bityzba::data!(ProtocolEvent::ProseRejected { participant, .. })
            | bityzba::data!(ProtocolEvent::ProtocolError { participant, .. })
            | bityzba::data!(ProtocolEvent::AnswerSubmitted { participant, .. })
            | bityzba::data!(ProtocolEvent::UsageRecorded { participant, .. })
            | bityzba::data!(ProtocolEvent::ThinkingRecorded { participant, .. }) => participant,
            bityzba::data!(ProtocolEvent::RunStarted { .. })
            | bityzba::data!(ProtocolEvent::EmbeddingSearchDegraded { .. })
            | bityzba::data!(ProtocolEvent::DialogClosedForAnswers { .. })
            | bityzba::data!(ProtocolEvent::CheckerOutcome { .. })
            | bityzba::data!(ProtocolEvent::RunAborted { .. })
            | bityzba::data!(ProtocolEvent::RunFinished { .. })
            | bityzba::data!(ProtocolEvent::RunFailed { .. }) => "harness",
        }
    }
}

/// Stateless generator for protocol schemas and phase tool sets.
#[invariant(true)]
#[derive(Debug, Clone, Copy, Default)]
pub struct ProtocolTools;

impl ProtocolTools {
    /// Protocol definitions, including `submit_answer` when a scenario schema is supplied.
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|definitions| definitions.len() == 5 + usize::from(answer_schema.is_some())) || ret.is_err())]
    pub fn definitions(
        answer_schema: Option<&Value>,
    ) -> Result<Vec<(ProtocolTool, ToolDefinition)>, ToolDefinitionError> {
        let mut definitions = vec![
            (
                ProtocolTool::RegisterIntent,
                definition::<RegisterIntentArguments>(
                    ProtocolTool::RegisterIntent,
                    "Register or revise the speaker's private English intent before composing Lojban.",
                )?,
            ),
            (
                ProtocolTool::SubmitLojban,
                definition::<SubmitLojbanArguments>(
                    ProtocolTool::SubmitLojban,
                    "Submit candidate Lojban to the production parser and tersmu gate.",
                )?,
            ),
            (
                ProtocolTool::ConfirmMeaning,
                definition::<ConfirmMeaningArguments>(
                    ProtocolTool::ConfirmMeaning,
                    "Compare an accepted tersmu rendering with the registered intent and either revise or post.",
                )?,
            ),
            (
                ProtocolTool::InterpretBlind,
                definition::<InterpretBlindArguments>(
                    ProtocolTool::InterpretBlind,
                    "Commit an English interpretation after seeing only the Lojban message.",
                )?,
            ),
            (
                ProtocolTool::Acknowledge,
                definition::<AcknowledgeArguments>(
                    ProtocolTool::Acknowledge,
                    "Record final understanding after the tersmu rendering is revealed.",
                )?,
            ),
        ];
        if let Some(answer_schema) = answer_schema {
            definitions.push((
                ProtocolTool::SubmitAnswer,
                ToolDefinition::new(
                    ProtocolTool::SubmitAnswer.name().to_owned(),
                    "Submit this participant's mechanically checked scenario answer.".to_owned(),
                    answer_schema.clone(),
                )?,
            ));
        }
        Ok(definitions)
    }

    /// Exactly the legal phase tools, plus reference tools while their budget remains.
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|definitions| !reference_tools_available || definitions.len() >= 5) || ret.is_err())]
    pub fn definitions_for_phase(
        phase: ProtocolPhase,
        answer_schema: Option<&Value>,
        reference_tools_available: bool,
    ) -> Result<Vec<ToolDefinition>, ToolDefinitionError> {
        let answer_available = answer_schema.is_some();
        let mut definitions = Self::definitions(answer_schema)?
            .into_iter()
            .filter_map(|(tool, definition)| {
                phase.allows(tool, answer_available).then_some(definition)
            })
            .collect::<Vec<_>>();
        if reference_tools_available {
            definitions.extend(ReferenceTools::definitions()?);
        }
        Ok(definitions)
    }
}

#[invariant(true, "wire arguments are validated immediately after deserialization")]
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct RegisterIntentArguments {
    /// The meaning the speaker intends to communicate, in English.
    meaning_en: String,
}

#[invariant(true, "wire arguments are validated immediately after deserialization")]
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct SubmitLojbanArguments {
    /// Candidate Lojban text to parse and semantically render.
    text: String,
}

#[invariant(true, "wire arguments are validated immediately after deserialization")]
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ConfirmMeaningArguments {
    /// True only when the tersmu rendering matches the registered intent.
    matches: bool,
    /// Mandatory English paraphrase of the tersmu rendering.
    paraphrase_en: String,
    /// Optional discrepancies between intent and rendering.
    discrepancies: Option<String>,
}

#[invariant(true, "wire arguments are validated immediately after deserialization")]
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct InterpretBlindArguments {
    /// English interpretation formed from the bare Lojban alone.
    interpretation_en: String,
}

#[invariant(true, "wire arguments are validated immediately after deserialization")]
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct AcknowledgeArguments {
    /// Final English understanding after inspecting the tersmu rendering.
    final_understanding_en: String,
    /// Optional differences between the blind and revealed readings.
    discrepancies: Option<String>,
}

#[contract_trait]
trait ProtocolArguments: for<'de> Deserialize<'de> {
    #[requires(true)]
    #[ensures(ret.as_ref().err().is_none_or(|message| !message.trim().is_empty()))]
    fn validate(&self) -> Result<(), String>;
}

#[contract_trait]
impl ProtocolArguments for RegisterIntentArguments {
    fn validate(&self) -> Result<(), String> {
        require_nonempty("meaning_en", &self.meaning_en)
    }
}

#[contract_trait]
impl ProtocolArguments for SubmitLojbanArguments {
    fn validate(&self) -> Result<(), String> {
        require_nonempty("text", &self.text)
    }
}

#[contract_trait]
impl ProtocolArguments for ConfirmMeaningArguments {
    fn validate(&self) -> Result<(), String> {
        require_nonempty("paraphrase_en", &self.paraphrase_en)?;
        require_present_nonempty("discrepancies", self.discrepancies.as_deref())
    }
}

#[contract_trait]
impl ProtocolArguments for InterpretBlindArguments {
    fn validate(&self) -> Result<(), String> {
        require_nonempty("interpretation_en", &self.interpretation_en)
    }
}

#[contract_trait]
impl ProtocolArguments for AcknowledgeArguments {
    fn validate(&self) -> Result<(), String> {
        require_nonempty("final_understanding_en", &self.final_understanding_en)?;
        require_present_nonempty("discrepancies", self.discrepancies.as_deref())
    }
}

#[requires(!field.trim().is_empty())]
#[ensures(ret.as_ref().err().is_none_or(|message| !message.trim().is_empty()))]
fn require_nonempty(field: &str, value: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        Err(format!("`{field}` cannot be empty"))
    } else {
        Ok(())
    }
}

#[requires(!field.trim().is_empty())]
#[ensures(ret.as_ref().err().is_none_or(|message| !message.trim().is_empty()))]
fn require_present_nonempty(field: &str, value: Option<&str>) -> Result<(), String> {
    if value.is_some_and(|value| value.trim().is_empty()) {
        Err(format!("`{field}` cannot be empty when present"))
    } else {
        Ok(())
    }
}

#[requires(!description.trim().is_empty())]
#[ensures(ret.as_ref().is_ok_and(|definition| definition.name() == tool.name()) || ret.is_err())]
fn definition<T: JsonSchema>(
    tool: ProtocolTool,
    description: &str,
) -> Result<ToolDefinition, ToolDefinitionError> {
    let parameters = serde_json::to_value(schema_for!(T))
        .expect("generated protocol tool schema serializes to JSON");
    ToolDefinition::new(tool.name().to_owned(), description.to_owned(), parameters)
}

/// Model-call boundary used by both OpenRouter conversations and offline scripts.
#[contract_trait]
pub trait ProtocolModel {
    /// Stable participant name.
    #[requires(true)]
    #[ensures(!ret.trim().is_empty())]
    fn participant_name(&self) -> &str;

    /// Provider-level enforcement mode configured for this participant.
    #[requires(true)]
    #[ensures(true)]
    fn tool_choice(&self) -> ProviderToolChoice;

    /// Corrective reprompts allowed after consecutive automatic prose responses.
    #[requires(true)]
    #[ensures(true)]
    fn max_tool_reprompts(&self) -> usize;

    /// Start a new provider tool loop and discard any stale request-scoped replay.
    #[requires(true)]
    #[ensures(true)]
    fn begin_tool_loop(&mut self) {}

    /// Add private harness-visible context.
    #[requires(!content.trim().is_empty())]
    #[ensures(true)]
    fn push_user(&mut self, content: String);

    /// Apply the bounded automatic-mode correction in the model-compatible role.
    #[requires(!tools.is_empty())]
    #[ensures(true)]
    fn push_tool_correction(&mut self, tools: &[ToolDefinition]);

    /// Request one turn using exactly the dynamically supplied definitions.
    #[requires(!tools.is_empty())]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    fn request(
        &mut self,
        tools: &[ToolDefinition],
        tool_choice: ProviderToolChoice,
        accounting: &mut RunAccounting,
    ) -> Result<crate::ModelTurn, ProtocolModelError>;

    /// Drain one observability record for every provider call made by the last request.
    #[requires(true)]
    #[ensures(true)]
    fn take_observations(&mut self) -> Vec<ProviderCallObservation> {
        Vec::new()
    }

    /// Thread one result back to the originating tool call.
    #[requires(!call.id.trim().is_empty())]
    #[requires(!call.function.name.trim().is_empty())]
    #[ensures(true)]
    fn push_tool_result(&mut self, call: &ToolCall, content: String);
}

/// Live implementation of [`ProtocolModel`] over the existing runtime.
#[invariant(true, "constructed from validated participant configuration")]
#[derive(Debug)]
pub struct OpenRouterParticipant<'client> {
    conversation: ParticipantConversation,
    tool_choice: ProviderToolChoice,
    supports_prefill: bool,
    client: &'client OpenRouterClient,
}

impl<'client> OpenRouterParticipant<'client> {
    /// Build a live participant with its persona and standing rules.
    ///
    /// [`ProtocolRunner::new_with_scenario`] supplies the sole scenario brief
    /// before the first request.
    #[requires(true)]
    #[ensures(ret.conversation.participant_name() == participant.name)]
    pub fn new(participant: &ParticipantConfig, client: &'client OpenRouterClient) -> Self {
        let system_prompt = format!("{}\n\n{STANDING_PROTOCOL_RULES}", participant.system_prompt);
        let policy = ParticipantModelPolicy::resolve(
            &participant.model,
            participant.tool_choice,
            participant.reasoning,
        );
        Self {
            conversation: ParticipantConversation::from_system_prompt(
                participant.name.clone(),
                participant.model.clone(),
                participant.provider.clone(),
                participant.prompt_caching,
                policy.reasoning,
                participant.temperature,
                system_prompt,
            ),
            tool_choice: policy.tool_choice,
            supports_prefill: policy.supports_prefill,
            client,
        }
    }

    /// Access the underlying private conversation for usage and transcript work.
    #[requires(true)]
    #[ensures(ret.participant_name() == self.conversation.participant_name())]
    pub fn conversation(&self) -> &ParticipantConversation {
        &self.conversation
    }
}

#[contract_trait]
impl ProtocolModel for OpenRouterParticipant<'_> {
    fn participant_name(&self) -> &str {
        self.conversation.participant_name()
    }

    fn tool_choice(&self) -> ProviderToolChoice {
        self.tool_choice
    }

    fn max_tool_reprompts(&self) -> usize {
        self.client.max_required_tool_reprompts()
    }

    fn begin_tool_loop(&mut self) {
        self.conversation.begin_tool_loop();
    }

    fn push_user(&mut self, content: String) {
        self.conversation.push_user(content);
    }

    fn push_tool_correction(&mut self, tools: &[ToolDefinition]) {
        if self.supports_prefill {
            let names = tools
                .iter()
                .map(ToolDefinition::name)
                .collect::<Vec<_>>()
                .join(", ");
            self.conversation.push_assistant_prefill(format!(
                "Actually, I must use one of the following tools: {names}."
            ));
        } else {
            self.conversation
                .push_user(REQUIRED_TOOL_CORRECTION.to_owned());
        }
    }

    fn request(
        &mut self,
        tools: &[ToolDefinition],
        tool_choice: ProviderToolChoice,
        accounting: &mut RunAccounting,
    ) -> Result<crate::ModelTurn, ProtocolModelError> {
        self.conversation
            .request(self.client, tools, tool_choice, accounting)
            .map_err(|error| ProtocolModelError::new(error.to_string()))
    }

    fn take_observations(&mut self) -> Vec<ProviderCallObservation> {
        self.conversation.take_pending_observations()
    }

    fn push_tool_result(&mut self, call: &ToolCall, content: String) {
        self.conversation.push_tool_result(call, content);
    }
}

/// Model boundary failure independent of the live HTTP implementation.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
#[error("{message}")]
#[invariant(true, "constructed only through ProtocolModelError::new")]
pub struct ProtocolModelError {
    pub message: String,
}

impl ProtocolModelError {
    /// Construct a nonempty model error.
    #[requires(!message.trim().is_empty())]
    #[ensures(!ret.message.is_empty())]
    pub fn new(message: String) -> Self {
        Self { message }
    }
}

/// Real in-process dispatcher for the five production reference tools.
#[invariant(true)]
#[derive(Debug, Clone, Copy, Default)]
pub struct ReferenceToolDispatcher;

#[contract_trait]
impl ToolDispatcher for ReferenceToolDispatcher {
    fn dispatch(&mut self, call: &ToolCall) -> Result<String, ToolDispatchError> {
        let output = ReferenceTools::dispatch(call).map_err(|error| {
            ToolDispatchError::new(call.function.name.clone(), error.to_string())
        })?;
        let mut result = String::from_utf8(output.stdout).map_err(|error| {
            ToolDispatchError::new(
                call.function.name.clone(),
                format!("production stdout was not UTF-8: {error}"),
            )
        })?;
        result.push_str(&output.stderr);
        if result.is_empty() {
            result = format!("status: {:?}", output.status);
        }
        Ok(result)
    }
}

/// Successful completion or graceful runtime budget abort.
#[invariant(::Completed { turns } => *turns > 0)]
#[invariant(::ScenarioCompleted { turns } => *turns > 0)]
#[invariant(::BudgetAborted { .. } => true)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "kebab-case")]
pub enum ProtocolRunOutcome {
    Completed { turns: usize },
    ScenarioCompleted { turns: usize },
    BudgetAborted { turns: usize, record: AbortRecord },
}

impl ProtocolRunOutcome {
    /// Number of turns begun before this terminal outcome.
    #[requires(true)]
    #[ensures(true)]
    pub fn turns(&self) -> usize {
        match self.as_data() {
            bityzba::data!(ProtocolRunOutcome::Completed { turns })
            | bityzba::data!(ProtocolRunOutcome::ScenarioCompleted { turns })
            | bityzba::data!(ProtocolRunOutcome::BudgetAborted { turns, .. }) => *turns,
        }
    }
}

/// Protocol orchestration failures; cap hits are events, not errors.
#[invariant(::InvalidConfiguration { message } => !message.trim().is_empty())]
#[invariant(::ToolDefinitions { message } => !message.trim().is_empty())]
#[invariant(::Model { participant, message } => !participant.trim().is_empty() && !message.trim().is_empty())]
#[invariant(::Gate { participant, message } => !participant.trim().is_empty() && !message.trim().is_empty())]
#[invariant(::InvalidTersmuEncoding { message } => !message.trim().is_empty())]
#[invariant(::Transcript { message } => !message.trim().is_empty())]
#[invariant(::AlreadyRun => true)]
#[derive(Debug, PartialEq, Eq)]
pub enum ProtocolRunError {
    InvalidConfiguration {
        message: String,
    },
    ToolDefinitions {
        message: String,
    },
    Model {
        participant: String,
        message: String,
    },
    Gate {
        participant: String,
        message: String,
    },
    InvalidTersmuEncoding {
        message: String,
    },
    Transcript {
        message: String,
    },
    AlreadyRun,
}

impl fmt::Display for ProtocolRunError {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.as_data() {
            bityzba::data!(ProtocolRunError::InvalidConfiguration { message }) => {
                write!(formatter, "invalid protocol configuration: {message}")
            }
            bityzba::data!(ProtocolRunError::ToolDefinitions { message }) => write!(
                formatter,
                "unable to construct protocol tool definitions: {message}"
            ),
            bityzba::data!(ProtocolRunError::Model {
                participant,
                message,
            }) => write!(
                formatter,
                "model request for `{participant}` failed: {message}"
            ),
            bityzba::data!(ProtocolRunError::Gate {
                participant,
                message,
            }) => write!(
                formatter,
                "jbotci gate for `{participant}` failed: {message}"
            ),
            bityzba::data!(ProtocolRunError::InvalidTersmuEncoding { message }) => {
                write!(formatter, "tersmu rendering was not UTF-8: {message}")
            }
            bityzba::data!(ProtocolRunError::Transcript { message }) => {
                write!(formatter, "transcript write failed: {message}")
            }
            bityzba::data!(ProtocolRunError::AlreadyRun) => {
                formatter.write_str("a protocol runner can only be run once")
            }
        }
    }
}

impl std::error::Error for ProtocolRunError {}

impl ProtocolRunError {
    #[requires(turn_number > 0)]
    #[ensures(ret.turn_number == turn_number)]
    fn runtime_failure(&self, turn_number: usize) -> RuntimeFailureRecord {
        let (participant, call_site) = match self.as_data() {
            bityzba::data!(ProtocolRunError::InvalidConfiguration { .. }) => {
                (None, RuntimeFailureSite::ProtocolConfiguration)
            }
            bityzba::data!(ProtocolRunError::ToolDefinitions { .. }) => {
                (None, RuntimeFailureSite::ToolDefinitions)
            }
            bityzba::data!(ProtocolRunError::Model { participant, .. }) => {
                (Some(participant.clone()), RuntimeFailureSite::ModelRequest)
            }
            bityzba::data!(ProtocolRunError::Gate { participant, .. }) => {
                (Some(participant.clone()), RuntimeFailureSite::JbotciGate)
            }
            bityzba::data!(ProtocolRunError::InvalidTersmuEncoding { .. }) => {
                (None, RuntimeFailureSite::TersmuRendering)
            }
            bityzba::data!(ProtocolRunError::Transcript { .. }) => {
                (None, RuntimeFailureSite::TranscriptWrite)
            }
            bityzba::data!(ProtocolRunError::AlreadyRun) => (None, RuntimeFailureSite::RunnerReuse),
        };
        new!(RuntimeFailureRecord {
            turn_number,
            participant,
            call_site,
            message: self.to_string(),
        })
    }
}

#[invariant(
    true,
    "events are appended in order and transcript failures are retained"
)]
#[derive(Debug, Default)]
struct ProtocolEventLog {
    events: Vec<ProtocolEvent>,
    writer: Option<TranscriptWriter>,
    write_error: Option<TranscriptError>,
    current_turn: usize,
}

impl ProtocolEventLog {
    #[requires(self.events.is_empty())]
    #[ensures(ret.is_ok() -> self.events.len() == 1)]
    fn attach(&mut self, path: &Path, header: RunHeader) -> Result<(), TranscriptError> {
        self.writer = Some(TranscriptWriter::create(path)?);
        self.push(new!(ProtocolEvent::RunStarted { header }));
        self.take_write_error().map_or(Ok(()), Err)
    }

    #[requires(true)]
    #[ensures(self.events.len() == old(self.events.len()) + 1)]
    fn push(&mut self, event: ProtocolEvent) {
        if let bityzba::data!(ProtocolEvent::TurnStarted { turn_number, .. }) = event.as_data() {
            self.current_turn = *turn_number;
        }
        let turn_number = if matches!(
            event.as_data(),
            bityzba::data!(ProtocolEvent::RunStarted { .. })
        ) {
            0
        } else {
            self.current_turn.max(1)
        };
        if self.write_error.is_none()
            && let Some(writer) = &mut self.writer
            && let Err(error) = writer.append_at(
                turn_number,
                event.transcript_participant().to_owned(),
                event.clone(),
            )
        {
            self.write_error = Some(error);
        }
        self.events.push(event);
    }

    #[requires(true)]
    #[ensures(self.write_error.is_none())]
    fn take_write_error(&mut self) -> Option<TranscriptError> {
        self.write_error.take()
    }
}

/// Exact wire identity of one reference lookup within a protocol phase.
#[invariant(!tool_name.trim().is_empty())]
#[invariant(!arguments.trim().is_empty())]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ReferenceLookupKey {
    tool_name: String,
    arguments: String,
}

/// First executed result retained for exact duplicate lookups in one phase.
#[invariant(!result.is_empty())]
#[invariant(*occurrences > 0)]
#[derive(Debug, Clone)]
struct ReferenceMemoEntry {
    result: String,
    succeeded: bool,
    occurrences: usize,
}

/// Per-phase protocol state for bounded, loop-aware reference research.
#[invariant(
    true,
    "all mutations are centralized in enter_phase, record_protocol_call, and dispatch_reference"
)]
#[derive(Debug)]
struct ReferencePhaseState {
    phase: ProtocolPhase,
    calls: usize,
    consecutive_calls: usize,
    nudge_sent: bool,
    memo: BTreeMap<ReferenceLookupKey, ReferenceMemoEntry>,
}

impl ReferencePhaseState {
    /// Start tracking the initial protocol phase.
    #[requires(true)]
    #[ensures(ret.phase == phase)]
    #[ensures(ret.calls == 0 && ret.consecutive_calls == 0 && ret.memo.is_empty())]
    fn new(phase: ProtocolPhase) -> Self {
        Self {
            phase,
            calls: 0,
            consecutive_calls: 0,
            nudge_sent: false,
            memo: BTreeMap::new(),
        }
    }

    /// Reset every reference concern when the typed protocol state changes.
    #[requires(true)]
    #[ensures(self.phase == phase)]
    fn enter_phase(&mut self, phase: ProtocolPhase) {
        if self.phase != phase {
            self.phase = phase;
            self.calls = 0;
            self.consecutive_calls = 0;
            self.nudge_sent = false;
            self.memo.clear();
        }
    }

    /// Whether the current phase may still offer the reference tools.
    #[requires(maximum > 0)]
    #[ensures(ret == (self.calls < maximum))]
    fn reference_tools_available(&self, maximum: usize) -> bool {
        self.calls < maximum
    }

    /// Break a run of idle research after any recognized protocol-tool call.
    #[requires(true)]
    #[ensures(self.phase == phase && self.consecutive_calls == 0)]
    fn record_protocol_call(&mut self, phase: ProtocolPhase) {
        self.enter_phase(phase);
        self.consecutive_calls = 0;
    }
}

/// One reference result plus an optional model-facing phase correction.
#[invariant(!content.is_empty())]
#[invariant(corrective_message.as_ref().is_none_or(|message| !message.trim().is_empty()))]
#[derive(Debug)]
struct ReferenceDispatch {
    content: String,
    corrective_message: Option<String>,
}

/// Turn position projected into model-facing protocol instructions.
#[invariant(*turn_number > 0)]
#[invariant(*turn_number <= *maximum_turns)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TurnDeadline {
    turn_number: usize,
    maximum_turns: usize,
}

/// Participant-specific scenario-answer state projected from the real gate.
#[invariant(::Unavailable => true)]
#[invariant(::Available => true)]
#[invariant(::Recorded { submitted, required } => *submitted > 0 && submitted <= required)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AnswerAffordance {
    Unavailable,
    Available,
    Recorded { submitted: usize, required: usize },
}

/// Append grounded turn and answer context to an existing phase instruction.
#[requires(!base.trim().is_empty())]
#[ensures(ret.starts_with(base))]
#[ensures(!ret.trim().is_empty())]
fn phase_instruction(
    base: &str,
    deadline: Option<TurnDeadline>,
    answer_affordance: AnswerAffordance,
) -> String {
    let mut instruction = base.to_owned();
    if let Some(deadline) = deadline {
        write!(
            instruction,
            "\n\nThis is turn {} of at most {}.",
            deadline.turn_number, deadline.maximum_turns
        )
        .expect("writing to a String cannot fail");
    }
    match answer_affordance.as_data() {
        bityzba::data!(AnswerAffordance::Unavailable) => {}
        bityzba::data!(AnswerAffordance::Available) => {
            instruction.push_str("\n\n");
            instruction.push_str(ANSWER_AVAILABLE_INSTRUCTION);
        }
        bityzba::data!(AnswerAffordance::Recorded {
            submitted,
            required,
        }) => {
            write!(
                instruction,
                "\n\nYour answer is recorded; {submitted} of {required} participants have submitted."
            )
            .expect("writing to a String cannot fail");
        }
    }
    instruction
}

/// Sequential round-robin protocol runner.
#[invariant(true, "validated on construction and mutated only through run")]
#[derive(Debug)]
pub struct ProtocolRunner<M, D> {
    participants: Vec<M>,
    reference_dispatcher: D,
    caps: CapsConfig,
    listener_mode: ListenerMode,
    tersmu_format: TersmuFormat,
    accounting: RunAccounting,
    visible_chat: Vec<VisibleMessage>,
    events: ProtocolEventLog,
    scenario: Option<ScenarioInstance>,
    answers: BTreeMap<String, ScenarioAnswer>,
    task_outcome: Option<TaskOutcome>,
    degraded_search_message: Option<String>,
    turns_started: usize,
    has_run: bool,
}

impl<M: ProtocolModel, D: ToolDispatcher> ProtocolRunner<M, D> {
    /// Construct a bounded runner over at least two uniquely named participants.
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|runner| runner.participants.len() >= 2) || ret.is_err())]
    pub fn new(
        participants: Vec<M>,
        caps: CapsConfig,
        listener_mode: ListenerMode,
        tersmu_format: TersmuFormat,
        reference_dispatcher: D,
    ) -> Result<Self, ProtocolRunError> {
        Self::new_inner(
            participants,
            caps,
            listener_mode,
            tersmu_format,
            reference_dispatcher,
            None,
        )
    }

    /// Construct a bounded runner with scenario-specific prompts and answers.
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|runner| runner.scenario.is_some()) || ret.is_err())]
    pub fn new_with_scenario(
        participants: Vec<M>,
        caps: CapsConfig,
        listener_mode: ListenerMode,
        tersmu_format: TersmuFormat,
        reference_dispatcher: D,
        scenario: ScenarioInstance,
    ) -> Result<Self, ProtocolRunError> {
        Self::new_inner(
            participants,
            caps,
            listener_mode,
            tersmu_format,
            reference_dispatcher,
            Some(scenario),
        )
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|runner| runner.participants.len() >= 2) || ret.is_err())]
    fn new_inner(
        participants: Vec<M>,
        caps: CapsConfig,
        listener_mode: ListenerMode,
        tersmu_format: TersmuFormat,
        reference_dispatcher: D,
        scenario: Option<ScenarioInstance>,
    ) -> Result<Self, ProtocolRunError> {
        if participants.len() < 2 {
            return Err(new!(ProtocolRunError::InvalidConfiguration {
                message: "a discussion requires at least two participants".to_owned(),
            }));
        }
        let names = participants
            .iter()
            .map(ProtocolModel::participant_name)
            .collect::<BTreeSet<_>>();
        if names.len() != participants.len() {
            return Err(new!(ProtocolRunError::InvalidConfiguration {
                message: "participant names must be unique".to_owned(),
            }));
        }
        if names.iter().any(|name| name.trim().is_empty()) {
            return Err(new!(ProtocolRunError::InvalidConfiguration {
                message: "participant names cannot be empty".to_owned(),
            }));
        }
        if let Some(scenario) = &scenario {
            let scenario_names = scenario
                .participants()
                .iter()
                .map(|participant| participant.name.as_str())
                .collect::<BTreeSet<_>>();
            if names != scenario_names {
                return Err(new!(ProtocolRunError::InvalidConfiguration {
                    message:
                        "protocol participants must exactly match the scenario participant names"
                            .to_owned(),
                }));
            }
        }
        let accounting = RunAccounting::new(caps.max_cost_usd).map_err(|error| {
            new!(ProtocolRunError::InvalidConfiguration {
                message: error.to_string(),
            })
        })?;
        Ok(Self {
            participants,
            reference_dispatcher,
            caps,
            listener_mode,
            tersmu_format,
            accounting,
            visible_chat: Vec::new(),
            events: ProtocolEventLog::default(),
            scenario,
            answers: BTreeMap::new(),
            task_outcome: None,
            degraded_search_message: None,
            turns_started: 0,
            has_run: false,
        })
    }

    /// Participant models, primarily for offline-script assertions.
    #[requires(true)]
    #[ensures(ret.len() == self.participants.len())]
    pub fn participants(&self) -> &[M] {
        &self.participants
    }

    /// Confirmed visible chat, containing no private English fields.
    #[requires(true)]
    #[ensures(ret.len() == self.visible_chat.len())]
    pub fn visible_chat(&self) -> &[VisibleMessage] {
        &self.visible_chat
    }

    /// Complete typed private event stream.
    #[requires(true)]
    #[ensures(ret.len() == self.events.events.len())]
    pub fn events(&self) -> &[ProtocolEvent] {
        &self.events.events
    }

    /// Attach a flushed append-only transcript before starting the run.
    #[requires(true)]
    #[ensures(ret.is_ok() -> self.events.events.len() == 1)]
    pub fn attach_transcript(
        &mut self,
        path: &Path,
        header: RunHeader,
    ) -> Result<(), ProtocolRunError> {
        if self.has_run || !self.events.events.is_empty() {
            return Err(new!(ProtocolRunError::InvalidConfiguration {
                message: "a transcript must be attached before the run starts".to_owned(),
            }));
        }
        let configured_names = header
            .config
            .participants
            .iter()
            .map(|participant| participant.name.as_str())
            .collect::<BTreeSet<_>>();
        let runner_names = self
            .participants
            .iter()
            .map(ProtocolModel::participant_name)
            .collect::<BTreeSet<_>>();
        let scenario_matches = self.scenario.as_ref().is_some_and(|scenario| {
            scenario
                .to_toml()
                .is_ok_and(|snapshot| snapshot == header.scenario_instance_toml)
        });
        if configured_names != runner_names
            || header.config.caps != self.caps
            || header.config.listener_mode != self.listener_mode
            || header.config.tersmu_format != self.tersmu_format
            || !scenario_matches
        {
            return Err(new!(ProtocolRunError::InvalidConfiguration {
                message: "transcript header does not describe this protocol runner".to_owned(),
            }));
        }
        self.events.attach(path, header).map_err(|error| {
            new!(ProtocolRunError::Transcript {
                message: error.to_string(),
            })
        })
    }

    /// Queue the typed warning produced by an explicitly tolerated failed preflight.
    #[requires(!message.trim().is_empty())]
    #[ensures(ret.is_ok() -> self.degraded_search_message.is_some())]
    pub(crate) fn record_degraded_search(
        &mut self,
        message: String,
    ) -> Result<(), ProtocolRunError> {
        if self.has_run || self.degraded_search_message.is_some() {
            return Err(new!(ProtocolRunError::InvalidConfiguration {
                message: "degraded search can be recorded exactly once before the run".to_owned(),
            }));
        }
        self.degraded_search_message = Some(message);
        Ok(())
    }

    /// Accepted typed scenario answers, keyed by participant name.
    #[requires(true)]
    #[ensures(ret.len() == self.answers.len())]
    pub fn answers(&self) -> &BTreeMap<String, ScenarioAnswer> {
        &self.answers
    }

    /// Final scenario checker outcome, once all answers arrive or a limit ends the run.
    #[requires(true)]
    #[ensures(ret.is_some() == self.task_outcome.is_some())]
    pub fn task_outcome(&self) -> Option<&TaskOutcome> {
        self.task_outcome.as_ref()
    }

    /// Run the configured number of round-robin speaker turns.
    #[requires(true)]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    pub fn run(&mut self) -> Result<ProtocolRunOutcome, ProtocolRunError> {
        if self.has_run {
            return Err(new!(ProtocolRunError::AlreadyRun));
        }
        self.has_run = true;
        let result = self.run_inner();
        match &result {
            Ok(outcome) => self.events.push(new!(ProtocolEvent::RunFinished {
                outcome: outcome.clone(),
            })),
            Err(error) if self.turns_started > 0 => {
                self.events.push(new!(ProtocolEvent::RunFailed {
                    failure: error.runtime_failure(self.turns_started),
                }));
            }
            Err(_) => {}
        }
        if let Some(error) = self.events.take_write_error() {
            return Err(new!(ProtocolRunError::Transcript {
                message: error.to_string(),
            }));
        }
        result
    }

    #[requires(true)]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    fn run_inner(&mut self) -> Result<ProtocolRunOutcome, ProtocolRunError> {
        if let Some(scenario) = &self.scenario {
            let prompts = self
                .participants
                .iter()
                .enumerate()
                .map(|(index, participant)| {
                    let name = participant.participant_name();
                    let prompt = scenario.prompt_for(name).ok_or_else(|| {
                        new!(ProtocolRunError::InvalidConfiguration {
                            message: format!("scenario has no private brief for `{name}`"),
                        })
                    })?;
                    Ok((index, prompt))
                })
                .collect::<Result<Vec<_>, ProtocolRunError>>()?;
            for (index, prompt) in prompts {
                self.participants[index].push_user(prompt);
            }
        }
        let turn_limit = self.effective_turn_limit();
        for turn_number in 1..=turn_limit {
            self.turns_started = turn_number;
            let speaker_index = (turn_number - 1) % self.participants.len();
            let speaker = self.participants[speaker_index]
                .participant_name()
                .to_owned();
            self.events.push(new!(ProtocolEvent::TurnStarted {
                turn_number,
                speaker: speaker.clone(),
            }));
            if let Some(message) = self.degraded_search_message.take() {
                self.events
                    .push(new!(ProtocolEvent::EmbeddingSearchDegraded { message }));
            }
            let speaker_outcome = self.run_speaker(turn_number, turn_limit, speaker_index)?;
            match speaker_outcome.as_data() {
                bityzba::data!(SpeakerOutcome::Posted { message }) => {
                    self.visible_chat.push(message.clone());
                    for listener_index in 0..self.participants.len() {
                        if listener_index != speaker_index {
                            let listener_outcome = self.run_listener(
                                turn_number,
                                turn_limit,
                                &speaker,
                                listener_index,
                                message,
                            )?;
                            match listener_outcome {
                                ListenerRunOutcome::Acknowledged
                                | ListenerRunOutcome::Abandoned => {}
                                ListenerRunOutcome::ScenarioCompleted => {
                                    return Ok(new!(ProtocolRunOutcome::ScenarioCompleted {
                                        turns: turn_number,
                                    }));
                                }
                                ListenerRunOutcome::BudgetAborted { record } => {
                                    return Ok(new!(ProtocolRunOutcome::BudgetAborted {
                                        turns: turn_number,
                                        record,
                                    }));
                                }
                            }
                        }
                    }
                }
                bityzba::data!(SpeakerOutcome::Forfeited) => {}
                bityzba::data!(SpeakerOutcome::ScenarioCompleted) => {
                    return Ok(new!(ProtocolRunOutcome::ScenarioCompleted {
                        turns: turn_number,
                    }));
                }
                bityzba::data!(SpeakerOutcome::BudgetAborted { record }) => {
                    return Ok(new!(ProtocolRunOutcome::BudgetAborted {
                        turns: turn_number,
                        record: record.clone(),
                    }));
                }
            }
            if self.closed_answer_phase_due(turn_number) {
                return match self.run_closed_answer_phase(turn_number)? {
                    ClosedAnswerPhaseOutcome::AllSubmitted => {
                        Ok(new!(ProtocolRunOutcome::ScenarioCompleted {
                            turns: turn_number,
                        }))
                    }
                    ClosedAnswerPhaseOutcome::Incomplete => {
                        Ok(new!(ProtocolRunOutcome::Completed { turns: turn_number }))
                    }
                    ClosedAnswerPhaseOutcome::BudgetAborted { record } => {
                        Ok(new!(ProtocolRunOutcome::BudgetAborted {
                            turns: turn_number,
                            record,
                        }))
                    }
                };
            }
        }
        if self
            .scenario
            .as_ref()
            .is_some_and(ScenarioInstance::is_scored)
        {
            self.finish_scenario(turn_limit.max(1));
        }
        Ok(new!(ProtocolRunOutcome::Completed {
            turns: self.turns_started,
        }))
    }

    /// Exact speaker-turn cap used by both loop termination and phase instructions.
    #[requires(true)]
    #[ensures(ret > 0)]
    #[ensures(ret <= self.caps.max_turns)]
    fn effective_turn_limit(&self) -> usize {
        self.scenario
            .as_ref()
            .map_or(self.caps.max_turns, |scenario| {
                let scenario_limit = scenario.maximum_rounds().map_or_else(
                    || scenario.maximum_turns(),
                    |maximum_rounds| {
                        scenario
                            .maximum_turns()
                            .min(maximum_rounds * self.participants.len())
                    },
                );
                self.caps.max_turns.min(scenario_limit)
            })
    }

    /// Whether the just-finished turn completes the first answer-eligible closed-dialog round.
    #[requires(turn_number > 0)]
    #[ensures(ret == self.scenario.as_ref().is_some_and(|scenario| scenario.answers_close_dialog() && turn_number % self.participants.len() == 0 && scenario.minimum_rounds().is_some_and(|minimum_rounds| turn_number / self.participants.len() >= minimum_rounds)))]
    fn closed_answer_phase_due(&self, turn_number: usize) -> bool {
        self.scenario.as_ref().is_some_and(|scenario| {
            scenario.answers_close_dialog()
                && turn_number % self.participants.len() == 0
                && scenario.minimum_rounds().is_some_and(|minimum_rounds| {
                    turn_number / self.participants.len() >= minimum_rounds
                })
        })
    }

    /// Freeze the visible transcript and collect every required answer independently.
    #[requires(turn_number > 0)]
    #[requires(self.closed_answer_phase_due(turn_number))]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    fn run_closed_answer_phase(
        &mut self,
        turn_number: usize,
    ) -> Result<ClosedAnswerPhaseOutcome, ProtocolRunError> {
        let round_number = turn_number / self.participants.len();
        self.events
            .push(new!(ProtocolEvent::DialogClosedForAnswers {
                turn_number,
                round_number,
            }));

        let required_indices = self
            .participants
            .iter()
            .enumerate()
            .filter_map(|(index, participant)| {
                let name = participant.participant_name();
                self.scenario
                    .as_ref()
                    .expect("closed answer phase requires a scenario")
                    .answer_required(name)
                    .then_some(index)
            })
            .collect::<Vec<_>>();

        // Establish the same frozen-dialog boundary for every answerer before requesting any
        // answer. Later submissions are recorded only in the submitting model's private history.
        for &participant_index in &required_indices {
            self.participants[participant_index].begin_tool_loop();
            self.participants[participant_index]
                .push_user(CLOSED_DIALOG_ANSWER_INSTRUCTION.to_owned());
        }

        for participant_index in required_indices {
            match self.run_closed_answer_submission(turn_number, participant_index)? {
                ClosedAnswerSubmissionOutcome::Submitted
                | ClosedAnswerSubmissionOutcome::Abandoned => {}
                ClosedAnswerSubmissionOutcome::BudgetAborted { record } => {
                    if self.task_outcome.is_none() {
                        self.finish_scenario(turn_number);
                    }
                    return Ok(ClosedAnswerPhaseOutcome::BudgetAborted { record });
                }
            }
        }

        if self.task_outcome.is_none() {
            self.finish_scenario(turn_number);
        }
        let all_submitted = self
            .scenario
            .as_ref()
            .expect("closed answer phase requires a scenario")
            .all_required_submitted(&self.answers);
        Ok(if all_submitted {
            ClosedAnswerPhaseOutcome::AllSubmitted
        } else {
            ClosedAnswerPhaseOutcome::Incomplete
        })
    }

    #[requires(turn_number > 0)]
    #[requires(self.scenario.as_ref().is_some_and(ScenarioInstance::is_scored))]
    #[ensures(self.task_outcome.is_some())]
    fn finish_scenario(&mut self, turn_number: usize) {
        if self.task_outcome.is_some() {
            return;
        }
        let scenario = self
            .scenario
            .as_ref()
            .expect("finish_scenario is called only for scenario runs");
        let outcome = scenario.check_answers(&self.answers);
        self.events.push(new!(ProtocolEvent::CheckerOutcome {
            turn_number,
            outcome: outcome.clone(),
        }));
        self.task_outcome = Some(outcome);
    }

    #[requires(turn_number > 0)]
    #[requires(!participant.trim().is_empty())]
    #[ensures(ret.as_ref().is_none_or(Value::is_object))]
    fn answer_schema_if_available(&self, turn_number: usize, participant: &str) -> Option<Value> {
        matches!(
            self.answer_affordance(turn_number, participant).as_data(),
            bityzba::data!(AnswerAffordance::Available)
        )
        .then(|| {
            self.scenario
                .as_ref()
                .expect("answer availability requires a scenario")
                .answer_schema()
                .expect("answer availability requires a scored scenario")
                .clone()
        })
    }

    /// Derive answer announcements and tool availability from one participant state.
    #[requires(turn_number > 0)]
    #[requires(!participant.trim().is_empty())]
    #[ensures(true)]
    fn answer_affordance(&self, turn_number: usize, participant: &str) -> AnswerAffordance {
        let Some(scenario) = &self.scenario else {
            return new!(AnswerAffordance::Unavailable);
        };
        if scenario.answers_close_dialog() {
            return new!(AnswerAffordance::Unavailable);
        }
        if !scenario.answer_required(participant) {
            return new!(AnswerAffordance::Unavailable);
        }
        if self.answers.contains_key(participant) {
            let required = scenario
                .participants()
                .iter()
                .filter(|participant| participant.answer_required)
                .count();
            return new!(AnswerAffordance::Recorded {
                submitted: self.answers.len(),
                required,
            });
        }
        let completed_rounds = (turn_number - 1) / self.participants.len();
        if scenario
            .minimum_rounds()
            .is_some_and(|minimum_rounds| completed_rounds >= minimum_rounds)
        {
            new!(AnswerAffordance::Available)
        } else {
            new!(AnswerAffordance::Unavailable)
        }
    }

    #[requires(turn_number > 0)]
    #[requires(participant_index < self.participants.len())]
    #[requires(!participant.trim().is_empty())]
    #[ensures(true)]
    fn record_provider_observations(
        &mut self,
        turn_number: usize,
        participant_index: usize,
        participant: &str,
    ) {
        for observation in self.participants[participant_index].take_observations() {
            if let Some(trace) = observation.thinking {
                self.events.push(new!(ProtocolEvent::ThinkingRecorded {
                    turn_number,
                    participant: participant.to_owned(),
                    trace,
                }));
            }
            self.events.push(new!(ProtocolEvent::UsageRecorded {
                turn_number,
                participant: participant.to_owned(),
                usage: observation.usage,
            }));
        }
    }

    /// Solicit one answer without reopening visible discussion or exposing another submission.
    #[requires(turn_number > 0)]
    #[requires(participant_index < self.participants.len())]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    fn run_closed_answer_submission(
        &mut self,
        turn_number: usize,
        participant_index: usize,
    ) -> Result<ClosedAnswerSubmissionOutcome, ProtocolRunError> {
        let participant = self.participants[participant_index]
            .participant_name()
            .to_owned();
        let phase = ProtocolPhase::Answer;
        let answer_schema = self
            .scenario
            .as_ref()
            .expect("closed answer phase requires a scenario")
            .answer_schema()
            .expect("closed answer phase requires a scored scenario")
            .clone();
        let tools = ProtocolTools::definitions_for_phase(phase, Some(&answer_schema), false)
            .map_err(|error| {
                new!(ProtocolRunError::ToolDefinitions {
                    message: error.to_string(),
                })
            })?;
        let tool_choice = self.participants[participant_index].tool_choice();
        let max_tool_reprompts = self.participants[participant_index].max_tool_reprompts();
        let mut tool_reprompts = 0usize;

        loop {
            let request = self.participants[participant_index].request(
                &tools,
                tool_choice,
                &mut self.accounting,
            );
            self.record_provider_observations(turn_number, participant_index, &participant);
            let turn = request.map_err(|error| {
                new!(ProtocolRunError::Model {
                    participant: participant.clone(),
                    message: error.to_string(),
                })
            })?;
            if let Some(calls) = turn.tool_calls() {
                tool_reprompts = 0;
                let calls = calls.to_vec();
                let mut submitted = false;
                for call in &calls {
                    let content = if submitted {
                        record_protocol_error(
                            &mut self.events,
                            &participant,
                            phase,
                            &call.function.name,
                            "The participant's independent answer is already recorded; this call was rejected."
                                .to_owned(),
                        )
                    } else if ProtocolTool::from_name(&call.function.name)
                        == Some(ProtocolTool::SubmitAnswer)
                    {
                        let action =
                            self.dispatch_submit_answer(turn_number, &participant, phase, call);
                        submitted = self.answers.contains_key(&participant);
                        let bityzba::data!(AnswerSubmissionAction { content, .. }) =
                            action.into_data();
                        content
                    } else {
                        record_protocol_error(
                            &mut self.events,
                            &participant,
                            phase,
                            &call.function.name,
                            "Only `submit_answer` is available after the visible dialog closes."
                                .to_owned(),
                        )
                    };
                    self.participants[participant_index].push_tool_result(call, content);
                }
                if submitted {
                    return Ok(ClosedAnswerSubmissionOutcome::Submitted);
                }
            } else if turn.content().is_some() {
                if tool_choice == ProviderToolChoice::Auto {
                    if !record_auto_prose_rejection(
                        &mut self.events,
                        turn_number,
                        &participant,
                        phase,
                        &mut tool_reprompts,
                        max_tool_reprompts,
                    ) {
                        record_protocol_error(
                            &mut self.events,
                            &participant,
                            phase,
                            "(no tool call)",
                            "Independent answer collection was abandoned after repeated prose responses."
                                .to_owned(),
                        );
                        return Ok(ClosedAnswerSubmissionOutcome::Abandoned);
                    }
                    self.participants[participant_index].push_tool_correction(&tools);
                } else {
                    let correction = record_protocol_error(
                        &mut self.events,
                        &participant,
                        phase,
                        "(no tool call)",
                        "The independent answer must be submitted with `submit_answer`.".to_owned(),
                    );
                    self.participants[participant_index].push_user(correction);
                }
            } else if let Some(record) = turn.abort_record() {
                let record = record.clone();
                self.events.push(new!(ProtocolEvent::RunAborted {
                    record: record.clone(),
                }));
                return Ok(ClosedAnswerSubmissionOutcome::BudgetAborted { record });
            } else {
                unreachable!("ModelTurn invariants cover all variants");
            }
        }
    }

    #[requires(turn_number > 0)]
    #[requires(turn_number <= turn_limit)]
    #[requires(speaker_index < self.participants.len())]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    fn run_speaker(
        &mut self,
        turn_number: usize,
        turn_limit: usize,
        speaker_index: usize,
    ) -> Result<SpeakerOutcome, ProtocolRunError> {
        let speaker = self.participants[speaker_index]
            .participant_name()
            .to_owned();
        let instruction = phase_instruction(
            SPEAKER_TURN_INSTRUCTION,
            Some(new!(TurnDeadline {
                turn_number,
                maximum_turns: turn_limit,
            })),
            self.answer_affordance(turn_number, &speaker),
        );
        self.participants[speaker_index].begin_tool_loop();
        self.participants[speaker_index].push_user(instruction);
        let mut state = SpeakerState::awaiting_intent();
        let mut reference_state = ReferencePhaseState::new(ProtocolPhase::Speaker {
            phase: state.phase(),
        });
        let tool_choice = self.participants[speaker_index].tool_choice();
        let max_tool_reprompts = self.participants[speaker_index].max_tool_reprompts();
        let mut tool_reprompts = 0usize;
        loop {
            let phase = ProtocolPhase::Speaker {
                phase: state.phase(),
            };
            reference_state.enter_phase(phase);
            let answer_schema = self.answer_schema_if_available(turn_number, &speaker);
            let tools = ProtocolTools::definitions_for_phase(
                phase,
                answer_schema.as_ref(),
                reference_state.reference_tools_available(self.caps.max_reference_calls_per_phase),
            )
            .map_err(|error| {
                new!(ProtocolRunError::ToolDefinitions {
                    message: error.to_string(),
                })
            })?;
            let request =
                self.participants[speaker_index].request(&tools, tool_choice, &mut self.accounting);
            self.record_provider_observations(turn_number, speaker_index, &speaker);
            let turn = request.map_err(|error| {
                new!(ProtocolRunError::Model {
                    participant: speaker.clone(),
                    message: error.to_string(),
                })
            })?;
            if let Some(calls) = turn.tool_calls() {
                tool_reprompts = 0;
                let calls = calls.to_vec();
                let mut outcome = None;
                let mut corrective_messages = Vec::new();
                for call in &calls {
                    let action = if outcome.is_some() {
                        let phase = ProtocolPhase::Speaker {
                            phase: state.phase(),
                        };
                        new!(SpeakerAction::Continue {
                            content: record_protocol_error(
                                &mut self.events,
                                &speaker,
                                phase,
                                &call.function.name,
                                "The speaker turn is already complete; this call was rejected."
                                    .to_owned(),
                            ),
                        })
                    } else if is_reference_tool(&call.function.name) {
                        let dispatch = dispatch_reference(
                            &mut self.reference_dispatcher,
                            &mut self.events,
                            &speaker,
                            ProtocolPhase::Speaker {
                                phase: state.phase(),
                            },
                            call,
                            &mut reference_state,
                            &self.caps,
                            answer_schema.is_some(),
                        );
                        let bityzba::data!(ReferenceDispatch {
                            content,
                            corrective_message,
                        }) = dispatch.into_data();
                        if let Some(message) = corrective_message {
                            corrective_messages.push(message);
                        }
                        new!(SpeakerAction::Continue { content })
                    } else {
                        if ProtocolTool::from_name(&call.function.name).is_some() {
                            reference_state.record_protocol_call(ProtocolPhase::Speaker {
                                phase: state.phase(),
                            });
                            corrective_messages.clear();
                        }
                        self.dispatch_speaker_protocol(turn_number, &speaker, &mut state, call)?
                    };
                    let content = action.content().to_owned();
                    self.participants[speaker_index].push_tool_result(call, content);
                    match action.as_data() {
                        bityzba::data!(SpeakerAction::Continue { .. }) => {}
                        bityzba::data!(SpeakerAction::Posted { message, .. }) => {
                            outcome = Some(new!(SpeakerOutcome::Posted {
                                message: message.clone(),
                            }));
                        }
                        bityzba::data!(SpeakerAction::Forfeited { .. }) => {
                            outcome = Some(new!(SpeakerOutcome::Forfeited));
                        }
                        bityzba::data!(SpeakerAction::ScenarioCompleted { .. }) => {
                            outcome = Some(new!(SpeakerOutcome::ScenarioCompleted));
                        }
                    }
                }
                if outcome.is_none() {
                    for message in corrective_messages {
                        self.participants[speaker_index].push_user(message);
                    }
                }
                if let Some(outcome) = outcome {
                    return Ok(outcome);
                }
            } else if turn.content().is_some() {
                if tool_choice == ProviderToolChoice::Auto {
                    if !record_auto_prose_rejection(
                        &mut self.events,
                        turn_number,
                        &speaker,
                        phase,
                        &mut tool_reprompts,
                        max_tool_reprompts,
                    ) {
                        self.events.push(new!(ProtocolEvent::TurnForfeited {
                            turn_number,
                            speaker: speaker.clone(),
                            reason: new!(TurnForfeitReason::ProtocolProseResponses {
                                maximum_attempts: max_tool_reprompts.saturating_add(1),
                            }),
                        }));
                        return Ok(new!(SpeakerOutcome::Forfeited));
                    }
                    self.participants[speaker_index].push_tool_correction(&tools);
                } else {
                    let correction = record_protocol_error(
                        &mut self.events,
                        &speaker,
                        phase,
                        "(no tool call)",
                        "A protocol tool call is required in this state.".to_owned(),
                    );
                    self.participants[speaker_index].push_user(correction);
                }
            } else if let Some(record) = turn.abort_record() {
                let record = record.clone();
                self.events.push(new!(ProtocolEvent::RunAborted {
                    record: record.clone(),
                }));
                return Ok(new!(SpeakerOutcome::BudgetAborted { record }));
            } else {
                unreachable!("ModelTurn invariants cover all variants");
            }
        }
    }

    #[requires(turn_number > 0)]
    #[requires(turn_number <= turn_limit)]
    #[requires(listener_index < self.participants.len())]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    fn run_listener(
        &mut self,
        turn_number: usize,
        turn_limit: usize,
        speaker: &str,
        listener_index: usize,
        message: &VisibleMessage,
    ) -> Result<ListenerRunOutcome, ProtocolRunError> {
        let listener = self.participants[listener_index]
            .participant_name()
            .to_owned();
        self.events.push(new!(ProtocolEvent::ListenerFlowStarted {
            turn_number,
            speaker: speaker.to_owned(),
            listener: listener.clone(),
            mode: self.listener_mode,
            message: message.clone(),
        }));
        let (prompt, mut state) = match self.listener_mode {
            ListenerMode::Informed => {
                let informed = message.revealed();
                (
                    informed.informed_prompt()?,
                    ListenerState::informed(informed),
                )
            }
            ListenerMode::BlindThenReveal => {
                let blind = message.blind();
                (blind.prompt(), ListenerState::blind(blind))
            }
        };
        let instruction = phase_instruction(
            &prompt,
            Some(new!(TurnDeadline {
                turn_number,
                maximum_turns: turn_limit,
            })),
            new!(AnswerAffordance::Unavailable),
        );
        self.participants[listener_index].begin_tool_loop();
        self.participants[listener_index].push_user(instruction);
        let mut reference_state = ReferencePhaseState::new(ProtocolPhase::Listener {
            phase: state.phase(),
        });
        let tool_choice = self.participants[listener_index].tool_choice();
        let max_tool_reprompts = self.participants[listener_index].max_tool_reprompts();
        let mut tool_reprompts = 0usize;
        loop {
            let phase = ProtocolPhase::Listener {
                phase: state.phase(),
            };
            reference_state.enter_phase(phase);
            let answer_schema = self.answer_schema_if_available(turn_number, &listener);
            let tools = ProtocolTools::definitions_for_phase(
                phase,
                answer_schema.as_ref(),
                reference_state.reference_tools_available(self.caps.max_reference_calls_per_phase),
            )
            .map_err(|error| {
                new!(ProtocolRunError::ToolDefinitions {
                    message: error.to_string(),
                })
            })?;
            let request = self.participants[listener_index].request(
                &tools,
                tool_choice,
                &mut self.accounting,
            );
            self.record_provider_observations(turn_number, listener_index, &listener);
            let turn = request.map_err(|error| {
                new!(ProtocolRunError::Model {
                    participant: listener.clone(),
                    message: error.to_string(),
                })
            })?;
            if let Some(calls) = turn.tool_calls() {
                tool_reprompts = 0;
                let calls = calls.to_vec();
                let mut acknowledged = false;
                let mut scenario_completed = false;
                let mut corrective_messages = Vec::new();
                for call in &calls {
                    let action = if acknowledged || scenario_completed {
                        new!(ListenerAction::Continue {
                            content: record_protocol_error(
                                &mut self.events,
                                &listener,
                                ProtocolPhase::Listener {
                                    phase: state.phase(),
                                },
                                &call.function.name,
                                "The listener flow is already complete; this call was rejected."
                                    .to_owned(),
                            ),
                        })
                    } else if is_reference_tool(&call.function.name) {
                        let dispatch = dispatch_reference(
                            &mut self.reference_dispatcher,
                            &mut self.events,
                            &listener,
                            ProtocolPhase::Listener {
                                phase: state.phase(),
                            },
                            call,
                            &mut reference_state,
                            &self.caps,
                            answer_schema.is_some(),
                        );
                        let bityzba::data!(ReferenceDispatch {
                            content,
                            corrective_message,
                        }) = dispatch.into_data();
                        if let Some(message) = corrective_message {
                            corrective_messages.push(message);
                        }
                        new!(ListenerAction::Continue { content })
                    } else {
                        if ProtocolTool::from_name(&call.function.name).is_some() {
                            reference_state.record_protocol_call(ProtocolPhase::Listener {
                                phase: state.phase(),
                            });
                            corrective_messages.clear();
                        }
                        self.dispatch_listener_protocol(
                            turn_number,
                            speaker,
                            &listener,
                            message,
                            &mut state,
                            call,
                        )?
                    };
                    let content = action.content().to_owned();
                    self.participants[listener_index].push_tool_result(call, content);
                    match action.as_data() {
                        bityzba::data!(ListenerAction::Continue { .. }) => {}
                        bityzba::data!(ListenerAction::Reveal { prompt, .. }) => {
                            self.participants[listener_index].push_user(prompt.clone());
                        }
                        bityzba::data!(ListenerAction::Acknowledged { .. }) => {
                            acknowledged = true;
                        }
                        bityzba::data!(ListenerAction::ScenarioCompleted { .. }) => {
                            scenario_completed = true;
                        }
                    }
                }
                if !acknowledged && !scenario_completed {
                    for message in corrective_messages {
                        self.participants[listener_index].push_user(message);
                    }
                }
                if scenario_completed {
                    return Ok(ListenerRunOutcome::ScenarioCompleted);
                }
                if acknowledged {
                    return Ok(ListenerRunOutcome::Acknowledged);
                }
            } else if turn.content().is_some() {
                if tool_choice == ProviderToolChoice::Auto {
                    if !record_auto_prose_rejection(
                        &mut self.events,
                        turn_number,
                        &listener,
                        phase,
                        &mut tool_reprompts,
                        max_tool_reprompts,
                    ) {
                        self.events.push(new!(ProtocolEvent::ListenerFlowAbandoned {
                            turn_number,
                            listener: listener.clone(),
                            phase,
                            reason: new!(ListenerFlowAbandonReason::ProtocolProseResponses {
                                maximum_attempts: max_tool_reprompts.saturating_add(1),
                            }),
                        }));
                        return Ok(ListenerRunOutcome::Abandoned);
                    }
                    self.participants[listener_index].push_tool_correction(&tools);
                } else {
                    let correction = record_protocol_error(
                        &mut self.events,
                        &listener,
                        phase,
                        "(no tool call)",
                        "A protocol tool call is required in this state.".to_owned(),
                    );
                    self.participants[listener_index].push_user(correction);
                }
            } else if let Some(record) = turn.abort_record() {
                let record = record.clone();
                self.events.push(new!(ProtocolEvent::RunAborted {
                    record: record.clone(),
                }));
                return Ok(ListenerRunOutcome::BudgetAborted { record });
            } else {
                unreachable!("ModelTurn invariants cover all variants");
            }
        }
    }

    #[requires(turn_number > 0)]
    #[requires(!speaker.trim().is_empty())]
    #[requires(!call.function.name.trim().is_empty())]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    fn dispatch_speaker_protocol(
        &mut self,
        turn_number: usize,
        speaker: &str,
        state: &mut SpeakerState,
        call: &ToolCall,
    ) -> Result<SpeakerAction, ProtocolRunError> {
        let phase = ProtocolPhase::Speaker {
            phase: state.phase(),
        };
        let Some(tool) = ProtocolTool::from_name(&call.function.name) else {
            return Ok(new!(SpeakerAction::Continue {
                content: record_protocol_error(
                    &mut self.events,
                    speaker,
                    phase,
                    &call.function.name,
                    format!(
                        "Unknown tool `{}`; call one of the tools offered for the current state.",
                        call.function.name
                    ),
                ),
            }));
        };
        let answer_available = self
            .answer_schema_if_available(turn_number, speaker)
            .is_some();
        if let Err(content) =
            validate_protocol_tool(&mut self.events, speaker, phase, tool, answer_available)
        {
            return Ok(new!(SpeakerAction::Continue { content }));
        }
        if tool == ProtocolTool::SubmitAnswer {
            let action = self.dispatch_submit_answer(turn_number, speaker, phase, call);
            let bityzba::data!(AnswerSubmissionAction {
                content,
                scenario_completed,
            }) = action.into_data();
            return Ok(if scenario_completed {
                new!(SpeakerAction::ScenarioCompleted { content })
            } else {
                new!(SpeakerAction::Continue { content })
            });
        }
        match tool {
            ProtocolTool::RegisterIntent => {
                let arguments = match decode_arguments::<RegisterIntentArguments>(call) {
                    Ok(arguments) => arguments,
                    Err(message) => {
                        return Ok(new!(SpeakerAction::Continue {
                            content: record_protocol_error(
                                &mut self.events,
                                speaker,
                                phase,
                                tool.name(),
                                message,
                            ),
                        }));
                    }
                };
                let (revision, revision_number, parse_attempts) = match state.as_data() {
                    bityzba::data!(SpeakerState::AwaitingIntent) => (false, 0, 0),
                    bityzba::data!(SpeakerState::Composing {
                        intent_revisions,
                        parse_attempts,
                        ..
                    }) => {
                        if *intent_revisions >= self.caps.max_intent_revisions_per_turn {
                            self.events.push(new!(ProtocolEvent::TurnForfeited {
                                turn_number,
                                speaker: speaker.to_owned(),
                                reason: new!(TurnForfeitReason::IntentRevisions {
                                    maximum: self.caps.max_intent_revisions_per_turn,
                                }),
                            }));
                            return Ok(new!(SpeakerAction::Forfeited {
                                content: format!(
                                    "Intent revision cap ({}) exceeded; the turn is forfeited.",
                                    self.caps.max_intent_revisions_per_turn
                                ),
                            }));
                        }
                        (true, *intent_revisions + 1, *parse_attempts)
                    }
                    _ => unreachable!("tool legality was validated against the state"),
                };
                let RegisterIntentArguments { meaning_en } = arguments;
                *state = new!(SpeakerState::Composing {
                    meaning_en: meaning_en.clone(),
                    intent_revisions: revision_number,
                    parse_attempts,
                });
                self.events.push(new!(ProtocolEvent::IntentRegistered {
                    turn_number,
                    speaker: speaker.to_owned(),
                    meaning_en,
                    revision,
                    revision_number,
                }));
                Ok(new!(SpeakerAction::Continue {
                    content: phase_instruction(
                        "Intent registered. Compose Lojban and call submit_lojban.",
                        None,
                        self.answer_affordance(turn_number, speaker),
                    ),
                }))
            }
            ProtocolTool::SubmitLojban => {
                let arguments = match decode_arguments::<SubmitLojbanArguments>(call) {
                    Ok(arguments) => arguments,
                    Err(message) => {
                        return Ok(new!(SpeakerAction::Continue {
                            content: record_protocol_error(
                                &mut self.events,
                                speaker,
                                phase,
                                tool.name(),
                                message,
                            ),
                        }));
                    }
                };
                let bityzba::data!(SpeakerState::Composing {
                    meaning_en,
                    intent_revisions,
                    parse_attempts,
                }) = state.as_data()
                else {
                    unreachable!("tool legality was validated against the state");
                };
                if *parse_attempts >= self.caps.max_parse_attempts_per_turn {
                    self.events.push(new!(ProtocolEvent::TurnForfeited {
                        turn_number,
                        speaker: speaker.to_owned(),
                        reason: new!(TurnForfeitReason::ParseAttempts {
                            maximum: self.caps.max_parse_attempts_per_turn,
                        }),
                    }));
                    return Ok(new!(SpeakerAction::Forfeited {
                        content: format!(
                            "Parse-attempt cap ({}) exhausted; the turn is forfeited.",
                            self.caps.max_parse_attempts_per_turn
                        ),
                    }));
                }
                let meaning_en = meaning_en.clone();
                let intent_revisions = *intent_revisions;
                let attempt = *parse_attempts + 1;
                let SubmitLojbanArguments { text } = arguments;
                self.events.push(new!(ProtocolEvent::CandidateSubmitted {
                    turn_number,
                    speaker: speaker.to_owned(),
                    text: text.clone(),
                    attempt,
                }));
                let outcome = crate::gate_lojban(text.clone(), Some(self.tersmu_format), None)
                    .map_err(|error| {
                        new!(ProtocolRunError::Gate {
                            participant: speaker.to_owned(),
                            message: error.to_string(),
                        })
                    })?;
                if let Some(diagnostics) = outcome.diagnostics_rendering() {
                    let diagnostics = diagnostics.to_owned();
                    let diagnostic_category = outcome
                        .diagnostic_category()
                        .expect("rejected gate outcomes carry a structured category");
                    *state = new!(SpeakerState::Composing {
                        meaning_en,
                        intent_revisions,
                        parse_attempts: attempt,
                    });
                    self.events.push(new!(ProtocolEvent::CandidateRejected {
                        turn_number,
                        speaker: speaker.to_owned(),
                        text,
                        diagnostics: diagnostics.clone(),
                        diagnostic_category,
                        attempt,
                    }));
                    if attempt >= self.caps.max_parse_attempts_per_turn {
                        self.events.push(new!(ProtocolEvent::TurnForfeited {
                            turn_number,
                            speaker: speaker.to_owned(),
                            reason: new!(TurnForfeitReason::ParseAttempts {
                                maximum: self.caps.max_parse_attempts_per_turn,
                            }),
                        }));
                        Ok(new!(SpeakerAction::Forfeited {
                            content: diagnostics,
                        }))
                    } else {
                        Ok(new!(SpeakerAction::Continue {
                            content: diagnostics,
                        }))
                    }
                } else if let Some(tersmu_rendering) = outcome.tersmu_rendering() {
                    let message = VisibleMessage::try_from_data(bityzba::data!(VisibleMessage {
                        text,
                        tersmu_rendering: tersmu_rendering.to_vec(),
                    }))
                    .expect("successful gate guarantees a nonempty rendering");
                    let content = std::str::from_utf8(&message.tersmu_rendering)
                        .map_err(|error| {
                            new!(ProtocolRunError::InvalidTersmuEncoding {
                                message: error.to_string(),
                            })
                        })?
                        .to_owned();
                    *state = new!(SpeakerState::AwaitingConfirmation {
                        meaning_en,
                        intent_revisions,
                        parse_attempts: attempt,
                        candidate: message.clone(),
                    });
                    self.events.push(new!(ProtocolEvent::CandidateAccepted {
                        turn_number,
                        speaker: speaker.to_owned(),
                        message,
                        attempt,
                    }));
                    Ok(new!(SpeakerAction::Continue { content }))
                } else {
                    unreachable!("GateOutcome invariants cover success and failure")
                }
            }
            ProtocolTool::ConfirmMeaning => {
                let arguments = match decode_arguments::<ConfirmMeaningArguments>(call) {
                    Ok(arguments) => arguments,
                    Err(message) => {
                        return Ok(new!(SpeakerAction::Continue {
                            content: record_protocol_error(
                                &mut self.events,
                                speaker,
                                phase,
                                tool.name(),
                                message,
                            ),
                        }));
                    }
                };
                let bityzba::data!(SpeakerState::AwaitingConfirmation {
                    meaning_en,
                    intent_revisions,
                    parse_attempts,
                    candidate,
                }) = state.as_data()
                else {
                    unreachable!("tool legality was validated against the state");
                };
                let meaning_en = meaning_en.clone();
                let intent_revisions = *intent_revisions;
                let parse_attempts = *parse_attempts;
                let candidate = candidate.clone();
                let ConfirmMeaningArguments {
                    matches,
                    paraphrase_en,
                    discrepancies,
                } = arguments;
                self.events.push(new!(ProtocolEvent::MeaningConfirmed {
                    turn_number,
                    speaker: speaker.to_owned(),
                    matches,
                    paraphrase_en,
                    discrepancies,
                }));
                if matches {
                    *state = new!(SpeakerState::Posted {
                        message: candidate.clone(),
                    });
                    self.events.push(new!(ProtocolEvent::MessagePosted {
                        turn_number,
                        speaker: speaker.to_owned(),
                        message: candidate.clone(),
                    }));
                    Ok(new!(SpeakerAction::Posted {
                        content: "Meaning confirmed; the Lojban and tersmu rendering were posted."
                            .to_owned(),
                        message: candidate,
                    }))
                } else {
                    *state = new!(SpeakerState::Composing {
                        meaning_en,
                        intent_revisions,
                        parse_attempts,
                    });
                    if parse_attempts >= self.caps.max_parse_attempts_per_turn {
                        self.events.push(new!(ProtocolEvent::TurnForfeited {
                            turn_number,
                            speaker: speaker.to_owned(),
                            reason: new!(TurnForfeitReason::ParseAttempts {
                                maximum: self.caps.max_parse_attempts_per_turn,
                            }),
                        }));
                        Ok(new!(SpeakerAction::Forfeited {
                            content: "Mismatch recorded, but no parse attempts remain; the turn is forfeited."
                                .to_owned(),
                        }))
                    } else {
                        Ok(new!(SpeakerAction::Continue {
                            content:
                                "Mismatch recorded. Revise the Lojban and call submit_lojban again."
                                    .to_owned(),
                        }))
                    }
                }
            }
            ProtocolTool::InterpretBlind
            | ProtocolTool::Acknowledge
            | ProtocolTool::SubmitAnswer => {
                unreachable!("listener tools cannot pass speaker-state validation")
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    #[requires(turn_number > 0)]
    #[requires(!speaker.trim().is_empty())]
    #[requires(!listener.trim().is_empty())]
    #[requires(!call.function.name.trim().is_empty())]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    fn dispatch_listener_protocol(
        &mut self,
        turn_number: usize,
        speaker: &str,
        listener: &str,
        visible_message: &VisibleMessage,
        state: &mut ListenerState,
        call: &ToolCall,
    ) -> Result<ListenerAction, ProtocolRunError> {
        let phase = ProtocolPhase::Listener {
            phase: state.phase(),
        };
        let Some(tool) = ProtocolTool::from_name(&call.function.name) else {
            return Ok(new!(ListenerAction::Continue {
                content: record_protocol_error(
                    &mut self.events,
                    listener,
                    phase,
                    &call.function.name,
                    format!(
                        "Unknown tool `{}`; call one of the tools offered for the current state.",
                        call.function.name
                    ),
                ),
            }));
        };
        let answer_available = self
            .answer_schema_if_available(turn_number, listener)
            .is_some();
        if let Err(content) =
            validate_protocol_tool(&mut self.events, listener, phase, tool, answer_available)
        {
            return Ok(new!(ListenerAction::Continue { content }));
        }
        if tool == ProtocolTool::SubmitAnswer {
            let action = self.dispatch_submit_answer(turn_number, listener, phase, call);
            let bityzba::data!(AnswerSubmissionAction {
                content,
                scenario_completed,
            }) = action.into_data();
            return Ok(if scenario_completed {
                new!(ListenerAction::ScenarioCompleted { content })
            } else {
                new!(ListenerAction::Continue { content })
            });
        }
        match tool {
            ProtocolTool::InterpretBlind => {
                let arguments = match decode_arguments::<InterpretBlindArguments>(call) {
                    Ok(arguments) => arguments,
                    Err(message) => {
                        return Ok(new!(ListenerAction::Continue {
                            content: record_protocol_error(
                                &mut self.events,
                                listener,
                                phase,
                                tool.name(),
                                message,
                            ),
                        }));
                    }
                };
                let InterpretBlindArguments { interpretation_en } = arguments;
                self.events
                    .push(new!(ProtocolEvent::BlindInterpretationRecorded {
                        turn_number,
                        speaker: speaker.to_owned(),
                        listener: listener.to_owned(),
                        interpretation_en: interpretation_en.clone(),
                    }));
                let revealed = visible_message.revealed();
                *state = new!(ListenerState::TersmuRevealed {
                    message: revealed.clone(),
                    interpretation_en,
                });
                self.events.push(new!(ProtocolEvent::TersmuRevealed {
                    turn_number,
                    speaker: speaker.to_owned(),
                    listener: listener.to_owned(),
                    message: visible_message.clone(),
                }));
                Ok(new!(ListenerAction::Reveal {
                    content: "Blind interpretation recorded; the tersmu rendering has now been revealed privately."
                        .to_owned(),
                    prompt: phase_instruction(
                        &revealed.prompt()?,
                        None,
                        self.answer_affordance(turn_number, listener),
                    ),
                }))
            }
            ProtocolTool::Acknowledge => {
                let arguments = match decode_arguments::<AcknowledgeArguments>(call) {
                    Ok(arguments) => arguments,
                    Err(message) => {
                        return Ok(new!(ListenerAction::Continue {
                            content: record_protocol_error(
                                &mut self.events,
                                listener,
                                phase,
                                tool.name(),
                                message,
                            ),
                        }));
                    }
                };
                let AcknowledgeArguments {
                    final_understanding_en,
                    discrepancies,
                } = arguments;
                *state = new!(ListenerState::Acknowledged {
                    final_understanding_en: final_understanding_en.clone(),
                    discrepancies: discrepancies.clone(),
                });
                self.events.push(new!(ProtocolEvent::Acknowledged {
                    turn_number,
                    speaker: speaker.to_owned(),
                    listener: listener.to_owned(),
                    final_understanding_en,
                    discrepancies,
                }));
                Ok(new!(ListenerAction::Acknowledged {
                    content: "Acknowledgement recorded; listener flow complete.".to_owned(),
                }))
            }
            ProtocolTool::RegisterIntent
            | ProtocolTool::SubmitLojban
            | ProtocolTool::ConfirmMeaning
            | ProtocolTool::SubmitAnswer => {
                unreachable!("speaker tools cannot pass listener-state validation")
            }
        }
    }

    #[requires(turn_number > 0)]
    #[requires(!participant.trim().is_empty())]
    #[requires(!call.function.name.trim().is_empty())]
    #[ensures(!ret.content.is_empty())]
    fn dispatch_submit_answer(
        &mut self,
        turn_number: usize,
        participant: &str,
        phase: ProtocolPhase,
        call: &ToolCall,
    ) -> AnswerSubmissionAction {
        let value = match serde_json::from_str::<Value>(&call.function.arguments) {
            Ok(value) if value.is_object() => value,
            Ok(_) => {
                return new!(AnswerSubmissionAction {
                    content: record_protocol_error(
                        &mut self.events,
                        participant,
                        phase,
                        ProtocolTool::SubmitAnswer.name(),
                        "Invalid arguments for protocol tool `submit_answer`: expected a JSON object."
                            .to_owned(),
                    ),
                    scenario_completed: false,
                });
            }
            Err(error) => {
                return new!(AnswerSubmissionAction {
                    content: record_protocol_error(
                        &mut self.events,
                        participant,
                        phase,
                        ProtocolTool::SubmitAnswer.name(),
                        format!("Invalid arguments for protocol tool `submit_answer`: {error}"),
                    ),
                    scenario_completed: false,
                });
            }
        };
        let scenario = self
            .scenario
            .as_ref()
            .expect("tool legality guarantees a scenario");
        let answer = match scenario.parse_answer(value) {
            Ok(answer) => answer,
            Err(error) => {
                return new!(AnswerSubmissionAction {
                    content: record_protocol_error(
                        &mut self.events,
                        participant,
                        phase,
                        ProtocolTool::SubmitAnswer.name(),
                        error.to_string(),
                    ),
                    scenario_completed: false,
                });
            }
        };
        self.answers.insert(participant.to_owned(), answer.clone());
        self.events.push(new!(ProtocolEvent::AnswerSubmitted {
            turn_number,
            participant: participant.to_owned(),
            answer,
        }));
        let scenario_completed = scenario.all_required_submitted(&self.answers);
        if scenario_completed {
            self.finish_scenario(turn_number);
        }
        let content = if scenario_completed {
            "Answer recorded; all required answers are present and the checker has completed."
        } else if phase == ProtocolPhase::Answer {
            "Answer recorded; the visible dialog remains closed while the other required participants answer independently."
        } else {
            "Answer recorded; continue the current protocol phase while other required participants answer."
        };
        new!(AnswerSubmissionAction {
            content: phase_instruction(
                content,
                None,
                self.answer_affordance(turn_number, participant),
            ),
            scenario_completed,
        })
    }
}

#[invariant(::Posted { message } => !message.text.trim().is_empty() && !message.tersmu_rendering.is_empty())]
#[invariant(::Forfeited => true)]
#[invariant(::ScenarioCompleted => true)]
#[invariant(::BudgetAborted { .. } => true)]
enum SpeakerOutcome {
    Posted { message: VisibleMessage },
    Forfeited,
    ScenarioCompleted,
    BudgetAborted { record: AbortRecord },
}

#[invariant(::Acknowledged => true)]
#[invariant(::Abandoned => true)]
#[invariant(::ScenarioCompleted => true)]
#[invariant(::BudgetAborted { .. } => true)]
enum ListenerRunOutcome {
    Acknowledged,
    Abandoned,
    ScenarioCompleted,
    BudgetAborted { record: AbortRecord },
}

#[invariant(::AllSubmitted => true)]
#[invariant(::Incomplete => true)]
#[invariant(::BudgetAborted { .. } => true)]
enum ClosedAnswerPhaseOutcome {
    AllSubmitted,
    Incomplete,
    BudgetAborted { record: AbortRecord },
}

#[invariant(::Submitted => true)]
#[invariant(::Abandoned => true)]
#[invariant(::BudgetAborted { .. } => true)]
enum ClosedAnswerSubmissionOutcome {
    Submitted,
    Abandoned,
    BudgetAborted { record: AbortRecord },
}

#[invariant(::Continue { content } => !content.is_empty())]
#[invariant(::Posted { content, message } => !content.is_empty() && !message.text.trim().is_empty() && !message.tersmu_rendering.is_empty())]
#[invariant(::Forfeited { content } => !content.is_empty())]
#[invariant(::ScenarioCompleted { content } => !content.is_empty())]
enum SpeakerAction {
    Continue {
        content: String,
    },
    Posted {
        content: String,
        message: VisibleMessage,
    },
    Forfeited {
        content: String,
    },
    ScenarioCompleted {
        content: String,
    },
}

impl SpeakerAction {
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn content(&self) -> &str {
        match self.as_data() {
            bityzba::data!(SpeakerAction::Continue { content })
            | bityzba::data!(SpeakerAction::Posted { content, .. })
            | bityzba::data!(SpeakerAction::Forfeited { content })
            | bityzba::data!(SpeakerAction::ScenarioCompleted { content }) => content,
        }
    }
}

#[invariant(::Continue { content } => !content.is_empty())]
#[invariant(::Reveal { content, prompt } => !content.is_empty() && !prompt.trim().is_empty())]
#[invariant(::Acknowledged { content } => !content.is_empty())]
#[invariant(::ScenarioCompleted { content } => !content.is_empty())]
enum ListenerAction {
    Continue { content: String },
    Reveal { content: String, prompt: String },
    Acknowledged { content: String },
    ScenarioCompleted { content: String },
}

impl ListenerAction {
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn content(&self) -> &str {
        match self.as_data() {
            bityzba::data!(ListenerAction::Continue { content })
            | bityzba::data!(ListenerAction::Reveal { content, .. })
            | bityzba::data!(ListenerAction::Acknowledged { content })
            | bityzba::data!(ListenerAction::ScenarioCompleted { content }) => content,
        }
    }
}

#[invariant(!content.is_empty())]
struct AnswerSubmissionAction {
    content: String,
    scenario_completed: bool,
}

#[requires(!participant.trim().is_empty())]
#[ensures(ret.is_ok() == phase.allows(tool, submit_answer_available))]
fn validate_protocol_tool(
    events: &mut ProtocolEventLog,
    participant: &str,
    phase: ProtocolPhase,
    tool: ProtocolTool,
    submit_answer_available: bool,
) -> Result<(), String> {
    if phase.allows(tool, submit_answer_available) {
        return Ok(());
    }
    Err(record_protocol_error(
        events,
        participant,
        phase,
        tool.name(),
        format!(
            "Protocol tool `{}` is not legal in state {phase:?}; call one of the currently offered tools.",
            tool.name()
        ),
    ))
}

#[requires(!participant.trim().is_empty())]
#[requires(!tool_name.trim().is_empty())]
#[requires(!message.trim().is_empty())]
#[ensures(!ret.is_empty())]
fn record_protocol_error(
    events: &mut ProtocolEventLog,
    participant: &str,
    phase: ProtocolPhase,
    tool_name: &str,
    message: String,
) -> String {
    events.push(new!(ProtocolEvent::ProtocolError {
        participant: participant.to_owned(),
        phase,
        tool_name: tool_name.to_owned(),
        message: message.clone(),
    }));
    message
}

/// Record one automatic prose response and consume a corrective reprompt slot.
#[requires(turn_number > 0)]
#[requires(!participant.trim().is_empty())]
#[requires(*reprompts <= maximum_reprompts)]
#[ensures(ret == (old(*reprompts) < maximum_reprompts))]
#[ensures(ret -> *reprompts == old(*reprompts) + 1)]
#[ensures(!ret -> *reprompts == old(*reprompts))]
fn record_auto_prose_rejection(
    events: &mut ProtocolEventLog,
    turn_number: usize,
    participant: &str,
    phase: ProtocolPhase,
    reprompts: &mut usize,
    maximum_reprompts: usize,
) -> bool {
    let attempt = reprompts.saturating_add(1);
    events.push(new!(ProtocolEvent::ProseRejected {
        turn_number,
        participant: participant.to_owned(),
        phase,
        attempt,
        maximum_attempts: maximum_reprompts.saturating_add(1),
    }));
    if *reprompts >= maximum_reprompts {
        false
    } else {
        *reprompts += 1;
        true
    }
}

#[requires(!participant.trim().is_empty())]
#[requires(is_reference_tool(&call.function.name))]
#[ensures(!ret.content.is_empty())]
fn dispatch_reference(
    dispatcher: &mut impl ToolDispatcher,
    events: &mut ProtocolEventLog,
    participant: &str,
    phase: ProtocolPhase,
    call: &ToolCall,
    reference_state: &mut ReferencePhaseState,
    caps: &CapsConfig,
    submit_answer_available: bool,
) -> ReferenceDispatch {
    reference_state.enter_phase(phase);
    if !reference_state.reference_tools_available(caps.max_reference_calls_per_phase) {
        return new!(ReferenceDispatch {
            content: record_protocol_error(
                events,
                participant,
                phase,
                &call.function.name,
                reference_budget_message(caps.max_reference_calls_per_phase),
            ),
            corrective_message: None,
        });
    }

    reference_state.calls += 1;
    reference_state.consecutive_calls += 1;
    let remaining_calls = caps
        .max_reference_calls_per_phase
        .saturating_sub(reference_state.calls);
    let key = new!(ReferenceLookupKey {
        tool_name: call.function.name.clone(),
        arguments: call.function.arguments.clone(),
    });

    let execution = if caps.reference_dedupe {
        if let Some(prior) = reference_state.memo.get(&key) {
            let repeat_number = prior.occurrences + 1;
            let result = prior.result.clone();
            let succeeded = prior.succeeded;
            reference_state.memo.insert(
                key.clone(),
                new!(ReferenceMemoEntry {
                    result: result.clone(),
                    succeeded,
                    occurrences: repeat_number,
                }),
            );
            events.push(new!(ProtocolEvent::ReferenceLookupRepeated {
                participant: participant.to_owned(),
                phase,
                tool_name: call.function.name.clone(),
                arguments: call.function.arguments.clone(),
                repeat_number,
                remaining_calls,
            }));
            new!(ReferenceExecution {
                result: format!(
                    "(repeat lookup #{repeat_number} of this exact query this phase — the result has not changed; you have {remaining_calls} reference calls left)\n{result}"
                ),
                succeeded,
            })
        } else {
            let execution = execute_reference(dispatcher, call);
            reference_state.memo.insert(
                key,
                new!(ReferenceMemoEntry {
                    result: execution.result.clone(),
                    succeeded: execution.succeeded,
                    occurrences: 1,
                }),
            );
            execution
        }
    } else {
        execute_reference(dispatcher, call)
    };

    let bityzba::data!(ReferenceExecution { result, succeeded }) = execution.into_data();
    events.push(new!(ProtocolEvent::ReferenceToolCompleted {
        participant: participant.to_owned(),
        phase,
        tool_name: call.function.name.clone(),
        arguments: call.function.arguments.clone(),
        result: result.clone(),
        succeeded,
    }));

    let corrective_message = if reference_state.calls == caps.max_reference_calls_per_phase {
        events.push(new!(ProtocolEvent::ReferenceCallBudgetExhausted {
            participant: participant.to_owned(),
            phase,
            maximum: caps.max_reference_calls_per_phase,
        }));
        Some(reference_budget_message(caps.max_reference_calls_per_phase))
    } else if !reference_state.nudge_sent
        && reference_state.consecutive_calls == caps.reference_nudge_after
    {
        reference_state.nudge_sent = true;
        let message = reference_nudge_message(phase, submit_answer_available);
        events.push(new!(ProtocolEvent::ReferenceResearchNudge {
            participant: participant.to_owned(),
            phase,
            consecutive_calls: reference_state.consecutive_calls,
            message: message.clone(),
        }));
        Some(message)
    } else {
        None
    };

    new!(ReferenceDispatch {
        content: result,
        corrective_message,
    })
}

/// One actual call through the production reference dispatcher.
#[invariant(!result.is_empty())]
#[derive(Debug)]
struct ReferenceExecution {
    result: String,
    succeeded: bool,
}

#[requires(is_reference_tool(&call.function.name))]
#[ensures(!ret.result.is_empty())]
fn execute_reference(dispatcher: &mut impl ToolDispatcher, call: &ToolCall) -> ReferenceExecution {
    match dispatcher.dispatch(call) {
        Ok(result) if !result.is_empty() => new!(ReferenceExecution {
            result,
            succeeded: true,
        }),
        Ok(_) => new!(ReferenceExecution {
            result: format!(
                "Reference tool `{}` returned an empty result.",
                call.function.name
            ),
            succeeded: false,
        }),
        Err(error) => new!(ReferenceExecution {
            result: error.to_string(),
            succeeded: false,
        }),
    }
}

#[requires(maximum > 0)]
#[ensures(!ret.is_empty())]
fn reference_budget_message(maximum: usize) -> String {
    format!(
        "The reference-call budget for this phase is spent ({maximum}/{maximum}). Reference tools are now unavailable; composition or interpretation must proceed with what is already known. Call one of the current phase's protocol tools."
    )
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn reference_nudge_message(phase: ProtocolPhase, submit_answer_available: bool) -> String {
    let tools = ProtocolTool::ALL
        .into_iter()
        .filter(|tool| phase.allows(*tool, submit_answer_available))
        .map(|tool| format!("`{}`", tool.name()))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "Reference research has not advanced the protocol. Current phase protocol tools: {tools}. Current phase intent: {}. Use a protocol tool now unless another reference lookup is essential.",
        phase.intent()
    )
}

#[requires(true)]
#[ensures(ret == REFERENCE_TOOL_NAMES.contains(&name))]
fn is_reference_tool(name: &str) -> bool {
    REFERENCE_TOOL_NAMES.contains(&name)
}

#[requires(!call.function.arguments.trim().is_empty())]
#[ensures(ret.as_ref().err().is_none_or(|message| !message.trim().is_empty()))]
fn decode_arguments<T: ProtocolArguments>(call: &ToolCall) -> Result<T, String> {
    let arguments: T = serde_json::from_str(&call.function.arguments).map_err(|error| {
        format!(
            "Invalid arguments for protocol tool `{}`: {error}",
            call.function.name
        )
    })?;
    arguments.validate().map_err(|message| {
        format!(
            "Invalid arguments for protocol tool `{}`: {message}",
            call.function.name
        )
    })?;
    Ok(arguments)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn phase_instruction_omits_context_when_answer_and_deadline_are_unavailable() {
        let instruction = phase_instruction(
            "Continue the current phase.",
            None,
            new!(AnswerAffordance::Unavailable),
        );

        assert_eq!(instruction, "Continue the current phase.");
        assert!(!instruction.contains("submit_answer"));
        assert!(!instruction.contains("turn "));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn state_tool_matrix_rejects_and_records_every_illegal_pair() {
        let mut examined = 0usize;
        let mut rejected = 0usize;
        let answer_schema = serde_json::json!({
            "type": "object",
            "properties": { "scene_index": { "type": "integer" } },
            "required": ["scene_index"]
        });
        for submit_answer_available in [false, true] {
            for reference_tools_available in [false, true] {
                for phase in ProtocolPhase::ALL {
                    let definitions = ProtocolTools::definitions_for_phase(
                        phase,
                        submit_answer_available.then_some(&answer_schema),
                        reference_tools_available,
                    )
                    .expect("phase definitions must be valid");
                    let names = definitions
                        .iter()
                        .map(ToolDefinition::name)
                        .collect::<BTreeSet<_>>();
                    for reference in REFERENCE_TOOL_NAMES {
                        assert_eq!(
                            names.contains(reference),
                            reference_tools_available,
                            "reference availability for {reference} in {phase:?}"
                        );
                    }
                    for tool in ProtocolTool::ALL {
                        examined += 1;
                        assert_eq!(
                            names.contains(tool.name()),
                            phase.allows(tool, submit_answer_available)
                        );
                        let mut events = ProtocolEventLog::default();
                        let result = validate_protocol_tool(
                            &mut events,
                            "tester",
                            phase,
                            tool,
                            submit_answer_available,
                        );
                        if phase.allows(tool, submit_answer_available) {
                            assert!(result.is_ok(), "legal {phase:?} x {tool:?}");
                            assert!(events.events.is_empty());
                        } else {
                            rejected += 1;
                            assert!(result.is_err(), "illegal {phase:?} x {tool:?}");
                            assert!(matches!(
                                events.events.as_slice(),
                                [event]
                                    if matches!(
                                        event.as_data(),
                                        bityzba::data!(ProtocolEvent::ProtocolError {
                                            phase: event_phase,
                                            tool_name,
                                            ..
                                        }) if *event_phase == phase && tool_name == tool.name()
                                    )
                            ));
                        }
                    }
                }
            }
        }
        assert_eq!(examined, 216);
        assert_eq!(rejected, 170);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn blind_message_type_and_prompt_have_no_rendering_path() {
        let message = new!(VisibleMessage {
            text: "mi klama".to_owned(),
            tersmu_rendering: b"UNIQUE-TERSMU-SENTINEL".to_vec(),
        });
        let blind = message.blind();
        let prompt = blind.prompt();
        assert_eq!(blind.text, "mi klama");
        assert!(!prompt.contains("UNIQUE-TERSMU-SENTINEL"));
        assert!(
            message
                .revealed()
                .prompt()
                .expect("UTF-8")
                .contains("UNIQUE-TERSMU-SENTINEL")
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn protocol_schemas_require_all_semantically_mandatory_fields() {
        let answer_schema = serde_json::json!({
            "type": "object",
            "properties": { "scene_index": { "type": "integer" } },
            "required": ["scene_index"]
        });
        let definitions =
            ProtocolTools::definitions(Some(&answer_schema)).expect("valid definitions");
        for (tool, definition) in definitions {
            let required = definition.function.parameters["required"]
                .as_array()
                .unwrap_or_else(|| panic!("{} must declare required fields", tool.name()));
            let required = required
                .iter()
                .filter_map(serde_json::Value::as_str)
                .collect::<BTreeSet<_>>();
            match tool {
                ProtocolTool::RegisterIntent => {
                    assert_eq!(required, BTreeSet::from(["meaning_en"]))
                }
                ProtocolTool::SubmitLojban => assert_eq!(required, BTreeSet::from(["text"])),
                ProtocolTool::ConfirmMeaning => {
                    assert_eq!(required, BTreeSet::from(["matches", "paraphrase_en"]));
                }
                ProtocolTool::InterpretBlind => {
                    assert_eq!(required, BTreeSet::from(["interpretation_en"]));
                }
                ProtocolTool::Acknowledge => {
                    assert_eq!(required, BTreeSet::from(["final_understanding_en"]));
                }
                ProtocolTool::SubmitAnswer => {
                    assert_eq!(required, BTreeSet::from(["scene_index"]));
                }
            }
        }
    }
}
