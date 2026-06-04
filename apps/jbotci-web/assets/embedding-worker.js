import {
  AutoModel,
  AutoTokenizer,
} from "https://cdn.jsdelivr.net/npm/@huggingface/transformers@4.2.0";

const MODEL_KEY = "embedding-gemma-300m-q4-768";
const MODEL_ID = "onnx-community/embeddinggemma-300m-ONNX";
const PREFERRED_MODEL_DTYPE = "q4";
const FALLBACK_MODEL_DTYPE = "q8";
const DIMENSIONS = 768;
const MAX_SEQUENCE_LENGTH = 2048;
const QUERY_PREFIX = "task: search result | query: ";
const DB_NAME = "jbotci-embeddings-v1";
const META_STORE = "meta";
const BLOB_STORE = "blobs";
const TRANSFORMERS_VERSION = "4.2.0";
const DEFAULT_REMOTE_BASE_URL = "/assets/embeddings/web/v1";
const LOG_PREFIX = "[jbotci embeddings worker]";
const LOCAL_VECTOR_SPACE_PREFIX = "browser-local";
const Q4_MIN_FREE_BYTES = 300 * 1024 * 1024;
const Q8_MIN_FREE_BYTES = 500 * 1024 * 1024;
const MODEL_SIZE_ESTIMATES = {
  q4: 217 * 1024 * 1024,
  q8: 330 * 1024 * 1024,
};
const ACTIVE_SETUP_STATUSES = new Set([
  "checking",
  "downloading-index",
  "downloading-model",
  "indexing",
  "loading-model",
]);
const EMBED_BATCH_SIZE = 8;

let tokenizerPromise = null;
let modelPromise = null;
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
  try {
    let value;
    if (type === "status") {
      value = await status();
    } else if (type === "setup") {
      value = await setup(
        payload?.corpusJson || "{}",
        normalizeRemoteBaseUrl(payload?.remoteBaseUrl),
      );
    } else if (type === "remove") {
      value = await removeAll();
    } else if (type === "search") {
      value = await search(
        payload?.corpusId,
        payload?.query,
        payload?.limit || 0,
        payload?.kindFiltersJson || "[]",
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
    });
  }
};

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

async function setup(corpusJson, remoteBaseUrl) {
  if (setupInProgress) {
    logInfo("setup request ignored because setup is already active");
    return status();
  }
  setupInProgress = true;
  try {
    const corpus = normalizeCorpus(JSON.parse(corpusJson));
    logInfo("setup started", {
      remoteBaseUrl,
      corpus: corpusSummary(corpus),
    });
    await requestPersistentStorage();
    await checkQuota();
    await updateStatus("loading-model", "Downloading or opening EmbeddingGemma.");
    await ensureModel();
    logInfo("query model ready", { runtime: activeQueryRuntime() });
    await checkQuota();
    await updateStatus("checking", "Looking for a vector pack.");
    const remoteAttempt = await loadRemotePackIfAvailable(corpus, remoteBaseUrl);
    if (!remoteAttempt.loaded) {
      logWarn("remote vector pack unavailable; falling back to browser-local indexing", {
        reason: remoteAttempt.reason,
        detail: remoteAttempt.detail || null,
      });
      await buildLocalPack(corpus);
    }
    const pack = await getMeta("pack");
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
  const meta = await getMeta("status");
  const pack = await getMeta("pack");
  const storedModelRuntime = modelRuntime || await getMeta("modelRuntime");
  const indexBytes = await packIndexBytes(pack);
  const display = statusDisplay(meta, pack);
  if (display.rewriteStoredStatus) {
    await updateStatus(display.status, display.detail, display.progress);
  }
  return {
    status: display.status,
    detail: display.detail,
    modelBytes: storedModelRuntime
      ? (MODEL_SIZE_ESTIMATES[storedModelRuntime.dtype] || 0)
      : 0,
    indexBytes,
    modelKey: MODEL_KEY,
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
  return {
    status: pack ? (meta?.status || "ready") : (meta?.status || "not-installed"),
    detail: meta?.detail || (pack
      ? "Embedding index is cached in this browser."
      : "No browser embedding index is installed."),
    progress: meta?.progress || null,
    rewriteStoredStatus: false,
  };
}

async function removeAll() {
  vectorCache.clear();
  const db = await openDb();
  await transaction(db, [META_STORE, BLOB_STORE], "readwrite", (tx) => {
    tx.objectStore(META_STORE).clear();
    tx.objectStore(BLOB_STORE).clear();
  });
  if (navigator.storage?.getDirectory) {
    const root = await navigator.storage.getDirectory();
    await removeOpfsDirectory(root, ["jbotci", "embeddings"]).catch(() => {});
  }
  await updateStatus("not-installed", "Browser embedding storage was removed.");
  return status();
}

async function search(corpusId, query, limit, kindFiltersJson) {
  const trimmedQuery = String(query || "").trim();
  if (!trimmedQuery) {
    return { hits: [], message: null };
  }
  const kindFilters = parseStringArray(kindFiltersJson)
    .map(normalizeWordTypeFilter)
    .filter((value) => value.length > 0);
  const pack = await getMeta("pack");
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
  const storedRuntime = modelRuntime || await getMeta("modelRuntime");
  if (storedRuntime && !packCompatibleWithRuntime(pack, queryRuntimeFromModelRuntime(storedRuntime))) {
    return {
      hits: [],
      message: "The cached embedding pack was built for a different browser embedding runtime. Open Settings and update embeddings.",
    };
  }
  await ensureModel();
  const runtime = activeQueryRuntime();
  if (!packCompatibleWithRuntime(pack, runtime)) {
    return {
      hits: [],
      message: "The cached embedding pack was built for a different browser embedding runtime. Open Settings and update embeddings.",
    };
  }
  const queryEmbedding = await embedTexts([QUERY_PREFIX + trimmedQuery]);
  const vectors = await readCorpusVectors(corpus);
  const hits = rankHits(vectors, queryEmbedding[0], corpus.items, corpus.dimensions, limit, kindFilters);
  return { hits, message: hits.length === 0 ? "No matches found." : null };
}

async function ensureModel() {
  if (tokenizerPromise === null) {
    tokenizerPromise = AutoTokenizer.from_pretrained(MODEL_ID);
  }
  if (modelPromise === null) {
    modelPromise = loadModelWithFallback();
  }
  const [tokenizer, model] = await Promise.all([tokenizerPromise, modelPromise]);
  return { tokenizer, model };
}

async function loadModelWithFallback() {
  if (await hasUsableWebGpu()) {
    try {
      await updateStatus("loading-model", "Opening EmbeddingGemma Q4 with WebGPU.");
      const model = await AutoModel.from_pretrained(MODEL_ID, {
        dtype: PREFERRED_MODEL_DTYPE,
        device: "webgpu",
        progress_callback: modelProgressCallback(PREFERRED_MODEL_DTYPE, "webgpu"),
      });
      modelRuntime = { dtype: PREFERRED_MODEL_DTYPE, device: "webgpu" };
      await putMeta("modelRuntime", modelRuntime);
      return model;
    } catch (error) {
      await updateStatus(
        "loading-model",
        `WebGPU Q4 failed; falling back to CPU/WASM Q8. ${errorMessage(error)}`,
      );
    }
  }
  const model = await AutoModel.from_pretrained(MODEL_ID, {
    dtype: FALLBACK_MODEL_DTYPE,
    progress_callback: modelProgressCallback(FALLBACK_MODEL_DTYPE, "wasm"),
  });
  modelRuntime = { dtype: FALLBACK_MODEL_DTYPE, device: "wasm" };
  await putMeta("modelRuntime", modelRuntime);
  return model;
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

function modelProgressCallback(dtype, device) {
  return (progress) => {
    if (!progress || typeof progress !== "object") {
      return;
    }
    const file = progress.file ? ` ${progress.file}` : "";
    if (progress.status === "progress" && progress.total) {
      const percent = Math.round((progress.loaded / progress.total) * 100);
      void updateStatus(
        "downloading-model",
        `Downloading EmbeddingGemma ${dtype}/${device}${file}: ${percent}%.`,
        progressValue("model", `EmbeddingGemma ${dtype}/${device}`, progress.loaded, progress.total),
      ).catch(() => {});
    } else if (progress.status === "download") {
      void updateStatus(
        "downloading-model",
        `Downloading EmbeddingGemma ${dtype}/${device}${file}.`,
        indeterminateProgress("model", `EmbeddingGemma ${dtype}/${device}`),
      ).catch(() => {});
    } else if (progress.status === "ready") {
      void updateStatus(
        "loading-model",
        `EmbeddingGemma ${dtype}/${device} is ready.`,
        progressValue("model", `EmbeddingGemma ${dtype}/${device}`, 1, 1),
      ).catch(() => {});
    }
  };
}

function errorMessage(error) {
  return error instanceof Error ? error.message : String(error);
}

function normalizeCorpus(raw) {
  const corpus = {
    modelKey: raw?.modelKey || raw?.model_key || raw?.["model-key"] || "",
    modelRevision: raw?.modelRevision || raw?.model_revision || "",
    inputFormatVersion: raw?.inputFormatVersion || raw?.input_format_version || "",
    inputHash: raw?.inputHash || raw?.input_hash || "",
    dictionaryHash: raw?.dictionaryHash || raw?.dictionary_hash || "",
    cllHash: raw?.cllHash || raw?.cll_hash || "",
    dictionary: normalizeInputDocuments(raw?.dictionary || [], "dictionary"),
    cll: normalizeInputDocuments(raw?.cll || [], "cll"),
  };
  if (corpus.modelKey !== MODEL_KEY) {
    throw new Error(`unsupported browser corpus model key: ${corpus.modelKey || "missing"}`);
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
  const { tokenizer, model } = await ensureModel();
  const output = [];
  if (progressContext !== null) {
    await updateStatus(
      "indexing",
      `Embedding ${progressContext.label}: 0 of ${texts.length} rows.`,
      progressValue("index", progressContext.label, 0, texts.length),
    );
  }
  for (let start = 0; start < texts.length; start += EMBED_BATCH_SIZE) {
    const batch = texts.slice(start, start + EMBED_BATCH_SIZE);
    const inputs = await tokenizer(batch, {
      padding: true,
      truncation: true,
      max_length: MAX_SEQUENCE_LENGTH,
    });
    const result = await model(inputs);
    const rows = await result.sentence_embedding.tolist();
    for (const row of rows) {
      const vector = Float32Array.from(row);
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
  if (corpus.modelKey !== MODEL_KEY) {
    throw new Error(`unsupported browser corpus model key: ${corpus.modelKey || "missing"}`);
  }
  const runtime = activeQueryRuntime();
  const vectorSpaceKey = `${LOCAL_VECTOR_SPACE_PREFIX}-${runtime.dtype}`;
  const packId = `${vectorSpaceKey}-${shortHash(corpus.inputHash)}`;
  const existing = await getMeta("pack");
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
    reusablePack?.corpora?.["vlacku-en"] || null,
  );
  corpora["cukta-cll"] = await buildLocalCorpus(
    "cukta-cll",
    corpus.cll,
    corpus.cllHash,
    packId,
    reusablePack?.corpora?.["cukta-cll"] || null,
  );
  await putMeta("pack", {
    source: "browser",
    packId,
    modelKey: MODEL_KEY,
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

async function buildLocalCorpus(corpusId, docs, inputHash, packId, reusableCorpus) {
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
  const vectors = new Float32Array(docs.length * DIMENSIONS);
  const pendingDocs = [];
  const pendingRows = [];
  let reused = 0;
  for (let row = 0; row < docs.length; row += 1) {
    const doc = docs[row];
    const reusedVector = reusableRows.get(doc.inputHash);
    if (reusedVector) {
      vectors.set(reusedVector, row * DIMENSIONS);
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
    const embeddings = await embedTexts(
      pendingDocs.map((doc) => doc.input),
      { label: corpusId },
    );
    for (let index = 0; index < embeddings.length; index += 1) {
      vectors.set(embeddings[index], pendingRows[index] * DIMENSIONS);
    }
  }
  const vectorKey = `local/${MODEL_KEY}/${packId}/${corpusId}/vectors.f32`;
  await putBinary(vectorKey, vectors.buffer);
  await updateStatus(
    "indexing",
    `Embedded ${corpusId}: ${docs.length} of ${docs.length} rows.`,
    progressValue("index", corpusId, docs.length, docs.length),
  );
  return {
    corpusId,
    inputHash,
    rowCount: docs.length,
    dimensions: DIMENSIONS,
    items: docs.map((doc, row) => ({
      id: doc.id,
      row,
      kind: normalizeWordTypeFilter(doc.kind || "") || null,
      inputHash: doc.inputHash,
    })),
    shards: [{ key: vectorKey, byteLen: vectors.byteLength }],
  };
}

async function reusableRowsByInputHash(corpus) {
  const rows = new Map();
  if (!corpus || corpus.dimensions !== DIMENSIONS || !Array.isArray(corpus.items)) {
    return rows;
  }
  const vectors = await readCorpusVectors(corpus).catch(() => null);
  if (!vectors || vectors.length < corpus.items.length * DIMENSIONS) {
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
    rows.set(item.inputHash, vectors.slice(row * DIMENSIONS, (row + 1) * DIMENSIONS));
  }
  return rows;
}

async function loadRemotePackIfAvailable(corpus, remoteBaseUrl) {
  const runtime = activeQueryRuntime();
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
  const existing = await getMeta("pack");
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
    const key = `remote/${MODEL_KEY}/${manifest.vector_space_key}/${manifest.pack_id}/${corpusManifest.corpus_id}/vectors.f32`;
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
      items: normalizeRemoteItems(items, corpusManifest.corpus_id),
      shards: [{ key, byteLen: bytes.byteLength }],
    };
  }
  const pack = {
    source: "remote",
    packId: manifest.pack_id,
    modelKey: MODEL_KEY,
    inputHash: manifest.input_hash,
    inputFormatVersion: manifest.input_format_version,
    vectorSpaceKey: manifest.vector_space_key,
    runtime,
    compatibleQueryRuntimes: manifest.compatible_query_runtimes || [],
    corpora,
  };
  await putMeta("pack", pack);
  logInfo("remote vector pack ready", {
    pack: packSummary(pack),
  });
  return remoteHit();
}

function selectCatalogVectorSpace(catalog, runtime) {
  const model = (catalog.models || []).find((item) => item.model_key === MODEL_KEY);
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
  for (const [field, actual, expected] of [
    ["schema_version", manifest.schema_version, 1],
    ["model_key", manifest.model_key, MODEL_KEY],
    ["input_hash", manifest.input_hash, corpus.inputHash],
    ["input_format_version", manifest.input_format_version, corpus.inputFormatVersion],
    ["dimensions", manifest.dimensions, DIMENSIONS],
    ["element_type", manifest.element_type, "f32le"],
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
    ["dimensions", corpusManifest.dimensions, DIMENSIONS],
    ["vector_byte_len", corpusManifest.vector_byte_len, corpusManifest.row_count * DIMENSIONS * 4],
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

function summarizeCompatibilityValue(value) {
  if (typeof value === "string" && value.length === 64) {
    return shortHash(value);
  }
  return value;
}

function runtimeMatches(candidate, runtime) {
  return candidate?.runtime === "transformers.js"
    && candidate?.dtype === runtime.dtype
    && (!candidate.version || candidate.version === TRANSFORMERS_VERSION);
}

function activeQueryRuntime() {
  if (!modelRuntime) {
    throw new Error("EmbeddingGemma query model is not loaded");
  }
  return queryRuntimeFromModelRuntime(modelRuntime);
}

function queryRuntimeFromModelRuntime(runtime) {
  return {
    runtime: "transformers.js",
    version: TRANSFORMERS_VERSION,
    dtype: runtime.dtype,
    device: runtime.device,
  };
}

function packCompatibleWithRuntime(pack, runtime) {
  if (!pack || pack.modelKey !== MODEL_KEY) {
    return false;
  }
  const runtimes = pack.compatibleQueryRuntimes || (pack.runtime ? [pack.runtime] : []);
  return runtimes.some((candidate) => runtimeMatches(candidate, runtime));
}

function cachedPackMatchesCorpus(pack, corpus, runtime, vectorSpaceKey) {
  return pack?.source === "browser"
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
  const vectors = new Float32Array(combined.buffer);
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
    shards,
  ].join("::");
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

async function checkQuota() {
  if (!navigator.storage?.estimate) {
    return;
  }
  const estimate = await navigator.storage.estimate();
  const usage = estimate.usage || 0;
  const quota = estimate.quota || 0;
  const minimum = modelRuntime?.dtype === PREFERRED_MODEL_DTYPE || (!modelRuntime && await hasUsableWebGpu())
    ? Q4_MIN_FREE_BYTES
    : Q8_MIN_FREE_BYTES;
  if (quota > 0 && quota - usage < minimum) {
    throw new Error("not enough browser storage quota for the EmbeddingGemma model and vector index");
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
  await putMeta("status", { status, detail, progress, updatedAt: Date.now() });
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
