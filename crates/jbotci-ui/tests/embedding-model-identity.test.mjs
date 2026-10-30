import assert from "node:assert/strict";
import { createHash, webcrypto } from "node:crypto";
import test from "node:test";

globalThis.self = globalThis;
globalThis.location = { href: "https://example.invalid/app/" };
Object.defineProperty(globalThis, "crypto", { value: webcrypto, configurable: true });
const {
  loadOnnxModelBytes, cachedFetchArrayBuffer, readModelArtifactIdentity, localVectorPackId,
  modelArtifactSource, modelIdentityCompatibilityIssue, manifestCompatibilityIssue,
} = await import("../assets/embedding-worker.js");
const bytes = (text) => new TextEncoder().encode(text).buffer;
const digest = (buffer) => createHash("sha256").update(new Uint8Array(buffer)).digest("hex");
const oldModel = bytes("previous model bytes");
const newModel = bytes("current model bytes");
const identity = {
  modelRevision: "current-revision",
  modelSha256: digest(newModel),
  tokenizerSha256: "a".repeat(64),
  artifactManifestSha256: "b".repeat(64),
};
const spec = {
  modelKey: "test-model", label: "Test model", dimensions: 2, maxSequenceLength: 512,
  customRuntime: { runtime: "custom", version: "0.2.0" },
  wasmRuntime: { runtime: "onnx-wasm", version: "0.2.0", onnxUrl: "https://example.invalid/models/model_q4.onnx" },
  vectorElementType: "f16le", outputPooling: "mean_normalized_windows",
};
const fallbackManifest = {
  schema_version: 1,
  model_key: spec.modelKey, runtime: spec.wasmRuntime.runtime, artifact_version: "0.2.0",
  dimensions: 2, max_sequence_length: 512, model_sha256: identity.modelSha256,
  model_url: "model_q4.onnx", model_byte_length: newModel.byteLength,
};
const artifactManifest = {
  schema_version: 1, runtime: spec.customRuntime.runtime,
  model_key: spec.modelKey, artifact_version: "0.2.0", max_sequence_length: 512,
  model: { hidden_size: 2 }, source_revision: identity.modelRevision,
  q4_onnx_sha256: identity.modelSha256, tokenizer: { canonical_json_sha256: identity.tokenizerSha256 },
};

function installCache(t, cachedBytes, networkBytes = newModel, manifest = fallbackManifest) {
  const entries = new Map(cachedBytes === null ? [] : [[spec.wasmRuntime.onnxUrl, new Response(cachedBytes)]]);
  const counts = { modelFetches: 0, deletes: 0, puts: 0 };
  const previousCaches = globalThis.caches;
  globalThis.caches = { open: async () => ({
    match: async (request) => entries.get(request.url)?.clone(),
    delete: async (request) => { counts.deletes += 1; return entries.delete(request.url); },
    put: async (request, response) => { counts.puts += 1; entries.set(request.url, response.clone()); },
  }) };
  t.after(() => {
    if (previousCaches === undefined) delete globalThis.caches;
    else globalThis.caches = previousCaches;
  });
  t.mock.method(console, "warn", () => {});
  t.mock.method(globalThis, "fetch", async (request) => {
    assert.equal(request.cache, "reload");
    if (request.url.endsWith("/manifest.json")) return new Response(JSON.stringify(manifest));
    assert.equal(request.url, spec.wasmRuntime.onnxUrl);
    counts.modelFetches += 1;
    return new Response(networkBytes);
  });
  return { counts, entries };
}

test("a stale cached ONNX file is evicted and replaced before use", async (t) => {
  const { counts, entries } = installCache(t, oldModel);
  const loaded = await loadOnnxModelBytes(spec, identity);
  assert.equal(digest(loaded), identity.modelSha256);
  assert.equal(digest(await entries.get(spec.wasmRuntime.onnxUrl).clone().arrayBuffer()), identity.modelSha256);
  assert.deepEqual(counts, { modelFetches: 1, deletes: 1, puts: 1 });
  await loadOnnxModelBytes(spec, identity);
  assert.equal(counts.modelFetches, 1);
});

test("corrupt fetched ONNX bytes are rejected and never cached", async (t) => {
  const { counts, entries } = installCache(t, oldModel, oldModel);
  await assert.rejects(loadOnnxModelBytes(spec, identity), /SHA-256 mismatch/);
  assert.equal(entries.has(spec.wasmRuntime.onnxUrl), false);
  assert.deepEqual(counts, { modelFetches: 1, deletes: 1, puts: 0 });
});

test("ONNX bytes are checked when CacheStorage is unavailable", async (t) => {
  installCache(t, null, oldModel);
  delete globalThis.caches;
  await assert.rejects(cachedFetchArrayBuffer(spec, spec.wasmRuntime.onnxUrl, "model", identity.modelSha256), /SHA-256 mismatch/);
});

test("a fallback manifest from another model is rejected before fetching weights", async (t) => {
  const { counts } = installCache(t, newModel, newModel, { ...fallbackManifest, model_sha256: digest(oldModel) });
  await assert.rejects(loadOnnxModelBytes(spec, identity), /model_sha256 mismatch/);
  assert.equal(counts.modelFetches, 0);
});

test("the query identity comes from the exact manifest bytes sent to Rust", async () => {
  const manifestBytes = bytes(JSON.stringify(artifactManifest));
  const source = modelArtifactSource(spec, async () => manifestBytes);
  assert.throws(() => source.identity(), /did not read/);
  assert.equal(await source.fetchArrayBuffer("https://example.invalid/model/manifest.json"), manifestBytes);
  assert.deepEqual(source.identity(), { ...identity, artifactManifestSha256: digest(manifestBytes) });
  assert.equal(Object.isFrozen(source.identity()), true);
  for (const field of ["source_revision", "q4_onnx_sha256", "tokenizer"]) {
    const invalid = { ...artifactManifest };
    delete invalid[field];
    await assert.rejects(readModelArtifactIdentity(bytes(JSON.stringify(invalid)), spec), /required|digest/);
  }
});

test("persisted packs require the loaded revision, weights and any declared tokenizer identity", () => {
  assert.equal(modelIdentityCompatibilityIssue(identity, identity), null);
  assert.equal(modelIdentityCompatibilityIssue(null, identity)?.field, "modelRevision");
  for (const field of ["modelRevision", "modelSha256", "tokenizerSha256", "artifactManifestSha256"]) {
    const changed = { ...identity, [field]: field === "modelRevision" ? "old-revision" : "f".repeat(64) };
    assert.equal(modelIdentityCompatibilityIssue(changed, identity)?.field, field);
  }
});

test("remote manifests with another model revision or digest cannot authorize a pack", () => {
  const runtime = { runtime: "custom", version: "0.2.0", dtype: "q4", device: "webgpu" };
  const corpus = { inputHash: "input", inputFormatVersion: "format" };
  const manifest = {
    schema_version: 1, model_key: spec.modelKey,
    model_revision: identity.modelRevision, q4_onnx_sha256: identity.modelSha256,
    input_hash: corpus.inputHash, input_format_version: corpus.inputFormatVersion,
    dimensions: 2, element_type: "f16le", normalized: true, distance: "dot",
    pooling: spec.outputPooling, max_window_tokens: 512,
    compatible_query_runtimes: [{ ...runtime, pooling: spec.outputPooling, max_window_tokens: 512 }],
  };
  const issue = (value) => manifestCompatibilityIssue(value, corpus, runtime, identity, spec);
  assert.equal(issue(manifest), null);
  assert.equal(issue({ ...manifest, model_revision: "old-revision" })?.field, "modelRevision");
  assert.equal(issue({ ...manifest, q4_onnx_sha256: digest(oldModel) })?.field, "modelSha256");
  for (const field of ["model_revision", "q4_onnx_sha256"]) {
    const missing = { ...manifest }; delete missing[field];
    assert.notEqual(issue(missing), null);
  }
});

test("a new model artifact cannot overwrite the previous local pack's shards", async () => {
  const first = (await readModelArtifactIdentity(bytes(JSON.stringify(artifactManifest)), spec)).modelIdentity;
  const changed = { ...artifactManifest, source_revision: "next-revision", q4_onnx_sha256: digest(oldModel) };
  const second = (await readModelArtifactIdentity(bytes(JSON.stringify(changed)), spec)).modelIdentity;
  const firstId = localVectorPackId("space", "same-corpus", first);
  const secondId = localVectorPackId("space", "same-corpus", second);
  assert.notEqual(firstId, secondId);
  assert.equal(firstId, localVectorPackId("space", "same-corpus", first));
  assert.throws(() => localVectorPackId("space", "same-corpus", {}), /artifact identity/);
});
