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

The experimental Python package has its own environment and verification
workflow. See [`bindings/python/README.md`](bindings/python/README.md) before
working on the PyO3 bindings; ordinary workspace commands intentionally omit
that non-default member.

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
manual-only while the image-backed Render path is being validated. The existing
Render Dockerfile remains the self-contained local and fallback build path.

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

`vendor/cll` tracks the
[int19h/cll](https://github.com/int19h/cll) upstream at the `v1.3.2` release.
It is kept as a submodule because CLL examples and references are part of the
core parser and reference-analysis development loop.
