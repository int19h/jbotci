import assert from "node:assert/strict";
import test from "node:test";

globalThis.self = globalThis;

const {
  installPackForSetup,
  packCorpusCompatibilityIssue,
  searchPackCompatibilityError,
  statusDisplay,
} = await import("../assets/embedding-worker.js");
const {
  embeddingAutomaticSetupPayload,
  embeddingStatusShouldAutoUpdate,
} = await import("../assets/embeddings.js");

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
  assert.equal(
    embeddingAutomaticSetupPayload(
      { status: "needs-update", source: "remote" },
      "{\"corpus\":true}",
      false,
    )?.allowBrowserLocalBuild,
    false,
  );
  assert.equal(
    embeddingAutomaticSetupPayload(
      { status: "needs-update", source: "remote" },
      "{\"corpus\":true}",
      true,
    ),
    null,
  );
});

test("automatic remote repair never falls back to browser-local indexing", async (t) => {
  const structuredMissReasons = [
    "catalog-unavailable",
    "no-compatible-vector-space",
    "manifest-unavailable",
    "manifest-incompatible",
    "corpus-manifest-incompatible",
  ];
  const remoteResults = structuredMissReasons.map((reason) => ({
    name: reason,
    expectedReason: reason,
    loadRemotePack: async () => ({
      loaded: false,
      reason,
      detail: { test: reason },
    }),
  }));
  remoteResults.push(
    {
      name: "download failure",
      expectedReason: "remote-update-failed",
      loadRemotePack: async () => {
        throw new Error("remote vector download failed");
      },
    },
  );

  for (const remoteResult of remoteResults) {
    await t.test(remoteResult.name, async () => {
      let localBuildCount = 0;
      const outcome = await installPackForSetup({
        allowBrowserLocalBuild: false,
        loadRemotePack: remoteResult.loadRemotePack,
        buildLocalPack: async () => {
          localBuildCount += 1;
        },
      });

      assert.equal(localBuildCount, 0);
      assert.equal(outcome.installed, false);
      assert.equal(outcome.status, "needs-update");
      assert.match(outcome.detail, /outdated/i);
      assert.equal(outcome.remoteReason, remoteResult.expectedReason);
    });
  }
});

test("explicit setup retains browser-local fallback after a remote miss", async () => {
  let localBuildCount = 0;
  const outcome = await installPackForSetup({
    allowBrowserLocalBuild: true,
    loadRemotePack: async () => ({
      loaded: false,
      reason: "catalog-unavailable",
      detail: null,
    }),
    buildLocalPack: async () => {
      localBuildCount += 1;
    },
  });

  assert.equal(localBuildCount, 1);
  assert.equal(outcome.installed, true);
});

const { parseEntryEligibility, rankHits, rankHitsFromScores, rankSearchResults } = await import("../assets/embedding-worker.js");

test("vote eligibility precedes the result limit in both browser rankers", () => {
  const count = 20;
  const items = Array.from({ length: 42 }, (_, row) => ({ id: row + 100, kind: "gismu" }));
  const scores = Float32Array.from(items, (_, row) => 42 - row);
  const eligibility = parseEntryEligibility(JSON.stringify({ eligible: items.slice(count + 1).map((item) => item.id), withoutVoteFilter: items.map((item) => item.id) }));
  for (const rank of [
    (limit, allowed) => rankHits(scores, [1], items, 1, limit, [], allowed),
    (limit, allowed) => rankHitsFromScores(scores, items, limit, [], allowed),
  ]) {
    assert.deepEqual(rank(count + 1, null).map((hit) => hit.id), items.slice(0, count + 1).map((item) => item.id));
    const result = rankSearchResults(rank, count + 1, eligibility);
    assert.deepEqual(result.hits.map((hit) => hit.id), items.slice(count + 1).map((item) => item.id));
    assert.equal(result.voteFilterProbe, null);
    assert.equal(result.message, null);
  }
});

test("word-type eligibility precedes the result limit", () => {
  const items = Array.from({ length: 42 }, (_, id) => ({ id, kind: id < 21 ? "cmavo" : "gismu" }));
  const scores = Float32Array.from(items, (_, row) => 42 - row);
  for (const hits of [rankHits(scores, [1], items, 1, 21, ["gismu"]), rankHitsFromScores(scores, items, 21, ["gismu"])]) {
    assert.deepEqual(hits.map((hit) => hit.id), items.slice(21).map((item) => item.id));
  }
});

test("empty filtered results carry one unfiltered probe for the notice", () => {
  const items = [{ id: 7, kind: "lujvo" }];
  const rank = (count, allowed) => rankHitsFromScores([1], items, count, [], allowed);
  const result = rankSearchResults(rank, 21, parseEntryEligibility('{"eligible":[],"withoutVoteFilter":[7]}'));
  assert.deepEqual(result.hits, []);
  assert.deepEqual(result.voteFilterProbe, { id: 7, score: 1 });
  assert.equal(result.message, null);
  const empty = rankSearchResults(rank, 21, parseEntryEligibility('{"eligible":[],"withoutVoteFilter":[]}'));
  assert.equal(empty.voteFilterProbe, null);
  assert.equal(empty.message, "No matches found.");
  for (const json of ['{}', '{"eligible":[-1],"withoutVoteFilter":[-1]}', '{"eligible":[1],"withoutVoteFilter":[]}']) {
    assert.throws(() => parseEntryEligibility(json));
  }
});
