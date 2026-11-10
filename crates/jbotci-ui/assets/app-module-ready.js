// Dioxus 0.7.9 appends this call to the wasm-bindgen module and drops the
// promise it returns:
//
//   __wbg_init({module_or_path: "..."}).then((wasm) => {
//     globalThis.__dx_mainWasm = wasm;
//     ...
//     wasm.main();
//   });
//
// import() resolves when that statement runs, before the promise settles.
// The promise is not exported. wasm-bindgen does not store an in-flight
// __wbg_init call, so a second call starts a second instantiation.
// The fulfillment function is the exact handle. Its source assigns
// globalThis.__dx_mainWasm. Capture the promise that `.then` returns.
// That promise rejects when fetch, compile, or instantiate fails.
// It also rejects when the callback throws, including a throw from wasm.main().
// A fixed time limit is the wrong test. A slow machine must be able to finish.

const DIOXUS_INIT_MARKER = "globalThis.__dx_mainWasm";

export function captureAppModuleInit() {
  const originalThen = Promise.prototype.then;
  let initPromise = null;

  function patchedThen(onFulfilled, onRejected) {
    const derived = originalThen.call(this, onFulfilled, onRejected);
    if (initPromise === null && isDioxusInitFulfillment(onFulfilled)) {
      // `derived` settles after the dioxus callback. The generated call does
      // not handle rejection. This handler marks that rejection as handled.
      // The caller awaits the same promise and reports the error.
      initPromise = derived;
      originalThen.call(
        derived,
        function markInitSettled() {},
        function markInitRejectionHandled() {},
      );
    }
    return derived;
  }

  Promise.prototype.then = patchedThen;
  return {
    get promise() {
      return initPromise;
    },
    stop() {
      if (Promise.prototype.then === patchedThen) {
        Promise.prototype.then = originalThen;
      }
    },
  };
}

export async function importAppModule(moduleUrl, options = {}) {
  const label = options.label || "Dioxus app module";
  if (typeof moduleUrl !== "string" || moduleUrl.length === 0) {
    throw new Error(`${label} did not load`);
  }
  const capture = captureAppModuleInit();
  let appModule;
  try {
    appModule = await import(moduleUrl);
  } catch (error) {
    throw new Error(`${label} did not load: ${errorText(error)}`);
  } finally {
    capture.stop();
  }
  await waitForAppModuleReady(appModule, {
    label,
    initPromise: capture.promise,
  });
  return appModule;
}

export async function waitForAppModuleReady(appModule, options = {}) {
  const label = options.label || "Dioxus app module";
  if (!appModule || typeof appModule !== "object") {
    throw new Error(`${label} did not load`);
  }
  if (typeof appModule.jbotciWorkerReady !== "function") {
    throw new Error(`${label} does not export jbotciWorkerReady`);
  }
  const initPromise = options.initPromise ?? null;
  if (initPromise === null && !wasmExportsReady(appModule)) {
    throw new Error(
      `${label} did not start wasm initialization. The dioxus module does not expose its init promise.`,
    );
  }
  if (initPromise !== null) {
    // Await this even when the wasm marker is already set. The dioxus callback
    // assigns the marker and then calls wasm.main(). A throw from main() rejects
    // this promise after the assignment.
    try {
      await initPromise;
    } catch (error) {
      throw new Error(`${label} wasm initialization failed: ${errorText(error)}`);
    }
  }
  if (!wasmExportsReady(appModule)) {
    throw new Error(
      `${label} wasm initialization finished without ${DIOXUS_INIT_MARKER} or appModule.__wasm`,
    );
  }
  await appModule.jbotciWorkerReady();
}

function wasmExportsReady(appModule) {
  // The web glue sets globalThis.__dx_mainWasm after instantiate.
  // Node and Deno builds export the instance as appModule.__wasm instead.
  return appModule.__wasm !== undefined || globalThis.__dx_mainWasm !== undefined;
}

function isDioxusInitFulfillment(onFulfilled) {
  if (typeof onFulfilled !== "function") {
    return false;
  }
  return Function.prototype.toString.call(onFulfilled).includes(DIOXUS_INIT_MARKER);
}

function errorText(error) {
  if (error instanceof Error) {
    return error.message;
  }
  return String(error);
}
