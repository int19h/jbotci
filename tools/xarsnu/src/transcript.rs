//! Append-only typed JSONL transcripts for protocol runs.

use std::collections::HashMap;
use std::fmt;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::Path;

#[allow(unused_imports)]
use bityzba::{ensures, invariant, new, requires};
use serde::{Deserialize, Serialize};

use crate::protocol::ProtocolEventData;
use crate::{ProtocolEvent, RunConfig, ScenarioConfigError, ScenarioInstance};

/// Current on-disk transcript schema version.
///
/// Schema v1 record payloads may gain additive optional fields. Readers must
/// accept their absence with the documented default; incompatible shape or
/// meaning changes require incrementing this version.
pub const TRANSCRIPT_SCHEMA_VERSION: u32 = 1;

/// Immutable experiment inputs recorded before the first model call.
#[invariant(*schema_version == TRANSCRIPT_SCHEMA_VERSION, "unsupported transcript schema version")]
#[invariant(!scenario_instance_toml.trim().is_empty(), "scenario snapshot cannot be empty")]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct RunHeader {
    pub schema_version: u32,
    pub config: RunConfig,
    pub scenario_instance_toml: String,
}

impl RunHeader {
    /// Snapshot the full run configuration and validated scenario instance.
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|header| header.schema_version == TRANSCRIPT_SCHEMA_VERSION) || ret.is_err())]
    pub fn new(
        config: RunConfig,
        scenario: &ScenarioInstance,
    ) -> Result<Self, ScenarioConfigError> {
        let scenario_instance_toml = scenario.to_toml()?;
        Ok(new!(RunHeader {
            schema_version: TRANSCRIPT_SCHEMA_VERSION,
            config,
            scenario_instance_toml,
        }))
    }
}

/// One line in the transcript, with context shared by every event kind.
#[invariant(!participant.trim().is_empty(), "event participant cannot be empty")]
#[invariant((*turn_number == 0) == event.is_run_started(), "the run header is the only turn-zero event")]
#[invariant(event.is_run_started() -> *sequence_number == 0, "the run header must be sequence zero")]
#[invariant(event.explicit_turn_number().is_none_or(|event_turn| event_turn == *turn_number), "envelope and payload turns must agree")]
#[invariant(participant.as_str() == event.transcript_participant(), "envelope and payload participants must agree")]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct TranscriptRecord {
    pub sequence_number: u64,
    pub turn_number: usize,
    pub participant: String,
    pub event: ProtocolEvent,
}

impl TranscriptRecord {
    #[requires(!participant.trim().is_empty())]
    #[requires(event.is_run_started() == (turn_number == 0))]
    #[ensures(ret.sequence_number == sequence_number)]
    fn from_event_at(
        sequence_number: u64,
        turn_number: usize,
        participant: String,
        event: ProtocolEvent,
    ) -> Self {
        new!(TranscriptRecord {
            sequence_number,
            turn_number,
            participant,
            event,
        })
    }
}

/// JSONL writer that flushes every event before returning.
#[invariant(true, "the sequence advances only after a complete flushed line")]
pub(crate) struct TranscriptWriter {
    writer: BufWriter<File>,
    next_sequence: u64,
}

impl fmt::Debug for TranscriptWriter {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TranscriptWriter")
            .field("next_sequence", &self.next_sequence)
            .finish_non_exhaustive()
    }
}

impl TranscriptWriter {
    /// Create a new transcript file without replacing an existing artifact.
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|writer| writer.next_sequence == 0) || ret.is_err())]
    pub fn create(path: &Path) -> Result<Self, TranscriptError> {
        let file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(path)
            .map_err(|error| TranscriptError::io(0, error))?;
        Ok(Self {
            writer: BufWriter::new(file),
            next_sequence: 0,
        })
    }

    /// Append using the runner's exact current turn and principal actor.
    #[requires(!participant.trim().is_empty())]
    #[requires(event.is_run_started() == (turn_number == 0))]
    #[ensures(ret.is_ok() -> self.next_sequence == old(self.next_sequence) + 1)]
    pub(crate) fn append_at(
        &mut self,
        turn_number: usize,
        participant: String,
        event: ProtocolEvent,
    ) -> Result<(), TranscriptError> {
        let line = usize::try_from(self.next_sequence)
            .unwrap_or(usize::MAX)
            .saturating_add(1);
        let record =
            TranscriptRecord::from_event_at(self.next_sequence, turn_number, participant, event);
        serde_json::to_writer(&mut self.writer, &record)
            .map_err(|error| TranscriptError::bad_json(line, error.to_string()))?;
        self.writer
            .write_all(b"\n")
            .and_then(|()| self.writer.flush())
            .map_err(|error| TranscriptError::io(line, error))?;
        self.next_sequence = self.next_sequence.saturating_add(1);
        Ok(())
    }
}

/// Read and validate an entire transcript before any reporting begins.
#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|records| !records.is_empty()) || ret.is_err())]
pub fn read_transcript(path: &Path) -> Result<Vec<TranscriptRecord>, TranscriptError> {
    let file = File::open(path).map_err(|error| TranscriptError::io(0, error))?;
    let mut records = Vec::new();
    let mut current_turn = 0usize;
    // Latest `IntentRegistered` revision number seen per (turn, speaker), used to
    // check that each confirm's `intent_sequence` names its governing intent.
    let mut latest_intent: HashMap<(usize, String), usize> = HashMap::new();
    for (index, line) in BufReader::new(file).lines().enumerate() {
        let line_number = index + 1;
        let line = line.map_err(|error| TranscriptError::io(line_number, error))?;
        let record = serde_json::from_str::<TranscriptRecord>(&line)
            .map_err(|error| TranscriptError::bad_json(line_number, error.to_string()))?;
        if index == 0 {
            if !record.event.is_run_started() {
                return Err(new!(TranscriptError {
                    line: line_number,
                    kind: new!(TranscriptErrorKind::MissingRunHeader),
                }));
            }
        } else if record.event.is_run_started() {
            return Err(new!(TranscriptError {
                line: line_number,
                kind: new!(TranscriptErrorKind::UnexpectedRunHeader),
            }));
        }
        let expected = u64::try_from(index).unwrap_or(u64::MAX);
        if record.sequence_number != expected {
            return Err(new!(TranscriptError {
                line: line_number,
                kind: new!(TranscriptErrorKind::SequenceGap {
                    expected,
                    actual: record.sequence_number,
                }),
            }));
        }
        if index > 0 {
            if let Some(started_turn) = record.event.started_turn_number() {
                current_turn = started_turn;
            }
            if record.turn_number != current_turn || current_turn == 0 {
                return Err(new!(TranscriptError {
                    line: line_number,
                    kind: new!(TranscriptErrorKind::TurnContextMismatch {
                        expected: current_turn,
                        actual: record.turn_number,
                    }),
                }));
            }
        }
        // A confirm's `intent_sequence`, when present, must name the latest intent
        // registered so far for the same turn and speaker (issue #612). `None` marks
        // a legacy transcript and is accepted without a governing intent.
        //
        // This is a targeted link check, deliberately NOT a full state-machine replay:
        // it confirms the intent↔confirm reference is internally consistent, not that
        // the whole protocol run was legal. It intentionally does not catch every
        // fabricated shape — e.g. a confirm forged after a forfeit, or one whose
        // `intent_sequence` is `None`, passes here. Full replay validation, if ever
        // needed, belongs in a separate pass; the audit tooling that scores drift is
        // the consumer that reconstructs run state.
        match record.event.as_data() {
            bityzba::data!(ProtocolEvent::IntentRegistered {
                turn_number,
                speaker,
                revision_number,
                ..
            }) => {
                latest_intent.insert((*turn_number, speaker.clone()), *revision_number);
            }
            bityzba::data!(ProtocolEvent::MeaningConfirmed {
                turn_number,
                speaker,
                intent_sequence: Some(sequence),
                ..
            }) => {
                let governing = latest_intent.get(&(*turn_number, speaker.clone())).copied();
                if governing != Some(*sequence) {
                    return Err(new!(TranscriptError {
                        line: line_number,
                        kind: new!(TranscriptErrorKind::ConfirmIntentSequenceMismatch {
                            turn: *turn_number,
                            confirm_sequence: *sequence,
                            governing,
                        }),
                    }));
                }
            }
            _ => {}
        }
        records.push(record);
    }
    if records.is_empty() {
        return Err(new!(TranscriptError {
            line: 1,
            kind: new!(TranscriptErrorKind::MissingRunHeader),
        }));
    }
    if !records
        .last()
        .is_some_and(|record| record.event.is_terminal())
    {
        return Err(new!(TranscriptError {
            line: records.len(),
            kind: new!(TranscriptErrorKind::Truncated),
        }));
    }
    Ok(records)
}

/// Typed reason a transcript could not be written or validated.
#[invariant(::Io { message } => !message.trim().is_empty())]
#[invariant(::BadJson { message } => !message.trim().is_empty())]
#[invariant(::MissingRunHeader => true)]
#[invariant(::UnexpectedRunHeader => true)]
#[invariant(::SequenceGap { .. } => true)]
#[invariant(::TurnContextMismatch { .. } => true)]
#[invariant(::ConfirmIntentSequenceMismatch { confirm_sequence, governing, .. } => governing.as_ref().is_none_or(|latest| *latest != *confirm_sequence), "the mismatch is only raised when the confirm names a non-latest or absent intent")]
#[invariant(::Truncated => true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TranscriptErrorKind {
    Io {
        message: String,
    },
    BadJson {
        message: String,
    },
    MissingRunHeader,
    UnexpectedRunHeader,
    SequenceGap {
        expected: u64,
        actual: u64,
    },
    TurnContextMismatch {
        expected: usize,
        actual: usize,
    },
    /// A confirm's `intent_sequence` did not name the latest intent registered so
    /// far for its turn/speaker; `governing` is that latest revision, or `None` if
    /// no intent was registered before the confirm.
    ConfirmIntentSequenceMismatch {
        turn: usize,
        confirm_sequence: usize,
        governing: Option<usize>,
    },
    Truncated,
}

/// Transcript failure with a one-based source line (`0` only for open failures).
#[invariant(*line > 0 || kind.is_io(), "content errors must identify a one-based line")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranscriptError {
    pub line: usize,
    pub kind: TranscriptErrorKind,
}

impl TranscriptErrorKind {
    #[requires(true)]
    #[ensures(ret == matches!(self.as_data(), bityzba::data!(TranscriptErrorKind::Io { .. })))]
    fn is_io(&self) -> bool {
        matches!(
            self.as_data(),
            bityzba::data!(TranscriptErrorKind::Io { .. })
        )
    }
}

impl TranscriptError {
    #[requires(!message.trim().is_empty())]
    #[ensures(ret.line == line)]
    fn bad_json(line: usize, message: String) -> Self {
        new!(TranscriptError {
            line,
            kind: new!(TranscriptErrorKind::BadJson { message }),
        })
    }

    #[requires(true)]
    #[ensures(ret.line == line)]
    fn io(line: usize, error: std::io::Error) -> Self {
        new!(TranscriptError {
            line,
            kind: new!(TranscriptErrorKind::Io {
                message: error.to_string(),
            }),
        })
    }
}

impl fmt::Display for TranscriptError {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let location = if self.line == 0 {
            "before line 1".to_owned()
        } else {
            format!("at line {}", self.line)
        };
        match self.kind.as_data() {
            bityzba::data!(TranscriptErrorKind::Io { message }) => {
                write!(formatter, "transcript I/O failure {location}: {message}")
            }
            bityzba::data!(TranscriptErrorKind::BadJson { message }) => {
                write!(formatter, "invalid transcript JSON {location}: {message}")
            }
            bityzba::data!(TranscriptErrorKind::MissingRunHeader) => {
                write!(formatter, "missing run header {location}")
            }
            bityzba::data!(TranscriptErrorKind::UnexpectedRunHeader) => {
                write!(formatter, "unexpected second run header {location}")
            }
            bityzba::data!(TranscriptErrorKind::SequenceGap { expected, actual }) => write!(
                formatter,
                "transcript sequence gap {location}: expected {expected}, found {actual}"
            ),
            bityzba::data!(TranscriptErrorKind::TurnContextMismatch { expected, actual }) => {
                write!(
                    formatter,
                    "transcript turn context mismatch {location}: expected {expected}, found {actual}"
                )
            }
            bityzba::data!(TranscriptErrorKind::ConfirmIntentSequenceMismatch {
                turn,
                confirm_sequence,
                governing,
            }) => {
                let governing = match governing {
                    Some(latest) => format!("latest registered intent is revision {latest}"),
                    None => "no intent was registered for this turn/speaker".to_owned(),
                };
                write!(
                    formatter,
                    "transcript confirm intent-sequence mismatch {location}: turn {turn} confirm names intent revision {confirm_sequence}, but {governing}"
                )
            }
            bityzba::data!(TranscriptErrorKind::Truncated) => {
                write!(
                    formatter,
                    "transcript is truncated {location}: no terminal event"
                )
            }
        }
    }
}

impl std::error::Error for TranscriptError {}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn transcript_creation_never_replaces_an_existing_artifact() {
        let executable = std::env::current_exe().expect("current test executable");
        let target_directory = executable
            .parent()
            .and_then(Path::parent)
            .and_then(Path::parent)
            .expect("Cargo target directory");
        let directory = target_directory.join("xarsnu-test-tmp");
        fs::create_dir_all(&directory).expect("create target temporary directory");
        let path = directory.join(format!("xarsnu-no-overwrite-{}.jsonl", std::process::id()));
        fs::write(&path, b"sentinel\n").expect("write existing artifact");

        let error = TranscriptWriter::create(&path).expect_err("existing path must be preserved");

        assert!(matches!(
            error.kind.as_data(),
            bityzba::data!(TranscriptErrorKind::Io { .. })
        ));
        assert_eq!(
            fs::read(&path).expect("read existing artifact"),
            b"sentinel\n"
        );
        fs::remove_file(path).expect("remove existing artifact");
    }
}
