const DEFAULT_REMOTE_BASE_URL = "/assets/embeddings/web/v1";
const LOG_PREFIX = "[jbotci embeddings]";
const EMBEDDING_GEMMA_MODEL_KEY = "embedding-gemma-300m-q4-768";
const F2LLM_MODEL_KEY = "f2llm-v2-80m-q4-320";
const SUPPORTED_MODEL_KEYS = new Set([
  EMBEDDING_GEMMA_MODEL_KEY,
  F2LLM_MODEL_KEY,
]);

let configuredWorkerUrl = null;
let configuredF2LlmRuntimeUrl = null;
let configuredRemoteBaseUrl = DEFAULT_REMOTE_BASE_URL;
let configuredModelKey = null;
let worker = null;
let forceWasm = false;
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

function retryWithWasmError(message) {
  const error = new Error(message);
  error.retryWithWasm = true;
  return error;
}

function shouldRetryWithWasm(error) {
  return typeof error === "object" && error !== null && error.retryWithWasm === true;
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
      request.reject(message.retryWithWasm ? retryWithWasmError(error) : error);
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
  return isAppleMobileDevice() ? F2LLM_MODEL_KEY : EMBEDDING_GEMMA_MODEL_KEY;
}

function activeModelKey() {
  return configuredModelKey || defaultModelKey();
}

function isAppleMobileDevice() {
  const userAgent = globalThis.navigator?.userAgent || "";
  const platform = globalThis.navigator?.userAgentData?.platform
    || globalThis.navigator?.platform
    || "";
  return /\b(iPhone|iPad|iPod)\b/i.test(userAgent)
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
  forceWasm = false;
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
    const requestForceWasm = payload.forceWasm === true || forceWasm;
    const modelKey = payload.modelKey || activeModelKey();
    const f2llmRuntimeUrl = configuredF2LlmRuntimeUrl?.href || null;
    if (type === "setup") {
      logInfo("sending setup request", {
        id,
        modelKey,
        remoteBaseUrl,
        forceWasm: requestForceWasm,
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
          forceWasm: requestForceWasm,
          f2llmRuntimeUrl,
        },
      });
    } catch (error) {
      pending.delete(id);
      reject(error instanceof Error ? error.message : String(error));
    }
  });
}

async function request(type, payload = {}, allowWasmRetry = true) {
  try {
    return await sendRequest(type, payload);
  } catch (error) {
    if (!allowWasmRetry || !shouldRetryWithWasm(error)) {
      throw error;
    }
    forceWasm = true;
    logWarn("restarting embedding worker for CPU/WASM fallback", {
      reason: error.message,
    });
    terminateWorker("embedding worker restarting for CPU/WASM fallback");
    return request(type, payload, false);
  }
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
