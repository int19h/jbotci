# F2LLM runtime oracles

An oracle is reference output used to assess another implementation.
The F2LLM tests use pinned token IDs, window boundaries, and embedding vectors.
`jbotci-f2llm-runtime` contains the shared runtime and the oracle data.

## Golden data and provenance

Golden data is fixed reference output for test inputs.
`crates/jbotci-f2llm-runtime/testdata/goldens/provenance.json` binds the golden files to their generators, reference harnesses, and source manifests.
The tests compare the recorded hashes with the embedded files.

The `legacy-v0.1.0` sets contain prototype vectors for three model sizes.
Their source ONNX bytes are unavailable, as the provenance file records.
These sets do not establish compatibility with runtime version `0.2.0`.

The `current-v0.2.0/f2llm-v2-80m-q4-320` set uses the published 80m q4 ONNX model with onnxruntime 1.28.0.
It records token IDs, windows, normalized window vectors, final vectors, and little-endian f32 digests.
The final vector is the normalized mean of the normalized window vectors.
Its inputs cover these boundaries:

- Empty and non-ASCII text.
- Post-EOS token counts of 511, 512, and 513.
- A 1,025-token document with windows `[512, 512, 1]`.
- The last slot of one eight-window batch and the first slot of the next batch.

Do not edit pinned files to accept a runtime difference.
Use `tools/f2llm-oracles/generate-f2llm-goldens.py` with the exact source manifests and hashes to reproduce the 80m set.
The provenance file supplies those identities.

## Runtime capabilities

`RuntimeCapabilities::EmbeddingOnly` permits embedding without the `shader-f16` adapter feature.
It does not compile the f16 vector-scoring pipeline.
`EmbeddingAndF16Scoring` requires that feature and supports both operations.
The browser worker selects `EmbeddingAndF16Scoring` by default.

The browser evidence page requests embedding-only capability directly.
It does not require a source patch.
The native golden test also selects embedding-only capability.

## Pure-core tests

From the repository root, operate the pure-core tests in release mode:

```sh
CARGO_TARGET_DIR=/build/jbotci/target/f2llm-oracles cargo test -r -p jbotci-f2llm-runtime --test pure_core
```

These tests exercise the runtime data and provenance without a GPU adapter.
They do not replace execution on a GPU.
GPU comparisons require the exact artifact bytes identified by the manifests.

## Native golden tests

`tools/f2llm-oracles/download-webgpu-artifacts.py` downloads a manifest and its objects into a local artifact directory.
It resolves object identities by byte length and SHA-256.
It refuses missing or mismatched payloads.
Its `--runtime-manifest` argument selects the manifest used by an oracle set.
For legacy sets, it matches published objects by byte length and SHA-256 and writes the exact vendored runtime manifest.

Prepare model directories under `/build/jbotci/scratch/f2llm-native-artifacts` before the native tests.
Use the manifests that correspond to the selected golden data.
For all four models, operate:

```sh
WGPU_BACKEND=vulkan \
  JBOTCI_F2LLM_ARTIFACT_ROOT=/build/jbotci/scratch/f2llm-native-artifacts \
  CARGO_TARGET_DIR=/build/jbotci/target/f2llm-oracles \
  cargo test -r -p jbotci-f2llm-runtime --features native \
  --test native_goldens -- --nocapture
```

The test records the selected adapter and comparison results.
Make sure that the adapter matches the intended test host.
An absent adapter fails the test.
The minimum cosine similarity to the ONNX reference is 0.999.

For an 80m test through the software Vulkan adapter, operate:

```sh
WGPU_BACKEND=vulkan \
  VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json \
  JBOTCI_F2LLM_FORCE_FALLBACK_ADAPTER=1 \
  JBOTCI_F2LLM_GOLDEN_MODE=80m \
  JBOTCI_F2LLM_ARTIFACT_ROOT=/build/jbotci/scratch/f2llm-native-artifacts \
  CARGO_TARGET_DIR=/build/jbotci/target/f2llm-oracles \
  cargo test -r -p jbotci-f2llm-runtime --features native \
  --test native_goldens -- --nocapture
```

`JBOTCI_F2LLM_NATIVE_EVIDENCE` overrides the evidence output path.
The default output is `f2llm-native-goldens.json` in the Cargo target directory.
Use a separate build lane and scratch directory for each comparison.

## Browser evidence

`tools/f2llm-oracles/wasm-webgpu-extraction-evidence.html` loads the tokenizer and tensor artifacts through the JavaScript runtime interface.
It sends the golden inputs through `embedTexts`.
It records token IDs, windows, vector bytes, vector hashes, and adapter features.

Build the app from `apps/jbotci-app` with a dedicated target directory:

```sh
CARGO_TARGET_DIR=/build/jbotci/target/f2llm-oracles dx build
```

Serve the built public directory, evidence page, golden data, and artifact directory from one local HTTP origin.
Use the page query parameters `module`, `wasm`, `goldens`, `artifacts`, and `implementation` to identify those inputs.
`tools/f2llm-oracles/run-wasm-webgpu-browser-evidence.mjs` operates the page through Chrome DevTools.
It waits for GPU work and readback before it writes evidence.

## Comparison commands

`xtask-full` provides three comparison commands.
`f2llm-extraction-gate` compares browser evidence from two builds.
It requires `--require-bit-identical-f32`, `--require-exact-token-ids`, and `--require-exact-windows`.
It compares artifact and runtime identities, adapter features, progress counts, vector bytes, and vector digests.
The `implementation` fields can differ between the two builds.

`f2llm-golden-gate` compares runtime evidence with golden data.
It requires exact token IDs and windows through `--require-exact-token-ids` and `--require-exact-windows`.
Use `--min-cosine 0.999` for the ONNX reference comparison.
It also requires browser evidence through `--wasm-evidence` and a report path through `--report-wasm-native-cosine`.
A native/browser cosine similarity below 0.999 requires investigation and a recorded report.
The command reports that difference without reducing the ONNX reference threshold.

`f2llm-wasm-export-gate` examines the built WebAssembly exports and the JavaScript functions that call them.
Use `--require` for `jbotciF2LlmWebGpuRuntimeLoad`, `jbotciF2LlmTokenizerLoad`, `embedTexts`, and `scoreF16Vectors`.
Each JavaScript function must call its corresponding real WebAssembly export.

From the repository root, show the command arguments:

```sh
CARGO_TARGET_DIR=/build/jbotci/target/f2llm-oracles cargo run -r -p xtask-full -- f2llm-extraction-gate --help
CARGO_TARGET_DIR=/build/jbotci/target/f2llm-oracles cargo run -r -p xtask-full -- f2llm-golden-gate --help
CARGO_TARGET_DIR=/build/jbotci/target/f2llm-oracles cargo run -r -p xtask-full -- f2llm-wasm-export-gate --help
```

Record exact commits, manifests, adapter details, and command arguments with comparison results.
Investigate a vector difference before accepting compatibility.
Do not lower the reference threshold to preserve a vector-space identity.
