let computeHandle = null;
let initModuleUrl = null;
let initPromise = null;
let appModuleReadyModulePromise = null;

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
  appModuleReadyModulePromise = null;
  initPromise = import(moduleUrl).then(async (appModule) => {
    if (typeof appModule.jbotciComputeHandle !== "function") {
      throw new Error("Dioxus app module does not export jbotciComputeHandle");
    }
    const { waitForAppModuleReady } = await appModuleReadyModule();
    await waitForAppModuleReady(appModule);
    computeHandle = appModule.jbotciComputeHandle;
  });
  return initPromise;
}

function appModuleReadyModule() {
  if (appModuleReadyModulePromise === null) {
    appModuleReadyModulePromise = import(versionedSiblingModuleUrl(
      "app-module-ready.js",
      initModuleUrl,
    ));
  }
  return appModuleReadyModulePromise;
}

function versionedSiblingModuleUrl(moduleName, versionSourceUrl) {
  const url = new URL(moduleName, import.meta.url);
  const versionSource = new URL(versionSourceUrl, self.location.href);
  url.searchParams.set(
    "jbotci-app",
    versionSource.pathname.split("/").pop() || versionSource.href,
  );
  return url.href;
}

self.onmessage = async (event) => {
  const { kind, id, requestJson, mainModuleUrl } = event.data || {};
  if (kind === "warm") {
    try {
      await initCompute(mainModuleUrl);
      self.postMessage({ kind: "ready", ok: true });
    } catch (error) {
      self.postMessage({ kind: "ready", ok: false, error: errorText(error) });
    }
    return;
  }
  let handle;
  try {
    await initCompute(mainModuleUrl);
    handle = computeHandle;
  } catch (error) {
    self.postMessage({ id, ok: false, error: errorText(error) });
    return;
  }
  let value;
  try {
    value = handle(requestJson || "{}");
  } catch (error) {
    // Nothing that throws out of the compute export leaves the Wasm instance in
    // a state this worker can vouch for: a trap, a Rust panic and a host
    // call-stack overflow all unwind through Wasm frames without running their
    // epilogues. Ordinary gentufa failures never reach here, because they come
    // back as a successful JSON response carrying an error result, so the fatal
    // bit is exact rather than a guess about the error's text. The client uses
    // it to retire this worker instead of returning it to the idle pool; the
    // request itself still fails and is never replayed.
    self.postMessage({ id, ok: false, fatal: true, error: errorText(error) });
    return;
  }
  self.postMessage({ id, ok: true, value });
};

function errorText(error) {
  return error instanceof Error ? error.message : String(error);
}
