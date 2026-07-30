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

Prepare the 160M, 330M, and 0.6B prototype assets:

```sh
cd /home/int19h.linux/git/jbotci-f2llm-webgpu-prototype
/home/int19h.linux/git/jbotci-f2llm-quant/.venv/bin/python \
  webgpu-embedding-runtime/prepare-f2llm-size.py \
  --sizes 160m 330m 0.6b
```

This writes local WebGPU artifacts under `local-assets/models/`, q4 ONNX
comparison models under `local-assets/q4-onnx/`, q4-derived golden embeddings
under `local-assets/goldens/`, and a combined `local-assets/prepare-summary.json`.
The heavier fp32/q4 ONNX build intermediates live in
`/home/int19h.linux/git/jbotci-f2llm-quant/artifacts`.

Current generated local assets:

| Model | Key | WebGPU artifact | q4 ONNX size |
| --- | --- | ---: | ---: |
| F2LLM-v2-160M | `f2llm-v2-160m-q4-640` | ~110 MB | 109,340,622 bytes |
| F2LLM-v2-330M | `f2llm-v2-330m-q4-896` | ~231 MB | 236,935,306 bytes |
| F2LLM-v2-0.6B | `f2llm-v2-0.6b-q4-1024` | ~416 MB | 432,024,557 bytes |

Manual WebGPU harness:

```sh
cd /home/int19h.linux/git/jbotci-f2llm-webgpu-prototype
node webgpu-embedding-runtime/browser-harness/server.mjs
```

Open
`http://127.0.0.1:7777/webgpu-embedding-runtime/browser-harness/` on macOS.
The model-size selector rewrites the artifact, golden, and q4 ONNX fields to the
matching local assets. Direct URLs:

- `http://127.0.0.1:7777/webgpu-embedding-runtime/browser-harness/?model=160m`
- `http://127.0.0.1:7777/webgpu-embedding-runtime/browser-harness/?model=330m`
- `http://127.0.0.1:7777/webgpu-embedding-runtime/browser-harness/?model=0.6b`

The harness loads the production runtime from
`/home/int19h.linux/git/jbotci/apps/jbotci-web/assets/f2llm-webgpu-runtime.js`
through the local `__jbotci-main` route, and ORT Web from the main repo's
`tools/embedding-pack/node_modules`. The `Run Golden Set` button compares the
custom WebGPU runtime against q4 ONNX-generated golden embeddings. `Run ONNX
References` and `Run ONNX Golden Set` also load the matching q4 ONNX file in
ORT Web and compare the browser runtime output against that reference. For iOS,
serve the same prototype folder from an HTTPS origin, because WebGPU requires a
secure context on non-localhost origins. The harness accepts these query
parameters: `model`, `runtime`, `artifact`, `vectors`, `corpus`, `query`,
`expected`, `ort`, `ortWasm`, `onnx`, and `onnxQ4`.

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
