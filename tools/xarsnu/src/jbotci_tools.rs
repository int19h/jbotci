//! In-process adapters for the production jbotci tool layer.

#[allow(unused_imports)]
use bityzba::{ensures, invariant, new, requires};
use jbotci_cli::{
    ToolCuktaRequest, ToolGentufaRequest, ToolJvozbaRequest, ToolRenderedOutput, ToolTersmuFormat,
    ToolTersmuRequest, ToolVlackuRequest, run_tool_cukta, run_tool_gentufa, run_tool_jvozba,
    run_tool_tersmu, run_tool_vlacku,
};
use schemars::transform::{Transform, transform_subschemas};
use schemars::{JsonSchema, Schema};
use serde::de::DeserializeOwned;
use serde_json::Value;
use thiserror::Error;

use crate::{TersmuFormat, ToolCall, ToolDefinition, ToolDefinitionError};

/// Typed result of gating one candidate through the production tersmu tool.
#[invariant(::ParseFailure { diagnostics_rendering } => !diagnostics_rendering.is_empty())]
#[invariant(::Success { tersmu_rendering } => !tersmu_rendering.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GateOutcome {
    ParseFailure {
        /// Exact production diagnostics channel, without trimming or annotation.
        diagnostics_rendering: String,
    },
    Success {
        /// Exact production tersmu stdout bytes, without trimming or rewrapping.
        tersmu_rendering: Vec<u8>,
    },
}

impl GateOutcome {
    /// Exact production diagnostics for a rejected candidate.
    #[requires(true)]
    #[ensures(ret.is_some() == matches!(self.as_data(), bityzba::data!(GateOutcome::ParseFailure { .. })))]
    pub fn diagnostics_rendering(&self) -> Option<&str> {
        match self.as_data() {
            bityzba::data!(GateOutcome::ParseFailure {
                diagnostics_rendering,
            }) => Some(diagnostics_rendering),
            _ => None,
        }
    }

    /// Exact production tersmu bytes for an accepted candidate.
    #[requires(true)]
    #[ensures(ret.is_some() == matches!(self.as_data(), bityzba::data!(GateOutcome::Success { .. })))]
    pub fn tersmu_rendering(&self) -> Option<&[u8]> {
        match self.as_data() {
            bityzba::data!(GateOutcome::Success { tersmu_rendering }) => Some(tersmu_rendering),
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
    let request = ToolTersmuRequest {
        text,
        format: tool_tersmu_format(format.unwrap_or_default()),
        dialect,
        story_time: false,
        indent: None,
    };
    let output = run_tool_tersmu(request).map_err(|error| GateError::ToolExecution {
        message: error.to_string(),
    })?;
    if output.status.is_success() {
        if output.stdout.is_empty() {
            return Err(GateError::InvalidToolOutput {
                message: "successful tersmu tool output was empty".to_owned(),
            });
        }
        return Ok(new!(GateOutcome::Success {
            tersmu_rendering: output.stdout,
        }));
    }
    if output.stderr.is_empty() {
        return Err(GateError::InvalidToolOutput {
            message: format!(
                "failed tersmu tool output had status {:?} but no diagnostics",
                output.status
            ),
        });
    }
    Ok(new!(GateOutcome::ParseFailure {
        diagnostics_rendering: output.stderr,
    }))
}

#[requires(true)]
#[ensures(true)]
fn tool_tersmu_format(format: TersmuFormat) -> ToolTersmuFormat {
    match format {
        TersmuFormat::TreeProj => ToolTersmuFormat::TreeProj,
        TersmuFormat::Tree => ToolTersmuFormat::Tree,
        TersmuFormat::Json => ToolTersmuFormat::Json,
    }
}

/// The production tersmu entry point failed before returning structured output.
#[invariant(true)]
#[invariant(::ToolExecution { .. } => true)]
#[invariant(::InvalidToolOutput { .. } => true)]
#[derive(Debug, Error, PartialEq, Eq)]
pub enum GateError {
    #[error("jbotci tersmu execution failed: {message}")]
    ToolExecution { message: String },
    #[error("invalid structured output from jbotci tersmu: {message}")]
    InvalidToolOutput { message: String },
}

/// Stateless adapter exposing the production reference tools and their schemas.
#[invariant(true)]
#[derive(Debug, Clone, Copy, Default)]
pub struct ReferenceTools;

impl ReferenceTools {
    /// Model-facing definitions generated directly from the production request types.
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|tools| tools.len() == 5) || ret.is_err())]
    pub fn definitions() -> Result<Vec<ToolDefinition>, ToolDefinitionError> {
        Ok(vec![
            ToolDefinition::new(
                "vlacku".to_owned(),
                "Look up Lojban dictionary entries and lujvo decomposition.".to_owned(),
                request_schema::<ToolVlackuRequest>(),
            )?,
            ToolDefinition::new(
                "gentufa".to_owned(),
                "Parse Lojban text into the production syntax representation.".to_owned(),
                request_schema::<ToolGentufaRequest>(),
            )?,
            ToolDefinition::new(
                "tersmu".to_owned(),
                "Compute the production semantic representation of Lojban text.".to_owned(),
                request_schema::<ToolTersmuRequest>(),
            )?,
            ToolDefinition::new(
                "jvozba".to_owned(),
                "Build a Lojban compound word from source words or fixed rafsi.".to_owned(),
                request_schema::<ToolJvozbaRequest>(),
            )?,
            ToolDefinition::new(
                "cukta".to_owned(),
                "Read or search The Complete Lojban Language reference book.".to_owned(),
                request_schema::<ToolCuktaRequest>(),
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
                let request = decode_request::<ToolTersmuRequest>(call)?;
                run_tool_tersmu(request).map_err(|error| execution_error(call, error.to_string()))
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
#[ensures(ret.is_object())]
fn request_schema<T: JsonSchema>() -> Value {
    let mut settings = schemars::generate::SchemaSettings::default();
    settings.inline_subschemas = true;
    settings.transforms.push(Box::new(StringEnumTypeTransform));
    let generator = schemars::generate::SchemaGenerator::new(settings);
    serde_json::to_value(generator.into_root_schema_for::<T>())
        .expect("production tool request schema serializes to JSON")
}

/// Keep model-facing request schemas identical to the production MCP schema shape.
///
/// Schemars omits the enclosing string type for documented unit enums represented
/// as a `oneOf` of string constants. The production MCP layer restores it because
/// tool clients otherwise present those fields as untyped.
#[invariant(true)]
#[derive(Clone, Debug)]
struct StringEnumTypeTransform;

impl Transform for StringEnumTypeTransform {
    #[requires(true)]
    #[ensures(true)]
    fn transform(&mut self, schema: &mut Schema) {
        if let Some(object) = schema.as_object_mut() {
            let is_string_const_enum =
                object
                    .get("oneOf")
                    .and_then(Value::as_array)
                    .is_some_and(|variants| {
                        !variants.is_empty()
                            && variants.iter().all(|variant| {
                                variant.get("const").is_some()
                                    && variant.get("type").and_then(Value::as_str) == Some("string")
                            })
                    });
            if is_string_const_enum && !object.contains_key("type") {
                object.insert("type".to_owned(), Value::String("string".to_owned()));
            }
        }
        transform_subschemas(self, schema);
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

#[cfg(test)]
mod tests {
    use super::*;
    use jbotci_cli::ToolStatus;
    use serde_json::json;

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
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn reference_definitions_use_each_production_request_schema() {
        let definitions = ReferenceTools::definitions().expect("valid definitions");
        assert_schema::<ToolVlackuRequest>(&definitions, "vlacku");
        assert_schema::<ToolGentufaRequest>(&definitions, "gentufa");
        assert_schema::<ToolTersmuRequest>(&definitions, "tersmu");
        assert_schema::<ToolJvozbaRequest>(&definitions, "jvozba");
        assert_schema::<ToolCuktaRequest>(&definitions, "cukta");
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
            story_time: false,
            indent: None,
        })
        .expect("direct JSON call");
        assert_ne!(json, default, "format selection must not be inert");
        assert_eq!(json, direct_json.stdout);

        let invalid_dialect = "(definitely-not-a-jbotci-dialect)".to_owned();
        let direct_error = run_tool_tersmu(ToolTersmuRequest {
            text: "mi klama".to_owned(),
            format: ToolTersmuFormat::TreeProj,
            dialect: Some(invalid_dialect.clone()),
            story_time: false,
            indent: None,
        })
        .expect_err("direct invalid dialect must fail")
        .to_string();
        let gate_error = gate_lojban("mi klama".to_owned(), None, Some(invalid_dialect))
            .expect_err("gate must forward the invalid dialect");
        assert_eq!(
            gate_error,
            GateError::ToolExecution {
                message: direct_error
            }
        );
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
        let direct = run_tool_tersmu(decode_request(&tersmu).expect("tersmu request"))
            .expect("direct tersmu");
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
            format: ToolTersmuFormat::TreeProj,
            dialect: None,
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
    fn assert_schema<T: JsonSchema>(definitions: &[ToolDefinition], name: &str) {
        let definition = definitions
            .iter()
            .find(|definition| definition.name() == name)
            .unwrap_or_else(|| panic!("missing {name} definition"));
        assert_eq!(definition.function.parameters, request_schema::<T>());
    }
}
