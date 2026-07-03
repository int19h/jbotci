const DEFAULT_TIMEOUT_MS = 30_000;
const DEFAULT_POLL_INTERVAL_MS = 25;

export async function waitForAppModuleReady(appModule, options = {}) {
  const label = options.label || "Dioxus app module";
  if (!appModule || typeof appModule !== "object") {
    throw new Error(`${label} did not load`);
  }
  if (typeof appModule.jbotciWorkerReady !== "function") {
    throw new Error(`${label} does not export jbotciWorkerReady`);
  }
  await waitForWasmBinding(appModule, label, options);
  await appModule.jbotciWorkerReady();
}

async function waitForWasmBinding(appModule, label, options) {
  if (!("__wasm" in appModule)) {
    throw new Error(`${label} does not export the wasm readiness binding`);
  }
  // Dioxus starts wasm-bindgen initialization at module import time without
  // top-level await, so import() can resolve while the live __wasm binding is
  // still unset. Calling app-owned wasm exports before this poll completes
  // dereferences the uninitialized instance in cold-cache workers.
  const timeoutMs = positiveNumberOrDefault(options.timeoutMs, DEFAULT_TIMEOUT_MS);
  const pollIntervalMs = positiveNumberOrDefault(
    options.pollIntervalMs,
    DEFAULT_POLL_INTERVAL_MS,
  );
  const deadline = Date.now() + timeoutMs;
  while (appModule.__wasm === undefined) {
    if (Date.now() >= deadline) {
      throw new Error(`${label} wasm initialization timed out`);
    }
    await delay(Math.min(pollIntervalMs, Math.max(0, deadline - Date.now())));
  }
}

function positiveNumberOrDefault(value, fallback) {
  return Number.isFinite(value) && value > 0 ? value : fallback;
}

function delay(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}
