//! Validated schema for the WebGPU execution artifact manifest.

use std::collections::BTreeMap;

#[allow(unused_imports)]
use bityzba::{ensures, invariant, requires};
use serde::Deserialize;

use crate::core::{validate_chunk_layout, validate_q4_tensor_storage};

const EXPECTED_SCHEMA_VERSION: u32 = 1;

#[invariant(*schema_version > 0)]
#[invariant(!runtime.is_empty())]
#[invariant(!artifact_version.is_empty())]
#[invariant(!model_key.is_empty())]
#[invariant(max_sequence_length.as_ref().is_none_or(|length| *length > 0))]
#[invariant(!tensors.is_empty())]
#[cfg_attr(test, allow(dead_code))]
#[derive(Debug, Deserialize)]
pub(super) struct ArtifactManifest {
    pub(super) schema_version: u32,
    pub(super) runtime: String,
    pub(super) artifact_version: String,
    pub(super) model_key: String,
    pub(super) max_sequence_length: Option<usize>,
    pub(super) model: ModelConfig,
    pub(super) tokenizer: TokenizerSpec,
    pub(super) tensors: BTreeMap<String, TensorSpec>,
}

#[invariant(*vocab_size > 0)]
#[invariant(*hidden_size > 0)]
#[invariant(*num_hidden_layers > 0)]
#[invariant(*num_attention_heads > 0)]
#[invariant(*num_key_value_heads > 0)]
#[invariant(*head_dim > 0)]
#[invariant(*intermediate_size > 0)]
#[invariant(*num_attention_heads % *num_key_value_heads == 0)]
#[invariant(num_attention_heads.checked_mul(*head_dim).is_some())]
#[invariant(num_key_value_heads.checked_mul(*head_dim).is_some())]
#[invariant(*head_dim % 2 == 0)]
#[invariant(rms_norm_eps.is_finite() && *rms_norm_eps > 0.0)]
#[invariant(rope_theta.is_finite() && *rope_theta > 0.0)]
#[derive(Debug, Clone, Deserialize)]
pub(super) struct ModelConfig {
    pub(super) vocab_size: usize,
    pub(super) hidden_size: usize,
    pub(super) num_hidden_layers: usize,
    pub(super) num_attention_heads: usize,
    pub(super) num_key_value_heads: usize,
    pub(super) head_dim: usize,
    pub(super) intermediate_size: usize,
    pub(super) rms_norm_eps: f32,
    pub(super) rope_theta: f32,
}

#[invariant(!url.trim().is_empty())]
#[invariant(*byte_length > 0)]
#[invariant(
    canonical_json_sha256.len() == 64
        && canonical_json_sha256
            .chars()
            .all(|character| character.is_ascii_hexdigit())
)]
#[derive(Debug, Deserialize)]
pub(super) struct TokenizerSpec {
    pub(super) url: String,
    pub(super) byte_length: usize,
    pub(super) canonical_json_sha256: String,
}

#[invariant(!kind.is_empty())]
#[invariant(shape.iter().all(|dimension| *dimension > 0))]
#[invariant(kind == "f32" -> !shape.is_empty())]
#[invariant((kind == "q4_onnx_gather" || kind == "q4_onnx_matmul") -> shape.len() == 2)]
#[invariant(group_size.as_ref().is_none_or(|value| *value > 0))]
#[invariant(groups.as_ref().is_none_or(|value| *value > 0))]
#[cfg_attr(test, allow(dead_code))]
#[derive(Debug, Clone, Deserialize)]
pub(super) struct TensorSpec {
    pub(super) kind: String,
    pub(super) shape: Vec<usize>,
    pub(super) group_size: Option<usize>,
    pub(super) groups: Option<usize>,
    pub(super) qweight: Option<ChunkedSpec>,
    pub(super) scales: Option<ChunkedSpec>,
    pub(super) zero_points: Option<ChunkedSpec>,
    pub(super) data: Option<ChunkedSpec>,
}

#[invariant(*byte_length > 0)]
#[invariant(!chunks.is_empty())]
#[derive(Debug, Clone, Deserialize)]
pub(super) struct ChunkedSpec {
    pub(super) byte_length: usize,
    pub(super) chunks: Vec<ChunkSpec>,
}

#[invariant(!url.is_empty())]
#[invariant(*byte_length > 0)]
#[invariant(sha256.len() == 64 && sha256.chars().all(|character| character.is_ascii_hexdigit()))]
#[derive(Debug, Clone, Deserialize)]
pub(super) struct ChunkSpec {
    pub(super) url: String,
    pub(super) byte_offset: usize,
    pub(super) byte_length: usize,
    pub(super) sha256: String,
}

#[requires(!expected_runtime.is_empty())]
#[requires(!expected_version.is_empty())]
#[requires(!expected_model_key.is_empty())]
#[requires(expected_dimensions > 0)]
#[ensures(true)]
pub(super) fn validate_artifact_manifest(
    manifest: &ArtifactManifest,
    expected_runtime: &str,
    expected_version: &str,
    expected_model_key: &str,
    expected_dimensions: usize,
) -> Result<(), String> {
    for (field, actual, expected) in [
        (
            "schema_version",
            manifest.schema_version.to_string(),
            EXPECTED_SCHEMA_VERSION.to_string(),
        ),
        (
            "runtime",
            manifest.runtime.clone(),
            expected_runtime.to_owned(),
        ),
        (
            "artifact_version",
            manifest.artifact_version.clone(),
            expected_version.to_owned(),
        ),
        (
            "model_key",
            manifest.model_key.clone(),
            expected_model_key.to_owned(),
        ),
    ] {
        if actual != expected {
            return Err(format!(
                "F2LLM WebGPU manifest {field} mismatch: expected {expected}, got {actual}"
            ));
        }
    }
    if manifest.model.hidden_size != expected_dimensions {
        return Err(format!(
            "F2LLM WebGPU hidden size mismatch: expected {}, got {}",
            expected_dimensions, manifest.model.hidden_size
        ));
    }
    let query_width = attention_width(manifest.model.num_attention_heads, manifest.model.head_dim);
    let key_value_width =
        attention_width(manifest.model.num_key_value_heads, manifest.model.head_dim);
    for (name, spec) in &manifest.tensors {
        validate_tensor_spec_shape(name, spec)?;
    }
    validate_model_tensor_shapes(manifest, query_width, key_value_width)
}

#[requires(heads > 0)]
#[requires(head_dim > 0)]
#[ensures(ret >= head_dim)]
fn attention_width(heads: usize, head_dim: usize) -> usize {
    heads
        .checked_mul(head_dim)
        .expect("ModelConfig invariant guarantees attention width does not overflow")
}

#[requires(query_width > 0)]
#[requires(key_value_width > 0)]
#[ensures(true)]
fn validate_model_tensor_shapes(
    manifest: &ArtifactManifest,
    query_width: usize,
    key_value_width: usize,
) -> Result<(), String> {
    let model = &manifest.model;
    validate_manifest_tensor_shape(
        manifest,
        "model.embed_tokens.weight",
        "q4_onnx_gather",
        &[model.vocab_size, model.hidden_size],
    )?;
    validate_manifest_tensor_shape(manifest, "model.norm.weight", "f32", &[model.hidden_size])?;
    for layer in 0..model.num_hidden_layers {
        let prefix = format!("model.layers.{layer}");
        validate_manifest_tensor_shape(
            manifest,
            &format!("{prefix}.input_layernorm.weight"),
            "f32",
            &[model.hidden_size],
        )?;
        validate_manifest_tensor_shape(
            manifest,
            &format!("{prefix}.post_attention_layernorm.weight"),
            "f32",
            &[model.hidden_size],
        )?;
        validate_manifest_tensor_shape(
            manifest,
            &format!("{prefix}.self_attn.q_norm.weight"),
            "f32",
            &[model.head_dim],
        )?;
        validate_manifest_tensor_shape(
            manifest,
            &format!("{prefix}.self_attn.k_norm.weight"),
            "f32",
            &[model.head_dim],
        )?;
        validate_manifest_tensor_shape(
            manifest,
            &format!("{prefix}.self_attn.q_proj.weight"),
            "q4_onnx_matmul",
            &[query_width, model.hidden_size],
        )?;
        validate_manifest_tensor_shape(
            manifest,
            &format!("{prefix}.self_attn.k_proj.weight"),
            "q4_onnx_matmul",
            &[key_value_width, model.hidden_size],
        )?;
        validate_manifest_tensor_shape(
            manifest,
            &format!("{prefix}.self_attn.v_proj.weight"),
            "q4_onnx_matmul",
            &[key_value_width, model.hidden_size],
        )?;
        validate_manifest_tensor_shape(
            manifest,
            &format!("{prefix}.self_attn.o_proj.weight"),
            "q4_onnx_matmul",
            &[model.hidden_size, query_width],
        )?;
        validate_manifest_tensor_shape(
            manifest,
            &format!("{prefix}.mlp.gate_proj.weight"),
            "q4_onnx_matmul",
            &[model.intermediate_size, model.hidden_size],
        )?;
        validate_manifest_tensor_shape(
            manifest,
            &format!("{prefix}.mlp.up_proj.weight"),
            "q4_onnx_matmul",
            &[model.intermediate_size, model.hidden_size],
        )?;
        validate_manifest_tensor_shape(
            manifest,
            &format!("{prefix}.mlp.down_proj.weight"),
            "q4_onnx_matmul",
            &[model.hidden_size, model.intermediate_size],
        )?;
    }
    Ok(())
}

#[requires(!name.is_empty())]
#[requires(!expected_kind.is_empty())]
#[requires(expected_shape.iter().all(|dimension| *dimension > 0))]
#[ensures(true)]
fn validate_manifest_tensor_shape(
    manifest: &ArtifactManifest,
    name: &str,
    expected_kind: &str,
    expected_shape: &[usize],
) -> Result<(), String> {
    let spec = manifest
        .tensors
        .get(name)
        .ok_or_else(|| format!("F2LLM WebGPU artifact is missing required tensor: {name}"))?;
    if spec.kind != expected_kind {
        return Err(format!(
            "{name} kind mismatch: expected {expected_kind}, got {}",
            spec.kind
        ));
    }
    if spec.shape != expected_shape {
        return Err(format!(
            "{name} shape mismatch: expected {:?}, got {:?}",
            expected_shape, spec.shape
        ));
    }
    Ok(())
}

#[requires(!name.is_empty())]
#[ensures(true)]
fn validate_tensor_spec_shape(name: &str, spec: &TensorSpec) -> Result<(), String> {
    match spec.kind.as_str() {
        "q4_onnx_gather" | "q4_onnx_matmul" => {
            let shape = rank2_shape_from_valid_tensor(&spec.shape);
            let group_size = spec
                .group_size
                .ok_or_else(|| format!("{name} is missing group_size"))?;
            let groups = spec
                .groups
                .ok_or_else(|| format!("{name} is missing groups"))?;
            let expected_groups = shape[1].div_ceil(group_size);
            if groups != expected_groups {
                return Err(format!(
                    "{name} groups mismatch: expected {expected_groups}, got {groups}"
                ));
            }
            Ok(())
        }
        "f32" => Ok(()),
        other => Err(format!("unsupported F2LLM tensor kind for {name}: {other}")),
    }
}

#[requires(shape.len() == 2)]
#[requires(shape.iter().all(|dimension| *dimension > 0))]
#[ensures(ret.iter().all(|dimension| *dimension > 0))]
fn rank2_shape_from_valid_tensor(shape: &[usize]) -> [usize; 2] {
    [shape[0], shape[1]]
}

#[requires(!name.is_empty())]
#[requires(kind == "q4_onnx_gather" || kind == "q4_onnx_matmul")]
#[requires(group_size > 0)]
#[requires(groups > 0)]
#[ensures(true)]
pub(super) fn validate_q4_tensor_chunks(
    name: &str,
    kind: &str,
    shape: [usize; 2],
    group_size: usize,
    groups: usize,
    qweight: &ChunkedSpec,
    scales: &ChunkedSpec,
    zero_points: &ChunkedSpec,
) -> Result<(), String> {
    validate_q4_tensor_storage(
        name,
        kind,
        shape,
        group_size,
        groups,
        qweight.byte_length,
        scales.byte_length,
        zero_points.byte_length,
    )
}

#[requires(!label.is_empty())]
#[ensures(true)]
pub(super) fn validate_chunked_spec(label: &str, chunked: &ChunkedSpec) -> Result<(), String> {
    let chunks = chunked
        .chunks
        .iter()
        .map(|chunk| {
            (
                chunk.url.as_str(),
                chunk.byte_offset,
                chunk.byte_length,
                chunk.sha256.as_str(),
            )
        })
        .collect::<Vec<_>>();
    validate_chunk_layout(label, chunked.byte_length, &chunks)
}

#[requires(true)]
#[ensures(true)]
pub(super) fn tensor_byte_length(spec: &TensorSpec) -> usize {
    match spec.kind.as_str() {
        "q4_onnx_gather" | "q4_onnx_matmul" => {
            spec.qweight
                .as_ref()
                .map_or(0, |chunked| chunked.byte_length)
                + spec
                    .scales
                    .as_ref()
                    .map_or(0, |chunked| chunked.byte_length)
                + spec
                    .zero_points
                    .as_ref()
                    .map_or(0, |chunked| chunked.byte_length)
        }
        "f32" => spec.data.as_ref().map_or(0, |chunked| chunked.byte_length),
        _ => 0,
    }
}

#[requires(!name.is_empty())]
#[ensures(true)]
pub(super) fn checked_rank2_shape(name: &str, shape: &[usize]) -> Result<[usize; 2], String> {
    if shape.len() != 2 || shape[0] == 0 || shape[1] == 0 {
        Err(format!("{name} must have rank 2 positive shape"))
    } else {
        Ok([shape[0], shape[1]])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn accepts_projected_attention_width_larger_than_hidden_size() {
        let manifest = projected_attention_manifest();
        validate_artifact_manifest(
            &manifest,
            "jbotci-webgpu-f2llm",
            "0.2.0",
            "f2llm-v2-330m-q4-896",
            896,
        )
        .expect("published F2LLM projection geometry is valid");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn rejects_q_projection_that_uses_hidden_width_as_attention_width() {
        let mut value = projected_attention_manifest_json();
        value["tensors"]["model.layers.0.self_attn.q_proj.weight"]["shape"] =
            serde_json::json!([896, 896]);
        let manifest: ArtifactManifest =
            serde_json::from_value(value).expect("test manifest parses");
        let error = validate_artifact_manifest(
            &manifest,
            "jbotci-webgpu-f2llm",
            "0.2.0",
            "f2llm-v2-330m-q4-896",
            896,
        )
        .expect_err("q_proj must use attention projection width");
        assert!(error.contains("self_attn.q_proj.weight shape mismatch"));
        assert!(error.contains("[2048, 896]"));
    }

    #[requires(true)]
    #[ensures(ret.model.hidden_size == 896)]
    fn projected_attention_manifest() -> ArtifactManifest {
        serde_json::from_value(projected_attention_manifest_json()).expect("test manifest parses")
    }

    #[requires(true)]
    #[ensures(ret.is_object())]
    fn projected_attention_manifest_json() -> serde_json::Value {
        serde_json::json!({
            "schema_version": 1,
            "runtime": "jbotci-webgpu-f2llm",
            "artifact_version": "0.2.0",
            "model_key": "f2llm-v2-330m-q4-896",
            "max_sequence_length": 512,
            "model": {
                "vocab_size": 151936,
                "hidden_size": 896,
                "num_hidden_layers": 1,
                "num_attention_heads": 16,
                "num_key_value_heads": 8,
                "head_dim": 128,
                "intermediate_size": 2560,
                "rms_norm_eps": 0.000001,
                "rope_theta": 1000000.0
            },
            "tokenizer": {
                "url": "tokenizer.json",
                "byte_length": 1,
                "canonical_json_sha256": "0000000000000000000000000000000000000000000000000000000000000000"
            },
            "tensors": {
                "model.embed_tokens.weight": q4_tensor_json("q4_onnx_gather", [151936, 896], 28),
                "model.norm.weight": f32_tensor_json([896]),
                "model.layers.0.input_layernorm.weight": f32_tensor_json([896]),
                "model.layers.0.post_attention_layernorm.weight": f32_tensor_json([896]),
                "model.layers.0.self_attn.q_norm.weight": f32_tensor_json([128]),
                "model.layers.0.self_attn.k_norm.weight": f32_tensor_json([128]),
                "model.layers.0.self_attn.q_proj.weight": q4_tensor_json("q4_onnx_matmul", [2048, 896], 28),
                "model.layers.0.self_attn.k_proj.weight": q4_tensor_json("q4_onnx_matmul", [1024, 896], 28),
                "model.layers.0.self_attn.v_proj.weight": q4_tensor_json("q4_onnx_matmul", [1024, 896], 28),
                "model.layers.0.self_attn.o_proj.weight": q4_tensor_json("q4_onnx_matmul", [896, 2048], 64),
                "model.layers.0.mlp.gate_proj.weight": q4_tensor_json("q4_onnx_matmul", [2560, 896], 28),
                "model.layers.0.mlp.up_proj.weight": q4_tensor_json("q4_onnx_matmul", [2560, 896], 28),
                "model.layers.0.mlp.down_proj.weight": q4_tensor_json("q4_onnx_matmul", [896, 2560], 80)
            }
        })
    }

    #[requires(!kind.is_empty())]
    #[requires(shape[0] > 0 && shape[1] > 0)]
    #[requires(groups > 0)]
    #[ensures(ret.is_object())]
    fn q4_tensor_json(kind: &str, shape: [usize; 2], groups: usize) -> serde_json::Value {
        serde_json::json!({
            "kind": kind,
            "shape": shape,
            "group_size": 32,
            "groups": groups
        })
    }

    #[requires(shape[0] > 0)]
    #[ensures(ret.is_object())]
    fn f32_tensor_json(shape: [usize; 1]) -> serde_json::Value {
        serde_json::json!({
            "kind": "f32",
            "shape": shape
        })
    }
}
