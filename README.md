# jbotci 

Lojban parser, reference analysis, dictionary with semantic search, gismu generation, lujvo composition and decomposition, and language server.

## Installing

Download the archive for your platform and `SHA256SUMS` from the
[GitHub Releases page](https://github.com/int19h/jbotci/releases). Each archive
expands to a versioned directory containing `jbotci` (`jbotci.exe` on Windows),
this README, the license, and the third-party notices.

Verify the archive before extracting it. On Linux, for example:

```sh
version=0.1.0 # replace with the release version you downloaded
archive="jbotci-${version}-x86_64-unknown-linux-musl.tar.gz"
grep -F "  ${archive}" SHA256SUMS | sha256sum --check -
tar -xzf "${archive}"
```

On macOS, use `shasum -a 256 --check` in place of `sha256sum --check`.
On Windows, compare `(Get-FileHash <archive> -Algorithm SHA256).Hash` with the
matching line in `SHA256SUMS`, then extract the `.zip` with `Expand-Archive` or
File Explorer.

To build from source instead, install the Rust toolchain and the native build
prerequisites: CMake, a C and C++ compiler/linker toolchain, libclang
development libraries, `pkg-config`, Python 3, and `zstd`. Package names vary
by platform. Then clone the repository with its submodules and build only the
CLI package:

```sh
git clone --recurse-submodules https://github.com/int19h/jbotci.git
cd jbotci
cargo build --release --locked -p jbotci
```

The executable is written to `target/release/jbotci` (or
`target/release/jbotci.exe` on Windows).

## Screenshots

<img width="2834" height="1966" alt="image" src="https://github.com/user-attachments/assets/e2ed8b5e-a0e9-47f3-92c4-31753572f651" />
<img width="2058" height="1508" alt="image" src="https://github.com/user-attachments/assets/c4dbec81-bb1c-4668-85e4-4fddc45ce90e" />
<img width="2330" height="968" alt="image" src="https://github.com/user-attachments/assets/aa6920aa-d8f5-4a72-90a3-9b33554394d7" />

## License

jbotci is licensed under the [MIT License](LICENSE.md).

It is distributed together with third-party fonts, reference data (the CLL
grammar and the jbovlaste/Lensisku dictionary), and Rust crate dependencies that
carry their own licenses. Those notices are collected in
[THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md).

## Local Commands

```sh
cargo xtask check
cargo xtask test
cargo xtask clippy
cargo xtask fixture-check
cargo xtask fixture-list --profile cargo
dx serve --web -p jbotci-app --inject-loading-scripts false --port 8080
cargo xtask build-web-release
cargo xtask dist-server --out-dir .jbotci-build/jbotci-web --base-path /
cargo xtask serve-web-release --port 8080
cargo xtask publish-web-embeddings-r2 --backend fixture --embedding-dtype q4
cargo xtask build-f2llm-webgpu-model
cargo xtask build-f2llm-webgpu-vectors
cargo xtask publish-f2llm-webgpu-r2 --skip-build
cargo xtask render-docker-build
cargo xtask render-docker-run --engine podman
```

Use the web release wrappers instead of raw `dx` release commands while Dioxus
0.7.x needs `--debug-symbols=false` to avoid the wasm-opt DWARF abort.

`dist-server` produces the Dioxus server bundle shape used for deployment:
`<out>/server` plus `<out>/public`. The Render Docker path builds that bundle
inside `deploy/render/Dockerfile` and runs the server with `IP`, `PORT`,
`DIOXUS_ASSET_ROOT`, and `DIOXUS_PUBLIC_PATH`.
`serve-web-release` builds the same release bundle with remote browser
embeddings and runs the bundled server locally.
`cargo xtask render-docker-build` passes the current Git commit into the Docker
build automatically. Direct Docker builds must provide either
`--build-arg RENDER_GIT_COMMIT=$(git rev-parse HEAD)` or
`--build-arg JBOTCI_GIT_COMMIT=$(git rev-parse HEAD)` so the web top bar can
link to the exact deployed commit.

The Render Dockerfile uses BuildKit cache mounts for Cargo registry/git
downloads, tool installs, and the Dioxus/Cargo `target/` tree used by the final
server bundle build. Direct Docker builds therefore need a builder that supports
`# syntax=docker/dockerfile:1` and `RUN --mount=type=cache`; if those cache
mounts are not persisted by the deployment builder, the Dioxus bundle build will
recompile dependencies.

The GitHub Actions Render image workflow builds the same `dist-server` output
outside Docker, packages only `server` and `public/` with
`deploy/render/Dockerfile.runtime`, and publishes a GHCR image. It is
manual-only. The Render Dockerfile provides the self-contained local and fallback
build path.

Browser embedding packs are deployed separately to Cloudflare R2 with
`cargo xtask publish-web-embeddings-r2`. Browser builds default to
`https://assets.jbotci.app/embeddings/web/v1`; set
`JBOTCI_WEB_EMBEDDINGS_BASE_URL` explicitly only when a deployment serves
embedding packs from a different origin or from `/assets/embeddings/web/v1`.

The F2LLM browser path uses custom WebGPU artifacts instead of Transformers.js.
Build its model artifacts and `f16le` vector packs with the production scripts
in `tools/embedding-pack/f2llm/`, or use `cargo xtask publish-f2llm-webgpu-r2`.
The publisher uploads model artifacts under `https://assets.jbotci.app/models/`,
uploads matching q4-generated `f16le` vector packs under the normal web
embedding R2 prefix, and merges only the F2LLM catalog entries so inactive
EmbeddingGemma entries are preserved.

If their output directory already exists, the embedding builders refuse to
start, and they refuse again if it appears before they finish. Pass `--overwrite`
to replace it. The rule covers `build-f2llm-webgpu-model`,
`build-f2llm-webgpu-vectors`, `build-f2llm-webgpu-assets`,
`build-gguf-embeddings`, `build-web-embeddings`, the three publish commands
when they build, and the two Python scripts. `dist-server` regenerates the
packs inside its own bundle on every run, and writes the models it exports to
`<out_dir>.f2llm-models` next to the bundle.

`build-f2llm-webgpu-vectors` writes one model's pack. To build all four models
into one merged pack without publishing, run
`cargo run -r -p xtask-full -- build-f2llm-webgpu-assets`, then publish the
result with `publish-f2llm-webgpu-r2 --skip-build`.

Incremental builds copy stored vectors for unchanged embedding inputs.
They embed changed and added rows, omit removed rows, and preserve the new corpus order.
Each web document stores one pooled vector, including documents with several token windows.

Pack builds save finished work between runs. A checkpoint records which chunks are complete.
To resume an aborted build, repeat the command with the same inputs and paths.
The builder compares the saved identity, corpus, layout, and file hashes before it uses saved rows.
A changed web identity, corpus, or layout discards the whole checkpoint with a message.
A missing or damaged web chunk causes only that chunk to be embedded again.

The Python web builder saves 256-row chunks in `<out>.partial` by default.
Use `--work-dir PATH` to choose another persistent directory.
Keep this directory separate from `--out` and `--stage`.
Use `--discard-checkpoint` to start fresh, or `--chunk-rows N` to change the chunk size.
The builder removes saved work after it promotes the finished pack.
It prints resumed chunks and rows, reused rows, and newly embedded rows for each corpus.

The web xtask commands use these same checkpoints.
The all-model commands keep each model under `<vector-out-dir>.parts` until the combined output is ready.
They retain completed model chunks while other models finish, then remove the parts after promotion.
To start these commands fresh, remove the matching `.partial` directories under the parts directory.
For the single-model command, remove `<out-dir>.partial`.
A finished output still requires `--overwrite`. Leftover work alone does not require this flag.

The native xtask commands retain setup checkpoints under `--index-dir`.
Setup saves each finished 256-row shard under `<pack-id>.tmp` beside the selected pack.
Resume with the same `--index-dir`, model, and corpus.
Setup compares the GGUF SHA-256, model identity, window size and scheme, pooling, corpus fingerprints, layout, and saved shard hashes.
An incompatible native checkpoint causes a full rebuild.
A build with `--skip-validation` cannot resume unverified native chunks.
A missing or damaged native shard causes only that shard to be embedded again.
Native chunks and checkpoints sync their file bytes before rename and sync their parent directories afterward.
To start fresh, remove the matching `<pack-id>.tmp` directory under `<index-dir>/<model-key>/v1/models/<model-key>/packs`.
Setup removes that work after it promotes the pack.
The xtask stages its export and promotes the local output directory only after every selected model succeeds.

The web builder also reuses identical inputs that move between the two web corpora.

For one web model, pass `--reuse-from /path/to/previous/pack` to
`build-f2llm-webgpu-vectors` or `build-vector-pack.py`.
The directory must contain `manifest.json`. A manifest URL also works.

For all web models, pass `--reuse-catalog /path/to/previous/catalog.json`
or `--reuse-published` to `build-f2llm-webgpu-assets`.
Catalog URLs also work. The builder selects each model and vector space separately.
The single-model builder accepts both catalog flags too.
`publish-f2llm-webgpu-r2` accepts the catalog flags when it builds.

Web reuse requires identical model bytes, tokenizer, vector space, windowing, pooling, dimensions, element type, normalization, and distance.
The model identity hashes the raw ONNX graph and every external data file that it references.
The builder records backend versions and execution providers as provenance, which describes how the build ran.
Changes to this provenance do not block reuse.
Exact row hashes identify the input text, so the input format version does not block reuse.
The builder compares stored vector bytes and checks source files against their SHA-256 hashes.
It refuses incompatible, damaged, or incomplete sources without a fallback to full embedding.

Older packs lack parts of this identity.
To adopt them once, add `--adopt-legacy-pack` to the web or native reuse command.
Every recorded identity field must match, and file hashes and row hashes must pass.
The new manifest records the source and states that compatibility of unrecorded parts is an owner attestation.
Without this flag, the builder refuses legacy sources and names the flag in its error.
The flag does not relax comparisons for packs with a complete identity.

If a model has no entry in a reuse catalog, name it with `--new-model MODEL_KEY`.
Repeat the flag for each new model.
The web builder refuses a named model that already has an entry.
It also refuses an unnamed model that lacks an entry.
The xtask resolves `--reuse-published` from `--remote-catalog-url`, whose default is the published web catalog.

Web full builds retain their existing corpus hashes and pack IDs.
Native pack IDs also bind the GGUF SHA-256 and window and pooling parameters.

For native packs, pass `--reuse-index-dir /path/to/previous-index` to
`build-gguf-embeddings` or `publish-gguf-embeddings-r2` when it builds.
The source must contain `<model-key>/v1/catalog.json` for every selected model.
This matches the layout that `--index-dir` produces.
Use separate source, scratch index, and output directories.

Native reuse requires matching GGUF bytes, model identity, window size and scheme, pooling, dimensions, and normalized f32le dot-product vectors.
The native manifest records the GGUF SHA-256, window and pooling parameters, and build provenance.
Native manifests and checkpoints also require the `vector_space` field.
Its current value is `jbotci-native-f2llm-q4-k-m-windowed-2048-v2`, from `F2LLM_NATIVE_VECTOR_SPACE_KEY`.
This key names the native vector function, including NFC and single-sequence inference.
Search, row reuse, and checkpoint resume require this exact key. Missing keys are stale, even with legacy adoption.
The pack ID includes this key. The corpus input format describes rendered text and stays at `v0`.
The corpus fingerprints therefore stay unchanged when only the vector function changes.
Changes to the llama backend or llama.cpp version do not block reuse for packs that record the GGUF hash.
Legacy adoption also requires the recorded runtime name to match.
Runtime version changes do not block reuse or resume.
The native xtask accepts local indexes. It does not fetch published packs for reuse.
Setup reports reused, embedded, and resumed row counts for each corpus.
Use `jbotci setup --embedding --report-json PATH` to save this report as JSON.
The xtask reads this separate report. Published manifests omit build counts.
A resumed pack therefore has the same files as a full build when the vectors match.
Native search refuses stale model or corpus identities before it resolves stored row indices.
Search compares dictionary and CLL fingerprints and row counts with the current corpora.
It also compares vector dimensions and the full vector function, including the pinned GGUF SHA-256.
The search backend compares the GGUF file with its pinned hash once when it loads the model.
If an explicit source is unusable, setup fails.
If setup reuses no rows, the xtask fails.
`--reuse-index-dir` conflicts with `--skip-validation`, so the model must pass its pinned hash comparison.
Reuse flags conflict with `--skip-build` on both publish commands.

Without a reuse flag, no completed source pack supplies rows, but compatible partial work still resumes.
To start fresh, remove the matching `<pack-id>.tmp` directory under `--index-dir`.
Reuse flags do not grant permission to replace an output. The existing `--overwrite` rule still applies.

With local model artifacts and previous native indexes available, build all packs without publishing:

```sh
cargo run -r -p xtask-full -- build-f2llm-webgpu-assets \
  --f2llm-artifact-root /path/to/model-artifacts \
  --model-out-root /build/jbotci/new-web-models \
  --vector-out-dir /build/jbotci/new-web-vectors \
  --reuse-published --adopt-legacy-pack
cargo run -r -p xtask-full -- build-gguf-embeddings \
  --model-dir /path/to/local-gguf-models \
  --reuse-index-dir /path/to/previous-index --adopt-legacy-pack \
  --index-dir /build/jbotci/new-native-index \
  --out-dir /build/jbotci/new-native-vectors
```

After the first adopted build, use the new catalog for later refreshes.
Those packs record the complete identity, so later refreshes do not need legacy adoption.

The pack builders support parallel work and host-specific backends.
The web builder uses ONNX Runtime on the CPU.
The native GGUF builder uses llama.cpp.

`build-f2llm-webgpu-assets --jobs 4` builds the four models concurrently.
Each job receives an equal share of the cores unless `--threads-per-job` overrides that value.
The largest model starts first because it accounts for about half of the work.

On an Apple-silicon Mac, `build-gguf-embeddings --llama-backend metal` selects the GPU backend.
`--llama-backend native-cpu` compiles llama.cpp for the build host CPU.
Use these backends for local pack builds. Do not distribute binaries compiled with them.

`vendor/cll` tracks the
[int19h/cll](https://github.com/int19h/cll) upstream at the `v1.3.6` release.
It is kept as a submodule because CLL examples and references are part of the
core parser and reference-analysis development loop.
