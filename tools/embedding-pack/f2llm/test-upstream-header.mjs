#!/usr/bin/env node

import { strict as assert } from "node:assert";

const MODEL_URL = "https://huggingface.co/codefuse-ai/F2LLM-v2-80M/resolve/main/model.safetensors";
const LAYERS = 8;

const first = await fetch(MODEL_URL, { headers: { Range: "bytes=0-7" } });
assert.equal(first.status, 206);
const prefix = new Uint8Array(await first.arrayBuffer());
let headerLength = 0n;
for (let index = 0; index < 8; index += 1) {
  headerLength |= BigInt(prefix[index]) << BigInt(index * 8);
}
assert.ok(headerLength > 0n && headerLength < 1024n * 1024n);

const headerResponse = await fetch(MODEL_URL, {
  headers: { Range: `bytes=8-${7n + headerLength}` },
});
assert.equal(headerResponse.status, 206);
const header = JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(await headerResponse.arrayBuffer()));
const keys = new Set(Object.keys(header).filter((key) => key !== "__metadata__"));

const required = [
  "embed_tokens.weight",
  "norm.weight",
];
for (let layer = 0; layer < LAYERS; layer += 1) {
  const prefixName = `layers.${layer}`;
  required.push(
    `${prefixName}.input_layernorm.weight`,
    `${prefixName}.post_attention_layernorm.weight`,
    `${prefixName}.self_attn.q_proj.weight`,
    `${prefixName}.self_attn.q_norm.weight`,
    `${prefixName}.self_attn.k_proj.weight`,
    `${prefixName}.self_attn.k_norm.weight`,
    `${prefixName}.self_attn.v_proj.weight`,
    `${prefixName}.self_attn.o_proj.weight`,
    `${prefixName}.mlp.gate_proj.weight`,
    `${prefixName}.mlp.up_proj.weight`,
    `${prefixName}.mlp.down_proj.weight`,
  );
}

const missing = required.filter((key) => !keys.has(key));
assert.deepEqual(missing, []);
assert.equal(keys.size, 90);

console.log("f2llm upstream safetensors header test passed");
