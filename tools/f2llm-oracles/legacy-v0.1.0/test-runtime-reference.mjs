#!/usr/bin/env node

import { strict as assert } from "node:assert";
import { readFileSync } from "node:fs";

const EPS = 1e-6;
const EXPECTED_EMBEDDING = [
  0.47408923506736755,
  0.39601829648017883,
  -0.39500799775123596,
  0.6799834966659546,
];

const MODEL = {
  vocab_size: 5,
  hidden_size: 4,
  num_hidden_layers: 1,
  num_attention_heads: 2,
  num_key_value_heads: 1,
  head_dim: 2,
  intermediate_size: 6,
  rms_norm_eps: 1e-6,
  rope_theta: 10000,
};

const F32 = {
  "model.layers.0.input_layernorm.weight": [1.0, 0.8, 1.2, 1.1],
  "model.layers.0.self_attn.q_norm.weight": [0.9, 1.1],
  "model.layers.0.self_attn.k_norm.weight": [1.05, 0.95],
  "model.layers.0.post_attention_layernorm.weight": [0.7, 1.3, 0.6, 1.4],
  "model.norm.weight": [1.0, 1.1, 0.9, 1.2],
};

const Q4_SOURCE = {
  "model.embed_tokens.weight": [
    [0.20, -0.30, 0.40, 0.10],
    [-0.10, 0.50, 0.25, -0.35],
    [0.45, 0.15, -0.20, 0.30],
    [-0.25, -0.15, 0.35, 0.55],
    [0.05, 0.25, -0.45, -0.10],
  ],
  "model.layers.0.self_attn.q_proj.weight": [
    [0.15, -0.20, 0.35, 0.10],
    [-0.30, 0.25, 0.05, 0.40],
    [0.22, 0.18, -0.28, 0.12],
    [-0.16, 0.32, 0.24, -0.08],
  ],
  "model.layers.0.self_attn.k_proj.weight": [
    [0.26, -0.14, 0.18, 0.30],
    [-0.22, 0.36, 0.12, -0.18],
  ],
  "model.layers.0.self_attn.v_proj.weight": [
    [0.31, 0.11, -0.27, 0.19],
    [-0.17, 0.29, 0.23, -0.13],
  ],
  "model.layers.0.self_attn.o_proj.weight": [
    [0.18, -0.24, 0.33, 0.09],
    [-0.21, 0.37, 0.07, -0.15],
    [0.25, 0.13, -0.19, 0.31],
    [-0.11, 0.28, 0.17, -0.23],
  ],
  "model.layers.0.mlp.gate_proj.weight": [
    [0.12, -0.18, 0.28, 0.22],
    [-0.24, 0.34, 0.16, -0.12],
    [0.31, 0.09, -0.21, 0.27],
    [-0.14, 0.26, 0.32, -0.20],
    [0.19, -0.29, 0.11, 0.35],
    [-0.33, 0.15, 0.25, -0.07],
  ],
  "model.layers.0.mlp.up_proj.weight": [
    [-0.16, 0.21, 0.30, -0.10],
    [0.27, -0.19, 0.13, 0.23],
    [-0.25, 0.31, -0.09, 0.17],
    [0.20, 0.12, -0.34, 0.14],
    [-0.11, 0.29, 0.18, -0.22],
    [0.33, -0.07, 0.24, 0.16],
  ],
  "model.layers.0.mlp.down_proj.weight": [
    [0.23, -0.12, 0.18, 0.27, -0.15, 0.09],
    [-0.28, 0.17, 0.21, -0.11, 0.31, -0.08],
    [0.14, 0.26, -0.22, 0.19, -0.10, 0.33],
    [-0.20, 0.08, 0.29, -0.24, 0.16, 0.11],
  ],
};

const q4 = Object.fromEntries(Object.entries(Q4_SOURCE).map(([name, matrix]) => [
  name,
  quantizeQ4Rowwise(matrix, 2),
]));

assert.deepEqual(Array.from(q4["model.embed_tokens.weight"].qbytes.slice(0, 6)), [29, 175, 247, 29, 175, 243]);
assertAlmostArray(
  dequantizeQ4(q4["model.embed_tokens.weight"]).slice(0, 2).flat(),
  [0.2142857164144516, -0.30000001192092896, 0.4000000059604645, 0.11428571492433548,
    -0.0714285746216774, 0.5, 0.25, -0.3500000238418579],
  1e-6,
);

const embedding = embed([0, 1, 2], MODEL, q4, F32);
console.log("tiny CPU embedding", JSON.stringify(Array.from(embedding)));
assertAlmostArray(Array.from(embedding), EXPECTED_EMBEDDING, 1e-6);
assertAlmost(norm(embedding), 1.0, 1e-6);
assertRopeDispatchCoversAllPairs();

const vectors = new Float32Array([
  0.25, -0.50, 0.25, 0.75,
  -0.75, 0.25, 0.50, -0.25,
  0.10, 0.20, 0.30, 0.40,
]);
const f16Bytes = f32VectorsToF16Bytes(vectors);
const hits = rankF16Vectors(f16Bytes, new Float32Array([0.2, -0.1, 0.3, 0.4]), [
  { id: 10 },
  { id: 11 },
  { id: 12 },
], 4, 2);
assert.deepEqual(hits.map((hit) => hit.id), [10, 12]);
assertAlmost(hits[0].score, 0.475, 1e-4);

console.log("f2llm CPU runtime reference tests passed");

function assertRopeDispatchCoversAllPairs() {
  const source = readFileSync(new URL("../../apps/jbotci-web/assets/f2llm-webgpu-runtime.js", import.meta.url), "utf8");
  assert.match(
    source,
    /Math\.ceil\(headDim \/ 2\)/,
    "RoPE dispatch must launch one Z workgroup per rotary pair because the shader Z workgroup size is 1",
  );
  assert.doesNotMatch(
    source,
    /Math\.ceil\(\(headDim \/ 2\) \/ DEFAULT_WORKGROUP_WIDTH\)/,
    "RoPE dispatch must not divide the rotary-pair dimension by the X/Y workgroup width",
  );
}

function embed(tokens, model, q4Tensors, f32Tensors) {
  let hidden = tokens.map((token) => dequantizeRow(q4Tensors["model.embed_tokens.weight"], token));
  for (let layer = 0; layer < model.num_hidden_layers; layer += 1) {
    hidden = layerForward(layer, hidden, model, q4Tensors, f32Tensors);
  }
  hidden = rmsNorm(hidden, f32Tensors["model.norm.weight"], model.rms_norm_eps);
  return normalize(Float32Array.from(hidden.at(-1)));
}

function layerForward(layer, hidden, model, q4Tensors, f32Tensors) {
  const prefix = `model.layers.${layer}`;
  const attnNorm = rmsNorm(hidden, f32Tensors[`${prefix}.input_layernorm.weight`], model.rms_norm_eps);
  let q = matmulQ4(attnNorm, q4Tensors[`${prefix}.self_attn.q_proj.weight`]);
  let k = matmulQ4(attnNorm, q4Tensors[`${prefix}.self_attn.k_proj.weight`]);
  const v = matmulQ4(attnNorm, q4Tensors[`${prefix}.self_attn.v_proj.weight`]);
  q = rmsNormHeads(q, model.num_attention_heads, model.head_dim, f32Tensors[`${prefix}.self_attn.q_norm.weight`], model.rms_norm_eps);
  k = rmsNormHeads(k, model.num_key_value_heads, model.head_dim, f32Tensors[`${prefix}.self_attn.k_norm.weight`], model.rms_norm_eps);
  applyRope(q, model.num_attention_heads, model.head_dim, model.rope_theta);
  applyRope(k, model.num_key_value_heads, model.head_dim, model.rope_theta);
  const attn = attention(q, k, v, model);
  const attnProjected = matmulQ4(attn, q4Tensors[`${prefix}.self_attn.o_proj.weight`]);
  const postAttention = addRows(hidden, attnProjected);
  const mlpNorm = rmsNorm(postAttention, f32Tensors[`${prefix}.post_attention_layernorm.weight`], model.rms_norm_eps);
  const gate = matmulQ4(mlpNorm, q4Tensors[`${prefix}.mlp.gate_proj.weight`]);
  const up = matmulQ4(mlpNorm, q4Tensors[`${prefix}.mlp.up_proj.weight`]);
  const activated = gate.map((row, rowIndex) =>
    row.map((value, index) => silu(value) * up[rowIndex][index])
  );
  const down = matmulQ4(activated, q4Tensors[`${prefix}.mlp.down_proj.weight`]);
  return addRows(postAttention, down);
}

function quantizeQ4Rowwise(matrix, groupSize) {
  const rows = matrix.length;
  const cols = matrix[0].length;
  const groups = Math.ceil(cols / groupSize);
  const qbytes = new Uint8Array(Math.ceil((rows * cols) / 2));
  const scales = new Float32Array(rows * groups);
  for (let row = 0; row < rows; row += 1) {
    for (let group = 0; group < groups; group += 1) {
      const start = group * groupSize;
      const end = Math.min(cols, start + groupSize);
      let maxAbs = 0;
      for (let col = start; col < end; col += 1) {
        maxAbs = Math.max(maxAbs, Math.abs(matrix[row][col]));
      }
      const scale = maxAbs > 0 ? maxAbs / 7 : 1;
      scales[row * groups + group] = scale;
      for (let col = start; col < end; col += 1) {
        const q = Math.max(0, Math.min(15, roundTiesToEven(matrix[row][col] / scale) + 8));
        const element = row * cols + col;
        const byteIndex = Math.floor(element / 2);
        if (element % 2 === 0) {
          qbytes[byteIndex] = (qbytes[byteIndex] & 0xf0) | q;
        } else {
          qbytes[byteIndex] = (qbytes[byteIndex] & 0x0f) | (q << 4);
        }
      }
    }
  }
  return { shape: [rows, cols], groupSize, groups, qbytes, scales };
}

function roundTiesToEven(value) {
  const floor = Math.floor(value);
  const fraction = value - floor;
  if (fraction < 0.5) {
    return floor;
  }
  if (fraction > 0.5) {
    return floor + 1;
  }
  return floor % 2 === 0 ? floor : floor + 1;
}

function dequantizeQ4(tensor) {
  const [rows, cols] = tensor.shape;
  const output = [];
  for (let row = 0; row < rows; row += 1) {
    output.push(Array.from(dequantizeRow(tensor, row)));
  }
  return output;
}

function dequantizeRow(tensor, row) {
  const cols = tensor.shape[1];
  const output = new Float32Array(cols);
  for (let col = 0; col < cols; col += 1) {
    const element = row * cols + col;
    const byte = tensor.qbytes[Math.floor(element / 2)];
    const nibble = element % 2 === 0 ? byte & 15 : (byte >> 4) & 15;
    const scale = tensor.scales[row * tensor.groups + Math.floor(col / tensor.groupSize)];
    output[col] = (nibble - 8) * scale;
  }
  return output;
}

function matmulQ4(input, weight) {
  const rows = input.length;
  const inCols = weight.shape[1];
  const outCols = weight.shape[0];
  const output = [];
  for (let row = 0; row < rows; row += 1) {
    const out = new Float32Array(outCols);
    for (let outCol = 0; outCol < outCols; outCol += 1) {
      const weights = dequantizeRow(weight, outCol);
      let sum = 0;
      for (let inCol = 0; inCol < inCols; inCol += 1) {
        sum += input[row][inCol] * weights[inCol];
      }
      out[outCol] = sum;
    }
    output.push(out);
  }
  return output;
}

function rmsNorm(rows, weight, eps) {
  return rows.map((row) => {
    let sum = 0;
    for (const value of row) {
      sum += value * value;
    }
    const invRms = 1 / Math.sqrt(sum / row.length + eps);
    return Float32Array.from(row, (value, index) => value * invRms * weight[index]);
  });
}

function rmsNormHeads(rows, heads, headDim, weight, eps) {
  return rows.map((row) => {
    const output = new Float32Array(row.length);
    for (let head = 0; head < heads; head += 1) {
      const base = head * headDim;
      let sum = 0;
      for (let dim = 0; dim < headDim; dim += 1) {
        const value = row[base + dim];
        sum += value * value;
      }
      const invRms = 1 / Math.sqrt(sum / headDim + eps);
      for (let dim = 0; dim < headDim; dim += 1) {
        output[base + dim] = row[base + dim] * invRms * weight[dim];
      }
    }
    return output;
  });
}

function applyRope(rows, heads, headDim, theta) {
  const half = headDim / 2;
  for (let token = 0; token < rows.length; token += 1) {
    for (let head = 0; head < heads; head += 1) {
      const base = head * headDim;
      for (let dim = 0; dim < half; dim += 1) {
        const angle = token / Math.pow(theta, (dim * 2) / headDim);
        const c = Math.cos(angle);
        const s = Math.sin(angle);
        const first = rows[token][base + dim];
        const second = rows[token][base + dim + half];
        rows[token][base + dim] = first * c - second * s;
        rows[token][base + dim + half] = second * c + first * s;
      }
    }
  }
}

function attention(q, k, v, model) {
  const output = [];
  const kvGroupSize = model.num_attention_heads / model.num_key_value_heads;
  const scale = 1 / Math.sqrt(model.head_dim);
  for (let token = 0; token < q.length; token += 1) {
    const row = new Float32Array(model.num_attention_heads * model.head_dim);
    for (let qHead = 0; qHead < model.num_attention_heads; qHead += 1) {
      const kvHead = Math.floor(qHead / kvGroupSize);
      const scores = [];
      for (let keyToken = 0; keyToken <= token; keyToken += 1) {
        let score = 0;
        for (let dim = 0; dim < model.head_dim; dim += 1) {
          score += q[token][qHead * model.head_dim + dim] * k[keyToken][kvHead * model.head_dim + dim];
        }
        scores.push(score * scale);
      }
      const maxScore = Math.max(...scores);
      const weights = scores.map((score) => Math.exp(score - maxScore));
      const denominator = weights.reduce((sum, value) => sum + value, 0);
      for (let dim = 0; dim < model.head_dim; dim += 1) {
        let weighted = 0;
        for (let keyToken = 0; keyToken <= token; keyToken += 1) {
          weighted += weights[keyToken] * v[keyToken][kvHead * model.head_dim + dim];
        }
        row[qHead * model.head_dim + dim] = weighted / denominator;
      }
    }
    output.push(row);
  }
  return output;
}

function addRows(left, right) {
  return left.map((row, rowIndex) =>
    Float32Array.from(row, (value, index) => value + right[rowIndex][index])
  );
}

function silu(value) {
  return value / (1 + Math.exp(-value));
}

function normalize(vector) {
  const magnitude = norm(vector);
  return Float32Array.from(vector, (value) => value / magnitude);
}

function norm(vector) {
  let sum = 0;
  for (const value of vector) {
    sum += value * value;
  }
  return Math.sqrt(sum);
}

function f32VectorsToF16Bytes(values) {
  const bytes = new Uint8Array(values.length * 2);
  const view = new DataView(bytes.buffer);
  for (let index = 0; index < values.length; index += 1) {
    view.setUint16(index * 2, f32ToF16(values[index]), true);
  }
  return bytes.buffer;
}

function rankF16Vectors(bytes, query, items, dimensions, limit) {
  const values = new DataView(bytes);
  const rows = Math.floor(bytes.byteLength / (dimensions * 2));
  const hits = [];
  for (let row = 0; row < rows; row += 1) {
    let score = 0;
    for (let dim = 0; dim < dimensions; dim += 1) {
      score += f16ToF32(values.getUint16((row * dimensions + dim) * 2, true)) * query[dim];
    }
    hits.push({ id: items[row].id, score });
  }
  hits.sort((left, right) => right.score - left.score || left.id - right.id);
  return hits.slice(0, limit);
}

function f32ToF16(value) {
  const floatView = new Float32Array(1);
  const intView = new Uint32Array(floatView.buffer);
  floatView[0] = value;
  const bits = intView[0];
  const sign = (bits >>> 16) & 0x8000;
  let exponent = ((bits >>> 23) & 0xff) - 127 + 15;
  let mantissa = bits & 0x7fffff;
  if (exponent <= 0) {
    if (exponent < -10) {
      return sign;
    }
    mantissa = (mantissa | 0x800000) >>> (1 - exponent);
    return sign | ((mantissa + 0x1000) >>> 13);
  }
  if (exponent >= 31) {
    return sign | 0x7c00;
  }
  return sign | (exponent << 10) | ((mantissa + 0x1000) >>> 13);
}

function f16ToF32(bits) {
  const sign = (bits & 0x8000) ? -1 : 1;
  const exponent = (bits >>> 10) & 0x1f;
  const fraction = bits & 0x03ff;
  if (exponent === 0) {
    return sign * Math.pow(2, -14) * (fraction / 1024);
  }
  if (exponent === 31) {
    return fraction === 0 ? sign * Infinity : NaN;
  }
  return sign * Math.pow(2, exponent - 15) * (1 + fraction / 1024);
}

function assertAlmost(actual, expected, eps) {
  assert.ok(Math.abs(actual - expected) <= eps, `${actual} != ${expected}`);
}

function assertAlmostArray(actual, expected, eps) {
  assert.equal(actual.length, expected.length);
  for (let index = 0; index < actual.length; index += 1) {
    assertAlmost(actual[index], expected[index], eps);
  }
}
