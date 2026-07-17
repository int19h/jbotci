//! Typed scenario instances and mechanical ground-truth checkers.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

#[allow(unused_imports)]
use bityzba::{ensures, invariant, new, requires};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// The three mechanically checked scenario families supported by xarsnu.
#[invariant(::ScheduleNegotiation => true)]
#[invariant(::DistributedClueDeduction => true)]
#[invariant(::ReferentialGame => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ScenarioKind {
    ScheduleNegotiation,
    DistributedClueDeduction,
    ReferentialGame,
}

/// A weekday in a recurring weekly schedule.
#[invariant(::Monday => true)]
#[invariant(::Tuesday => true)]
#[invariant(::Wednesday => true)]
#[invariant(::Thursday => true)]
#[invariant(::Friday => true)]
#[invariant(::Saturday => true)]
#[invariant(::Sunday => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Weekday {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}

impl fmt::Display for Weekday {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Monday => "Monday",
            Self::Tuesday => "Tuesday",
            Self::Wednesday => "Wednesday",
            Self::Thursday => "Thursday",
            Self::Friday => "Friday",
            Self::Saturday => "Saturday",
            Self::Sunday => "Sunday",
        })
    }
}

/// Typed answer for a schedule-negotiation scenario.
#[invariant(*start_minute < 24 * 60, "start-minute must fall within a day")]
#[invariant(*duration_minutes > 0 && *duration_minutes <= 24 * 60, "duration-minutes must be between one minute and one day")]
#[invariant(*start_minute + *duration_minutes <= 24 * 60, "the meeting must end within the same day")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct ScheduleAnswer {
    pub day: Weekday,
    pub start_minute: u16,
    pub duration_minutes: u16,
}

impl ScheduleAnswer {
    /// Construct a checked meeting slot.
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|answer| answer.start_minute == start_minute) || ret.is_err())]
    pub fn new(
        day: Weekday,
        start_minute: u16,
        duration_minutes: u16,
    ) -> Result<Self, ScenarioAnswerError> {
        Self::try_from_data(bityzba::data!(ScheduleAnswer {
            day,
            start_minute,
            duration_minutes,
        }))
        .map_err(|error| {
            new!(ScenarioAnswerError {
                message: error.to_string(),
            })
        })
    }
}

/// One row in a complete person/profession/city assignment.
#[invariant(!person.trim().is_empty(), "person cannot be empty")]
#[invariant(!profession.trim().is_empty(), "profession cannot be empty")]
#[invariant(!city.trim().is_empty(), "city cannot be empty")]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct Assignment {
    pub person: String,
    pub profession: String,
    pub city: String,
}

impl Assignment {
    /// Construct one checked assignment row.
    #[requires(true)]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    pub fn new(
        person: String,
        profession: String,
        city: String,
    ) -> Result<Self, ScenarioAnswerError> {
        Self::try_from_data(bityzba::data!(Assignment {
            person,
            profession,
            city,
        }))
        .map_err(|error| {
            new!(ScenarioAnswerError {
                message: error.to_string(),
            })
        })
    }
}

/// Typed answer for a distributed-clue deduction scenario.
#[invariant(!assignments.is_empty(), "assignments cannot be empty")]
#[invariant(assignments.iter().enumerate().all(|(index, assignment)| assignments[..index].iter().all(|earlier| earlier.person != assignment.person && earlier.profession != assignment.profession && earlier.city != assignment.city)), "people, professions, and cities must each be unique")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct DeductionAnswer {
    pub assignments: Vec<Assignment>,
}

impl DeductionAnswer {
    /// Construct a checked complete assignment.
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|answer| !answer.assignments.is_empty()) || ret.is_err())]
    pub fn new(assignments: Vec<Assignment>) -> Result<Self, ScenarioAnswerError> {
        let count = assignments.len();
        Self::try_from_data(bityzba::data!(DeductionAnswer { assignments }))
            .map_err(|error| {
                new!(ScenarioAnswerError {
                    message: error.to_string(),
                })
            })
            .inspect(|answer| debug_assert_eq!(answer.assignments.len(), count))
    }
}

/// Typed answer for a referential-game listener.
#[invariant(*scene_index > 0, "scene-index is one-based")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct ReferentialAnswer {
    pub scene_index: usize,
}

impl ReferentialAnswer {
    /// Construct a checked one-based scene selection.
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|answer| answer.scene_index == scene_index) || ret.is_err())]
    pub fn new(scene_index: usize) -> Result<Self, ScenarioAnswerError> {
        Self::try_from_data(bityzba::data!(ReferentialAnswer { scene_index })).map_err(|error| {
            new!(ScenarioAnswerError {
                message: error.to_string(),
            })
        })
    }
}

/// A scenario-specific answer after validation at the model/tool boundary.
#[invariant(::Schedule { .. } => true)]
#[invariant(::Deduction { .. } => true)]
#[invariant(::Referential { .. } => true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "scenario-kind", content = "answer", rename_all = "kebab-case")]
pub enum ScenarioAnswer {
    Schedule { answer: ScheduleAnswer },
    Deduction { answer: DeductionAnswer },
    Referential { answer: ReferentialAnswer },
}

impl ScenarioAnswer {
    /// Wrap a schedule answer without losing its checked type.
    #[requires(true)]
    #[ensures(matches!(ret, ScenarioAnswer::Schedule { .. }))]
    pub fn schedule(answer: ScheduleAnswer) -> Self {
        Self::Schedule { answer }
    }

    /// Wrap a deduction answer without losing its checked type.
    #[requires(true)]
    #[ensures(matches!(ret, ScenarioAnswer::Deduction { .. }))]
    pub fn deduction(answer: DeductionAnswer) -> Self {
        Self::Deduction { answer }
    }

    /// Wrap a referential answer without losing its checked type.
    #[requires(true)]
    #[ensures(matches!(ret, ScenarioAnswer::Referential { .. }))]
    pub fn referential(answer: ReferentialAnswer) -> Self {
        Self::Referential { answer }
    }
}

/// Rejection of malformed model-supplied answer data.
#[invariant(!message.trim().is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenarioAnswerError {
    message: String,
}

impl fmt::Display for ScenarioAnswerError {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid scenario answer: {}", self.message)
    }
}

impl std::error::Error for ScenarioAnswerError {}

/// Aggregate checker result.
#[invariant(::Success => true)]
#[invariant(::Partial => true)]
#[invariant(::Failure => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TaskStatus {
    Success,
    Partial,
    Failure,
}

/// Mechanical result for one participant.
#[invariant(!participant.trim().is_empty())]
#[invariant(*required || answer.is_none(), "non-answering roles cannot carry answers")]
#[invariant(*required || correct.is_none(), "non-answering roles have no correctness verdict")]
#[invariant(answer.is_some() == *submitted, "submission flag must match answer presence")]
#[invariant(correct.is_none() == !required, "required roles always receive a verdict")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ParticipantTaskOutcome {
    pub participant: String,
    pub required: bool,
    pub submitted: bool,
    pub correct: Option<bool>,
    pub answer: Option<ScenarioAnswer>,
}

/// Complete mechanically computed task outcome.
#[invariant(!participants.is_empty())]
#[invariant(participants.iter().enumerate().all(|(index, participant)| participants[..index].iter().all(|earlier| earlier.participant != participant.participant)), "participant outcomes must be unique")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct TaskOutcome {
    pub status: TaskStatus,
    pub participants: Vec<ParticipantTaskOutcome>,
}

/// Participant-visible scenario context. Ground truth is intentionally absent.
#[invariant(!name.trim().is_empty())]
#[invariant(!private_brief.trim().is_empty())]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ScenarioParticipant {
    pub name: String,
    pub private_brief: String,
    pub answer_required: bool,
}

/// A validated scenario instance, including hidden checker data.
#[invariant(!document.id.trim().is_empty())]
#[invariant(!document.title.trim().is_empty())]
#[invariant(!document.public_setup.trim().is_empty())]
#[invariant(document.answer_schema.is_object())]
#[derive(Debug, Clone, PartialEq)]
pub struct ScenarioInstance {
    document: ScenarioDocument,
    participants: Vec<ScenarioParticipant>,
}

impl ScenarioInstance {
    /// Parse and semantically validate a scenario TOML document.
    #[requires(true)]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    pub fn from_toml(source: &str) -> Result<Self, ScenarioConfigError> {
        let document: ScenarioDocument =
            toml::from_str(source).map_err(|error| new!(ScenarioConfigError::Toml(error)))?;
        validate_document(&document)?;
        let participants = participant_views(&document)?;
        Ok(new!(ScenarioInstance {
            document,
            participants,
        }))
    }

    /// Serialize a validated scenario back to TOML.
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|source| !source.trim().is_empty()) || ret.is_err())]
    pub fn to_toml(&self) -> Result<String, ScenarioConfigError> {
        toml::to_string_pretty(&self.document)
            .map_err(|error| new!(ScenarioConfigError::Serialize(error)))
    }

    /// Stable scenario instance id.
    #[requires(true)]
    #[ensures(!ret.trim().is_empty())]
    pub fn id(&self) -> &str {
        &self.document.id
    }

    /// Human-readable title.
    #[requires(true)]
    #[ensures(!ret.trim().is_empty())]
    pub fn title(&self) -> &str {
        &self.document.title
    }

    /// Scenario family.
    #[requires(true)]
    #[ensures(true)]
    pub fn kind(&self) -> ScenarioKind {
        self.document.data.kind()
    }

    /// English public setup, augmented with public structured data where needed.
    #[requires(true)]
    #[ensures(!ret.trim().is_empty())]
    pub fn public_setup(&self) -> String {
        match &self.document.data {
            ScenarioDataDocument::ReferentialGame { scenes, .. } => {
                let mut setup = self.document.public_setup.clone();
                setup.push_str("\n\nPublic scenes:");
                for (index, scene) in scenes.iter().enumerate() {
                    setup.push_str(&format!("\n{}. {}", index + 1, scene.description));
                }
                setup
            }
            _ => self.document.public_setup.clone(),
        }
    }

    /// Participant-visible private briefs; hidden checker truth is not exposed here.
    #[requires(true)]
    #[ensures(ret.len() >= 2)]
    pub fn participants(&self) -> &[ScenarioParticipant] {
        &self.participants
    }

    /// Complete English context for one participant.
    #[requires(!participant.trim().is_empty())]
    #[ensures(ret.as_ref().is_some_and(|context| !context.trim().is_empty()) || ret.is_none())]
    pub fn prompt_for(&self, participant: &str) -> Option<String> {
        self.participants
            .iter()
            .find(|entry| entry.name == participant)
            .map(|entry| {
                format!(
                    "Scenario: {}\n\nPublic setup:\n{}\n\nYour private brief:\n{}",
                    self.title(),
                    self.public_setup(),
                    entry.private_brief
                )
            })
    }

    /// JSON Schema used verbatim for the dynamic `submit_answer` tool.
    #[requires(true)]
    #[ensures(ret.is_object())]
    pub fn answer_schema(&self) -> &Value {
        &self.document.answer_schema
    }

    /// Completed discussion rounds required before `submit_answer` is offered.
    #[requires(true)]
    #[ensures(ret > 0)]
    pub fn minimum_rounds(&self) -> usize {
        self.document.minimum_rounds
    }

    /// Maximum scenario rounds.
    #[requires(true)]
    #[ensures(ret >= self.minimum_rounds())]
    pub fn maximum_rounds(&self) -> usize {
        self.document.maximum_rounds
    }

    /// Maximum speaker turns, independent of the run-level safety cap.
    #[requires(true)]
    #[ensures(ret > 0)]
    pub fn maximum_turns(&self) -> usize {
        self.document.maximum_turns
    }

    /// Whether this participant must submit an answer.
    #[requires(!participant.trim().is_empty())]
    #[ensures(true)]
    pub fn answer_required(&self, participant: &str) -> bool {
        self.participants
            .iter()
            .any(|entry| entry.name == participant && entry.answer_required)
    }

    /// Parse model-facing JSON into this scenario's typed answer.
    #[requires(value.is_object())]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    pub fn parse_answer(&self, value: Value) -> Result<ScenarioAnswer, ScenarioAnswerError> {
        let malformed = |error: serde_json::Error| {
            new!(ScenarioAnswerError {
                message: error.to_string(),
            })
        };
        match self.kind() {
            ScenarioKind::ScheduleNegotiation => serde_json::from_value::<ScheduleAnswer>(value)
                .map(ScenarioAnswer::schedule)
                .map_err(malformed),
            ScenarioKind::DistributedClueDeduction => {
                serde_json::from_value::<DeductionAnswer>(value)
                    .map(ScenarioAnswer::deduction)
                    .map_err(malformed)
            }
            ScenarioKind::ReferentialGame => serde_json::from_value::<ReferentialAnswer>(value)
                .map(ScenarioAnswer::referential)
                .map_err(malformed),
        }
    }

    /// Pure mechanical checker over already typed participant answers.
    #[requires(true)]
    #[ensures(ret.participants.len() == self.participants.len())]
    pub fn check_answers(&self, answers: &BTreeMap<String, ScenarioAnswer>) -> TaskOutcome {
        let participants = self
            .participants
            .iter()
            .map(|participant| {
                let answer = answers.get(&participant.name).cloned();
                let correct = participant.answer_required.then(|| {
                    answer
                        .as_ref()
                        .is_some_and(|answer| self.answer_is_correct(answer))
                });
                new!(ParticipantTaskOutcome {
                    participant: participant.name.clone(),
                    required: participant.answer_required,
                    submitted: answer.is_some(),
                    correct,
                    answer,
                })
            })
            .collect::<Vec<_>>();
        let required = participants
            .iter()
            .filter(|participant| participant.required)
            .collect::<Vec<_>>();
        let correct = required
            .iter()
            .filter(|participant| participant.correct == Some(true))
            .count();
        let status = if correct == required.len() {
            TaskStatus::Success
        } else if correct > 0 {
            TaskStatus::Partial
        } else {
            TaskStatus::Failure
        };
        new!(TaskOutcome {
            status,
            participants,
        })
    }

    /// Whether every answer-required participant has submitted.
    #[requires(true)]
    #[ensures(true)]
    pub fn all_required_submitted(&self, answers: &BTreeMap<String, ScenarioAnswer>) -> bool {
        self.participants
            .iter()
            .filter(|participant| participant.answer_required)
            .all(|participant| answers.contains_key(&participant.name))
    }

    #[requires(true)]
    #[ensures(true)]
    fn answer_is_correct(&self, answer: &ScenarioAnswer) -> bool {
        match (&self.document.data, answer) {
            (
                ScenarioDataDocument::ScheduleNegotiation {
                    meeting_duration_minutes,
                    participants,
                    ..
                },
                ScenarioAnswer::Schedule { answer },
            ) => {
                answer.duration_minutes == *meeting_duration_minutes
                    && participants.iter().all(|participant| {
                        participant.availability.iter().any(|window| {
                            window.day == answer.day
                                && answer.start_minute >= window.start_minute
                                && answer.start_minute + answer.duration_minutes
                                    <= window.end_minute
                        })
                    })
            }
            (
                ScenarioDataDocument::DistributedClueDeduction { solution, .. },
                ScenarioAnswer::Deduction { answer },
            ) => assignments_equal(&answer.assignments, solution),
            (
                ScenarioDataDocument::ReferentialGame {
                    target_scene_index, ..
                },
                ScenarioAnswer::Referential { answer },
            ) => answer.scene_index == *target_scene_index,
            _ => false,
        }
    }
}

/// A scenario document failed syntactic or semantic validation.
#[invariant(::Toml(_) => true)]
#[invariant(::Serialize(_) => true)]
#[invariant(::InvalidField { field, message } => !field.trim().is_empty() && !message.trim().is_empty())]
#[derive(Debug)]
pub enum ScenarioConfigError {
    Toml(toml::de::Error),
    Serialize(toml::ser::Error),
    InvalidField { field: String, message: String },
}

impl ScenarioConfigError {
    /// Semantic field name for validation failures; syntax/serialization failures have none.
    #[requires(true)]
    #[ensures(ret.is_none_or(|field| !field.trim().is_empty()))]
    pub fn field(&self) -> Option<&str> {
        match self.as_data() {
            bityzba::data!(ScenarioConfigError::InvalidField { field, .. }) => Some(field),
            bityzba::data!(ScenarioConfigError::Toml(_))
            | bityzba::data!(ScenarioConfigError::Serialize(_)) => None,
        }
    }
}

impl fmt::Display for ScenarioConfigError {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.as_data() {
            bityzba::data!(ScenarioConfigError::Toml(error)) => {
                write!(formatter, "invalid scenario TOML: {error}")
            }
            bityzba::data!(ScenarioConfigError::Serialize(error)) => {
                write!(formatter, "unable to serialize scenario TOML: {error}")
            }
            bityzba::data!(ScenarioConfigError::InvalidField { field, message }) => {
                write!(formatter, "invalid scenario field `{field}`: {message}")
            }
        }
    }
}

impl std::error::Error for ScenarioConfigError {}

#[invariant(true, "unvalidated TOML wire model")]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct ScenarioDocument {
    id: String,
    title: String,
    public_setup: String,
    answer_schema: Value,
    minimum_rounds: usize,
    maximum_rounds: usize,
    maximum_turns: usize,
    #[serde(flatten)]
    data: ScenarioDataDocument,
}

#[invariant(::ScheduleNegotiation => true, "unvalidated TOML wire variant")]
#[invariant(::DistributedClueDeduction => true, "unvalidated TOML wire variant")]
#[invariant(::ReferentialGame => true, "unvalidated TOML wire variant")]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "scenario-type",
    rename_all = "kebab-case",
    rename_all_fields = "kebab-case"
)]
enum ScenarioDataDocument {
    ScheduleNegotiation {
        meeting_duration_minutes: u16,
        slot_granularity_minutes: u16,
        participants: Vec<ScheduleParticipantDocument>,
    },
    DistributedClueDeduction {
        people: Vec<String>,
        professions: Vec<String>,
        cities: Vec<String>,
        participants: Vec<DeductionParticipantDocument>,
        solution: Vec<AssignmentDocument>,
    },
    ReferentialGame {
        scenes: Vec<SceneDocument>,
        target_scene_index: usize,
        participants: Vec<ReferentialParticipantDocument>,
    },
}

impl ScenarioDataDocument {
    #[requires(true)]
    #[ensures(true)]
    fn kind(&self) -> ScenarioKind {
        match self {
            Self::ScheduleNegotiation { .. } => ScenarioKind::ScheduleNegotiation,
            Self::DistributedClueDeduction { .. } => ScenarioKind::DistributedClueDeduction,
            Self::ReferentialGame { .. } => ScenarioKind::ReferentialGame,
        }
    }
}

#[invariant(true, "unvalidated TOML wire model")]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
struct ScheduleParticipantDocument {
    name: String,
    availability: Vec<TimeWindowDocument>,
}

#[invariant(true, "unvalidated TOML wire model")]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
struct TimeWindowDocument {
    day: Weekday,
    start_minute: u16,
    end_minute: u16,
}

#[invariant(true, "unvalidated TOML wire model")]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
struct DeductionParticipantDocument {
    name: String,
    clues: Vec<PuzzleClueDocument>,
}

#[invariant(true, "unvalidated TOML wire model")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
struct AssignmentDocument {
    person: String,
    profession: String,
    city: String,
}

#[invariant(::Profession => true, "unvalidated TOML wire variant")]
#[invariant(::City => true, "unvalidated TOML wire variant")]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "kebab-case"
)]
enum PuzzleClueDocument {
    Profession {
        person: String,
        profession: String,
        matches: bool,
    },
    City {
        person: String,
        city: String,
        matches: bool,
    },
}

#[invariant(true, "unvalidated TOML wire model")]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
struct SceneDocument {
    description: String,
}

#[invariant(true, "unvalidated TOML wire model")]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
struct ReferentialParticipantDocument {
    name: String,
    role: ReferentialRole,
}

#[invariant(::Speaker => true)]
#[invariant(::Listener => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum ReferentialRole {
    Speaker,
    Listener,
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn validate_document(document: &ScenarioDocument) -> Result<(), ScenarioConfigError> {
    require_nonempty("id", &document.id)?;
    require_nonempty("title", &document.title)?;
    require_nonempty("public-setup", &document.public_setup)?;
    if !document.answer_schema.is_object() {
        return Err(invalid("answer-schema", "must be a JSON Schema object"));
    }
    if document.minimum_rounds == 0 {
        return Err(invalid("minimum-rounds", "must be positive"));
    }
    if document.maximum_rounds < document.minimum_rounds {
        return Err(invalid("maximum-rounds", "must be at least minimum-rounds"));
    }
    if document.maximum_turns == 0 {
        return Err(invalid("maximum-turns", "must be positive"));
    }
    let expected_schema = answer_schema(document.data.kind());
    if document.answer_schema != expected_schema {
        return Err(invalid(
            "answer-schema",
            "does not match the selected scenario-type",
        ));
    }
    match &document.data {
        ScenarioDataDocument::ScheduleNegotiation {
            meeting_duration_minutes,
            slot_granularity_minutes,
            participants,
        } => validate_schedule(
            *meeting_duration_minutes,
            *slot_granularity_minutes,
            participants,
        )?,
        ScenarioDataDocument::DistributedClueDeduction {
            people,
            professions,
            cities,
            participants,
            solution,
        } => validate_deduction(people, professions, cities, participants, solution)?,
        ScenarioDataDocument::ReferentialGame {
            scenes,
            target_scene_index,
            participants,
        } => validate_referential(scenes, *target_scene_index, participants)?,
    }
    let participant_count = match &document.data {
        ScenarioDataDocument::ScheduleNegotiation { participants, .. } => participants.len(),
        ScenarioDataDocument::DistributedClueDeduction { participants, .. } => participants.len(),
        ScenarioDataDocument::ReferentialGame { participants, .. } => participants.len(),
    };
    if document.maximum_turns > document.maximum_rounds * participant_count {
        return Err(invalid(
            "maximum-turns",
            "cannot exceed maximum-rounds times participant count",
        ));
    }
    if document.maximum_turns <= document.minimum_rounds * participant_count {
        return Err(invalid(
            "maximum-turns",
            "must leave at least one turn after minimum-rounds for answer submission",
        ));
    }
    Ok(())
}

#[requires(!field.trim().is_empty())]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn require_nonempty(field: &str, value: &str) -> Result<(), ScenarioConfigError> {
    if value.trim().is_empty() {
        Err(invalid(field, "cannot be empty"))
    } else {
        Ok(())
    }
}

#[requires(!field.trim().is_empty())]
#[requires(!message.trim().is_empty())]
#[ensures(!ret.to_string().is_empty())]
fn invalid(field: &str, message: &str) -> ScenarioConfigError {
    new!(ScenarioConfigError::InvalidField {
        field: field.to_owned(),
        message: message.to_owned(),
    })
}

#[requires(meeting_duration_minutes > 0)]
#[requires(slot_granularity_minutes > 0)]
#[ensures(true)]
fn schedule_slots(
    meeting_duration_minutes: u16,
    slot_granularity_minutes: u16,
    participants: &[ScheduleParticipantDocument],
) -> Vec<ScheduleAnswer> {
    let mut slots = BTreeSet::new();
    for participant in participants {
        for window in &participant.availability {
            let mut start = window.start_minute;
            while start + meeting_duration_minutes <= window.end_minute {
                if participants.iter().all(|candidate| {
                    candidate.availability.iter().any(|candidate_window| {
                        candidate_window.day == window.day
                            && start >= candidate_window.start_minute
                            && start + meeting_duration_minutes <= candidate_window.end_minute
                    })
                }) {
                    slots.insert((window.day, start));
                }
                let Some(next) = start.checked_add(slot_granularity_minutes) else {
                    break;
                };
                start = next;
            }
        }
    }
    slots
        .into_iter()
        .map(|(day, start_minute)| {
            ScheduleAnswer::new(day, start_minute, meeting_duration_minutes)
                .expect("validated schedule windows produce valid slots")
        })
        .collect()
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn validate_schedule(
    meeting_duration_minutes: u16,
    slot_granularity_minutes: u16,
    participants: &[ScheduleParticipantDocument],
) -> Result<(), ScenarioConfigError> {
    if meeting_duration_minutes == 0 {
        return Err(invalid("meeting-duration-minutes", "must be positive"));
    }
    if slot_granularity_minutes == 0 {
        return Err(invalid("slot-granularity-minutes", "must be positive"));
    }
    validate_participant_names(
        participants
            .iter()
            .map(|participant| participant.name.as_str()),
    )?;
    for participant in participants {
        if participant.availability.is_empty() {
            return Err(invalid("participants.availability", "cannot be empty"));
        }
        for window in &participant.availability {
            if window.start_minute >= window.end_minute || window.end_minute > 24 * 60 {
                return Err(invalid(
                    "participants.availability",
                    "windows must be ordered and end within one day",
                ));
            }
        }
        let individual = schedule_slots(
            meeting_duration_minutes,
            slot_granularity_minutes,
            std::slice::from_ref(participant),
        );
        if individual.len() < 2 {
            return Err(invalid(
                "participants.availability",
                "each private brief must leave more than one possible answer",
            ));
        }
    }
    let joint = schedule_slots(
        meeting_duration_minutes,
        slot_granularity_minutes,
        participants,
    );
    if joint.len() != 1 {
        return Err(invalid(
            "participants.availability",
            "combined private constraints must determine exactly one answer",
        ));
    }
    Ok(())
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn validate_deduction(
    people: &[String],
    professions: &[String],
    cities: &[String],
    participants: &[DeductionParticipantDocument],
    solution: &[AssignmentDocument],
) -> Result<(), ScenarioConfigError> {
    if people.len() < 3 || professions.len() != people.len() || cities.len() != people.len() {
        return Err(invalid(
            "people/professions/cities",
            "must contain equally sized sets of at least three unique values",
        ));
    }
    validate_unique_nonempty("people", people)?;
    validate_unique_nonempty("professions", professions)?;
    validate_unique_nonempty("cities", cities)?;
    validate_participant_names(
        participants
            .iter()
            .map(|participant| participant.name.as_str()),
    )?;
    if solution.len() != people.len() || !assignments_cover(solution, people, professions, cities) {
        return Err(invalid(
            "solution",
            "must be a complete bijection over the declared entity sets",
        ));
    }
    for participant in participants {
        if participant.clues.is_empty() {
            return Err(invalid("participants.clues", "cannot be empty"));
        }
        validate_clues(&participant.clues, people, professions, cities)?;
        if matching_assignments(people, professions, cities, &participant.clues).len() < 2 {
            return Err(invalid(
                "participants.clues",
                "no single private brief may determine the answer",
            ));
        }
    }
    let all_clues = participants
        .iter()
        .flat_map(|participant| participant.clues.iter().cloned())
        .collect::<Vec<_>>();
    let matches = matching_assignments(people, professions, cities, &all_clues);
    if matches.len() != 1 || !assignments_equal_documents(&matches[0], solution) {
        return Err(invalid(
            "participants.clues",
            "combined clues must uniquely determine the declared solution",
        ));
    }
    Ok(())
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn validate_referential(
    scenes: &[SceneDocument],
    target_scene_index: usize,
    participants: &[ReferentialParticipantDocument],
) -> Result<(), ScenarioConfigError> {
    if scenes.len() < 3 {
        return Err(invalid("scenes", "must contain at least three distractors"));
    }
    for scene in scenes {
        require_nonempty("scenes.description", &scene.description)?;
    }
    if target_scene_index == 0 || target_scene_index > scenes.len() {
        return Err(invalid(
            "target-scene-index",
            "must select an existing one-based scene index",
        ));
    }
    validate_participant_names(
        participants
            .iter()
            .map(|participant| participant.name.as_str()),
    )?;
    if participants
        .iter()
        .filter(|participant| participant.role == ReferentialRole::Speaker)
        .count()
        != 1
    {
        return Err(invalid(
            "participants.role",
            "exactly one participant must be the speaker",
        ));
    }
    if participants
        .iter()
        .filter(|participant| participant.role == ReferentialRole::Listener)
        .count()
        < 1
    {
        return Err(invalid(
            "participants.role",
            "at least one listener is required",
        ));
    }
    Ok(())
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn validate_participant_names<'a>(
    names: impl Iterator<Item = &'a str>,
) -> Result<(), ScenarioConfigError> {
    let names = names.collect::<Vec<_>>();
    if names.len() < 2 {
        return Err(invalid("participants", "at least two are required"));
    }
    if names.iter().any(|name| name.trim().is_empty()) {
        return Err(invalid("participants.name", "cannot be empty"));
    }
    if names
        .iter()
        .enumerate()
        .any(|(index, name)| names[..index].contains(name))
    {
        return Err(invalid("participants.name", "must be unique"));
    }
    Ok(())
}

#[requires(!field.trim().is_empty())]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn validate_unique_nonempty(field: &str, values: &[String]) -> Result<(), ScenarioConfigError> {
    if values.iter().any(|value| value.trim().is_empty()) {
        return Err(invalid(field, "values cannot be empty"));
    }
    if values
        .iter()
        .enumerate()
        .any(|(index, value)| values[..index].contains(value))
    {
        return Err(invalid(field, "values must be unique"));
    }
    Ok(())
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn validate_clues(
    clues: &[PuzzleClueDocument],
    people: &[String],
    professions: &[String],
    cities: &[String],
) -> Result<(), ScenarioConfigError> {
    for clue in clues {
        match clue {
            PuzzleClueDocument::Profession {
                person, profession, ..
            } if people.contains(person) && professions.contains(profession) => {}
            PuzzleClueDocument::City { person, city, .. }
                if people.contains(person) && cities.contains(city) => {}
            _ => {
                return Err(invalid(
                    "participants.clues",
                    "must reference declared people, professions, and cities",
                ));
            }
        }
    }
    Ok(())
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|participants| participants.len() >= 2) || ret.is_err())]
fn participant_views(
    document: &ScenarioDocument,
) -> Result<Vec<ScenarioParticipant>, ScenarioConfigError> {
    let participants = match &document.data {
        ScenarioDataDocument::ScheduleNegotiation {
            participants,
            ..
        } => participants
            .iter()
            .map(|participant| {
                let windows = participant
                    .availability
                    .iter()
                    .map(|window| {
                        format!(
                            "{} from {} to {}",
                            window.day,
                            format_minutes(window.start_minute),
                            format_minutes(window.end_minute)
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("; ");
                new!(ScenarioParticipant {
                    name: participant.name.clone(),
                    private_brief: format!(
                        "Your recurring weekly availability is: {windows}. Do not assume that the other participants share these constraints."
                    ),
                    answer_required: true,
                })
            })
            .collect(),
        ScenarioDataDocument::DistributedClueDeduction {
            participants,
            ..
        } => participants
            .iter()
            .map(|participant| {
                let clues = participant
                    .clues
                    .iter()
                    .map(render_clue)
                    .collect::<Vec<_>>()
                    .join(" ");
                new!(ScenarioParticipant {
                    name: participant.name.clone(),
                    private_brief: format!(
                        "Only you initially know these puzzle clues: {clues} Exchange their meaning with the other participants before answering."
                    ),
                    answer_required: true,
                })
            })
            .collect(),
        ScenarioDataDocument::ReferentialGame {
            target_scene_index,
            participants,
            ..
        } => participants
            .iter()
            .map(|participant| {
                let (private_brief, answer_required) = match participant.role {
                    ReferentialRole::Speaker => (
                        format!(
                            "You are the describing speaker. The hidden target is public scene index {target_scene_index}. Describe it without stating its index."
                        ),
                        false,
                    ),
                    ReferentialRole::Listener => (
                        "You are a listener. Infer the hidden scene from the speaker's messages and submit its one-based scene index."
                            .to_owned(),
                        true,
                    ),
                };
                new!(ScenarioParticipant {
                    name: participant.name.clone(),
                    private_brief,
                    answer_required,
                })
            })
            .collect(),
    };
    Ok(participants)
}

#[requires(minute <= 24 * 60)]
#[ensures(!ret.is_empty())]
fn format_minutes(minute: u16) -> String {
    format!("{:02}:{:02}", minute / 60, minute % 60)
}

#[requires(true)]
#[ensures(!ret.trim().is_empty())]
fn render_clue(clue: &PuzzleClueDocument) -> String {
    match clue {
        PuzzleClueDocument::Profession {
            person,
            profession,
            matches: true,
        } => format!("{person}'s profession is {profession}."),
        PuzzleClueDocument::Profession {
            person,
            profession,
            matches: false,
        } => format!("{person}'s profession is not {profession}."),
        PuzzleClueDocument::City {
            person,
            city,
            matches: true,
        } => format!("{person} lives in {city}."),
        PuzzleClueDocument::City {
            person,
            city,
            matches: false,
        } => format!("{person} does not live in {city}."),
    }
}

#[requires(true)]
#[ensures(ret.is_object())]
fn answer_schema(kind: ScenarioKind) -> Value {
    let base = |properties: Value, required: Value| {
        json!({
            "type": "object",
            "additionalProperties": false,
            "properties": properties,
            "required": required,
        })
    };
    match kind {
        ScenarioKind::ScheduleNegotiation => base(
            json!({
                "day": {
                    "type": "string",
                    "enum": ["monday", "tuesday", "wednesday", "thursday", "friday", "saturday", "sunday"]
                },
                "start_minute": { "type": "integer", "minimum": 0, "maximum": 1439 },
                "duration_minutes": { "type": "integer", "minimum": 1, "maximum": 1440 }
            }),
            json!(["day", "start_minute", "duration_minutes"]),
        ),
        ScenarioKind::DistributedClueDeduction => base(
            json!({
                "assignments": {
                    "type": "array",
                    "minItems": 1,
                    "items": {
                        "type": "object",
                        "additionalProperties": false,
                        "properties": {
                            "person": { "type": "string", "minLength": 1 },
                            "profession": { "type": "string", "minLength": 1 },
                            "city": { "type": "string", "minLength": 1 }
                        },
                        "required": ["person", "profession", "city"]
                    }
                }
            }),
            json!(["assignments"]),
        ),
        ScenarioKind::ReferentialGame => base(
            json!({ "scene_index": { "type": "integer", "minimum": 1 } }),
            json!(["scene_index"]),
        ),
    }
}

#[requires(true)]
#[ensures(true)]
fn assignments_cover(
    assignments: &[AssignmentDocument],
    people: &[String],
    professions: &[String],
    cities: &[String],
) -> bool {
    let assigned_people = assignments
        .iter()
        .map(|assignment| &assignment.person)
        .collect::<BTreeSet<_>>();
    let assigned_professions = assignments
        .iter()
        .map(|assignment| &assignment.profession)
        .collect::<BTreeSet<_>>();
    let assigned_cities = assignments
        .iter()
        .map(|assignment| &assignment.city)
        .collect::<BTreeSet<_>>();
    assigned_people == people.iter().collect()
        && assigned_professions == professions.iter().collect()
        && assigned_cities == cities.iter().collect()
}

#[requires(true)]
#[ensures(true)]
fn assignments_equal(answer: &[Assignment], solution: &[AssignmentDocument]) -> bool {
    let answer = answer
        .iter()
        .map(|assignment| {
            (
                assignment.person.as_str(),
                assignment.profession.as_str(),
                assignment.city.as_str(),
            )
        })
        .collect::<BTreeSet<_>>();
    let solution = solution
        .iter()
        .map(|assignment| {
            (
                assignment.person.as_str(),
                assignment.profession.as_str(),
                assignment.city.as_str(),
            )
        })
        .collect::<BTreeSet<_>>();
    answer == solution
}

#[requires(true)]
#[ensures(true)]
fn assignments_equal_documents(left: &[AssignmentDocument], right: &[AssignmentDocument]) -> bool {
    left.len() == right.len()
        && left.iter().all(|candidate| {
            right.iter().any(|expected| {
                candidate.person == expected.person
                    && candidate.profession == expected.profession
                    && candidate.city == expected.city
            })
        })
}

#[requires(true)]
#[ensures(!ret.is_empty() || values.is_empty())]
fn permutations(values: &[String]) -> Vec<Vec<String>> {
    if values.is_empty() {
        return vec![Vec::new()];
    }
    let mut result = Vec::new();
    for index in 0..values.len() {
        let mut remainder = values.to_vec();
        let head = remainder.remove(index);
        for mut tail in permutations(&remainder) {
            let mut permutation = Vec::with_capacity(values.len());
            permutation.push(head.clone());
            permutation.append(&mut tail);
            result.push(permutation);
        }
    }
    result
}

#[requires(people.len() == professions.len() && people.len() == cities.len())]
#[ensures(true)]
fn matching_assignments(
    people: &[String],
    professions: &[String],
    cities: &[String],
    clues: &[PuzzleClueDocument],
) -> Vec<Vec<AssignmentDocument>> {
    let profession_permutations = permutations(professions);
    let city_permutations = permutations(cities);
    let mut matches = Vec::new();
    for profession_order in &profession_permutations {
        for city_order in &city_permutations {
            let assignments = people
                .iter()
                .zip(profession_order)
                .zip(city_order)
                .map(|((person, profession), city)| AssignmentDocument {
                    person: person.clone(),
                    profession: profession.clone(),
                    city: city.clone(),
                })
                .collect::<Vec<_>>();
            if clues.iter().all(|clue| clue_matches(clue, &assignments)) {
                matches.push(assignments);
            }
        }
    }
    matches
}

#[requires(true)]
#[ensures(true)]
fn clue_matches(clue: &PuzzleClueDocument, assignments: &[AssignmentDocument]) -> bool {
    match clue {
        PuzzleClueDocument::Profession {
            person,
            profession,
            matches,
        } => assignments
            .iter()
            .find(|assignment| assignment.person == *person)
            .is_some_and(|assignment| (assignment.profession == *profession) == *matches),
        PuzzleClueDocument::City {
            person,
            city,
            matches,
        } => assignments
            .iter()
            .find(|assignment| assignment.person == *person)
            .is_some_and(|assignment| (assignment.city == *city) == *matches),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCHEDULE_FIXTURES: [&str; 2] = [
        include_str!("../scenarios/schedule-negotiation-1.toml"),
        include_str!("../scenarios/schedule-negotiation-2.toml"),
    ];
    const DEDUCTION_FIXTURES: [&str; 2] = [
        include_str!("../scenarios/distributed-clue-deduction-1.toml"),
        include_str!("../scenarios/distributed-clue-deduction-2.toml"),
    ];

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn canonical_answer_schemas_are_objects() {
        for kind in [
            ScenarioKind::ScheduleNegotiation,
            ScenarioKind::DistributedClueDeduction,
            ScenarioKind::ReferentialGame,
        ] {
            assert!(answer_schema(kind).is_object());
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn scripted_schedule_answers_without_communication_never_succeed() {
        for source in SCHEDULE_FIXTURES {
            let instance = ScenarioInstance::from_toml(source).expect("schedule fixture");
            let ScenarioDataDocument::ScheduleNegotiation {
                meeting_duration_minutes,
                slot_granularity_minutes,
                participants,
            } = &instance.document.data
            else {
                unreachable!("schedule fixture has schedule data")
            };
            let answers = participants
                .iter()
                .enumerate()
                .map(|(index, participant)| {
                    let candidates = schedule_slots(
                        *meeting_duration_minutes,
                        *slot_granularity_minutes,
                        std::slice::from_ref(participant),
                    );
                    assert!(
                        candidates.len() > 1,
                        "private brief must not determine the answer"
                    );
                    let answer = candidates[index % candidates.len()].clone();
                    (participant.name.clone(), ScenarioAnswer::schedule(answer))
                })
                .collect::<BTreeMap<_, _>>();
            assert_ne!(
                instance.check_answers(&answers).status,
                TaskStatus::Success,
                "{}",
                instance.id()
            );
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn scripted_deduction_answers_without_communication_never_succeed() {
        for source in DEDUCTION_FIXTURES {
            let instance = ScenarioInstance::from_toml(source).expect("deduction fixture");
            let ScenarioDataDocument::DistributedClueDeduction {
                people,
                professions,
                cities,
                participants,
                ..
            } = &instance.document.data
            else {
                unreachable!("deduction fixture has deduction data")
            };
            let answers = participants
                .iter()
                .enumerate()
                .map(|(index, participant)| {
                    let candidates =
                        matching_assignments(people, professions, cities, &participant.clues);
                    assert!(
                        candidates.len() > 1,
                        "private brief must not determine the answer"
                    );
                    let rows = candidates[index % candidates.len()]
                        .iter()
                        .map(|assignment| {
                            Assignment::new(
                                assignment.person.clone(),
                                assignment.profession.clone(),
                                assignment.city.clone(),
                            )
                            .expect("candidate assignment")
                        })
                        .collect();
                    let answer = DeductionAnswer::new(rows).expect("candidate deduction answer");
                    (participant.name.clone(), ScenarioAnswer::deduction(answer))
                })
                .collect::<BTreeMap<_, _>>();
            assert_ne!(
                instance.check_answers(&answers).status,
                TaskStatus::Success,
                "{}",
                instance.id()
            );
        }
    }
}
