let configuredWorkerUrl = null;
let worker = null;
let nextRequestId = 1;
const pending = new Map();

function rejectPending(error) {
  for (const request of pending.values()) {
    request.reject(error);
  }
  pending.clear();
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
      request.reject(message.error || "embedding worker request failed");
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

export function jbotciEmbeddingConfigureWorker(workerUrl) {
  if (typeof workerUrl !== "string" || workerUrl.length === 0) {
    throw new Error("embedding worker URL is empty");
  }
  const nextWorkerUrl = new URL(workerUrl, globalThis.location.href);
  if (configuredWorkerUrl !== null && configuredWorkerUrl.href === nextWorkerUrl.href) {
    return;
  }
  configuredWorkerUrl = nextWorkerUrl;
  if (worker !== null) {
    worker.terminate();
    worker = null;
    rejectPending("embedding worker URL changed");
  }
}

function request(type, payload = {}) {
  return new Promise((resolve, reject) => {
    const id = nextRequestId++;
    pending.set(id, { resolve, reject });
    try {
      ensureWorker().postMessage({ id, type, payload });
    } catch (error) {
      pending.delete(id);
      reject(error instanceof Error ? error.message : String(error));
    }
  });
}

export function jbotciEmbeddingStatus() {
  return request("status");
}

export function jbotciEmbeddingSetup(corpusJson) {
  return request("setup", { corpusJson });
}

export function jbotciEmbeddingRemove() {
  return request("remove");
}

export function jbotciEmbeddingSearch(corpusId, query, limit, kindFiltersJson = "[]") {
  return request("search", { corpusId, query, limit, kindFiltersJson });
}
