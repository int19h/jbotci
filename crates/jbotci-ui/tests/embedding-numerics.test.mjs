import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

globalThis.self = globalThis;
const {
  f32ToF16Bits, f16ToF32, requireVector, normalize, meanPoolNormalized,
  F2LlmOnnxWasmRuntime, readCorpusVectors, validatedVectorStore,
  rankHits, rankHitsFromScores,
} = await import("../assets/embedding-worker.js");

const reference = JSON.parse(readFileSync(new URL("./fixtures/f16-reference.json", import.meta.url), "utf8"));

test("binary16 conversion matches every Python reference bit pattern", () => {
  const view = new DataView(new ArrayBuffer(4));
  for (const { f32_bits: input, f16_bits: expected } of reference.cases) {
    view.setUint32(0, input, true);
    assert.equal(f32ToF16Bits(view.getFloat32(0, true)), expected, `float32 bits 0x${input.toString(16)}`);
  }
  assert.equal(f32ToF16Bits(0.500244140625), 0x3800);
  assert.equal(f32ToF16Bits(3 * 2 ** -26), 0x0001);
});

test("every non-NaN binary16 value round trips with its sign", () => {
  for (let bits = 0; bits <= 0xffff; bits += 1) {
    if ((bits & 0x7c00) === 0x7c00 && (bits & 0x03ff) !== 0) continue;
    assert.equal(f32ToF16Bits(f16ToF32(bits)), bits, `binary16 bits 0x${bits.toString(16)}`);
  }
  assert.equal(f32ToF16Bits(NaN) & 0x7fff, 0x7e00);
});

test("one vector boundary rejects invalid dimensions, components and norms", () => {
  for (const vector of [[0, 0], [NaN, 1], [Infinity, 1], [-Infinity, 1], [Number.MAX_VALUE, Number.MAX_VALUE]]) {
    assert.throws(() => requireVector(vector, 2), /finite|positive/);
    assert.throws(() => normalize(vector.slice(), 2), /finite|positive/);
  }
  assert.throws(() => requireVector([1], 2), /dimension/);
  assert.throws(() => requireVector([], 0), /dimension/);
  assert.equal(requireVector([3, 4], 2), 5);
  const healthy = Float32Array.of(3, 4);
  normalize(healthy, 2);
  assert.deepEqual(Array.from(healthy), [Math.fround(3 / 5), Math.fround(4 / 5)]);
});

test("CPU fallback rejects invalid ONNX output before it becomes an embedding", async () => {
  for (const data of [[0, 0], [NaN, 1], [Infinity, 1], [1]]) {
    const runtime = new F2LlmOnnxWasmRuntime({
      ort: { Tensor: class {} },
      session: { inputNames: ["input_ids"], run: async () => ({ embedding: { dims: [1, 2], data } }) },
      tokenizer: { tokenWindows: () => [[151645]] },
      dimensions: 2, maxSequenceLength: 512,
    });
    await assert.rejects(runtime.embedText("example"), /dimension|finite|positive/);
  }
  assert.throws(() => meanPoolNormalized([Float32Array.of(1, 0), Float32Array.of(-1, 0)], 2), /positive/);
  assert.throws(() => meanPoolNormalized([Float32Array.of(0, 0)], 2), /positive/);
});

function vectorBuffer(vector, elementType) {
  const size = elementType === "f16le" ? 2 : 4;
  const view = new DataView(new ArrayBuffer(vector.length * size));
  vector.forEach((value, index) => {
    if (size === 2) view.setUint16(index * size, f32ToF16Bits(value), true);
    else view.setFloat32(index * size, value, true);
  });
  return view.buffer;
}

test("stored rows are rejected on both CPU load and GPU upload", async () => {
  for (const elementType of ["f16le", "f32le"]) {
    for (const [index, vector] of [[0, 0], [NaN, 1], [Infinity, 1], [-Infinity, 1]].entries()) {
      const bytes = vectorBuffer(vector, elementType);
      const key = `invalid-${elementType}-${index}`;
      const corpus = { corpusId: key, rowCount: 1, dimensions: 2, elementType, shards: [{ key, byteLen: bytes.byteLength }] };
      await assert.rejects(readCorpusVectors(corpus, async () => bytes), /finite|positive/);
      await assert.rejects(validatedVectorStore(corpus, async () => bytes)(key), /finite|positive/);
    }
    const bytes = vectorBuffer([1, 0], elementType);
    const key = `healthy-${elementType}`;
    const corpus = { corpusId: key, rowCount: 1, dimensions: 2, elementType, shards: [{ key, byteLen: bytes.byteLength }] };
    assert.deepEqual(Array.from(await readCorpusVectors(corpus, async () => bytes)), [1, 0]);
    assert.equal(await validatedVectorStore(corpus, async () => bytes)(key), bytes);
    await assert.rejects(validatedVectorStore(corpus, async () => bytes)("unknown"), /unknown/);
    const truncated = bytes.slice(1);
    await assert.rejects(readCorpusVectors({ ...corpus, corpusId: `${key}-short`, shards: [{ key, byteLen: truncated.byteLength }] }, async () => truncated), /complete rows/);
  }
});

test("CPU ranking rejects zero queries and non-finite scores for bounded and full rankings", () => {
  const items = [{ id: 0 }];
  for (const limit of [0, 1]) {
    for (const query of [[0, 0], [NaN, 1], [Infinity, 1], [1]]) {
      assert.throws(() => rankHits([1, 0], query, items, 2, limit, []), /dimension|finite|positive/);
    }
    for (const value of [NaN, Infinity, -Infinity]) {
      assert.throws(() => rankHits([value, 0], [1, 0], items, 2, limit, []), /finite/);
    }
    assert.throws(() => rankHits([1e308], [1e308], items, 1, limit, []), /score must be finite/);
    assert.throws(() => rankHits([1], [1, 0], items, 2, limit, []), /dimensions/);
  }
});

test("GPU ranking rejects non-finite scores even for filtered rows", () => {
  const items = [{ id: 0, kind: "cmavo" }, { id: 1, kind: "gismu" }];
  for (const limit of [0, 1]) {
    for (const value of [NaN, Infinity, -Infinity]) {
      assert.throws(() => rankHitsFromScores([value, 1], items, limit, ["gismu"]), /finite/);
    }
    assert.throws(() => rankHitsFromScores([1], items, limit, []), /score count/);
    assert.deepEqual(rankHitsFromScores([0, 1], items, limit, ["gismu"]), [{ id: 1, score: 1 }]);
  }
});
