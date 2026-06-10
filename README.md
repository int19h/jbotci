# jbotci 

## Local Commands

```sh
cargo xtask check
cargo xtask test
cargo xtask clippy
cargo xtask fixture-check
cargo xtask fixture-list --profile cargo
cargo xtask build-web-release
cargo xtask dist-server --out-dir .jbotci-build/jbotci-web --base-path /
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

`vendor/cll` is kept as a submodule because CLL examples and references are
part of the core parser and semantics development loop.
