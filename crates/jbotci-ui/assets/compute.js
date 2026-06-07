let configuredWorkerUrl = null;
let configuredAppModuleUrl = null;
let nextRequestId = 1;

const MIN_IDLE_WORKERS = 1;
const MAX_IDLE_WORKERS = 3;

const pending = new Map();
const channelRequests = new Map();
const workers = new Set();

function workerUrl() {
  return configuredWorkerUrl ?? new URL("./compute-worker.js", import.meta.url);
}

function appModuleUrl() {
  const bootstrapUrl = globalThis.JBOTCI_WEB_BOOTSTRAP?.mainModuleUrl;
  const url = configuredAppModuleUrl ?? bootstrapUrl;
  if (typeof url !== "string" || url.length === 0) {
    throw new Error("compute app module URL is not configured");
  }
  return new URL(url, globalThis.location.href);
}

function currentWorkerContext() {
  const workerUrlHref = workerUrl().href;
  const mainModuleUrl = appModuleUrl().href;
  return {
    workerUrlHref,
    mainModuleUrl,
    key: JSON.stringify([workerUrlHref, mainModuleUrl]),
  };
}

function errorText(error) {
  return error instanceof Error ? error.message : String(error);
}

function workersForContext(key, state = null) {
  return Array.from(workers).filter((entry) =>
    entry.key === key && (state === null || entry.state === state)
  );
}

function spareWorkersForContext(key) {
  return workersForContext(key).filter((entry) =>
    entry.state === "idle" || entry.state === "warming"
  );
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

function settleWorkerReady(entry, outcome) {
  if (entry.readySettled) {
    return;
  }
  entry.readySettled = true;
  if (outcome.ok) {
    entry.readyResolve();
  } else {
    entry.readyReject(outcome.error);
  }
}

function terminateWorkerEntry(entry, reason = "compute worker terminated") {
  if (entry.state === "terminated") {
    return;
  }
  entry.state = "terminated";
  entry.activeRequestId = null;
  workers.delete(entry);
  settleWorkerReady(entry, { ok: false, error: reason });
  entry.worker.terminate();
}

function terminateAllWorkers(reason) {
  const pendingRequests = Array.from(pending.entries());
  for (const [id, request] of pendingRequests) {
    pending.delete(id);
    removeChannelRequest(request.channel, id);
    terminateWorkerEntry(request.workerEntry, reason);
    request.reject(reason);
  }
  channelRequests.clear();
  for (const entry of Array.from(workers)) {
    terminateWorkerEntry(entry, reason);
  }
}

function failActiveWorker(entry, error) {
  const reason = errorText(error);
  if (entry.activeRequestId === null) {
    terminateWorkerEntry(entry, reason);
    return;
  }
  const id = entry.activeRequestId;
  const request = pending.get(id);
  if (!request) {
    terminateWorkerEntry(entry, reason);
    return;
  }
  pending.delete(id);
  removeChannelRequest(request.channel, id);
  terminateWorkerEntry(entry, reason);
  request.reject(reason);
}

function markWorkerReady(entry) {
  settleWorkerReady(entry, { ok: true });
  if (entry.state === "warming") {
    entry.state = "idle";
    pruneSpareWorkers(entry.key);
  }
}

function handleWorkerMessage(entry, event) {
  const message = event.data || {};
  if (message.kind === "ready") {
    if (message.ok === false) {
      const error = message.error || "compute worker initialization failed";
      settleWorkerReady(entry, { ok: false, error });
      if (entry.state !== "active") {
        terminateWorkerEntry(entry, error);
      }
    } else {
      markWorkerReady(entry);
    }
    return;
  }
  finishRequest(message.id, message.ok
    ? { ok: true, value: message.value }
    : { ok: false, error: message.error || "compute worker request failed" });
}

function workerErrorMessage(event) {
  const location = event.filename
    ? ` at ${event.filename}${event.lineno ? `:${event.lineno}` : ""}`
    : "";
  return event.message
    ? `compute worker failed${location}: ${event.message}`
    : `compute worker failed${location}`;
}

function createWorkerEntry(context) {
  const worker = new Worker(context.workerUrlHref, { type: "module" });
  let readyResolve;
  let readyReject;
  const entry = {
    worker,
    workerUrlHref: context.workerUrlHref,
    mainModuleUrl: context.mainModuleUrl,
    key: context.key,
    state: "warming",
    activeRequestId: null,
    readySettled: false,
    readyPromise: new Promise((resolve, reject) => {
      readyResolve = resolve;
      readyReject = reject;
    }),
    readyResolve,
    readyReject,
  };
  workers.add(entry);
  entry.readyPromise.catch(() => {});
  worker.onmessage = (event) => handleWorkerMessage(entry, event);
  worker.onerror = (event) => failActiveWorker(entry, workerErrorMessage(event));
  worker.onmessageerror = () => {
    failActiveWorker(entry, "compute worker returned an unreadable message");
  };
  try {
    worker.postMessage({ kind: "warm", mainModuleUrl: context.mainModuleUrl });
  } catch (error) {
    terminateWorkerEntry(entry, errorText(error));
    throw error;
  }
  return entry;
}

function pruneSpareWorkers(key) {
  let idle = workersForContext(key, "idle");
  let warming = workersForContext(key, "warming");
  while (idle.length + warming.length > MAX_IDLE_WORKERS) {
    const entry = warming.pop() ?? idle.pop();
    terminateWorkerEntry(entry);
    idle = workersForContext(key, "idle");
    warming = workersForContext(key, "warming");
  }
}

function ensureWarmSpare() {
  let context;
  try {
    context = currentWorkerContext();
  } catch (_) {
    return;
  }
  pruneSpareWorkers(context.key);
  while (spareWorkersForContext(context.key).length < MIN_IDLE_WORKERS) {
    try {
      createWorkerEntry(context);
    } catch (_) {
      break;
    }
    pruneSpareWorkers(context.key);
  }
}

function acquireWorkerForRequest(id) {
  const context = currentWorkerContext();
  pruneSpareWorkers(context.key);
  const entry =
    workersForContext(context.key, "idle")[0]
    ?? workersForContext(context.key, "warming")[0]
    ?? createWorkerEntry(context);
  entry.state = "active";
  entry.activeRequestId = id;
  return entry;
}

function releaseWorker(entry) {
  entry.activeRequestId = null;
  if (entry.workerUrlHref !== workerUrl().href || entry.mainModuleUrl !== appModuleUrl().href) {
    terminateWorkerEntry(entry, "compute worker URL changed");
    ensureWarmSpare();
    return;
  }
  entry.state = "idle";
  pruneSpareWorkers(entry.key);
  ensureWarmSpare();
}

function finishRequest(id, outcome) {
  const request = pending.get(id);
  if (!request) {
    return;
  }
  pending.delete(id);
  removeChannelRequest(request.channel, id);
  releaseWorker(request.workerEntry);
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
  const previousWorkerUrl = workerUrl().href;
  const nextWorkerUrl = new URL(workerUrlString, globalThis.location.href);
  if (previousWorkerUrl !== nextWorkerUrl.href) {
    terminateAllWorkers("compute worker URL changed");
  }
  configuredWorkerUrl = nextWorkerUrl;
  ensureWarmSpare();
}

export function jbotciComputeConfigureAppModule(appModuleUrlString) {
  if (typeof appModuleUrlString !== "string" || appModuleUrlString.length === 0) {
    throw new Error("compute app module URL is empty");
  }
  let previousAppModuleUrl = null;
  try {
    previousAppModuleUrl = appModuleUrl().href;
  } catch (_) {
    previousAppModuleUrl = null;
  }
  const nextAppModuleUrl = new URL(appModuleUrlString, globalThis.location.href).href;
  if (previousAppModuleUrl !== null && previousAppModuleUrl !== nextAppModuleUrl) {
    terminateAllWorkers("compute app module URL changed");
  }
  configuredAppModuleUrl = nextAppModuleUrl;
  ensureWarmSpare();
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
    terminateWorkerEntry(request.workerEntry, "compute request cancelled");
    request.reject("compute request cancelled");
  }
  ensureWarmSpare();
}

export function jbotciComputeRequest(channel, requestJson) {
  return new Promise((resolve, reject) => {
    const id = nextRequestId++;
    let workerEntry;
    try {
      workerEntry = acquireWorkerForRequest(id);
    } catch (error) {
      reject(errorText(error));
      return;
    }
    pending.set(id, { workerEntry, channel, resolve, reject });
    addChannelRequest(channel, id);
    ensureWarmSpare();
    workerEntry.readyPromise
      .then(() => {
        const request = pending.get(id);
        if (!request || request.workerEntry !== workerEntry) {
          return;
        }
        try {
          workerEntry.worker.postMessage({
            id,
            requestJson,
            mainModuleUrl: workerEntry.mainModuleUrl,
          });
        } catch (error) {
          failActiveWorker(workerEntry, error);
        }
      })
      .catch((error) => {
        if (pending.has(id)) {
          failActiveWorker(workerEntry, error);
        }
      });
  });
}
