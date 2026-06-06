const EXPECTED_SCHEMA_VERSION = 1;
const DEFAULT_MAX_SEQUENCE_LENGTH = 512;
const DEFAULT_WORKGROUP_WIDTH = 8;
const VECTOR_WORKGROUP_SIZE = 64;
const TEXT_DECODER = new TextDecoder("utf-8", { fatal: true });
const TEXT_ENCODER = new TextEncoder();

const SHADERS = {
  embedding: `
struct Params {
  seq: u32,
  hidden: u32,
  groups: u32,
  group_size: u32,
};
@group(0) @binding(0) var<storage, read> tokens: array<u32>;
@group(0) @binding(1) var<storage, read> qbytes: array<u32>;
@group(0) @binding(2) var<storage, read> scales: array<f32>;
@group(0) @binding(3) var<storage, read_write> output: array<f32>;
@group(0) @binding(4) var<uniform> params: Params;

fn q4_byte(byte_index: u32) -> u32 {
  let word = qbytes[byte_index / 4u];
  let shift = (byte_index % 4u) * 8u;
  return (word >> shift) & 255u;
}

fn q4_value(row: u32, col: u32) -> f32 {
  let element = row * params.hidden + col;
  let packed = q4_byte(element / 2u);
  let nibble = select((packed >> 4u) & 15u, packed & 15u, (element & 1u) == 0u);
  let group = col / params.group_size;
  let scale = scales[row * params.groups + group];
  return (f32(i32(nibble)) - 8.0) * scale;
}

@compute @workgroup_size(${DEFAULT_WORKGROUP_WIDTH}, ${DEFAULT_WORKGROUP_WIDTH}, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
  let token_index = id.x;
  let dim = id.y;
  if (token_index >= params.seq || dim >= params.hidden) {
    return;
  }
  let token = tokens[token_index];
  output[token_index * params.hidden + dim] = q4_value(token, dim);
}
`,

  embeddingOnnxQ4: `
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

@compute @workgroup_size(${DEFAULT_WORKGROUP_WIDTH}, ${DEFAULT_WORKGROUP_WIDTH}, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
  let token_index = id.x;
  let dim = id.y;
  if (token_index >= params.seq || dim >= params.hidden) {
    return;
  }
  let token = tokens[token_index];
  output[token_index * params.hidden + dim] = q4_value(token, dim);
}
`,

  rmsNorm: `
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

@compute @workgroup_size(${DEFAULT_WORKGROUP_WIDTH}, ${DEFAULT_WORKGROUP_WIDTH}, 1)
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
`,

  linearQ4: `
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
@group(0) @binding(3) var<storage, read_write> output: array<f32>;
@group(0) @binding(4) var<uniform> params: Params;

fn q4_byte(byte_index: u32) -> u32 {
  let word = qbytes[byte_index / 4u];
  let shift = (byte_index % 4u) * 8u;
  return (word >> shift) & 255u;
}

fn weight_value(row: u32, col: u32) -> f32 {
  let element = row * params.in_cols + col;
  let packed = q4_byte(element / 2u);
  let nibble = select((packed >> 4u) & 15u, packed & 15u, (element & 1u) == 0u);
  let group = col / params.group_size;
  let scale = scales[row * params.groups + group];
  return (f32(i32(nibble)) - 8.0) * scale;
}

@compute @workgroup_size(${DEFAULT_WORKGROUP_WIDTH}, ${DEFAULT_WORKGROUP_WIDTH}, 1)
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
`,

  linearOnnxQ4: `
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

@compute @workgroup_size(${DEFAULT_WORKGROUP_WIDTH}, ${DEFAULT_WORKGROUP_WIDTH}, 1)
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
`,

  rope: `
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
@group(0) @binding(1) var<uniform> params: Params;

@compute @workgroup_size(${DEFAULT_WORKGROUP_WIDTH}, ${DEFAULT_WORKGROUP_WIDTH}, 1)
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
  let angle = f32(token) / pow(params.theta, exponent);
  let c = cos(angle);
  let s = sin(angle);
  let first = values[base + dim];
  let second = values[base + dim + half_dim];
  values[base + dim] = first * c - second * s;
  values[base + dim + half_dim] = second * c + first * s;
}
`,

  attention: `
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
@group(0) @binding(3) var<storage, read_write> output: array<f32>;
@group(0) @binding(4) var<uniform> params: Params;

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
  var max_score = -3.402823e38;
  for (var key_token = 0u; key_token <= token; key_token = key_token + 1u) {
    max_score = max(max_score, score_for(token, key_token, q_head, kv_head));
  }
  var denominator = 0.0;
  var weighted = 0.0;
  for (var key_token = 0u; key_token <= token; key_token = key_token + 1u) {
    let probability_weight = exp(score_for(token, key_token, q_head, kv_head) - max_score);
    denominator = denominator + probability_weight;
    let value_base = (key_token * params.kv_heads + kv_head) * params.head_dim;
    weighted = weighted + probability_weight * v[value_base + dim];
  }
  let out_base = (token * params.q_heads + q_head) * params.head_dim;
  output[out_base + dim] = weighted / denominator;
}
`,

  add: `
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
`,

  siluMul: `
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
`,

  vectorDotF16: `
struct Params {
  rows: u32,
  dims: u32,
  _pad0: u32,
  _pad1: u32,
};
@group(0) @binding(0) var<storage, read> vectors: array<u32>;
@group(0) @binding(1) var<storage, read> query: array<f32>;
@group(0) @binding(2) var<storage, read_write> scores: array<f32>;
@group(0) @binding(3) var<uniform> params: Params;

@compute @workgroup_size(${VECTOR_WORKGROUP_SIZE}, 1, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
  let row = id.x;
  if (row >= params.rows) {
    return;
  }
  var score = 0.0;
  let packed_base = row * (params.dims / 2u);
  for (var pair = 0u; pair < params.dims / 2u; pair = pair + 1u) {
    let values = unpack2x16float(vectors[packed_base + pair]);
    let dim = pair * 2u;
    score = score + values.x * query[dim] + values.y * query[dim + 1u];
  }
  scores[row] = score;
}
`,
};

export class F2LlmWebGpuRuntime {
  constructor({ device, manifest, tokenizer, tensors, maxSequenceLength, dimensions, fetchArrayBuffer }) {
    this.device = device;
    this.manifest = manifest;
    this.tokenizer = tokenizer;
    this.tensors = tensors;
    this.maxSequenceLength = maxSequenceLength;
    this.dimensions = dimensions;
    this.fetchArrayBuffer = fetchArrayBuffer;
    this.pipelines = new Map();
    this.vectorBuffers = new Map();
    this.transientBuffers = [];
    this.model = manifest.model;
  }

  static async load(options) {
    const baseUrl = normalizeBaseUrl(options.baseUrl);
    const fetchArrayBuffer = options.fetchArrayBuffer || defaultFetchArrayBuffer;
    const manifest = await fetchJsonWith(fetchArrayBuffer, `${baseUrl}/manifest.json`, "F2LLM WebGPU manifest");
    validateManifest(manifest, options);
    const adapter = await navigator.gpu.requestAdapter();
    if (adapter === null) {
      throw new Error("WebGPU adapter is unavailable.");
    }
    const device = await adapter.requestDevice();
    const tokenizer = await loadTokenizer(baseUrl, manifest, fetchArrayBuffer);
    const tensors = new Map();
    const runtime = new F2LlmWebGpuRuntime({
      device,
      manifest,
      tokenizer,
      tensors,
      maxSequenceLength: options.maxSequenceLength || manifest.max_sequence_length || DEFAULT_MAX_SEQUENCE_LENGTH,
      dimensions: options.dimensions || manifest.model.hidden_size,
      fetchArrayBuffer,
    });
    await runtime.loadTensors(baseUrl, options.progress || null);
    return runtime;
  }

  async loadTensors(baseUrl, progress) {
    const tensorEntries = Object.entries(this.manifest.tensors || {});
    let loadedBytes = 0;
    const totalBytes = tensorEntries.reduce((total, [, tensor]) =>
      total + tensorByteLength(tensor), 0);
    for (const [name, spec] of tensorEntries) {
      if (progress !== null) {
        await progress({
          status: "downloading-model",
          detail: `Downloading F2LLM WebGPU tensor ${name}.`,
          progress: progressValue("model", "F2LLM WebGPU artifact", loadedBytes, totalBytes),
        });
      }
      const tensor = await this.loadTensor(baseUrl, name, spec);
      this.tensors.set(name, tensor);
      loadedBytes += tensorByteLength(spec);
    }
    if (progress !== null) {
      await progress({
        status: "loading-model",
        detail: "F2LLM WebGPU artifact is ready.",
        progress: progressValue("model", "F2LLM WebGPU artifact", totalBytes, totalBytes),
      });
    }
    validateRequiredTensors(this.manifest, this.tensors);
  }

  async loadTensor(baseUrl, name, spec) {
    if (spec.kind === "q4_rowwise") {
      const qBuffer = await this.loadChunkedBuffer(baseUrl, `${name}.qweight`, spec.qweight);
      const scaleBuffer = await this.loadChunkedBuffer(baseUrl, `${name}.scales`, spec.scales);
      return {
        kind: spec.kind,
        shape: checkedShape(name, spec.shape, 2),
        groupSize: checkedPositiveInteger(spec.group_size, `${name}.group_size`),
        groups: checkedPositiveInteger(spec.groups, `${name}.groups`),
        qBuffer,
        scaleBuffer,
      };
    }
    if (spec.kind === "q4_onnx_gather" || spec.kind === "q4_onnx_matmul") {
      const qBuffer = await this.loadChunkedBuffer(baseUrl, `${name}.qweight`, spec.qweight);
      const scaleBuffer = await this.loadChunkedBuffer(baseUrl, `${name}.scales`, spec.scales);
      const zeroPointBuffer = await this.loadChunkedBuffer(baseUrl, `${name}.zero_points`, spec.zero_points);
      return {
        kind: spec.kind,
        shape: checkedShape(name, spec.shape, 2),
        groupSize: checkedPositiveInteger(spec.group_size, `${name}.group_size`),
        groups: checkedPositiveInteger(spec.groups, `${name}.groups`),
        qBuffer,
        scaleBuffer,
        zeroPointBuffer,
      };
    }
    if (spec.kind === "f32") {
      return {
        kind: spec.kind,
        shape: checkedShape(name, spec.shape, 1),
        buffer: await this.loadChunkedBuffer(baseUrl, name, spec.data),
      };
    }
    throw new Error(`unsupported F2LLM tensor kind for ${name}: ${spec.kind}`);
  }

  async loadChunkedBuffer(baseUrl, label, chunked) {
    const byteLength = checkedPositiveInteger(chunked?.byte_length, `${label}.byte_length`);
    const buffer = this.device.createBuffer({
      label,
      size: alignTo(byteLength, 4),
      usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST,
    });
    for (const chunk of chunked.chunks || []) {
      const chunkOffset = checkedNonNegativeInteger(chunk.byte_offset, `${label}.byte_offset`);
      const bytes = await this.fetchArrayBuffer(`${baseUrl}/${chunk.url}`, label);
      if (bytes.byteLength !== checkedPositiveInteger(chunk.byte_length, `${label}.chunk.byte_length`)) {
        throw new Error(`${label} chunk ${chunk.url} has the wrong byte length`);
      }
      await verifySha256(bytes, chunk.sha256, `${label} chunk ${chunk.url}`);
      this.device.queue.writeBuffer(buffer, chunkOffset, bytes);
    }
    return buffer;
  }

  async embedTexts(texts) {
    const output = [];
    for (const text of texts) {
      output.push(await this.embedText(String(text || "")));
    }
    return output;
  }

  async embedText(text) {
    const tokenIds = this.tokenizer.encode(text, this.maxSequenceLength);
    const hiddenBuffer = await this.runModel(tokenIds);
    try {
      const lastTokenOffset = (tokenIds.length - 1) * this.model.hidden_size * 4;
      const vector = await this.readF32BufferSlice(hiddenBuffer, lastTokenOffset, this.model.hidden_size);
      normalize(vector);
      return vector;
    } finally {
      hiddenBuffer.destroy();
    }
  }

  async runModel(tokenIds) {
    const sequenceLength = tokenIds.length;
    const hiddenSize = this.model.hidden_size;
    const tokenBuffer = this.createUploadBuffer("f2llm token ids", Uint32Array.from(tokenIds));
    let hidden = this.createStorageBuffer("f2llm hidden states", sequenceLength * hiddenSize * 4);
    {
      const encoder = this.device.createCommandEncoder({ label: "f2llm embedding" });
      const embed = this.q4Tensor("model.embed_tokens.weight");
      this.encodeEmbedding(encoder, tokenBuffer, embed, hidden, sequenceLength, hiddenSize);
      this.device.queue.submit([encoder.finish()]);
      await this.device.queue.onSubmittedWorkDone();
      this.destroyTransientBuffers();
      tokenBuffer.destroy();
    }

    for (let layer = 0; layer < this.model.num_hidden_layers; layer += 1) {
      const oldHidden = hidden;
      hidden = await this.runLayer(layer, oldHidden, sequenceLength);
      oldHidden.destroy();
    }

    const finalHidden = this.createStorageBuffer("f2llm final norm hidden states", sequenceLength * hiddenSize * 4);
    {
      const encoder = this.device.createCommandEncoder({ label: "f2llm final norm" });
      this.encodeRmsNorm(
        encoder,
        hidden,
        this.f32Tensor("model.norm.weight"),
        finalHidden,
        sequenceLength,
        hiddenSize,
        this.model.rms_norm_eps,
      );
      this.device.queue.submit([encoder.finish()]);
      await this.device.queue.onSubmittedWorkDone();
      this.destroyTransientBuffers();
      hidden.destroy();
    }
    return finalHidden;
  }

  async runLayer(layer, hidden, sequenceLength) {
    const model = this.model;
    const prefix = `model.layers.${layer}`;
    const hiddenBytes = sequenceLength * model.hidden_size * 4;
    const qCols = model.num_attention_heads * model.head_dim;
    const kvCols = model.num_key_value_heads * model.head_dim;
    const mlpBytes = sequenceLength * model.intermediate_size * 4;
    const tempBuffers = [];
    const temp = (label, bytes) => {
      const buffer = this.createStorageBuffer(label, bytes);
      tempBuffers.push(buffer);
      return buffer;
    };

    const attnNorm = temp(`f2llm layer ${layer} attention norm`, hiddenBytes);
    const q = temp(`f2llm layer ${layer} q`, sequenceLength * qCols * 4);
    const k = temp(`f2llm layer ${layer} k`, sequenceLength * kvCols * 4);
    const qNorm = temp(`f2llm layer ${layer} q norm`, sequenceLength * qCols * 4);
    const kNorm = temp(`f2llm layer ${layer} k norm`, sequenceLength * kvCols * 4);
    const v = temp(`f2llm layer ${layer} v`, sequenceLength * kvCols * 4);
    const attention = temp(`f2llm layer ${layer} attention`, sequenceLength * qCols * 4);
    const attentionProjected = temp(`f2llm layer ${layer} attention projected`, hiddenBytes);
    const postAttention = temp(`f2llm layer ${layer} post attention`, hiddenBytes);
    const mlpNorm = temp(`f2llm layer ${layer} mlp norm`, hiddenBytes);
    const gate = temp(`f2llm layer ${layer} gate`, mlpBytes);
    const up = temp(`f2llm layer ${layer} up`, mlpBytes);
    const activated = temp(`f2llm layer ${layer} activated`, mlpBytes);
    const down = temp(`f2llm layer ${layer} down`, hiddenBytes);
    const nextHidden = this.createStorageBuffer(`f2llm layer ${layer} output`, hiddenBytes);

    const encoder = this.device.createCommandEncoder({ label: `f2llm layer ${layer}` });
    this.encodeRmsNorm(
      encoder,
      hidden,
      this.f32Tensor(`${prefix}.input_layernorm.weight`),
      attnNorm,
      sequenceLength,
      model.hidden_size,
      model.rms_norm_eps,
    );
    this.encodeLinearQ4(encoder, attnNorm, this.q4Tensor(`${prefix}.self_attn.q_proj.weight`), q, sequenceLength, model.hidden_size, qCols);
    this.encodeLinearQ4(encoder, attnNorm, this.q4Tensor(`${prefix}.self_attn.k_proj.weight`), k, sequenceLength, model.hidden_size, kvCols);
    this.encodeLinearQ4(encoder, attnNorm, this.q4Tensor(`${prefix}.self_attn.v_proj.weight`), v, sequenceLength, model.hidden_size, kvCols);
    this.encodeRmsNorm(
      encoder,
      q,
      this.f32Tensor(`${prefix}.self_attn.q_norm.weight`),
      qNorm,
      sequenceLength * model.num_attention_heads,
      model.head_dim,
      model.rms_norm_eps,
    );
    this.encodeRmsNorm(
      encoder,
      k,
      this.f32Tensor(`${prefix}.self_attn.k_norm.weight`),
      kNorm,
      sequenceLength * model.num_key_value_heads,
      model.head_dim,
      model.rms_norm_eps,
    );
    this.encodeRope(encoder, qNorm, sequenceLength, model.num_attention_heads, model.head_dim, model.rope_theta);
    this.encodeRope(encoder, kNorm, sequenceLength, model.num_key_value_heads, model.head_dim, model.rope_theta);
    this.encodeAttention(encoder, qNorm, kNorm, v, attention, sequenceLength);
    this.encodeLinearQ4(encoder, attention, this.q4Tensor(`${prefix}.self_attn.o_proj.weight`), attentionProjected, sequenceLength, qCols, model.hidden_size);
    this.encodeAdd(encoder, hidden, attentionProjected, postAttention, sequenceLength * model.hidden_size);
    this.encodeRmsNorm(
      encoder,
      postAttention,
      this.f32Tensor(`${prefix}.post_attention_layernorm.weight`),
      mlpNorm,
      sequenceLength,
      model.hidden_size,
      model.rms_norm_eps,
    );
    this.encodeLinearQ4(encoder, mlpNorm, this.q4Tensor(`${prefix}.mlp.gate_proj.weight`), gate, sequenceLength, model.hidden_size, model.intermediate_size);
    this.encodeLinearQ4(encoder, mlpNorm, this.q4Tensor(`${prefix}.mlp.up_proj.weight`), up, sequenceLength, model.hidden_size, model.intermediate_size);
    this.encodeSiluMul(encoder, gate, up, activated, sequenceLength * model.intermediate_size);
    this.encodeLinearQ4(encoder, activated, this.q4Tensor(`${prefix}.mlp.down_proj.weight`), down, sequenceLength, model.intermediate_size, model.hidden_size);
    this.encodeAdd(encoder, postAttention, down, nextHidden, sequenceLength * model.hidden_size);
    this.device.queue.submit([encoder.finish()]);
    await this.device.queue.onSubmittedWorkDone();
    this.destroyTransientBuffers();
    for (const buffer of tempBuffers) {
      buffer.destroy();
    }
    return nextHidden;
  }

  async rankHits({ corpus, query, limit, itemMatches, readBinary }) {
    if ((corpus.elementType || "f32le") !== "f16le") {
      throw new Error(`F2LLM WebGPU search requires f16le vectors, got ${corpus.elementType}`);
    }
    if (query.length !== corpus.dimensions) {
      throw new Error(`query dimension mismatch: expected ${corpus.dimensions}, got ${query.length}`);
    }
    const rowCount = Math.min(corpus.items.length, corpus.rowCount || 0);
    if (rowCount === 0) {
      return [];
    }
    const vectorBuffer = await this.vectorBufferForCorpus(corpus, readBinary);
    const queryBuffer = this.createUploadBuffer("f2llm search query", query);
    const scoresBuffer = this.createStorageBuffer("f2llm search scores", rowCount * 4, GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_SRC);
    const params = uintUniform([rowCount, corpus.dimensions, 0, 0]);
    const paramsBuffer = this.createUploadBuffer("f2llm search params", params, GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST);
    const pipeline = this.pipeline("vectorDotF16", SHADERS.vectorDotF16);
    const bindGroup = this.device.createBindGroup({
      label: "f2llm search bind group",
      layout: pipeline.getBindGroupLayout(0),
      entries: [
        { binding: 0, resource: { buffer: vectorBuffer.buffer } },
        { binding: 1, resource: { buffer: queryBuffer } },
        { binding: 2, resource: { buffer: scoresBuffer } },
        { binding: 3, resource: { buffer: paramsBuffer } },
      ],
    });
    const readBuffer = this.device.createBuffer({
      label: "f2llm search scores readback",
      size: alignTo(rowCount * 4, 4),
      usage: GPUBufferUsage.COPY_DST | GPUBufferUsage.MAP_READ,
    });
    const encoder = this.device.createCommandEncoder({ label: "f2llm search" });
    const pass = encoder.beginComputePass({ label: "f2llm search dot" });
    pass.setPipeline(pipeline);
    pass.setBindGroup(0, bindGroup);
    pass.dispatchWorkgroups(Math.ceil(rowCount / VECTOR_WORKGROUP_SIZE));
    pass.end();
    encoder.copyBufferToBuffer(scoresBuffer, 0, readBuffer, 0, rowCount * 4);
    this.device.queue.submit([encoder.finish()]);
    await readBuffer.mapAsync(GPUMapMode.READ);
    const scores = new Float32Array(readBuffer.getMappedRange().slice(0));
    readBuffer.unmap();
    queryBuffer.destroy();
    scoresBuffer.destroy();
    paramsBuffer.destroy();
    readBuffer.destroy();
    return topHits(scores, corpus.items, limit, itemMatches);
  }

  async vectorBufferForCorpus(corpus, readBinary) {
    const cacheKey = corpusVectorCacheKey(corpus);
    const cached = this.vectorBuffers.get(cacheKey);
    if (cached !== undefined) {
      return cached;
    }
    const totalBytes = (corpus.shards || []).reduce((total, shard) => total + checkedNonNegativeInteger(shard.byteLen, "vector shard byteLen"), 0);
    const expectedBytes = (corpus.rowCount || 0) * corpus.dimensions * 2;
    if (totalBytes !== expectedBytes) {
      throw new Error(`f16 vector byte length mismatch: expected ${expectedBytes}, got ${totalBytes}`);
    }
    const buffer = this.device.createBuffer({
      label: `f2llm vectors ${corpus.corpusId || "corpus"}`,
      size: alignTo(totalBytes, 4),
      usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST,
    });
    let offset = 0;
    for (const shard of corpus.shards || []) {
      const bytes = await readBinary(shard.key);
      if (bytes.byteLength !== shard.byteLen) {
        throw new Error(`vector shard ${shard.key} has the wrong byte length`);
      }
      this.device.queue.writeBuffer(buffer, offset, bytes);
      offset += bytes.byteLength;
    }
    const value = { buffer, byteLength: totalBytes };
    this.vectorBuffers.set(cacheKey, value);
    return value;
  }

  encodeEmbedding(encoder, tokens, weight, output, sequenceLength, hiddenSize) {
    if (weight.shape[1] !== hiddenSize) {
      throw new Error(`embedding hidden size mismatch: expected ${hiddenSize}, got ${weight.shape[1]}`);
    }
    const params = uintUniform([sequenceLength, hiddenSize, weight.groups, weight.groupSize]);
    if (weight.kind === "q4_onnx_gather") {
      this.encodePipeline(encoder, "embeddingOnnxQ4", SHADERS.embeddingOnnxQ4, [
        tokens,
        weight.qBuffer,
        weight.scaleBuffer,
        weight.zeroPointBuffer,
        output,
        this.createUniformBuffer("f2llm embedding params", params),
      ], [Math.ceil(sequenceLength / DEFAULT_WORKGROUP_WIDTH), Math.ceil(hiddenSize / DEFAULT_WORKGROUP_WIDTH), 1]);
      return;
    }
    this.encodePipeline(encoder, "embedding", SHADERS.embedding, [
      tokens,
      weight.qBuffer,
      weight.scaleBuffer,
      output,
      this.createUniformBuffer("f2llm embedding params", params),
    ], [Math.ceil(sequenceLength / DEFAULT_WORKGROUP_WIDTH), Math.ceil(hiddenSize / DEFAULT_WORKGROUP_WIDTH), 1]);
  }

  encodeRmsNorm(encoder, input, weight, output, rows, cols, eps) {
    if (weight.shape[0] !== cols) {
      throw new Error(`RMSNorm dimension mismatch: expected ${cols}, got ${weight.shape[0]}`);
    }
    const params = mixedUniform((view) => {
      view.setUint32(0, rows, true);
      view.setUint32(4, cols, true);
      view.setFloat32(8, eps, true);
      view.setUint32(12, 0, true);
    }, 16);
    this.encodePipeline(encoder, "rmsNorm", SHADERS.rmsNorm, [
      input,
      weight.buffer,
      output,
      this.createUniformBuffer("f2llm rms norm params", params),
    ], [Math.ceil(rows / DEFAULT_WORKGROUP_WIDTH), Math.ceil(cols / DEFAULT_WORKGROUP_WIDTH), 1]);
  }

  encodeLinearQ4(encoder, input, weight, output, rows, inCols, outCols) {
    if (weight.shape[0] !== outCols || weight.shape[1] !== inCols) {
      throw new Error(
        `linear shape mismatch: expected [${outCols}, ${inCols}], got [${weight.shape.join(", ")}]`,
      );
    }
    const params = uintUniform([rows, inCols, outCols, weight.groupSize, weight.groups, 0, 0, 0]);
    if (weight.kind === "q4_onnx_matmul") {
      this.encodePipeline(encoder, "linearOnnxQ4", SHADERS.linearOnnxQ4, [
        input,
        weight.qBuffer,
        weight.scaleBuffer,
        weight.zeroPointBuffer,
        output,
        this.createUniformBuffer("f2llm linear params", params),
      ], [Math.ceil(rows / DEFAULT_WORKGROUP_WIDTH), Math.ceil(outCols / DEFAULT_WORKGROUP_WIDTH), 1]);
      return;
    }
    this.encodePipeline(encoder, "linearQ4", SHADERS.linearQ4, [
      input,
      weight.qBuffer,
      weight.scaleBuffer,
      output,
      this.createUniformBuffer("f2llm linear params", params),
    ], [Math.ceil(rows / DEFAULT_WORKGROUP_WIDTH), Math.ceil(outCols / DEFAULT_WORKGROUP_WIDTH), 1]);
  }

  encodeRope(encoder, values, sequenceLength, heads, headDim, theta) {
    if (headDim % 2 !== 0) {
      throw new Error(`RoPE head dimension must be even, got ${headDim}`);
    }
    const params = mixedUniform((view) => {
      view.setUint32(0, sequenceLength, true);
      view.setUint32(4, heads, true);
      view.setUint32(8, headDim, true);
      view.setUint32(12, 0, true);
      view.setFloat32(16, theta, true);
      view.setUint32(20, 0, true);
      view.setUint32(24, 0, true);
      view.setUint32(28, 0, true);
    }, 32);
    this.encodePipeline(encoder, "rope", SHADERS.rope, [
      values,
      this.createUniformBuffer("f2llm rope params", params),
    ], [
      Math.ceil(sequenceLength / DEFAULT_WORKGROUP_WIDTH),
      Math.ceil(heads / DEFAULT_WORKGROUP_WIDTH),
      Math.ceil(headDim / 2),
    ]);
  }

  encodeAttention(encoder, q, k, v, output, sequenceLength) {
    const scale = 1 / Math.sqrt(this.model.head_dim);
    const params = mixedUniform((view) => {
      view.setUint32(0, sequenceLength, true);
      view.setUint32(4, this.model.num_attention_heads, true);
      view.setUint32(8, this.model.num_key_value_heads, true);
      view.setUint32(12, this.model.head_dim, true);
      view.setFloat32(16, scale, true);
      view.setUint32(20, 0, true);
      view.setUint32(24, 0, true);
      view.setUint32(28, 0, true);
    }, 32);
    this.encodePipeline(encoder, "attention", SHADERS.attention, [
      q,
      k,
      v,
      output,
      this.createUniformBuffer("f2llm attention params", params),
    ], [
      Math.ceil(sequenceLength / 4),
      Math.ceil(this.model.num_attention_heads / 4),
      Math.ceil(this.model.head_dim / 4),
    ]);
  }

  encodeAdd(encoder, left, right, output, len) {
    const params = uintUniform([len, 0, 0, 0]);
    this.encodePipeline(encoder, "add", SHADERS.add, [
      left,
      right,
      output,
      this.createUniformBuffer("f2llm add params", params),
    ], [Math.ceil(len / 256), 1, 1]);
  }

  encodeSiluMul(encoder, gate, up, output, len) {
    const params = uintUniform([len, 0, 0, 0]);
    this.encodePipeline(encoder, "siluMul", SHADERS.siluMul, [
      gate,
      up,
      output,
      this.createUniformBuffer("f2llm silu params", params),
    ], [Math.ceil(len / 256), 1, 1]);
  }

  encodePipeline(encoder, name, shader, buffers, workgroups) {
    const pipeline = this.pipeline(name, shader);
    const bindGroup = this.device.createBindGroup({
      label: `f2llm ${name} bind group`,
      layout: pipeline.getBindGroupLayout(0),
      entries: buffers.map((buffer, index) => ({ binding: index, resource: { buffer } })),
    });
    const pass = encoder.beginComputePass({ label: `f2llm ${name}` });
    pass.setPipeline(pipeline);
    pass.setBindGroup(0, bindGroup);
    pass.dispatchWorkgroups(workgroups[0], workgroups[1], workgroups[2]);
    pass.end();
  }

  pipeline(name, shader) {
    let pipeline = this.pipelines.get(name);
    if (pipeline === undefined) {
      pipeline = this.device.createComputePipeline({
        label: `f2llm ${name}`,
        layout: "auto",
        compute: {
          module: this.device.createShaderModule({ label: `f2llm ${name}`, code: shader }),
          entryPoint: "main",
        },
      });
      this.pipelines.set(name, pipeline);
    }
    return pipeline;
  }

  q4Tensor(name) {
    const tensor = this.tensors.get(name);
    if (!isQ4Tensor(tensor)) {
      throw new Error(`missing q4 F2LLM tensor: ${name}`);
    }
    return tensor;
  }

  f32Tensor(name) {
    const tensor = this.tensors.get(name);
    if (tensor?.kind !== "f32") {
      throw new Error(`missing f32 F2LLM tensor: ${name}`);
    }
    return tensor;
  }

  createStorageBuffer(label, byteLength, usage = GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_SRC) {
    return this.device.createBuffer({
      label,
      size: alignTo(byteLength, 4),
      usage,
    });
  }

  createUploadBuffer(label, data, usage = GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST) {
    const byteLength = data.byteLength;
    const buffer = this.device.createBuffer({
      label,
      size: alignTo(byteLength, 4),
      usage,
    });
    this.device.queue.writeBuffer(buffer, 0, data);
    return buffer;
  }

  createUniformBuffer(label, data) {
    const buffer = this.createUploadBuffer(label, data, GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST);
    this.transientBuffers.push(buffer);
    return buffer;
  }

  destroyTransientBuffers() {
    for (const buffer of this.transientBuffers) {
      buffer.destroy();
    }
    this.transientBuffers = [];
  }

  async readF32BufferSlice(buffer, byteOffset, elements) {
    const byteLength = elements * 4;
    const readBuffer = this.device.createBuffer({
      label: "f2llm readback",
      size: alignTo(byteLength, 4),
      usage: GPUBufferUsage.COPY_DST | GPUBufferUsage.MAP_READ,
    });
    const encoder = this.device.createCommandEncoder({ label: "f2llm readback" });
    encoder.copyBufferToBuffer(buffer, byteOffset, readBuffer, 0, byteLength);
    this.device.queue.submit([encoder.finish()]);
    await readBuffer.mapAsync(GPUMapMode.READ);
    const values = new Float32Array(readBuffer.getMappedRange().slice(0, byteLength));
    readBuffer.unmap();
    readBuffer.destroy();
    return values;
  }
}

export class QwenByteBpeTokenizer {
  constructor({ vocab, merges, eosId }) {
    this.vocab = vocab;
    this.eosId = eosId;
    this.byteEncoder = bytesToUnicode();
    this.mergeRanks = new Map();
    for (let rank = 0; rank < merges.length; rank += 1) {
      const merge = merges[rank];
      const pair = Array.isArray(merge) ? merge : String(merge).split(" ");
      if (pair.length >= 2) {
        this.mergeRanks.set(pairKey(pair[0], pair[1]), rank);
      }
    }
    this.cache = new Map();
    this.pattern = /('s|'t|'re|'ve|'m|'ll|'d)|[^\r\n\p{L}\p{N}]?\p{L}+|\p{N}| ?[^\s\p{L}\p{N}]+[\r\n]*|\s*[\r\n]+|\s+(?!\S)|\s+/giu;
  }

  encode(text, maxLength) {
    const normalized = String(text || "").normalize("NFC");
    const ids = [];
    for (const match of normalized.matchAll(this.pattern)) {
      const piece = match[0];
      const byteLevel = this.byteLevelEncode(piece);
      for (const id of this.bpe(byteLevel)) {
        ids.push(id);
      }
    }
    const limit = Math.max(1, Number(maxLength) || DEFAULT_MAX_SEQUENCE_LENGTH);
    if (ids.length + 1 > limit) {
      ids.length = limit - 1;
    }
    ids.push(this.eosId);
    return ids;
  }

  byteLevelEncode(text) {
    const bytes = TEXT_ENCODER.encode(text);
    let encoded = "";
    for (const byte of bytes) {
      encoded += this.byteEncoder[byte];
    }
    return encoded;
  }

  bpe(token) {
    const cached = this.cache.get(token);
    if (cached !== undefined) {
      return cached;
    }
    let word = Array.from(token);
    if (word.length === 0) {
      return [];
    }
    for (;;) {
      let bestRank = Number.POSITIVE_INFINITY;
      let bestPair = null;
      for (let index = 0; index + 1 < word.length; index += 1) {
        const rank = this.mergeRanks.get(pairKey(word[index], word[index + 1]));
        if (rank !== undefined && rank < bestRank) {
          bestRank = rank;
          bestPair = [word[index], word[index + 1]];
        }
      }
      if (bestPair === null) {
        break;
      }
      const next = [];
      for (let index = 0; index < word.length; index += 1) {
        if (index + 1 < word.length && word[index] === bestPair[0] && word[index + 1] === bestPair[1]) {
          next.push(bestPair[0] + bestPair[1]);
          index += 1;
        } else {
          next.push(word[index]);
        }
      }
      word = next;
      if (word.length === 1) {
        break;
      }
    }
    const ids = word.map((piece) => {
      const id = this.vocab[piece];
      if (id === undefined) {
        throw new Error(`F2LLM tokenizer piece is missing from vocab: ${JSON.stringify(piece)}`);
      }
      return id;
    });
    this.cache.set(token, ids);
    return ids;
  }
}

async function loadTokenizer(baseUrl, manifest, fetchArrayBuffer) {
  const tokenizerSpec = manifest.tokenizer;
  const bytes = await fetchArrayBuffer(`${baseUrl}/${tokenizerSpec.url}`, "F2LLM tokenizer");
  await verifySha256(
    bytes,
    tokenizerSpec.canonical_json_sha256,
    "F2LLM tokenizer canonical JSON",
  );
  const tokenizer = JSON.parse(TEXT_DECODER.decode(bytes));
  if (tokenizer.schema_version !== EXPECTED_SCHEMA_VERSION) {
    throw new Error(`unsupported F2LLM tokenizer schema version: ${tokenizer.schema_version}`);
  }
  return new QwenByteBpeTokenizer({
    vocab: tokenizer.vocab,
    merges: tokenizer.merges,
    eosId: tokenizer.special_tokens?.eos_id,
  });
}

function validateManifest(manifest, options) {
  if (manifest.schema_version !== EXPECTED_SCHEMA_VERSION) {
    throw new Error(`unsupported F2LLM WebGPU artifact schema version: ${manifest.schema_version}`);
  }
  for (const [field, actual, expected] of [
    ["model_key", manifest.model_key, options.expectedModelKey],
    ["runtime", manifest.runtime, options.expectedRuntime],
    ["artifact_version", manifest.artifact_version, options.expectedVersion],
  ]) {
    if (actual !== expected) {
      throw new Error(`F2LLM WebGPU manifest ${field} mismatch: expected ${expected}, got ${actual}`);
    }
  }
  const model = manifest.model || {};
  for (const field of [
    "vocab_size",
    "hidden_size",
    "num_hidden_layers",
    "num_attention_heads",
    "num_key_value_heads",
    "head_dim",
    "intermediate_size",
    "rms_norm_eps",
    "rope_theta",
  ]) {
    if (typeof model[field] !== "number" || !(model[field] > 0)) {
      throw new Error(`F2LLM WebGPU manifest model.${field} must be a positive number`);
    }
  }
  if (options.dimensions && model.hidden_size !== options.dimensions) {
    throw new Error(`F2LLM hidden size mismatch: expected ${options.dimensions}, got ${model.hidden_size}`);
  }
  if (model.num_attention_heads % model.num_key_value_heads !== 0) {
    throw new Error("F2LLM attention heads must be divisible by key/value heads");
  }
  if (model.head_dim % 2 !== 0) {
    throw new Error("F2LLM RoPE head dimension must be even");
  }
}

function validateRequiredTensors(manifest, tensors) {
  const layers = manifest.model.num_hidden_layers;
  const required = [
    "model.embed_tokens.weight",
    "model.norm.weight",
  ];
  for (let layer = 0; layer < layers; layer += 1) {
    const prefix = `model.layers.${layer}`;
    required.push(
      `${prefix}.input_layernorm.weight`,
      `${prefix}.post_attention_layernorm.weight`,
      `${prefix}.self_attn.q_proj.weight`,
      `${prefix}.self_attn.q_norm.weight`,
      `${prefix}.self_attn.k_proj.weight`,
      `${prefix}.self_attn.k_norm.weight`,
      `${prefix}.self_attn.v_proj.weight`,
      `${prefix}.self_attn.o_proj.weight`,
      `${prefix}.mlp.gate_proj.weight`,
      `${prefix}.mlp.up_proj.weight`,
      `${prefix}.mlp.down_proj.weight`,
    );
  }
  const missing = required.filter((name) => !tensors.has(name));
  if (missing.length > 0) {
    throw new Error(`F2LLM WebGPU artifact is missing required tensors: ${missing.join(", ")}`);
  }
}

function tensorByteLength(tensor) {
  if (tensor.kind === "q4_rowwise") {
    return (tensor.qweight?.byte_length || 0) + (tensor.scales?.byte_length || 0);
  }
  if (tensor.kind === "q4_onnx_gather" || tensor.kind === "q4_onnx_matmul") {
    return (
      (tensor.qweight?.byte_length || 0)
      + (tensor.scales?.byte_length || 0)
      + (tensor.zero_points?.byte_length || 0)
    );
  }
  return tensor.data?.byte_length || 0;
}

function isQ4Tensor(tensor) {
  return tensor?.kind === "q4_rowwise"
    || tensor?.kind === "q4_onnx_gather"
    || tensor?.kind === "q4_onnx_matmul";
}

function topHits(scores, items, limit, itemMatches) {
  const limitCount = Math.trunc(Number(limit) || 0);
  const hits = [];
  for (let row = 0; row < scores.length && row < items.length; row += 1) {
    const item = items[row];
    if (typeof itemMatches === "function" && !itemMatches(item)) {
      continue;
    }
    const candidate = { id: item.id, score: scores[row] };
    if (limitCount <= 0) {
      hits.push(candidate);
    } else if (hits.length < limitCount) {
      hits.push(candidate);
    } else {
      const worst = worstHitIndex(hits);
      if (worst !== -1 && compareHits(candidate, hits[worst]) < 0) {
        hits[worst] = candidate;
      }
    }
  }
  hits.sort(compareHits);
  return hits;
}

function compareHits(left, right) {
  return right.score - left.score || left.id - right.id;
}

function worstHitIndex(hits) {
  let worst = -1;
  for (let index = 0; index < hits.length; index += 1) {
    if (worst === -1 || compareHits(hits[index], hits[worst]) > 0) {
      worst = index;
    }
  }
  return worst;
}

function corpusVectorCacheKey(corpus) {
  const shards = (corpus.shards || [])
    .map((shard) => `${shard.key || ""}:${shard.byteLen || 0}`)
    .join("|");
  return [
    corpus.corpusId || "",
    corpus.inputHash || "",
    corpus.rowCount || 0,
    corpus.dimensions || 0,
    corpus.elementType || "",
    shards,
  ].join("::");
}

function normalize(vector) {
  let sum = 0;
  for (const value of vector) {
    sum += value * value;
  }
  const magnitude = Math.sqrt(sum);
  if (magnitude === 0) {
    return;
  }
  for (let index = 0; index < vector.length; index += 1) {
    vector[index] /= magnitude;
  }
}

async function fetchJsonWith(fetchArrayBuffer, url, label) {
  const bytes = await fetchArrayBuffer(url, label);
  return JSON.parse(TEXT_DECODER.decode(bytes));
}

async function fetchJson(url, label) {
  return fetchJsonWith(defaultFetchArrayBuffer, url, label);
}

async function defaultFetchArrayBuffer(url, label) {
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error(`failed to fetch ${label} from ${url}: ${response.status}`);
  }
  return response.arrayBuffer();
}

async function verifySha256(buffer, expected, name) {
  if (typeof expected !== "string" || expected.length === 0) {
    throw new Error(`${name} is missing SHA-256`);
  }
  const actual = await sha256Hex(buffer);
  if (actual !== expected) {
    throw new Error(`${name} SHA-256 mismatch`);
  }
}

async function sha256Hex(buffer) {
  const digest = await crypto.subtle.digest("SHA-256", buffer);
  return Array.from(new Uint8Array(digest))
    .map((byte) => byte.toString(16).padStart(2, "0"))
    .join("");
}

function progressValue(kind, label, loaded, total) {
  const numericLoaded = Math.max(0, Number(loaded) || 0);
  const numericTotal = Math.max(0, Number(total) || 0);
  const percent = numericTotal > 0
    ? Math.min(100, Math.round((numericLoaded / numericTotal) * 100))
    : null;
  return {
    kind,
    label,
    loaded: numericLoaded,
    total: numericTotal,
    percent,
  };
}

function normalizeBaseUrl(baseUrl) {
  if (typeof baseUrl !== "string" || baseUrl.trim().length === 0) {
    throw new Error("F2LLM WebGPU artifact base URL is empty");
  }
  return baseUrl.trim().replace(/\/+$/, "");
}

function checkedShape(name, shape, rank) {
  if (!Array.isArray(shape) || shape.length !== rank) {
    throw new Error(`${name} must have rank ${rank}`);
  }
  return shape.map((value, index) =>
    checkedPositiveInteger(value, `${name}.shape[${index}]`)
  );
}

function checkedPositiveInteger(value, name) {
  const number = Number(value);
  if (!Number.isInteger(number) || number <= 0) {
    throw new Error(`${name} must be a positive integer`);
  }
  return number;
}

function checkedNonNegativeInteger(value, name) {
  const number = Number(value);
  if (!Number.isInteger(number) || number < 0) {
    throw new Error(`${name} must be a non-negative integer`);
  }
  return number;
}

function uintUniform(values) {
  const buffer = new ArrayBuffer(alignTo(values.length * 4, 16));
  const view = new DataView(buffer);
  for (let index = 0; index < values.length; index += 1) {
    view.setUint32(index * 4, values[index], true);
  }
  return buffer;
}

function mixedUniform(write, byteLength) {
  const buffer = new ArrayBuffer(alignTo(byteLength, 16));
  write(new DataView(buffer));
  return buffer;
}

function alignTo(value, alignment) {
  return Math.ceil(value / alignment) * alignment;
}

function pairKey(left, right) {
  return `${left}\u0000${right}`;
}

function bytesToUnicode() {
  const bytes = [];
  for (let value = 33; value <= 126; value += 1) {
    bytes.push(value);
  }
  for (let value = 161; value <= 172; value += 1) {
    bytes.push(value);
  }
  for (let value = 174; value <= 255; value += 1) {
    bytes.push(value);
  }
  const chars = bytes.slice();
  let next = 0;
  for (let value = 0; value <= 255; value += 1) {
    if (!bytes.includes(value)) {
      bytes.push(value);
      chars.push(256 + next);
      next += 1;
    }
  }
  const encoder = [];
  for (let index = 0; index < bytes.length; index += 1) {
    encoder[bytes[index]] = String.fromCodePoint(chars[index]);
  }
  return encoder;
}
