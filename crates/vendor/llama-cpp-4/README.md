# llama-cpp-4 (vendored)

This is the crates.io release `llama-cpp-4` 0.3.0, the safe API over the
vendored `llama-cpp-sys-4` next to it. The upstream README is kept as
`README.upstream.md`.

jbotci vendors it for two changes, each recorded in `patches/`; apply the same
patches when updating to a newer release:

1. Two builders on `LlamaContextParams` that the release does not expose,
   `with_n_seq_max` and `with_kv_unified`. The native embedding backend uses
   them to embed many short inputs in one decode call, which makes building
   embedding packs several times faster on a GPU.
2. `embeddings_ith`, `embeddings_seq_ith` and `get_embeddings` slice the
   returned vector by `n_embd_out`, the size llama.cpp actually writes,
   instead of `n_embd`, which can be larger and would read past the vector.
