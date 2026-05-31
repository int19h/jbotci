const workerModuleUrl = new URL("./generated/jbotci_web_worker.js", import.meta.url);
const wasmUrl = new URL("./generated/jbotci_web_worker_bg.wasm", import.meta.url);
let computeHandle = null;
let initPromise = import(workerModuleUrl.href).then(async (workerModule) => {
  computeHandle = workerModule.jbotciComputeHandle;
  await workerModule.default({ module_or_path: wasmUrl });
});

self.onmessage = async (event) => {
  const { id, requestJson } = event.data || {};
  try {
    await initPromise;
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
