//! In-process adapters for the production jbotci tool layer.

use std::fmt;

#[allow(unused_imports)]
use bityzba::{ensures, invariant, new, requires};
use jbotci_cli::{
    ToolCuktaRequest, ToolGentufaRequest, ToolJvozbaRequest, ToolRenderedOutput,
    ToolTersmuAnalysisData, ToolTersmuFormat, ToolTersmuRequest, ToolVlackuMode, ToolVlackuRequest,
    run_tool_cukta, run_tool_gentufa, run_tool_jvozba, run_tool_tersmu,
    run_tool_tersmu_with_analysis, run_tool_vlacku, tool_request_schema,
};
use jbotci_dialect::{DialectDefinition, DialectSettings, parse_dialect_selection_formula};
use jbotci_morphology::{
    MorphologyOptions, segment_words_with_modifiers_recovered_with_options_and_source_id_attempt,
};
use jbotci_syntax::{
    ParseOptions, SyntaxRecoveryParseData,
    parse_syntax_tree_with_recovery_with_source_and_options_attempt,
};
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{ExternalRendererCommand, TersmuFormat, ToolCall, ToolDefinition, ToolDefinitionError};

/// Typed result of gating one candidate through the production tersmu tool.
#[invariant(::ParseFailure { diagnostics_rendering, .. } => !diagnostics_rendering.is_empty())]
#[invariant(::Success { tersmu_rendering, renderer_incompatibilities } => !tersmu_rendering.is_empty() && renderer_incompatibilities.iter().all(|record| !record.trim().is_empty()))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GateOutcome {
    ParseFailure {
        /// Exact production diagnostics channel, without trimming or annotation.
        diagnostics_rendering: String,
        /// Parser phase determined from the same structured production parse path.
        category: DiagnosticCategory,
    },
    Success {
        /// Exact production tersmu stdout bytes, without trimming or rewrapping.
        tersmu_rendering: Vec<u8>,
        /// The declared compact-representation incompatibility records of the
        /// candidate's semantic graph (issue #721/#723), each in its exact
        /// `<INCOMPATIBILITY .../>` declaration form, computed from the same
        /// graph as the rendering. Empty when the graph renders compact.
        renderer_incompatibilities: Vec<String>,
    },
}

/// Coarse phase used by transcript summaries for rejected gate attempts.
#[invariant(::Morphology => true)]
#[invariant(::Syntax => true)]
#[invariant(::Other => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DiagnosticCategory {
    Morphology,
    Syntax,
    Other,
}

impl GateOutcome {
    /// Exact production diagnostics for a rejected candidate.
    #[requires(true)]
    #[ensures(ret.is_some() == matches!(self.as_data(), bityzba::data!(GateOutcome::ParseFailure { .. })))]
    pub fn diagnostics_rendering(&self) -> Option<&str> {
        match self.as_data() {
            bityzba::data!(GateOutcome::ParseFailure {
                diagnostics_rendering,
                ..
            }) => Some(diagnostics_rendering),
            _ => None,
        }
    }

    /// Structured failure phase for a rejected candidate.
    #[requires(true)]
    #[ensures(ret.is_some() == matches!(self.as_data(), bityzba::data!(GateOutcome::ParseFailure { .. })))]
    pub fn diagnostic_category(&self) -> Option<DiagnosticCategory> {
        match self.as_data() {
            bityzba::data!(GateOutcome::ParseFailure { category, .. }) => Some(*category),
            _ => None,
        }
    }

    /// Exact production tersmu bytes for an accepted candidate.
    #[requires(true)]
    #[ensures(ret.is_some() == matches!(self.as_data(), bityzba::data!(GateOutcome::Success { .. })))]
    pub fn tersmu_rendering(&self) -> Option<&[u8]> {
        match self.as_data() {
            bityzba::data!(GateOutcome::Success { tersmu_rendering, .. }) => Some(tersmu_rendering),
            _ => None,
        }
    }

    /// The declared renderer incompatibility records of an accepted candidate
    /// (issue #723), in their exact `<INCOMPATIBILITY .../>` declaration form.
    #[requires(true)]
    #[ensures(ret.is_some() == matches!(self.as_data(), bityzba::data!(GateOutcome::Success { .. })))]
    pub fn renderer_incompatibilities(&self) -> Option<&[String]> {
        match self.as_data() {
            bityzba::data!(GateOutcome::Success {
                renderer_incompatibilities,
                ..
            }) => Some(renderer_incompatibilities),
            _ => None,
        }
    }

    /// Whether the production tool classified this candidate as successful.
    #[requires(true)]
    #[ensures(ret == self.tersmu_rendering().is_some())]
    pub fn is_success(&self) -> bool {
        self.tersmu_rendering().is_some()
    }
}

/// Run the production semantic gate and classify only from its structured status.
#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
pub fn gate_lojban(
    text: String,
    format: Option<TersmuFormat>,
    dialect: Option<String>,
) -> Result<GateOutcome, GateError> {
    let format = format.unwrap_or_default();
    let request = ToolTersmuRequest {
        text: text.clone(),
        format: tool_tersmu_format(&format),
        dialect: dialect.clone(),
        show_defs: true,
        story_time: false,
        indent: None,
    };
    let analysis = run_tool_tersmu_with_analysis(request).map_err(|error| {
        new!(GateError::ToolExecution {
            message: error.to_string(),
        })
    })?;
    let bityzba::data!(ToolTersmuAnalysis {
        rendered: output,
        compact_incompatibilities,
    }) = analysis.into_data();
    if output.status.is_success() {
        if output.stdout.is_empty() {
            return Err(new!(GateError::InvalidToolOutput {
                message: "successful tersmu tool output was empty".to_owned(),
            }));
        }
        let tersmu_rendering = match &format {
            TersmuFormat::External(command) => run_external_renderer(command, &output.stdout)?,
            TersmuFormat::Json | TersmuFormat::Smusni | TersmuFormat::Xml => output.stdout,
        };
        return Ok(new!(GateOutcome::Success {
            tersmu_rendering,
            renderer_incompatibilities: compact_incompatibilities,
        }));
    }
    if output.stderr.is_empty() {
        return Err(new!(GateError::InvalidToolOutput {
            message: format!(
                "failed tersmu tool output had status {:?} but no diagnostics",
                output.status
            ),
        }));
    }
    let category = classify_gate_failure(&text, dialect.as_deref())?;
    Ok(new!(GateOutcome::ParseFailure {
        diagnostics_rendering: output.stderr,
        category,
    }))
}

/// Re-run only the structured morphology/syntax classification used by tersmu.
///
/// This deliberately does not inspect rendered diagnostic text. A failure after
/// valid morphology and syntax is classified as `Other` (for example, a semantic
/// graph construction failure).
#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn classify_gate_failure(
    text: &str,
    dialect: Option<&str>,
) -> Result<DiagnosticCategory, GateError> {
    let dialect = dialect
        .map_or_else(
            || Ok(DialectDefinition::default()),
            |source| parse_dialect_selection_formula(&DialectSettings::default(), source),
        )
        .map_err(|error| {
            new!(GateError::ToolExecution {
                message: error.to_string(),
            })
        })?;
    let morphology_options = MorphologyOptions::default().with_dialect_definition(&dialect);
    let morphology_attempt =
        segment_words_with_modifiers_recovered_with_options_and_source_id_attempt(
            text,
            &morphology_options,
            None,
        );
    let morphology = &morphology_attempt.result;
    if !morphology.errors.is_empty() {
        return Ok(DiagnosticCategory::Morphology);
    }
    let syntax_attempt = parse_syntax_tree_with_recovery_with_source_and_options_attempt(
        &morphology.words,
        text,
        &ParseOptions::default().with_dialect_definition(&dialect),
    );
    let syntax = &syntax_attempt.result;
    Ok(match syntax.as_data() {
        bityzba::data!(SyntaxRecoveryParse::Recovered { .. }) => DiagnosticCategory::Syntax,
        bityzba::data!(SyntaxRecoveryParse::Valid { .. }) => DiagnosticCategory::Other,
    })
}

#[requires(true)]
#[ensures(true)]
fn tool_tersmu_format(format: &TersmuFormat) -> ToolTersmuFormat {
    match format {
        TersmuFormat::Json => ToolTersmuFormat::Json,
        TersmuFormat::Smusni => ToolTersmuFormat::Smusni,
        TersmuFormat::Xml => ToolTersmuFormat::Xml,
        TersmuFormat::External(_) => ToolTersmuFormat::Json,
    }
}

/// Pipe a production tersmu JSON graph through an external renderer.
#[requires(!tersmu_json.is_empty())]
#[ensures(ret.as_ref().is_ok_and(|rendering| !rendering.is_empty()) || ret.is_err())]
fn run_external_renderer(
    command: &ExternalRendererCommand,
    tersmu_json: &[u8],
) -> Result<Vec<u8>, GateError> {
    use std::io::Write as _;
    use std::process::{Command, Stdio};

    let renderer_error = |message: String| {
        new!(GateError::ExternalRenderer {
            command: command.program().to_owned(),
            message,
        })
    };
    let mut child = Command::new(command.program())
        .args(command.args())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| renderer_error(format!("failed to spawn renderer: {error}")))?;

    // Drain stdout/stderr concurrently with stdin. A renderer is allowed to
    // emit more than one pipe buffer before consuming the complete graph.
    let mut stdin = child
        .stdin
        .take()
        .expect("stdin was requested as piped at spawn");
    let payload = tersmu_json.to_vec();
    let stdin_writer = std::thread::spawn(move || stdin.write_all(&payload));
    let output = child
        .wait_with_output()
        .map_err(|error| renderer_error(format!("failed to collect renderer output: {error}")))?;
    let stdin_result = stdin_writer
        .join()
        .map_err(|_| renderer_error("stdin writer thread panicked".to_owned()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let message = match stdin_result {
            Ok(()) => format!("exited with {}: {stderr}", output.status),
            Err(error) => format!(
                "exited with {}, and writing its stdin also failed: {error}: {stderr}",
                output.status
            ),
        };
        return Err(renderer_error(message));
    }
    // A successful renderer may intentionally stop reading after a JSON
    // prefix, so a broken stdin pipe alone is not an infrastructure failure.
    if output.stdout.is_empty() {
        return Err(renderer_error(
            "exited successfully but produced no output on stdout".to_owned(),
        ));
    }
    Ok(output.stdout)
}

/// The production tersmu entry point failed before returning structured output.
///
/// `ExternalRenderer` carries a non-trivial invariant, so (unlike the other
/// two variants, which are structurally unconstrained) this type cannot use
/// `#[derive(thiserror::Error)]`: bityzba's validated-wrapper codegen for a
/// type with a real invariant relocates the raw variant shape, which strips
/// derive-time access to per-variant `#[error(...)]` attributes. `Display`
/// and `Error` are implemented by hand instead, following the same pattern
/// as `ProtocolRunError`.
#[invariant(true)]
#[invariant(::ToolExecution { .. } => true)]
#[invariant(::InvalidToolOutput { .. } => true)]
#[invariant(::ExternalRenderer { command, message } => !command.trim().is_empty() && !message.trim().is_empty())]
#[derive(Debug, PartialEq, Eq)]
pub enum GateError {
    ToolExecution { message: String },
    InvalidToolOutput { message: String },
    ExternalRenderer { command: String, message: String },
}

impl fmt::Display for GateError {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.as_data() {
            bityzba::data!(GateError::ToolExecution { message }) => {
                write!(formatter, "jbotci tersmu execution failed: {message}")
            }
            bityzba::data!(GateError::InvalidToolOutput { message }) => write!(
                formatter,
                "invalid structured output from jbotci tersmu: {message}"
            ),
            bityzba::data!(GateError::ExternalRenderer { command, message }) => write!(
                formatter,
                "external tersmu renderer `{command}` failed: {message}"
            ),
        }
    }
}

impl std::error::Error for GateError {}

/// Stateless adapter exposing the production reference tools and their schemas.
#[invariant(true)]
#[derive(Debug, Clone, Copy, Default)]
pub struct ReferenceTools;

/// xarsnu-local tersmu request. The production MCP and REST surfaces default to
/// XML, while this reference-tool surface retains its established smusni
/// default.
#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
struct XarsnuTersmuRequest {
    /// The Lojban text to interpret.
    text: String,
    /// How to render the graph. Defaults to the model-facing `smusni`
    /// declaration notation on xarsnu. Use `xml` for canonical scoped SFN-XML
    /// or `json` for the canonical interchange graph.
    #[serde(default = "xarsnu_tersmu_format_default")]
    format: ToolTersmuFormat,
    /// Optional dialect selector: a builtin dialect name (e.g. `zantufa`,
    /// `gadganzu`, `ce-ki-tau`) or a parenthesized formula combining them.
    #[serde(default)]
    dialect: Option<String>,
    /// Prepend dictionary definitions for content words. Definitions are on by
    /// default and are suppressed for JSON.
    #[serde(default = "xarsnu_show_defs_default")]
    show_defs: bool,
    /// Carry tense forward across sentences as an advancing narrative story
    /// time. Off by default.
    #[serde(default)]
    story_time: bool,
    /// Spaces per indent level. Defaults to 2; set 0 for compact JSON.
    #[serde(default)]
    indent: Option<usize>,
}

impl From<XarsnuTersmuRequest> for ToolTersmuRequest {
    #[requires(true)]
    #[ensures(ret.format == value.format)]
    fn from(value: XarsnuTersmuRequest) -> Self {
        Self {
            text: value.text,
            format: value.format,
            dialect: value.dialect,
            show_defs: value.show_defs,
            story_time: value.story_time,
            indent: value.indent,
        }
    }
}

#[requires(true)]
#[ensures(ret == ToolTersmuFormat::Smusni)]
fn xarsnu_tersmu_format_default() -> ToolTersmuFormat {
    ToolTersmuFormat::Smusni
}

#[requires(true)]
#[ensures(ret)]
fn xarsnu_show_defs_default() -> bool {
    true
}

impl ReferenceTools {
    /// Model-facing definitions generated from each reference-tool surface
    /// request type.
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|tools| tools.len() == 5) || ret.is_err())]
    pub fn definitions() -> Result<Vec<ToolDefinition>, ToolDefinitionError> {
        Ok(vec![
            ToolDefinition::new(
                "vlacku".to_owned(),
                "Look up Lojban dictionary entries and lujvo decomposition.".to_owned(),
                tool_request_schema::<ToolVlackuRequest>(),
            )?,
            ToolDefinition::new(
                "gentufa".to_owned(),
                "Parse Lojban text into the production syntax representation.".to_owned(),
                tool_request_schema::<ToolGentufaRequest>(),
            )?,
            ToolDefinition::new(
                "tersmu".to_owned(),
                "Compute the production semantic representation of Lojban text.".to_owned(),
                tool_request_schema::<XarsnuTersmuRequest>(),
            )?,
            ToolDefinition::new(
                "jvozba".to_owned(),
                "Build a Lojban compound word from source words or fixed rafsi.".to_owned(),
                tool_request_schema::<ToolJvozbaRequest>(),
            )?,
            ToolDefinition::new(
                "cukta".to_owned(),
                "Read or search The Complete Lojban Language reference book. Concept and meaning queries are supported and are often the right choice for grammar questions; a search need not name a known word or section."
                    .to_owned(),
                tool_request_schema::<ToolCuktaRequest>(),
            )?,
        ])
    }

    /// Dispatch one model call through the same production entry point as MCP.
    ///
    /// The returned structured output is untouched: status, stdout bytes,
    /// stderr text, and content type are exactly those returned by jbotci.
    #[requires(true)]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    pub fn dispatch(call: &ToolCall) -> Result<ToolRenderedOutput, ReferenceToolError> {
        match call.function.name.as_str() {
            "vlacku" => {
                let request = decode_request::<ToolVlackuRequest>(call)?;
                run_tool_vlacku(request).map_err(|error| execution_error(call, error.to_string()))
            }
            "gentufa" => {
                let request = decode_request::<ToolGentufaRequest>(call)?;
                run_tool_gentufa(request).map_err(|error| execution_error(call, error.to_string()))
            }
            "tersmu" => {
                let request = decode_request::<XarsnuTersmuRequest>(call)?;
                run_tool_tersmu(request.into())
                    .map_err(|error| execution_error(call, error.to_string()))
            }
            "jvozba" => {
                let request = decode_request::<ToolJvozbaRequest>(call)?;
                run_tool_jvozba(request).map_err(|error| execution_error(call, error.to_string()))
            }
            "cukta" => {
                let request = decode_request::<ToolCuktaRequest>(call)?;
                run_tool_cukta(request).map_err(|error| execution_error(call, error.to_string()))
            }
            name => Err(ReferenceToolError::UnknownTool {
                name: name.to_owned(),
            }),
        }
    }
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn decode_request<T: DeserializeOwned>(call: &ToolCall) -> Result<T, ReferenceToolError> {
    serde_json::from_str(&call.function.arguments).map_err(|error| {
        ReferenceToolError::InvalidArguments {
            tool_name: call.function.name.clone(),
            message: error.to_string(),
        }
    })
}

#[requires(!message.trim().is_empty())]
#[ensures(matches!(ret, ReferenceToolError::ToolExecution { .. }))]
fn execution_error(call: &ToolCall, message: String) -> ReferenceToolError {
    ReferenceToolError::ToolExecution {
        tool_name: call.function.name.clone(),
        message,
    }
}

/// A reference-tool call could not be decoded, dispatched, or executed.
#[invariant(true)]
#[invariant(::UnknownTool { .. } => true)]
#[invariant(::InvalidArguments { .. } => true)]
#[invariant(::ToolExecution { .. } => true)]
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ReferenceToolError {
    #[error("unknown jbotci reference tool `{name}`")]
    UnknownTool { name: String },
    #[error("invalid arguments for jbotci reference tool `{tool_name}`: {message}")]
    InvalidArguments { tool_name: String, message: String },
    #[error("jbotci reference tool `{tool_name}` failed: {message}")]
    ToolExecution { tool_name: String, message: String },
}

/// Prove that the production semantic dictionary path can load and query its assets.
#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
pub fn preflight_embedding_search() -> Result<(), EmbeddingSearchPreflightError> {
    let output = run_tool_vlacku(ToolVlackuRequest {
        mode: ToolVlackuMode::Meaning,
        query: "movement from one place to another".to_owned(),
        count: Some(1),
        word_types: Vec::new(),
        min_votes: None,
        min_similarity: None,
        decompose_lujvo: false,
        show_etymology: false,
    })
    .map_err(|error| {
        new!(EmbeddingSearchPreflightError::ToolExecution {
            message: error.to_string(),
        })
    })?;
    if output.status.is_success() {
        return Ok(());
    }
    let mut diagnostic = String::from_utf8_lossy(&output.stdout).into_owned();
    diagnostic.push_str(&output.stderr);
    if diagnostic.trim().is_empty() {
        diagnostic = format!("semantic vlacku returned status {:?}", output.status);
    }
    Err(EmbeddingSearchPreflightError::unavailable(diagnostic))
}

/// Semantic search could not complete the real production preflight query.
#[invariant(::ToolExecution { message } => !message.trim().is_empty())]
#[invariant(::Unavailable { diagnostic } => !diagnostic.trim().is_empty())]
#[derive(Debug, PartialEq, Eq)]
pub enum EmbeddingSearchPreflightError {
    ToolExecution { message: String },
    Unavailable { diagnostic: String },
}

impl EmbeddingSearchPreflightError {
    /// Construct an unavailable-assets failure from a production diagnostic.
    #[requires(!diagnostic.trim().is_empty())]
    #[ensures(matches!(ret.as_data(), bityzba::data!(EmbeddingSearchPreflightError::Unavailable { .. })))]
    pub fn unavailable(diagnostic: String) -> Self {
        new!(EmbeddingSearchPreflightError::Unavailable { diagnostic })
    }
}

impl std::fmt::Display for EmbeddingSearchPreflightError {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.as_data() {
            bityzba::data!(EmbeddingSearchPreflightError::ToolExecution { message }) => {
                write!(
                    formatter,
                    "semantic vlacku preflight failed to execute: {message}"
                )
            }
            bityzba::data!(EmbeddingSearchPreflightError::Unavailable { diagnostic }) => {
                write!(
                    formatter,
                    "semantic vlacku preflight was unavailable: {diagnostic}"
                )
            }
        }
    }
}

impl std::error::Error for EmbeddingSearchPreflightError {}

#[cfg(test)]
mod tests {
    use super::*;
    use jbotci_cli::ToolStatus;
    use serde_json::{Value, json};

    // This private table row has no invalid structural combinations; each test
    // asserts the behavior its particular string values are meant to exercise.
    #[invariant(true)]
    #[derive(Debug, Clone, Copy)]
    struct GateFixture {
        name: &'static str,
        text: &'static str,
    }

    const GATE_FIXTURES: [GateFixture; 5] = [
        GateFixture {
            name: "valid",
            text: "mi klama",
        },
        GateFixture {
            name: "morphology-error",
            text: "mi @ klama",
        },
        GateFixture {
            name: "syntax-error",
            text: "mi cu",
        },
        GateFixture {
            name: "multi-error",
            text: "mi cu i do cu",
        },
        GateFixture {
            name: "quantifier-scope-and-abstraction",
            text: "ro lo prenu cu djuno lo du'u su'o lo gerku cu prami ri",
        },
    ];

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gate_payloads_are_byte_identical_to_direct_production_calls() {
        let mut saw_trailing_newline = false;
        let mut saw_long_line = false;
        for fixture in GATE_FIXTURES {
            let request = tersmu_request(fixture.text);
            let direct = run_tool_tersmu(request).expect("direct production tool call");
            let gated = gate_lojban(fixture.text.to_owned(), None, None)
                .unwrap_or_else(|error| panic!("{} gate failed: {error}", fixture.name));
            if direct.status.is_success() {
                let actual = gated
                    .tersmu_rendering()
                    .unwrap_or_else(|| panic!("{} should succeed", fixture.name));
                assert_eq!(actual, direct.stdout.as_slice(), "{} stdout", fixture.name);
                saw_trailing_newline |= actual.ends_with(b"\n");
                saw_long_line |= actual
                    .split(|byte| *byte == b'\n')
                    .any(|line| line.len() > 120);
            } else {
                let actual = gated
                    .diagnostics_rendering()
                    .unwrap_or_else(|| panic!("{} should fail", fixture.name));
                assert_eq!(
                    actual.as_bytes(),
                    direct.stderr.as_bytes(),
                    "{} stderr",
                    fixture.name
                );
                saw_trailing_newline |= actual.ends_with('\n');
                saw_long_line |= actual.lines().any(|line| line.len() > 120);
            }
        }
        assert!(saw_trailing_newline, "fixtures must catch trimming");
        assert!(saw_long_line, "fixtures must catch line rewrapping");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gate_classification_matches_structured_tool_status() {
        for fixture in GATE_FIXTURES {
            let direct =
                run_tool_tersmu(tersmu_request(fixture.text)).expect("direct production tool call");
            let gated = gate_lojban(fixture.text.to_owned(), None, None).expect("gate call");
            assert_eq!(
                gated.is_success(),
                direct.status == ToolStatus::Success,
                "{} classification",
                fixture.name
            );
        }
        assert_eq!(
            gate_lojban("mi @ klama".to_owned(), None, None)
                .expect("morphology failure")
                .diagnostic_category(),
            Some(DiagnosticCategory::Morphology)
        );
        assert_eq!(
            gate_lojban("mi cu".to_owned(), None, None)
                .expect("syntax failure")
                .diagnostic_category(),
            Some(DiagnosticCategory::Syntax)
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn reference_definitions_use_each_surface_request_schema() {
        let definitions = ReferenceTools::definitions().expect("valid definitions");
        assert_schema(
            &definitions,
            "vlacku",
            tool_request_schema::<ToolVlackuRequest>(),
        );
        assert_schema(
            &definitions,
            "gentufa",
            tool_request_schema::<ToolGentufaRequest>(),
        );
        assert_schema(
            &definitions,
            "tersmu",
            tool_request_schema::<XarsnuTersmuRequest>(),
        );
        assert_schema(
            &definitions,
            "jvozba",
            tool_request_schema::<ToolJvozbaRequest>(),
        );
        assert_schema(
            &definitions,
            "cukta",
            tool_request_schema::<ToolCuktaRequest>(),
        );
        let cukta = definitions
            .iter()
            .find(|definition| definition.name() == "cukta")
            .expect("cukta definition");
        assert!(
            cukta
                .function
                .description
                .contains("Concept and meaning queries")
        );
        assert!(
            cukta
                .function
                .description
                .contains("often the right choice for grammar questions")
        );
        let tersmu = definitions
            .iter()
            .find(|definition| definition.name() == "tersmu")
            .expect("tersmu definition");
        let format = &tersmu.function.parameters["properties"]["format"];
        assert_eq!(
            format["default"],
            json!("smusni"),
            "xarsnu's published schema must retain its surface-local default"
        );
        assert!(
            format["description"]
                .as_str()
                .is_some_and(|description| description.contains("on xarsnu")),
            "{format}"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn xarsnu_tersmu_defaults_to_smusni_and_preserves_explicit_xml() {
        let omitted = tool_call("tersmu", json!({ "text": "mi klama" }));
        let omitted_output = ReferenceTools::dispatch(&omitted).expect("omitted format");
        let expected_smusni = run_tool_tersmu(ToolTersmuRequest {
            text: "mi klama".to_owned(),
            format: ToolTersmuFormat::Smusni,
            dialect: None,
            show_defs: true,
            story_time: false,
            indent: None,
        })
        .expect("direct smusni");
        assert_eq!(omitted_output, expected_smusni);
        assert_eq!(
            omitted_output.content_type.as_deref(),
            Some("text/plain; charset=utf-8")
        );

        let explicit_xml = tool_call(
            "tersmu",
            json!({
                "text": "mi klama",
                "format": "xml",
                "show-defs": false
            }),
        );
        let xml_output = ReferenceTools::dispatch(&explicit_xml).expect("explicit XML");
        let expected_xml = run_tool_tersmu(ToolTersmuRequest {
            text: "mi klama".to_owned(),
            format: ToolTersmuFormat::Xml,
            dialect: None,
            show_defs: false,
            story_time: false,
            indent: None,
        })
        .expect("direct XML");
        assert_eq!(xml_output, expected_xml);
        // jbotci#719: the document opens with the KEY teaching comment.
        assert!(xml_output.stdout.starts_with(b"<!--\n"));
        assert_eq!(
            xml_output.content_type.as_deref(),
            Some("application/xml; charset=utf-8")
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gate_format_and_dialect_knobs_are_forwarded() {
        let default = gate_lojban("mi klama".to_owned(), None, None)
            .expect("default gate")
            .tersmu_rendering()
            .expect("default success")
            .to_owned();
        let json = gate_lojban("mi klama".to_owned(), Some(TersmuFormat::Json), None)
            .expect("JSON gate")
            .tersmu_rendering()
            .expect("JSON success")
            .to_owned();
        let direct_json = run_tool_tersmu(ToolTersmuRequest {
            text: "mi klama".to_owned(),
            format: ToolTersmuFormat::Json,
            dialect: None,
            show_defs: true,
            story_time: false,
            indent: None,
        })
        .expect("direct JSON call");
        assert_ne!(json, default, "format selection must not be inert");
        assert_eq!(json, direct_json.stdout);

        let smusni = gate_lojban("mi klama".to_owned(), Some(TersmuFormat::Smusni), None)
            .expect("smusni gate")
            .tersmu_rendering()
            .expect("smusni success")
            .to_owned();
        let direct_smusni = run_tool_tersmu(ToolTersmuRequest {
            text: "mi klama".to_owned(),
            format: ToolTersmuFormat::Smusni,
            dialect: None,
            show_defs: true,
            story_time: false,
            indent: None,
        })
        .expect("direct smusni call");
        assert_eq!(smusni, default, "smusni is now the default rendering");
        assert_ne!(smusni, json, "smusni is a distinct rendering from json");
        assert_eq!(smusni, direct_smusni.stdout);

        let xml = gate_lojban("mi klama".to_owned(), Some(TersmuFormat::Xml), None)
            .expect("XML gate")
            .tersmu_rendering()
            .expect("XML success")
            .to_owned();
        let direct_xml = run_tool_tersmu(ToolTersmuRequest {
            text: "mi klama".to_owned(),
            format: ToolTersmuFormat::Xml,
            dialect: None,
            show_defs: true,
            story_time: false,
            indent: None,
        })
        .expect("direct XML call");
        assert_eq!(xml, direct_xml.stdout);
        assert_ne!(xml, json, "xml is a distinct rendering from json");
        assert_ne!(xml, smusni, "xml is a distinct rendering from smusni");
        // The gate always requests show-defs, whose definitions preamble
        // precedes the XML document, so the canonical `<SFN` root follows the
        // definitions rather than opening the payload.
        assert!(
            xml.windows(b"<SFN ".len())
                .any(|window| window == b"<SFN "),
            "xml gate must contain the canonical scoped SFN-XML rendering"
        );

        let invalid_dialect = "(definitely-not-a-jbotci-dialect)".to_owned();
        let direct_error = run_tool_tersmu(ToolTersmuRequest {
            text: "mi klama".to_owned(),
            format: ToolTersmuFormat::Smusni,
            dialect: Some(invalid_dialect.clone()),
            show_defs: true,
            story_time: false,
            indent: None,
        })
        .expect_err("direct invalid dialect must fail")
        .to_string();
        let gate_error = gate_lojban("mi klama".to_owned(), None, Some(invalid_dialect))
            .expect_err("gate must forward the invalid dialect");
        assert_eq!(
            gate_error,
            new!(GateError::ToolExecution {
                message: direct_error
            })
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn external_renderer_receives_json_on_stdin_and_returns_stdout() {
        let command = new!(ExternalRendererCommand(vec![
            "sh".to_owned(),
            "-c".to_owned(),
            "printf 'stub:'; cat".to_owned(),
        ]));
        let rendered = gate_lojban(
            "mi klama".to_owned(),
            Some(TersmuFormat::External(command)),
            None,
        )
        .expect("external renderer gate")
        .tersmu_rendering()
        .expect("successful rendering")
        .to_owned();
        let direct_json = run_tool_tersmu(ToolTersmuRequest {
            text: "mi klama".to_owned(),
            format: ToolTersmuFormat::Json,
            dialect: None,
            show_defs: true,
            story_time: false,
            indent: None,
        })
        .expect("direct JSON rendering");
        let mut expected = b"stub:".to_vec();
        expected.extend_from_slice(&direct_json.stdout);
        assert_eq!(rendered, expected);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn external_renderer_nonzero_exit_propagates_as_gate_error() {
        let command = new!(ExternalRendererCommand(vec![
            "sh".to_owned(),
            "-c".to_owned(),
            "cat >/dev/null; printf 'stub failure' >&2; exit 23".to_owned(),
        ]));
        let error = gate_lojban(
            "mi klama".to_owned(),
            Some(TersmuFormat::External(command)),
            None,
        )
        .expect_err("renderer failure must be loud");
        match error.as_data() {
            bityzba::data!(GateError::ExternalRenderer { command, message }) => {
                assert_eq!(command, "sh");
                assert!(message.contains("23"), "{message}");
                assert!(message.contains("stub failure"), "{message}");
            }
            other => panic!("expected external renderer gate error, got {other:?}"),
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn external_renderer_spawn_failure_is_a_loud_gate_error() {
        let missing = TersmuFormat::External(new!(ExternalRendererCommand(vec![
            "/definitely/not/a/real/executable-q4-external-test".to_owned(),
        ])));
        let error = gate_lojban("mi klama".to_owned(), Some(missing), None)
            .expect_err("a renderer that cannot be spawned must surface as a gate error");
        assert!(matches!(
            error.as_data(),
            bityzba::data!(GateError::ExternalRenderer { .. })
        ));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn external_renderer_is_never_invoked_for_a_rejected_candidate() {
        // "false" would surface as a GateError::ExternalRenderer if it were
        // ever spawned; a candidate that fails to parse must still resolve
        // to a normal ParseFailure GateOutcome without the gate call itself
        // erroring, proving the external renderer only runs on success.
        let would_fail_loudly_if_invoked =
            TersmuFormat::External(new!(ExternalRendererCommand(vec!["false".to_owned()])));
        let outcome = gate_lojban("mi cu".to_owned(), Some(would_fail_loudly_if_invoked), None)
            .expect("a rejected candidate must not fail the gate call itself");
        assert!(outcome.diagnostics_rendering().is_some());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn every_reference_adapter_returns_unmodified_production_output() {
        let vlacku = tool_call("vlacku", json!({ "query": "klama" }));
        assert_eq!(
            ReferenceTools::dispatch(&vlacku).expect("adapter vlacku"),
            run_tool_vlacku(decode_request(&vlacku).expect("vlacku request"))
                .expect("direct vlacku")
        );

        let gentufa = tool_call("gentufa", json!({ "text": "mi klama" }));
        assert_eq!(
            ReferenceTools::dispatch(&gentufa).expect("adapter gentufa"),
            run_tool_gentufa(decode_request(&gentufa).expect("gentufa request"))
                .expect("direct gentufa")
        );

        let tersmu = tool_call("tersmu", json!({ "text": "mi klama" }));
        let adapted = ReferenceTools::dispatch(&tersmu).expect("adapter tersmu");
        let request: XarsnuTersmuRequest = decode_request(&tersmu).expect("xarsnu tersmu request");
        let direct = run_tool_tersmu(request.into()).expect("direct tersmu");
        assert!(direct.stdout.ends_with(b"\n"), "fixture catches trimming");
        assert_eq!(adapted, direct);

        let jvozba = tool_call(
            "jvozba",
            json!({
                "parts": [
                    { "kind": "word", "value": "xanri" },
                    { "kind": "word", "value": "casnu" }
                ]
            }),
        );
        assert_eq!(
            ReferenceTools::dispatch(&jvozba).expect("adapter jvozba"),
            run_tool_jvozba(decode_request(&jvozba).expect("jvozba request"))
                .expect("direct jvozba")
        );

        let cukta = tool_call("cukta", json!({ "mode": "section", "query": "11.7" }));
        assert_eq!(
            ReferenceTools::dispatch(&cukta).expect("adapter cukta"),
            run_tool_cukta(decode_request(&cukta).expect("cukta request")).expect("direct cukta")
        );
    }

    #[requires(!text.trim().is_empty())]
    #[ensures(ret.text == text)]
    fn tersmu_request(text: &str) -> ToolTersmuRequest {
        ToolTersmuRequest {
            text: text.to_owned(),
            format: ToolTersmuFormat::Smusni,
            dialect: None,
            show_defs: true,
            story_time: false,
            indent: None,
        }
    }

    #[requires(!name.trim().is_empty())]
    #[requires(arguments.is_object())]
    #[ensures(ret.function.name == name)]
    fn tool_call(name: &str, arguments: Value) -> ToolCall {
        serde_json::from_value(json!({
            "id": format!("call-{name}"),
            "type": "function",
            "function": {
                "name": name,
                "arguments": arguments.to_string()
            }
        }))
        .expect("valid test tool call")
    }

    #[requires(!name.trim().is_empty())]
    #[ensures(true)]
    fn assert_schema(definitions: &[ToolDefinition], name: &str, expected: Value) {
        let definition = definitions
            .iter()
            .find(|definition| definition.name() == name)
            .unwrap_or_else(|| panic!("missing {name} definition"));
        assert_eq!(definition.function.parameters, expected);
    }
}
