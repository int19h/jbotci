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
- `crates/jbotci-fixtures`: unified TOML test fixture loader.
- `crates/jbotci-source`: shared source-span and provenance support.
- `xtask`: local workspace automation.

## Local Commands

```sh
cargo xtask check
cargo xtask test
cargo xtask fixture-check
```

`vendor/cll` is kept as a submodule because CLL examples and references are
part of the core parser and semantics development loop.
