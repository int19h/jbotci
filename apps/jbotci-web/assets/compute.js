let configuredWorkerUrl = null;
let nextRequestId = 1;
const pending = new Map();
const channelRequests = new Map();

function workerUrl() {
  return configuredWorkerUrl ?? new URL("./compute-worker.js", import.meta.url);
}

function addChannelRequest(channel, id) {
  const ids = channelRequests.get(channel) ?? new Set();
  ids.add(id);
  channelRequests.set(channel, ids);
}

function removeChannelRequest(channel, id) {
  const ids = channelRequests.get(channel);
  if (!ids) {
    return;
  }
  ids.delete(id);
  if (ids.size === 0) {
    channelRequests.delete(channel);
  }
}

function finishRequest(id, outcome) {
  const request = pending.get(id);
  if (!request) {
    return;
  }
  pending.delete(id);
  removeChannelRequest(request.channel, id);
  request.worker.terminate();
  if (outcome.ok) {
    request.resolve(outcome.value);
  } else {
    request.reject(outcome.error);
  }
}

export function jbotciComputeConfigureWorker(workerUrlString) {
  if (typeof workerUrlString !== "string" || workerUrlString.length === 0) {
    throw new Error("compute worker URL is empty");
  }
  configuredWorkerUrl = new URL(workerUrlString, globalThis.location.href);
}

export function jbotciComputeCancel(channel) {
  const ids = Array.from(channelRequests.get(channel) ?? []);
  for (const id of ids) {
    const request = pending.get(id);
    if (!request) {
      continue;
    }
    pending.delete(id);
    removeChannelRequest(request.channel, id);
    request.worker.terminate();
    request.reject("compute request cancelled");
  }
}

export function jbotciComputeRequest(channel, requestJson) {
  return new Promise((resolve, reject) => {
    const id = nextRequestId++;
    let worker;
    try {
      worker = new Worker(workerUrl(), { type: "module" });
    } catch (error) {
      reject(error instanceof Error ? error.message : String(error));
      return;
    }
    pending.set(id, { worker, channel, resolve, reject });
    addChannelRequest(channel, id);
    worker.onmessage = (event) => {
      const message = event.data || {};
      finishRequest(message.id, message.ok
        ? { ok: true, value: message.value }
        : { ok: false, error: message.error || "compute worker request failed" });
    };
    worker.onerror = (event) => {
      const location = event.filename
        ? ` at ${event.filename}${event.lineno ? `:${event.lineno}` : ""}`
        : "";
      const error = event.message
        ? `compute worker failed${location}: ${event.message}`
        : `compute worker failed${location}`;
      finishRequest(id, { ok: false, error });
    };
    worker.onmessageerror = () => {
      finishRequest(id, {
        ok: false,
        error: "compute worker returned an unreadable message",
      });
    };
    try {
      worker.postMessage({ id, requestJson });
    } catch (error) {
      finishRequest(id, {
        ok: false,
        error: error instanceof Error ? error.message : String(error),
      });
    }
  });
}
