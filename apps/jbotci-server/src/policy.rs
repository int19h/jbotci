//! Validated limits shared by every server adapter.

use std::ffi::OsString;
use std::time::Duration;

use crate::work::WorkLimits;
use bityzba::{data, invariant, new, requires};
use jbotci_jvozba::JvozbaBuildLimits;

pub(crate) use jbotci_gentufa::GentufaRenderLimits as DiagramLimits;

#[requires(true)]
#[ensures(ret.max_blocks > 0)]
pub(crate) fn default_diagram_limits() -> DiagramLimits {
    new!(DiagramLimits {
        max_blocks: 600,
        max_columns: 160,
        // Earlier measurements used a 512 MiB instance with a 240 MiB model.
        // An 8-megapixel RGBA render used about 50 MiB with layout and encoding.
        // Keep that measured policy as the default for every server adapter.
        max_pixels: 8_000_000,
        max_bytes: 8 * 1024 * 1024,
    })
}

#[invariant(*source_field_units > 0 && *source_total_units > 0)]
#[invariant(*result_entries > 0 && *text_bytes > 0 && *response_bytes > 0)]
#[invariant(!deadline.is_zero() && *deadline <= Duration::from_millis(u64::MAX))]
#[invariant(!error_delivery.is_zero() && *error_delivery <= Duration::from_millis(u64::MAX))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerPolicy {
    pub(crate) work: WorkLimits,
    pub(crate) source_field_units: usize,
    pub(crate) source_total_units: usize,
    pub(crate) result_entries: usize,
    pub(crate) text_bytes: usize,
    pub(crate) response_bytes: usize,
    pub(crate) deadline: Duration,
    pub(crate) error_delivery: Duration,
    pub(crate) jvozba: JvozbaBuildLimits,
    pub(crate) diagram: DiagramLimits,
}

impl Default for ServerPolicy {
    #[requires(true)]
    #[ensures(ret.source_field_units == 4000 && ret.source_total_units == 4000)]
    fn default() -> Self {
        new!(ServerPolicy {
            work: WorkLimits::default(),
            source_field_units: 4000,
            source_total_units: 4000,
            result_entries: 512,
            text_bytes: 1024 * 1024,
            response_bytes: 12 * 1024 * 1024,
            deadline: Duration::from_secs(20),
            error_delivery: Duration::from_secs(1),
            jvozba: default_jvozba_limits(),
            diagram: default_diagram_limits(),
        })
    }
}

impl ServerPolicy {
    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn tool_limits(&self) -> jbotci_cli::ToolExecutionLimits {
        jbotci_cli::ToolExecutionLimits {
            render: jbotci_output::budget::RenderLimits::new(
                jbotci_output::budget::ByteLimit::new(self.text_bytes)
                    .expect("validated text limit"),
                jbotci_output::budget::ByteLimit::new(self.response_bytes)
                    .expect("validated response limit"),
            ),
            entries: std::num::NonZeroUsize::new(self.result_entries)
                .expect("validated entry limit"),
            jvozba: self.jvozba,
            diagram: self.diagram,
        }
    }

    #[requires(true)]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    pub fn from_env() -> Result<Self, PolicyError> {
        Self::from_lookup(|name| std::env::var_os(name))
    }

    #[requires(true)]
    #[ensures(true)]
    fn from_lookup(mut lookup: impl FnMut(&str) -> Option<OsString>) -> Result<Self, PolicyError> {
        let d = Self::default();
        let read = |lookup: &mut dyn FnMut(&str) -> Option<OsString>, name, default, min, max| {
            read_limit(lookup(name), name, default, min, max)
        };
        let workers_max = tokio::sync::Semaphore::MAX_PERMITS;
        let compute_workers = read(
            &mut lookup,
            "JBOTCI_COMPUTE_ACTIVE",
            d.work.compute_workers,
            1,
            workers_max,
        )?;
        let compute_queue = read(
            &mut lookup,
            "JBOTCI_COMPUTE_QUEUED",
            d.work.compute_queue,
            0,
            usize::MAX,
        )?;
        let fetch_workers = read(
            &mut lookup,
            "JBOTCI_FETCH_ACTIVE",
            d.work.fetch_workers,
            1,
            workers_max,
        )?;
        let fetch_queue = read(
            &mut lookup,
            "JBOTCI_FETCH_QUEUED",
            d.work.fetch_queue,
            0,
            usize::MAX,
        )?;
        let work = new!(WorkLimits {
            compute_workers,
            compute_queue,
            fetch_workers,
            fetch_queue,
        });
        let max_pieces = read(
            &mut lookup,
            "JBOTCI_JVOZBA_PIECES",
            d.jvozba.max_pieces,
            2,
            usize::MAX,
        )?;
        let max_placements = read_u64_limit(
            lookup("JBOTCI_JVOZBA_PLACEMENTS"),
            "JBOTCI_JVOZBA_PLACEMENTS",
            d.jvozba.max_placements,
            1,
            u64::MAX,
        )?;
        let max_spelling_letters = read(
            &mut lookup,
            "JBOTCI_JVOZBA_LETTERS",
            d.jvozba.max_spelling_letters,
            1,
            usize::MAX,
        )?;
        let jvozba = new!(JvozbaBuildLimits {
            max_pieces,
            max_placements,
            max_spelling_letters,
        });
        let max_blocks = read(
            &mut lookup,
            "JBOTCI_DIAGRAM_BLOCKS",
            d.diagram.max_blocks,
            1,
            usize::MAX,
        )?;
        let max_columns = read(
            &mut lookup,
            "JBOTCI_DIAGRAM_COLUMNS",
            d.diagram.max_columns,
            1,
            usize::MAX,
        )?;
        let max_pixels = read_u64_limit(
            lookup("JBOTCI_DIAGRAM_PIXELS"),
            "JBOTCI_DIAGRAM_PIXELS",
            d.diagram.max_pixels,
            1,
            u64::MAX,
        )?;
        let max_bytes = read(
            &mut lookup,
            "JBOTCI_DIAGRAM_BYTES",
            d.diagram.max_bytes,
            1,
            usize::MAX,
        )?;
        let diagram = new!(DiagramLimits {
            max_blocks,
            max_columns,
            max_pixels,
            max_bytes,
        });
        let source_field_units = read(
            &mut lookup,
            "JBOTCI_SOURCE_FIELD_UNITS",
            d.source_field_units,
            1,
            usize::MAX,
        )?;
        let source_total_units = read(
            &mut lookup,
            "JBOTCI_SOURCE_TOTAL_UNITS",
            d.source_total_units,
            1,
            usize::MAX,
        )?;
        let result_entries = read(
            &mut lookup,
            "JBOTCI_RESULT_ENTRIES",
            d.result_entries,
            1,
            usize::MAX,
        )?;
        let text_bytes = read(
            &mut lookup,
            "JBOTCI_TEXT_BYTES",
            d.text_bytes,
            1,
            usize::MAX,
        )?;
        let response_bytes = read(
            &mut lookup,
            "JBOTCI_RESPONSE_BYTES",
            d.response_bytes,
            1,
            usize::MAX,
        )?;
        let deadline = Duration::from_millis(read_u64_limit(
            lookup("JBOTCI_DEADLINE_MS"),
            "JBOTCI_DEADLINE_MS",
            d.deadline.as_millis() as u64,
            1,
            u64::MAX,
        )?);
        let error_delivery = Duration::from_millis(read_u64_limit(
            lookup("JBOTCI_ERROR_DELIVERY_MS"),
            "JBOTCI_ERROR_DELIVERY_MS",
            d.error_delivery.as_millis() as u64,
            1,
            u64::MAX,
        )?);
        let policy = new!(ServerPolicy {
            work,
            jvozba,
            diagram,
            source_field_units,
            source_total_units,
            result_entries,
            text_bytes,
            response_bytes,
            deadline,
            error_delivery,
        });
        if std::time::Instant::now()
            .checked_add(policy.deadline)
            .is_none()
        {
            return Err(new!(PolicyError::ClockRange));
        }
        if std::time::Instant::now()
            .checked_add(policy.deadline)
            .and_then(|deadline| deadline.checked_add(policy.error_delivery))
            .is_none()
        {
            return Err(new!(PolicyError::DeliveryClockRange));
        }
        Ok(policy)
    }

    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn validate_sources<'a>(
        &self,
        sources: impl IntoIterator<Item = &'a str>,
    ) -> Result<(), RequestLimitError> {
        let mut total = 0usize;
        for source in sources {
            let units = source.encode_utf16().count();
            if units > self.source_field_units {
                return Err(RequestLimitError::SourceField);
            }
            total = total
                .checked_add(units)
                .ok_or(RequestLimitError::SourceTotal)?;
            if total > self.source_total_units {
                return Err(RequestLimitError::SourceTotal);
            }
        }
        Ok(())
    }

    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn validate_count(&self, count: usize) -> Result<(), RequestLimitError> {
        if count == 0 || count > self.result_entries {
            Err(RequestLimitError::ResultCount)
        } else {
            Ok(())
        }
    }
}

#[invariant(::Range { name, min, max } => !name.is_empty() && min <= max)]
#[invariant(::ClockRange => true)]
#[invariant(::DeliveryClockRange => true)]
#[derive(Debug)]
pub enum PolicyError {
    Range {
        name: &'static str,
        min: u64,
        max: u64,
    },
    ClockRange,
    DeliveryClockRange,
}
impl std::fmt::Display for PolicyError {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.as_data() {
            data!(PolicyError::Range { name, min, max }) => {
                write!(f, "{name} must be an integer from {min} through {max}")
            }
            data!(PolicyError::DeliveryClockRange) => {
                f.write_str("JBOTCI_ERROR_DELIVERY_MS exceeds the range of the server clock")
            }
            data!(PolicyError::ClockRange) => {
                f.write_str("JBOTCI_DEADLINE_MS exceeds the range of the server clock")
            }
        }
    }
}
impl std::error::Error for PolicyError {}
impl PolicyError {
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn name(&self) -> &'static str {
        match self.as_data() {
            data!(PolicyError::Range { name, .. }) => name,
            data!(PolicyError::ClockRange) => "JBOTCI_DEADLINE_MS",
            data!(PolicyError::DeliveryClockRange) => "JBOTCI_ERROR_DELIVERY_MS",
        }
    }
}

#[requires(!name.is_empty() && min <= default && default <= max)]
#[ensures(ret.as_ref().is_ok_and(|value| min <= *value && *value <= max) || ret.is_err())]
fn read_limit(
    value: Option<OsString>,
    name: &'static str,
    default: usize,
    min: usize,
    max: usize,
) -> Result<usize, PolicyError> {
    read_u64_limit(value, name, default as u64, min as u64, max as u64).map(|value| value as usize)
}

#[requires(!name.is_empty() && min <= default && default <= max)]
#[ensures(ret.as_ref().is_ok_and(|value| min <= *value && *value <= max) || ret.is_err())]
fn read_u64_limit(
    value: Option<OsString>,
    name: &'static str,
    default: u64,
    min: u64,
    max: u64,
) -> Result<u64, PolicyError> {
    let Some(value) = value else {
        return Ok(default);
    };
    value
        .to_str()
        .and_then(|text| text.parse::<u64>().ok())
        .filter(|value| min <= *value && *value <= max)
        .ok_or_else(|| new!(PolicyError::Range { name, min, max }))
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RequestLimitError {
    SourceField,
    SourceTotal,
    ResultCount,
    CompoundParts,
}
impl std::fmt::Display for RequestLimitError {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::SourceField => "A source field exceeds the server limit.",
            Self::SourceTotal => "The source fields exceed the total server limit.",
            Self::ResultCount => {
                "The requested result count must be positive and within the server limit."
            }
            Self::CompoundParts => crate::error::ServerErrorCode::OperationLimit.message(),
        })
    }
}
impl std::error::Error for RequestLimitError {}

/// How much work one server compound build may do. The search compares every
/// spelling before it can name a best one and nothing stops it once it starts,
/// so the whole request is refused when the plan is too large.
///
/// Measured in release on the development machine, with `klama` repeated so
/// each non-final piece offers two candidates: 48 placements took 5.5ms, 320
/// took 65ms, 768 took 178ms, 4096 took 1.24s, 9216 took 3.1s, 20480 took
/// 7.4s, and 458752 took 213s; the cost per placement rose from 0.12ms to
/// 0.46ms across those samples. 8192 placements is a couple of seconds of one
/// compute worker in the shapes measured, and it admits eight plain words or
/// five pieces offering four candidates each. The piece and spelling bounds
/// stand apart from it because a short choice list can still carry long
/// pieces, and because recursion depth follows the pieces; no attested
/// compound approaches either figure.
#[requires(true)]
#[ensures(ret.max_placements == 8192)]
fn default_jvozba_limits() -> JvozbaBuildLimits {
    new!(JvozbaBuildLimits {
        max_pieces: 24,
        max_placements: 8192,
        max_spelling_letters: 256,
    })
}

#[bityzba::contract_trait]
pub(crate) trait ServerRequest {
    #[requires(true)]
    #[ensures(true)]
    fn validate(&self, policy: &ServerPolicy) -> Result<(), RequestLimitError>;
}

#[bityzba::contract_trait]
impl ServerRequest for jbotci_cli::ToolGentufaRequest {
    fn validate(&self, policy: &ServerPolicy) -> Result<(), RequestLimitError> {
        policy.validate_sources(std::iter::once(self.text.as_str()).chain(self.dialect.as_deref()))
    }
}
#[bityzba::contract_trait]
impl ServerRequest for jbotci_cli::ToolVlaseiRequest {
    fn validate(&self, policy: &ServerPolicy) -> Result<(), RequestLimitError> {
        policy.validate_sources(std::iter::once(self.text.as_str()).chain(self.dialect.as_deref()))
    }
}
#[bityzba::contract_trait]
impl ServerRequest for jbotci_cli::ToolCuktaRequest {
    fn validate(&self, policy: &ServerPolicy) -> Result<(), RequestLimitError> {
        policy.validate_sources(self.query.as_deref())?;
        if matches!(
            self.mode,
            jbotci_cli::ToolCuktaMode::Word | jbotci_cli::ToolCuktaMode::Meaning
        ) {
            policy.validate_count(
                self.count
                    .unwrap_or(jbotci_cll::DEFAULT_CUKTA_CLI_RESULT_COUNT),
            )
        } else {
            policy.validate_count(1)
        }
    }
}
#[bityzba::contract_trait]
impl ServerRequest for jbotci_cli::ToolVlackuRequest {
    fn validate(&self, policy: &ServerPolicy) -> Result<(), RequestLimitError> {
        policy.validate_sources(
            std::iter::once(self.query.as_str()).chain(self.word_types.iter().map(String::as_str)),
        )?;
        policy.validate_count(
            self.count
                .unwrap_or(jbotci_search::vlacku::DEFAULT_VLACKU_RESULT_COUNT),
        )
    }
}
#[bityzba::contract_trait]
impl ServerRequest for jbotci_cli::ToolJvozbaRequest {
    fn validate(&self, policy: &ServerPolicy) -> Result<(), RequestLimitError> {
        if self.parts.len() > policy.jvozba.max_pieces {
            return Err(RequestLimitError::CompoundParts);
        }
        policy.validate_sources(self.parts.iter().map(|part| part.value.as_str()))
    }
}
#[bityzba::contract_trait]
impl ServerRequest for jbotci_cli::ToolGimfihiRequest {
    fn validate(&self, policy: &ServerPolicy) -> Result<(), RequestLimitError> {
        policy.validate_sources(
            self.sources
                .iter()
                .flat_map(|source| [source.language.as_str(), source.word.as_str()])
                .chain(self.preset.as_deref())
                .chain(self.highlight.as_deref())
                .chain(self.shapes.iter().map(String::as_str)),
        )?;
        policy.validate_count(self.count.unwrap_or(jbotci_gimfihi::GIMFIHI_DEFAULT_COUNT))
    }
}

#[bityzba::contract_trait]
impl ServerRequest for crate::discord::request::DiscordRequest {
    fn validate(&self, policy: &ServerPolicy) -> Result<(), RequestLimitError> {
        policy.validate_sources(
            self.source_fields()
                .iter()
                .filter_map(|(_, source)| source.map(|source| source.as_str())),
        )?;
        use crate::discord::request::{CuktaMode, PAGE_SIZE};
        let count = match self {
            Self::Vlacku(_) => PAGE_SIZE + 1,
            Self::Cukta(request)
                if matches!(request.options.mode, CuktaMode::Word | CuktaMode::Meaning) =>
            {
                PAGE_SIZE + 1
            }
            Self::Gimfihi(request)
                if request
                    .sources
                    .as_ref()
                    .is_some_and(|source| !source.as_str().trim().is_empty()) =>
            {
                PAGE_SIZE + 1
            }
            _ => 1,
        };
        policy.validate_count(count)?;
        Ok(())
    }
}

#[bityzba::contract_trait]
impl ServerRequest for jbotci_web_core::WebRoute {
    fn validate(&self, policy: &ServerPolicy) -> Result<(), RequestLimitError> {
        use jbotci_web_core::CuktaWebView;
        match self {
            Self::Gentufa(state) => policy.validate_sources(
                std::iter::once(state.text.as_str()).chain(state.dialect.as_deref()),
            ),
            Self::Vlacku(state) => {
                policy.validate_sources(
                    std::iter::once(state.query.as_str())
                        .chain(state.word_types.iter().map(String::as_str)),
                )?;
                policy.validate_count(state.count)
            }
            Self::Cukta(state) => match &state.view {
                CuktaWebView::Search(state) => {
                    policy.validate_sources([state.query.as_str()])?;
                    policy.validate_count(state.count)
                }
                CuktaWebView::Section { reference } => {
                    policy.validate_sources([reference.as_str()])
                }
                CuktaWebView::Index => Ok(()),
            },
            Self::Gimfihi(state) => {
                policy.validate_sources(
                    state
                        .sources
                        .iter()
                        .flat_map(|source| [source.language.as_str(), source.word.as_str()])
                        .chain(
                            state
                                .sources
                                .iter()
                                .filter_map(|source| source.weight.as_deref()),
                        )
                        .chain(state.highlight.as_deref()),
                )?;
                policy.validate_count(state.count)
            }
            Self::Settings => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VARIABLES: &[&str] = &[
        "JBOTCI_COMPUTE_ACTIVE",
        "JBOTCI_COMPUTE_QUEUED",
        "JBOTCI_FETCH_ACTIVE",
        "JBOTCI_FETCH_QUEUED",
        "JBOTCI_SOURCE_FIELD_UNITS",
        "JBOTCI_SOURCE_TOTAL_UNITS",
        "JBOTCI_RESULT_ENTRIES",
        "JBOTCI_TEXT_BYTES",
        "JBOTCI_RESPONSE_BYTES",
        "JBOTCI_DEADLINE_MS",
        "JBOTCI_ERROR_DELIVERY_MS",
        "JBOTCI_JVOZBA_PIECES",
        "JBOTCI_JVOZBA_PLACEMENTS",
        "JBOTCI_JVOZBA_LETTERS",
        "JBOTCI_DIAGRAM_BLOCKS",
        "JBOTCI_DIAGRAM_COLUMNS",
        "JBOTCI_DIAGRAM_PIXELS",
        "JBOTCI_DIAGRAM_BYTES",
    ];

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn missing_variables_use_the_shared_defaults() {
        assert_eq!(
            ServerPolicy::from_lookup(|_| None).unwrap(),
            ServerPolicy::default()
        );
        let policy = ServerPolicy::default();
        assert_eq!(policy.work.compute_workers, 1);
        assert_eq!(policy.work.compute_queue, 8);
        assert_eq!(policy.work.fetch_workers, 4);
        assert_eq!(policy.work.fetch_queue, 8);
        assert_eq!(policy.result_entries, 512);
        assert_eq!(policy.text_bytes, 1_048_576);
        assert_eq!(policy.response_bytes, 12_582_912);
        assert_eq!(policy.deadline, Duration::from_secs(20));
        assert_eq!(policy.error_delivery, Duration::from_secs(1));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn every_limit_accepts_an_explicit_value_and_rejects_invalid_values() {
        for &variable in VARIABLES {
            let policy =
                ServerPolicy::from_lookup(|name| (name == variable).then(|| OsString::from("2")))
                    .unwrap();
            let observed = match variable {
                "JBOTCI_COMPUTE_ACTIVE" => policy.work.compute_workers,
                "JBOTCI_COMPUTE_QUEUED" => policy.work.compute_queue,
                "JBOTCI_FETCH_ACTIVE" => policy.work.fetch_workers,
                "JBOTCI_FETCH_QUEUED" => policy.work.fetch_queue,
                "JBOTCI_SOURCE_FIELD_UNITS" => policy.source_field_units,
                "JBOTCI_SOURCE_TOTAL_UNITS" => policy.source_total_units,
                "JBOTCI_RESULT_ENTRIES" => policy.result_entries,
                "JBOTCI_TEXT_BYTES" => policy.text_bytes,
                "JBOTCI_RESPONSE_BYTES" => policy.response_bytes,
                "JBOTCI_DEADLINE_MS" => policy.deadline.as_millis() as usize,
                "JBOTCI_ERROR_DELIVERY_MS" => policy.error_delivery.as_millis() as usize,
                "JBOTCI_JVOZBA_PIECES" => policy.jvozba.max_pieces,
                "JBOTCI_JVOZBA_PLACEMENTS" => policy.jvozba.max_placements as usize,
                "JBOTCI_JVOZBA_LETTERS" => policy.jvozba.max_spelling_letters,
                "JBOTCI_DIAGRAM_BLOCKS" => policy.diagram.max_blocks,
                "JBOTCI_DIAGRAM_COLUMNS" => policy.diagram.max_columns,
                "JBOTCI_DIAGRAM_PIXELS" => policy.diagram.max_pixels as usize,
                "JBOTCI_DIAGRAM_BYTES" => policy.diagram.max_bytes,
                _ => unreachable!(),
            };
            assert_eq!(observed, 2, "{variable}");
            for invalid in ["", "-1", "1.5", " 2", "two", "18446744073709551616"] {
                let error = ServerPolicy::from_lookup(|name| {
                    (name == variable).then(|| OsString::from(invalid))
                })
                .unwrap_err();
                assert_eq!(error.name(), variable);
                assert!(error.to_string().contains(variable));
            }
            let is_queue = variable.ends_with("_QUEUED");
            assert_eq!(
                ServerPolicy::from_lookup(|name| (name == variable).then(|| OsString::from("0")))
                    .is_ok(),
                is_queue,
                "{variable}"
            );
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn queues_accept_zero_and_representable_counts_and_compounds_require_two_pieces() {
        for name in ["JBOTCI_COMPUTE_QUEUED", "JBOTCI_FETCH_QUEUED"] {
            for count in [0, 1024, 1025, usize::MAX] {
                assert!(ServerPolicy::from_lookup(|key| (key == name).then(|| count.to_string().into())).is_ok());
            }
        }
        assert!(
            ServerPolicy::from_lookup(|key| (key == "JBOTCI_JVOZBA_PIECES").then(|| "1".into()))
                .is_err()
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn wide_limits_do_not_depend_on_the_pointer_size() {
        let policy = ServerPolicy::from_lookup(|name| match name {
            "JBOTCI_JVOZBA_PLACEMENTS" | "JBOTCI_DIAGRAM_PIXELS" => {
                Some(u64::MAX.to_string().into())
            }
            "JBOTCI_DEADLINE_MS" => Some("31536000000".into()),
            _ => None,
        })
        .unwrap();
        assert_eq!(policy.jvozba.max_placements, u64::MAX);
        assert_eq!(policy.diagram.max_pixels, u64::MAX);
        assert_eq!(policy.deadline, Duration::from_millis(31_536_000_000));
    }

    #[cfg(unix)]
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn non_unicode_environment_value_is_an_error() {
        use std::os::unix::ffi::OsStringExt;
        let error = ServerPolicy::from_lookup(|key| {
            (key == "JBOTCI_TEXT_BYTES").then(|| OsString::from_vec(vec![255]))
        })
        .unwrap_err();
        assert_eq!(error.name(), "JBOTCI_TEXT_BYTES");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn sources_count_utf16_before_normalization_and_share_a_total() {
        let policy =
            ServerPolicy::default().with_data(data! {source_field_units: 4, source_total_units: 5});
        assert!(policy.validate_sources(["😀😀", "a"]).is_ok());
        assert_eq!(
            policy.validate_sources(["😀😀a"]),
            Err(RequestLimitError::SourceField)
        );
        assert_eq!(
            policy.validate_sources(["😀😀", "aa"]),
            Err(RequestLimitError::SourceTotal)
        );
        assert!(policy.validate_count(512).is_ok());
        assert_eq!(
            policy.validate_count(0),
            Err(RequestLimitError::ResultCount)
        );
        assert_eq!(
            policy.validate_count(513),
            Err(RequestLimitError::ResultCount)
        );
    }
}
