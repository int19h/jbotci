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

let configuredWorkerUrl = null;
let configuredF2LlmRuntimeUrl = null;
let configuredOrtModuleUrl = null;
let configuredOrtWasmMjsUrl = null;
let configuredOrtWasmUrl = null;
let configuredRemoteBaseUrl = DEFAULT_REMOTE_BASE_URL;
let configuredModelKey = null;
let worker = null;
let nextRequestId = 1;
const pending = new Map();

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

function rejectPending(error) {
  for (const request of pending.values()) {
    request.reject(error);
  }
  pending.clear();
}

function terminateWorker(error) {
  if (worker !== null) {
    worker.terminate();
    worker = null;
  }
  rejectPending(error);
}

function ensureWorker() {
  if (worker !== null) {
    return worker;
  }
  const workerUrl =
    configuredWorkerUrl ?? new URL("./embedding-worker.js", import.meta.url);
  try {
    worker = new Worker(workerUrl, { type: "module" });
  } catch (error) {
    worker = null;
    throw new Error(error instanceof Error ? error.message : String(error));
  }
  worker.onmessage = (event) => {
    const message = event.data || {};
    const request = pending.get(message.id);
    if (!request) {
      return;
    }
    pending.delete(message.id);
    if (message.ok) {
      request.resolve(JSON.stringify(message.value));
    } else {
      const error = message.error || "embedding worker request failed";
      logWarn("worker request failed", {
        id: message.id,
        error,
      });
      request.reject(error);
    }
  };
  worker.onerror = (event) => {
    const location = event.filename
      ? ` at ${event.filename}${event.lineno ? `:${event.lineno}` : ""}`
      : "";
    const error = event.message
      ? `embedding worker failed${location}: ${event.message}`
      : `embedding worker failed${location}`;
    rejectPending(error);
    worker = null;
  };
  worker.onmessageerror = () => {
    rejectPending("embedding worker returned an unreadable message");
    worker = null;
  };
  return worker;
}

function defaultModelKey() {
  return isMobileDevice() ? F2LLM_80M_MODEL_KEY : F2LLM_330M_MODEL_KEY;
}

function activeModelKey() {
  return configuredModelKey || defaultModelKey();
}

function isMobileDevice() {
  const userAgent = globalThis.navigator?.userAgent || "";
  const platform = globalThis.navigator?.userAgentData?.platform
    || globalThis.navigator?.platform
    || "";
  return /\b(Android|iPhone|iPad|iPod|Mobile)\b/i.test(userAgent)
    || (platform === "MacIntel" && Number(globalThis.navigator?.maxTouchPoints || 0) > 1);
}

export function jbotciEmbeddingConfigureWorker(workerUrl) {
  if (typeof workerUrl !== "string" || workerUrl.length === 0) {
    throw new Error("embedding worker URL is empty");
  }
  const nextWorkerUrl = new URL(workerUrl, globalThis.location.href);
  if (configuredWorkerUrl !== null && configuredWorkerUrl.href === nextWorkerUrl.href) {
    return;
  }
  configuredWorkerUrl = nextWorkerUrl;
  logInfo("configured worker URL", { workerUrl: configuredWorkerUrl.href });
  if (worker !== null) {
    terminateWorker("embedding worker URL changed");
  }
}

export function jbotciEmbeddingConfigureF2LlmRuntime(runtimeUrl) {
  if (typeof runtimeUrl !== "string" || runtimeUrl.length === 0) {
    throw new Error("F2LLM WebGPU runtime URL is empty");
  }
  const nextRuntimeUrl = new URL(runtimeUrl, globalThis.location.href);
  if (configuredF2LlmRuntimeUrl !== null && configuredF2LlmRuntimeUrl.href === nextRuntimeUrl.href) {
    return;
  }
  configuredF2LlmRuntimeUrl = nextRuntimeUrl;
  logInfo("configured F2LLM WebGPU runtime URL", { runtimeUrl: configuredF2LlmRuntimeUrl.href });
  if (worker !== null) {
    terminateWorker("F2LLM WebGPU runtime URL changed");
  }
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
  logInfo("configured ONNX Runtime Web assets", {
    moduleUrl: configuredOrtModuleUrl.href,
    wasmMjsUrl: configuredOrtWasmMjsUrl.href,
    wasmUrl: configuredOrtWasmUrl.href,
  });
  if (worker !== null) {
    terminateWorker("ONNX Runtime Web assets changed");
  }
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
  logInfo("configured model", { modelKey: configuredModelKey });
  if (worker !== null) {
    terminateWorker("embedding model changed");
  }
}

export function jbotciEmbeddingPreferredModelKey() {
  return activeModelKey();
}

function sendRequest(type, payload = {}) {
  return new Promise((resolve, reject) => {
    const id = nextRequestId++;
    const remoteBaseUrl = payload.remoteBaseUrl || configuredRemoteBaseUrl;
    const modelKey = payload.modelKey || activeModelKey();
    const f2llmRuntimeUrl = configuredF2LlmRuntimeUrl?.href || null;
    const ortModuleUrl = configuredOrtModuleUrl?.href || null;
    const ortWasmMjsUrl = configuredOrtWasmMjsUrl?.href || null;
    const ortWasmUrl = configuredOrtWasmUrl?.href || null;
    if (type === "setup") {
      logInfo("sending setup request", {
        id,
        modelKey,
        remoteBaseUrl,
        corpusJsonBytes: typeof payload.corpusJson === "string" ? payload.corpusJson.length : 0,
      });
    }
    pending.set(id, { resolve, reject });
    try {
      ensureWorker().postMessage({
        id,
        type,
        payload: {
          ...payload,
          modelKey,
          remoteBaseUrl,
          f2llmRuntimeUrl,
          ortModuleUrl,
          ortWasmMjsUrl,
          ortWasmUrl,
        },
      });
    } catch (error) {
      pending.delete(id);
      reject(error instanceof Error ? error.message : String(error));
    }
  });
}

async function request(type, payload = {}) {
  return sendRequest(type, payload);
}

export function jbotciEmbeddingStatus() {
  return request("status");
}

export function jbotciEmbeddingSetup(corpusJson, remoteBaseUrl = configuredRemoteBaseUrl) {
  return request("setup", { corpusJson, remoteBaseUrl });
}

export function jbotciEmbeddingRemove() {
  return request("remove");
}

export function jbotciEmbeddingSearch(corpusId, query, limit, kindFiltersJson = "[]") {
  return request("search", { corpusId, query, limit, kindFiltersJson });
}
