import assert from "node:assert/strict";
import test from "node:test";
import { readFile } from "node:fs/promises";
import { createHash, webcrypto } from "node:crypto";

globalThis.self = { postMessage() {} };
globalThis.crypto ||= webcrypto;
Object.defineProperty(globalThis, "navigator", { configurable: true, value: { storage: {} } });
const workerUrl = new URL("../assets/embedding-worker.js", import.meta.url);
const source = (await readFile(workerUrl, "utf8")).replaceAll("import.meta.url", JSON.stringify(workerUrl.href));
const identity = { modelRevision: "revision", modelSha256: "a".repeat(64), tokenizerSha256: "b".repeat(64), artifactManifestSha256: "c".repeat(64) };
const runtime = { runtime: "test-runtime", version: "1", dtype: "q4", device: "webgpu", pooling: "mean", max_window_tokens: 512 };
async function worker() {
  const harness = `
export function configureTestStore(db) {
  const identity = ${JSON.stringify(identity)};
  const runtime = ${JSON.stringify(runtime)};
  modelCatalog = { defaultDesktopModelKey: 'test', models: { test: {
    modelKey: 'test', remoteVectorPacks: true, dimensions: 2, vectorElementType: 'f32le', localVectorSpaceKey: 'space',
    outputPooling: 'mean', maxSequenceLength: 512, customRuntime: runtime,
  } } };
  activeModelKey = 'test'; selectedModelKey = 'test'; modelRuntime = { ...runtime, modelIdentity: identity };
  dbPromise = Promise.resolve(db);
  f2llmRuntimeModule = async () => ({ jbotciF2LlmQueryInput: value => value });
  ensureModel = async () => ({});
  embedTexts = async () => [new Float32Array([1, 0])];
  isCustomWebGpuRuntime = () => false;
}
export function countVectorHashes() {
  const original = sha256Hex; let count = 0;
  sha256Hex = async bytes => { if (bytes.byteLength === 8) count++; return original(bytes); };
  return () => count;
}
export { search, getBinary, loadRemotePackIfAvailable };
`;
  return import(`data:text/javascript;base64,${Buffer.from(source + harness + `\n// worker ${Math.random()}`).toString("base64")}`);
}

// Requests and read-write transactions follow IndexedDB's serialized store scope.
// Two independent worker modules share this database, as two PWA tabs do.
class Database {
  maps = new Map([["meta", new Map()], ["blobs", new Map()]]);
  pending = [];
  running = false;
  illegalDeletes = [];
  transaction(names, mode) {
    const tx = new Transaction(this, Array.isArray(names) ? names : [names], mode);
    this.pending.push(tx);
    if (!this.running) { this.running = true; setImmediate(() => this.next()); }
    return tx;
  }
  next() {
    const tx = this.pending[0];
    if (!tx) { this.running = false; return; }
    tx.maps = new Map([...this.maps].map(([key, values]) => [key, new Map([...values].map(([k, v]) => [k, structuredClone(v)]))]));
    tx.step();
  }
}
class Transaction {
  operations = [];
  constructor(db, names, mode) { this.db = db; this.names = names; this.mode = mode; }
  enqueue(fn) { const request = {}; this.operations.push(() => { request.result = fn(); request.onsuccess?.(); }); return request; }
  step() {
    try {
      const operation = this.operations.shift();
      if (operation) { operation(); setImmediate(() => this.step()); return; }
      if (this.mode === "readwrite") this.db.maps = this.maps;
      this.oncomplete?.();
    } catch (error) { this.error = error; this.onerror?.(); }
    this.db.pending.shift();
    setImmediate(() => this.db.next());
  }
  objectStore(name) {
    assert.ok(this.names.includes(name));
    return {
      get: key => this.enqueue(() => structuredClone(this.maps.get(name).get(key))),
      getAll: () => this.enqueue(() => {
        const values = [...this.maps.get(name).values()].map(value => structuredClone(value));
        this.db.afterReferenceRead?.();
        return values;
      }),
      put: (value, key) => this.enqueue(() => this.maps.get(name).set(key, structuredClone(value))),
      delete: key => this.enqueue(() => {
        if (name === "blobs") {
          const refs = [...this.maps.get("meta").values()].flatMap(value => Object.values(value?.corpora || {}))
            .flatMap(corpus => [corpus.rawItemsKey, ...(corpus.shards || []).map(shard => shard.key)]);
          if (refs.includes(key)) this.db.illegalDeletes.push(key);
        }
        return this.maps.get(name).delete(key);
      }),
      openKeyCursor: () => {
        const request = {};
        let keys;
        let offset = 0;
        const step = () => {
          keys ||= [...this.maps.get(name).keys()];
          request.result = offset < keys.length ? { key: keys[offset++], continue: () => this.operations.push(step) } : null;
          request.onsuccess?.();
        };
        this.operations.push(step);
        return request;
      },
    };
  }
}
const hash = bytes => createHash("sha256").update(new Uint8Array(bytes)).digest("hex");
function corpus(id, generation = 0) {
  const items = [{ [id === "vlacku-en" ? "entry_index" : "chunk_index"]: 0, row: 0, kind: "gismu", input_hash: `${id}-${generation}` }];
  const raw = new TextEncoder().encode(JSON.stringify(items)).buffer;
  const vectors = new Float32Array(generation ? [-1, 0] : [1, 0]).buffer;
  return { raw, vectors, manifest: { corpus_id: id, input_hash: `${id}-${generation}`, row_count: 1,
    dimensions: 2, items_url: `${id}/items.json`, items_sha256: hash(raw), vector_url: `${id}/vectors.f32`,
    vector_sha256: hash(vectors), vector_byte_len: vectors.byteLength } };
}
const packInfo = (id = "old", modelKey = "test") => ({ source: "remote", modelKey, modelIdentity: identity, packId: id,
  inputHash: id, inputFormatVersion: "format", vectorSpaceKey: "space", pooling: "mean", maxWindowTokens: 512,
  runtime, compatibleQueryRuntimes: [runtime] });
const manifest = { element_type: "f32le" };
async function install(w, db, data, previous, info, fetches = []) {
  const result = await w.installRemoteCorpus(data.manifest, manifest, previous, "root",
    async url => { fetches.push(url); return data.raw.slice(0); },
    async url => { fetches.push(url); return data.vectors.slice(0); },
    async key => { const value = db.maps.get("blobs").get(key); if (!value) throw new Error("evicted"); return value.slice(0); });
  await w.commitCorpus(info, result.record, result.rawItems, result.vectors, db);
  return result.record;
}

function remoteFixture(data, failCorpus = null) {
  const doc = { inputHash: "new", inputFormatVersion: "format", dictionaryHash: data[0].manifest.input_hash,
    cllHash: data[1].manifest.input_hash, dictionary: [{}], cll: [{}] };
  const remote = { schema_version: 1, model_key: "test", model_revision: identity.modelRevision, q4_onnx_sha256: identity.modelSha256,
    input_hash: "new", input_format_version: "format", dimensions: 2, element_type: "f32le", normalized: true,
    distance: "dot", vector_space_key: "space", pooling: "mean", max_window_tokens: 512, pack_id: "new",
    compatible_query_runtimes: [runtime], corpora: data.map(value => value.manifest) };
  const fetched = [];
  const fetch = async input => {
    const url = String(input.url || input);
    if (url.endsWith("/catalog.json")) return Response.json({ models: [{ model_key: "test", vector_spaces: [{ manifest_url: "packs/new/manifest.json", compatible_query_runtimes: [runtime] }] }] });
    if (url.endsWith("/manifest.json")) return Response.json(remote);
    fetched.push(url);
    const entry = data.find(value => url.endsWith(value.manifest.items_url) || url.endsWith(value.manifest.vector_url));
    assert.ok(entry, `unexpected fetch ${url}`);
    if (entry.manifest.corpus_id === failCorpus && url.endsWith(entry.manifest.vector_url)) throw new Error("interrupted download");
    return new Response(url.endsWith(entry.manifest.items_url) ? entry.raw : entry.vectors);
  };
  return { doc, fetched, fetch };
}
for (const changed of ["vlacku-en", "cukta-cll"]) {
  test(`worker update downloads only ${changed}`, async () => {
    const w = await worker(); const db = new Database(); w.configureTestStore(db);
    for (const id of ["vlacku-en", "cukta-cll"]) await install(w, db, corpus(id), null, packInfo());
    const remote = remoteFixture([corpus("vlacku-en", Number(changed === "vlacku-en")), corpus("cukta-cll", Number(changed === "cukta-cll"))]);
    const oldFetch = globalThis.fetch; globalThis.fetch = remote.fetch;
    try { assert.equal((await w.loadRemotePackIfAvailable(remote.doc, "https://fixture.invalid")).loaded, true); }
    finally { globalThis.fetch = oldFetch; }
    assert.deepEqual(remote.fetched.sort(), [`https://fixture.invalid/packs/new/${changed}/items.json`, `https://fixture.invalid/packs/new/${changed}/vectors.f32`].sort());
  });
}

test("a finished corpus survives interruption of the other corpus", async () => {
  const w = await worker(); const db = new Database(); w.configureTestStore(db);
  for (const id of ["vlacku-en", "cukta-cll"]) await install(w, db, corpus(id), null, packInfo());
  const remote = remoteFixture([corpus("vlacku-en", 1), corpus("cukta-cll", 1)], "cukta-cll");
  const oldFetch = globalThis.fetch; globalThis.fetch = remote.fetch;
  try { await assert.rejects(w.loadRemotePackIfAvailable(remote.doc, "https://fixture.invalid"), /interrupted/); }
  finally { globalThis.fetch = oldFetch; }
  const live = { inputHash: "new", inputFormatVersion: "format", corpora: { "vlacku-en": { inputHash: "vlacku-en-1", rowCount: 1 }, "cukta-cll": { inputHash: "cukta-cll-1", rowCount: 1 } } };
  assert.equal((await w.search(live, "vlacku-en", "query", 1, "[]", false)).hits.length, 1);
  assert.equal((await w.search(live, "cukta-cll", "query", 1, "[]", false)).hits.length, 0);
});
for (const damage of ["wrong-vector", "wrong-items", "missing-vector", "missing-items", "partly-evicted"]) {
  test(`a ${damage} store cannot authorize reuse`, async () => {
    const w = await worker(); const db = new Database(); w.configureTestStore(db);
    const data = corpus("vlacku-en");
    const old = await install(w, db, data, null, packInfo());
    if (damage === "wrong-items") { const bad = data.raw.slice(0); new Uint8Array(bad)[0] ^= 1; db.maps.get("blobs").set(old.rawItemsKey, bad); }
    if (damage === "wrong-vector") { const bad = data.vectors.slice(0); new Uint8Array(bad)[0] ^= 1; db.maps.get("blobs").set(old.shards[0].key, bad); }
    if (damage === "missing-vector" || damage === "partly-evicted") db.maps.get("blobs").delete(old.shards[0].key);
    if (damage === "missing-items" || damage === "partly-evicted") db.maps.get("blobs").delete(old.rawItemsKey);
    const fetches = [];
    await install(w, db, data, old, packInfo("new"), fetches);
    assert.equal(fetches.includes("root/vlacku-en/vectors.f32"), ["wrong-vector", "missing-vector", "partly-evicted"].includes(damage));
    assert.equal(fetches.includes("root/vlacku-en/items.json"), ["wrong-items", "missing-items", "partly-evicted"].includes(damage));
  });
}

test("legacy vectors are hashed once while raw items are obtained without reconstruction", async () => {
  const w = await worker(); const db = new Database(); w.configureTestStore(db);
  const data = corpus("vlacku-en");
  const old = await install(w, db, data, null, packInfo());
  delete old.itemsSha256; delete old.vectorSha256; delete old.rawItemsKey;
  const fetches = [];
  const hashCount = w.countVectorHashes();
  let reads = 0;
  const result = await w.installRemoteCorpus(data.manifest, manifest, old, "root",
    async url => { fetches.push(url); return data.raw; }, async url => { fetches.push(url); return data.vectors; },
    async key => { reads++; return db.maps.get("blobs").get(key); });
  assert.equal(reads, 1);
  assert.equal(hashCount(), 1);
  assert.deepEqual(fetches, ["root/vlacku-en/items.json"]);
  assert.equal(result.record.vectorSha256, data.manifest.vector_sha256);
  assert.equal(result.record.itemsSha256, data.manifest.items_sha256);
});

for (const stale of ["vlacku-en", "cukta-cll"]) {
  test(`search ignores only the other stale corpus: ${stale}`, async () => {
    const w = await worker(); const db = new Database(); w.configureTestStore(db);
    for (const id of ["vlacku-en", "cukta-cll"]) await install(w, db, corpus(id), null, packInfo());
    const identity = { inputHash: "new-combined", inputFormatVersion: "format", corpora: {
      "vlacku-en": { inputHash: stale === "vlacku-en" ? "new-dictionary" : "vlacku-en-0", rowCount: 1 },
      "cukta-cll": { inputHash: stale === "cukta-cll" ? "new-cll" : "cukta-cll-0", rowCount: 1 } } };
    const current = stale === "vlacku-en" ? "cukta-cll" : "vlacku-en";
    const good = await w.search(identity, current, "query", 1, "[]", false);
    assert.equal(good.hits.length, 1);
    const bad = await w.search(identity, stale, "query", 1, "[]", false);
    assert.equal(bad.hits.length, 0);
    assert.match(bad.message, new RegExp(stale));
    const pack = db.maps.get("meta").get("pack:test");
    assert.equal((await w.corpusState(pack, identity, current)).kind, "verified-current");
    assert.equal((await w.corpusState(pack, identity, stale)).kind, "stale");
    new Uint8Array(db.maps.get("blobs").get(pack.corpora[current].shards[0].key))[0] ^= 1;
    assert.equal((await w.search(identity, current, "query", 1, "[]", false)).hits.length, 0);
  });
}

test("partial OPFS eviction falls back to a real download through the worker store reader", async () => {
  const w = await worker(); const db = new Database(); w.configureTestStore(db);
  const data = corpus("vlacku-en");
  const old = await install(w, db, data, null, packInfo());
  old.shards = [{ key: "remote/test/old/vlacku-en/vectors.f32", byteLen: 8 }];
  db.maps.get("meta").set("blobMeta", { [old.shards[0].key]: { storage: "opfs", byteLen: 8 } });
  navigator.storage.getDirectory = async () => ({ getDirectoryHandle: async () => { throw new Error("evicted OPFS directory"); } });
  const fetched = [];
  try {
    await w.installRemoteCorpus(data.manifest, manifest, old, "root",
      async url => { fetched.push(url); return data.raw; }, async url => { fetched.push(url); return data.vectors; }, w.getBinary);
  } finally { delete navigator.storage.getDirectory; }
  assert.deepEqual(fetched, ["root/vlacku-en/vectors.f32"]);
});

test("two workers publish and collect content without deleting referenced bytes", async () => {
  const a = await worker(); const b = await worker(); const db = new Database(); a.configureTestStore(db); b.configureTestStore(db);
  const data = corpus("vlacku-en");
  const old = await install(a, db, data, null, packInfo("one"));
  await install(b, db, data, null, packInfo("two", "other"));
  const orphan = `content/sha256/${"e".repeat(64)}`;
  db.maps.get("blobs").set(orphan, new ArrayBuffer(1));
  await Promise.all([a.collectContentGarbage(db), install(b, db, corpus("vlacku-en", 1), old, packInfo("new"))]);
  assert.ok(db.maps.get("blobs").has(old.shards[0].key), "the other pack still references the old bytes");
  assert.equal(db.maps.get("blobs").has(orphan), false);
  assert.deepEqual(db.illegalDeletes, []);
  // Remove the last reference, then prove that garbage collection is not a no-op.
  await new Promise(resolve => { const tx = db.transaction(["meta"], "readwrite"); tx.objectStore("meta").delete("pack:other"); tx.oncomplete = resolve; });
  await a.collectContentGarbage(db);
  assert.equal(db.maps.get("blobs").has(old.shards[0].key), false);

  // Force publication after the reference snapshot. A split read/delete collector
  // lets the writer run first, then deletes its now-referenced blob.
  const incoming = corpus("vlacku-en", 2);
  const next = await b.installRemoteCorpus(incoming.manifest, manifest, null, "root",
    async () => incoming.raw, async () => incoming.vectors);
  db.maps.get("blobs").set(next.record.rawItemsKey, next.rawItems);
  db.maps.get("blobs").set(next.record.shards[0].key, next.vectors);
  let publication;
  db.afterReferenceRead = () => {
    db.afterReferenceRead = null;
    publication = b.commitCorpus(packInfo("concurrent", "racer"), next.record, next.rawItems, next.vectors, db);
  };
  await a.collectContentGarbage(db);
  assert.ok(publication, "the concurrent writer must run");
  await publication;
  assert.ok(db.maps.get("blobs").has(next.record.rawItemsKey));
  assert.ok(db.maps.get("blobs").has(next.record.shards[0].key));
  assert.deepEqual(db.illegalDeletes, []);
});
