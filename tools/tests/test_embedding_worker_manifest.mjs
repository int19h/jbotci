import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

const worker = await readFile(new URL("../../crates/jbotci-ui/assets/embedding-worker.js", import.meta.url), "utf8");
const manifest = JSON.parse(await readFile(new URL("../../crates/jbotci-f2llm-runtime/testdata/manifests/v1/incremental-80m.json", import.meta.url), "utf8"));
const runtimeSource = await readFile(new URL("../../crates/jbotci-f2llm-runtime/src/lib.rs", import.meta.url), "utf8");
const uiSource = await readFile(new URL("../../crates/jbotci-ui/src/lib.rs", import.meta.url), "utf8");
const releaseVersion = runtimeSource.match(/F2LLM_RUNTIME_VERSION: &str = "([^"]+)"/)[1];
const releaseSpace = uiSource.match(/F2LLM_BROWSER_VECTOR_SPACE_KEY: &str = "([^"]+)"/)[1];
assert.equal(releaseVersion, "0.3.0");
assert.equal(releaseSpace, "jbotci-browser-f2llm-q4-f16-windowed-512-v2");
for (const script of ["build-vector-pack.py", "validate-vector-pack.py"]) {
  const source = await readFile(new URL(`../embedding-pack/f2llm/${script}`, import.meta.url), "utf8");
  assert.ok(source.includes(`RUNTIME_VERSION = "${releaseVersion}"`));
  assert.ok(source.includes(`VECTOR_SPACE_KEY = "${releaseSpace}"`));
}
// Run the complete worker as a module with a passive worker host. No model or
// network request runs: this exercises its actual manifest compatibility path.
const harness = `
modelCatalog = { defaultDesktopModelKey: manifest.model_key, models: {
    [manifest.model_key]: { modelKey: manifest.model_key, dimensions: manifest.dimensions,
        vectorElementType: manifest.element_type, outputPooling: manifest.pooling,
        maxSequenceLength: manifest.max_sequence_length }
} };
const corpus = { inputHash: manifest.input_hash, inputFormatVersion: manifest.input_format_version };
assert.equal(manifestCompatibilityIssue(manifest, corpus, manifest.compatible_query_runtimes[0]), null);
assert.equal(manifestCompatibilityIssue({ ...manifest, dimensions: 1 }, corpus,
    manifest.compatible_query_runtimes[0]).field, "dimensions");
const releaseRuntime = { ...manifest.compatible_query_runtimes[0], version: releaseVersion };
assert.equal(manifestCompatibilityIssue(manifest, corpus, releaseRuntime).field, "compatible_query_runtimes");
const identity = { modelRevision: "revision", modelSha256: "a".repeat(64),
  tokenizerSha256: "b".repeat(64), artifactManifestSha256: "c".repeat(64) };
modelRuntime = { modelIdentity: identity };
modelCatalog.models[manifest.model_key].localVectorSpaceKey = releaseSpace;
const oldPack = { source: "browser", modelKey: manifest.model_key, modelIdentity: identity,
  compatibleQueryRuntimes: manifest.compatible_query_runtimes, inputHash: corpus.inputHash,
  inputFormatVersion: corpus.inputFormatVersion, vectorSpaceKey: manifest.vector_space_key,
  pooling: manifest.pooling, maxWindowTokens: manifest.max_window_tokens };
assert.equal(packCompatibleWithRuntime(oldPack, releaseRuntime), false);
assert.equal(cachedPackMatchesCorpus(oldPack, corpus, releaseRuntime, releaseSpace), false);
const context = { ...corpus, modelIdentity: identity, runtime: releaseRuntime, vectorSpaceKey: releaseSpace, packId: "pack" };
assert.equal(partialLocalBuildCompatible({ ...oldPack, source: "browser-partial", packId: "pack" }, context), false);
const currentPack = { ...oldPack, vectorSpaceKey: releaseSpace, compatibleQueryRuntimes: [releaseRuntime] };
assert.equal(cachedPackMatchesCorpus(currentPack, corpus, releaseRuntime, releaseSpace), true);

`;
const module = `import assert from 'node:assert/strict';
globalThis.self = {};
const manifest = ${JSON.stringify(manifest)};
const releaseVersion = ${JSON.stringify(releaseVersion)};
const releaseSpace = ${JSON.stringify(releaseSpace)};
${worker.replaceAll("import.meta.url", JSON.stringify(new URL("../../crates/jbotci-ui/assets/embedding-worker.js", import.meta.url).href))}
${harness}`;
await import(`data:text/javascript;base64,${Buffer.from(module).toString("base64")}`);
assert.ok(true);
console.log("embedding worker accepts incremental manifest metadata");
