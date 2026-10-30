import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

const worker = await readFile(new URL("../../crates/jbotci-ui/assets/embedding-worker.js", import.meta.url), "utf8");
const manifest = JSON.parse(await readFile(new URL("../../crates/jbotci-f2llm-runtime/testdata/manifests/v1/incremental-80m.json", import.meta.url), "utf8"));
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
`;
const module = `import assert from 'node:assert/strict';
globalThis.self = {};
const manifest = ${JSON.stringify(manifest)};
${worker.replaceAll("import.meta.url", JSON.stringify(new URL("../../crates/jbotci-ui/assets/embedding-worker.js", import.meta.url).href))}
${harness}`;
await import(`data:text/javascript;base64,${Buffer.from(module).toString("base64")}`);
assert.ok(true);
console.log("embedding worker accepts incremental manifest metadata");
