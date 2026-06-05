# F2LLM WebGPU embedding artifacts

This directory contains offline-only tooling for the browser F2LLM embedding path.
The generated files are intended to be published under `assets.jbotci.app`; they
are not loaded through ONNX Runtime or Transformers.js in the browser.

Build the q4 model artifact:

```sh
python3 -m venv .venv-f2llm-webgpu
. .venv-f2llm-webgpu/bin/activate
pip install -r tools/webgpu-embedding-runtime/requirements.txt
python tools/webgpu-embedding-runtime/export-f2llm-webgpu-from-onnx-q4.py \
  --out .jbotci-build/f2llm-v2-80m-webgpu/v1
```

The ONNX-q4 exporter consumes the existing quantized Transformers.js trial model
from `/home/int19h.linux/git/jbotci-f2llm-quant` by default. It repacks the
`MatMulNBits` and `GatherBlockQuantized` initializers directly into small
WebGPU shards, preserving ONNX q4 bytes, scales, zero-points, block size, and
low-nibble-first packing. The resulting artifact avoids ONNX Runtime in the app
while using the same quantization as the validated q4 ONNX model.

The older rowwise symmetric exporter is still available for comparison:

```sh
python tools/webgpu-embedding-runtime/export-f2llm-webgpu.py \
  --out .jbotci-build/f2llm-v2-80m-webgpu-rowwise/v1
```

Build the optional browser ONNX reference model used by the harness on macOS:

```sh
python tools/webgpu-embedding-runtime/export-f2llm-onnx-reference.py \
  --out .jbotci-build/f2llm-onnx-reference/v1
```

Build the matching remote vector pack from an exported web embedding corpus:

```sh
python tools/webgpu-embedding-runtime/build-f2llm-vector-pack.py \
  --input .jbotci-build/web-embedding-corpus.json \
  --out .jbotci-build/r2-web-embeddings
```

The browser expects the model artifact at
`https://assets.jbotci.app/models/f2llm-v2-80m-webgpu/v1` and the vector catalog
under the normal web embedding base URL.

Run container-side checks:

```sh
node tools/webgpu-embedding-runtime/test-tokenizer.mjs
node tools/webgpu-embedding-runtime/test-runtime-reference.mjs
node tools/webgpu-embedding-runtime/test-f2llm-upstream-header.mjs
python3 -m py_compile \
  tools/webgpu-embedding-runtime/export-f2llm-webgpu.py \
  tools/webgpu-embedding-runtime/export-f2llm-webgpu-from-onnx-q4.py \
  tools/webgpu-embedding-runtime/export-f2llm-onnx-reference.py \
  tools/webgpu-embedding-runtime/validate-f2llm-webgpu-artifact.py \
  tools/webgpu-embedding-runtime/build-f2llm-vector-pack.py \
  tools/webgpu-embedding-runtime/generate-f2llm-goldens.py
python tools/webgpu-embedding-runtime/validate-f2llm-webgpu-artifact.py
```

The CPU reference test uses a tiny deterministic Qwen3-shaped model and checks
q4 packing/dequantization, Q/K RMSNorm, RoPE, causal attention, SwiGLU, final
pooling, normalization, and f16 vector ranking. It does not replace a WebGPU
run, but it catches artifact-layout and operator-order mistakes in this
container.

Manual WebGPU harness:

```sh
node tools/webgpu-embedding-runtime/browser-harness/server.mjs
```

Open
`http://127.0.0.1:7777/tools/webgpu-embedding-runtime/browser-harness/` on
macOS. The harness defaults to local artifacts under
`.jbotci-build/f2llm-v2-80m-webgpu/v1` and local vector packs under
`.jbotci-build/r2-web-embeddings`. The golden embedding URL defaults to
`.jbotci-build/f2llm-webgpu-goldens/goldens.json`; use the `Run Golden Set`
button for static PyTorch/Transformers reference checks. The `Run ONNX
References` button loads `.jbotci-build/f2llm-onnx-reference/v1/model.onnx`
and, when available, the existing q4 Transformers.js trial model from
`/home/int19h.linux/git/jbotci-f2llm-quant/artifacts/f2llm-v2-80m-q4-hqq32-transformersjs/onnx/model_q4.onnx`
through local ORT Web. The q4 model emits `last_hidden_state`; the harness uses
last-token pooling and L2 normalization before comparing it with the fp32 ONNX
wrapper and the custom WebGPU runtime. `Run ONNX Golden Set` repeats that
comparison across all golden cases and expects the custom WebGPU runtime to
match q4 ONNX closely. Pass `--q4-onnx <path>` to the harness server to test a
different q4 ONNX file. Use the `artifact` and `vectors` URL
fields only when intentionally testing a published asset origin or a full
vector-pack search flow. For iOS, serve the same worktree from an HTTPS origin,
because WebGPU requires a secure context on non-localhost origins. The harness
accepts these query parameters: `runtime`, `artifact`, `vectors`, `corpus`,
`query`, `expected`, `ort`, `ortWasm`, `onnx`, and `onnxQ4`.

During a manual run, click `Run All`, then `Repeat Embed`, then `Download Log
JSON`. Attach that JSON when reporting results. It includes user agent and
WebGPU adapter limits, storage/heap snapshots where available, model/vector
manifest summaries, fetch timings, token count, embedding hash/statistics,
search hits, and repeated-run timings.

Optional expected embedding JSON shape:

```json
{
  "embedding": [0.01, -0.02],
  "threshold": 0.98
}
```
