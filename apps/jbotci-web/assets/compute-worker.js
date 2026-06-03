let computeHandle = null;
let initModuleUrl = null;
let initPromise = null;

const INIT_TIMEOUT_MS = 30000;

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function waitForMainWasm(appModule) {
  const startedAt = Date.now();
  while (Date.now() - startedAt < INIT_TIMEOUT_MS) {
    if (appModule.__wasm !== undefined || globalThis.__dx_mainWasm !== undefined) {
      return;
    }
    await sleep(10);
  }
  throw new Error("Dioxus app wasm initialization did not complete in the compute worker");
}

function initCompute(mainModuleUrl) {
  if (typeof mainModuleUrl !== "string" || mainModuleUrl.length === 0) {
    throw new Error("compute worker did not receive the app module URL");
  }
  const moduleUrl = new URL(mainModuleUrl, self.location.href).href;
  if (initPromise !== null && initModuleUrl === moduleUrl) {
    return initPromise;
  }
  initModuleUrl = moduleUrl;
  computeHandle = null;
  initPromise = import(moduleUrl).then(async (appModule) => {
    if (typeof appModule.jbotciComputeHandle !== "function") {
      throw new Error("Dioxus app module does not export jbotciComputeHandle");
    }
    await waitForMainWasm(appModule);
    computeHandle = appModule.jbotciComputeHandle;
  });
  return initPromise;
}

self.onmessage = async (event) => {
  const { kind, id, requestJson, mainModuleUrl } = event.data || {};
  if (kind === "warm") {
    try {
      await initCompute(mainModuleUrl);
      self.postMessage({ kind: "ready", ok: true });
    } catch (error) {
      self.postMessage({
        kind: "ready",
        ok: false,
        error: error instanceof Error ? error.message : String(error),
      });
    }
    return;
  }
  try {
    await initCompute(mainModuleUrl);
    const value = computeHandle(requestJson || "{}");
    self.postMessage({ id, ok: true, value });
  } catch (error) {
    self.postMessage({
      id,
      ok: false,
      error: error instanceof Error ? error.message : String(error),
    });
  }
};
