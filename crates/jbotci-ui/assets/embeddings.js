import { createWorkerClient } from "./worker-client.js";

const DEFAULT_REMOTE_BASE_URL = "https://assets.jbotci.app/embeddings/web/v1";
const LOG_PREFIX = "[jbotci embeddings]";
const F2LLM_80M_MODEL_KEY = "f2llm-v2-80m-q4-320";
const F2LLM_330M_MODEL_KEY = "f2llm-v2-330m-q4-896";
const SUPPORTED_MODEL_KEYS = new Set([
  F2LLM_80M_MODEL_KEY,
  "f2llm-v2-160m-q4-640",
  F2LLM_330M_MODEL_KEY,
  "f2llm-v2-0.6b-q4-1024",
]);

const CHANNEL_STATUS = "embedding-status";
const CHANNEL_SETUP = "embedding-setup";
const CHANNEL_REMOVE = "embedding-remove";

let configuredOrtModuleUrl = null;
let configuredOrtWasmMjsUrl = null;
let configuredOrtWasmUrl = null;
let configuredRemoteBaseUrl = DEFAULT_REMOTE_BASE_URL;
let configuredModelKey = null;

function logInfo(message, detail = null) {
  if (detail === null) {
    console.info(`${LOG_PREFIX} ${message}`);
  } else {
    console.info(`${LOG_PREFIX} ${message}`, detail);
  }
}

function activeModelKey() {
  return configuredModelKey || defaultModelKey();
}

function defaultModelKey() {
  return isMobileDevice() ? F2LLM_80M_MODEL_KEY : F2LLM_330M_MODEL_KEY;
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
  return {
    modelKey: activeModelKey(),
    remoteBaseUrl: configuredRemoteBaseUrl,
    ortModuleUrl: configuredOrtModuleUrl?.href || null,
    ortWasmMjsUrl: configuredOrtWasmMjsUrl?.href || null,
    ortWasmUrl: configuredOrtWasmUrl?.href || null,
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
      modelKey: context.config.modelKey,
      remoteBaseUrl: context.config.remoteBaseUrl,
      ortModuleUrl: context.config.ortModuleUrl,
      ortWasmMjsUrl: context.config.ortWasmMjsUrl,
      ortWasmUrl: context.config.ortWasmUrl,
    },
  }),
  requestMessage: ({ id, payload, workerEntry }) => ({
    id,
    type: payload.type,
    payload: {
      ...payload.payload,
      modelKey: workerEntry.config.modelKey,
      mainModuleUrl: workerEntry.mainModuleUrl,
      ortModuleUrl: workerEntry.config.ortModuleUrl,
      ortWasmMjsUrl: workerEntry.config.ortWasmMjsUrl,
      ortWasmUrl: workerEntry.config.ortWasmUrl,
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

export function jbotciEmbeddingConfigureModel(modelKey) {
  if (typeof modelKey !== "string" || modelKey.trim().length === 0) {
    throw new Error("embedding model key is empty");
  }
  const nextModelKey = modelKey.trim();
  if (!SUPPORTED_MODEL_KEYS.has(nextModelKey)) {
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
