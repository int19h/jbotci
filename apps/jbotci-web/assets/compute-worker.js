import init, { jbotciComputeHandle } from "./generated/jbotci_web_worker.js";

const wasmUrl = new URL("./generated/jbotci_web_worker_bg.wasm", import.meta.url);
let initPromise = init({ module_or_path: wasmUrl });

self.onmessage = async (event) => {
  const { id, requestJson } = event.data || {};
  try {
    await initPromise;
    const value = jbotciComputeHandle(requestJson || "{}");
    self.postMessage({ id, ok: true, value });
  } catch (error) {
    self.postMessage({
      id,
      ok: false,
      error: error instanceof Error ? error.message : String(error),
    });
  }
};
