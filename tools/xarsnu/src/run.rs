//! Live conductor that composes configuration, scenario, runtime, protocol, and transcript.

use std::collections::BTreeSet;
use std::fmt::{self, Write as _};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[allow(unused_imports)]
use bityzba::{ensures, invariant, new, requires};

use crate::protocol::ProtocolRunOutcomeData;
use crate::{
    EmbeddingSearchPreflightError, MeaningReviewer, OpenRouterClient, OpenRouterError,
    OpenRouterParticipant, OpenRouterReviewer, ProtocolModel, ProtocolRunError,
    ProtocolRunOutcome, ProtocolRunner, ReferenceToolDispatcher, RunConfig, RunHeader,
    ScenarioInstance, TaskOutcome, TaskStatus, ToolDispatcher, preflight_embedding_search,
};

static TRANSCRIPT_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Successful live-run result. Task failure remains a successful execution result.
#[invariant(!transcript_path.as_os_str().is_empty())]
#[invariant(task_outcome.is_some() || matches!(outcome.as_data(), bityzba::data!(ProtocolRunOutcome::Completed { .. }) | bityzba::data!(ProtocolRunOutcome::BudgetAborted { .. })))]
#[derive(Debug, Clone, PartialEq)]
pub struct RunSummary {
    pub transcript_path: PathBuf,
    pub outcome: ProtocolRunOutcome,
    pub task_outcome: Option<TaskOutcome>,
    pub warnings: Vec<RunWarning>,
}

impl RunSummary {
    /// One-line human-readable outcome for the CLI.
    #[requires(true)]
    #[ensures(!ret.trim().is_empty())]
    pub fn outcome_line(&self) -> String {
        if let Some(task_outcome) = &self.task_outcome {
            format!(
                "task {} after {} turn(s)",
                task_status_name(task_outcome.status),
                self.outcome.turns(),
            )
        } else {
            match self.outcome.as_data() {
                bityzba::data!(ProtocolRunOutcome::Completed { turns }) => {
                    format!("dialog completed after {turns} turns")
                }
                bityzba::data!(ProtocolRunOutcome::BudgetAborted { turns, .. }) => {
                    format!("budget aborted after {turns} turn(s)")
                }
                bityzba::data!(ProtocolRunOutcome::ScenarioCompleted { .. }) => {
                    unreachable!("scenario completion always carries a task outcome")
                }
            }
        }
    }
}

/// Non-fatal condition retained only through an explicit run-config override.
#[invariant(::EmbeddingSearchDegraded { message } => !message.trim().is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunWarning {
    EmbeddingSearchDegraded { message: String },
}

impl fmt::Display for RunWarning {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.as_data() {
            bityzba::data!(RunWarning::EmbeddingSearchDegraded { message }) => write!(
                formatter,
                "embedding search is degraded despite startup preflight failure: {message}"
            ),
        }
    }
}

/// Typed failure from loading or executing one live run.
#[invariant(::ConfigRead { path, message } => !path.as_os_str().is_empty() && !message.trim().is_empty())]
#[invariant(::ConfigInvalid { path, message } => !path.as_os_str().is_empty() && !message.trim().is_empty())]
#[invariant(::ScenarioNotFound { reference, searched } => !reference.trim().is_empty() && !searched.is_empty())]
#[invariant(::ScenarioRead { path, message } => !path.as_os_str().is_empty() && !message.trim().is_empty())]
#[invariant(::ScenarioInvalid { path, message } => !path.as_os_str().is_empty() && !message.trim().is_empty())]
#[invariant(::ParticipantMismatch { configured_only, scenario_only } => !configured_only.is_empty() || !scenario_only.is_empty())]
#[invariant(::EmbeddingSearchUnavailable { message } => !message.trim().is_empty())]
#[invariant(::Client { message } => !message.trim().is_empty())]
#[invariant(::Header { message } => !message.trim().is_empty())]
#[invariant(::ProtocolSetup { message } => !message.trim().is_empty())]
#[invariant(::TranscriptPath { config_path, message } => !config_path.as_os_str().is_empty() && !message.trim().is_empty())]
#[invariant(::Protocol { transcript_path, .. } => !transcript_path.as_os_str().is_empty())]
#[derive(Debug)]
pub enum RunError {
    ConfigRead {
        path: PathBuf,
        message: String,
    },
    ConfigInvalid {
        path: PathBuf,
        message: String,
    },
    ScenarioNotFound {
        reference: String,
        searched: Vec<PathBuf>,
    },
    ScenarioRead {
        path: PathBuf,
        message: String,
    },
    ScenarioInvalid {
        path: PathBuf,
        message: String,
    },
    ParticipantMismatch {
        configured_only: Vec<String>,
        scenario_only: Vec<String>,
    },
    EmbeddingSearchUnavailable {
        message: String,
    },
    Client {
        message: String,
    },
    Header {
        message: String,
    },
    ProtocolSetup {
        message: String,
    },
    TranscriptPath {
        config_path: PathBuf,
        message: String,
    },
    Protocol {
        transcript_path: PathBuf,
        source: ProtocolRunError,
    },
}

impl RunError {
    /// Transcript artifact retained by a run that reached transcript setup.
    #[requires(true)]
    #[ensures(ret.is_none_or(|path| !path.as_os_str().is_empty()))]
    pub fn transcript_path(&self) -> Option<&Path> {
        match self.as_data() {
            bityzba::data!(RunError::Protocol {
                transcript_path,
                ..
            }) => Some(transcript_path),
            _ => None,
        }
    }
}

impl fmt::Display for RunError {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.as_data() {
            bityzba::data!(RunError::ConfigRead { path, message }) => {
                write!(
                    formatter,
                    "unable to read xarsnu config `{}`: {message}",
                    path.display()
                )
            }
            bityzba::data!(RunError::ConfigInvalid { path, message }) => {
                write!(
                    formatter,
                    "invalid xarsnu config `{}`: {message}",
                    path.display()
                )
            }
            bityzba::data!(RunError::ScenarioNotFound {
                reference,
                searched,
            }) => {
                write!(formatter, "scenario `{reference}` was not found; searched")?;
                for path in searched {
                    write!(formatter, " `{}`", path.display())?;
                }
                Ok(())
            }
            bityzba::data!(RunError::ScenarioRead { path, message }) => write!(
                formatter,
                "unable to read scenario `{}`: {message}",
                path.display()
            ),
            bityzba::data!(RunError::ScenarioInvalid { path, message }) => write!(
                formatter,
                "invalid scenario `{}`: {message}",
                path.display()
            ),
            bityzba::data!(RunError::ParticipantMismatch {
                configured_only,
                scenario_only,
            }) => write!(
                formatter,
                "config/scenario participant mismatch (config only: {}; scenario only: {})",
                names_or_none(configured_only),
                names_or_none(scenario_only),
            ),
            bityzba::data!(RunError::EmbeddingSearchUnavailable { message }) => write!(
                formatter,
                "embedding search preflight failed: {message} Set `allow-degraded-search = true` only to run an intentional degraded-search measurement arm."
            ),
            bityzba::data!(RunError::Client { message }) => {
                write!(
                    formatter,
                    "unable to initialize OpenRouter client: {message}"
                )
            }
            bityzba::data!(RunError::Header { message }) => {
                write!(formatter, "unable to snapshot run inputs: {message}")
            }
            bityzba::data!(RunError::ProtocolSetup { message }) => {
                write!(formatter, "unable to set up protocol runner: {message}")
            }
            bityzba::data!(RunError::TranscriptPath {
                config_path,
                message,
            }) => write!(
                formatter,
                "unable to derive transcript path from `{}`: {message}",
                config_path.display()
            ),
            bityzba::data!(RunError::Protocol { source, .. }) => {
                fmt::Display::fmt(source, formatter)
            }
        }
    }
}

impl std::error::Error for RunError {
    #[requires(true)]
    #[ensures(true)]
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self.as_data() {
            bityzba::data!(RunError::Protocol { source, .. }) => Some(source),
            _ => None,
        }
    }
}

#[invariant(!config_path.as_os_str().is_empty())]
#[derive(Debug)]
struct LoadedRun {
    config_path: PathBuf,
    config: RunConfig,
    scenario: ScenarioInstance,
}

/// Execute the exact live path used by the CLI with an injected client factory.
///
/// Loading and validation happen before the factory is called. Injection keeps
/// the conductor offline-testable; production passes the environment-backed
/// OpenRouter constructor and receives the validated run HTTP settings.
#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
pub fn run<F>(config_path: &Path, client_factory: F) -> Result<RunSummary, RunError>
where
    F: FnOnce(Duration, Option<&str>) -> Result<OpenRouterClient, OpenRouterError>,
{
    run_with_preflight(
        config_path,
        client_factory,
        preflight_embedding_search,
        |_| {},
    )
}

/// Execute the production live path and surface tolerated startup warnings immediately.
#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
pub fn run_with_warning_handler<F, W>(
    config_path: &Path,
    client_factory: F,
    warning_handler: W,
) -> Result<RunSummary, RunError>
where
    F: FnOnce(Duration, Option<&str>) -> Result<OpenRouterClient, OpenRouterError>,
    W: FnMut(&RunWarning),
{
    run_with_preflight(
        config_path,
        client_factory,
        preflight_embedding_search,
        warning_handler,
    )
}

/// Execute the live path with an injected semantic-search preflight for offline tests.
#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
pub fn run_with_preflight<F, P, W>(
    config_path: &Path,
    client_factory: F,
    embedding_preflight: P,
    mut warning_handler: W,
) -> Result<RunSummary, RunError>
where
    F: FnOnce(Duration, Option<&str>) -> Result<OpenRouterClient, OpenRouterError>,
    P: FnOnce() -> Result<(), EmbeddingSearchPreflightError>,
    W: FnMut(&RunWarning),
{
    let loaded = load(config_path)?;
    let degraded_search_warning = match embedding_preflight() {
        Ok(()) => None,
        Err(error) if loaded.config.allow_degraded_search => {
            let warning = new!(RunWarning::EmbeddingSearchDegraded {
                message: error.to_string(),
            });
            warning_handler(&warning);
            Some(warning)
        }
        Err(error) => {
            return Err(new!(RunError::EmbeddingSearchUnavailable {
                message: error.to_string(),
            }));
        }
    };
    let client = client_factory(
        loaded.config.client.http_timeout(),
        loaded.config.client.base_url.as_deref(),
    )
    .map_err(|error| {
        new!(RunError::Client {
            message: error.to_string(),
        })
    })?;
    let bityzba::data!(LoadedRun {
        config_path,
        config,
        scenario,
    }) = loaded.into_data();
    let default_max_completion_tokens = config.client.max_completion_tokens;
    let participants = config
        .participants
        .iter()
        .map(|participant| {
            OpenRouterParticipant::new(participant, &client, default_max_completion_tokens)
        })
        .collect::<Vec<_>>();
    let caps = config.caps.clone();
    let listener_mode = config.listener_mode;
    let tersmu_format = config.tersmu_format.clone();
    // The adversarial reviewer is prepared before the config moves into the
    // transcript header; `OpenRouterReviewer::new` requires `enabled` (issue #723).
    let reviewer = config.meaning_review.enabled.then(|| {
        OpenRouterReviewer::new(
            &config.participants,
            config.meaning_review.clone(),
            &client,
            default_max_completion_tokens,
        )
    });
    let header = RunHeader::new(config, &scenario).map_err(|error| {
        new!(RunError::Header {
            message: error.to_string(),
        })
    })?;
    match reviewer {
        Some(reviewer) => {
            let runner = ProtocolRunner::new_with_scenario_and_review(
                participants,
                caps,
                listener_mode,
                tersmu_format,
                ReferenceToolDispatcher,
                scenario,
                reviewer,
            )
            .map_err(|error| {
                new!(RunError::ProtocolSetup {
                    message: error.to_string(),
                })
            })?;
            execute_protocol(runner, header, &config_path, degraded_search_warning)
        }
        None => {
            let runner = ProtocolRunner::new_with_scenario(
                participants,
                caps,
                listener_mode,
                tersmu_format,
                ReferenceToolDispatcher,
                scenario,
            )
            .map_err(|error| {
                new!(RunError::ProtocolSetup {
                    message: error.to_string(),
                })
            })?;
            execute_protocol(runner, header, &config_path, degraded_search_warning)
        }
    }
}

/// Attach the transcript, replay any tolerated preflight warning, and run the
/// protocol to its terminal outcome.
#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn execute_protocol<M, D, R>(
    mut runner: ProtocolRunner<M, D, R>,
    header: RunHeader,
    config_path: &Path,
    degraded_search_warning: Option<RunWarning>,
) -> Result<RunSummary, RunError>
where
    M: ProtocolModel,
    D: ToolDispatcher,
    R: MeaningReviewer,
{
    let transcript_path = transcript_path(config_path)?;
    runner
        .attach_transcript(&transcript_path, header)
        .map_err(|source| {
            new!(RunError::Protocol {
                transcript_path: transcript_path.clone(),
                source,
            })
        })?;
    let warnings = if let Some(warning) = degraded_search_warning {
        let bityzba::data!(RunWarning::EmbeddingSearchDegraded { message }) = warning.as_data();
        runner
            .record_degraded_search(message.clone())
            .map_err(|error| {
                new!(RunError::ProtocolSetup {
                    message: error.to_string(),
                })
            })?;
        vec![warning]
    } else {
        Vec::new()
    };
    let outcome = runner.run().map_err(|source| {
        new!(RunError::Protocol {
            transcript_path: transcript_path.clone(),
            source,
        })
    })?;
    Ok(new!(RunSummary {
        transcript_path,
        outcome,
        task_outcome: runner.task_outcome().cloned(),
        warnings,
    }))
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn load(config_path: &Path) -> Result<LoadedRun, RunError> {
    let config_path = config_path.to_owned();
    let source = fs::read_to_string(&config_path).map_err(|error| {
        new!(RunError::ConfigRead {
            path: config_path.clone(),
            message: error.to_string(),
        })
    })?;
    let config = RunConfig::from_toml(&source).map_err(|error| {
        new!(RunError::ConfigInvalid {
            path: config_path.clone(),
            message: error.to_string(),
        })
    })?;
    let scenario = load_scenario(&config_path, &config.scenario)?;
    validate_participants(&config, &scenario)?;
    Ok(new!(LoadedRun {
        config_path,
        config,
        scenario,
    }))
}

#[requires(!reference.trim().is_empty())]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn load_scenario(config_path: &Path, reference: &str) -> Result<ScenarioInstance, RunError> {
    let candidates = scenario_candidates(config_path, reference);
    for path in &candidates {
        let source = match fs::read_to_string(path) {
            Ok(source) => source,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => {
                return Err(new!(RunError::ScenarioRead {
                    path: path.clone(),
                    message: error.to_string(),
                }));
            }
        };
        let scenario = ScenarioInstance::from_toml(&source).map_err(|error| {
            new!(RunError::ScenarioInvalid {
                path: path.clone(),
                message: error.to_string(),
            })
        })?;
        return Ok(scenario);
    }
    Err(new!(RunError::ScenarioNotFound {
        reference: reference.to_owned(),
        searched: candidates,
    }))
}

#[requires(!reference.trim().is_empty())]
#[ensures(!ret.is_empty())]
fn scenario_candidates(config_path: &Path, reference: &str) -> Vec<PathBuf> {
    let reference_path = Path::new(reference);
    if reference_path.is_absolute() {
        return vec![reference_path.to_owned()];
    }
    let config_directory = config_path.parent().unwrap_or_else(|| Path::new("."));
    let relative = config_directory.join(reference_path);
    let fallback = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("scenarios")
        .join(reference_path);
    if relative == fallback {
        vec![relative]
    } else {
        vec![relative, fallback]
    }
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn validate_participants(config: &RunConfig, scenario: &ScenarioInstance) -> Result<(), RunError> {
    let configured = config
        .participants
        .iter()
        .map(|participant| participant.name.as_str())
        .collect::<BTreeSet<_>>();
    let scenario = scenario
        .participants()
        .iter()
        .map(|participant| participant.name.as_str())
        .collect::<BTreeSet<_>>();
    if configured == scenario {
        return Ok(());
    }
    let configured_only = configured
        .difference(&scenario)
        .map(|name| (*name).to_owned())
        .collect();
    let scenario_only = scenario
        .difference(&configured)
        .map(|name| (*name).to_owned())
        .collect();
    Err(new!(RunError::ParticipantMismatch {
        configured_only,
        scenario_only,
    }))
}

#[requires(!config_path.as_os_str().is_empty())]
#[ensures(ret.as_ref().is_ok_and(|path| path.extension().is_some_and(|extension| extension == "jsonl")) || ret.is_err())]
fn transcript_path(config_path: &Path) -> Result<PathBuf, RunError> {
    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| {
            new!(RunError::TranscriptPath {
                config_path: config_path.to_owned(),
                message: error.to_string(),
            })
        })?;
    let stem = config_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.is_empty())
        .unwrap_or("xarsnu-run");
    let sequence = TRANSCRIPT_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let file_name = format!(
        "{stem}.xarsnu.{}.{}.{}.jsonl",
        elapsed.as_nanos(),
        std::process::id(),
        sequence,
    );
    Ok(config_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(file_name))
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn names_or_none(names: &[String]) -> String {
    if names.is_empty() {
        return "none".to_owned();
    }
    let mut rendered = String::new();
    for (index, name) in names.iter().enumerate() {
        if index > 0 {
            rendered.push_str(", ");
        }
        write!(rendered, "`{name}`").expect("writing to String cannot fail");
    }
    rendered
}

#[requires(true)]
#[ensures(!ret.is_empty())]
const fn task_status_name(status: TaskStatus) -> &'static str {
    match status {
        TaskStatus::Success => "succeeded",
        TaskStatus::Partial => "partially succeeded",
        TaskStatus::Failure => "failed",
    }
}
