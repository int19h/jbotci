const HARNESS_BUILD = "runtime-onnx-q4-artifact-2026-06-05-1";
const DEFAULT_RUNTIME_URL = withQueryParam(
  new URL("../../../apps/jbotci-web/assets/f2llm-webgpu-runtime.js", import.meta.url),
  "v",
  HARNESS_BUILD,
).href;
const DEFAULT_ARTIFACT_URL = new URL(
  "../../../.jbotci-build/f2llm-v2-80m-webgpu/v1",
  import.meta.url,
).href;
const DEFAULT_VECTORS_URL = new URL(
  "../../../.jbotci-build/r2-web-embeddings",
  import.meta.url,
).href;
const DEFAULT_GOLDENS_URL = new URL(
  "../../../.jbotci-build/f2llm-webgpu-goldens/goldens.json",
  import.meta.url,
).href;
const DEFAULT_ORT_URL = new URL(
  "../../../tools/embedding-pack/node_modules/onnxruntime-web/dist/ort.wasm.min.mjs",
  import.meta.url,
).href;
const DEFAULT_ORT_WASM_URL = new URL(
  "../../../tools/embedding-pack/node_modules/onnxruntime-web/dist/",
  import.meta.url,
).href;
const DEFAULT_ONNX_URL = new URL(
  "../../../.jbotci-build/f2llm-onnx-reference/v1/model.onnx",
  import.meta.url,
).href;
const DEFAULT_ONNX_Q4_URL = new URL(
  "/__jbotci-f2llm-q4-onnx/model_q4.onnx",
  import.meta.url,
).href;
const MODEL_KEY = "f2llm-v2-80m-q4-320";
const RUNTIME = "jbotci-webgpu-f2llm";
const RUNTIME_VERSION = "0.1.0";
const DIMENSIONS = 320;
const MAX_SEQUENCE_LENGTH = 512;
const QUERY_PREFIX =
  "Instruct: Given a question, retrieve passages that can help answer the question.\nQuery: ";

const state = {
  startedAt: performance.now(),
  events: [],
  runtimeModule: null,
  runtime: null,
  artifactManifest: null,
  vectorManifest: null,
  corpus: null,
  lastEmbedding: null,
  ortModule: null,
  ortSessions: new Map(),
};

function withQueryParam(url, name, value) {
  url.searchParams.set(name, value);
  return url;
}

const dom = Object.fromEntries(
  [
    "runtime-url",
    "artifact-url",
    "vectors-url",
    "corpus-id",
    "repeat-count",
    "expected-url",
    "ort-url",
    "ort-wasm-url",
    "onnx-url",
    "onnx-q4-url",
    "query",
    "status",
    "log",
    "env",
    "load",
    "embed",
    "goldens",
    "onnx",
    "onnx-goldens",
    "pack",
    "search",
    "repeat",
    "run-all",
    "copy",
    "download",
    "clear",
  ].map((id) => [id, document.getElementById(id)]),
);

const params = new URLSearchParams(location.search);
dom["runtime-url"].value = params.get("runtime") || DEFAULT_RUNTIME_URL;
dom["artifact-url"].value = params.get("artifact") || DEFAULT_ARTIFACT_URL;
dom["vectors-url"].value = params.get("vectors") || DEFAULT_VECTORS_URL;
dom["corpus-id"].value = params.get("corpus") || "vlacku-en";
dom.query.value = params.get("query") || dom.query.value;
dom["expected-url"].value = params.get("expected") || DEFAULT_GOLDENS_URL;
dom["ort-url"].value = params.get("ort") || DEFAULT_ORT_URL;
dom["ort-wasm-url"].value = params.get("ortWasm") || DEFAULT_ORT_WASM_URL;
dom["onnx-url"].value = params.get("onnx") || DEFAULT_ONNX_URL;
dom["onnx-q4-url"].value = params.get("onnxQ4") || params.get("q4Onnx") || DEFAULT_ONNX_Q4_URL;

wire("env", logEnvironment);
wire("load", loadRuntime);
wire("embed", embedQuery);
wire("goldens", runGoldenSet);
wire("onnx", runOnnxReference);
wire("onnx-goldens", runOnnxGoldenSet);
wire("pack", loadVectorPack);
wire("search", search);
wire("repeat", repeatEmbeddings);
wire("run-all", runAll);
wire("copy", copyLog);
wire("download", downloadLog);
dom.clear.addEventListener("click", () => {
  state.events = [];
  renderLog();
  setStatus("Log cleared.");
});

record("harness-ready", {
  harnessBuild: HARNESS_BUILD,
  location: location.href,
  queryParams: Object.fromEntries(params.entries()),
  defaults: {
    artifactUrl: DEFAULT_ARTIFACT_URL,
    vectorsUrl: DEFAULT_VECTORS_URL,
    goldensUrl: DEFAULT_GOLDENS_URL,
    ortUrl: DEFAULT_ORT_URL,
    ortWasmUrl: DEFAULT_ORT_WASM_URL,
    onnxUrl: DEFAULT_ONNX_URL,
    onnxQ4Url: DEFAULT_ONNX_Q4_URL,
  },
  runtimeUrl: dom["runtime-url"].value,
  artifactUrl: dom["artifact-url"].value,
  vectorsUrl: dom["vectors-url"].value,
  corpusId: dom["corpus-id"].value,
});

function wire(id, action) {
  dom[id].addEventListener("click", () => runAction(id, action));
}

async function runAction(label, action) {
  setButtonsDisabled(true);
  setStatus(`Running ${label}...`);
  const start = performance.now();
  try {
    await action();
    setStatus(`${label} completed in ${formatMs(performance.now() - start)}.`);
  } catch (error) {
    record("error", {
      action: label,
      message: error instanceof Error ? error.message : String(error),
      stack: error instanceof Error ? error.stack : null,
    });
    setStatus(`${label} failed: ${error instanceof Error ? error.message : String(error)}`);
  } finally {
    setButtonsDisabled(false);
  }
}

async function runAll() {
  await logEnvironment();
  await loadRuntime();
  await embedQuery();
  await runGoldenSet();
  await runOnnxReference();
  await runOnnxGoldenSet();
  await repeatEmbeddings();
}

async function logEnvironment() {
  const gpu = await gpuEnvironment();
  record("environment", {
    location: location.href,
    secureContext: globalThis.isSecureContext,
    userAgent: navigator.userAgent,
    platform: navigator.platform,
    userAgentData: userAgentData(),
    language: navigator.language,
    languages: navigator.languages,
    hardwareConcurrency: navigator.hardwareConcurrency,
    deviceMemory: navigator.deviceMemory ?? null,
    maxTouchPoints: navigator.maxTouchPoints,
    storage: await storageEstimate(),
    performanceMemory: performanceMemory(),
    gpu,
  });
}

async function gpuEnvironment() {
  if (!navigator.gpu?.requestAdapter) {
    return { available: false, reason: "navigator.gpu.requestAdapter is missing" };
  }
  const adapter = await navigator.gpu.requestAdapter().catch((error) => ({
    error: error instanceof Error ? error.message : String(error),
  }));
  if (!adapter || adapter.error) {
    return { available: false, reason: adapter?.error || "requestAdapter returned null" };
  }
  let info = null;
  if (adapter.info) {
    info = adapter.info;
  } else if (typeof adapter.requestAdapterInfo === "function") {
    info = await adapter.requestAdapterInfo().catch((error) => ({
      error: error instanceof Error ? error.message : String(error),
    }));
  }
  return {
    available: true,
    info,
    features: Array.from(adapter.features || []).sort(),
    limits: adapterLimits(adapter.limits),
  };
}

async function loadRuntime() {
  const runtimeUrl = requiredUrl(dom["runtime-url"].value, "runtime module URL");
  const artifactBaseUrl = trimBaseUrl(dom["artifact-url"].value, "artifact base URL");
  record("load-runtime-start", {
    runtimeUrl,
    artifactBaseUrl,
    memory: await memorySnapshot(),
  });
  state.artifactManifest = await loadArtifactManifest(artifactBaseUrl);
  const moduleStart = performance.now();
  state.runtimeModule = await import(runtimeUrl);
  record("runtime-module-imported", {
    ms: elapsed(moduleStart),
    exports: Object.keys(state.runtimeModule).sort(),
  });
  const loadStart = performance.now();
  state.runtime = await state.runtimeModule.F2LlmWebGpuRuntime.load({
    baseUrl: artifactBaseUrl,
    expectedModelKey: MODEL_KEY,
    expectedRuntime: RUNTIME,
    expectedVersion: RUNTIME_VERSION,
    maxSequenceLength: MAX_SEQUENCE_LENGTH,
    dimensions: DIMENSIONS,
    progress: (progress) => {
      record("runtime-progress", progress);
    },
  });
  record("load-runtime-finished", {
    ms: elapsed(loadStart),
    memory: await memorySnapshot(),
  });
}

async function loadArtifactManifest(baseUrl) {
  const start = performance.now();
  const manifest = await fetchJson(`${baseUrl}/manifest.json`, "artifact manifest");
  const tensors = Object.entries(manifest.tensors || {});
  const tensorBytes = tensors.reduce((sum, [, tensor]) => sum + tensorByteLength(tensor), 0);
  const chunkCounts = tensors.reduce((sum, [, tensor]) => sum + tensorChunkCount(tensor), 0);
  const largestChunk = Math.max(0, ...tensors.flatMap(([, tensor]) => tensorChunks(tensor).map((chunk) => chunk.byte_length || 0)));
  record("artifact-manifest", {
    ms: elapsed(start),
    schemaVersion: manifest.schema_version,
    runtime: manifest.runtime,
    artifactVersion: manifest.artifact_version,
    modelKey: manifest.model_key,
    maxSequenceLength: manifest.max_sequence_length,
    model: manifest.model,
    quantization: manifest.quantization,
    tokenizer: manifest.tokenizer,
    tensorCount: tensors.length,
    tensorBytes,
    chunkCounts,
    largestChunk,
    requiredTensorAudit: requiredTensorAudit(manifest),
  });
  return manifest;
}

async function embedQuery() {
  if (state.runtime === null) {
    await loadRuntime();
  }
  const text = QUERY_PREFIX + dom.query.value.trim();
  const tokenIds = state.runtime.tokenizer.encode(text, MAX_SEQUENCE_LENGTH);
  record("tokenized-query", {
    inputChars: text.length,
    tokenCount: tokenIds.length,
    tokenHead: tokenIds.slice(0, 16),
    tokenTail: tokenIds.slice(-16),
  });
  const before = await memorySnapshot();
  const start = performance.now();
  const rows = await state.runtime.embedTexts([text]);
  const vector = rows[0];
  state.lastEmbedding = vector;
  const summary = await embeddingSummary(vector);
  record("embed-query-finished", {
    ms: elapsed(start),
    before,
    after: await memorySnapshot(),
    summary,
  });
  await compareExpectedEmbedding(vector);
}

async function compareExpectedEmbedding(vector) {
  const expectedUrl = dom["expected-url"].value.trim();
  if (expectedUrl.length === 0) {
    return;
  }
  const expected = await fetchJson(expectedUrl, "expected embedding JSON");
  const text = QUERY_PREFIX + dom.query.value.trim();
  const expectedVector = expectedVectorForText(expected, text);
  if (!expectedVector) {
    record("expected-embedding-missing", {
      expectedUrl,
      keys: Object.keys(expected),
      inputChars: text.length,
    });
    return;
  }
  const comparison = compareVectors(vector, expectedVector);
  const threshold = expected.cosine_threshold ?? expected.threshold ?? null;
  record("expected-embedding-comparison", {
    expectedUrl,
    dimensions: expectedVector.length,
    ...comparison,
    threshold,
    passed: threshold === null ? null : comparison.cosine >= Number(threshold),
  });
}

async function runGoldenSet() {
  if (state.runtime === null) {
    await loadRuntime();
  }
  const expectedUrl = dom["expected-url"].value.trim();
  if (expectedUrl.length === 0) {
    throw new Error("golden embedding JSON URL is empty");
  }
  const goldens = await fetchJson(expectedUrl, "golden embedding JSON");
  const cases = Array.isArray(goldens.cases) ? goldens.cases : [];
  if (cases.length === 0) {
    throw new Error("golden embedding JSON has no cases");
  }
  const threshold = Number(goldens.cosine_threshold ?? goldens.threshold ?? 0.95);
  const before = await memorySnapshot();
  const results = [];
  for (const golden of cases) {
    const input = String(golden.input ?? "");
    const tokenIds = state.runtime.tokenizer.encode(input, MAX_SEQUENCE_LENGTH);
    const expectedTokenIds = Array.isArray(golden.token_ids) ? golden.token_ids.map(Number) : null;
    const tokenMatch = expectedTokenIds === null
      ? null
      : arraysEqual(tokenIds, expectedTokenIds);
    const start = performance.now();
    const rows = await state.runtime.embedTexts([input]);
    const summary = await embeddingSummary(rows[0]);
    const comparison = compareVectors(rows[0], golden.embedding || []);
    results.push({
      name: golden.name || "",
      kind: golden.kind || "",
      inputChars: input.length,
      tokenCount: tokenIds.length,
      tokenMatch,
      tokenHead: tokenIds.slice(0, 16),
      tokenTail: tokenIds.slice(-16),
      ms: elapsed(start),
      summary,
      comparison,
      passed: tokenMatch !== false && comparison.cosine >= threshold,
    });
  }
  record("golden-set-finished", {
    expectedUrl,
    referenceRuntime: goldens.reference?.runtime || null,
    referenceModel: goldens.reference?.model || null,
    caseCount: cases.length,
    threshold,
    passed: results.every((result) => result.passed),
    before,
    after: await memorySnapshot(),
    results,
  });
}

async function runOnnxReference() {
  if (state.runtime === null) {
    await loadRuntime();
  }
  const text = QUERY_PREFIX + dom.query.value.trim();
  const tokenIds = state.runtime.tokenizer.encode(text, MAX_SEQUENCE_LENGTH);
  const references = onnxReferences();
  if (references.length === 0) {
    throw new Error("at least one ONNX model URL must be provided");
  }
  const before = await memorySnapshot();
  const customStart = performance.now();
  const customRows = await state.runtime.embedTexts([text]);
  const customMs = elapsed(customStart);
  const customVector = customRows[0];
  const customSummary = await embeddingSummary(customVector);
  const onnxResults = [];
  for (const reference of references) {
    const session = await loadOnnxSession(reference);
    const result = await runOnnxEmbedding({
      reference,
      session,
      tokenIds,
    });
    onnxResults.push(result);
  }
  const comparisons = [];
  for (const result of onnxResults) {
    comparisons.push({
      left: "custom-webgpu-q4",
      right: result.label,
      ...compareVectors(customVector, result.vector),
    });
  }
  for (let left = 0; left < onnxResults.length; left += 1) {
    for (let right = left + 1; right < onnxResults.length; right += 1) {
      comparisons.push({
        left: onnxResults[left].label,
        right: onnxResults[right].label,
        ...compareVectors(onnxResults[left].vector, onnxResults[right].vector),
      });
    }
  }
  record("onnx-reference-finished", {
    ortUrl: dom["ort-url"].value.trim(),
    ortWasmUrl: dom["ort-wasm-url"].value.trim(),
    onnxUrl: dom["onnx-url"].value.trim(),
    onnxQ4Url: dom["onnx-q4-url"].value.trim(),
    tokenCount: tokenIds.length,
    tokenHead: tokenIds.slice(0, 16),
    tokenTail: tokenIds.slice(-16),
    customMs,
    before,
    after: await memorySnapshot(),
    customSummary,
    references: onnxResults.map((result) => ({
      label: result.label,
      url: result.url,
      ms: result.ms,
      inputNames: result.inputNames,
      outputNames: result.outputNames,
      outputName: result.outputName,
      outputDims: result.outputDims,
      pooling: result.pooling,
      summary: result.summary,
    })),
    comparisons,
  });
}

async function runOnnxGoldenSet() {
  if (state.runtime === null) {
    await loadRuntime();
  }
  const expectedUrl = dom["expected-url"].value.trim();
  if (expectedUrl.length === 0) {
    throw new Error("golden embedding JSON URL is empty");
  }
  const goldens = await fetchJson(expectedUrl, "golden embedding JSON");
  const cases = Array.isArray(goldens.cases) ? goldens.cases : [];
  if (cases.length === 0) {
    throw new Error("golden embedding JSON has no cases");
  }
  const references = onnxReferences();
  if (references.length === 0) {
    throw new Error("at least one ONNX model URL must be provided");
  }
  const q4Threshold = Number(goldens.q4_onnx_threshold ?? 0.995);
  const before = await memorySnapshot();
  const results = [];
  for (const golden of cases) {
    const input = String(golden.input ?? "");
    const tokenIds = state.runtime.tokenizer.encode(input, MAX_SEQUENCE_LENGTH);
    const expectedTokenIds = Array.isArray(golden.token_ids) ? golden.token_ids.map(Number) : null;
    const tokenMatch = expectedTokenIds === null ? null : arraysEqual(tokenIds, expectedTokenIds);
    const customStart = performance.now();
    const customRows = await state.runtime.embedTexts([input]);
    const customMs = elapsed(customStart);
    const customVector = customRows[0];
    const customSummary = await embeddingSummary(customVector);
    const referenceResults = [];
    for (const reference of references) {
      const session = await loadOnnxSession(reference);
      const result = await runOnnxEmbedding({ reference, session, tokenIds });
      referenceResults.push(result);
    }
    const comparisons = referenceResults.map((result) => ({
      left: "custom-webgpu-q4",
      right: result.label,
      ...compareVectors(customVector, result.vector),
    }));
    const q4Comparison = comparisons.find((comparison) => comparison.right === "q4-onnx-transformersjs") || null;
    results.push({
      name: golden.name || "",
      kind: golden.kind || "",
      inputChars: input.length,
      tokenCount: tokenIds.length,
      tokenMatch,
      tokenHead: tokenIds.slice(0, 16),
      tokenTail: tokenIds.slice(-16),
      customMs,
      customSummary,
      references: referenceResults.map((result) => ({
        label: result.label,
        ms: result.ms,
        outputName: result.outputName,
        outputDims: result.outputDims,
        pooling: result.pooling,
        summary: result.summary,
      })),
      comparisons,
      passed: tokenMatch !== false && q4Comparison !== null && q4Comparison.cosine >= q4Threshold,
    });
  }
  record("onnx-golden-set-finished", {
    expectedUrl,
    q4Threshold,
    caseCount: cases.length,
    passed: results.every((result) => result.passed),
    before,
    after: await memorySnapshot(),
    results,
  });
}

function onnxReferences() {
  const refs = [];
  const f32Url = dom["onnx-url"].value.trim();
  if (f32Url.length > 0) {
    refs.push({
      label: "fp32-onnx-wrapper",
      url: requiredUrl(f32Url, "ONNX reference model URL"),
    });
  }
  const q4Url = dom["onnx-q4-url"].value.trim();
  if (q4Url.length > 0) {
    refs.push({
      label: "q4-onnx-transformersjs",
      url: requiredUrl(q4Url, "q4 ONNX model URL"),
    });
  }
  return refs;
}

async function loadOrtModule() {
  if (state.ortModule !== null) {
    return state.ortModule;
  }
  const ortUrl = requiredUrl(dom["ort-url"].value, "ORT Web module URL");
  const wasmUrl = trimBaseUrl(dom["ort-wasm-url"].value, "ORT WASM directory URL");
  const importStart = performance.now();
  const ort = await import(ortUrl);
  state.ortModule = ort;
  ort.env.wasm.wasmPaths = `${wasmUrl}/`;
  ort.env.wasm.numThreads = 1;
  ort.env.wasm.proxy = false;
  record("ort-module-imported", {
    ms: elapsed(importStart),
    version: ort.env.versions?.web || null,
    wasmPaths: ort.env.wasm.wasmPaths,
  });
  return ort;
}

async function loadOnnxSession(reference) {
  const key = `${reference.label}\n${reference.url}`;
  const cached = state.ortSessions.get(key);
  if (cached) {
    return cached;
  }
  const ort = await loadOrtModule();
  const createStart = performance.now();
  await preflightOnnxModel(reference);
  const session = await ort.InferenceSession.create(reference.url, {
    executionProviders: ["wasm"],
    graphOptimizationLevel: "all",
  });
  state.ortSessions.set(key, session);
  record("onnx-session-loaded", {
    label: reference.label,
    ms: elapsed(createStart),
    onnxUrl: reference.url,
    inputNames: session.inputNames,
    outputNames: session.outputNames,
    memory: await memorySnapshot(),
  });
  return session;
}

async function preflightOnnxModel(reference) {
  const start = performance.now();
  const response = await fetchWithDiagnostics(reference.url, `${reference.label} ONNX preflight`, {
    method: "HEAD",
    cache: "no-cache",
  });
  const detail = {
    label: reference.label,
    url: response.url || reference.url,
    ms: elapsed(start),
    status: response.status,
    statusText: response.statusText,
    ok: response.ok,
    contentType: response.headers.get("content-type"),
    contentLength: response.headers.get("content-length"),
    acceptRanges: response.headers.get("accept-ranges"),
    hint: onnxPreflightHint(reference, response),
  };
  record("onnx-model-preflight", detail);
  if (!response.ok) {
    throw new Error(
      `${reference.label} ONNX preflight failed: ${response.status} ${response.statusText}. ${detail.hint}`,
    );
  }
}

function onnxPreflightHint(reference, response) {
  if (response.ok) {
    return null;
  }
  if (reference.url.includes("/__jbotci-f2llm-q4-onnx/")) {
    return "The local q4 ONNX route is served by browser-harness/server.mjs. Restart that server after updating the worktree, and check that it logs a local q4 ONNX path or pass --q4-onnx explicitly.";
  }
  return "Check that the ONNX URL points to an existing local file or a remote URL with CORS enabled.";
}

async function runOnnxEmbedding({ reference, session, tokenIds }) {
  const ort = await loadOrtModule();
  const feeds = onnxFeeds(ort, session.inputNames, tokenIds);
  const start = performance.now();
  const outputs = await session.run(feeds);
  const outputName = selectOnnxOutputName(outputs);
  const output = outputs[outputName];
  const vector = pooledOnnxVector(output, tokenIds.length);
  normalizeVectorInPlace(vector);
  return {
    label: reference.label,
    url: reference.url,
    ms: elapsed(start),
    inputNames: session.inputNames,
    outputNames: session.outputNames,
    outputName,
    outputDims: Array.from(output.dims || []),
    pooling: output.dims?.length === 3 ? "last-token" : "already-pooled",
    vector,
    summary: await embeddingSummary(vector),
  };
}

function onnxFeeds(ort, inputNames, tokenIds) {
  const feeds = {};
  for (const inputName of inputNames) {
    if (inputName === "input_ids") {
      feeds[inputName] = new ort.Tensor(
        "int64",
        BigInt64Array.from(tokenIds.map((token) => BigInt(token))),
        [1, tokenIds.length],
      );
    } else if (inputName === "attention_mask") {
      const attentionMask = new BigInt64Array(tokenIds.length);
      attentionMask.fill(1n);
      feeds[inputName] = new ort.Tensor("int64", attentionMask, [1, tokenIds.length]);
    } else if (inputName === "position_ids") {
      feeds[inputName] = new ort.Tensor(
        "int64",
        BigInt64Array.from(tokenIds.map((_, index) => BigInt(index))),
        [1, tokenIds.length],
      );
    } else {
      throw new Error(`unsupported ONNX input: ${inputName}`);
    }
  }
  return feeds;
}

function selectOnnxOutputName(outputs) {
  if (outputs.embedding) {
    return "embedding";
  }
  if (outputs.last_hidden_state) {
    return "last_hidden_state";
  }
  return Object.keys(outputs)[0];
}

function pooledOnnxVector(output, tokenCount) {
  const dims = Array.from(output.dims || []);
  const data = output.data;
  if (dims.length === 2 && dims[0] === 1 && dims[1] === DIMENSIONS) {
    return Float32Array.from(data);
  }
  if (dims.length === 3 && dims[0] === 1 && dims[1] >= tokenCount && dims[2] === DIMENSIONS) {
    const start = (tokenCount - 1) * DIMENSIONS;
    return Float32Array.from(data.slice(start, start + DIMENSIONS));
  }
  if (data.length === DIMENSIONS) {
    return Float32Array.from(data);
  }
  throw new Error(`unsupported ONNX output shape: [${dims.join(", ")}], data length ${data.length}`);
}

function expectedVectorForText(expected, text) {
  if (Array.isArray(expected.embedding)) {
    return expected.embedding;
  }
  if (Array.isArray(expected.embeddings?.[text])) {
    return expected.embeddings[text];
  }
  const cases = Array.isArray(expected.cases) ? expected.cases : [];
  const direct = cases.find((item) => item.input === text);
  if (Array.isArray(direct?.embedding)) {
    return direct.embedding;
  }
  return null;
}

function compareVectors(actualVector, expectedVector) {
  if (!Array.isArray(expectedVector) && !(expectedVector instanceof Float32Array)) {
    throw new Error("expected vector is not an array");
  }
  if (actualVector.length !== expectedVector.length) {
    throw new Error(`vector length mismatch: actual ${actualVector.length}, expected ${expectedVector.length}`);
  }
  let dot = 0;
  let expectedNorm = 0;
  let actualNorm = 0;
  let maxAbsDiff = 0;
  for (let index = 0; index < actualVector.length; index += 1) {
    const actual = Number(actualVector[index]);
    const wanted = Number(expectedVector[index]);
    dot += actual * wanted;
    actualNorm += actual * actual;
    expectedNorm += wanted * wanted;
    maxAbsDiff = Math.max(maxAbsDiff, Math.abs(actual - wanted));
  }
  return {
    cosine: dot / Math.sqrt(actualNorm * expectedNorm),
    maxAbsDiff,
  };
}

function normalizeVectorInPlace(vector) {
  let squared = 0;
  for (const value of vector) {
    squared += value * value;
  }
  const norm = Math.sqrt(squared);
  if (norm === 0) {
    return;
  }
  for (let index = 0; index < vector.length; index += 1) {
    vector[index] /= norm;
  }
}

function arraysEqual(left, right) {
  if (left.length !== right.length) {
    return false;
  }
  for (let index = 0; index < left.length; index += 1) {
    if (Number(left[index]) !== Number(right[index])) {
      return false;
    }
  }
  return true;
}

async function loadVectorPack() {
  const vectorsBaseUrl = trimBaseUrl(dom["vectors-url"].value, "vector base URL");
  const catalog = await fetchJson(`${vectorsBaseUrl}/catalog.json`, "vector catalog");
  const model = (catalog.models || []).find((entry) => entry.model_key === MODEL_KEY);
  if (!model) {
    throw new Error(`catalog does not contain ${MODEL_KEY}`);
  }
  const vectorSpace = (model.vector_spaces || []).find((space) =>
    (space.compatible_query_runtimes || []).some((runtime) =>
      runtime.runtime === RUNTIME
        && runtime.version === RUNTIME_VERSION
        && runtime.dtype === "q4"
        && runtime.device === "webgpu"
    )
  );
  if (!vectorSpace?.manifest_url) {
    throw new Error("catalog has no compatible F2LLM WebGPU vector space");
  }
  const manifestUrl = absoluteUrl(vectorsBaseUrl, vectorSpace.manifest_url);
  const manifest = await fetchJson(manifestUrl, "vector manifest");
  const corpusId = dom["corpus-id"].value;
  const corpusManifest = (manifest.corpora || []).find((entry) => entry.corpus_id === corpusId);
  if (!corpusManifest) {
    throw new Error(`vector manifest does not contain corpus ${corpusId}`);
  }
  const packBase = manifestUrl.replace(/\/manifest\.json$/, "");
  const itemsUrl = absoluteUrl(packBase, corpusManifest.items_url);
  const itemBytes = await fetchArrayBuffer(itemsUrl, "corpus items");
  await verifySha256(itemBytes, corpusManifest.items_sha256, corpusManifest.items_url);
  const items = JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(itemBytes));
  const vectorUrl = absoluteUrl(packBase, corpusManifest.vector_url);
  state.vectorManifest = manifest;
  state.corpus = {
    corpusId,
    inputHash: corpusManifest.input_hash,
    rowCount: corpusManifest.row_count,
    dimensions: corpusManifest.dimensions,
    elementType: manifest.element_type,
    items: normalizeItems(items),
    shards: [{ key: vectorUrl, byteLen: corpusManifest.vector_byte_len }],
    vectorSha256: corpusManifest.vector_sha256,
  };
  record("vector-pack-loaded", {
    vectorSpace,
    manifest: {
      modelKey: manifest.model_key,
      packId: manifest.pack_id,
      vectorSpaceKey: manifest.vector_space_key,
      elementType: manifest.element_type,
      dimensions: manifest.dimensions,
      maxSequenceLength: manifest.max_sequence_length,
      compatibleQueryRuntimes: manifest.compatible_query_runtimes,
    },
    corpus: {
      corpusId,
      rows: state.corpus.rowCount,
      dimensions: state.corpus.dimensions,
      itemsBytes: itemBytes.byteLength,
      vectorBytes: corpusManifest.vector_byte_len,
      vectorUrl,
      firstItems: state.corpus.items.slice(0, 5),
    },
    memory: await memorySnapshot(),
  });
}

async function search() {
  if (state.runtime === null) {
    await loadRuntime();
  }
  if (state.lastEmbedding === null) {
    await embedQuery();
  }
  if (state.corpus === null) {
    await loadVectorPack();
  }
  const before = await memorySnapshot();
  const start = performance.now();
  const hits = await state.runtime.rankHits({
    corpus: state.corpus,
    query: state.lastEmbedding,
    limit: 10,
    itemMatches: () => true,
    readBinary: async (url) => {
      const vectorStart = performance.now();
      const bytes = await fetchArrayBuffer(url, "corpus vectors");
      await verifySha256(bytes, state.corpus.vectorSha256, "corpus vectors");
      record("vector-shard-fetched", {
        url,
        bytes: bytes.byteLength,
        ms: elapsed(vectorStart),
        memory: await memorySnapshot(),
      });
      return bytes;
    },
  });
  record("search-finished", {
    ms: elapsed(start),
    before,
    after: await memorySnapshot(),
    hits,
  });
}

async function repeatEmbeddings() {
  if (state.runtime === null) {
    await loadRuntime();
  }
  const count = Math.max(1, Math.min(50, Number.parseInt(dom["repeat-count"].value, 10) || 1));
  const text = QUERY_PREFIX + dom.query.value.trim();
  const runs = [];
  const before = await memorySnapshot();
  for (let index = 0; index < count; index += 1) {
    const start = performance.now();
    const rows = await state.runtime.embedTexts([text]);
    const summary = await embeddingSummary(rows[0]);
    runs.push({
      index,
      ms: elapsed(start),
      sha256: summary.sha256,
      norm: summary.norm,
    });
  }
  record("repeat-embeddings-finished", {
    count,
    before,
    after: await memorySnapshot(),
    runs,
  });
}

async function embeddingSummary(vector) {
  let sum = 0;
  let squared = 0;
  let min = Infinity;
  let max = -Infinity;
  for (const value of vector) {
    sum += value;
    squared += value * value;
    min = Math.min(min, value);
    max = Math.max(max, value);
  }
  return {
    dimensions: vector.length,
    norm: Math.sqrt(squared),
    min,
    max,
    mean: sum / vector.length,
    first16: Array.from(vector.slice(0, 16)),
    sha256: await sha256Hex(float32Bytes(vector)),
  };
}

function normalizeItems(items) {
  return items.map((item, row) => ({
    id: Number(item.entry_index ?? item.chunk_index ?? item.id),
    row,
    kind: typeof item.kind === "string" ? item.kind : null,
    inputHash: item.input_hash || item.inputHash || null,
  }));
}

function requiredTensorAudit(manifest) {
  const tensors = manifest.tensors || {};
  const missing = [];
  const layers = manifest.model?.num_hidden_layers || 0;
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
  for (const name of required) {
    if (!tensors[name]) {
      missing.push(name);
    }
  }
  return {
    requiredCount: required.length,
    missing,
  };
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

function tensorChunkCount(tensor) {
  return tensorChunks(tensor).length;
}

function tensorChunks(tensor) {
  if (tensor.kind === "q4_rowwise") {
    return [...(tensor.qweight?.chunks || []), ...(tensor.scales?.chunks || [])];
  }
  if (tensor.kind === "q4_onnx_gather" || tensor.kind === "q4_onnx_matmul") {
    return [
      ...(tensor.qweight?.chunks || []),
      ...(tensor.scales?.chunks || []),
      ...(tensor.zero_points?.chunks || []),
    ];
  }
  return tensor.data?.chunks || [];
}

async function fetchJson(url, label) {
  const start = performance.now();
  const response = await fetchWithDiagnostics(url, label, { cache: "no-cache" });
  if (!response.ok) {
    const preview = await response.text().catch((error) => `preview unavailable: ${error.message}`);
    record("fetch-http-error", {
      label,
      url: response.url || url,
      status: response.status,
      statusText: response.statusText,
      contentType: response.headers.get("content-type"),
      preview: preview.slice(0, 240),
    });
    throw new Error(`${label} fetch failed: ${response.status} ${response.url || url}`);
  }
  const text = await response.text();
  record("fetch-json", {
    label,
    url,
    ms: elapsed(start),
    bytes: text.length,
    contentType: response.headers.get("content-type"),
    contentLength: response.headers.get("content-length"),
    contentEncoding: response.headers.get("content-encoding"),
  });
  return JSON.parse(text);
}

async function fetchArrayBuffer(url, label) {
  const start = performance.now();
  const response = await fetchWithDiagnostics(url, label);
  if (!response.ok) {
    const preview = await response.text().catch((error) => `preview unavailable: ${error.message}`);
    record("fetch-http-error", {
      label,
      url: response.url || url,
      status: response.status,
      statusText: response.statusText,
      contentType: response.headers.get("content-type"),
      preview: preview.slice(0, 240),
    });
    throw new Error(`${label} fetch failed: ${response.status} ${response.url || url}`);
  }
  const bytes = await response.arrayBuffer();
  record("fetch-binary", {
    label,
    url,
    ms: elapsed(start),
    bytes: bytes.byteLength,
    contentType: response.headers.get("content-type"),
    contentLength: response.headers.get("content-length"),
    contentEncoding: response.headers.get("content-encoding"),
  });
  return bytes;
}

async function fetchWithDiagnostics(url, label, options = undefined) {
  const resolvedUrl = new URL(url, location.href).href;
  try {
    return await fetch(resolvedUrl, options);
  } catch (error) {
    const requestOrigin = new URL(resolvedUrl).origin;
    const crossOrigin = requestOrigin !== location.origin;
    record("fetch-network-error", {
      label,
      url: resolvedUrl,
      pageOrigin: location.origin,
      requestOrigin,
      crossOrigin,
      hint: crossOrigin
        ? "No HTTP response was visible to JavaScript. For cross-origin asset URLs this usually means missing CORS headers, although TLS and network failures look the same."
        : "No HTTP response was visible to JavaScript. Check that the local harness server is running and that the file exists.",
      message: error instanceof Error ? error.message : String(error),
    });
    throw error;
  }
}

async function verifySha256(buffer, expected, name) {
  const actual = await sha256Hex(buffer);
  if (actual !== expected) {
    throw new Error(`${name} SHA-256 mismatch: expected ${expected}, got ${actual}`);
  }
}

async function sha256Hex(buffer) {
  const digest = await crypto.subtle.digest("SHA-256", buffer);
  return Array.from(new Uint8Array(digest))
    .map((byte) => byte.toString(16).padStart(2, "0"))
    .join("");
}

function float32Bytes(vector) {
  const bytes = new ArrayBuffer(vector.length * 4);
  const view = new DataView(bytes);
  for (let index = 0; index < vector.length; index += 1) {
    view.setFloat32(index * 4, vector[index], true);
  }
  return bytes;
}

function record(type, detail = {}) {
  const event = {
    index: state.events.length,
    type,
    elapsedMs: elapsed(state.startedAt),
    at: new Date().toISOString(),
    detail,
  };
  state.events.push(event);
  renderLog();
}

function renderLog() {
  dom.log.textContent = JSON.stringify({
    summary: {
      events: state.events.length,
      artifactUrl: dom["artifact-url"].value,
      vectorsUrl: dom["vectors-url"].value,
      onnxUrl: dom["onnx-url"].value,
      onnxQ4Url: dom["onnx-q4-url"].value,
      corpusId: dom["corpus-id"].value,
    },
    events: state.events,
  }, null, 2);
  dom.log.scrollTop = dom.log.scrollHeight;
}

function setStatus(text) {
  dom.status.textContent = text;
}

function setButtonsDisabled(disabled) {
  for (const id of ["env", "load", "embed", "goldens", "onnx", "onnx-goldens", "pack", "search", "repeat", "run-all"]) {
    dom[id].disabled = disabled;
  }
}

async function copyLog() {
  await navigator.clipboard.writeText(dom.log.textContent);
  setStatus("Copied log JSON.");
}

function downloadLog() {
  const blob = new Blob([dom.log.textContent], { type: "application/json" });
  const link = document.createElement("a");
  link.href = URL.createObjectURL(blob);
  link.download = `f2llm-webgpu-harness-${new Date().toISOString().replace(/[:.]/g, "-")}.json`;
  link.click();
  URL.revokeObjectURL(link.href);
}

async function memorySnapshot() {
  return {
    storage: await storageEstimate(),
    performanceMemory: performanceMemory(),
  };
}

async function storageEstimate() {
  if (!navigator.storage?.estimate) {
    return null;
  }
  return navigator.storage.estimate().catch((error) => ({
    error: error instanceof Error ? error.message : String(error),
  }));
}

function performanceMemory() {
  const memory = performance.memory;
  if (!memory) {
    return null;
  }
  return {
    jsHeapSizeLimit: memory.jsHeapSizeLimit,
    totalJSHeapSize: memory.totalJSHeapSize,
    usedJSHeapSize: memory.usedJSHeapSize,
  };
}

function userAgentData() {
  const data = navigator.userAgentData;
  if (!data) {
    return null;
  }
  return {
    mobile: data.mobile,
    platform: data.platform,
    brands: data.brands,
  };
}

function adapterLimits(limits) {
  if (!limits) {
    return null;
  }
  const names = [
    "maxBufferSize",
    "maxStorageBufferBindingSize",
    "maxComputeWorkgroupStorageSize",
    "maxComputeInvocationsPerWorkgroup",
    "maxComputeWorkgroupSizeX",
    "maxComputeWorkgroupSizeY",
    "maxComputeWorkgroupSizeZ",
    "maxComputeWorkgroupsPerDimension",
    "maxStorageBuffersPerShaderStage",
    "maxBindingsPerBindGroup",
  ];
  return Object.fromEntries(names.map((name) => [name, limits[name]]));
}

function requiredUrl(value, label) {
  const text = String(value || "").trim();
  if (text.length === 0) {
    throw new Error(`${label} is empty`);
  }
  return new URL(text, location.href).href;
}

function trimBaseUrl(value, label) {
  return requiredUrl(value, label).replace(/\/+$/, "");
}

function absoluteUrl(base, path) {
  return new URL(path, `${base.replace(/\/+$/, "")}/`).href;
}

function elapsed(start) {
  return Number((performance.now() - start).toFixed(3));
}

function formatMs(ms) {
  return `${ms.toFixed(1)} ms`;
}
