// The worker creates a promise before it imports the Dioxus app module.
// It stores the resolve function as globalThis.jbotciWorkerStarted.
// Rust main calls that function in a worker.
// Dioxus calls main after fetch, compile, and instantiate.
// The call is the ready signal.
//
// Fetch, compile, instantiate, or main can fail before that call.
// A panic or a trap is the same kind of failure.
// The Dioxus glue drops the promise for that work.
// Nothing else handles the rejection.
// This file listens for unhandledrejection during the import window.
// Before the ready call, only module startup runs in the worker.
// A rejection in that window is an init failure.
// This file removes the listener after the ready call.
// It also removes the listener after a rejection, and after a thrown import.
//
// This app cannot load the module and then stay silent.
// In a worker, main always calls jbotciWorkerStarted.
// The call runs before the Dioxus init promise can fulfill.
// A throw from main is an unhandled rejection.
// A test module can return from main without the call.
// No exact event reports that case.
// This file does not use a time limit for it.

export async function importAppModule(moduleUrl, options = {}) {
  const label = options.label || "Dioxus app module";
  if (typeof moduleUrl !== "string" || moduleUrl.length === 0) {
    throw new Error(`${label} did not load`);
  }

  let resolveStarted;
  let rejectStarted;
  let settled = false;
  const started = new Promise((resolve, reject) => {
    resolveStarted = resolve;
    rejectStarted = reject;
  });

  function finish(kind, error) {
    if (settled) {
      return;
    }
    settled = true;
    globalThis.removeEventListener("unhandledrejection", onUnhandledRejection);
    if (globalThis.jbotciWorkerStarted === onReady) {
      delete globalThis.jbotciWorkerStarted;
    }
    if (kind === "ready") {
      resolveStarted();
    } else {
      rejectStarted(error);
    }
  }

  function onReady() {
    finish("ready");
  }

  function onUnhandledRejection(event) {
    const reason = event && "reason" in event ? event.reason : event;
    if (event && typeof event.preventDefault === "function") {
      event.preventDefault();
    }
    finish(
      "fail",
      new Error(`${label} wasm initialization failed: ${errorText(reason)}`),
    );
  }

  globalThis.jbotciWorkerStarted = onReady;
  globalThis.addEventListener("unhandledrejection", onUnhandledRejection);

  let appModule;
  try {
    appModule = await import(moduleUrl);
  } catch (error) {
    finish("fail", new Error(`${label} did not load: ${errorText(error)}`));
  }
  await started;
  return appModule;
}

function errorText(error) {
  if (error instanceof Error) {
    return error.message;
  }
  return String(error);
}
