# F2LLM runtime oracles and acceptance gates

This document freezes the N0 evidence for issue #695 and the commands that N1–N5 must
make pass. N0 defines contracts and oracles only: it does not move runtime code or wire
any consumer to `jbotci-f2llm-runtime`.

## Published pack snapshot

The durable snapshot is:

`/home/int19h.linux/artifacts/jbotci/issue-695/published-web-packs-2026-07-30`

It was captured from R2 bucket `jbotci-web-assets`, prefix `embeddings/web/v1`, at
`2026-07-30T05:10:45Z`. The catalog was fetched before and after the capture and was
byte-identical. Its six `manifest_url` values were followed; every referenced item file
and vector shard was captured and checked against the manifest. Vector byte lengths
were also checked as `row_count * dimensions * element_width`.

The snapshot has 31 R2 objects totaling 274,341,161 bytes:

| Kind | Objects | Bytes |
| --- | ---: | ---: |
| catalog | 1 | 5,469 |
| pack manifest | 6 | 14,654 |
| item file | 12 | 20,289,678 |
| vector shard | 12 | 254,031,360 |

This includes both published EmbeddingGemma packs and all four F2LLM sizes. The
machine-readable inventory is `snapshot.json`; `SHA256SUMS` covers every captured R2
object.

- `snapshot.json`: `d34980e9d1fbc9b2f71eb1ba1b7a78a5ca99d413cb0fabb54b66eecb26674206`
- `SHA256SUMS`: `c717f296aad9347ed7fa3d65050e90d0ee3ed7f2e09c3dafcf869f32f5d4a725`

Verify it with:

```sh
cd /home/int19h.linux/artifacts/jbotci/issue-695/published-web-packs-2026-07-30
sha256sum -c SHA256SUMS
```

The published 80m ONNX and all four current WebGPU manifest bytes have separate durable
captures under the same issue directory. The 80m model is exactly 55,252,118 bytes with
SHA-256 `00ec8cc51400b74b0d215b794536a81a24f9002926c340ebde139092b3a36cc6`.

## Golden provenance

`testdata/goldens/provenance.json` binds every legacy fixture to exact vendored
generator, reference harness, golden, and source-artifact manifest bytes.

The three prototype fixture sets remain byte-for-byte `0.1.0` evidence. Their q4 ONNX
digests survive in the captured published v1 pack manifests, but those ONNX bytes are
unrecoverable and are explicitly marked unavailable. Current artifact manifest digests
are recorded separately and differ from every prototype `0.1.0` manifest. Consequently,
the old vectors are not presented as `0.2.0` compatibility evidence.

The `f2llm-v2-80m-q4-320` fixture is independently generated for runtime/artifact
version `0.2.0` from the published ONNX using onnxruntime 1.28.0. It covers:

- empty and genuinely non-ASCII input;
- exact post-EOS token counts 511, 512, and 513;
- a 1,025-token document with windows `[512, 512, 1]`;
- the last slot of an eight-window inference batch and the first slot of the next one.

It records exact token IDs, window structure, per-window normalized embeddings, final
mean-of-normalized-windows embeddings, and f32le digests. Two clean generator runs were
byte-identical at:

`1af849624dd143f1d447254fc8815c57a921a17d9237d4e572d8024de1b596d3`

Regenerate it only from the durable published inputs:

```sh
/build/jbotci/scratch/f2llm-venv/bin/python \
  tools/f2llm-oracles/generate-f2llm-goldens.py \
  --out /build/jbotci/scratch/issue-695/f2llm-v2-80m-q4-320-goldens.json \
  --q4-onnx /home/int19h.linux/artifacts/jbotci/issue-695/published-80m-onnx-2026-07-30/models/f2llm-v2-80m-onnx-q4/v1/model_q4.onnx \
  --onnx-manifest /home/int19h.linux/artifacts/jbotci/issue-695/published-80m-onnx-2026-07-30/models/f2llm-v2-80m-onnx-q4/v1/manifest.json \
  --artifact-manifest /home/int19h.linux/artifacts/jbotci/issue-695/published-webgpu-manifests-2026-07-30/models/f2llm-v2-80m-webgpu/v1/manifest.json \
  --expected-q4-sha256 00ec8cc51400b74b0d215b794536a81a24f9002926c340ebde139092b3a36cc6 \
  --expected-onnx-manifest-sha256 c034193d12df6a623cfa00b8661752a42e82e3fd705d4f073356068b194e9bb3 \
  --expected-artifact-manifest-sha256 f25482d5612b2f74f5b76739eb33bdb52862866918cc7a5e4fb7dfb3aa06c6c2
```

## N1–N5 command contract

These command names and thresholds are the acceptance interface for the later issues.
Where an N0 command does not exist yet, the named issue must add it rather than replacing
the gate with an ad hoc comparison. All commands use
`CARGO_TARGET_DIR=/build/jbotci/target/<issue>` and scratch data under
`/build/jbotci/scratch/<issue>`.

### N1 — wasm-only extraction

```sh
CARGO_TARGET_DIR=/build/jbotci/target/<issue> cargo test -r -p jbotci-f2llm-runtime --test pure_core
CARGO_TARGET_DIR=/build/jbotci/target/<issue> cargo run -r -p xtask-full -- \
  f2llm-extraction-gate \
  --before /build/jbotci/scratch/<issue>/before.json \
  --after /build/jbotci/scratch/<issue>/after.json \
  --require-bit-identical-f32 \
  --require-exact-token-ids \
  --require-exact-windows
CARGO_TARGET_DIR=/build/jbotci/target/<issue> cargo run -r -p xtask-full -- \
  f2llm-wasm-export-gate \
  --require jbotciF2LlmWebGpuRuntimeLoad \
  --require jbotciF2LlmTokenizerLoad \
  --require embedTexts \
  --require scoreF16Vectors
CARGO_TARGET_DIR=/build/jbotci/target/<issue> cargo tree -p jbotci-ui --target aarch64-unknown-linux-gnu \
  | tee /build/jbotci/scratch/<issue>/jbotci-ui-native-tree.txt
! rg '(^| )wgpu v|jbotci-f2llm-runtime' \
  /build/jbotci/scratch/<issue>/jbotci-ui-native-tree.txt
```

### N2 — native bring-up

```sh
CARGO_TARGET_DIR=/build/jbotci/target/<issue> cargo test -r -p jbotci-f2llm-runtime --test pure_core
VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json \
  CARGO_TARGET_DIR=/build/jbotci/target/<issue> \
  cargo test -r -p jbotci-f2llm-runtime --features native-wgpu \
  --test native_goldens -- --nocapture
CARGO_TARGET_DIR=/build/jbotci/target/<issue> cargo run -r -p xtask-full -- \
  f2llm-golden-gate \
  --goldens crates/jbotci-f2llm-runtime/testdata/goldens \
  --target native \
  --min-cosine 0.999 \
  --require-exact-token-ids \
  --require-exact-windows \
  --report-wasm-native-cosine /build/jbotci/scratch/<issue>/wasm-native.json
```

Adapter absence is a failure. Any wasm/native cosine below 0.999 requires investigation
and a recorded report; the shared ONNX-reference minimum of 0.999 is a hard failure and
is never loosened to retain a vector-space key.

### N3 — native builder and staging

```sh
CARGO_TARGET_DIR=/build/jbotci/target/<issue> cargo run -r -p xtask-full -- \
  export-web-embedding-corpus \
  --output /build/jbotci/scratch/<issue>/corpus.json
VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json \
  CARGO_TARGET_DIR=/build/jbotci/target/<issue> \
  cargo run -r -p xtask-full -- build-web-vectors-native \
  --corpus /build/jbotci/scratch/<issue>/corpus.json \
  --artifact-root /build/jbotci/scratch/<issue>/published-artifacts \
  --models f2llm-v2-80m-q4-320,f2llm-v2-160m-q4-640,f2llm-v2-330m-q4-896,f2llm-v2-0.6b-q4-1024 \
  --stage /build/jbotci/scratch/<issue>/native-stage \
  --batch-size 8
CARGO_TARGET_DIR=/build/jbotci/target/<issue> cargo run -r -p xtask-full -- \
  f2llm-pack-gate \
  --old /home/int19h.linux/artifacts/jbotci/issue-695/published-web-packs-2026-07-30/embeddings/web/v1 \
  --new /build/jbotci/scratch/<issue>/native-stage \
  --models f2llm-v2-80m-q4-320,f2llm-v2-160m-q4-640,f2llm-v2-330m-q4-896,f2llm-v2-0.6b-q4-1024 \
  --join corpus,document-id,input-hash \
  --require-all-rows \
  --min-cosine 0.999 \
  --max-component-error 0.01
CARGO_TARGET_DIR=/build/jbotci/target/<issue> cargo run -r -p xtask-full -- \
  f2llm-retrieval-gate \
  --old /home/int19h.linux/artifacts/jbotci/issue-695/published-web-packs-2026-07-30/embeddings/web/v1 \
  --new /build/jbotci/scratch/<issue>/native-stage \
  --max-score-error 0.002 \
  --max-inversion-old-gap 0.004 \
  --max-top-k-boundary-gap 0.004
CARGO_TARGET_DIR=/build/jbotci/target/<issue> cargo test -r -p jbotci-f2llm-runtime \
  f32_to_f16_matches_numpy_halfway_and_special_values
CARGO_TARGET_DIR=/build/jbotci/target/<issue> cargo run -r -p xtask-full -- \
  f2llm-80m-python-double-check \
  --published-onnx /home/int19h.linux/artifacts/jbotci/issue-695/published-80m-onnx-2026-07-30/models/f2llm-v2-80m-onnx-q4/v1/model_q4.onnx \
  --native-pack /build/jbotci/scratch/<issue>/native-stage \
  --min-cosine 0.999 \
  --max-component-error 0.01
```

The builder writes staging only. The corpus DTO must reject a changed document,
aggregate fingerprint, or noncanonical document ID after recomputing all hashes.

### N4 — dual-schema worker web release

```sh
CARGO_TARGET_DIR=/build/jbotci/target/<issue> cargo test -r -p jbotci-f2llm-runtime manifest_
CARGO_TARGET_DIR=/build/jbotci/target/<issue> cargo test -r -p jbotci-f2llm-runtime torn_catalog_manifest
CARGO_TARGET_DIR=/build/jbotci/target/<issue> cargo test -r -p jbotci-ui \
  embedding_worker_dual_schema
CARGO_TARGET_DIR=/build/jbotci/target/<issue> cargo run -r -p xtask-full -- build-web-release
CARGO_TARGET_DIR=/build/jbotci/target/<issue> cargo run -r -p xtask-full -- \
  f2llm-worker-release-gate \
  --release .jbotci-build/jbotci-web \
  --old-catalog /home/int19h.linux/artifacts/jbotci/issue-695/published-web-packs-2026-07-30/embeddings/web/v1/catalog.json \
  --require-normal-cache \
  --require-service-worker-activation \
  --require-v1 \
  --require-v2
```

The catalog/manifest checks cover model key, vector-space key, and pack ID, including
torn-pair failures.

### N5 — publication and rollback rehearsal

```sh
CARGO_TARGET_DIR=/build/jbotci/target/<issue> cargo run -r -p xtask-full -- \
  publish-f2llm-webgpu-r2 \
  --bucket jbotci-web-assets \
  --embedding-prefix embeddings/web/v1 \
  --corpus /build/jbotci/scratch/<issue>/corpus.json \
  --model-out-root /build/jbotci/scratch/<issue>/models \
  --vector-out-dir /build/jbotci/scratch/<issue>/native-stage \
  --skip-build
CARGO_TARGET_DIR=/build/jbotci/target/<issue> cargo run -r -p xtask-full -- \
  f2llm-published-browser-gate \
  --catalog https://assets.jbotci.app/embeddings/web/v1/catalog.json \
  --models f2llm-v2-80m-q4-320,f2llm-v2-160m-q4-640,f2llm-v2-330m-q4-896,f2llm-v2-0.6b-q4-1024 \
  --edge-cache-poll \
  --require-service-worker-activation
npx --yes wrangler@latest r2 object put \
  jbotci-web-assets/embeddings/web/v1/catalog.json \
  --file /home/int19h.linux/artifacts/jbotci/issue-695/published-web-packs-2026-07-30/embeddings/web/v1/catalog.json \
  --content-type application/json \
  --remote
cmp \
  /home/int19h.linux/artifacts/jbotci/issue-695/published-web-packs-2026-07-30/embeddings/web/v1/catalog.json \
  <(npx --yes wrangler@latest r2 object get \
      jbotci-web-assets/embeddings/web/v1/catalog.json --remote --pipe)
```

N5 uploads only unique object URLs and publishes the catalog last. The final two
commands are the required rollback rehearsal; do not run them before the N5 publication
authorization. Old objects are retained.
