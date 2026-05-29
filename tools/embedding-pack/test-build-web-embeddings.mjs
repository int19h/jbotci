#!/usr/bin/env node

import { strict as assert } from "node:assert";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { spawnSync } from "node:child_process";

const root = await mkdtemp(join(tmpdir(), "jbotci-web-embeddings-"));
try {
  const input = join(root, "corpus.json");
  const out = join(root, "out");
  await writeFile(
    input,
    JSON.stringify({
      modelKey: "embedding-gemma-300m-q4-768",
      modelRevision: "8dd0ca2a66a8f14470acb0e2a71f801afbc5fb73",
      inputFormatVersion: "egemma-v0-parity-test",
      inputHash: "a".repeat(64),
      dictionaryHash: "b".repeat(64),
      cllHash: "c".repeat(64),
      dictionary: [
        { id: 0, input: "title: klama | text: go", inputHash: "d".repeat(64), kind: "gismu" },
      ],
      cll: [
        { id: 0, input: "title: 1 | text: grammar", inputHash: "e".repeat(64), kind: "section" },
      ],
    }),
  );
  const result = spawnSync(
    process.execPath,
    [
      new URL("./build-web-embeddings.mjs", import.meta.url).pathname,
      "--input",
      input,
      "--out",
      out,
      "--backend",
      "fixture",
      "--dimensions",
      "4",
      "--dtype",
      "q4",
    ],
    { stdio: "inherit" },
  );
  assert.equal(result.status, 0);
  const catalog = JSON.parse(await readFile(join(out, "catalog.json"), "utf8"));
  assert.equal(catalog.schema_version, 1);
  const vectorSpace = catalog.models[0].vector_spaces[0];
  assert.equal(vectorSpace.vector_space_key, "transformers-js-q4");
  const manifestPath = join(out, vectorSpace.manifest_url);
  const manifest = JSON.parse(await readFile(manifestPath, "utf8"));
  assert.equal(manifest.vector_space_key, "transformers-js-q4");
  assert.equal(manifest.dimensions, 4);
  assert.equal(manifest.corpora.length, 2);
  assert.equal(manifest.corpora[0].vector_byte_len, 16);
  assert.equal(manifest.corpora[0].vector_sha256.length, 64);
  await readFile(`${manifestPath}.br`);
} finally {
  await rm(root, { recursive: true, force: true });
}
