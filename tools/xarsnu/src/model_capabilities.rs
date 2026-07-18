//! Vendored OpenRouter capability policy used to resolve participant defaults.

use std::collections::BTreeMap;
use std::sync::OnceLock;

#[allow(unused_imports)]
use bityzba::{ensures, invariant, new, requires};
use serde::{Deserialize, Serialize};

use crate::{ReasoningConfig, ToolChoice};

const SNAPSHOT_JSON: &str = include_str!("../openrouter-model-capabilities.json");

/// Provider tool-choice value after metadata and explicit overrides are resolved.
#[invariant(::Required => true)]
#[invariant(::Auto => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderToolChoice {
    Required,
    Auto,
}

/// Effective OpenRouter behavior for one participant model.
#[invariant(true, "all boolean combinations are meaningful provider policies")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ParticipantModelPolicy {
    pub tool_choice: ProviderToolChoice,
    pub reasoning: ReasoningConfig,
    pub supports_prefill: bool,
}

impl ParticipantModelPolicy {
    /// Resolve metadata defaults while preserving explicit participant overrides.
    #[requires(!model.trim().is_empty())]
    #[ensures(configured_tool_choice == ToolChoice::Required -> ret.tool_choice == ProviderToolChoice::Required)]
    #[ensures(configured_tool_choice == ToolChoice::Auto -> ret.tool_choice == ProviderToolChoice::Auto)]
    #[ensures(configured_reasoning.is_some() -> ret.reasoning == configured_reasoning.unwrap())]
    pub(crate) fn resolve(
        model: &str,
        configured_tool_choice: ToolChoice,
        configured_reasoning: Option<ReasoningConfig>,
    ) -> Self {
        let capabilities = model_capabilities(model).copied().unwrap_or_default();
        let tool_choice = match configured_tool_choice {
            ToolChoice::Metadata if capabilities.required_tool_calls => {
                ProviderToolChoice::Required
            }
            ToolChoice::Metadata | ToolChoice::Auto => ProviderToolChoice::Auto,
            ToolChoice::Required => ProviderToolChoice::Required,
        };
        let reasoning = configured_reasoning.unwrap_or_else(|| {
            if tool_choice == ProviderToolChoice::Required && capabilities.disabled_reasoning {
                ReasoningConfig::Off
            } else {
                ReasoningConfig::Default
            }
        });
        Self {
            tool_choice,
            reasoning,
            supports_prefill: capabilities.prefill,
        }
    }
}

#[invariant(!source.trim().is_empty())]
#[invariant(!refresh.trim().is_empty())]
#[invariant(*model_count > 0)]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SnapshotProvenance {
    source: String,
    refresh: String,
    model_count: usize,
}

#[invariant(!models.is_empty())]
#[invariant(_provenance.model_count == models.len())]
#[derive(Debug, Deserialize)]
struct CapabilitySnapshot {
    _provenance: SnapshotProvenance,
    models: BTreeMap<String, ModelCapabilities>,
}

#[invariant(
    true,
    "independent capability probe results permit every boolean combination"
)]
#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ModelCapabilities {
    prefill: bool,
    #[allow(dead_code)]
    structured_outputs: bool,
    required_tool_calls: bool,
    disabled_reasoning: bool,
    #[allow(dead_code)]
    cache_control: bool,
}

/// Look up an exact OpenRouter model id in the audited vendored snapshot.
#[requires(!model.trim().is_empty())]
#[ensures(true)]
fn model_capabilities(model: &str) -> Option<&'static ModelCapabilities> {
    static SNAPSHOT: OnceLock<CapabilitySnapshot> = OnceLock::new();
    SNAPSHOT
        .get_or_init(|| {
            serde_json::from_str(SNAPSHOT_JSON)
                .expect("vendored OpenRouter model capability snapshot must be valid")
        })
        .models
        .get(model)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn vendored_snapshot_provenance_and_count_are_current() {
        let snapshot: CapabilitySnapshot =
            serde_json::from_str(SNAPSHOT_JSON).expect("valid capability snapshot");
        assert_eq!(snapshot._provenance.model_count, snapshot.models.len());
        assert_eq!(snapshot.models.len(), 342);
        assert!(snapshot._provenance.source.contains("int19h/bickr/blob/"));
        assert!(
            snapshot
                ._provenance
                .refresh
                .contains("probe-openrouter-model-capabilities.mjs")
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn named_thinking_models_resolve_to_the_required_dispositions() {
        let deepseek = ParticipantModelPolicy::resolve(
            "deepseek/deepseek-v4-flash",
            ToolChoice::Metadata,
            None,
        );
        assert_eq!(deepseek.tool_choice, ProviderToolChoice::Auto);
        assert_eq!(deepseek.reasoning, ReasoningConfig::Default);
        assert!(deepseek.supports_prefill);

        let qwen =
            ParticipantModelPolicy::resolve("qwen/qwen3.5-flash-02-23", ToolChoice::Metadata, None);
        assert_eq!(qwen.tool_choice, ProviderToolChoice::Auto);
        assert_eq!(qwen.reasoning, ReasoningConfig::Default);
        assert!(!qwen.supports_prefill);

        let mimo = ParticipantModelPolicy::resolve("xiaomi/mimo-v2.5", ToolChoice::Metadata, None);
        assert_eq!(mimo.tool_choice, ProviderToolChoice::Required);
        assert_eq!(mimo.reasoning, ReasoningConfig::Off);
        assert!(mimo.supports_prefill);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn explicit_participant_settings_override_metadata() {
        let required = ParticipantModelPolicy::resolve(
            "deepseek/deepseek-v4-flash",
            ToolChoice::Required,
            Some(ReasoningConfig::High),
        );
        assert_eq!(required.tool_choice, ProviderToolChoice::Required);
        assert_eq!(required.reasoning, ReasoningConfig::High);

        let automatic = ParticipantModelPolicy::resolve(
            "xiaomi/mimo-v2.5",
            ToolChoice::Auto,
            Some(ReasoningConfig::Low),
        );
        assert_eq!(automatic.tool_choice, ProviderToolChoice::Auto);
        assert_eq!(automatic.reasoning, ReasoningConfig::Low);
    }
}
