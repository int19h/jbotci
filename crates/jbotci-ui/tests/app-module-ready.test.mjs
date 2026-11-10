import assert from "node:assert/strict";
import test from "node:test";

// Node has no unhandledrejection listener on globalThis.
// The helper under test uses that listener. This bridge forwards the process event.
let rejectionTarget = new EventTarget();
const prevented = [];

function useFreshRejectionTarget() {
  rejectionTarget = new EventTarget();
  globalThis.addEventListener = rejectionTarget.addEventListener.bind(rejectionTarget);
  globalThis.removeEventListener = rejectionTarget.removeEventListener.bind(rejectionTarget);
}

useFreshRejectionTarget();

// Node's test runner fails a test on an unhandled rejection.
// A browser worker does not. The worker calls preventDefault on that event.
// This bridge forwards the process event to the same listener.
// It removes the runner listener first.
// The loader then rejects with the init error.
test.before(() => {
  process.removeAllListeners("unhandledRejection");
  process.on("unhandledRejection", (reason) => {
    const event = new Event("unhandledrejection", { cancelable: true });
    event.reason = reason;
    rejectionTarget.dispatchEvent(event);
    prevented.push(event.defaultPrevented);
  });
});

const { importAppModule } = await import("../assets/app-module-ready.js");

function moduleUrl(source) {
  return `data:text/javascript,${encodeURIComponent(source)}`;
}

function delay(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

// The generated Dioxus glue calls __wbindgen_start inside __wbg_init.
// import() resolves before that async work finishes.
// The dropped .then calls main only if __wbindgen_start is missing.
function dioxusModule({ fail = false, panic = false, silent = false, delayMs = 30 } = {}) {
  return moduleUrl(`
    async function __wbg_init() {
      await new Promise((resolve) => setTimeout(resolve, ${delayMs}));
      if (${fail ? "true" : "false"}) {
        throw new Error("wasm compile failed");
      }
      const wasm = {
        __wbindgen_start() {
          globalThis.__loaderDoneAtStart = globalThis.__loaderDone === true;
          if (${panic ? "true" : "false"}) {
            throw new Error("wasm main failed");
          }
          if (${silent ? "false" : "true"}) {
            globalThis.jbotciWorkerStarted();
          }
          globalThis.__mainRan = true;
        },
      };
      wasm.__wbindgen_start();
      return wasm;
    }
    __wbg_init({ module_or_path: "/./wasm/jbotci-app_bg.wasm" }).then((wasm) => {
      globalThis.__dx_mainWasm = wasm;
      globalThis.__initFulfilled = true;
      if (wasm.__wbindgen_start == undefined) {
        wasm.main();
      }
    });
  `);
}

function resetSignals() {
  useFreshRejectionTarget();
  prevented.length = 0;
  delete globalThis.__dx_mainWasm;
  delete globalThis.__initFulfilled;
  delete globalThis.__loaderDone;
  delete globalThis.__loaderDoneAtStart;
  delete globalThis.__mainRan;
  delete globalThis.jbotciWorkerStarted;
}

function track(loader) {
  globalThis.__loaderDone = false;
  return loader.then(
    (value) => {
      globalThis.__loaderDone = true;
      return value;
    },
    (error) => {
      globalThis.__loaderDone = true;
      throw error;
    },
  );
}

test("the loader waits for the Rust ready call", { concurrency: false }, async () => {
  resetSignals();
  const originalThen = Promise.prototype.then;
  const loader = track(importAppModule(dioxusModule({ delayMs: 40 })));
  const appModule = await loader;
  assert.equal(typeof appModule, "object");
  assert.notEqual(appModule, null);
  assert.equal(globalThis.__loaderDoneAtStart, false);
  assert.equal(globalThis.__mainRan, true);
  assert.equal(globalThis.__loaderDone, true);
  assert.equal(globalThis.jbotciWorkerStarted, undefined);
  assert.equal(Promise.prototype.then, originalThen);
  assert.equal(prevented.length, 0);
  const later = new Event("unhandledrejection", { cancelable: true });
  later.reason = new Error("later");
  rejectionTarget.dispatchEvent(later);
  assert.equal(later.defaultPrevented, false);
});

test("a failed fetch or compile rejects with the cause", { concurrency: false }, async () => {
  resetSignals();
  const originalThen = Promise.prototype.then;
  await assert.rejects(
    () => track(importAppModule(dioxusModule({ fail: true, delayMs: 20 }))),
    (error) => {
      assert.match(error.message, /Dioxus app module wasm initialization failed/);
      assert.match(error.message, /wasm compile failed/);
      return true;
    },
  );
  assert.equal(globalThis.__mainRan, undefined);
  assert.equal(globalThis.jbotciWorkerStarted, undefined);
  assert.equal(Promise.prototype.then, originalThen);
  assert.equal(prevented.at(-1), true);
});

test("a panic in main rejects with the cause", { concurrency: false }, async () => {
  resetSignals();
  await assert.rejects(
    () => track(importAppModule(dioxusModule({ panic: true, delayMs: 20 }))),
    (error) => {
      assert.match(error.message, /wasm initialization failed/);
      assert.match(error.message, /wasm main failed/);
      return true;
    },
  );
  assert.equal(globalThis.__mainRan, undefined);
  assert.equal(globalThis.jbotciWorkerStarted, undefined);
  assert.equal(prevented.at(-1), true);
});

test("a bad module URL reports that the import did not load", { concurrency: false }, async () => {
  resetSignals();
  await assert.rejects(() => importAppModule(""), /Dioxus app module did not load/);
  await assert.rejects(
    () => importAppModule("data:text/javascript,%%%"),
    (error) => {
      assert.match(error.message, /Dioxus app module did not load:/);
      assert.doesNotMatch(error.message, /wasm initialization failed/);
      return true;
    },
  );
  assert.equal(globalThis.jbotciWorkerStarted, undefined);
});

test("a module that never signals leaves the loader pending", { concurrency: false }, async () => {
  resetSignals();
  // This binary cannot hit this case. Worker main always calls the ready function.
  // The test module returns without that call, and its init promise fulfills.
  // A no-op import wrapper would already have resolved. This loader must not.
  const loader = track(importAppModule(dioxusModule({ silent: true, delayMs: 20 })));
  const outcome = await Promise.race([
    loader.then(() => "resolved", () => "rejected"),
    (async () => {
      const started = Date.now();
      while (globalThis.__initFulfilled !== true) {
        if (Date.now() - started > 2000) {
          return "init-never-fulfilled";
        }
        await delay(5);
      }
      await delay(30);
      return globalThis.__loaderDone === true ? "resolved-after-fulfill" : "still-pending";
    })(),
  ]);
  assert.equal(outcome, "still-pending");
  assert.equal(globalThis.__mainRan, true);
  assert.equal(globalThis.__initFulfilled, true);
  assert.equal(prevented.length, 0);
});
