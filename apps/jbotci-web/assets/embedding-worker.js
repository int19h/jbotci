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
const REMOTE_BASE = "/assets/embeddings/v1";
const LOCAL_PACK_ID = "egemma-v0-parity-1-browser-local";
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

self.onmessage = async (event) => {
  const { id, type, payload } = event.data || {};
  try {
    let value;
    if (type === "status") {
      value = await status();
    } else if (type === "setup") {
      value = await setup(payload?.corpusJson || "{}");
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
    self.postMessage({
      id,
      ok: false,
      error: error instanceof Error ? error.message : String(error),
    });
  }
};

async function setup(corpusJson) {
  if (setupInProgress) {
    return status();
  }
  setupInProgress = true;
  try {
    const corpus = JSON.parse(corpusJson);
    await requestPersistentStorage();
    await checkQuota();
    await updateStatus("checking", "Looking for a same-origin vector pack.");
    const remoteLoaded = await loadRemotePackIfAvailable(corpus);
    await updateStatus("loading-model", "Downloading or opening EmbeddingGemma Q4.");
    await ensureModel();
    if (!remoteLoaded) {
      await buildLocalPack(corpus);
    }
    await updateStatus("ready", remoteLoaded
      ? "Using cached same-origin vector pack with local query embeddings."
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
  let pack = await getMeta("pack");
  if (!pack) {
    await loadRemotePackIfAvailable();
    pack = await getMeta("pack");
  }
  if (!pack) {
    return {
      hits: [],
      message: "Open Settings and download embeddings before using meaning search.",
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
  await ensureModel();
  const queryEmbedding = await embedTexts([QUERY_PREFIX + trimmedQuery]);
  const vectors = await readCorpusVectors(corpus);
  const hits = rankHits(vectors, queryEmbedding[0], corpus.items, limit, kindFilters);
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
  const modelKey = corpus.modelKey || corpus.model_key || corpus["model-key"];
  if (modelKey !== MODEL_KEY) {
    throw new Error(`unsupported browser corpus model key: ${modelKey || "missing"}`);
  }
  const corpora = {};
  corpora["vlacku-en"] = await buildLocalCorpus("vlacku-en", corpus.dictionary || []);
  corpora["cukta-cll"] = await buildLocalCorpus("cukta-cll", corpus.cll || []);
  await putMeta("pack", {
    source: "browser",
    packId: LOCAL_PACK_ID,
    modelKey: MODEL_KEY,
    corpora,
  });
}

function corpusKindLookups(corpus) {
  return {
    "vlacku-en": documentKindLookup(corpus?.dictionary || []),
    "cukta-cll": documentKindLookup(corpus?.cll || []),
  };
}

function documentKindLookup(docs) {
  const lookup = new Map();
  for (const doc of docs) {
    const id = Number(doc?.id);
    const kind = normalizeWordTypeFilter(doc?.kind || "");
    if (Number.isInteger(id) && kind.length > 0) {
      lookup.set(id, kind);
    }
  }
  return lookup;
}

async function buildLocalCorpus(corpusId, docs) {
  await updateStatus(
    "indexing",
    `Building ${corpusId} embeddings in this browser.`,
    progressValue("index", corpusId, 0, docs.length),
  );
  const embeddings = await embedTexts(
    docs.map((doc) => doc.input),
    { label: corpusId },
  );
  const vectors = new Float32Array(embeddings.length * DIMENSIONS);
  for (let row = 0; row < embeddings.length; row += 1) {
    vectors.set(embeddings[row], row * DIMENSIONS);
  }
  const vectorKey = `local/${MODEL_KEY}/${LOCAL_PACK_ID}/${corpusId}/vectors-0000.f32`;
  await putBinary(vectorKey, vectors.buffer);
  return {
    corpusId,
    rowCount: docs.length,
    dimensions: DIMENSIONS,
    items: docs.map((doc, row) => ({
      id: doc.id,
      row,
      kind: normalizeWordTypeFilter(doc.kind || "") || null,
    })),
    shards: [{ key: vectorKey, byteLen: vectors.byteLength }],
  };
}

async function loadRemotePackIfAvailable(corpus = null) {
  const catalog = await fetchJsonIfAvailable(`${REMOTE_BASE}/catalog.json`);
  if (catalog === null) {
    return false;
  }
  const model = (catalog.models || []).find((item) => item.model_key === MODEL_KEY);
  if (!model?.manifest_url) {
    return false;
  }
  const manifestUrl = `${REMOTE_BASE}/${model.manifest_url}`;
  const manifest = await fetchJsonIfAvailable(manifestUrl);
  if (manifest === null) {
    return false;
  }
  if (!runtimeCompatible(manifest)) {
    return false;
  }
  const packBase = manifestUrl.replace(/\/manifest\.json$/, "");
  const totalVectorBytes = remotePackVectorBytes(manifest);
  let downloadedVectorBytes = 0;
  const corpora = {};
  const kindLookups = corpusKindLookups(corpus);
  for (const corpusManifest of manifest.corpora || []) {
    await updateStatus(
      "downloading-index",
      `Downloading ${corpusManifest.corpus_id} vector pack.`,
      progressValue("index", corpusManifest.corpus_id, downloadedVectorBytes, totalVectorBytes),
    );
    const items = await fetchJson(`${packBase}/${corpusManifest.items_url}`);
    const shards = [];
    for (const shard of corpusManifest.shards || []) {
      const shardUrl = `${packBase}/${shard.url}`;
      const bytes = await fetchArrayBuffer(shardUrl);
      if (bytes.byteLength !== shard.byte_len && bytes.byteLength !== shard.byteLen) {
        throw new Error(`remote vector shard ${shard.url} has the wrong size`);
      }
      const key = `remote/${MODEL_KEY}/${manifest.pack_id}/${corpusManifest.corpus_id}/${shard.url}`;
      await putBinary(key, bytes);
      downloadedVectorBytes += bytes.byteLength;
      await updateStatus(
        "downloading-index",
        `Downloaded ${corpusManifest.corpus_id} vector shard ${shard.url}.`,
        progressValue("index", corpusManifest.corpus_id, downloadedVectorBytes, totalVectorBytes),
      );
      shards.push({ key, byteLen: bytes.byteLength });
    }
    const kindLookup = kindLookups[corpusManifest.corpus_id] || new Map();
    corpora[corpusManifest.corpus_id] = {
      corpusId: corpusManifest.corpus_id,
      rowCount: corpusManifest.row_count,
      dimensions: corpusManifest.dimensions,
      items: items.map((item, row) => {
        const id = item.entry_index ?? item.chunk_index;
        return {
          id,
          row,
          kind: normalizeWordTypeFilter(item.kind || kindLookup.get(id) || "") || null,
        };
      }),
      shards,
    };
  }
  await putMeta("pack", {
    source: "remote",
    packId: manifest.pack_id,
    modelKey: MODEL_KEY,
    corpora,
  });
  return true;
}

function runtimeCompatible(manifest) {
  if (manifest.schema_version !== 1 || manifest.model_key !== MODEL_KEY) {
    return false;
  }
  return (manifest.compatible_query_runtimes || []).some((runtime) =>
    runtime.runtime === "transformers.js"
  );
}

function remotePackVectorBytes(manifest) {
  let total = 0;
  for (const corpus of manifest.corpora || []) {
    for (const shard of corpus.shards || []) {
      total += shard.byte_len || shard.byteLen || 0;
    }
  }
  return total;
}

async function fetchJsonIfAvailable(url) {
  const response = await fetch(url, { cache: "no-cache" }).catch(() => null);
  if (!response?.ok) {
    return null;
  }
  const text = await response.text();
  if (looksLikeHtmlResponse(response, text)) {
    return null;
  }
  try {
    return JSON.parse(text);
  } catch (_) {
    return null;
  }
}

async function fetchJson(url) {
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error(`failed to fetch ${url}: ${response.status}`);
  }
  const text = await response.text();
  if (looksLikeHtmlResponse(response, text)) {
    throw new Error(`expected JSON from ${url}, got HTML instead`);
  }
  try {
    return JSON.parse(text);
  } catch (error) {
    throw new Error(`invalid JSON from ${url}: ${errorMessage(error)}`);
  }
}

async function fetchArrayBuffer(url) {
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error(`failed to fetch ${url}: ${response.status}`);
  }
  return response.arrayBuffer();
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
  return new Float32Array(combined.buffer);
}

function rankHits(vectors, query, items, limit, kindFilters) {
  const rowCount = Math.min(items.length, Math.floor(vectors.length / DIMENSIONS));
  const hits = [];
  for (let row = 0; row < rowCount; row += 1) {
    if (!itemMatchesKindFilters(items[row], kindFilters)) {
      continue;
    }
    let score = 0;
    const base = row * DIMENSIONS;
    for (let dim = 0; dim < DIMENSIONS; dim += 1) {
      score += vectors[base + dim] * query[dim];
    }
    hits.push({ id: items[row].id, score });
  }
  hits.sort((left, right) => right.score - left.score || left.id - right.id);
  return limit > 0 ? hits.slice(0, limit) : hits;
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
  const minimum = await hasUsableWebGpu() ? Q4_MIN_FREE_BYTES : Q8_MIN_FREE_BYTES;
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
