import assert from "node:assert/strict";
import test from "node:test";

globalThis.self = globalThis;

const {
  packCorpusCompatibilityIssue,
  searchPackCompatibilityError,
  statusDisplay,
} = await import("../assets/embedding-worker.js");
const { embeddingStatusShouldAutoUpdate } = await import("../assets/embeddings.js");

const currentIdentity = {
  inputHash: "current-aggregate-hash",
  inputFormatVersion: "jbotci-embedding-input-v1",
  corpora: {
    "vlacku-en": {
      inputHash: "current-dictionary-hash",
      rowCount: 3,
    },
    "cukta-cll": {
      inputHash: "current-cll-hash",
      rowCount: 2,
    },
  },
};

const stalePersistedPack = {
  inputHash: "stale-aggregate-hash",
  inputFormatVersion: currentIdentity.inputFormatVersion,
  corpora: {
    "vlacku-en": {
      inputHash: "stale-dictionary-hash",
      rowCount: 3,
      items: [{}, {}, {}],
    },
    "cukta-cll": {
      inputHash: "stale-cll-hash",
      rowCount: 2,
      items: [{}, {}],
    },
  },
};

test("stale persisted corpus identity is needs-update on every recovery path", () => {
  const issue = packCorpusCompatibilityIssue(stalePersistedPack, currentIdentity);
  assert.equal(issue?.field, "inputHash");

  for (const persistedStatus of [
    { status: "ready", detail: "cached" },
    { status: "checking", detail: "interrupted" },
    null,
  ]) {
    const display = statusDisplay(persistedStatus, stalePersistedPack, false, issue);
    assert.equal(display.status, "needs-update");
    assert.match(display.detail, /outdated/i);
    assert.equal(display.progress, null);
  }
});

test("stale persisted corpus identity produces a typed refused-search error", () => {
  const issue = packCorpusCompatibilityIssue(stalePersistedPack, currentIdentity);
  const response = searchPackCompatibilityError(issue);

  assert.deepEqual(response.hits, []);
  assert.equal(response.error.code, "embedding-index-needs-update");
  assert.equal(response.error.message, response.message);
  assert.match(response.message, /open settings and click update/i);
  assert.equal(response.error.compatibilityIssue.field, "inputHash");
});

test("only stale downloaded packs are auto-updated", () => {
  assert.equal(
    embeddingStatusShouldAutoUpdate({ status: "needs-update", source: "remote" }),
    true,
  );
  assert.equal(
    embeddingStatusShouldAutoUpdate({ status: "needs-update", source: "browser" }),
    false,
  );
  assert.equal(
    embeddingStatusShouldAutoUpdate({ status: "ready", source: "remote" }),
    false,
  );
});
