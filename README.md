# jbotci v1

Rust port of jbotci.

The immediate goal is a clean workspace that can grow toward full v0 parity:
command-line tools first, then the Dioxus web/server and GUI surfaces.

## Workspace

- `apps/jbotci`: CLI binary.
- `apps/jbotci-server`: server binary for the Dioxus web app and HTTP-facing integrations.
- `crates/jbotci-morphology`: morphology object model and parser.
- `crates/jbotci-syntax`: syntax object model and parser.
- `crates/jbotci-semantics`: semantic object model and builder.
- `crates/jbotci-output`: output format models and render entry points.
- `crates/jbotci-dictionary`: dictionary data model.
- `crates/jbotci-cll`: CLL data/reference model.
- `crates/jbotci-search`: semantic search abstractions.
- `crates/jbotci-jvozba`: lujvo composition and decomposition.
- `crates/jbotci-source`: shared source-span and provenance support.
- `tests/fixtures`: cross-cutting integration fixture corpus.
- `tests/support`: test-only fixture loader and runner support.
- `xtask`: local workspace automation.

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

The Render Dockerfile uses cargo-chef dependency layers plus BuildKit cache
mounts for Cargo registry/git downloads and tool installs. Direct Docker builds
therefore need a builder that supports `# syntax=docker/dockerfile:1` and
`RUN --mount=type=cache`; if cache mounts are not persisted by the deployment
builder, the cargo-chef layers still provide dependency reuse through normal
Docker layer caching.

Browser embedding packs are deployed separately to Cloudflare R2 with
`cargo xtask publish-web-embeddings-r2`. Production builds set
`JBOTCI_WEB_EMBEDDINGS_BASE_URL` to
`https://assets.jbotci.app/embeddings/web/v1`; local static builds default to
`/assets/embeddings/web/v1`.

The parser facets are scaffolded but intentionally return `NotImplemented` at
this checkpoint. Use `cargo xtask fixture-test --profile all --facet morphology
--facet syntax` when you want to exercise the intentionally red runner path.

`vendor/cll` is kept as a submodule because CLL examples and references are
part of the core parser and semantics development loop.
