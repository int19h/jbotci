use std::collections::{BTreeMap, HashMap};

#[allow(unused_imports)]
use bityzba::{ensures, invariant, requires};
use futures_channel::oneshot;
use js_sys::{Array, Float32Array, Function, Object, Promise, Reflect, Uint32Array, Uint8Array};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

use crate::f2llm_runtime_core::{
    mean_pool_normalized, normalize_in_place, pack_token_windows, QwenByteBpeTokenizer,
    TokenWindow, DEFAULT_MAX_SEQUENCE_LENGTH,
};

const EXPECTED_SCHEMA_VERSION: u32 = 1;
const DEFAULT_WORKGROUP_WIDTH: u32 = 8;
const VECTOR_WORKGROUP_SIZE: u32 = 64;

const EMBEDDING_ONNX_Q4_SHADER: &str = r#"
struct Params {
  seq: u32,
  hidden: u32,
  groups: u32,
  group_size: u32,
};
@group(0) @binding(0) var<storage, read> tokens: array<u32>;
@group(0) @binding(1) var<storage, read> qbytes: array<u32>;
@group(0) @binding(2) var<storage, read> scales: array<f32>;
@group(0) @binding(3) var<storage, read> zero_points: array<u32>;
@group(0) @binding(4) var<storage, read_write> output: array<f32>;
@group(0) @binding(5) var<uniform> params: Params;

fn q4_byte(byte_index: u32) -> u32 {
  let word = qbytes[byte_index / 4u];
  let shift = (byte_index % 4u) * 8u;
  return (word >> shift) & 255u;
}

fn zero_point_byte(byte_index: u32) -> u32 {
  let word = zero_points[byte_index / 4u];
  let shift = (byte_index % 4u) * 8u;
  return (word >> shift) & 255u;
}

fn q4_nibble(element: u32) -> u32 {
  let packed = q4_byte(element / 2u);
  return select((packed >> 4u) & 15u, packed & 15u, (element & 1u) == 0u);
}

fn zero_point_nibble(element: u32) -> u32 {
  let packed = zero_point_byte(element / 2u);
  return select((packed >> 4u) & 15u, packed & 15u, (element & 1u) == 0u);
}

fn q4_value(row: u32, col: u32) -> f32 {
  let group = col / params.group_size;
  let q = q4_nibble(row * params.hidden + col);
  let zero_point = zero_point_nibble(row * params.groups + group);
  let scale = scales[row * params.groups + group];
  return f32(i32(q) - i32(zero_point)) * scale;
}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
  let token_index = id.x;
  let dim = id.y;
  if (token_index >= params.seq || dim >= params.hidden) {
    return;
  }
  let token = tokens[token_index];
  output[token_index * params.hidden + dim] = q4_value(token, dim);
}
"#;

const RMS_NORM_SHADER: &str = r#"
struct Params {
  rows: u32,
  cols: u32,
  eps: f32,
  _pad: u32,
};
@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read> weight: array<f32>;
@group(0) @binding(2) var<storage, read_write> output: array<f32>;
@group(0) @binding(3) var<uniform> params: Params;

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
  let row = id.x;
  let col = id.y;
  if (row >= params.rows || col >= params.cols) {
    return;
  }
  let base = row * params.cols;
  var sum = 0.0;
  for (var dim = 0u; dim < params.cols; dim = dim + 1u) {
    let value = input[base + dim];
    sum = sum + value * value;
  }
  let inv_rms = inverseSqrt(sum / f32(params.cols) + params.eps);
  output[base + col] = input[base + col] * inv_rms * weight[col];
}
"#;

const LINEAR_ONNX_Q4_SHADER: &str = r#"
struct Params {
  rows: u32,
  in_cols: u32,
  out_cols: u32,
  group_size: u32,
  groups: u32,
  _pad0: u32,
  _pad1: u32,
  _pad2: u32,
};
@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read> qbytes: array<u32>;
@group(0) @binding(2) var<storage, read> scales: array<f32>;
@group(0) @binding(3) var<storage, read> zero_points: array<f32>;
@group(0) @binding(4) var<storage, read_write> output: array<f32>;
@group(0) @binding(5) var<uniform> params: Params;

fn q4_byte(byte_index: u32) -> u32 {
  let word = qbytes[byte_index / 4u];
  let shift = (byte_index % 4u) * 8u;
  return (word >> shift) & 255u;
}

fn q4_nibble(element: u32) -> u32 {
  let packed = q4_byte(element / 2u);
  return select((packed >> 4u) & 15u, packed & 15u, (element & 1u) == 0u);
}

fn weight_value(row: u32, col: u32) -> f32 {
  let element = row * params.in_cols + col;
  let group = col / params.group_size;
  let quant_index = row * params.groups + group;
  let q = q4_nibble(element);
  return (f32(i32(q)) - zero_points[quant_index]) * scales[quant_index];
}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
  let row = id.x;
  let out_col = id.y;
  if (row >= params.rows || out_col >= params.out_cols) {
    return;
  }
  var sum = 0.0;
  let input_base = row * params.in_cols;
  for (var in_col = 0u; in_col < params.in_cols; in_col = in_col + 1u) {
    sum = sum + input[input_base + in_col] * weight_value(out_col, in_col);
  }
  output[row * params.out_cols + out_col] = sum;
}
"#;

const PACKED_ROPE_SHADER: &str = r#"
struct Params {
  seq: u32,
  heads: u32,
  head_dim: u32,
  _pad0: u32,
  theta: f32,
  _pad1: u32,
  _pad2: u32,
  _pad3: u32,
};
@group(0) @binding(0) var<storage, read_write> values: array<f32>;
@group(0) @binding(1) var<storage, read> local_positions: array<u32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
  let token = id.x;
  let head = id.y;
  let dim = id.z;
  let half_dim = params.head_dim / 2u;
  if (token >= params.seq || head >= params.heads || dim >= half_dim) {
    return;
  }
  let base = (token * params.heads + head) * params.head_dim;
  let exponent = f32(dim * 2u) / f32(params.head_dim);
  let angle = f32(local_positions[token]) / pow(params.theta, exponent);
  let c = cos(angle);
  let s = sin(angle);
  let first = values[base + dim];
  let second = values[base + dim + half_dim];
  values[base + dim] = first * c - second * s;
  values[base + dim + half_dim] = second * c + first * s;
}
"#;

const PACKED_ATTENTION_SHADER: &str = r#"
struct Params {
  seq: u32,
  q_heads: u32,
  kv_heads: u32,
  head_dim: u32,
  scale: f32,
  _pad0: u32,
  _pad1: u32,
  _pad2: u32,
};
@group(0) @binding(0) var<storage, read> q: array<f32>;
@group(0) @binding(1) var<storage, read> k: array<f32>;
@group(0) @binding(2) var<storage, read> v: array<f32>;
@group(0) @binding(3) var<storage, read> segment_starts: array<u32>;
@group(0) @binding(4) var<storage, read_write> output: array<f32>;
@group(0) @binding(5) var<uniform> params: Params;

fn score_for(query_token: u32, key_token: u32, q_head: u32, kv_head: u32) -> f32 {
  var score = 0.0;
  let q_base = (query_token * params.q_heads + q_head) * params.head_dim;
  let k_base = (key_token * params.kv_heads + kv_head) * params.head_dim;
  for (var dim = 0u; dim < params.head_dim; dim = dim + 1u) {
    score = score + q[q_base + dim] * k[k_base + dim];
  }
  return score * params.scale;
}

@compute @workgroup_size(4, 4, 4)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
  let token = id.x;
  let q_head = id.y;
  let dim = id.z;
  if (token >= params.seq || q_head >= params.q_heads || dim >= params.head_dim) {
    return;
  }
  let kv_group_size = params.q_heads / params.kv_heads;
  let kv_head = q_head / kv_group_size;
  let segment_start = segment_starts[token];
  var max_score = -3.402823e38;
  for (var key_token = segment_start; key_token <= token; key_token = key_token + 1u) {
    max_score = max(max_score, score_for(token, key_token, q_head, kv_head));
  }
  var denominator = 0.0;
  var weighted = 0.0;
  for (var key_token = segment_start; key_token <= token; key_token = key_token + 1u) {
    let probability_weight = exp(score_for(token, key_token, q_head, kv_head) - max_score);
    denominator = denominator + probability_weight;
    let value_base = (key_token * params.kv_heads + kv_head) * params.head_dim;
    weighted = weighted + probability_weight * v[value_base + dim];
  }
  let out_base = (token * params.q_heads + q_head) * params.head_dim;
  output[out_base + dim] = weighted / denominator;
}
"#;

const ADD_SHADER: &str = r#"
struct Params {
  len: u32,
  _pad0: u32,
  _pad1: u32,
  _pad2: u32,
};
@group(0) @binding(0) var<storage, read> left: array<f32>;
@group(0) @binding(1) var<storage, read> right: array<f32>;
@group(0) @binding(2) var<storage, read_write> output: array<f32>;
@group(0) @binding(3) var<uniform> params: Params;

@compute @workgroup_size(256, 1, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
  let index = id.x;
  if (index >= params.len) {
    return;
  }
  output[index] = left[index] + right[index];
}
"#;

const SILU_MUL_SHADER: &str = r#"
struct Params {
  len: u32,
  _pad0: u32,
  _pad1: u32,
  _pad2: u32,
};
@group(0) @binding(0) var<storage, read> gate: array<f32>;
@group(0) @binding(1) var<storage, read> up: array<f32>;
@group(0) @binding(2) var<storage, read_write> output: array<f32>;
@group(0) @binding(3) var<uniform> params: Params;

@compute @workgroup_size(256, 1, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
  let index = id.x;
  if (index >= params.len) {
    return;
  }
  let value = gate[index];
  output[index] = (value / (1.0 + exp(-value))) * up[index];
}
"#;

const VECTOR_DOT_F16_SHADER: &str = r#"
enable f16;
struct Params {
  rows: u32,
  dims: u32,
  _pad0: u32,
  _pad1: u32,
};
@group(0) @binding(0) var<storage, read> vectors: array<f16>;
@group(0) @binding(1) var<storage, read> query: array<f32>;
@group(0) @binding(2) var<storage, read_write> scores: array<f32>;
@group(0) @binding(3) var<uniform> params: Params;

@compute @workgroup_size(64, 1, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
  let row = id.x;
  if (row >= params.rows) {
    return;
  }
  var score = 0.0;
  let base = row * params.dims;
  for (var dim = 0u; dim < params.dims; dim = dim + 1u) {
    score = score + f32(vectors[base + dim]) * query[dim];
  }
  scores[row] = score;
}
"#;

#[invariant(true)]
#[derive(Debug, Deserialize)]
struct ArtifactManifest {
    schema_version: u32,
    runtime: String,
    artifact_version: String,
    model_key: String,
    max_sequence_length: Option<usize>,
    model: ModelConfig,
    tokenizer: TokenizerSpec,
    tensors: BTreeMap<String, TensorSpec>,
}

#[invariant(true)]
#[derive(Debug, Clone, Deserialize)]
struct ModelConfig {
    vocab_size: usize,
    hidden_size: usize,
    num_hidden_layers: usize,
    num_attention_heads: usize,
    num_key_value_heads: usize,
    head_dim: usize,
    intermediate_size: usize,
    rms_norm_eps: f32,
    rope_theta: f32,
}

#[invariant(true)]
#[derive(Debug, Deserialize)]
struct TokenizerSpec {
    url: String,
    byte_length: usize,
    canonical_json_sha256: String,
}

#[invariant(true)]
#[derive(Debug, Clone, Deserialize)]
struct TensorSpec {
    kind: String,
    shape: Vec<usize>,
    group_size: Option<usize>,
    groups: Option<usize>,
    qweight: Option<ChunkedSpec>,
    scales: Option<ChunkedSpec>,
    zero_points: Option<ChunkedSpec>,
    data: Option<ChunkedSpec>,
}

#[invariant(true)]
#[derive(Debug, Clone, Deserialize)]
struct ChunkedSpec {
    byte_length: usize,
    chunks: Vec<ChunkSpec>,
}

#[invariant(true)]
#[derive(Debug, Clone, Deserialize)]
struct ChunkSpec {
    url: String,
    byte_offset: usize,
    byte_length: usize,
    sha256: String,
}

#[invariant(::Q4OnnxGather(_) => true)]
#[invariant(::Q4OnnxMatmul(_) => true)]
#[invariant(::F32(_) => true)]
#[derive(Debug, Clone)]
enum Tensor {
    Q4OnnxGather(Q4Tensor),
    Q4OnnxMatmul(Q4Tensor),
    F32(F32Tensor),
}

#[invariant(true)]
#[derive(Debug, Clone)]
struct Q4Tensor {
    shape: [usize; 2],
    group_size: usize,
    groups: usize,
    q_buffer: wgpu::Buffer,
    scale_buffer: wgpu::Buffer,
    zero_point_buffer: wgpu::Buffer,
}

#[invariant(true)]
#[derive(Debug, Clone)]
struct F32Tensor {
    shape: Vec<usize>,
    buffer: wgpu::Buffer,
}

#[invariant(true)]
#[derive(Debug, Clone)]
struct VectorBuffer {
    buffer: wgpu::Buffer,
}

#[invariant(true)]
#[derive(Debug)]
struct CorpusShard {
    key: String,
    byte_len: usize,
}

#[invariant(true)]
#[derive(Debug)]
struct CorpusVectorSpec {
    corpus_id: String,
    input_hash: String,
    row_count: usize,
    dimensions: usize,
    element_type: String,
    shards: Vec<CorpusShard>,
}

#[wasm_bindgen]
#[invariant(true)]
#[derive(Debug)]
pub struct JbotciF2LlmTokenizer {
    tokenizer: QwenByteBpeTokenizer,
}

#[wasm_bindgen]
impl JbotciF2LlmTokenizer {
    #[wasm_bindgen(js_name = tokenWindows)]
    #[requires(true)]
    #[ensures(true)]
    pub fn token_windows(&self, text: &str, max_length: usize) -> Result<Array, JsValue> {
        let windows = self
            .tokenizer
            .token_windows(text, max_length)
            .map_err(js_error)?;
        let array = Array::new();
        for window in windows {
            array.push(&Uint32Array::from(window.as_slice()));
        }
        Ok(array)
    }
}

#[wasm_bindgen(js_name = jbotciF2LlmTokenizerLoad)]
#[requires(true)]
#[ensures(true)]
pub fn jbotci_f2llm_tokenizer_load(bytes: JsValue) -> Result<JbotciF2LlmTokenizer, JsValue> {
    let bytes = bytes_from_js(&bytes)?;
    let tokenizer = QwenByteBpeTokenizer::from_compact_json(&bytes).map_err(js_error)?;
    Ok(JbotciF2LlmTokenizer { tokenizer })
}

#[wasm_bindgen]
#[invariant(true)]
#[derive(Debug)]
pub struct JbotciF2LlmWebGpuRuntime {
    inner: WebGpuRuntime,
}

#[wasm_bindgen]
impl JbotciF2LlmWebGpuRuntime {
    #[wasm_bindgen(js_name = embedTexts)]
    #[requires(true)]
    #[ensures(true)]
    pub async fn embed_texts(&mut self, texts: Array) -> Result<Array, JsValue> {
        let mut rust_texts = Vec::with_capacity(texts.length() as usize);
        for index in 0..texts.length() {
            rust_texts.push(texts.get(index).as_string().unwrap_or_default());
        }
        let vectors = self.inner.embed_texts(&rust_texts).await.map_err(js_error)?;
        let output = Array::new();
        for vector in vectors {
            output.push(&Float32Array::from(vector.as_slice()));
        }
        Ok(output)
    }

    #[wasm_bindgen(js_name = scoreF16Vectors)]
    #[requires(true)]
    #[ensures(true)]
    pub async fn score_f16_vectors(
        &mut self,
        corpus: JsValue,
        query: Float32Array,
        read_binary: Function,
    ) -> Result<Float32Array, JsValue> {
        let corpus = parse_corpus_vector_spec(&corpus)?;
        let query = query.to_vec();
        let scores = self
            .inner
            .score_f16_vectors(&corpus, &query, &read_binary)
            .await
            .map_err(js_error)?;
        Ok(Float32Array::from(scores.as_slice()))
    }
}

#[wasm_bindgen(js_name = jbotciF2LlmWebGpuRuntimeLoad)]
#[requires(true)]
#[ensures(true)]
pub async fn jbotci_f2llm_webgpu_runtime_load(
    options: JsValue,
    fetch_array_buffer: Function,
    progress: JsValue,
) -> Result<JbotciF2LlmWebGpuRuntime, JsValue> {
    let options = RuntimeLoadOptions::from_js(&options)?;
    let progress = progress.dyn_into::<Function>().ok();
    let inner = WebGpuRuntime::load(options, fetch_array_buffer, progress)
        .await
        .map_err(js_error)?;
    Ok(JbotciF2LlmWebGpuRuntime { inner })
}

#[invariant(true)]
#[derive(Debug)]
struct RuntimeLoadOptions {
    base_url: String,
    expected_model_key: String,
    expected_runtime: String,
    expected_version: String,
    max_sequence_length: usize,
    dimensions: usize,
}

impl RuntimeLoadOptions {
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok() || ret.is_err())]
    fn from_js(value: &JsValue) -> Result<Self, JsValue> {
        Ok(Self {
            base_url: required_string(value, "baseUrl")?.trim_end_matches('/').to_owned(),
            expected_model_key: required_string(value, "expectedModelKey")?,
            expected_runtime: required_string(value, "expectedRuntime")?,
            expected_version: required_string(value, "expectedVersion")?,
            max_sequence_length: optional_usize(value, "maxSequenceLength")?
                .unwrap_or(DEFAULT_MAX_SEQUENCE_LENGTH),
            dimensions: required_usize(value, "dimensions")?,
        })
    }
}

#[invariant(true)]
#[derive(Debug)]
struct WebGpuRuntime {
    device: wgpu::Device,
    queue: wgpu::Queue,
    manifest: ArtifactManifest,
    tokenizer: QwenByteBpeTokenizer,
    tensors: HashMap<String, Tensor>,
    pipelines: HashMap<&'static str, wgpu::ComputePipeline>,
    transient_buffers: Vec<wgpu::Buffer>,
    vector_buffers: HashMap<String, VectorBuffer>,
    max_sequence_length: usize,
    dimensions: usize,
}

impl WebGpuRuntime {
    #[requires(!options.base_url.is_empty())]
    #[ensures(ret.as_ref().is_ok() || ret.is_err())]
    async fn load(
        options: RuntimeLoadOptions,
        fetch_array_buffer: Function,
        progress: Option<Function>,
    ) -> Result<Self, String> {
        let manifest_url = format!("{}/manifest.json", options.base_url);
        let manifest_bytes =
            fetch_bytes(&fetch_array_buffer, &manifest_url, "F2LLM WebGPU manifest").await?;
        let manifest: ArtifactManifest = serde_json::from_slice(&manifest_bytes)
            .map_err(|error| format!("failed to parse F2LLM WebGPU manifest: {error}"))?;
        validate_manifest(&manifest, &options)?;

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::BROWSER_WEBGPU,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .map_err(|error| format!("failed to request WebGPU adapter: {error}"))?;
        let required_features = if adapter.features().contains(wgpu::Features::SHADER_F16) {
            wgpu::Features::SHADER_F16
        } else {
            return Err("F2LLM WebGPU vector scoring requires the shader-f16 feature".to_owned());
        };
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                required_features,
                ..wgpu::DeviceDescriptor::default()
            })
            .await
            .map_err(|error| format!("failed to request WebGPU device: {error}"))?;

        let tokenizer_bytes = fetch_tokenizer_bytes(
            &fetch_array_buffer,
            &options.base_url,
            &manifest.tokenizer,
        )
        .await?;
        let tokenizer =
            QwenByteBpeTokenizer::from_compact_json(&tokenizer_bytes).map_err(|error| {
                format!("failed to initialize F2LLM tokenizer from artifact: {error}")
            })?;
        let mut runtime = Self {
            device,
            queue,
            max_sequence_length: options
                .max_sequence_length
                .max(manifest.max_sequence_length.unwrap_or(DEFAULT_MAX_SEQUENCE_LENGTH)),
            dimensions: options.dimensions,
            manifest,
            tokenizer,
            tensors: HashMap::new(),
            pipelines: HashMap::new(),
            transient_buffers: Vec::new(),
            vector_buffers: HashMap::new(),
        };
        runtime
            .load_tensors(&options.base_url, &fetch_array_buffer, &progress)
            .await?;
        validate_required_tensors(&runtime.manifest, &runtime.tensors)?;
        Ok(runtime)
    }

    #[requires(!base_url.is_empty())]
    #[ensures(ret.is_ok() || ret.is_err())]
    async fn load_tensors(
        &mut self,
        base_url: &str,
        fetch_array_buffer: &Function,
        progress: &Option<Function>,
    ) -> Result<(), String> {
        let entries = self
            .manifest
            .tensors
            .iter()
            .map(|(name, spec)| (name.clone(), spec.clone_for_load()))
            .collect::<Vec<_>>();
        let total_bytes = entries
            .iter()
            .map(|(_, spec)| tensor_byte_length(spec))
            .sum::<usize>();
        let mut loaded_bytes = 0;
        for (name, spec) in entries {
            call_progress(
                progress,
                "downloading-model",
                &format!("Downloading F2LLM WebGPU tensor {name}."),
                loaded_bytes,
                total_bytes,
            )
            .await?;
            let tensor = self
                .load_tensor(base_url, fetch_array_buffer, &name, &spec)
                .await?;
            loaded_bytes += tensor_byte_length(&spec);
            self.tensors.insert(name, tensor);
        }
        call_progress(
            progress,
            "loading-model",
            "F2LLM WebGPU artifact is ready.",
            total_bytes,
            total_bytes,
        )
        .await?;
        Ok(())
    }

    #[requires(!base_url.is_empty())]
    #[requires(!name.is_empty())]
    #[ensures(ret.as_ref().is_ok() || ret.is_err())]
    async fn load_tensor(
        &self,
        base_url: &str,
        fetch_array_buffer: &Function,
        name: &str,
        spec: &TensorSpec,
    ) -> Result<Tensor, String> {
        match spec.kind.as_str() {
            "q4_onnx_gather" | "q4_onnx_matmul" => {
                let qweight = spec
                    .qweight
                    .as_ref()
                    .ok_or_else(|| format!("{name} is missing qweight"))?;
                let scales = spec
                    .scales
                    .as_ref()
                    .ok_or_else(|| format!("{name} is missing scales"))?;
                let zero_points = spec
                    .zero_points
                    .as_ref()
                    .ok_or_else(|| format!("{name} is missing zero_points"))?;
                let q_buffer = self
                    .load_chunked_buffer(base_url, fetch_array_buffer, &format!("{name}.qweight"), qweight)
                    .await?;
                let scale_buffer = self
                    .load_chunked_buffer(base_url, fetch_array_buffer, &format!("{name}.scales"), scales)
                    .await?;
                let zero_point_buffer = self
                    .load_chunked_buffer(
                        base_url,
                        fetch_array_buffer,
                        &format!("{name}.zero_points"),
                        zero_points,
                    )
                    .await?;
                let shape = checked_rank2_shape(name, &spec.shape)?;
                let tensor = Q4Tensor {
                    shape,
                    group_size: spec
                        .group_size
                        .ok_or_else(|| format!("{name} is missing group_size"))?,
                    groups: spec
                        .groups
                        .ok_or_else(|| format!("{name} is missing groups"))?,
                    q_buffer,
                    scale_buffer,
                    zero_point_buffer,
                };
                if spec.kind == "q4_onnx_gather" {
                    Ok(Tensor::Q4OnnxGather(tensor))
                } else {
                    Ok(Tensor::Q4OnnxMatmul(tensor))
                }
            }
            "f32" => {
                let data = spec
                    .data
                    .as_ref()
                    .ok_or_else(|| format!("{name} is missing f32 data"))?;
                Ok(Tensor::F32(F32Tensor {
                    shape: spec.shape.clone(),
                    buffer: self
                        .load_chunked_buffer(base_url, fetch_array_buffer, name, data)
                        .await?,
                }))
            }
            other => Err(format!("unsupported F2LLM tensor kind for {name}: {other}")),
        }
    }

    #[requires(!base_url.is_empty())]
    #[requires(!label.is_empty())]
    #[requires(chunked.byte_length > 0)]
    #[ensures(ret.as_ref().is_ok() || ret.is_err())]
    async fn load_chunked_buffer(
        &self,
        base_url: &str,
        fetch_array_buffer: &Function,
        label: &str,
        chunked: &ChunkedSpec,
    ) -> Result<wgpu::Buffer, String> {
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: align_to(chunked.byte_length as u64, 4),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        for chunk in &chunked.chunks {
            let url = format!("{base_url}/{}", chunk.url);
            let bytes = fetch_bytes(fetch_array_buffer, &url, label).await?;
            if bytes.len() != chunk.byte_length {
                return Err(format!(
                    "{label} chunk {} has the wrong byte length: expected {}, got {}",
                    chunk.url,
                    chunk.byte_length,
                    bytes.len()
                ));
            }
            verify_sha256(&bytes, &chunk.sha256, &format!("{label} chunk {}", chunk.url))?;
            self.queue
                .write_buffer(&buffer, chunk.byte_offset as u64, &bytes);
        }
        self.queue.submit([]);
        Ok(buffer)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|vectors| vectors.len() == texts.len()) || ret.is_err())]
    async fn embed_texts(&mut self, texts: &[String]) -> Result<Vec<Vec<f32>>, String> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }
        let mut windows = Vec::new();
        let mut window_counts = vec![0usize; texts.len()];
        for (text_index, text) in texts.iter().enumerate() {
            for token_ids in self.tokenizer.token_windows(text, self.max_sequence_length)? {
                window_counts[text_index] += 1;
                windows.push(TokenWindow {
                    text_index,
                    token_ids,
                });
            }
        }
        let mut window_vectors = vec![Vec::<Vec<f32>>::new(); texts.len()];
        for batch in pack_token_windows(&windows, self.max_sequence_length) {
            let rows = self.embed_packed_batch(&batch.segments).await?;
            if rows.len() != batch.segments.len() {
                return Err(format!(
                    "packed batch row count mismatch: expected {}, got {}",
                    batch.segments.len(),
                    rows.len()
                ));
            }
            for (segment, vector) in batch.segments.iter().zip(rows) {
                window_vectors[segment.text_index].push(vector);
            }
        }
        let mut output = Vec::with_capacity(texts.len());
        for (text_index, vectors) in window_vectors.into_iter().enumerate() {
            if vectors.len() != window_counts[text_index] || vectors.is_empty() {
                return Err(format!(
                    "missing F2LLM window embeddings for text row {text_index}"
                ));
            }
            output.push(mean_pool_normalized(&vectors, self.dimensions));
        }
        Ok(output)
    }

    #[requires(!segments.is_empty())]
    #[requires(segments.iter().all(|segment| !segment.token_ids.is_empty()))]
    #[ensures(ret.as_ref().is_ok_and(|rows| rows.len() == segments.len()) || ret.is_err())]
    async fn embed_packed_batch(&mut self, segments: &[TokenWindow]) -> Result<Vec<Vec<f32>>, String> {
        let total_tokens = segments
            .iter()
            .map(|segment| segment.token_ids.len())
            .sum::<usize>();
        if total_tokens > self.max_sequence_length {
            return Err(format!(
                "packed F2LLM batch has {total_tokens} tokens, maximum is {}",
                self.max_sequence_length
            ));
        }
        let mut token_ids = Vec::with_capacity(total_tokens);
        let mut local_positions = Vec::with_capacity(total_tokens);
        let mut segment_starts = Vec::with_capacity(total_tokens);
        let mut last_token_offsets = Vec::with_capacity(segments.len());
        let mut offset = 0usize;
        for segment in segments {
            let start = offset;
            for (local, token_id) in segment.token_ids.iter().enumerate() {
                token_ids.push(*token_id);
                local_positions.push(local as u32);
                segment_starts.push(start as u32);
                offset += 1;
            }
            last_token_offsets.push(offset - 1);
        }
        let hidden = self
            .run_packed_model(&token_ids, &local_positions, &segment_starts)
            .await?;
        let hidden_values = self
            .read_f32_buffer_slice(
                &hidden,
                0,
                total_tokens * self.manifest.model.hidden_size,
            )
            .await?;
        hidden.destroy();
        let mut rows = Vec::with_capacity(segments.len());
        for token_offset in last_token_offsets {
            let start = token_offset * self.manifest.model.hidden_size;
            let end = start + self.manifest.model.hidden_size;
            let mut vector = hidden_values[start..end].to_vec();
            normalize_in_place(&mut vector);
            rows.push(vector);
        }
        Ok(rows)
    }

    #[requires(!token_ids.is_empty())]
    #[requires(token_ids.len() == local_positions.len())]
    #[requires(token_ids.len() == segment_starts.len())]
    #[ensures(ret.as_ref().is_ok() || ret.is_err())]
    async fn run_packed_model(
        &mut self,
        token_ids: &[u32],
        local_positions: &[u32],
        segment_starts: &[u32],
    ) -> Result<wgpu::Buffer, String> {
        let sequence_length = token_ids.len();
        let hidden_size = self.manifest.model.hidden_size;
        let token_buffer = self.create_upload_buffer(
            "f2llm packed token ids",
            bytemuck::cast_slice(token_ids),
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        );
        let local_position_buffer = self.create_upload_buffer(
            "f2llm packed local positions",
            bytemuck::cast_slice(local_positions),
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        );
        let segment_start_buffer = self.create_upload_buffer(
            "f2llm packed segment starts",
            bytemuck::cast_slice(segment_starts),
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        );
        let mut hidden = self.create_storage_buffer(
            "f2llm packed hidden states",
            sequence_length * hidden_size * 4,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        );
        {
            let mut encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("f2llm packed embedding"),
                });
            let embed = self.q4_gather_tensor("model.embed_tokens.weight")?;
            self.encode_embedding(&mut encoder, &token_buffer, &embed, &hidden, sequence_length)?;
            self.queue.submit([encoder.finish()]);
            self.submitted_work_done().await?;
            self.destroy_transient_buffers();
        }
        for layer in 0..self.manifest.model.num_hidden_layers {
            let old_hidden = hidden;
            hidden = self
                .run_packed_layer(
                    layer,
                    &old_hidden,
                    sequence_length,
                    &local_position_buffer,
                    &segment_start_buffer,
                )
                .await?;
            old_hidden.destroy();
        }
        let final_hidden = self.create_storage_buffer(
            "f2llm packed final norm hidden states",
            sequence_length * hidden_size * 4,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        );
        {
            let mut encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("f2llm packed final norm"),
                });
            let norm = self.f32_tensor("model.norm.weight")?;
            self.encode_rms_norm(
                &mut encoder,
                &hidden,
                &norm,
                &final_hidden,
                sequence_length,
                hidden_size,
                self.manifest.model.rms_norm_eps,
            )?;
            self.queue.submit([encoder.finish()]);
            self.submitted_work_done().await?;
            self.destroy_transient_buffers();
            hidden.destroy();
        }
        token_buffer.destroy();
        local_position_buffer.destroy();
        segment_start_buffer.destroy();
        Ok(final_hidden)
    }

    #[requires(sequence_length > 0)]
    #[ensures(ret.as_ref().is_ok() || ret.is_err())]
    async fn run_packed_layer(
        &mut self,
        layer: usize,
        hidden: &wgpu::Buffer,
        sequence_length: usize,
        local_position_buffer: &wgpu::Buffer,
        segment_start_buffer: &wgpu::Buffer,
    ) -> Result<wgpu::Buffer, String> {
        let model = self.manifest.model.clone();
        let prefix = format!("model.layers.{layer}");
        let hidden_bytes = sequence_length * model.hidden_size * 4;
        let q_cols = model.num_attention_heads * model.head_dim;
        let kv_cols = model.num_key_value_heads * model.head_dim;
        let mlp_bytes = sequence_length * model.intermediate_size * 4;
        let mut temp_buffers = Vec::new();
        let mut temp = |runtime: &Self, label: &str, bytes: usize| {
            let buffer = runtime.create_storage_buffer(
                label,
                bytes,
                wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            );
            temp_buffers.push(buffer.clone());
            buffer
        };
        let attn_norm = temp(self, "f2llm attention norm", hidden_bytes);
        let q = temp(self, "f2llm q", sequence_length * q_cols * 4);
        let k = temp(self, "f2llm k", sequence_length * kv_cols * 4);
        let q_norm = temp(self, "f2llm q norm", sequence_length * q_cols * 4);
        let k_norm = temp(self, "f2llm k norm", sequence_length * kv_cols * 4);
        let v = temp(self, "f2llm v", sequence_length * kv_cols * 4);
        let attention = temp(self, "f2llm attention", sequence_length * q_cols * 4);
        let attention_projected = temp(self, "f2llm attention projected", hidden_bytes);
        let post_attention = temp(self, "f2llm post attention", hidden_bytes);
        let mlp_norm = temp(self, "f2llm mlp norm", hidden_bytes);
        let gate = temp(self, "f2llm gate", mlp_bytes);
        let up = temp(self, "f2llm up", mlp_bytes);
        let activated = temp(self, "f2llm activated", mlp_bytes);
        let down = temp(self, "f2llm down", hidden_bytes);
        let next_hidden = self.create_storage_buffer(
            "f2llm layer output",
            hidden_bytes,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        );

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("f2llm packed layer"),
            });
        self.encode_named_rms_norm(
            &mut encoder,
            hidden,
            &format!("{prefix}.input_layernorm.weight"),
            &attn_norm,
            sequence_length,
            model.hidden_size,
            model.rms_norm_eps,
        )?;
        self.encode_named_linear_q4(
            &mut encoder,
            &attn_norm,
            &format!("{prefix}.self_attn.q_proj.weight"),
            &q,
            sequence_length,
            model.hidden_size,
            q_cols,
        )?;
        self.encode_named_linear_q4(
            &mut encoder,
            &attn_norm,
            &format!("{prefix}.self_attn.k_proj.weight"),
            &k,
            sequence_length,
            model.hidden_size,
            kv_cols,
        )?;
        self.encode_named_linear_q4(
            &mut encoder,
            &attn_norm,
            &format!("{prefix}.self_attn.v_proj.weight"),
            &v,
            sequence_length,
            model.hidden_size,
            kv_cols,
        )?;
        self.encode_named_rms_norm(
            &mut encoder,
            &q,
            &format!("{prefix}.self_attn.q_norm.weight"),
            &q_norm,
            sequence_length * model.num_attention_heads,
            model.head_dim,
            model.rms_norm_eps,
        )?;
        self.encode_named_rms_norm(
            &mut encoder,
            &k,
            &format!("{prefix}.self_attn.k_norm.weight"),
            &k_norm,
            sequence_length * model.num_key_value_heads,
            model.head_dim,
            model.rms_norm_eps,
        )?;
        self.encode_packed_rope(
            &mut encoder,
            &q_norm,
            local_position_buffer,
            sequence_length,
            model.num_attention_heads,
            model.head_dim,
            model.rope_theta,
        )?;
        self.encode_packed_rope(
            &mut encoder,
            &k_norm,
            local_position_buffer,
            sequence_length,
            model.num_key_value_heads,
            model.head_dim,
            model.rope_theta,
        )?;
        self.encode_packed_attention(
            &mut encoder,
            &q_norm,
            &k_norm,
            &v,
            segment_start_buffer,
            &attention,
            sequence_length,
        )?;
        self.encode_named_linear_q4(
            &mut encoder,
            &attention,
            &format!("{prefix}.self_attn.o_proj.weight"),
            &attention_projected,
            sequence_length,
            q_cols,
            model.hidden_size,
        )?;
        self.encode_add(
            &mut encoder,
            hidden,
            &attention_projected,
            &post_attention,
            sequence_length * model.hidden_size,
        )?;
        self.encode_named_rms_norm(
            &mut encoder,
            &post_attention,
            &format!("{prefix}.post_attention_layernorm.weight"),
            &mlp_norm,
            sequence_length,
            model.hidden_size,
            model.rms_norm_eps,
        )?;
        self.encode_named_linear_q4(
            &mut encoder,
            &mlp_norm,
            &format!("{prefix}.mlp.gate_proj.weight"),
            &gate,
            sequence_length,
            model.hidden_size,
            model.intermediate_size,
        )?;
        self.encode_named_linear_q4(
            &mut encoder,
            &mlp_norm,
            &format!("{prefix}.mlp.up_proj.weight"),
            &up,
            sequence_length,
            model.hidden_size,
            model.intermediate_size,
        )?;
        self.encode_silu_mul(
            &mut encoder,
            &gate,
            &up,
            &activated,
            sequence_length * model.intermediate_size,
        )?;
        self.encode_named_linear_q4(
            &mut encoder,
            &activated,
            &format!("{prefix}.mlp.down_proj.weight"),
            &down,
            sequence_length,
            model.intermediate_size,
            model.hidden_size,
        )?;
        self.encode_add(
            &mut encoder,
            &post_attention,
            &down,
            &next_hidden,
            sequence_length * model.hidden_size,
        )?;
        self.queue.submit([encoder.finish()]);
        self.submitted_work_done().await?;
        self.destroy_transient_buffers();
        for buffer in temp_buffers {
            buffer.destroy();
        }
        Ok(next_hidden)
    }

    #[requires(query.len() == corpus.dimensions)]
    #[requires(corpus.element_type == "f16le")]
    #[ensures(ret.as_ref().is_ok_and(|scores| scores.len() == corpus.row_count) || ret.is_err())]
    async fn score_f16_vectors(
        &mut self,
        corpus: &CorpusVectorSpec,
        query: &[f32],
        read_binary: &Function,
    ) -> Result<Vec<f32>, String> {
        let row_count = corpus.row_count;
        if row_count == 0 {
            return Ok(Vec::new());
        }
        let vector_buffer = self.vector_buffer_for_corpus(corpus, read_binary).await?;
        let query_buffer = self.create_upload_buffer(
            "f2llm search query",
            bytemuck::cast_slice(query),
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        );
        let scores_buffer = self.create_storage_buffer(
            "f2llm search scores",
            row_count * 4,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        );
        let params = u32_uniform(&[row_count as u32, corpus.dimensions as u32, 0, 0]);
        let params_buffer = self.create_upload_buffer(
            "f2llm search params",
            &params,
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        );
        let read_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("f2llm search scores readback"),
            size: align_to((row_count * 4) as u64, 4),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("f2llm search"),
            });
        self.encode_pipeline(
            &mut encoder,
            "vectorDotF16",
            VECTOR_DOT_F16_SHADER,
            &[
                &vector_buffer.buffer,
                &query_buffer,
                &scores_buffer,
                &params_buffer,
            ],
            (div_ceil(row_count as u32, VECTOR_WORKGROUP_SIZE), 1, 1),
        )?;
        encoder.copy_buffer_to_buffer(&scores_buffer, 0, &read_buffer, 0, (row_count * 4) as u64);
        self.queue.submit([encoder.finish()]);
        let bytes = self.map_read_buffer(&read_buffer, row_count * 4).await?;
        query_buffer.destroy();
        scores_buffer.destroy();
        params_buffer.destroy();
        read_buffer.destroy();
        Ok(bytes
            .chunks_exact(4)
            .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
            .collect())
    }

    #[requires(corpus.element_type == "f16le")]
    #[ensures(ret.as_ref().is_ok() || ret.is_err())]
    async fn vector_buffer_for_corpus(
        &mut self,
        corpus: &CorpusVectorSpec,
        read_binary: &Function,
    ) -> Result<VectorBuffer, String> {
        let cache_key = corpus_vector_cache_key(corpus);
        if !self.vector_buffers.contains_key(&cache_key) {
            let total_bytes = corpus
                .shards
                .iter()
                .map(|shard| shard.byte_len)
                .sum::<usize>();
            let expected_bytes = corpus.row_count * corpus.dimensions * 2;
            if total_bytes != expected_bytes {
                return Err(format!(
                    "f16 vector byte length mismatch: expected {expected_bytes}, got {total_bytes}"
                ));
            }
            let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("f2llm vectors"),
                size: align_to(total_bytes as u64, 4),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let mut offset = 0usize;
            for shard in &corpus.shards {
                let bytes = read_binary_bytes(read_binary, &shard.key).await?;
                if bytes.len() != shard.byte_len {
                    return Err(format!(
                        "vector shard {} has the wrong byte length: expected {}, got {}",
                        shard.key,
                        shard.byte_len,
                        bytes.len()
                    ));
                }
                self.queue.write_buffer(&buffer, offset as u64, &bytes);
                offset += bytes.len();
            }
            self.queue.submit([]);
            self.vector_buffers.insert(
                cache_key.clone(),
                VectorBuffer { buffer },
            );
        }
        self.vector_buffers
            .get(&cache_key)
            .cloned()
            .ok_or_else(|| "failed to cache F2LLM vector buffer".to_owned())
    }

    #[requires(!weight_name.is_empty())]
    #[requires(rows > 0)]
    #[requires(cols > 0)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn encode_named_rms_norm(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        input: &wgpu::Buffer,
        weight_name: &str,
        output: &wgpu::Buffer,
        rows: usize,
        cols: usize,
        eps: f32,
    ) -> Result<(), String> {
        let weight = self.f32_tensor(weight_name)?;
        self.encode_rms_norm(encoder, input, &weight, output, rows, cols, eps)
    }

    #[requires(!weight_name.is_empty())]
    #[requires(rows > 0)]
    #[requires(in_cols > 0)]
    #[requires(out_cols > 0)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn encode_named_linear_q4(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        input: &wgpu::Buffer,
        weight_name: &str,
        output: &wgpu::Buffer,
        rows: usize,
        in_cols: usize,
        out_cols: usize,
    ) -> Result<(), String> {
        let weight = self.q4_matmul_tensor(weight_name)?;
        self.encode_linear_q4(encoder, input, &weight, output, rows, in_cols, out_cols)
    }

    #[requires(sequence_length > 0)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn encode_embedding(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        tokens: &wgpu::Buffer,
        weight: &Q4Tensor,
        output: &wgpu::Buffer,
        sequence_length: usize,
    ) -> Result<(), String> {
        if weight.shape[1] != self.manifest.model.hidden_size {
            return Err(format!(
                "embedding hidden size mismatch: expected {}, got {}",
                self.manifest.model.hidden_size, weight.shape[1]
            ));
        }
        let params = u32_uniform(&[
            sequence_length as u32,
            self.manifest.model.hidden_size as u32,
            weight.groups as u32,
            weight.group_size as u32,
        ]);
        let params = self.create_uniform_buffer("f2llm embedding params", &params);
        self.encode_pipeline(
            encoder,
            "embeddingOnnxQ4",
            EMBEDDING_ONNX_Q4_SHADER,
            &[
                tokens,
                &weight.q_buffer,
                &weight.scale_buffer,
                &weight.zero_point_buffer,
                output,
                &params,
            ],
            (
                div_ceil(sequence_length as u32, DEFAULT_WORKGROUP_WIDTH),
                div_ceil(self.manifest.model.hidden_size as u32, DEFAULT_WORKGROUP_WIDTH),
                1,
            ),
        )
    }

    #[requires(rows > 0)]
    #[requires(cols > 0)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn encode_rms_norm(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        input: &wgpu::Buffer,
        weight: &F32Tensor,
        output: &wgpu::Buffer,
        rows: usize,
        cols: usize,
        eps: f32,
    ) -> Result<(), String> {
        if weight.shape.first().copied() != Some(cols) {
            return Err(format!(
                "RMSNorm dimension mismatch: expected {cols}, got {:?}",
                weight.shape
            ));
        }
        let params = mixed_uniform(&[
            UniformValue::U32(rows as u32),
            UniformValue::U32(cols as u32),
            UniformValue::F32(eps),
            UniformValue::U32(0),
        ]);
        let params = self.create_uniform_buffer("f2llm rms norm params", &params);
        self.encode_pipeline(
            encoder,
            "rmsNorm",
            RMS_NORM_SHADER,
            &[input, &weight.buffer, output, &params],
            (
                div_ceil(rows as u32, DEFAULT_WORKGROUP_WIDTH),
                div_ceil(cols as u32, DEFAULT_WORKGROUP_WIDTH),
                1,
            ),
        )
    }

    #[requires(rows > 0)]
    #[requires(in_cols > 0)]
    #[requires(out_cols > 0)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn encode_linear_q4(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        input: &wgpu::Buffer,
        weight: &Q4Tensor,
        output: &wgpu::Buffer,
        rows: usize,
        in_cols: usize,
        out_cols: usize,
    ) -> Result<(), String> {
        if weight.shape != [out_cols, in_cols] {
            return Err(format!(
                "linear shape mismatch: expected [{out_cols}, {in_cols}], got {:?}",
                weight.shape
            ));
        }
        let params = u32_uniform(&[
            rows as u32,
            in_cols as u32,
            out_cols as u32,
            weight.group_size as u32,
            weight.groups as u32,
            0,
            0,
            0,
        ]);
        let params = self.create_uniform_buffer("f2llm linear params", &params);
        self.encode_pipeline(
            encoder,
            "linearOnnxQ4",
            LINEAR_ONNX_Q4_SHADER,
            &[
                input,
                &weight.q_buffer,
                &weight.scale_buffer,
                &weight.zero_point_buffer,
                output,
                &params,
            ],
            (
                div_ceil(rows as u32, DEFAULT_WORKGROUP_WIDTH),
                div_ceil(out_cols as u32, DEFAULT_WORKGROUP_WIDTH),
                1,
            ),
        )
    }

    #[requires(sequence_length > 0)]
    #[requires(head_dim > 0)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn encode_packed_rope(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        values: &wgpu::Buffer,
        local_position_buffer: &wgpu::Buffer,
        sequence_length: usize,
        heads: usize,
        head_dim: usize,
        theta: f32,
    ) -> Result<(), String> {
        if head_dim % 2 != 0 {
            return Err(format!("RoPE head dimension must be even, got {head_dim}"));
        }
        let params = mixed_uniform(&[
            UniformValue::U32(sequence_length as u32),
            UniformValue::U32(heads as u32),
            UniformValue::U32(head_dim as u32),
            UniformValue::U32(0),
            UniformValue::F32(theta),
            UniformValue::U32(0),
            UniformValue::U32(0),
            UniformValue::U32(0),
        ]);
        let params = self.create_uniform_buffer("f2llm packed rope params", &params);
        self.encode_pipeline(
            encoder,
            "packedRope",
            PACKED_ROPE_SHADER,
            &[values, local_position_buffer, &params],
            (
                div_ceil(sequence_length as u32, DEFAULT_WORKGROUP_WIDTH),
                div_ceil(heads as u32, DEFAULT_WORKGROUP_WIDTH),
                div_ceil((head_dim / 2) as u32, 1),
            ),
        )
    }

    #[requires(sequence_length > 0)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn encode_packed_attention(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        q: &wgpu::Buffer,
        k: &wgpu::Buffer,
        v: &wgpu::Buffer,
        segment_start_buffer: &wgpu::Buffer,
        output: &wgpu::Buffer,
        sequence_length: usize,
    ) -> Result<(), String> {
        let q_heads = self.manifest.model.num_attention_heads;
        let kv_heads = self.manifest.model.num_key_value_heads;
        let head_dim = self.manifest.model.head_dim;
        let scale = 1.0 / (head_dim as f32).sqrt();
        let params = mixed_uniform(&[
            UniformValue::U32(sequence_length as u32),
            UniformValue::U32(q_heads as u32),
            UniformValue::U32(kv_heads as u32),
            UniformValue::U32(head_dim as u32),
            UniformValue::F32(scale),
            UniformValue::U32(0),
            UniformValue::U32(0),
            UniformValue::U32(0),
        ]);
        let params = self.create_uniform_buffer("f2llm packed attention params", &params);
        self.encode_pipeline(
            encoder,
            "packedAttention",
            PACKED_ATTENTION_SHADER,
            &[q, k, v, segment_start_buffer, output, &params],
            (
                div_ceil(sequence_length as u32, 4),
                div_ceil(q_heads as u32, 4),
                div_ceil(head_dim as u32, 4),
            ),
        )
    }

    #[requires(len > 0)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn encode_add(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        left: &wgpu::Buffer,
        right: &wgpu::Buffer,
        output: &wgpu::Buffer,
        len: usize,
    ) -> Result<(), String> {
        let params = u32_uniform(&[len as u32, 0, 0, 0]);
        let params = self.create_uniform_buffer("f2llm add params", &params);
        self.encode_pipeline(
            encoder,
            "add",
            ADD_SHADER,
            &[left, right, output, &params],
            (div_ceil(len as u32, 256), 1, 1),
        )
    }

    #[requires(len > 0)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn encode_silu_mul(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        gate: &wgpu::Buffer,
        up: &wgpu::Buffer,
        output: &wgpu::Buffer,
        len: usize,
    ) -> Result<(), String> {
        let params = u32_uniform(&[len as u32, 0, 0, 0]);
        let params = self.create_uniform_buffer("f2llm silu params", &params);
        self.encode_pipeline(
            encoder,
            "siluMul",
            SILU_MUL_SHADER,
            &[gate, up, output, &params],
            (div_ceil(len as u32, 256), 1, 1),
        )
    }

    #[requires(!name.is_empty())]
    #[requires(!shader.is_empty())]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn encode_pipeline(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        name: &'static str,
        shader: &'static str,
        buffers: &[&wgpu::Buffer],
        workgroups: (u32, u32, u32),
    ) -> Result<(), String> {
        if !self.pipelines.contains_key(name) {
            let module = self.device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(name),
                source: wgpu::ShaderSource::Wgsl(shader.into()),
            });
            let pipeline = self
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some(name),
                    layout: None,
                    module: &module,
                    entry_point: Some("main"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    cache: None,
                });
            self.pipelines.insert(name, pipeline);
        }
        let pipeline = self
            .pipelines
            .get(name)
            .ok_or_else(|| format!("missing F2LLM pipeline {name}"))?;
        let entries = buffers
            .iter()
            .enumerate()
            .map(|(index, buffer)| wgpu::BindGroupEntry {
                binding: index as u32,
                resource: buffer.as_entire_binding(),
            })
            .collect::<Vec<_>>();
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(name),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &entries,
        });
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some(name),
            timestamp_writes: None,
        });
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.dispatch_workgroups(workgroups.0, workgroups.1, workgroups.2);
        Ok(())
    }

    #[requires(byte_length > 0)]
    #[ensures(true)]
    fn create_storage_buffer(
        &self,
        label: &str,
        byte_length: usize,
        usage: wgpu::BufferUsages,
    ) -> wgpu::Buffer {
        self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: align_to(byte_length as u64, 4),
            usage,
            mapped_at_creation: false,
        })
    }

    #[requires(true)]
    #[ensures(true)]
    fn create_upload_buffer(
        &self,
        label: &str,
        data: &[u8],
        usage: wgpu::BufferUsages,
    ) -> wgpu::Buffer {
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: align_to(data.len() as u64, 4),
            usage,
            mapped_at_creation: false,
        });
        if !data.is_empty() {
            self.queue.write_buffer(&buffer, 0, data);
        }
        buffer
    }

    #[requires(true)]
    #[ensures(true)]
    fn create_uniform_buffer(&mut self, label: &str, data: &[u8]) -> wgpu::Buffer {
        let buffer = self.create_upload_buffer(
            label,
            data,
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        );
        self.transient_buffers.push(buffer.clone());
        buffer
    }

    #[requires(true)]
    #[ensures(self.transient_buffers.is_empty())]
    fn destroy_transient_buffers(&mut self) {
        for buffer in self.transient_buffers.drain(..) {
            buffer.destroy();
        }
    }

    #[requires(elements > 0)]
    #[ensures(ret.as_ref().is_ok_and(|values| values.len() == elements) || ret.is_err())]
    async fn read_f32_buffer_slice(
        &self,
        buffer: &wgpu::Buffer,
        byte_offset: usize,
        elements: usize,
    ) -> Result<Vec<f32>, String> {
        let byte_length = elements * 4;
        let read_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("f2llm readback"),
            size: align_to(byte_length as u64, 4),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("f2llm readback"),
            });
        encoder.copy_buffer_to_buffer(
            buffer,
            byte_offset as u64,
            &read_buffer,
            0,
            byte_length as u64,
        );
        self.queue.submit([encoder.finish()]);
        let bytes = self.map_read_buffer(&read_buffer, byte_length).await?;
        read_buffer.destroy();
        Ok(bytes
            .chunks_exact(4)
            .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
            .collect())
    }

    #[requires(byte_length > 0)]
    #[ensures(ret.as_ref().is_ok_and(|bytes| bytes.len() == byte_length) || ret.is_err())]
    async fn map_read_buffer(
        &self,
        buffer: &wgpu::Buffer,
        byte_length: usize,
    ) -> Result<Vec<u8>, String> {
        let slice = buffer.slice(0..align_to(byte_length as u64, 4));
        let (sender, receiver) = oneshot::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result.map_err(|error| error.to_string()));
        });
        receiver
            .await
            .map_err(|_| "F2LLM readback map callback was dropped".to_owned())??;
        let view = slice.get_mapped_range();
        let bytes = view[..byte_length].to_vec();
        drop(view);
        buffer.unmap();
        Ok(bytes)
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    async fn submitted_work_done(&self) -> Result<(), String> {
        let (sender, receiver) = oneshot::channel();
        self.queue.on_submitted_work_done(move || {
            let _ = sender.send(());
        });
        receiver
            .await
            .map_err(|_| "F2LLM submitted-work callback was dropped".to_owned())
    }

    #[requires(!name.is_empty())]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn q4_gather_tensor(&self, name: &str) -> Result<Q4Tensor, String> {
        match self.tensors.get(name) {
            Some(Tensor::Q4OnnxGather(tensor)) => Ok(tensor.clone()),
            _ => Err(format!("missing q4 gather F2LLM tensor: {name}")),
        }
    }

    #[requires(!name.is_empty())]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn q4_matmul_tensor(&self, name: &str) -> Result<Q4Tensor, String> {
        match self.tensors.get(name) {
            Some(Tensor::Q4OnnxMatmul(tensor)) => Ok(tensor.clone()),
            _ => Err(format!("missing q4 matmul F2LLM tensor: {name}")),
        }
    }

    #[requires(!name.is_empty())]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn f32_tensor(&self, name: &str) -> Result<F32Tensor, String> {
        match self.tensors.get(name) {
            Some(Tensor::F32(tensor)) => Ok(tensor.clone()),
            _ => Err(format!("missing f32 F2LLM tensor: {name}")),
        }
    }
}

impl TensorSpec {
    #[requires(true)]
    #[ensures(true)]
    fn clone_for_load(&self) -> Self {
        self.clone()
    }
}

#[invariant(::U32(_) => true)]
#[invariant(::F32(_) => true)]
#[derive(Debug, Clone, Copy)]
enum UniformValue {
    U32(u32),
    F32(f32),
}

#[requires(!manifest.model_key.is_empty())]
#[ensures(ret.is_ok() || ret.is_err())]
fn validate_manifest(manifest: &ArtifactManifest, options: &RuntimeLoadOptions) -> Result<(), String> {
    for (field, actual, expected) in [
        ("schema_version", manifest.schema_version.to_string(), EXPECTED_SCHEMA_VERSION.to_string()),
        ("runtime", manifest.runtime.clone(), options.expected_runtime.clone()),
        (
            "artifact_version",
            manifest.artifact_version.clone(),
            options.expected_version.clone(),
        ),
        ("model_key", manifest.model_key.clone(), options.expected_model_key.clone()),
    ] {
        if actual != expected {
            return Err(format!(
                "F2LLM WebGPU manifest {field} mismatch: expected {expected}, got {actual}"
            ));
        }
    }
    for (field, value) in [
        ("vocab_size", manifest.model.vocab_size),
        ("hidden_size", manifest.model.hidden_size),
        ("num_hidden_layers", manifest.model.num_hidden_layers),
        ("num_attention_heads", manifest.model.num_attention_heads),
        ("num_key_value_heads", manifest.model.num_key_value_heads),
        ("head_dim", manifest.model.head_dim),
        ("intermediate_size", manifest.model.intermediate_size),
    ] {
        if value == 0 {
            return Err(format!("F2LLM WebGPU manifest model.{field} must be positive"));
        }
    }
    if manifest.model.hidden_size != options.dimensions {
        return Err(format!(
            "F2LLM WebGPU hidden size mismatch: expected {}, got {}",
            options.dimensions, manifest.model.hidden_size
        ));
    }
    Ok(())
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn validate_required_tensors(
    manifest: &ArtifactManifest,
    tensors: &HashMap<String, Tensor>,
) -> Result<(), String> {
    let mut required = vec!["model.embed_tokens.weight".to_owned(), "model.norm.weight".to_owned()];
    for layer in 0..manifest.model.num_hidden_layers {
        let prefix = format!("model.layers.{layer}");
        required.extend([
            format!("{prefix}.input_layernorm.weight"),
            format!("{prefix}.post_attention_layernorm.weight"),
            format!("{prefix}.self_attn.q_norm.weight"),
            format!("{prefix}.self_attn.k_norm.weight"),
            format!("{prefix}.self_attn.q_proj.weight"),
            format!("{prefix}.self_attn.k_proj.weight"),
            format!("{prefix}.self_attn.v_proj.weight"),
            format!("{prefix}.self_attn.o_proj.weight"),
            format!("{prefix}.mlp.gate_proj.weight"),
            format!("{prefix}.mlp.up_proj.weight"),
            format!("{prefix}.mlp.down_proj.weight"),
        ]);
    }
    let missing = required
        .into_iter()
        .filter(|name| !tensors.contains_key(name))
        .collect::<Vec<_>>();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "F2LLM WebGPU artifact is missing required tensors: {}",
            missing.join(", ")
        ))
    }
}

#[requires(true)]
#[ensures(true)]
fn tensor_byte_length(spec: &TensorSpec) -> usize {
    match spec.kind.as_str() {
        "q4_onnx_gather" | "q4_onnx_matmul" => {
            spec.qweight.as_ref().map_or(0, |chunked| chunked.byte_length)
                + spec.scales.as_ref().map_or(0, |chunked| chunked.byte_length)
                + spec.zero_points.as_ref().map_or(0, |chunked| chunked.byte_length)
        }
        "f32" => spec.data.as_ref().map_or(0, |chunked| chunked.byte_length),
        _ => 0,
    }
}

#[requires(!base_url.is_empty())]
#[requires(!tokenizer.url.is_empty())]
#[ensures(ret.as_ref().is_ok_and(|bytes| bytes.len() == tokenizer.byte_length) || ret.is_err())]
async fn fetch_tokenizer_bytes(
    fetch_array_buffer: &Function,
    base_url: &str,
    tokenizer: &TokenizerSpec,
) -> Result<Vec<u8>, String> {
    let url = format!("{base_url}/{}", tokenizer.url);
    let bytes = fetch_bytes(fetch_array_buffer, &url, "F2LLM tokenizer").await?;
    if bytes.len() != tokenizer.byte_length {
        return Err(format!(
            "F2LLM tokenizer byte length mismatch: expected {}, got {}",
            tokenizer.byte_length,
            bytes.len()
        ));
    }
    verify_sha256(
        &bytes,
        &tokenizer.canonical_json_sha256,
        "F2LLM tokenizer canonical JSON",
    )?;
    Ok(bytes)
}

#[requires(!url.is_empty())]
#[requires(!label.is_empty())]
#[ensures(ret.as_ref().is_ok() || ret.is_err())]
async fn fetch_bytes(fetch_array_buffer: &Function, url: &str, label: &str) -> Result<Vec<u8>, String> {
    let value = fetch_array_buffer
        .call2(&JsValue::NULL, &JsValue::from_str(url), &JsValue::from_str(label))
        .map_err(|error| js_value_message(&error))?;
    let promise = Promise::from(value);
    let value = JsFuture::from(promise)
        .await
        .map_err(|error| js_value_message(&error))?;
    bytes_from_js(&value).map_err(|error| js_value_message(&error))
}

#[requires(!key.is_empty())]
#[ensures(ret.as_ref().is_ok() || ret.is_err())]
async fn read_binary_bytes(read_binary: &Function, key: &str) -> Result<Vec<u8>, String> {
    let value = read_binary
        .call1(&JsValue::NULL, &JsValue::from_str(key))
        .map_err(|error| js_value_message(&error))?;
    let promise = Promise::from(value);
    let value = JsFuture::from(promise)
        .await
        .map_err(|error| js_value_message(&error))?;
    bytes_from_js(&value).map_err(|error| js_value_message(&error))
}

#[requires(!name.is_empty())]
#[requires(expected.len() == 64)]
#[ensures(ret.is_ok() || ret.is_err())]
fn verify_sha256(bytes: &[u8], expected: &str, name: &str) -> Result<(), String> {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let actual = format!("{:x}", hasher.finalize());
    if actual == expected {
        Ok(())
    } else {
        Err(format!("{name} SHA-256 mismatch"))
    }
}

#[requires(!name.is_empty())]
#[ensures(ret.is_ok() || ret.is_err())]
fn checked_rank2_shape(name: &str, shape: &[usize]) -> Result<[usize; 2], String> {
    if shape.len() != 2 || shape[0] == 0 || shape[1] == 0 {
        Err(format!("{name} must have rank 2 positive shape"))
    } else {
        Ok([shape[0], shape[1]])
    }
}

#[requires(alignment > 0)]
#[ensures(ret % alignment == 0)]
fn align_to(value: u64, alignment: u64) -> u64 {
    value.div_ceil(alignment) * alignment
}

#[requires(denominator > 0)]
#[ensures(ret * denominator >= numerator)]
fn div_ceil(numerator: u32, denominator: u32) -> u32 {
    numerator.div_ceil(denominator)
}

#[requires(true)]
#[ensures(ret.len() % 16 == 0)]
fn u32_uniform(values: &[u32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(align_to((values.len() * 4) as u64, 16) as usize);
    for value in values {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    while bytes.len() % 16 != 0 {
        bytes.push(0);
    }
    bytes
}

#[requires(true)]
#[ensures(ret.len() % 16 == 0)]
fn mixed_uniform(values: &[UniformValue]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(align_to((values.len() * 4) as u64, 16) as usize);
    for value in values {
        match value {
            UniformValue::U32(value) => bytes.extend_from_slice(&value.to_le_bytes()),
            UniformValue::F32(value) => bytes.extend_from_slice(&value.to_le_bytes()),
        }
    }
    while bytes.len() % 16 != 0 {
        bytes.push(0);
    }
    bytes
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok() || ret.is_err())]
fn bytes_from_js(value: &JsValue) -> Result<Vec<u8>, JsValue> {
    if value.is_instance_of::<Uint8Array>() {
        return Ok(Uint8Array::new(value).to_vec());
    }
    if value.is_instance_of::<js_sys::ArrayBuffer>() {
        return Ok(Uint8Array::new(value).to_vec());
    }
    Err(JsValue::from_str("expected ArrayBuffer or Uint8Array"))
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok() || ret.is_err())]
fn parse_corpus_vector_spec(value: &JsValue) -> Result<CorpusVectorSpec, JsValue> {
    let shards_value = Reflect::get(value, &JsValue::from_str("shards"))?;
    let shards_array = Array::from(&shards_value);
    let mut shards = Vec::with_capacity(shards_array.length() as usize);
    for index in 0..shards_array.length() {
        let shard = shards_array.get(index);
        shards.push(CorpusShard {
            key: required_string(&shard, "key")?,
            byte_len: required_usize(&shard, "byteLen")?,
        });
    }
    Ok(CorpusVectorSpec {
        corpus_id: optional_string(value, "corpusId")?.unwrap_or_default(),
        input_hash: optional_string(value, "inputHash")?.unwrap_or_default(),
        row_count: required_usize(value, "rowCount")?,
        dimensions: required_usize(value, "dimensions")?,
        element_type: optional_string(value, "elementType")?.unwrap_or_else(|| "f32le".to_owned()),
        shards,
    })
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|value| !value.is_empty()) || ret.is_err())]
fn required_string(value: &JsValue, key: &str) -> Result<String, JsValue> {
    optional_string(value, key)?.ok_or_else(|| JsValue::from_str(&format!("{key} is required")))
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn optional_string(value: &JsValue, key: &str) -> Result<Option<String>, JsValue> {
    let field = Reflect::get(value, &JsValue::from_str(key))?;
    if field.is_undefined() || field.is_null() {
        Ok(None)
    } else {
        field
            .as_string()
            .map(Some)
            .ok_or_else(|| JsValue::from_str(&format!("{key} must be a string")))
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|value| *value > 0) || ret.is_err())]
fn required_usize(value: &JsValue, key: &str) -> Result<usize, JsValue> {
    optional_usize(value, key)?.ok_or_else(|| JsValue::from_str(&format!("{key} is required")))
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn optional_usize(value: &JsValue, key: &str) -> Result<Option<usize>, JsValue> {
    let field = Reflect::get(value, &JsValue::from_str(key))?;
    if field.is_undefined() || field.is_null() {
        return Ok(None);
    }
    let number = field
        .as_f64()
        .ok_or_else(|| JsValue::from_str(&format!("{key} must be a number")))?;
    if !number.is_finite() || number <= 0.0 || number.fract() != 0.0 {
        return Err(JsValue::from_str(&format!("{key} must be a positive integer")));
    }
    Ok(Some(number as usize))
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn corpus_vector_cache_key(corpus: &CorpusVectorSpec) -> String {
    let shards = corpus
        .shards
        .iter()
        .map(|shard| format!("{}:{}", shard.key, shard.byte_len))
        .collect::<Vec<_>>()
        .join("|");
    format!(
        "{}::{}::{}::{}::{}::{}",
        corpus.corpus_id,
        corpus.input_hash,
        corpus.row_count,
        corpus.dimensions,
        corpus.element_type,
        shards
    )
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
async fn call_progress(
    progress: &Option<Function>,
    status: &str,
    detail: &str,
    loaded: usize,
    total: usize,
) -> Result<(), String> {
    let Some(progress) = progress else {
        return Ok(());
    };
    let value = Object::new();
    Reflect::set(&value, &JsValue::from_str("status"), &JsValue::from_str(status))
        .map_err(|error| js_value_message(&error))?;
    Reflect::set(&value, &JsValue::from_str("detail"), &JsValue::from_str(detail))
        .map_err(|error| js_value_message(&error))?;
    let progress_value = Object::new();
    Reflect::set(
        &progress_value,
        &JsValue::from_str("kind"),
        &JsValue::from_str("model"),
    )
    .map_err(|error| js_value_message(&error))?;
    Reflect::set(
        &progress_value,
        &JsValue::from_str("loaded"),
        &JsValue::from_f64(loaded as f64),
    )
    .map_err(|error| js_value_message(&error))?;
    Reflect::set(
        &progress_value,
        &JsValue::from_str("total"),
        &JsValue::from_f64(total as f64),
    )
    .map_err(|error| js_value_message(&error))?;
    Reflect::set(&value, &JsValue::from_str("progress"), &progress_value)
        .map_err(|error| js_value_message(&error))?;
    let result = progress
        .call1(&JsValue::NULL, &value)
        .map_err(|error| js_value_message(&error))?;
    if result.is_instance_of::<Promise>() {
        JsFuture::from(Promise::from(result))
            .await
            .map_err(|error| js_value_message(&error))?;
    }
    Ok(())
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn js_value_message(value: &JsValue) -> String {
    value
        .as_string()
        .unwrap_or_else(|| format!("{value:?}"))
}

#[requires(!message.is_empty())]
#[ensures(true)]
fn js_error(message: String) -> JsValue {
    JsValue::from_str(&message)
}
