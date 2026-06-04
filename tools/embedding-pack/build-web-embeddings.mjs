#!/usr/bin/env node

import { brotliCompress } from "node:zlib";
import { createHash } from "node:crypto";
import { mkdir, readFile, rename, rm, stat, writeFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { promisify } from "node:util";

const brotli = promisify(brotliCompress);

const SCHEMA_VERSION = 1;
const MODEL_ID = "onnx-community/embeddinggemma-300m-ONNX";
const TRANSFORMERS_VERSION = "4.2.0";
const DEFAULT_DTYPES = ["q4", "q8"];
const DEFAULT_DIMENSIONS = 768;
const MAX_SEQUENCE_LENGTH = 2048;
const BATCH_SIZE = 8;
const CORPORA = [
  ["vlacku-en", "dictionary", "entry_index"],
  ["cukta-cll", "cll", "chunk_index"],
];

main().catch((error) => {
  console.error(error instanceof Error ? error.stack || error.message : String(error));
  process.exitCode = 1;
});

async function main() {
  const args = parseArgs(process.argv.slice(2));
  if (args.help) {
    printHelp();
    return;
  }
  const corpus = JSON.parse(await readFile(required(args.input, "--input"), "utf8"));
  const finalOutRoot = required(args.out, "--out");
  const outRoot = args.stage || `${finalOutRoot}.staging`;
  if (outRoot === finalOutRoot) {
    throw new Error("--stage must differ from --out");
  }
  const backend = args.backend || "transformers";
  const dtypes = args.dtype.length > 0 ? args.dtype : DEFAULT_DTYPES;
  await mkdir(outRoot, { recursive: true });
  await rm(join(outRoot, "catalog.json"), { force: true });
  await rm(join(outRoot, "catalog.json.br"), { force: true });

  const catalog = {
    schema_version: SCHEMA_VERSION,
    models: [],
  };
  const modelEntry = {
    model_key: corpus.modelKey,
    vector_spaces: [],
  };
  catalog.models.push(modelEntry);

  for (const dtype of dtypes) {
    const vectorSpaceKey = `transformers-js-${dtype}`;
    const packId = [
      corpus.inputFormatVersion,
      shortHash(corpus.modelRevision),
      shortHash(corpus.inputHash),
      vectorSpaceKey,
    ].join("-");
    const packRoot = join(
      outRoot,
      "models",
      corpus.modelKey,
      "spaces",
      vectorSpaceKey,
      "packs",
      packId,
    );
    const manifestUrl = `models/${corpus.modelKey}/spaces/${vectorSpaceKey}/packs/${packId}/manifest.json`;
    const reusableManifest = await readReusableManifest(
      join(packRoot, "manifest.json"),
      packRoot,
      corpus,
      vectorSpaceKey,
      packId,
      dtype,
      backend,
      args.dimensions,
    );
    if (reusableManifest !== null) {
      console.error(`reusing complete ${vectorSpaceKey} pack at ${packRoot}`);
      modelEntry.vector_spaces.push(catalogVectorSpace(manifestUrl, reusableManifest));
      continue;
    }
    await rm(packRoot, { recursive: true, force: true });
    await mkdir(packRoot, { recursive: true });
    const embedder = await createEmbedder({ backend, dtype, dimensions: args.dimensions });
    const corpora = [];
    for (const [corpusId, sourceKey, idField] of CORPORA) {
      corpora.push(await writeCorpus(packRoot, corpus, corpusId, sourceKey, idField, embedder));
    }
    const manifest = {
      schema_version: SCHEMA_VERSION,
      model_key: corpus.modelKey,
      model_revision: corpus.modelRevision,
      web_model: MODEL_ID,
      transformers_version: TRANSFORMERS_VERSION,
      vector_space_key: vectorSpaceKey,
      pack_id: packId,
      input_format_version: corpus.inputFormatVersion,
      input_hash: corpus.inputHash,
      built_by: {
        runtime: backend === "transformers" ? "node-transformers.js" : "fixture",
        version: backend === "transformers" ? TRANSFORMERS_VERSION : "test",
        dtype,
        device: backend === "transformers" ? "onnxruntime-node" : "fixture",
      },
      dimensions: embedder.dimensions,
      element_type: "f32le",
      normalized: true,
      distance: "dot",
      compatible_query_runtimes: [
        {
          runtime: "transformers.js",
          version: TRANSFORMERS_VERSION,
          dtype,
        },
      ],
      corpora,
    };
    await writeJson(join(packRoot, "manifest.json"), manifest);
    modelEntry.vector_spaces.push(catalogVectorSpace(manifestUrl, manifest));
  }
  await writeJson(join(outRoot, "catalog.json"), catalog);
  await promoteStagingOutput(outRoot, finalOutRoot);
}

function parseArgs(argv) {
  const args = { dtype: [] };
  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === "--help" || arg === "-h") {
      args.help = true;
    } else if (arg === "--input") {
      args.input = argv[++i];
    } else if (arg === "--out") {
      args.out = argv[++i];
    } else if (arg === "--dtype") {
      args.dtype.push(argv[++i]);
    } else if (arg === "--backend") {
      args.backend = argv[++i];
    } else if (arg === "--dimensions") {
      args.dimensions = Number.parseInt(argv[++i], 10);
    } else if (arg === "--stage") {
      args.stage = argv[++i];
    } else {
      throw new Error(`unknown argument: ${arg}`);
    }
  }
  return args;
}

function printHelp() {
  console.log(`Usage: node build-web-embeddings.mjs --input corpus.json --out dist/assets/embeddings/web/v1 [--stage dist/assets/embeddings/web/v1.staging] [--dtype q4] [--dtype q8] [--backend transformers|fixture]`);
}

function required(value, name) {
  if (!value) {
    throw new Error(`${name} is required`);
  }
  return value;
}

async function createEmbedder({ backend, dtype, dimensions }) {
  if (backend === "fixture") {
    return {
      dimensions: dimensions || 4,
      async embed(texts) {
        return texts.map((text) => normalize(fakeEmbedding(text, dimensions || 4)));
      },
    };
  }
  if (backend !== "transformers") {
    throw new Error(`unsupported backend: ${backend}`);
  }
  const { AutoModel, AutoTokenizer } = await import("@huggingface/transformers");
  const tokenizer = await AutoTokenizer.from_pretrained(MODEL_ID);
  const model = await AutoModel.from_pretrained(MODEL_ID, { dtype });
  return {
    dimensions: dimensions || DEFAULT_DIMENSIONS,
    async embed(texts) {
      const output = [];
      for (let start = 0; start < texts.length; start += BATCH_SIZE) {
        const batch = texts.slice(start, start + BATCH_SIZE);
        const inputs = await tokenizer(batch, {
          padding: true,
          truncation: true,
          max_length: MAX_SEQUENCE_LENGTH,
        });
        const result = await model(inputs);
        const rows = await result.sentence_embedding.tolist();
        for (const row of rows) {
          const vector = Float32Array.from(row);
          if (vector.length !== (dimensions || DEFAULT_DIMENSIONS)) {
            throw new Error(
              `Embedding dimension mismatch: expected ${dimensions || DEFAULT_DIMENSIONS}, got ${vector.length}`,
            );
          }
          output.push(normalize(vector));
        }
        console.error(`embedded ${Math.min(start + batch.length, texts.length)} of ${texts.length}`);
      }
      return output;
    },
  };
}

async function writeCorpus(packRoot, corpus, corpusId, sourceKey, idField, embedder) {
  const docs = corpus[sourceKey] || [];
  const vectors = await embedder.embed(docs.map((doc) => doc.input));
  const corpusDir = join(packRoot, "corpora", corpusId);
  await mkdir(corpusDir, { recursive: true });
  const items = docs.map((doc, row) => ({
    [idField]: doc.id,
    input_hash: doc.inputHash,
    kind: doc.kind || null,
    row,
  }));
  const itemsPath = join(corpusDir, "items.json");
  await writeJson(itemsPath, items);
  const vectorBytes = vectorsToBytes(vectors, embedder.dimensions);
  const vectorPath = join(corpusDir, "vectors.f32");
  await writeFileWithBrotli(vectorPath, vectorBytes);
  return {
    corpus_id: corpusId,
    input_format_version: corpus.inputFormatVersion,
    input_hash: corpusId === "vlacku-en" ? corpus.dictionaryHash : corpus.cllHash,
    row_count: docs.length,
    dimensions: embedder.dimensions,
    items_url: `corpora/${corpusId}/items.json`,
    items_sha256: sha256(await readFile(itemsPath)),
    vector_url: `corpora/${corpusId}/vectors.f32`,
    vector_byte_len: vectorBytes.byteLength,
    vector_sha256: sha256(vectorBytes),
  };
}

function catalogVectorSpace(manifestUrl, manifest) {
  return {
    vector_space_key: manifest.vector_space_key,
    latest_pack_id: manifest.pack_id,
    manifest_url: manifestUrl,
    compatible_query_runtimes: manifest.compatible_query_runtimes,
  };
}

async function readReusableManifest(
  manifestPath,
  packRoot,
  corpus,
  vectorSpaceKey,
  packId,
  dtype,
  backend,
  dimensions,
) {
  const manifest = await readJsonIfExists(manifestPath);
  if (manifest === null) {
    return null;
  }
  const expectedRuntime = backend === "transformers" ? "node-transformers.js" : "fixture";
  const expectedVersion = backend === "transformers" ? TRANSFORMERS_VERSION : "test";
  const expectedDevice = backend === "transformers" ? "onnxruntime-node" : "fixture";
  const expectedDimensions = dimensions || DEFAULT_DIMENSIONS;
  if (
    manifest.schema_version !== SCHEMA_VERSION ||
    manifest.model_key !== corpus.modelKey ||
    manifest.model_revision !== corpus.modelRevision ||
    manifest.web_model !== MODEL_ID ||
    manifest.transformers_version !== TRANSFORMERS_VERSION ||
    manifest.vector_space_key !== vectorSpaceKey ||
    manifest.pack_id !== packId ||
    manifest.input_format_version !== corpus.inputFormatVersion ||
    manifest.input_hash !== corpus.inputHash ||
    manifest.dimensions !== expectedDimensions ||
    manifest.element_type !== "f32le" ||
    manifest.normalized !== true ||
    manifest.distance !== "dot" ||
    manifest.built_by?.runtime !== expectedRuntime ||
    manifest.built_by?.version !== expectedVersion ||
    manifest.built_by?.dtype !== dtype ||
    manifest.built_by?.device !== expectedDevice ||
    !Array.isArray(manifest.compatible_query_runtimes) ||
    !Array.isArray(manifest.corpora)
  ) {
    return null;
  }
  for (const [corpusId, sourceKey, idField] of CORPORA) {
    const docs = corpus[sourceKey] || [];
    const corpusManifest = manifest.corpora.find((item) => item?.corpus_id === corpusId);
    if (!corpusManifest) {
      return null;
    }
    if (
      corpusManifest.input_format_version !== corpus.inputFormatVersion ||
      corpusManifest.input_hash !== (corpusId === "vlacku-en" ? corpus.dictionaryHash : corpus.cllHash) ||
      corpusManifest.row_count !== docs.length ||
      corpusManifest.dimensions !== expectedDimensions ||
      corpusManifest.items_url !== `corpora/${corpusId}/items.json` ||
      corpusManifest.vector_url !== `corpora/${corpusId}/vectors.f32`
    ) {
      return null;
    }
    const itemsPath = join(packRoot, corpusManifest.items_url);
    const vectorPath = join(packRoot, corpusManifest.vector_url);
    const itemsBytes = await readFileIfExists(itemsPath);
    const vectorBytes = await readFileIfExists(vectorPath);
    if (itemsBytes === null || vectorBytes === null) {
      return null;
    }
    if (
      !(await pathExists(`${itemsPath}.br`)) ||
      !(await pathExists(`${vectorPath}.br`)) ||
      corpusManifest.items_sha256 !== sha256(itemsBytes) ||
      corpusManifest.vector_sha256 !== sha256(vectorBytes) ||
      corpusManifest.vector_byte_len !== vectorBytes.byteLength
    ) {
      return null;
    }
    const items = JSON.parse(itemsBytes.toString("utf8"));
    if (
      !Array.isArray(items) ||
      items.length !== docs.length ||
      items.some((item, row) =>
        item?.[idField] !== docs[row].id ||
        item?.input_hash !== docs[row].inputHash ||
        item?.row !== row
      )
    ) {
      return null;
    }
  }
  return manifest;
}

async function readJsonIfExists(path) {
  const bytes = await readFileIfExists(path);
  return bytes === null ? null : JSON.parse(bytes.toString("utf8"));
}

async function readFileIfExists(path) {
  try {
    return await readFile(path);
  } catch (error) {
    if (error?.code === "ENOENT") {
      return null;
    }
    throw error;
  }
}

async function pathExists(path) {
  try {
    await stat(path);
    return true;
  } catch (error) {
    if (error?.code === "ENOENT") {
      return false;
    }
    throw error;
  }
}

async function promoteStagingOutput(stageRoot, finalOutRoot) {
  const backupRoot = `${finalOutRoot}.previous`;
  await rm(backupRoot, { recursive: true, force: true });
  let backupCreated = false;
  try {
    if (await pathExists(finalOutRoot)) {
      await rename(finalOutRoot, backupRoot);
      backupCreated = true;
    }
    await rename(stageRoot, finalOutRoot);
    if (backupCreated) {
      await rm(backupRoot, { recursive: true, force: true });
    }
  } catch (error) {
    if (backupCreated && !(await pathExists(finalOutRoot)) && (await pathExists(backupRoot))) {
      await rename(backupRoot, finalOutRoot).catch(() => {});
    }
    throw error;
  }
}

async function writeJson(path, value) {
  await writeFileWithBrotli(path, `${JSON.stringify(value, null, 2)}\n`);
}

async function writeFileWithBrotli(path, data) {
  await mkdir(dirname(path), { recursive: true });
  await writeFile(path, data);
  const compressed = await brotli(Buffer.isBuffer(data) ? data : Buffer.from(data), {
    params: {
      1: 5,
      2: 22,
    },
  });
  await writeFile(`${path}.br`, compressed);
}

function vectorsToBytes(vectors, dimensions) {
  const bytes = new ArrayBuffer(vectors.length * dimensions * 4);
  const view = new DataView(bytes);
  let offset = 0;
  for (const vector of vectors) {
    if (vector.length !== dimensions) {
      throw new Error(`vector dimension mismatch: expected ${dimensions}, got ${vector.length}`);
    }
    for (const value of vector) {
      view.setFloat32(offset, value, true);
      offset += 4;
    }
  }
  return Buffer.from(bytes);
}

function fakeEmbedding(text, dimensions) {
  const digest = createHash("sha256").update(text).digest();
  const vector = new Float32Array(dimensions);
  for (let i = 0; i < dimensions; i += 1) {
    vector[i] = (digest[i % digest.length] - 127) / 127;
  }
  return vector;
}

function normalize(vector) {
  let magnitude = 0;
  for (const value of vector) {
    magnitude += value * value;
  }
  magnitude = Math.sqrt(magnitude);
  if (magnitude > 0) {
    for (let i = 0; i < vector.length; i += 1) {
      vector[i] /= magnitude;
    }
  }
  return vector;
}

function sha256(data) {
  return createHash("sha256").update(data).digest("hex");
}

function shortHash(value) {
  return String(value).slice(0, 12);
}
