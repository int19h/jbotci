const F2LLM_80M_MODEL_KEY = "f2llm-v2-80m-q4-320";
const F2LLM_160M_MODEL_KEY = "f2llm-v2-160m-q4-640";
const F2LLM_330M_MODEL_KEY = "f2llm-v2-330m-q4-896";
const F2LLM_0_6B_MODEL_KEY = "f2llm-v2-0.6b-q4-1024";
const MI_B = 1024 * 1024;
const F2LLM_WEBGPU_RUNTIME = "jbotci-webgpu-f2llm";
const F2LLM_WEBGPU_RUNTIME_VERSION = "0.1.0";
const F2LLM_WASM_RUNTIME = "jbotci-onnxruntime-web-f2llm";
const F2LLM_WASM_RUNTIME_VERSION = "0.1.0";
const F2LLM_QUERY_PREFIX = "Instruct: Given a question, retrieve passages that can help answer the question.\nQuery: ";
const MODEL_CACHE_NAME = "jbotci-f2llm-models-v1";
const DEFAULT_ORT_MODULE_URL = new URL("./ort/ort.wasm.min.mjs", import.meta.url).href;
const DEFAULT_ORT_WASM_MJS_URL = new URL("./ort/ort-wasm-simd-threaded.mjs", import.meta.url).href;
const DEFAULT_ORT_WASM_URL = new URL("./ort/ort-wasm-simd-threaded.wasm", import.meta.url).href;
let f2llmRuntimeUrl = null;
let f2llmRuntimeModulePromise = null;
let ortModuleUrl = DEFAULT_ORT_MODULE_URL;
let ortWasmMjsUrl = DEFAULT_ORT_WASM_MJS_URL;
let ortWasmUrl = DEFAULT_ORT_WASM_URL;
let ortModulePromise = null;
const MODEL_SPECS = {
  [F2LLM_80M_MODEL_KEY]: {
    modelKey: F2LLM_80M_MODEL_KEY,
    label: "F2LLM v2 80M",
    modelId: "codefuse-ai/F2LLM-v2-80M",
    customRuntime: {
      runtime: F2LLM_WEBGPU_RUNTIME,
      version: F2LLM_WEBGPU_RUNTIME_VERSION,
      artifactBaseUrl: "https://assets.jbotci.app/models/f2llm-v2-80m-webgpu/v1",
      dtype: "q4",
      device: "webgpu",
    },
    wasmRuntime: {
      runtime: F2LLM_WASM_RUNTIME,
      version: F2LLM_WASM_RUNTIME_VERSION,
      onnxUrl: "https://assets.jbotci.app/models/f2llm-v2-80m-onnx-q4/v1/model_q4.onnx",
      dtype: "q4",
      device: "wasm",
    },
    preferredRuntime: { dtype: "q4", device: "webgpu" },
    dimensions: 320,
    maxSequenceLength: 512,
    queryPrefix: F2LLM_QUERY_PREFIX,
    remoteVectorPacks: true,
    browserLocalIndexing: true,
    localVectorSpaceKey: "jbotci-browser-f2llm-q4-f16",
    vectorElementType: "f16le",
    embedBatchSize: 1,
    modelSizeEstimates: {
      q4: 68 * MI_B,
    },
    minFreeBytesByDtype: {
      q4: 180 * MI_B,
    },
    outputPooling: "last_token",
  },
  [F2LLM_160M_MODEL_KEY]: {
    modelKey: F2LLM_160M_MODEL_KEY,
    label: "F2LLM v2 160M",
    modelId: "codefuse-ai/F2LLM-v2-160M",
    customRuntime: {
      runtime: F2LLM_WEBGPU_RUNTIME,
      version: F2LLM_WEBGPU_RUNTIME_VERSION,
      artifactBaseUrl: "https://assets.jbotci.app/models/f2llm-v2-160m-webgpu/v1",
      dtype: "q4",
      device: "webgpu",
    },
    preferredRuntime: { dtype: "q4", device: "webgpu" },
    dimensions: 640,
    maxSequenceLength: 512,
    queryPrefix: F2LLM_QUERY_PREFIX,
    remoteVectorPacks: true,
    browserLocalIndexing: true,
    localVectorSpaceKey: "jbotci-browser-f2llm-q4-f16",
    vectorElementType: "f16le",
    embedBatchSize: 1,
    modelSizeEstimates: { q4: 110 * MI_B },
    minFreeBytesByDtype: { q4: 260 * MI_B },
    outputPooling: "last_token",
  },
  [F2LLM_330M_MODEL_KEY]: {
    modelKey: F2LLM_330M_MODEL_KEY,
    label: "F2LLM v2 330M",
    modelId: "codefuse-ai/F2LLM-v2-330M",
    customRuntime: {
      runtime: F2LLM_WEBGPU_RUNTIME,
      version: F2LLM_WEBGPU_RUNTIME_VERSION,
      artifactBaseUrl: "https://assets.jbotci.app/models/f2llm-v2-330m-webgpu/v1",
      dtype: "q4",
      device: "webgpu",
    },
    preferredRuntime: { dtype: "q4", device: "webgpu" },
    dimensions: 896,
    maxSequenceLength: 512,
    queryPrefix: F2LLM_QUERY_PREFIX,
    remoteVectorPacks: true,
    browserLocalIndexing: true,
    localVectorSpaceKey: "jbotci-browser-f2llm-q4-f16",
    vectorElementType: "f16le",
    embedBatchSize: 1,
    modelSizeEstimates: { q4: 231 * MI_B },
    minFreeBytesByDtype: { q4: 420 * MI_B },
    outputPooling: "last_token",
  },
  [F2LLM_0_6B_MODEL_KEY]: {
    modelKey: F2LLM_0_6B_MODEL_KEY,
    label: "F2LLM v2 0.6B",
    modelId: "codefuse-ai/F2LLM-v2-0.6B",
    customRuntime: {
      runtime: F2LLM_WEBGPU_RUNTIME,
      version: F2LLM_WEBGPU_RUNTIME_VERSION,
      artifactBaseUrl: "https://assets.jbotci.app/models/f2llm-v2-0.6b-webgpu/v1",
      dtype: "q4",
      device: "webgpu",
    },
    preferredRuntime: { dtype: "q4", device: "webgpu" },
    dimensions: 1024,
    maxSequenceLength: 512,
    queryPrefix: F2LLM_QUERY_PREFIX,
    remoteVectorPacks: true,
    browserLocalIndexing: true,
    localVectorSpaceKey: "jbotci-browser-f2llm-q4-f16",
    vectorElementType: "f16le",
    embedBatchSize: 1,
    modelSizeEstimates: { q4: 416 * MI_B },
    minFreeBytesByDtype: { q4: 700 * MI_B },
    outputPooling: "last_token",
  },
};
const DB_NAME = "jbotci-embeddings-v1";
const META_STORE = "meta";
const BLOB_STORE = "blobs";
const DEFAULT_REMOTE_BASE_URL = "/assets/embeddings/web/v1";
const LOG_PREFIX = "[jbotci embeddings worker]";
const ACTIVE_SETUP_STATUSES = new Set([
  "checking",
  "downloading-index",
  "downloading-model",
  "indexing",
  "loading-model",
]);

let selectedModelKey = F2LLM_330M_MODEL_KEY;
let activeModelKey = F2LLM_330M_MODEL_KEY;
let activeRuntimeMode = "webgpu";
let lastWebGpuAvailable = null;
let modelLoadPromise = null;
let modelRuntime = null;
let dbPromise = null;
let setupInProgress = false;
const vectorCache = new Map();

function logInfo(message, detail = null) {
  if (detail === null) {
    console.info(`${LOG_PREFIX} ${message}`);
  } else {
    console.info(`${LOG_PREFIX} ${message}`, detail);
  }
}

function logWarn(message, detail = null) {
  if (detail === null) {
    console.warn(`${LOG_PREFIX} ${message}`);
  } else {
    console.warn(`${LOG_PREFIX} ${message}`, detail);
  }
}

function logError(message, detail = null) {
  if (detail === null) {
    console.error(`${LOG_PREFIX} ${message}`);
  } else {
    console.error(`${LOG_PREFIX} ${message}`, detail);
  }
}

self.onmessage = async (event) => {
  const { id, type, payload } = event.data || {};
  const forceWasm = payload?.forceWasm === true;
  try {
    setF2LlmRuntimeUrl(payload?.f2llmRuntimeUrl);
    setOrtAssets(payload?.ortModuleUrl, payload?.ortWasmMjsUrl, payload?.ortWasmUrl);
    setSelectedModel(payload?.modelKey);
    await resolveActiveModel(forceWasm);
    let value;
    if (type === "status") {
      value = await status();
    } else if (type === "setup") {
      value = await setup(
        payload?.corpusJson || "{}",
        normalizeRemoteBaseUrl(payload?.remoteBaseUrl),
        forceWasm,
      );
    } else if (type === "remove") {
      value = await removeSelectedModel();
    } else if (type === "search") {
      value = await search(
        payload?.corpusId,
        payload?.query,
        payload?.limit || 0,
        payload?.kindFiltersJson || "[]",
        forceWasm,
      );
    } else {
      throw new Error(`unknown embedding worker request: ${type}`);
    }
    self.postMessage({ id, ok: true, value });
  } catch (error) {
    logError("request failed", {
      type,
      error: errorMessage(error),
    });
    self.postMessage({
      id,
      ok: false,
      error: error instanceof Error ? error.message : String(error),
      retryWithWasm: false,
    });
  }
};

function activeModelSpec() {
  return MODEL_SPECS[activeModelKey];
}

function modelSpecForKey(modelKey) {
  const key = typeof modelKey === "string" && modelKey.trim().length > 0
    ? modelKey.trim()
    : F2LLM_330M_MODEL_KEY;
  const spec = MODEL_SPECS[key];
  if (!spec) {
    throw new Error(`unsupported browser embedding model key: ${key}`);
  }
  return spec;
}

function setSelectedModel(modelKey) {
  const spec = modelSpecForKey(modelKey);
  if (spec.modelKey === selectedModelKey) {
    return spec;
  }
  if (setupInProgress) {
    throw new Error("cannot change browser embedding model while setup is active");
  }
  selectedModelKey = spec.modelKey;
  logInfo("selected embedding model changed", {
    modelKey: spec.modelKey,
    label: spec.label,
  });
  return spec;
}

async function resolveActiveModel(forceWasm = false) {
  const selected = modelSpecForKey(selectedModelKey);
  const webGpuAvailable = !forceWasm && await hasUsableWebGpu();
  lastWebGpuAvailable = webGpuAvailable;
  const effective = webGpuAvailable ? selected : MODEL_SPECS[F2LLM_80M_MODEL_KEY];
  const runtimeMode = webGpuAvailable ? "webgpu" : "wasm";
  if (effective.modelKey === activeModelKey && runtimeMode === activeRuntimeMode) {
    return effective;
  }
  if (setupInProgress) {
    throw new Error("cannot change browser embedding runtime while setup is active");
  }
  activeModelKey = effective.modelKey;
  activeRuntimeMode = runtimeMode;
  modelLoadPromise = null;
  modelRuntime = null;
  vectorCache.clear();
  logInfo("active embedding model resolved", {
    selectedModelKey: selected.modelKey,
    activeModelKey,
    runtimeMode,
    webGpuAvailable,
  });
  return effective;
}

function setF2LlmRuntimeUrl(runtimeUrl) {
  if (typeof runtimeUrl !== "string" || runtimeUrl.trim().length === 0) {
    return;
  }
  const nextUrl = runtimeUrl.trim();
  if (f2llmRuntimeUrl === nextUrl) {
    return;
  }
  if (setupInProgress) {
    throw new Error("cannot change F2LLM WebGPU runtime URL while setup is active");
  }
  f2llmRuntimeUrl = nextUrl;
  f2llmRuntimeModulePromise = null;
  modelLoadPromise = null;
  modelRuntime = null;
  vectorCache.clear();
  logInfo("configured F2LLM WebGPU runtime module", { runtimeUrl: f2llmRuntimeUrl });
}

function setOrtAssets(moduleUrl, wasmMjsUrl, wasmUrl) {
  if (typeof moduleUrl !== "string" || moduleUrl.trim().length === 0) {
    return;
  }
  if (typeof wasmMjsUrl !== "string" || wasmMjsUrl.trim().length === 0) {
    return;
  }
  if (typeof wasmUrl !== "string" || wasmUrl.trim().length === 0) {
    return;
  }
  const nextModuleUrl = new URL(moduleUrl, globalThis.location.href).href;
  const nextWasmMjsUrl = new URL(wasmMjsUrl, globalThis.location.href).href;
  const nextWasmUrl = new URL(wasmUrl, globalThis.location.href).href;
  if (
    ortModuleUrl === nextModuleUrl
    && ortWasmMjsUrl === nextWasmMjsUrl
    && ortWasmUrl === nextWasmUrl
  ) {
    return;
  }
  if (setupInProgress) {
    throw new Error("cannot change ONNX Runtime Web assets while setup is active");
  }
  ortModuleUrl = nextModuleUrl;
  ortWasmMjsUrl = nextWasmMjsUrl;
  ortWasmUrl = nextWasmUrl;
  ortModulePromise = null;
  if (activeRuntimeMode === "wasm") {
    modelLoadPromise = null;
    modelRuntime = null;
  }
  logInfo("configured ONNX Runtime Web assets", {
    ortModuleUrl,
    ortWasmMjsUrl,
    ortWasmUrl,
  });
}

function normalizeRemoteBaseUrl(remoteBaseUrl) {
  if (typeof remoteBaseUrl !== "string" || remoteBaseUrl.trim().length === 0) {
    return DEFAULT_REMOTE_BASE_URL;
  }
  const trimmed = remoteBaseUrl.trim();
  return trimmed === "/" ? "" : trimmed.replace(/\/+$/, "");
}

function remotePackUrl(remoteBaseUrl, path) {
  return `${remoteBaseUrl}/${path.replace(/^\/+/, "")}`;
}

function corpusSummary(corpus) {
  return {
    modelKey: corpus?.modelKey || null,
    sourceModelKey: corpus?.sourceModelKey || null,
    inputFormatVersion: corpus?.inputFormatVersion || null,
    inputHash: shortHash(corpus?.inputHash),
    dictionaryHash: shortHash(corpus?.dictionaryHash),
    cllHash: shortHash(corpus?.cllHash),
    dictionaryRows: Array.isArray(corpus?.dictionary) ? corpus.dictionary.length : null,
    cllRows: Array.isArray(corpus?.cll) ? corpus.cll.length : null,
  };
}

function packSummary(pack) {
  if (!pack) {
    return null;
  }
  return {
    source: pack.source || null,
    packId: pack.packId || null,
    modelKey: pack.modelKey || null,
    inputHash: shortHash(pack.inputHash),
    vectorSpaceKey: pack.vectorSpaceKey || null,
    corpora: Object.fromEntries(Object.entries(pack.corpora || {}).map(([id, corpus]) => [
      id,
      {
        inputHash: shortHash(corpus?.inputHash),
        rowCount: corpus?.rowCount || null,
        dimensions: corpus?.dimensions || null,
      },
    ])),
  };
}

async function setup(corpusJson, remoteBaseUrl, forceWasm) {
  const spec = activeModelSpec();
  if (setupInProgress) {
    logInfo("setup request ignored because setup is already active");
    return status();
  }
  setupInProgress = true;
  try {
    const corpus = normalizeCorpus(JSON.parse(corpusJson));
    logInfo("setup started", {
      modelKey: spec.modelKey,
      modelLabel: spec.label,
      remoteBaseUrl,
      corpus: corpusSummary(corpus),
    });
    await requestPersistentStorage();
    await checkQuota(forceWasm);
    await updateStatus("loading-model", `Downloading or opening ${spec.label}.`);
    await ensureModel(forceWasm);
    logInfo("query model ready", { runtime: activeQueryRuntime() });
    await checkQuota(forceWasm);
    await updateStatus("checking", "Looking for a vector pack.");
    const remoteAttempt = await loadRemotePackIfAvailable(corpus, remoteBaseUrl);
    if (!remoteAttempt.loaded) {
      if (spec.browserLocalIndexing === false) {
        throw new Error(
          `${spec.label} requires a compatible prebuilt remote vector pack; ${remoteAttempt.reason}.`,
        );
      }
      logWarn("remote vector pack unavailable; falling back to browser-local indexing", {
        reason: remoteAttempt.reason,
        detail: remoteAttempt.detail || null,
      });
      await buildLocalPack(corpus);
    }
    const pack = await getModelMeta("pack");
    logInfo("setup finished", {
      pack: packSummary(pack),
    });
    await updateStatus("ready", pack?.source === "remote"
      ? "Using cached vector pack with local query embeddings."
      : "Using a browser-built vector pack with local query embeddings.");
    return status();
  } catch (error) {
    await updateStatus("error", `Embedding setup failed: ${errorMessage(error)}`);
    throw error;
  } finally {
    setupInProgress = false;
  }
}

async function status() {
  const spec = activeModelSpec();
  const meta = activeStatusMeta(await getModelMeta("status"));
  const pack = activeModelPack(await getModelMeta("pack"));
  const storedModelRuntime = activeStoredModelRuntime(modelRuntime || await getModelMeta("modelRuntime"));
  const indexBytes = await packIndexBytes(pack);
  const display = statusDisplay(meta, pack);
  if (display.rewriteStoredStatus) {
    await updateStatus(display.status, display.detail, display.progress);
  }
  return {
    status: display.status,
    detail: display.detail,
    modelBytes: modelBytesForRuntime(storedModelRuntime || spec.preferredRuntime),
    indexBytes,
    modelKey: spec.modelKey,
    modelLabel: spec.label,
    selectedModelKey,
    selectedModelLabel: MODEL_SPECS[selectedModelKey]?.label || null,
    effectiveModelKey: spec.modelKey,
    effectiveRuntimeMode: activeRuntimeMode,
    webGpuAvailable: lastWebGpuAvailable,
    modelDtype: storedModelRuntime?.dtype || null,
    modelDevice: storedModelRuntime?.device || null,
    packId: pack?.packId || null,
    vectorSpaceKey: pack?.vectorSpaceKey || null,
    source: pack?.source || null,
    progress: display.progress,
  };
}

function statusDisplay(meta, pack) {
  if (!setupInProgress && ACTIVE_SETUP_STATUSES.has(meta?.status)) {
    if (pack) {
      return {
        status: "ready",
        detail: "Embedding index is cached in this browser.",
        progress: null,
        rewriteStoredStatus: true,
      };
    }
    return {
      status: "not-installed",
      detail: "Previous embedding setup was interrupted. Click Download to restart indexing; cached model files will be reused.",
      progress: null,
      rewriteStoredStatus: true,
    };
  }
  if (!pack && meta?.status === "ready") {
    return {
      status: "not-installed",
      detail: "No browser embedding index is installed for the selected model.",
      progress: null,
      rewriteStoredStatus: true,
    };
  }
  return {
    status: pack ? (meta?.status || "ready") : (meta?.status === "error" ? "error" : "not-installed"),
    detail: meta?.detail || (pack
      ? "Embedding index is cached in this browser."
      : "No browser embedding index is installed."),
    progress: meta?.progress || null,
    rewriteStoredStatus: false,
  };
}

async function removeSelectedModel() {
  const spec = activeModelSpec();
  vectorCache.clear();
  modelLoadPromise = null;
  modelRuntime = null;
  const db = await openDb();
  await transaction(db, [META_STORE, BLOB_STORE], "readwrite", (tx) => {
    const meta = tx.objectStore(META_STORE);
    meta.delete(modelMetaKey("status", spec.modelKey));
    meta.delete(modelMetaKey("pack", spec.modelKey));
    meta.delete(modelMetaKey("modelRuntime", spec.modelKey));
    const blobs = tx.objectStore(BLOB_STORE);
    const prefixes = modelBlobPrefixes(spec.modelKey);
    const cursor = blobs.openKeyCursor();
    cursor.onsuccess = () => {
      const current = cursor.result;
      if (!current) {
        return;
      }
      const key = String(current.key || "");
      if (prefixes.some((prefix) => key.startsWith(prefix))) {
        blobs.delete(current.key);
      }
      current.continue();
    };
  });
  await removeBlobMetaForModel(spec.modelKey);
  if (navigator.storage?.getDirectory) {
    const root = await navigator.storage.getDirectory();
    for (const prefix of modelBlobPrefixes(spec.modelKey)) {
      await removeOpfsDirectory(root, ["jbotci", "embeddings", ...prefix.split("/")]).catch(() => {});
    }
  }
  const cacheRemoved = await removeCachedModelArtifacts(spec);
  await updateStatus(
    "not-installed",
    cacheRemoved
      ? `${spec.label} model files and vector index were removed.`
      : `${spec.label} vector index was removed. Model file cache cleanup was not available in this browser.`,
  );
  return status();
}

async function search(corpusId, query, limit, kindFiltersJson, forceWasm) {
  const trimmedQuery = String(query || "").trim();
  if (!trimmedQuery) {
    return { hits: [], message: null };
  }
  const kindFilters = parseStringArray(kindFiltersJson)
    .map(normalizeWordTypeFilter)
    .filter((value) => value.length > 0);
  const pack = activeModelPack(await getModelMeta("pack"));
  if (!pack) {
    return {
      hits: [],
      message: "Download model and embeddings to use semantic search",
    };
  }
  const corpus = pack.corpora?.[corpusId];
  if (!corpus) {
    return {
      hits: [],
      message: `The cached embedding pack does not contain ${corpusId}.`,
    };
  }
  if (kindFilters.length > 0 && !corpusSupportsKindFilters(corpus)) {
    return {
      hits: [],
      message: "The cached embedding pack does not include word-type metadata. Remove and download embeddings again.",
    };
  }
  const storedRuntime = activeStoredModelRuntime(modelRuntime || await getModelMeta("modelRuntime"));
  if (storedRuntime && !packCompatibleWithRuntime(pack, queryRuntimeFromModelRuntime(storedRuntime))) {
    return {
      hits: [],
      message: "The cached embedding pack was built for a different browser embedding runtime. Open Settings and update embeddings.",
    };
  }
  await ensureModel(forceWasm);
  const runtime = activeQueryRuntime();
  if (!packCompatibleWithRuntime(pack, runtime)) {
    return {
      hits: [],
      message: "The cached embedding pack was built for a different browser embedding runtime. Open Settings and update embeddings.",
    };
  }
  const queryEmbedding = await embedTexts([activeModelSpec().queryPrefix + trimmedQuery]);
  if (isCustomWebGpuRuntime(runtime)) {
    const loadedRuntime = await ensureModel(forceWasm);
    const hits = await loadedRuntime.rankHits({
      corpus,
      query: queryEmbedding[0],
      limit,
      itemMatches: (item) => itemMatchesKindFilters(item, kindFilters),
      readBinary: getBinary,
    });
    return { hits, message: hits.length === 0 ? "No matches found." : null };
  }
  const vectors = await readCorpusVectors(corpus);
  const hits = rankHits(vectors, queryEmbedding[0], corpus.items, corpus.dimensions, limit, kindFilters);
  return { hits, message: hits.length === 0 ? "No matches found." : null };
}

async function ensureModel(forceWasm = false) {
  if (modelLoadPromise === null) {
    modelLoadPromise = loadTokenizerAndModel(forceWasm);
  }
  return modelLoadPromise;
}

async function loadTokenizerAndModel(forceWasm = false) {
  const spec = activeModelSpec();
  if (activeRuntimeMode === "wasm") {
    return loadOnnxWasmRuntime(spec);
  }
  return loadCustomRuntime(spec, forceWasm);
}

async function loadCustomRuntime(spec, forceWasm = false) {
  if (forceWasm) {
    throw new Error(`${spec.label} does not provide a CPU/WASM fallback runtime.`);
  }
  const runtime = spec.customRuntime;
  if (runtime.device !== "webgpu") {
    throw new Error(`${spec.label} custom runtime has unsupported device: ${runtime.device}`);
  }
  if (!await hasUsableWebGpu()) {
    throw new Error(`${spec.label} ${runtime.dtype}/webgpu requires WebGPU, but no WebGPU adapter is available.`);
  }
  await updateStatus(
    "loading-model",
    `Opening ${spec.label} ${runtime.dtype} with ${runtime.device}.`,
    indeterminateProgress("model", `${spec.label} ${runtime.dtype}/${runtime.device}`),
  );
  const { F2LlmWebGpuRuntime } = await f2llmRuntimeModule();
  const loaded = await F2LlmWebGpuRuntime.load({
    baseUrl: runtime.artifactBaseUrl,
    expectedModelKey: spec.modelKey,
    expectedRuntime: runtime.runtime,
    expectedVersion: runtime.version,
    maxSequenceLength: spec.maxSequenceLength,
    dimensions: spec.dimensions,
    fetchArrayBuffer: cachedFetchArrayBufferForSpec(spec),
    progress: async (progress) => {
      await updateStatus(
        progress.status || "downloading-model",
        progress.detail || `Downloading ${spec.label} WebGPU artifact.`,
        progress.progress || null,
      );
    },
  });
  modelRuntime = {
    modelKey: spec.modelKey,
    runtime: runtime.runtime,
    version: runtime.version,
    dtype: runtime.dtype,
    device: runtime.device,
  };
  await putModelMeta("modelRuntime", modelRuntime);
  return loaded;
}

async function loadOnnxWasmRuntime(spec) {
  if (!spec.wasmRuntime) {
    throw new Error(`${spec.label} does not provide a CPU/WASM fallback runtime.`);
  }
  const runtime = spec.wasmRuntime;
  await updateStatus(
    "loading-model",
    `Opening ${spec.label} ${runtime.dtype} with ${runtime.device}.`,
    indeterminateProgress("model", `${spec.label} ${runtime.dtype}/${runtime.device}`),
  );
  const [{ QwenByteBpeTokenizer }, ort] = await Promise.all([
    f2llmRuntimeModule(),
    ortModule(),
  ]);
  const tokenizer = await loadF2LlmTokenizer(spec, QwenByteBpeTokenizer);
  const onnxBytes = await cachedFetchArrayBufferForSpec(spec)(runtime.onnxUrl, `${spec.label} ONNX q4 model`);
  const session = await ort.InferenceSession.create(new Uint8Array(onnxBytes), {
    executionProviders: ["wasm"],
    graphOptimizationLevel: "all",
  });
  const loaded = new F2LlmOnnxWasmRuntime({
    ort,
    session,
    tokenizer,
    dimensions: spec.dimensions,
    maxSequenceLength: spec.maxSequenceLength,
  });
  modelRuntime = {
    modelKey: spec.modelKey,
    runtime: runtime.runtime,
    version: runtime.version,
    dtype: runtime.dtype,
    device: runtime.device,
  };
  await putModelMeta("modelRuntime", modelRuntime);
  await updateStatus(
    "loading-model",
    `${spec.label} ${runtime.dtype}/${runtime.device} is ready.`,
    progressValue("model", `${spec.label} ${runtime.dtype}/${runtime.device}`, 1, 1),
  );
  return loaded;
}

async function f2llmRuntimeModule() {
  if (typeof f2llmRuntimeUrl !== "string" || f2llmRuntimeUrl.length === 0) {
    throw new Error("F2LLM WebGPU runtime module URL is not configured.");
  }
  if (f2llmRuntimeModulePromise === null) {
    f2llmRuntimeModulePromise = import(f2llmRuntimeUrl);
  }
  return f2llmRuntimeModulePromise;
}

async function ortModule() {
  if (ortModulePromise === null) {
    ortModulePromise = import(ortModuleUrl).then((ort) => {
      ort.env.wasm.wasmPaths = {
        mjs: ortWasmMjsUrl,
        wasm: ortWasmUrl,
      };
      ort.env.wasm.numThreads = 1;
      ort.env.wasm.proxy = false;
      return ort;
    });
  }
  return ortModulePromise;
}

class F2LlmOnnxWasmRuntime {
  constructor({ ort, session, tokenizer, dimensions, maxSequenceLength }) {
    this.ort = ort;
    this.session = session;
    this.tokenizer = tokenizer;
    this.dimensions = dimensions;
    this.maxSequenceLength = maxSequenceLength;
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
    const feeds = this.feeds(tokenIds);
    const outputs = await this.session.run(feeds);
    const outputName = selectOnnxOutputName(outputs);
    const vector = pooledOnnxVector(outputs[outputName], tokenIds.length, this.dimensions);
    normalize(vector);
    return vector;
  }

  feeds(tokenIds) {
    const feeds = {};
    for (const inputName of this.session.inputNames) {
      if (inputName === "input_ids") {
        feeds[inputName] = new this.ort.Tensor(
          "int64",
          BigInt64Array.from(tokenIds.map((token) => BigInt(token))),
          [1, tokenIds.length],
        );
      } else if (inputName === "attention_mask") {
        const attentionMask = new BigInt64Array(tokenIds.length);
        attentionMask.fill(1n);
        feeds[inputName] = new this.ort.Tensor("int64", attentionMask, [1, tokenIds.length]);
      } else if (inputName === "position_ids") {
        feeds[inputName] = new this.ort.Tensor(
          "int64",
          BigInt64Array.from(tokenIds.map((_, index) => BigInt(index))),
          [1, tokenIds.length],
        );
      } else {
        throw new Error(`unsupported F2LLM ONNX input: ${inputName}`);
      }
    }
    return feeds;
  }
}

async function loadF2LlmTokenizer(spec, QwenByteBpeTokenizer) {
  const fetchModel = cachedFetchArrayBufferForSpec(spec);
  const manifest = await fetchJsonWith(fetchModel, `${spec.customRuntime.artifactBaseUrl}/manifest.json`, `${spec.label} WebGPU manifest`);
  const tokenizerSpec = manifest.tokenizer;
  const bytes = await fetchModel(
    `${spec.customRuntime.artifactBaseUrl}/${tokenizerSpec.url}`,
    `${spec.label} tokenizer`,
  );
  await verifySha256(
    bytes,
    tokenizerSpec.canonical_json_sha256,
    `${spec.label} tokenizer canonical JSON`,
  );
  const tokenizer = parseJsonBytes(bytes, `${spec.label} tokenizer`);
  if (tokenizer.schema_version !== 1) {
    throw new Error(`unsupported F2LLM tokenizer schema version: ${tokenizer.schema_version}`);
  }
  return new QwenByteBpeTokenizer({
    vocab: tokenizer.vocab,
    merges: tokenizer.merges,
    eosId: tokenizer.special_tokens?.eos_id,
  });
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

function pooledOnnxVector(output, tokenCount, dimensions) {
  const dims = Array.from(output?.dims || []);
  const data = output?.data;
  if (!data) {
    throw new Error("F2LLM ONNX output is missing tensor data");
  }
  if (dims.length === 2 && dims[0] === 1 && dims[1] === dimensions) {
    return Float32Array.from(data);
  }
  if (dims.length === 3 && dims[0] === 1 && dims[1] >= tokenCount && dims[2] === dimensions) {
    const start = (tokenCount - 1) * dimensions;
    return Float32Array.from(data.slice(start, start + dimensions));
  }
  throw new Error(`unsupported F2LLM ONNX output shape: ${dims.join("x")}`);
}

async function hasUsableWebGpu() {
  if (!navigator.gpu?.requestAdapter) {
    return false;
  }
  try {
    const adapter = await navigator.gpu.requestAdapter();
    return adapter !== null;
  } catch (_) {
    return false;
  }
}

function errorMessage(error) {
  return error instanceof Error ? error.message : String(error);
}

function normalizeCorpus(raw) {
  const spec = activeModelSpec();
  const corpus = {
    modelKey: raw?.modelKey || raw?.model_key || raw?.["model-key"] || "",
    sourceModelKey: raw?.modelKey || raw?.model_key || raw?.["model-key"] || "",
    modelRevision: raw?.modelRevision || raw?.model_revision || "",
    inputFormatVersion: raw?.inputFormatVersion || raw?.input_format_version || "",
    inputHash: raw?.inputHash || raw?.input_hash || "",
    dictionaryHash: raw?.dictionaryHash || raw?.dictionary_hash || "",
    cllHash: raw?.cllHash || raw?.cll_hash || "",
    dictionary: normalizeInputDocuments(raw?.dictionary || [], "dictionary"),
    cll: normalizeInputDocuments(raw?.cll || [], "cll"),
  };
  if (corpus.modelKey !== spec.modelKey) {
    logInfo("using exported embedding corpus text with selected browser model", {
      corpusModelKey: corpus.modelKey,
      selectedModelKey: spec.modelKey,
    });
    corpus.modelKey = spec.modelKey;
  }
  for (const [name, value] of [
    ["modelRevision", corpus.modelRevision],
    ["inputFormatVersion", corpus.inputFormatVersion],
    ["inputHash", corpus.inputHash],
    ["dictionaryHash", corpus.dictionaryHash],
    ["cllHash", corpus.cllHash],
  ]) {
    if (typeof value !== "string" || value.length === 0) {
      throw new Error(`embedding corpus is missing ${name}`);
    }
  }
  return corpus;
}

function normalizeInputDocuments(docs, label) {
  if (!Array.isArray(docs)) {
    throw new Error(`embedding corpus ${label} documents must be an array`);
  }
  return docs.map((doc, row) => {
    const id = Number(doc?.id);
    const input = String(doc?.input || "");
    const inputHash = doc?.inputHash || doc?.input_hash || "";
    if (!Number.isInteger(id) || id < 0) {
      throw new Error(`embedding corpus ${label} row ${row} has an invalid id`);
    }
    if (input.length === 0) {
      throw new Error(`embedding corpus ${label} row ${row} is missing input text`);
    }
    if (typeof inputHash !== "string" || inputHash.length === 0) {
      throw new Error(`embedding corpus ${label} row ${row} is missing inputHash`);
    }
    return {
      id,
      input,
      inputHash,
      kind: typeof doc?.kind === "string" ? doc.kind : null,
    };
  });
}

function normalizeRemoteItems(items, corpusId) {
  return items.map((item, row) => {
    const id = Number(item.entry_index ?? item.chunk_index ?? item.id);
    const inputHash = item.input_hash || item.inputHash || "";
    if (!Number.isInteger(id) || id < 0) {
      throw new Error(`remote ${corpusId} row ${row} has an invalid id`);
    }
    if (typeof inputHash !== "string" || inputHash.length === 0) {
      throw new Error(`remote ${corpusId} row ${row} is missing input_hash`);
    }
    return {
      id,
      row,
      kind: normalizeWordTypeFilter(item.kind || "") || null,
      inputHash,
    };
  });
}

async function embedTexts(texts, progressContext = null) {
  const spec = activeModelSpec();
  const loaded = await ensureModel();
  if (typeof loaded?.embedTexts !== "function") {
    throw new Error(`${spec.label} runtime does not provide embedTexts`);
  }
  const output = [];
  if (progressContext !== null) {
    await updateStatus(
      "indexing",
      `Embedding ${progressContext.label}: 0 of ${texts.length} rows.`,
      progressValue("index", progressContext.label, 0, texts.length),
    );
  }
  for (let start = 0; start < texts.length; start += spec.embedBatchSize) {
    const batch = texts.slice(start, start + spec.embedBatchSize);
    const rows = await loaded.embedTexts(batch);
    for (const vector of rows) {
      if (vector.length !== spec.dimensions) {
        throw new Error(
          `${spec.label} embedding dimension mismatch: expected ${spec.dimensions}, got ${vector.length}`,
        );
      }
      normalize(vector);
      output.push(vector);
    }
    if (progressContext !== null) {
      const done = Math.min(start + batch.length, texts.length);
      await updateStatus(
        "indexing",
        `Embedding ${progressContext.label}: ${done} of ${texts.length} rows.`,
        progressValue("index", progressContext.label, done, texts.length),
      );
    }
  }
  return output;
}

async function buildLocalPack(corpus) {
  const spec = activeModelSpec();
  if (corpus.modelKey !== spec.modelKey) {
    throw new Error(`unsupported browser corpus model key: ${corpus.modelKey || "missing"}`);
  }
  const runtime = activeQueryRuntime();
  const elementType = localVectorElementType(spec);
  const vectorSpaceKey = spec.localVectorSpaceKey || `jbotci-browser-f2llm-${runtime.dtype}-${elementType}`;
  const packId = `${vectorSpaceKey}-${shortHash(corpus.inputHash)}`;
  const existing = activeModelPack(await getModelMeta("pack"));
  if (cachedPackMatchesCorpus(existing, corpus, runtime, vectorSpaceKey)) {
    logInfo("existing browser-local vector pack is already current", {
      pack: packSummary(existing),
    });
    return;
  }
  logInfo("building browser-local vector pack", {
    packId,
    runtime,
    vectorSpaceKey,
    corpus: corpusSummary(corpus),
    reusablePack: packSummary(packCompatibleWithRuntime(existing, runtime) ? existing : null),
  });
  vectorCache.clear();
  const reusablePack = packCompatibleWithRuntime(existing, runtime) ? existing : null;
  const corpora = {};
  corpora["vlacku-en"] = await buildLocalCorpus(
    "vlacku-en",
    corpus.dictionary,
    corpus.dictionaryHash,
    packId,
    elementType,
    reusablePack?.corpora?.["vlacku-en"] || null,
  );
  corpora["cukta-cll"] = await buildLocalCorpus(
    "cukta-cll",
    corpus.cll,
    corpus.cllHash,
    packId,
    elementType,
    reusablePack?.corpora?.["cukta-cll"] || null,
  );
  await putModelMeta("pack", {
    source: "browser",
    packId,
    modelKey: spec.modelKey,
    inputHash: corpus.inputHash,
    inputFormatVersion: corpus.inputFormatVersion,
    vectorSpaceKey,
    runtime,
    compatibleQueryRuntimes: [runtime],
    corpora,
  });
  logInfo("browser-local vector pack ready", {
    packId,
    vectorSpaceKey,
    corpusIds: Object.keys(corpora),
  });
  vectorCache.clear();
}

async function buildLocalCorpus(corpusId, docs, inputHash, packId, elementType, reusableCorpus) {
  const spec = activeModelSpec();
  await updateStatus(
    "indexing",
    `Preparing ${corpusId} embeddings in this browser.`,
    progressValue("index", corpusId, 0, docs.length),
  );
  const reusableRows = await reusableRowsByInputHash(reusableCorpus);
  logInfo("building browser-local corpus", {
    corpusId,
    rows: docs.length,
    inputHash: shortHash(inputHash),
    reusableRows: reusableRows.size,
  });
  const vectors = createLocalVectorStore(docs.length, spec.dimensions, elementType);
  const pendingDocs = [];
  const pendingRows = [];
  let reused = 0;
  for (let row = 0; row < docs.length; row += 1) {
    const doc = docs[row];
    const reusedVector = reusableRows.get(doc.inputHash);
    if (reusedVector) {
      writeLocalVector(vectors, row, reusedVector, spec.dimensions, elementType);
      reused += 1;
    } else {
      pendingDocs.push(doc);
      pendingRows.push(row);
    }
  }
  await updateStatus(
    "indexing",
    `Embedding ${corpusId}: ${reused} rows reused, ${pendingDocs.length} rows to compute.`,
    progressValue("index", corpusId, reused, docs.length),
  );
  if (pendingDocs.length > 0) {
    await embedLocalRows(
      pendingDocs,
      pendingRows,
      vectors,
      elementType,
      {
        label: corpusId,
        completedRows: reused,
        totalRows: docs.length,
      },
    );
  }
  const vectorKey = `local/${spec.modelKey}/${packId}/${corpusId}/vectors.${localVectorFileExtension(elementType)}`;
  const vectorBuffer = localVectorStoreBuffer(vectors);
  await putBinary(vectorKey, vectorBuffer);
  await updateStatus(
    "indexing",
    `Embedded ${corpusId}: ${docs.length} of ${docs.length} rows.`,
    progressValue("index", corpusId, docs.length, docs.length),
  );
  return {
    corpusId,
    inputHash,
    rowCount: docs.length,
    dimensions: spec.dimensions,
    elementType,
    items: docs.map((doc, row) => ({
      id: doc.id,
      row,
      kind: normalizeWordTypeFilter(doc.kind || "") || null,
      inputHash: doc.inputHash,
    })),
    shards: [{ key: vectorKey, byteLen: vectorBuffer.byteLength }],
  };
}

async function embedLocalRows(docs, rows, vectors, elementType, progressContext) {
  const spec = activeModelSpec();
  const loaded = await ensureModel();
  if (typeof loaded?.embedTexts !== "function") {
    throw new Error(`${spec.label} runtime does not provide embedTexts`);
  }
  for (let start = 0; start < docs.length; start += spec.embedBatchSize) {
    const batchDocs = docs.slice(start, start + spec.embedBatchSize);
    const batchVectors = await loaded.embedTexts(batchDocs.map((doc) => doc.input));
    if (batchVectors.length !== batchDocs.length) {
      throw new Error(`${spec.label} runtime returned the wrong embedding row count`);
    }
    for (let index = 0; index < batchVectors.length; index += 1) {
      const vector = batchVectors[index];
      if (vector.length !== spec.dimensions) {
        throw new Error(
          `${spec.label} embedding dimension mismatch: expected ${spec.dimensions}, got ${vector.length}`,
        );
      }
      normalize(vector);
      writeLocalVector(vectors, rows[start + index], vector, spec.dimensions, elementType);
    }
    const done = progressContext.completedRows + Math.min(start + batchDocs.length, docs.length);
    await updateStatus(
      "indexing",
      `Embedding ${progressContext.label}: ${done} of ${progressContext.totalRows} rows.`,
      progressValue("index", progressContext.label, done, progressContext.totalRows),
    );
  }
}

function localVectorElementType(spec) {
  return spec.localVectorElementType || spec.vectorElementType || "f32le";
}

function createLocalVectorStore(rowCount, dimensions, elementType) {
  const elementCount = rowCount * dimensions;
  if (elementType === "f16le") {
    return new DataView(new ArrayBuffer(elementCount * 2));
  }
  if (elementType === "f32le") {
    return new Float32Array(elementCount);
  }
  throw new Error(`unsupported browser-local vector element type: ${elementType}`);
}

function writeLocalVector(store, row, vector, dimensions, elementType) {
  const base = row * dimensions;
  if (elementType === "f16le") {
    for (let dim = 0; dim < dimensions; dim += 1) {
      store.setUint16((base + dim) * 2, f32ToF16Bits(vector[dim]), true);
    }
    return;
  }
  if (elementType === "f32le") {
    store.set(vector, base);
    return;
  }
  throw new Error(`unsupported browser-local vector element type: ${elementType}`);
}

function localVectorStoreBuffer(store) {
  if (store instanceof DataView) {
    return store.buffer;
  }
  return store.buffer.slice(store.byteOffset, store.byteOffset + store.byteLength);
}

function localVectorFileExtension(elementType) {
  if (elementType === "f16le") {
    return "f16";
  }
  if (elementType === "f32le") {
    return "f32";
  }
  throw new Error(`unsupported browser-local vector element type: ${elementType}`);
}

async function reusableRowsByInputHash(corpus) {
  const spec = activeModelSpec();
  const rows = new Map();
  if (!corpus || corpus.dimensions !== spec.dimensions || !Array.isArray(corpus.items)) {
    return rows;
  }
  const vectors = await readCorpusVectors(corpus).catch(() => null);
  if (!vectors || vectors.length < corpus.items.length * spec.dimensions) {
    return rows;
  }
  for (const item of corpus.items) {
    if (typeof item.inputHash !== "string" || item.inputHash.length === 0) {
      continue;
    }
    const row = Number(item.row);
    if (!Number.isInteger(row) || row < 0) {
      continue;
    }
    rows.set(item.inputHash, vectors.slice(row * spec.dimensions, (row + 1) * spec.dimensions));
  }
  return rows;
}

async function loadRemotePackIfAvailable(corpus, remoteBaseUrl) {
  const spec = activeModelSpec();
  const runtime = activeQueryRuntime();
  if (!spec.remoteVectorPacks) {
    return remoteMiss("model-has-no-remote-vector-packs", {
      modelKey: spec.modelKey,
      modelLabel: spec.label,
    });
  }
  logInfo("looking for remote vector pack", {
    remoteBaseUrl,
    runtime,
    corpus: corpusSummary(corpus),
  });
  const catalogUrl = remotePackUrl(remoteBaseUrl, "catalog.json");
  const catalog = await fetchJsonIfAvailable(catalogUrl, "remote catalog");
  if (catalog === null) {
    return remoteMiss("catalog-unavailable", { catalogUrl });
  }
  logInfo("remote catalog loaded", {
    catalogUrl,
    catalog: catalogSummary(catalog),
  });
  const vectorSpace = selectCatalogVectorSpace(catalog, runtime);
  if (!vectorSpace?.manifest_url) {
    return remoteMiss("no-compatible-vector-space", {
      runtime,
      catalog: catalogSummary(catalog),
    });
  }
  logInfo("selected remote vector space", {
    vectorSpace: vectorSpaceSummary(vectorSpace),
  });
  const manifestUrl = remotePackUrl(remoteBaseUrl, vectorSpace.manifest_url);
  const manifest = await fetchJsonIfAvailable(manifestUrl, "remote manifest");
  if (manifest === null) {
    return remoteMiss("manifest-unavailable", { manifestUrl });
  }
  logInfo("remote manifest loaded", {
    manifestUrl,
    manifest: manifestSummary(manifest),
  });
  const manifestIssue = manifestCompatibilityIssue(manifest, corpus, runtime);
  if (manifestIssue !== null) {
    return remoteMiss("manifest-incompatible", manifestIssue);
  }
  const existing = activeModelPack(await getModelMeta("pack"));
  if (
    existing?.source === "remote"
    && existing.packId === manifest.pack_id
    && existing.inputHash === corpus.inputHash
    && existing.vectorSpaceKey === manifest.vector_space_key
    && packCompatibleWithRuntime(existing, runtime)
  ) {
    logInfo("existing remote vector pack is already current", {
      pack: packSummary(existing),
    });
    return remoteHit();
  }
  vectorCache.clear();
  const packBase = manifestUrl.replace(/\/manifest\.json$/, "");
  const totalVectorBytes = remotePackVectorBytes(manifest);
  let downloadedVectorBytes = 0;
  const corpora = {};
  for (const corpusManifest of manifest.corpora || []) {
    const corpusIssue = corpusManifestCompatibilityIssue(corpusManifest, corpus);
    if (corpusIssue !== null) {
      return remoteMiss("corpus-manifest-incompatible", corpusIssue);
    }
    logInfo("remote corpus pack accepted", {
      corpusId: corpusManifest.corpus_id,
      inputHash: shortHash(corpusManifest.input_hash),
      rowCount: corpusManifest.row_count,
      dimensions: corpusManifest.dimensions,
      itemsUrl: `${packBase}/${corpusManifest.items_url}`,
      vectorUrl: `${packBase}/${corpusManifest.vector_url}`,
      vectorByteLen: corpusManifest.vector_byte_len,
    });
    await updateStatus(
      "downloading-index",
      `Downloading ${corpusManifest.corpus_id} vector pack.`,
      progressValue("index", corpusManifest.corpus_id, downloadedVectorBytes, totalVectorBytes),
    );
    const itemsUrl = `${packBase}/${corpusManifest.items_url}`;
    logInfo("fetching remote corpus items", {
      corpusId: corpusManifest.corpus_id,
      url: itemsUrl,
    });
    const itemBytes = await fetchArrayBuffer(itemsUrl, "remote corpus items");
    await verifySha256(itemBytes, corpusManifest.items_sha256, corpusManifest.items_url);
    const items = parseJsonBytes(itemBytes, corpusManifest.items_url);
    if (!Array.isArray(items) || items.length !== corpusManifest.row_count) {
      throw new Error(`remote items ${corpusManifest.items_url} have the wrong row count`);
    }
    const vectorUrl = `${packBase}/${corpusManifest.vector_url}`;
    logInfo("fetching remote corpus vectors", {
      corpusId: corpusManifest.corpus_id,
      url: vectorUrl,
      expectedBytes: corpusManifest.vector_byte_len,
    });
    const bytes = await fetchArrayBufferWithProgress(vectorUrl, async (loaded) => {
      await updateStatus(
        "downloading-index",
        `Downloading ${corpusManifest.corpus_id} vectors.`,
        progressValue(
          "index",
          corpusManifest.corpus_id,
          downloadedVectorBytes + loaded,
          totalVectorBytes,
        ),
      );
    });
    if (bytes.byteLength !== corpusManifest.vector_byte_len) {
      throw new Error(`remote vector file ${corpusManifest.vector_url} has the wrong size`);
    }
    await verifySha256(bytes, corpusManifest.vector_sha256, corpusManifest.vector_url);
    const vectorExtension = manifest.element_type === "f16le" ? "f16" : "f32";
    const key = `remote/${spec.modelKey}/${manifest.vector_space_key}/${manifest.pack_id}/${corpusManifest.corpus_id}/vectors.${vectorExtension}`;
    await putBinary(key, bytes);
    logInfo("cached remote corpus vectors", {
      corpusId: corpusManifest.corpus_id,
      key,
      bytes: bytes.byteLength,
    });
    downloadedVectorBytes += bytes.byteLength;
    corpora[corpusManifest.corpus_id] = {
      corpusId: corpusManifest.corpus_id,
      inputHash: corpusManifest.input_hash,
      rowCount: corpusManifest.row_count,
      dimensions: corpusManifest.dimensions,
      elementType: manifest.element_type,
      items: normalizeRemoteItems(items, corpusManifest.corpus_id),
      shards: [{ key, byteLen: bytes.byteLength }],
    };
  }
  const pack = {
    source: "remote",
    packId: manifest.pack_id,
    modelKey: spec.modelKey,
    inputHash: manifest.input_hash,
    inputFormatVersion: manifest.input_format_version,
    vectorSpaceKey: manifest.vector_space_key,
    runtime,
    compatibleQueryRuntimes: manifest.compatible_query_runtimes || [],
    corpora,
  };
  await putModelMeta("pack", pack);
  logInfo("remote vector pack ready", {
    pack: packSummary(pack),
  });
  return remoteHit();
}

function selectCatalogVectorSpace(catalog, runtime) {
  const modelKey = activeModelSpec().modelKey;
  const model = (catalog.models || []).find((item) => item.model_key === modelKey);
  if (!model) {
    return null;
  }
  return (model.vector_spaces || []).find((space) =>
    (space.compatible_query_runtimes || []).some((candidate) =>
      runtimeMatches(candidate, runtime)
    )
  ) || null;
}

function remoteHit() {
  return { loaded: true };
}

function remoteMiss(reason, detail = null) {
  logWarn("remote vector pack rejected", { reason, detail });
  return { loaded: false, reason, detail };
}

function catalogSummary(catalog) {
  return {
    schemaVersion: catalog?.schema_version || null,
    models: (catalog?.models || []).map((model) => ({
      modelKey: model.model_key || null,
      vectorSpaces: (model.vector_spaces || []).map(vectorSpaceSummary),
    })),
  };
}

function vectorSpaceSummary(vectorSpace) {
  return {
    vectorSpaceKey: vectorSpace?.vector_space_key || null,
    latestPackId: vectorSpace?.latest_pack_id || null,
    manifestUrl: vectorSpace?.manifest_url || null,
    compatibleQueryRuntimes: vectorSpace?.compatible_query_runtimes || [],
  };
}

function manifestSummary(manifest) {
  return {
    schemaVersion: manifest?.schema_version || null,
    modelKey: manifest?.model_key || null,
    inputFormatVersion: manifest?.input_format_version || null,
    inputHash: shortHash(manifest?.input_hash),
    vectorSpaceKey: manifest?.vector_space_key || null,
    packId: manifest?.pack_id || null,
    dimensions: manifest?.dimensions || null,
    elementType: manifest?.element_type || null,
    compatibleQueryRuntimes: manifest?.compatible_query_runtimes || [],
    corpora: (manifest?.corpora || []).map((corpus) => ({
      corpusId: corpus.corpus_id || null,
      inputHash: shortHash(corpus.input_hash),
      rowCount: corpus.row_count || null,
      dimensions: corpus.dimensions || null,
      vectorByteLen: corpus.vector_byte_len || null,
    })),
  };
}

function manifestCompatibilityIssue(manifest, corpus, runtime) {
  const spec = activeModelSpec();
  const expectedElementType = spec.vectorElementType || "f32le";
  for (const [field, actual, expected] of [
    ["schema_version", manifest.schema_version, 1],
    ["model_key", manifest.model_key, spec.modelKey],
    ["input_hash", manifest.input_hash, corpus.inputHash],
    ["input_format_version", manifest.input_format_version, corpus.inputFormatVersion],
    ["dimensions", manifest.dimensions, spec.dimensions],
    ["element_type", manifest.element_type, expectedElementType],
    ["normalized", manifest.normalized, true],
    ["distance", manifest.distance, "dot"],
  ]) {
    if (actual !== expected) {
      return {
        field,
        expected: summarizeCompatibilityValue(expected),
        actual: summarizeCompatibilityValue(actual),
        corpus: corpusSummary(corpus),
        manifest: manifestSummary(manifest),
        runtime,
      };
    }
  }
  if (!(manifest.compatible_query_runtimes || []).some((candidate) =>
    runtimeMatches(candidate, runtime)
  )) {
    return {
      field: "compatible_query_runtimes",
      expected: runtime,
      actual: manifest.compatible_query_runtimes || [],
      corpus: corpusSummary(corpus),
      manifest: manifestSummary(manifest),
      runtime,
    };
  }
  return null;
}

function corpusManifestCompatibilityIssue(corpusManifest, corpus) {
  const spec = activeModelSpec();
  const expectedElementSize = bytesPerVectorElement(spec.vectorElementType || "f32le");
  const expectedHash = corpusManifest.corpus_id === "vlacku-en"
    ? corpus.dictionaryHash
    : corpusManifest.corpus_id === "cukta-cll"
      ? corpus.cllHash
      : null;
  if (expectedHash === null) {
    return {
      field: "corpus_id",
      expected: ["vlacku-en", "cukta-cll"],
      actual: corpusManifest.corpus_id || null,
      corpus: corpusSummary(corpus),
      corpusManifest: manifestSummary({ corpora: [corpusManifest] }).corpora[0],
    };
  }
  for (const [field, actual, expected] of [
    ["input_hash", corpusManifest.input_hash, expectedHash],
    ["dimensions", corpusManifest.dimensions, spec.dimensions],
    ["vector_byte_len", corpusManifest.vector_byte_len, corpusManifest.row_count * spec.dimensions * expectedElementSize],
  ]) {
    if (actual !== expected) {
      return {
        field,
        corpusId: corpusManifest.corpus_id,
        expected: summarizeCompatibilityValue(expected),
        actual: summarizeCompatibilityValue(actual),
        corpus: corpusSummary(corpus),
        corpusManifest: manifestSummary({ corpora: [corpusManifest] }).corpora[0],
      };
    }
  }
  return null;
}

function bytesPerVectorElement(elementType) {
  if (elementType === "f32le") {
    return 4;
  }
  if (elementType === "f16le") {
    return 2;
  }
  throw new Error(`unsupported vector element type: ${elementType}`);
}

function summarizeCompatibilityValue(value) {
  if (typeof value === "string" && value.length === 64) {
    return shortHash(value);
  }
  return value;
}

function runtimeMatches(candidate, runtime) {
  if (runtime?.runtime === F2LLM_WEBGPU_RUNTIME) {
    return candidate?.runtime === F2LLM_WEBGPU_RUNTIME
      && candidate?.dtype === runtime.dtype
      && (!candidate.device || candidate.device === runtime.device)
      && (!candidate.version || candidate.version === runtime.version);
  }
  if (runtime?.runtime === F2LLM_WASM_RUNTIME) {
    return candidate?.runtime === F2LLM_WASM_RUNTIME
      && candidate?.dtype === runtime.dtype
      && (!candidate.device || candidate.device === runtime.device)
      && (!candidate.version || candidate.version === runtime.version);
  }
  return false;
}

function f2llmRuntimeDescriptor(runtime) {
  if (runtime?.runtime === F2LLM_WEBGPU_RUNTIME) {
    return {
      runtime: runtime.runtime,
      version: runtime.version || F2LLM_WEBGPU_RUNTIME_VERSION,
      dtype: runtime.dtype,
      device: runtime.device,
    };
  }
  if (runtime?.runtime === F2LLM_WASM_RUNTIME) {
    return {
      runtime: runtime.runtime,
      version: runtime.version || F2LLM_WASM_RUNTIME_VERSION,
      dtype: runtime.dtype,
      device: runtime.device,
    };
  }
  throw new Error(`unsupported F2LLM runtime: ${runtime?.runtime || "missing"}`);
}

function queryRuntimeFromModelRuntime(runtime) {
  return f2llmRuntimeDescriptor(runtime);
}

function activeQueryRuntime() {
  if (!modelRuntime) {
    throw new Error(`${activeModelSpec().label} query model is not loaded`);
  }
  return queryRuntimeFromModelRuntime(modelRuntime);
}

function isCustomWebGpuRuntime(runtime) {
  return runtime?.runtime === F2LLM_WEBGPU_RUNTIME && runtime?.device === "webgpu";
}

function activeModelPack(pack) {
  return pack?.modelKey === activeModelSpec().modelKey ? pack : null;
}

function activeStoredModelRuntime(runtime) {
  if (!runtime) {
    return null;
  }
  const spec = activeModelSpec();
  return runtime.modelKey === spec.modelKey ? runtime : null;
}

function activeStatusMeta(meta) {
  if (!meta) {
    return null;
  }
  const spec = activeModelSpec();
  return meta.modelKey === spec.modelKey ? meta : null;
}

function modelBytesForRuntime(runtime) {
  const spec = activeModelSpec();
  return spec.modelSizeEstimates[runtime?.dtype] || 0;
}

function packCompatibleWithRuntime(pack, runtime) {
  if (!pack || pack.modelKey !== activeModelSpec().modelKey) {
    return false;
  }
  const runtimes = pack.compatibleQueryRuntimes || (pack.runtime ? [pack.runtime] : []);
  return runtimes.some((candidate) => runtimeMatches(candidate, runtime));
}

function cachedPackMatchesCorpus(pack, corpus, runtime, vectorSpaceKey) {
  return pack?.source === "browser"
    && pack.modelKey === activeModelSpec().modelKey
    && pack.inputHash === corpus.inputHash
    && pack.inputFormatVersion === corpus.inputFormatVersion
    && pack.vectorSpaceKey === vectorSpaceKey
    && packCompatibleWithRuntime(pack, runtime);
}

function remotePackVectorBytes(manifest) {
  let total = 0;
  for (const corpus of manifest.corpora || []) {
    total += corpus.vector_byte_len || 0;
  }
  return total;
}

async function fetchJsonIfAvailable(url, label = "JSON") {
  logInfo("fetching JSON", { label, url });
  const response = await fetch(url, { cache: "no-cache" }).catch((error) => {
    logWarn("JSON fetch failed", {
      label,
      url,
      error: errorMessage(error),
    });
    return null;
  });
  if (!response) {
    return null;
  }
  if (!response.ok) {
    logWarn("JSON fetch returned non-OK response", {
      label,
      url,
      status: response.status,
      statusText: response.statusText,
    });
    return null;
  }
  const text = await response.text();
  if (looksLikeHtmlResponse(response, text)) {
    logWarn("JSON fetch returned HTML", {
      label,
      url,
      contentType: response.headers.get("content-type") || "",
      preview: text.trimStart().slice(0, 120),
    });
    return null;
  }
  try {
    return JSON.parse(text);
  } catch (error) {
    logWarn("JSON fetch returned invalid JSON", {
      label,
      url,
      error: errorMessage(error),
      preview: text.trimStart().slice(0, 120),
    });
    return null;
  }
}

async function fetchJsonWith(fetchArrayBuffer, url, label = "JSON") {
  return parseJsonBytes(await fetchArrayBuffer(url, label), label);
}

function cachedFetchArrayBufferForSpec(spec) {
  return (url, label = "model artifact") => cachedFetchArrayBuffer(spec, url, label);
}

async function cachedFetchArrayBuffer(spec, url, label) {
  const normalizedUrl = new URL(url, globalThis.location.href).href;
  logInfo("fetching cached model artifact", {
    modelKey: spec.modelKey,
    label,
    url: normalizedUrl,
  });
  if (typeof caches === "undefined") {
    return fetchArrayBuffer(normalizedUrl, label);
  }
  const cache = await caches.open(MODEL_CACHE_NAME);
  const request = new Request(normalizedUrl, { method: "GET" });
  const cached = await cache.match(request);
  if (cached) {
    logInfo("using cached model artifact", {
      modelKey: spec.modelKey,
      label,
      url: normalizedUrl,
      contentLength: cached.headers.get("content-length"),
    });
    return cached.arrayBuffer();
  }
  const response = await fetch(normalizedUrl);
  if (!response.ok) {
    throw new Error(`failed to fetch ${label} from ${normalizedUrl}: ${response.status}`);
  }
  await cache.put(request, response.clone());
  logInfo("stored model artifact in cache", {
    modelKey: spec.modelKey,
    label,
    url: normalizedUrl,
    contentLength: response.headers.get("content-length"),
  });
  return response.arrayBuffer();
}

async function removeCachedModelArtifacts(spec) {
  if (typeof caches === "undefined") {
    return false;
  }
  const cache = await caches.open(MODEL_CACHE_NAME);
  const prefixes = modelArtifactUrlPrefixes(spec);
  for (const request of await cache.keys()) {
    const url = request.url;
    if (prefixes.some((prefix) => url.startsWith(prefix))) {
      await cache.delete(request);
    }
  }
  return true;
}

function modelArtifactUrlPrefixes(spec) {
  const prefixes = [];
  if (spec.customRuntime?.artifactBaseUrl) {
    prefixes.push(new URL(`${spec.customRuntime.artifactBaseUrl.replace(/\/+$/, "")}/`, globalThis.location.href).href);
  }
  if (spec.wasmRuntime?.onnxUrl) {
    prefixes.push(new URL(spec.wasmRuntime.onnxUrl, globalThis.location.href).href);
  }
  return prefixes;
}

async function fetchArrayBuffer(url, label = "binary") {
  logInfo("fetching binary", { label, url });
  const response = await fetch(url);
  if (!response.ok) {
    logWarn("binary fetch returned non-OK response", {
      label,
      url,
      status: response.status,
      statusText: response.statusText,
    });
    throw new Error(`failed to fetch ${url}: ${response.status}`);
  }
  logInfo("binary fetch response ready", {
    label,
    url,
    contentLength: response.headers.get("content-length"),
    contentEncoding: response.headers.get("content-encoding"),
  });
  return response.arrayBuffer();
}

async function fetchArrayBufferWithProgress(url, onProgress) {
  logInfo("fetching binary with progress", { url });
  const response = await fetch(url);
  if (!response.ok) {
    logWarn("binary progress fetch returned non-OK response", {
      url,
      status: response.status,
      statusText: response.statusText,
    });
    throw new Error(`failed to fetch ${url}: ${response.status}`);
  }
  logInfo("binary progress fetch response ready", {
    url,
    contentLength: response.headers.get("content-length"),
    contentEncoding: response.headers.get("content-encoding"),
  });
  if (!response.body?.getReader) {
    const buffer = await response.arrayBuffer();
    await onProgress(buffer.byteLength);
    return buffer;
  }
  const reader = response.body.getReader();
  const chunks = [];
  let loaded = 0;
  for (;;) {
    const { done, value } = await reader.read();
    if (done) {
      break;
    }
    chunks.push(value);
    loaded += value.byteLength;
    await onProgress(loaded);
  }
  const bytes = new Uint8Array(loaded);
  let offset = 0;
  for (const chunk of chunks) {
    bytes.set(chunk, offset);
    offset += chunk.byteLength;
  }
  return bytes.buffer;
}

async function verifySha256(buffer, expected, name) {
  const actual = await sha256Hex(buffer);
  if (actual !== expected) {
    logWarn("SHA-256 verification failed", {
      name,
      expected: summarizeCompatibilityValue(expected),
      actual: summarizeCompatibilityValue(actual),
      byteLength: buffer.byteLength,
    });
    throw new Error(`${name} SHA-256 mismatch`);
  }
  logInfo("SHA-256 verified", {
    name,
    sha256: summarizeCompatibilityValue(actual),
    byteLength: buffer.byteLength,
  });
}

async function sha256Hex(buffer) {
  const digest = await crypto.subtle.digest("SHA-256", buffer);
  return Array.from(new Uint8Array(digest))
    .map((byte) => byte.toString(16).padStart(2, "0"))
    .join("");
}

function parseJsonBytes(buffer, name) {
  const text = new TextDecoder("utf-8", { fatal: true }).decode(buffer);
  try {
    return JSON.parse(text);
  } catch (error) {
    throw new Error(`invalid JSON from ${name}: ${errorMessage(error)}`);
  }
}

function looksLikeHtmlResponse(response, text) {
  const contentType = response.headers.get("content-type") || "";
  if (contentType.toLowerCase().includes("text/html")) {
    return true;
  }
  const trimmed = text.trimStart().toLowerCase();
  return trimmed.startsWith("<!doctype html") || trimmed.startsWith("<html");
}

async function readCorpusVectors(corpus) {
  const cacheKey = corpusVectorCacheKey(corpus);
  const cached = vectorCache.get(cacheKey);
  if (cached) {
    return cached;
  }
  const buffers = [];
  let totalBytes = 0;
  for (const shard of corpus.shards || []) {
    const buffer = await getBinary(shard.key);
    buffers.push(buffer);
    totalBytes += buffer.byteLength;
  }
  const combined = new Uint8Array(totalBytes);
  let offset = 0;
  for (const buffer of buffers) {
    combined.set(new Uint8Array(buffer), offset);
    offset += buffer.byteLength;
  }
  const elementType = corpus.elementType || "f32le";
  const vectors = elementType === "f32le"
    ? new Float32Array(combined.buffer)
    : elementType === "f16le"
      ? f16leBytesToF32(combined.buffer)
      : null;
  if (vectors === null) {
    throw new Error(`unsupported CPU vector element type: ${elementType}`);
  }
  vectorCache.set(cacheKey, vectors);
  return vectors;
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
    corpus.elementType || "f32le",
    shards,
  ].join("::");
}

function f16leBytesToF32(buffer) {
  const input = new DataView(buffer);
  const output = new Float32Array(buffer.byteLength / 2);
  for (let offset = 0, index = 0; offset < buffer.byteLength; offset += 2, index += 1) {
    output[index] = f16ToF32(input.getUint16(offset, true));
  }
  return output;
}

function f16ToF32(bits) {
  const sign = (bits & 0x8000) ? -1 : 1;
  const exponent = (bits >> 10) & 0x1f;
  const fraction = bits & 0x03ff;
  if (exponent === 0) {
    return fraction === 0
      ? sign * 0
      : sign * 2 ** -14 * (fraction / 1024);
  }
  if (exponent === 0x1f) {
    return fraction === 0 ? sign * Infinity : NaN;
  }
  return sign * 2 ** (exponent - 15) * (1 + fraction / 1024);
}

function f32ToF16Bits(value) {
  if (Number.isNaN(value)) {
    return 0x7e00;
  }
  const sign = value < 0 || Object.is(value, -0) ? 0x8000 : 0;
  const abs = Math.abs(value);
  if (abs === 0) {
    return sign;
  }
  if (abs === Infinity) {
    return sign | 0x7c00;
  }
  if (abs >= 65504) {
    return sign | 0x7bff;
  }
  if (abs < 2 ** -24) {
    return sign;
  }
  if (abs < 2 ** -14) {
    return sign | Math.round(abs / (2 ** -24));
  }
  let exponent = Math.floor(Math.log2(abs));
  let fraction = Math.round((abs / (2 ** exponent) - 1) * 1024);
  if (fraction === 1024) {
    exponent += 1;
    fraction = 0;
  }
  return sign | ((exponent + 15) << 10) | (fraction & 0x03ff);
}

function rankHits(vectors, query, items, dimensions, limit, kindFilters) {
  const rowCount = Math.min(items.length, Math.floor(vectors.length / dimensions));
  const limitCount = Math.trunc(Number(limit) || 0);
  if (limitCount <= 0) {
    return rankAllHits(vectors, query, items, dimensions, rowCount, kindFilters);
  }
  const hits = [];
  for (let row = 0; row < rowCount; row += 1) {
    if (!itemMatchesKindFilters(items[row], kindFilters)) {
      continue;
    }
    let score = 0;
    const base = row * dimensions;
    for (let dim = 0; dim < dimensions; dim += 1) {
      score += vectors[base + dim] * query[dim];
    }
    const candidate = { id: items[row].id, score };
    if (hits.length < limitCount) {
      hits.push(candidate);
      continue;
    }
    const worstIndex = worstHitIndex(hits);
    if (worstIndex !== -1 && compareHits(candidate, hits[worstIndex]) < 0) {
      hits[worstIndex] = candidate;
    }
  }
  hits.sort(compareHits);
  return hits;
}

function rankAllHits(vectors, query, items, dimensions, rowCount, kindFilters) {
  const hits = [];
  for (let row = 0; row < rowCount; row += 1) {
    if (!itemMatchesKindFilters(items[row], kindFilters)) {
      continue;
    }
    let score = 0;
    const base = row * dimensions;
    for (let dim = 0; dim < dimensions; dim += 1) {
      score += vectors[base + dim] * query[dim];
    }
    hits.push({ id: items[row].id, score });
  }
  hits.sort(compareHits);
  return hits;
}

function compareHits(left, right) {
  return right.score - left.score || left.id - right.id;
}

function worstHitIndex(hits) {
  let worstIndex = -1;
  for (let index = 0; index < hits.length; index += 1) {
    if (worstIndex === -1 || compareHits(hits[index], hits[worstIndex]) > 0) {
      worstIndex = index;
    }
  }
  return worstIndex;
}

function itemMatchesKindFilters(item, kindFilters) {
  if (kindFilters.length === 0) {
    return true;
  }
  const kind = normalizeWordTypeFilter(item?.kind || "");
  return kind.length > 0 && kindFilters.some((wanted) => matchesWordTypeFilter(wanted, kind));
}

function matchesWordTypeFilter(wanted, normalizedType) {
  return wanted === normalizedType
    || (wanted === "cmavo" && isCmavoLike(normalizedType))
    || (wanted === "letteral" && isLetteralLike(normalizedType))
    || (wanted === "cmevla" && isCmevlaLike(normalizedType))
    || (wanted === "gismu" && isGismuLike(normalizedType))
    || (wanted === "fu'ivla" && isFuhivlaLike(normalizedType))
    || (wanted === "lujvo" && isLujvoLike(normalizedType))
    || (wanted === "brivla" && isBrivlaLike(normalizedType));
}

function normalizeWordTypeFilter(value) {
  return String(value || "").trim().toLowerCase().split(" ").join("-");
}

function isCmavoLike(normalizedType) {
  return normalizedType === "cmavo"
    || normalizedType.startsWith("cmavo-")
    || normalizedType === "experimental-cmavo"
    || normalizedType === "obsolete-cmavo";
}

function isLetteralLike(normalizedType) {
  return normalizedType === "bu-letteral" || normalizedType === "letteral";
}

function isCmevlaLike(normalizedType) {
  return normalizedType === "cmevla" || normalizedType === "obsolete-cmevla";
}

function isGismuLike(normalizedType) {
  return normalizedType === "gismu" || normalizedType === "experimental-gismu";
}

function isFuhivlaLike(normalizedType) {
  return normalizedType === "fu'ivla" || normalizedType === "obsolete-fu'ivla";
}

function isLujvoLike(normalizedType) {
  return normalizedType === "lujvo"
    || normalizedType === "zei-lujvo"
    || normalizedType === "obsolete-zei-lujvo";
}

function isBrivlaLike(normalizedType) {
  return isGismuLike(normalizedType)
    || isLujvoLike(normalizedType)
    || isFuhivlaLike(normalizedType);
}

function corpusSupportsKindFilters(corpus) {
  return (corpus.items || []).some((item) =>
    typeof item.kind === "string" && item.kind.length > 0
  );
}

function parseStringArray(json) {
  const value = JSON.parse(json || "[]");
  if (!Array.isArray(value)) {
    throw new Error("embedding search filters must be a JSON array");
  }
  return value.filter((item) => typeof item === "string");
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

async function requestPersistentStorage() {
  if (navigator.storage?.persist) {
    await navigator.storage.persist().catch(() => false);
  }
}

async function checkQuota(forceWasm = false) {
  if (!navigator.storage?.estimate) {
    return;
  }
  const spec = activeModelSpec();
  const estimate = await navigator.storage.estimate();
  const usage = estimate.usage || 0;
  const quota = estimate.quota || 0;
  const runtime = activeStoredModelRuntime(modelRuntime);
  const expectedRuntime = runtime || (activeRuntimeMode === "wasm" ? spec.wasmRuntime : spec.preferredRuntime);
  const minimum = spec.minFreeBytesByDtype[expectedRuntime.dtype] || 0;
  if (quota > 0 && quota - usage < minimum) {
    throw new Error(`not enough browser storage quota for the ${spec.label} model and vector index`);
  }
}

async function packIndexBytes(pack) {
  if (!pack?.corpora) {
    return 0;
  }
  let total = 0;
  for (const corpus of Object.values(pack.corpora)) {
    for (const shard of corpus.shards || []) {
      total += shard.byteLen || 0;
    }
  }
  return total;
}

async function updateStatus(status, detail, progress = null) {
  await putModelMeta("status", {
    status,
    detail,
    progress,
    modelKey: activeModelSpec().modelKey,
    updatedAt: Date.now(),
  });
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

function indeterminateProgress(kind, label) {
  return { kind, label, loaded: null, total: null, percent: null };
}

function shortHash(value) {
  return typeof value === "string" && value.length >= 12 ? value.slice(0, 12) : value || null;
}

async function openDb() {
  if (dbPromise !== null) {
    return dbPromise;
  }
  dbPromise = new Promise((resolve, reject) => {
    const request = indexedDB.open(DB_NAME, 1);
    request.onupgradeneeded = () => {
      const db = request.result;
      if (!db.objectStoreNames.contains(META_STORE)) {
        db.createObjectStore(META_STORE);
      }
      if (!db.objectStoreNames.contains(BLOB_STORE)) {
        db.createObjectStore(BLOB_STORE);
      }
    };
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(request.error || new Error("failed to open IndexedDB"));
  });
  return dbPromise;
}

function transaction(db, stores, mode, body) {
  return new Promise((resolve, reject) => {
    const tx = db.transaction(stores, mode);
    tx.oncomplete = () => resolve();
    tx.onerror = () => reject(tx.error || new Error("IndexedDB transaction failed"));
    tx.onabort = () => reject(tx.error || new Error("IndexedDB transaction aborted"));
    body(tx);
  });
}

async function getMeta(key) {
  const db = await openDb();
  return new Promise((resolve, reject) => {
    const tx = db.transaction(META_STORE, "readonly");
    const request = tx.objectStore(META_STORE).get(key);
    request.onsuccess = () => resolve(request.result ?? null);
    request.onerror = () => reject(request.error || new Error(`failed to read ${key}`));
  });
}

function modelMetaKey(kind, modelKey = activeModelSpec().modelKey) {
  return `${kind}:${modelKey}`;
}

async function getModelMeta(kind, modelKey = activeModelSpec().modelKey) {
  const scoped = await getMeta(modelMetaKey(kind, modelKey));
  if (scoped !== null) {
    return scoped;
  }
  if (modelKey !== F2LLM_80M_MODEL_KEY) {
    return null;
  }
  const legacy = await getMeta(kind);
  if (kind === "status") {
    return activeStatusMeta(legacy);
  }
  if (kind === "pack") {
    return activeModelPack(legacy);
  }
  if (kind === "modelRuntime") {
    return activeStoredModelRuntime(legacy);
  }
  return null;
}

async function putModelMeta(kind, value, modelKey = activeModelSpec().modelKey) {
  await putMeta(modelMetaKey(kind, modelKey), {
    ...value,
    modelKey,
  });
}

async function putMeta(key, value) {
  const db = await openDb();
  await transaction(db, META_STORE, "readwrite", (tx) => {
    tx.objectStore(META_STORE).put(value, key);
  });
}

async function putBinary(key, buffer) {
  if (navigator.storage?.getDirectory) {
    try {
      await putOpfsBinary(key, buffer);
      await putBlobMeta(key, { storage: "opfs", byteLen: buffer.byteLength });
      return;
    } catch (_) {
      // Fall back to IndexedDB Blob storage below.
    }
  }
  const db = await openDb();
  await transaction(db, BLOB_STORE, "readwrite", (tx) => {
    tx.objectStore(BLOB_STORE).put(buffer, key);
  });
  await putBlobMeta(key, { storage: "indexeddb", byteLen: buffer.byteLength });
}

async function getBinary(key) {
  const meta = await getBlobMeta(key);
  if (meta?.storage === "opfs" && navigator.storage?.getDirectory) {
    try {
      return await getOpfsBinary(key);
    } catch (_) {
      // Fall back to IndexedDB below.
    }
  }
  const db = await openDb();
  return new Promise((resolve, reject) => {
    const tx = db.transaction(BLOB_STORE, "readonly");
    const request = tx.objectStore(BLOB_STORE).get(key);
    request.onsuccess = () => {
      if (!request.result) {
        reject(new Error(`missing vector shard ${key}`));
      } else {
        resolve(request.result);
      }
    };
    request.onerror = () => reject(request.error || new Error(`failed to read ${key}`));
  });
}

async function putBlobMeta(key, value) {
  const blobMeta = (await getMeta("blobMeta")) || {};
  blobMeta[key] = value;
  await putMeta("blobMeta", blobMeta);
}

async function getBlobMeta(key) {
  const blobMeta = (await getMeta("blobMeta")) || {};
  return blobMeta[key] || null;
}

async function removeBlobMetaForModel(modelKey) {
  const blobMeta = (await getMeta("blobMeta")) || {};
  const prefixes = modelBlobPrefixes(modelKey);
  let changed = false;
  for (const key of Object.keys(blobMeta)) {
    if (prefixes.some((prefix) => key.startsWith(prefix))) {
      delete blobMeta[key];
      changed = true;
    }
  }
  if (changed) {
    await putMeta("blobMeta", blobMeta);
  }
}

function modelBlobPrefixes(modelKey) {
  return [
    `remote/${modelKey}`,
    `local/${modelKey}`,
  ];
}

async function putOpfsBinary(key, buffer) {
  const file = await opfsFileHandle(key, true);
  const writable = await file.createWritable();
  await writable.write(buffer);
  await writable.close();
}

async function getOpfsBinary(key) {
  const file = await opfsFileHandle(key, false);
  return file.getFile().then((blob) => blob.arrayBuffer());
}

async function opfsFileHandle(key, create) {
  let directory = await navigator.storage.getDirectory();
  for (const part of ["jbotci", "embeddings", ...key.split("/").slice(0, -1)]) {
    directory = await directory.getDirectoryHandle(part, { create });
  }
  return directory.getFileHandle(key.split("/").at(-1), { create });
}

async function removeOpfsDirectory(root, path) {
  if (path.length === 0) {
    return;
  }
  if (path.length === 1) {
    await root.removeEntry(path[0], { recursive: true });
    return;
  }
  const next = await root.getDirectoryHandle(path[0]);
  await removeOpfsDirectory(next, path.slice(1));
}
