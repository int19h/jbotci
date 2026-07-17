//! Tool-gated speaker and listener protocol for xarsnu runs.

use std::collections::BTreeSet;
use std::fmt;

#[allow(unused_imports)]
use bityzba::{contract_trait, ensures, invariant, new, requires};
use schemars::{JsonSchema, schema_for};
use serde::Deserialize;
use thiserror::Error;

use crate::{
    AbortRecord, CapsConfig, OpenRouterClient, ParticipantConfig, ParticipantConversation,
    ReferenceTools, RunAccounting, TersmuFormat, ToolCall, ToolChoice, ToolDefinition,
    ToolDefinitionError, ToolDispatchError, ToolDispatcher,
};

const STANDING_PROTOCOL_RULES: &str = "You are participating in xarsnu's tool-gated Lojban discussion protocol. Think out loud about meaning in this private conversation before acting. English intents, paraphrases, interpretations, and reasoning are private and must never be written into the visible chat. The visible chat is constructed only from confirmed Lojban text and its jbotci tersmu rendering. Use the reference tools whenever they help, and finish each phase by calling the protocol tool currently offered.";
const SPEAKER_TURN_INSTRUCTION: &str = "You are the speaker for this turn. First register your intended meaning in English. Then submit candidate Lojban until jbotci accepts one. Finally compare the returned tersmu rendering with your intent and call confirm_meaning with a mandatory English paraphrase. A mismatch requires revision; a match posts the Lojban and tersmu rendering.";
const LISTENER_BLIND_INSTRUCTION: &str = "Interpret the following visible Lojban message without access to its tersmu rendering. Think privately, then call interpret_blind with your English interpretation.";
const LISTENER_REVEAL_INSTRUCTION: &str = "The tersmu rendering is now revealed. Compare it with your blind reading, then call acknowledge with your final English understanding and any discrepancies.";
const REFERENCE_TOOL_NAMES: [&str; 5] = ["vlacku", "gentufa", "tersmu", "jvozba", "cukta"];

/// One of the five state-changing protocol tools.
#[invariant(::RegisterIntent => true)]
#[invariant(::SubmitLojban => true)]
#[invariant(::ConfirmMeaning => true)]
#[invariant(::InterpretBlind => true)]
#[invariant(::Acknowledge => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProtocolTool {
    RegisterIntent,
    SubmitLojban,
    ConfirmMeaning,
    InterpretBlind,
    Acknowledge,
}

impl ProtocolTool {
    /// Exhaustive protocol-tool list, used by legality checks and tests.
    pub const ALL: [Self; 5] = [
        Self::RegisterIntent,
        Self::SubmitLojban,
        Self::ConfirmMeaning,
        Self::InterpretBlind,
        Self::Acknowledge,
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    pub const fn allows(self, tool: ProtocolTool) -> bool {
        matches!(
            (self, tool),
            (Self::AwaitingIntent, ProtocolTool::RegisterIntent)
                | (Self::Composing, ProtocolTool::RegisterIntent)
                | (Self::Composing, ProtocolTool::SubmitLojban)
                | (Self::AwaitingConfirmation, ProtocolTool::ConfirmMeaning)
        )
    }
}

/// Listener-side state name.
#[invariant(::BlindInterpretation => true)]
#[invariant(::TersmuRevealed => true)]
#[invariant(::Acknowledged => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListenerPhase {
    BlindInterpretation,
    TersmuRevealed,
    Acknowledged,
}

impl ListenerPhase {
    /// Exhaustive listener-state list.
    pub const ALL: [Self; 3] = [
        Self::BlindInterpretation,
        Self::TersmuRevealed,
        Self::Acknowledged,
    ];

    /// Whether this state admits the requested protocol tool.
    #[requires(true)]
    #[ensures(true)]
    pub const fn allows(self, tool: ProtocolTool) -> bool {
        matches!(
            (self, tool),
            (Self::BlindInterpretation, ProtocolTool::InterpretBlind)
                | (Self::TersmuRevealed, ProtocolTool::Acknowledge)
        )
    }
}

/// Unified state name stored in events and used by the tool gate.
#[invariant(::Speaker { .. } => true)]
#[invariant(::Listener { .. } => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolPhase {
    Speaker { phase: SpeakerPhase },
    Listener { phase: ListenerPhase },
}

impl ProtocolPhase {
    /// Exhaustive protocol-state list.
    pub const ALL: [Self; 7] = [
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
            phase: ListenerPhase::BlindInterpretation,
        },
        Self::Listener {
            phase: ListenerPhase::TersmuRevealed,
        },
        Self::Listener {
            phase: ListenerPhase::Acknowledged,
        },
    ];

    /// Whether this state admits the requested protocol tool.
    #[requires(true)]
    #[ensures(true)]
    pub const fn allows(self, tool: ProtocolTool) -> bool {
        match self {
            Self::Speaker { phase } => phase.allows(tool),
            Self::Listener { phase } => phase.allows(tool),
        }
    }
}

/// A confirmed visible-channel message. It cannot carry English private data.
#[invariant(!text.trim().is_empty(), "visible Lojban text cannot be empty")]
#[invariant(!tersmu_rendering.is_empty(), "visible tersmu rendering cannot be empty")]
#[derive(Debug, Clone, PartialEq, Eq)]
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
#[derive(Debug, Clone, PartialEq, Eq)]
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevealedMessage {
    pub text: String,
    pub tersmu_rendering: Vec<u8>,
}

impl RevealedMessage {
    /// Build the reveal prompt, rejecting an impossible non-UTF-8 rendering.
    #[requires(true)]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    pub fn prompt(&self) -> Result<String, ProtocolRunError> {
        let rendering = std::str::from_utf8(&self.tersmu_rendering).map_err(|error| {
            new!(ProtocolRunError::InvalidTersmuEncoding {
                message: error.to_string(),
            })
        })?;
        Ok(format!(
            "{LISTENER_REVEAL_INSTRUCTION}\n\nLojban message:\n{}\n\ntersmu rendering:\n{rendering}",
            self.text
        ))
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

/// Typed listener machine. Its blind variant cannot contain a tersmu rendering.
#[invariant(::BlindInterpretation { message } => !message.text.trim().is_empty())]
#[invariant(::TersmuRevealed { message, interpretation_en } => !message.text.trim().is_empty() && !message.tersmu_rendering.is_empty() && !interpretation_en.trim().is_empty())]
#[invariant(::Acknowledged { final_understanding_en, discrepancies } => !final_understanding_en.trim().is_empty() && discrepancies.as_ref().is_none_or(|value| !value.trim().is_empty()))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ListenerState {
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TurnForfeitReason {
    ParseAttempts { maximum: usize },
    IntentRevisions { maximum: usize },
}

/// Typed private event stream consumed by the later transcript layer.
#[invariant(::TurnStarted { turn_number, speaker } => *turn_number > 0 && !speaker.trim().is_empty())]
#[invariant(::IntentRegistered { turn_number, speaker, meaning_en, .. } => *turn_number > 0 && !speaker.trim().is_empty() && !meaning_en.trim().is_empty())]
#[invariant(::CandidateSubmitted { turn_number, speaker, text, attempt } => *turn_number > 0 && !speaker.trim().is_empty() && !text.trim().is_empty() && *attempt > 0)]
#[invariant(::CandidateRejected { turn_number, speaker, text, diagnostics, attempt } => *turn_number > 0 && !speaker.trim().is_empty() && !text.trim().is_empty() && !diagnostics.is_empty() && *attempt > 0)]
#[invariant(::CandidateAccepted { turn_number, speaker, message, attempt } => *turn_number > 0 && !speaker.trim().is_empty() && !message.text.trim().is_empty() && !message.tersmu_rendering.is_empty() && *attempt > 0)]
#[invariant(::MeaningConfirmed { turn_number, speaker, paraphrase_en, discrepancies, .. } => *turn_number > 0 && !speaker.trim().is_empty() && !paraphrase_en.trim().is_empty() && discrepancies.as_ref().is_none_or(|value| !value.trim().is_empty()))]
#[invariant(::MessagePosted { turn_number, speaker, message } => *turn_number > 0 && !speaker.trim().is_empty() && !message.text.trim().is_empty() && !message.tersmu_rendering.is_empty())]
#[invariant(::BlindInterpretationRecorded { turn_number, speaker, listener, interpretation_en } => *turn_number > 0 && !speaker.trim().is_empty() && !listener.trim().is_empty() && !interpretation_en.trim().is_empty())]
#[invariant(::TersmuRevealed { turn_number, speaker, listener, message } => *turn_number > 0 && !speaker.trim().is_empty() && !listener.trim().is_empty() && !message.text.trim().is_empty() && !message.tersmu_rendering.is_empty())]
#[invariant(::Acknowledged { turn_number, speaker, listener, final_understanding_en, discrepancies } => *turn_number > 0 && !speaker.trim().is_empty() && !listener.trim().is_empty() && !final_understanding_en.trim().is_empty() && discrepancies.as_ref().is_none_or(|value| !value.trim().is_empty()))]
#[invariant(::ReferenceToolCompleted { participant, tool_name, arguments, result, .. } => !participant.trim().is_empty() && !tool_name.trim().is_empty() && !arguments.trim().is_empty() && !result.is_empty())]
#[invariant(::ProtocolError { participant, tool_name, message, .. } => !participant.trim().is_empty() && !tool_name.trim().is_empty() && !message.trim().is_empty())]
#[invariant(::TurnForfeited { turn_number, speaker, .. } => *turn_number > 0 && !speaker.trim().is_empty())]
#[invariant(::RunAborted { .. } => true)]
#[derive(Debug, Clone, PartialEq)]
pub enum ProtocolEvent {
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
    RunAborted {
        record: AbortRecord,
    },
}

/// Stateless generator for the five protocol schemas and phase tool sets.
#[invariant(true)]
#[derive(Debug, Clone, Copy, Default)]
pub struct ProtocolTools;

impl ProtocolTools {
    /// All five protocol definitions, generated from their typed argument models.
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|definitions| definitions.len() == 5) || ret.is_err())]
    pub fn definitions() -> Result<Vec<(ProtocolTool, ToolDefinition)>, ToolDefinitionError> {
        Ok(vec![
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
        ])
    }

    /// Exactly the legal phase tools plus all five reference tools.
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|definitions| definitions.len() >= 5) || ret.is_err())]
    pub fn definitions_for_phase(
        phase: ProtocolPhase,
    ) -> Result<Vec<ToolDefinition>, ToolDefinitionError> {
        let mut definitions = Self::definitions()?
            .into_iter()
            .filter_map(|(tool, definition)| phase.allows(tool).then_some(definition))
            .collect::<Vec<_>>();
        definitions.extend(ReferenceTools::definitions()?);
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

    /// Add private harness-visible context.
    #[requires(!content.trim().is_empty())]
    #[ensures(true)]
    fn push_user(&mut self, content: String);

    /// Request one turn using exactly the dynamically supplied definitions.
    #[requires(!tools.is_empty())]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    fn request(
        &mut self,
        tools: &[ToolDefinition],
        tool_choice: ToolChoice,
        accounting: &mut RunAccounting,
    ) -> Result<crate::ModelTurn, ProtocolModelError>;

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
    client: &'client OpenRouterClient,
}

impl<'client> OpenRouterParticipant<'client> {
    /// Build a live participant with persona, private brief, and standing rules.
    #[requires(true)]
    #[ensures(ret.conversation.participant_name() == participant.name)]
    pub fn new(participant: &ParticipantConfig, client: &'client OpenRouterClient) -> Self {
        let system_prompt = format!("{}\n\n{STANDING_PROTOCOL_RULES}", participant.system_prompt);
        Self {
            conversation: ParticipantConversation::from_parts(
                participant.name.clone(),
                participant.model.clone(),
                participant.temperature,
                system_prompt,
                participant.private_brief.clone(),
            ),
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

    fn push_user(&mut self, content: String) {
        self.conversation.push_user(content);
    }

    fn request(
        &mut self,
        tools: &[ToolDefinition],
        tool_choice: ToolChoice,
        accounting: &mut RunAccounting,
    ) -> Result<crate::ModelTurn, ProtocolModelError> {
        self.conversation
            .request(self.client, tools, tool_choice, accounting)
            .map_err(|error| ProtocolModelError::new(error.to_string()))
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
#[invariant(::BudgetAborted { .. } => true)]
#[derive(Debug, Clone, PartialEq)]
pub enum ProtocolRunOutcome {
    Completed { turns: usize },
    BudgetAborted { turns: usize, record: AbortRecord },
}

/// Protocol orchestration failures; cap hits are events, not errors.
#[invariant(::InvalidConfiguration { message } => !message.trim().is_empty())]
#[invariant(::ToolDefinitions { message } => !message.trim().is_empty())]
#[invariant(::Model { participant, message } => !participant.trim().is_empty() && !message.trim().is_empty())]
#[invariant(::Gate { participant, message } => !participant.trim().is_empty() && !message.trim().is_empty())]
#[invariant(::InvalidTersmuEncoding { message } => !message.trim().is_empty())]
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
            bityzba::data!(ProtocolRunError::AlreadyRun) => {
                formatter.write_str("a protocol runner can only be run once")
            }
        }
    }
}

impl std::error::Error for ProtocolRunError {}

/// Sequential round-robin protocol runner.
#[invariant(true, "validated on construction and mutated only through run")]
#[derive(Debug)]
pub struct ProtocolRunner<M, D> {
    participants: Vec<M>,
    reference_dispatcher: D,
    caps: CapsConfig,
    tersmu_format: TersmuFormat,
    accounting: RunAccounting,
    visible_chat: Vec<VisibleMessage>,
    events: Vec<ProtocolEvent>,
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
        tersmu_format: TersmuFormat,
        reference_dispatcher: D,
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
        let accounting = RunAccounting::new(caps.max_cost_usd).map_err(|error| {
            new!(ProtocolRunError::InvalidConfiguration {
                message: error.to_string(),
            })
        })?;
        Ok(Self {
            participants,
            reference_dispatcher,
            caps,
            tersmu_format,
            accounting,
            visible_chat: Vec::new(),
            events: Vec::new(),
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
    #[ensures(ret.len() == self.events.len())]
    pub fn events(&self) -> &[ProtocolEvent] {
        &self.events
    }

    /// Run the configured number of round-robin speaker turns.
    #[requires(true)]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    pub fn run(&mut self) -> Result<ProtocolRunOutcome, ProtocolRunError> {
        if self.has_run {
            return Err(new!(ProtocolRunError::AlreadyRun));
        }
        self.has_run = true;
        for turn_number in 1..=self.caps.max_turns {
            self.turns_started = turn_number;
            let speaker_index = (turn_number - 1) % self.participants.len();
            let speaker = self.participants[speaker_index]
                .participant_name()
                .to_owned();
            self.events.push(new!(ProtocolEvent::TurnStarted {
                turn_number,
                speaker: speaker.clone(),
            }));
            let speaker_outcome = self.run_speaker(turn_number, speaker_index)?;
            match speaker_outcome.as_data() {
                bityzba::data!(SpeakerOutcome::Posted { message }) => {
                    self.visible_chat.push(message.clone());
                    for listener_index in 0..self.participants.len() {
                        if listener_index != speaker_index {
                            if let Some(record) =
                                self.run_listener(turn_number, &speaker, listener_index, message)?
                            {
                                return Ok(new!(ProtocolRunOutcome::BudgetAborted {
                                    turns: turn_number,
                                    record,
                                }));
                            }
                        }
                    }
                }
                bityzba::data!(SpeakerOutcome::Forfeited) => {}
                bityzba::data!(SpeakerOutcome::BudgetAborted { record }) => {
                    return Ok(new!(ProtocolRunOutcome::BudgetAborted {
                        turns: turn_number,
                        record: record.clone(),
                    }));
                }
            }
        }
        Ok(new!(ProtocolRunOutcome::Completed {
            turns: self.turns_started,
        }))
    }

    #[requires(turn_number > 0)]
    #[requires(speaker_index < self.participants.len())]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    fn run_speaker(
        &mut self,
        turn_number: usize,
        speaker_index: usize,
    ) -> Result<SpeakerOutcome, ProtocolRunError> {
        self.participants[speaker_index].push_user(SPEAKER_TURN_INSTRUCTION.to_owned());
        let speaker = self.participants[speaker_index]
            .participant_name()
            .to_owned();
        let mut state = SpeakerState::awaiting_intent();
        loop {
            let phase = ProtocolPhase::Speaker {
                phase: state.phase(),
            };
            let tools = ProtocolTools::definitions_for_phase(phase).map_err(|error| {
                new!(ProtocolRunError::ToolDefinitions {
                    message: error.to_string(),
                })
            })?;
            let turn = self.participants[speaker_index]
                .request(&tools, ToolChoice::Required, &mut self.accounting)
                .map_err(|error| {
                    new!(ProtocolRunError::Model {
                        participant: speaker.clone(),
                        message: error.to_string(),
                    })
                })?;
            if let Some(calls) = turn.tool_calls() {
                let calls = calls.to_vec();
                let mut outcome = None;
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
                        new!(SpeakerAction::Continue {
                            content: dispatch_reference(
                                &mut self.reference_dispatcher,
                                &mut self.events,
                                &speaker,
                                ProtocolPhase::Speaker {
                                    phase: state.phase(),
                                },
                                call,
                            ),
                        })
                    } else {
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
                    }
                }
                if let Some(outcome) = outcome {
                    return Ok(outcome);
                }
            } else if turn.content().is_some() {
                let correction = record_protocol_error(
                    &mut self.events,
                    &speaker,
                    phase,
                    "(no tool call)",
                    "A protocol tool call is required in this state.".to_owned(),
                );
                self.participants[speaker_index].push_user(correction);
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
    #[requires(listener_index < self.participants.len())]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    fn run_listener(
        &mut self,
        turn_number: usize,
        speaker: &str,
        listener_index: usize,
        message: &VisibleMessage,
    ) -> Result<Option<AbortRecord>, ProtocolRunError> {
        let listener = self.participants[listener_index]
            .participant_name()
            .to_owned();
        let blind = message.blind();
        self.participants[listener_index].push_user(blind.prompt());
        let mut state = ListenerState::blind(blind);
        loop {
            let phase = ProtocolPhase::Listener {
                phase: state.phase(),
            };
            let tools = ProtocolTools::definitions_for_phase(phase).map_err(|error| {
                new!(ProtocolRunError::ToolDefinitions {
                    message: error.to_string(),
                })
            })?;
            let turn = self.participants[listener_index]
                .request(&tools, ToolChoice::Required, &mut self.accounting)
                .map_err(|error| {
                    new!(ProtocolRunError::Model {
                        participant: listener.clone(),
                        message: error.to_string(),
                    })
                })?;
            if let Some(calls) = turn.tool_calls() {
                let calls = calls.to_vec();
                let mut acknowledged = false;
                for call in &calls {
                    let action = if acknowledged {
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
                        new!(ListenerAction::Continue {
                            content: dispatch_reference(
                                &mut self.reference_dispatcher,
                                &mut self.events,
                                &listener,
                                ProtocolPhase::Listener {
                                    phase: state.phase(),
                                },
                                call,
                            ),
                        })
                    } else {
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
                    }
                }
                if acknowledged {
                    return Ok(None);
                }
            } else if turn.content().is_some() {
                let correction = record_protocol_error(
                    &mut self.events,
                    &listener,
                    phase,
                    "(no tool call)",
                    "A protocol tool call is required in this state.".to_owned(),
                );
                self.participants[listener_index].push_user(correction);
            } else if let Some(record) = turn.abort_record() {
                let record = record.clone();
                self.events.push(new!(ProtocolEvent::RunAborted {
                    record: record.clone(),
                }));
                return Ok(Some(record));
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
        if let Err(content) = validate_protocol_tool(&mut self.events, speaker, phase, tool) {
            return Ok(new!(SpeakerAction::Continue { content }));
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
                    content: "Intent registered. Compose Lojban and call submit_lojban.".to_owned(),
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
            ProtocolTool::InterpretBlind | ProtocolTool::Acknowledge => {
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
        if let Err(content) = validate_protocol_tool(&mut self.events, listener, phase, tool) {
            return Ok(new!(ListenerAction::Continue { content }));
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
                    prompt: revealed.prompt()?,
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
            | ProtocolTool::ConfirmMeaning => {
                unreachable!("speaker tools cannot pass listener-state validation")
            }
        }
    }
}

#[invariant(::Posted { message } => !message.text.trim().is_empty() && !message.tersmu_rendering.is_empty())]
#[invariant(::Forfeited => true)]
#[invariant(::BudgetAborted { .. } => true)]
enum SpeakerOutcome {
    Posted { message: VisibleMessage },
    Forfeited,
    BudgetAborted { record: AbortRecord },
}

#[invariant(::Continue { content } => !content.is_empty())]
#[invariant(::Posted { content, message } => !content.is_empty() && !message.text.trim().is_empty() && !message.tersmu_rendering.is_empty())]
#[invariant(::Forfeited { content } => !content.is_empty())]
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
}

impl SpeakerAction {
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn content(&self) -> &str {
        match self.as_data() {
            bityzba::data!(SpeakerAction::Continue { content })
            | bityzba::data!(SpeakerAction::Posted { content, .. })
            | bityzba::data!(SpeakerAction::Forfeited { content }) => content,
        }
    }
}

#[invariant(::Continue { content } => !content.is_empty())]
#[invariant(::Reveal { content, prompt } => !content.is_empty() && !prompt.trim().is_empty())]
#[invariant(::Acknowledged { content } => !content.is_empty())]
enum ListenerAction {
    Continue { content: String },
    Reveal { content: String, prompt: String },
    Acknowledged { content: String },
}

impl ListenerAction {
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn content(&self) -> &str {
        match self.as_data() {
            bityzba::data!(ListenerAction::Continue { content })
            | bityzba::data!(ListenerAction::Reveal { content, .. })
            | bityzba::data!(ListenerAction::Acknowledged { content }) => content,
        }
    }
}

#[requires(!participant.trim().is_empty())]
#[ensures(ret.is_ok() == phase.allows(tool))]
fn validate_protocol_tool(
    events: &mut Vec<ProtocolEvent>,
    participant: &str,
    phase: ProtocolPhase,
    tool: ProtocolTool,
) -> Result<(), String> {
    if phase.allows(tool) {
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
    events: &mut Vec<ProtocolEvent>,
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

#[requires(!participant.trim().is_empty())]
#[requires(is_reference_tool(&call.function.name))]
#[ensures(!ret.is_empty())]
fn dispatch_reference(
    dispatcher: &mut impl ToolDispatcher,
    events: &mut Vec<ProtocolEvent>,
    participant: &str,
    phase: ProtocolPhase,
    call: &ToolCall,
) -> String {
    let (result, succeeded) = match dispatcher.dispatch(call) {
        Ok(result) if !result.is_empty() => (result, true),
        Ok(_) => (
            format!(
                "Reference tool `{}` returned an empty result.",
                call.function.name
            ),
            false,
        ),
        Err(error) => (error.to_string(), false),
    };
    events.push(new!(ProtocolEvent::ReferenceToolCompleted {
        participant: participant.to_owned(),
        phase,
        tool_name: call.function.name.clone(),
        arguments: call.function.arguments.clone(),
        result: result.clone(),
        succeeded,
    }));
    result
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
    fn state_tool_matrix_rejects_and_records_every_illegal_pair() {
        let mut examined = 0usize;
        let mut rejected = 0usize;
        for phase in ProtocolPhase::ALL {
            let definitions = ProtocolTools::definitions_for_phase(phase)
                .expect("phase definitions must be valid");
            let names = definitions
                .iter()
                .map(ToolDefinition::name)
                .collect::<BTreeSet<_>>();
            for reference in REFERENCE_TOOL_NAMES {
                assert!(
                    names.contains(reference),
                    "{reference} missing in {phase:?}"
                );
            }
            for tool in ProtocolTool::ALL {
                examined += 1;
                assert_eq!(names.contains(tool.name()), phase.allows(tool));
                let mut events = Vec::new();
                let result = validate_protocol_tool(&mut events, "tester", phase, tool);
                if phase.allows(tool) {
                    assert!(result.is_ok(), "legal {phase:?} x {tool:?}");
                    assert!(events.is_empty());
                } else {
                    rejected += 1;
                    assert!(result.is_err(), "illegal {phase:?} x {tool:?}");
                    assert!(matches!(
                        events.as_slice(),
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
        assert_eq!(examined, 35);
        assert_eq!(rejected, 29);
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
        let definitions = ProtocolTools::definitions().expect("valid definitions");
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
            }
        }
    }
}
