// Regression tests for worker disposal after a fatal compute failure.
//
// A throw out of `jbotciComputeHandle` is a Wasm trap, a Rust panic or a host
// call-stack overflow. Ordinary gentufa failures never reach that path: they
// come back as a successful JSON response carrying `result.status == "error"`.
// Nothing restores the Wasm instance after such a throw, so the worker reports
// it as fatal and the client retires that worker instead of returning it to the
// idle pool. Embedding failures are ordinary application outcomes and must stay
// non-fatal, so the shared client keys the behaviour off an explicit protocol
// flag rather than off `ok: false`.

import assert from "node:assert/strict";
import test from "node:test";

globalThis.location = { href: "https://example.invalid/app/" };
globalThis.JBOTCI_WEB_BOOTSTRAP = {
  mainModuleUrl: "https://example.invalid/app/wasm/jbotci-app.js",
};

const spawned = [];

class FakeWorker {
  constructor(url) {
    this.url = url;
    this.posted = [];
    this.terminated = false;
    this.openRequestId = null;
    this.onmessage = null;
    this.onerror = null;
    this.onmessageerror = null;
    spawned.push(this);
  }

  postMessage(message) {
    this.posted.push(message);
    if (message.kind === "warm") {
      // Report readiness asynchronously so tests do not depend on worker timing.
      queueMicrotask(() => this.emit({ kind: "ready", ok: true }));
    } else {
      this.openRequestId = message.id;
    }
  }

  terminate() {
    this.terminated = true;
  }

  emit(data) {
    if (this.terminated) {
      throw new Error("a terminated worker must not deliver further messages");
    }
    if (data.id !== undefined && data.id === this.openRequestId) {
      this.openRequestId = null;
    }
    this.onmessage?.({ data });
  }
}

globalThis.Worker = FakeWorker;

const { createWorkerClient } = await import("../assets/worker-client.js");

function newClient(label) {
  spawned.length = 0;
  return createWorkerClient({
    label,
    defaultWorkerUrl: () => new URL("https://example.invalid/app/worker.js"),
    minIdleWorkers: 0,
    maxIdleWorkers: 3,
    warmMessage: (context) => ({ kind: "warm", mainModuleUrl: context.mainModuleUrl }),
    requestMessage: ({ id, payload }) => ({ id, payload }),
  });
}

/// Yields until the fake worker pool has settled the warm handshake and the
/// client has posted the pending request.
async function settle() {
  for (let turn = 0; turn < 20; turn += 1) {
    await new Promise((resolve) => setTimeout(resolve, 0));
  }
}

/// Every live worker holding a request message it has not yet answered, in the
/// order the workers were created.
async function workersWithOpenRequests() {
  await settle();
  return spawned.filter((entry) => !entry.terminated && entry.openRequestId !== null);
}

/// The single live worker holding an unanswered request.
async function workerWithOpenRequest() {
  const workers = await workersWithOpenRequests();
  assert.equal(workers.length, 1, "exactly one worker is holding a request");
  return workers[0];
}

function requestIdOf(worker) {
  assert.notEqual(worker.openRequestId, null, "the worker has an unanswered request");
  return worker.openRequestId;
}

test("a fatal compute failure retires its worker and the next request gets a fresh one", async () => {
  const client = newClient("compute");

  const first = client.request("gentufa", "request-1");
  const firstWorker = await workerWithOpenRequest();
  const firstId = requestIdOf(firstWorker);
  firstWorker.emit({
    id: firstId,
    ok: false,
    fatal: true,
    error: "Maximum call stack size exceeded",
  });

  await assert.rejects(first, /Maximum call stack size exceeded/);
  assert.equal(firstWorker.terminated, true, "the trapped worker is terminated");

  const second = client.request("gentufa", "request-2");
  const secondWorker = await workerWithOpenRequest();
  assert.notEqual(secondWorker, firstWorker, "the next request uses a different worker");
  assert.equal(secondWorker.terminated, false);
  secondWorker.emit({ id: requestIdOf(secondWorker), ok: true, value: "response-2" });
  assert.equal(await second, "response-2");
});

test("a fatal failure rejects its request exactly once and never replays it", async () => {
  const client = newClient("compute");

  const pending = client.request("gentufa", "request-1");
  const worker = await workerWithOpenRequest();
  const id = requestIdOf(worker);
  const postedBefore = worker.posted.length;
  worker.emit({ id, ok: false, fatal: true, error: "trap" });

  let rejections = 0;
  await pending.catch(() => {
    rejections += 1;
  });
  assert.equal(rejections, 1);
  assert.equal(
    worker.posted.length,
    postedBefore,
    "the failed request is not resent to the trapped worker",
  );
  for (const other of spawned) {
    const requests = other.posted.filter((posted) => posted.kind !== "warm");
    assert.ok(requests.length <= 1, "no worker receives the failed request twice");
  }
});

test("an ordinary non-fatal failure leaves its worker reusable", async () => {
  const client = newClient("embedding");

  const first = client.request("embedding-status", "request-1");
  const worker = await workerWithOpenRequest();
  worker.emit({
    id: requestIdOf(worker),
    ok: false,
    error: "Semantic search index is outdated. Open Settings and click Update.",
  });
  await assert.rejects(first, /outdated/);
  assert.equal(worker.terminated, false, "an application failure does not retire the worker");

  const second = client.request("embedding-status", "request-2");
  const secondWorker = await workerWithOpenRequest();
  assert.equal(secondWorker, worker, "the same worker serves the next request");
  secondWorker.emit({ id: requestIdOf(secondWorker), ok: true, value: "response-2" });
  assert.equal(await second, "response-2");
});

test("a response for an unknown or already-settled id retires nothing", async () => {
  const client = newClient("compute");

  const pending = client.request("gentufa", "request-1");
  const worker = await workerWithOpenRequest();
  const id = requestIdOf(worker);

  worker.emit({ id: id + 1000, ok: false, fatal: true, error: "stray" });
  assert.equal(worker.terminated, false, "a stray id does not retire a worker");

  worker.emit({ id, ok: true, value: "response-1" });
  assert.equal(await pending, "response-1");

  worker.emit({ id, ok: false, fatal: true, error: "duplicate" });
  assert.equal(worker.terminated, false, "a duplicate response does not retire a worker");
});

test("a fatal response from a worker that does not own the live id retires nothing", async () => {
  const client = newClient("compute");

  const first = client.request("gentufa", "request-1");
  const second = client.request("gentufa", "request-2");
  const [firstWorker, secondWorker] = await workersWithOpenRequests();
  assert.ok(secondWorker, "two concurrent requests occupy two workers");
  assert.notEqual(secondWorker, firstWorker, "concurrent requests use different workers");
  const firstId = requestIdOf(firstWorker);

  // The second worker claims the first worker's live request.
  secondWorker.emit({ id: firstId, ok: false, fatal: true, error: "not mine" });
  assert.equal(firstWorker.terminated, false, "the owning worker is untouched");
  assert.equal(secondWorker.terminated, false, "the impostor does not retire the owner");

  firstWorker.emit({ id: firstId, ok: true, value: "response-1" });
  secondWorker.emit({ id: requestIdOf(secondWorker), ok: true, value: "response-2" });
  assert.equal(await first, "response-1");
  assert.equal(await second, "response-2");
});

test("cancellation still retires the worker running the cancelled request", async () => {
  const client = newClient("compute");

  const pending = client.request("gentufa", "request-1");
  const worker = await workerWithOpenRequest();
  assert.equal(client.hasPending("gentufa"), true);

  client.cancel("gentufa");
  await assert.rejects(pending, /cancelled/);
  assert.equal(worker.terminated, true, "a cancelled request does not leave its worker in the pool");
  assert.equal(client.hasPending("gentufa"), false);
});

test("a worker that fails initialization rejects its request without being reused", async () => {
  const client = newClient("compute");

  const pending = client.request("gentufa", "request-1");
  const worker = await workerWithOpenRequest();
  worker.onmessage({ data: { kind: "ready", ok: false, error: "module load failed" } });
  worker.emit({ id: requestIdOf(worker), ok: false, error: "module load failed" });

  await assert.rejects(pending, /module load failed/);
});
