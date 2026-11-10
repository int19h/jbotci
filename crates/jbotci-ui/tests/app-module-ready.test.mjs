import assert from "node:assert/strict";
import test from "node:test";

const { importAppModule, waitForAppModuleReady } = await import(
  "../assets/app-module-ready.js"
);

function moduleUrl(source) {
  return `data:text/javascript,${encodeURIComponent(source)}`;
}

function dioxusModule({ fail = false, delayMs = 0 } = {}) {
  return moduleUrl(`
    export function jbotciWorkerReady() {
      globalThis.__readyCalls = (globalThis.__readyCalls || 0) + 1;
      globalThis.__readySawWasm = globalThis.__dx_mainWasm !== undefined;
      return Promise.resolve();
    }
    async function __wbg_init() {
      await new Promise((resolve) => setTimeout(resolve, ${delayMs}));
      if (${fail ? "true" : "false"}) {
        throw new Error("wasm compile failed");
      }
      return { main() { globalThis.__mainRan = true; } };
    }
    globalThis.__wasm_split_main_initSync = function initSync() {};
    __wbg_init({ module_or_path: "/app_bg.wasm" }).then((wasm) => {
      globalThis.__dx_mainWasm = wasm;
      globalThis.__dx_mainInit = __wbg_init;
      globalThis.__dx_mainInitSync = globalThis.__wasm_split_main_initSync;
      if (wasm.__wbindgen_start == undefined) {
        wasm.main();
      }
    });
  `);
}

function resetSignals() {
  delete globalThis.__dx_mainWasm;
  delete globalThis.__dx_mainInit;
  delete globalThis.__dx_mainInitSync;
  delete globalThis.__wasm_split_main_initSync;
  delete globalThis.__readyCalls;
  delete globalThis.__readySawWasm;
  delete globalThis.__mainRan;
}

test("app module readiness follows the dioxus init promise", { concurrency: false }, async () => {
  const unhandled = [];
  const onUnhandled = (reason) => {
    unhandled.push(reason);
  };
  process.on("unhandledRejection", onUnhandled);
  try {
    resetSignals();
    const originalThen = Promise.prototype.then;
    await importAppModule(dioxusModule({ delayMs: 40 }));
    assert.equal(Promise.prototype.then, originalThen);
    assert.equal(globalThis.__readyCalls, 1);
    assert.equal(globalThis.__readySawWasm, true);
    assert.equal(globalThis.__mainRan, true);
    assert.equal(unhandled.length, 0);

    resetSignals();
    await assert.rejects(
      () => importAppModule(dioxusModule({ fail: true, delayMs: 10 })),
      (error) => {
        assert.match(error.message, /wasm initialization failed/);
        assert.match(error.message, /wasm compile failed/);
        return true;
      },
    );
    assert.equal(globalThis.__readyCalls, undefined);

    resetSignals();
    const mainFails = moduleUrl(`
      export function jbotciWorkerReady() {
        globalThis.__readyCalls = 1;
        return Promise.resolve();
      }
      async function __wbg_init() {
        return { main() {} };
      }
      __wbg_init({ module_or_path: "/app_bg.wasm" }).then((wasm) => {
        globalThis.__dx_mainWasm = wasm;
        throw new Error("wasm main failed");
      });
    `);
    await assert.rejects(
      () => importAppModule(mainFails),
      (error) => {
        assert.match(error.message, /wasm initialization failed/);
        assert.match(error.message, /wasm main failed/);
        return true;
      },
    );
    assert.equal(globalThis.__readyCalls, undefined);
    await new Promise((resolve) => setTimeout(resolve, 20));
    assert.equal(unhandled.length, 0);
    assert.equal(Promise.prototype.then, originalThen);

    resetSignals();
    const exported = moduleUrl(`
      export const __wasm = { exported: true };
      export function jbotciWorkerReady() {
        globalThis.__readyCalls = 1;
        return Promise.resolve();
      }
    `);
    await importAppModule(exported);
    assert.equal(globalThis.__readyCalls, 1);

    resetSignals();
    const missing = moduleUrl(`
      export function jbotciWorkerReady() {}
    `);
    delete globalThis.__dx_mainWasm;
    await assert.rejects(
      () => waitForAppModuleReady({}),
      /does not export jbotciWorkerReady/,
    );
    await assert.rejects(
      () => importAppModule(missing),
      /did not start wasm initialization/,
    );

    resetSignals();
    await assert.rejects(
      () => importAppModule("data:text/javascript,%%%"),
      /did not load/,
    );
    assert.equal(Promise.prototype.then, originalThen);
  } finally {
    process.off("unhandledRejection", onUnhandled);
    resetSignals();
  }
});
