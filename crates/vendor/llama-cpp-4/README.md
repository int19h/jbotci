# llama-cpp-4 (vendored)

This is the crates.io release `llama-cpp-4` 0.3.0, the safe API over the
vendored `llama-cpp-sys-4` next to it. The upstream README is kept as
`README.upstream.md`.

jbotci vendors it to add two builders to `LlamaContextParams` that the release
does not expose: `with_n_seq_max` and `with_kv_unified`. The native embedding
backend uses them to embed many short inputs in one decode call, which makes
building embedding packs several times faster on a GPU. The change is recorded
in `patches/`; apply the same patch when updating to a newer release.
