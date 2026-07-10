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
