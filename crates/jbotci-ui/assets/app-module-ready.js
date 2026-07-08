const DEFAULT_TIMEOUT_MS = 30_000;
const DEFAULT_POLL_INTERVAL_MS = 25;
const WASM_READINESS_SIGNALS = "appModule.__wasm or globalThis.__dx_mainWasm";

export async function waitForAppModuleReady(appModule, options = {}) {
  const label = options.label || "Dioxus app module";
  if (!appModule || typeof appModule !== "object") {
    throw new Error(`${label} did not load`);
  }
  if (typeof appModule.jbotciWorkerReady !== "function") {
    throw new Error(`${label} does not export jbotciWorkerReady`);
  }
  await waitForWasmExports(appModule, label, options);
  await appModule.jbotciWorkerReady();
}

async function waitForWasmExports(appModule, label, options) {
  // Dioxus starts wasm-bindgen initialization at module import time without
  // top-level await, so import() can resolve before the generated module has
  // assigned its wasm exports. Depending on the generated bootstrap shape, the
  // readiness signal is either a live module export or Dioxus' split-main global.
  const timeoutMs = positiveNumberOrDefault(options.timeoutMs, DEFAULT_TIMEOUT_MS);
  const pollIntervalMs = positiveNumberOrDefault(
    options.pollIntervalMs,
    DEFAULT_POLL_INTERVAL_MS,
  );
  const deadline = Date.now() + timeoutMs;
  while (!wasmExportsReady(appModule)) {
    if (Date.now() >= deadline) {
      throw new Error(
        `${label} wasm initialization timed out waiting for ${WASM_READINESS_SIGNALS}`,
      );
    }
    await delay(Math.min(pollIntervalMs, Math.max(0, deadline - Date.now())));
  }
}

function wasmExportsReady(appModule) {
  return appModule.__wasm !== undefined || globalThis.__dx_mainWasm !== undefined;
}

function positiveNumberOrDefault(value, fallback) {
  return Number.isFinite(value) && value > 0 ? value : fallback;
}

function delay(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}
