import { createWorkerClient } from "./worker-client.js";

const DEFAULT_REMOTE_BASE_URL = "https://assets.jbotci.app/embeddings/web/v1";
const LOG_PREFIX = "[jbotci embeddings]";
const DEBUG_STORAGE_KEY = "jbotci.embedding.debug";

const CHANNEL_STATUS = "embedding-status";
const CHANNEL_SETUP = "embedding-setup";
const CHANNEL_REMOVE = "embedding-remove";

let configuredCatalog = null;
let configuredCatalogKey = null;
let configuredOrtModuleUrl = null;
let configuredOrtWasmMjsUrl = null;
let configuredOrtWasmUrl = null;
let configuredRemoteBaseUrl = DEFAULT_REMOTE_BASE_URL;
let configuredModelKey = null;

function logInfo(message, detail = null) {
  if (!debugLoggingEnabled()) {
    return;
  }
  if (detail === null) {
    console.info(`${LOG_PREFIX} ${message}`);
  } else {
    console.info(`${LOG_PREFIX} ${message}`, detail);
  }
}

function debugLoggingEnabled() {
  if (globalThis.JBOTCI_EMBEDDING_DEBUG === true) {
    return true;
  }
  try {
    if (globalThis.localStorage?.getItem(DEBUG_STORAGE_KEY) === "1") {
      return true;
    }
  } catch (_) {
    // Ignore storage failures; this only controls optional diagnostics.
  }
  try {
    return new URL(globalThis.location?.href || "http://localhost/")
      .searchParams
      .get("jbotci-embedding-debug") === "1";
  } catch (_) {
    return false;
  }
}

function activeModelKey() {
  return configuredModelKey || defaultModelKey();
}

function defaultModelKey() {
  const catalog = requireCatalog();
  return isMobileDevice() ? catalog.defaultMobileModelKey : catalog.defaultDesktopModelKey;
}

function isMobileDevice() {
  const userAgent = globalThis.navigator?.userAgent || "";
  const platform = globalThis.navigator?.userAgentData?.platform
    || globalThis.navigator?.platform
    || "";
  return /\b(Android|iPhone|iPad|iPod|Mobile)\b/i.test(userAgent)
    || (platform === "MacIntel" && Number(globalThis.navigator?.maxTouchPoints || 0) > 1);
}

function workerConfig() {
  const catalog = requireCatalog();
  return {
    modelKey: activeModelKey(),
    modelCatalog: catalog,
    catalogKey: configuredCatalogKey,
    remoteBaseUrl: configuredRemoteBaseUrl,
    ortModuleUrl: configuredOrtModuleUrl?.href || null,
    ortWasmMjsUrl: configuredOrtWasmMjsUrl?.href || null,
    ortWasmUrl: configuredOrtWasmUrl?.href || null,
    debug: debugLoggingEnabled(),
    minIdleWorkers: 0,
    maxIdleWorkers: 1,
  };
}

const client = createWorkerClient({
  label: "embedding",
  defaultWorkerUrl: () => new URL("./embedding-worker.js", import.meta.url),
  minIdleWorkers: 0,
  maxIdleWorkers: 1,
  workerConfig,
  contextKey: (config) => [
    config.catalogKey,
    config.modelKey,
    config.ortModuleUrl,
    config.ortWasmMjsUrl,
    config.ortWasmUrl,
  ],
  responseValue: (value) => JSON.stringify(value),
  warmMessage: (context) => ({
    kind: "warm",
    mainModuleUrl: context.mainModuleUrl,
    payload: {
      modelCatalog: context.config.modelCatalog,
      modelKey: context.config.modelKey,
      remoteBaseUrl: context.config.remoteBaseUrl,
      ortModuleUrl: context.config.ortModuleUrl,
      ortWasmMjsUrl: context.config.ortWasmMjsUrl,
      ortWasmUrl: context.config.ortWasmUrl,
      debug: context.config.debug,
    },
  }),
  requestMessage: ({ id, payload, workerEntry }) => ({
    id,
    type: payload.type,
    payload: {
      ...payload.payload,
      modelCatalog: workerEntry.config.modelCatalog,
      modelKey: workerEntry.config.modelKey,
      mainModuleUrl: workerEntry.mainModuleUrl,
      ortModuleUrl: workerEntry.config.ortModuleUrl,
      ortWasmMjsUrl: workerEntry.config.ortWasmMjsUrl,
      ortWasmUrl: workerEntry.config.ortWasmUrl,
      debug: workerEntry.config.debug,
    },
  }),
});

export function jbotciEmbeddingConfigureWorker(workerUrl) {
  client.configureWorker(workerUrl);
  logInfo("configured worker URL", {
    workerUrl: new URL(workerUrl, globalThis.location.href).href,
  });
}

export function jbotciEmbeddingConfigureOrtAssets(moduleUrl, wasmMjsUrl, wasmUrl) {
  if (typeof moduleUrl !== "string" || moduleUrl.length === 0) {
    throw new Error("ONNX Runtime Web module URL is empty");
  }
  if (typeof wasmMjsUrl !== "string" || wasmMjsUrl.length === 0) {
    throw new Error("ONNX Runtime Web wasm loader URL is empty");
  }
  if (typeof wasmUrl !== "string" || wasmUrl.length === 0) {
    throw new Error("ONNX Runtime Web wasm URL is empty");
  }
  const nextModuleUrl = new URL(moduleUrl, globalThis.location.href);
  const nextWasmMjsUrl = new URL(wasmMjsUrl, globalThis.location.href);
  const nextWasmUrl = new URL(wasmUrl, globalThis.location.href);
  if (
    configuredOrtModuleUrl !== null
    && configuredOrtModuleUrl.href === nextModuleUrl.href
    && configuredOrtWasmMjsUrl.href === nextWasmMjsUrl.href
    && configuredOrtWasmUrl.href === nextWasmUrl.href
  ) {
    return;
  }
  configuredOrtModuleUrl = nextModuleUrl;
  configuredOrtWasmMjsUrl = nextWasmMjsUrl;
  configuredOrtWasmUrl = nextWasmUrl;
  client.terminateAllWorkers("embedding ONNX Runtime Web assets changed");
  logInfo("configured ONNX Runtime Web assets", {
    moduleUrl: configuredOrtModuleUrl.href,
    wasmMjsUrl: configuredOrtWasmMjsUrl.href,
    wasmUrl: configuredOrtWasmUrl.href,
  });
}

export function jbotciEmbeddingConfigureRemoteBase(remoteBaseUrl) {
  if (typeof remoteBaseUrl !== "string" || remoteBaseUrl.trim().length === 0) {
    throw new Error("embedding remote base URL is empty");
  }
  const trimmed = remoteBaseUrl.trim();
  const normalized = trimmed.length > 1 ? trimmed.replace(/\/+$/, "") : trimmed;
  configuredRemoteBaseUrl = normalized || DEFAULT_REMOTE_BASE_URL;
  logInfo("configured remote base URL", { remoteBaseUrl: configuredRemoteBaseUrl });
}

export function jbotciEmbeddingConfigureCatalog(catalogJson) {
  if (typeof catalogJson !== "string" || catalogJson.trim().length === 0) {
    throw new Error("embedding model catalog JSON is empty");
  }
  let parsed;
  try {
    parsed = JSON.parse(catalogJson);
  } catch (error) {
    throw new Error(`invalid embedding model catalog JSON: ${errorMessage(error)}`);
  }
  const catalog = validateModelCatalog(parsed);
  const catalogKey = JSON.stringify(catalog);
  if (catalogKey === configuredCatalogKey) {
    return;
  }
  configuredCatalog = catalog;
  configuredCatalogKey = catalogKey;
  if (configuredModelKey !== null && !catalog.models[configuredModelKey]) {
    configuredModelKey = null;
  }
  client.terminateAllWorkers("embedding model catalog changed");
  logInfo("configured model catalog", {
    modelKeys: Object.keys(catalog.models),
    defaultMobileModelKey: catalog.defaultMobileModelKey,
    defaultDesktopModelKey: catalog.defaultDesktopModelKey,
  });
}

export function jbotciEmbeddingConfigureModel(modelKey) {
  if (typeof modelKey !== "string" || modelKey.trim().length === 0) {
    throw new Error("embedding model key is empty");
  }
  const nextModelKey = modelKey.trim();
  if (!requireCatalog().models[nextModelKey]) {
    throw new Error(`unsupported embedding model key: ${nextModelKey}`);
  }
  if (configuredModelKey === nextModelKey) {
    return;
  }
  configuredModelKey = nextModelKey;
  client.terminateAllWorkers("embedding model changed");
  logInfo("configured model", { modelKey: configuredModelKey });
}

export function jbotciEmbeddingPreferredModelKey() {
  return activeModelKey();
}

export function jbotciEmbeddingCancel(channel) {
  client.cancel(channel);
}

function request(channel, type, payload = {}) {
  const requestPayload = {
    ...payload,
    remoteBaseUrl: payload.remoteBaseUrl || configuredRemoteBaseUrl,
  };
  if (type === "setup") {
    logInfo("sending setup request", {
      modelKey: activeModelKey(),
      remoteBaseUrl: requestPayload.remoteBaseUrl,
      corpusJsonBytes: typeof requestPayload.corpusJson === "string"
        ? requestPayload.corpusJson.length
        : 0,
    });
  }
  return client.request(channel, { type, payload: requestPayload });
}

export function jbotciEmbeddingStatus() {
  return request(CHANNEL_STATUS, "status", {
    setupActive: client.hasPending(CHANNEL_SETUP),
  });
}

export function jbotciEmbeddingSetup(corpusJson, remoteBaseUrl = configuredRemoteBaseUrl) {
  return request(CHANNEL_SETUP, "setup", { corpusJson, remoteBaseUrl });
}

export function jbotciEmbeddingRemove() {
  return request(CHANNEL_REMOVE, "remove");
}

export function jbotciEmbeddingSearch(channel, corpusId, query, limit, kindFiltersJson = "[]") {
  return request(channel, "search", { corpusId, query, limit, kindFiltersJson });
}

function requireCatalog() {
  if (configuredCatalog === null) {
    throw new Error("embedding model catalog has not been configured");
  }
  return configuredCatalog;
}

function validateModelCatalog(raw) {
  if (!raw || typeof raw !== "object" || Array.isArray(raw)) {
    throw new Error("embedding model catalog must be an object");
  }
  if (raw.schemaVersion !== 1) {
    throw new Error(`unsupported embedding model catalog schema version: ${raw.schemaVersion}`);
  }
  const models = raw.models;
  if (!models || typeof models !== "object" || Array.isArray(models)) {
    throw new Error("embedding model catalog models must be an object");
  }
  const normalizedModels = {};
  for (const [key, spec] of Object.entries(models)) {
    normalizedModels[key] = validateModelSpec(key, spec);
  }
  for (const field of ["defaultMobileModelKey", "defaultDesktopModelKey", "wasmFallbackModelKey"]) {
    if (typeof raw[field] !== "string" || !normalizedModels[raw[field]]) {
      throw new Error(`embedding model catalog ${field} must name a configured model`);
    }
  }
  const fallback = normalizedModels[raw.wasmFallbackModelKey];
  if (!fallback.wasmRuntime?.onnxUrl) {
    throw new Error("embedding model catalog wasmFallbackModelKey must provide a wasmRuntime.onnxUrl");
  }
  return {
    schemaVersion: 1,
    defaultMobileModelKey: raw.defaultMobileModelKey,
    defaultDesktopModelKey: raw.defaultDesktopModelKey,
    wasmFallbackModelKey: raw.wasmFallbackModelKey,
    models: normalizedModels,
  };
}

function validateModelSpec(key, spec) {
  if (!spec || typeof spec !== "object" || Array.isArray(spec)) {
    throw new Error(`embedding model ${key} must be an object`);
  }
  const modelKey = requiredString(spec, "modelKey", `embedding model ${key}`);
  if (modelKey !== key) {
    throw new Error(`embedding model ${key} modelKey mismatch: ${modelKey}`);
  }
  const customRuntime = requiredObject(spec, "customRuntime", `embedding model ${key}`);
  const preferredRuntime = requiredObject(spec, "preferredRuntime", `embedding model ${key}`);
  const normalized = {
    modelKey,
    label: requiredString(spec, "label", `embedding model ${key}`),
    modelId: requiredString(spec, "modelId", `embedding model ${key}`),
    customRuntime: {
      runtime: requiredString(customRuntime, "runtime", `embedding model ${key} customRuntime`),
      version: requiredString(customRuntime, "version", `embedding model ${key} customRuntime`),
      artifactBaseUrl: requiredString(customRuntime, "artifactBaseUrl", `embedding model ${key} customRuntime`),
      dtype: requiredString(customRuntime, "dtype", `embedding model ${key} customRuntime`),
      device: requiredString(customRuntime, "device", `embedding model ${key} customRuntime`),
    },
    preferredRuntime: {
      dtype: requiredString(preferredRuntime, "dtype", `embedding model ${key} preferredRuntime`),
      device: requiredString(preferredRuntime, "device", `embedding model ${key} preferredRuntime`),
    },
    dimensions: requiredPositiveInteger(spec, "dimensions", `embedding model ${key}`),
    maxSequenceLength: requiredPositiveInteger(spec, "maxSequenceLength", `embedding model ${key}`),
    queryPrefix: requiredString(spec, "queryPrefix", `embedding model ${key}`),
    remoteVectorPacks: spec.remoteVectorPacks === true,
    browserLocalIndexing: spec.browserLocalIndexing !== false,
    localVectorSpaceKey: requiredString(spec, "localVectorSpaceKey", `embedding model ${key}`),
    vectorElementType: requiredString(spec, "vectorElementType", `embedding model ${key}`),
    embedBatchSize: requiredPositiveInteger(spec, "embedBatchSize", `embedding model ${key}`),
    modelSizeEstimates: requiredObject(spec, "modelSizeEstimates", `embedding model ${key}`),
    minFreeBytesByDtype: requiredObject(spec, "minFreeBytesByDtype", `embedding model ${key}`),
    outputPooling: requiredString(spec, "outputPooling", `embedding model ${key}`),
  };
  if (spec.wasmRuntime !== undefined && spec.wasmRuntime !== null) {
    const wasmRuntime = requiredObject(spec, "wasmRuntime", `embedding model ${key}`);
    normalized.wasmRuntime = {
      runtime: requiredString(wasmRuntime, "runtime", `embedding model ${key} wasmRuntime`),
      version: requiredString(wasmRuntime, "version", `embedding model ${key} wasmRuntime`),
      onnxUrl: requiredString(wasmRuntime, "onnxUrl", `embedding model ${key} wasmRuntime`),
      dtype: requiredString(wasmRuntime, "dtype", `embedding model ${key} wasmRuntime`),
      device: requiredString(wasmRuntime, "device", `embedding model ${key} wasmRuntime`),
    };
  }
  return normalized;
}

function requiredObject(value, field, label) {
  const fieldValue = value?.[field];
  if (!fieldValue || typeof fieldValue !== "object" || Array.isArray(fieldValue)) {
    throw new Error(`${label}.${field} must be an object`);
  }
  return fieldValue;
}

function requiredString(value, field, label) {
  const fieldValue = value?.[field];
  if (typeof fieldValue !== "string" || fieldValue.trim().length === 0) {
    throw new Error(`${label}.${field} must be a non-empty string`);
  }
  return fieldValue;
}

function requiredPositiveInteger(value, field, label) {
  const fieldValue = value?.[field];
  if (!Number.isInteger(fieldValue) || fieldValue <= 0) {
    throw new Error(`${label}.${field} must be a positive integer`);
  }
  return fieldValue;
}

function errorMessage(error) {
  return error instanceof Error ? error.message : String(error);
}
