# Phase-B frozen corpus (vendored)

These fixtures are the frozen Phase-B corpus of the tersmu notation program,
vendored from the research repo so the completeness tests are self-contained.

Provenance (see `FREEZE-PHASE-B.md` in the research repo):

- Oracle commit: `cab176bcce9e35ba8a8646249f521fd84c2591a0`
  ("renderer: lean3 preset (design-record adoption)").
- Corpus: 33 `battery-docs/b*.json`/`nd*.json` + 4
  `notation-renderer-v0/samples/*.json` = 37 documents.
- Combined corpus hash (over the sorted `sha256sum` listing of the raw
  `*.json` bytes): `949ab9b86724068e1ff971f57718b772969dd5f4c45898efe5158369d3072d39`.

For each document `<name>`:

- `<name>.lojban` — the Lojban input. For the 33 battery docs this is the
  frozen `.txt`; for the 4 samples (which ship only as `.json`) it is the
  root utterance's `source.text` extracted verbatim from the frozen graph.
- `<name>.frozen.json` — the frozen `lojban-semantics-json-1` graph (the
  Python oracle's input). Re-serialized with a stable 1-space indent; content
  is byte-identical in meaning to the frozen file (`json.load`/`json.dump`).
- `<name>.lean3.txt` — the byte-parity **expected output** of the frozen Python
  `lean3` renderer, produced verbatim by
  `python3 experiments/notation-renderer-v0/render_v5.py <name>.frozen.json
  --profile lean3` at the oracle commit `cab176bcce`. These are the fixtures the
  `tests/lean3_parity.rs` byte-parity test compares this build's `render_lean3`
  output against (graph re-derived from `<name>.lojban`, never from the frozen
  JSON — see below). Regenerate by re-running the oracle over the
  `<name>.frozen.json` files.

The completeness and `lean3` byte-parity tests **re-derive** each graph from
`<name>.lojban` using *this* jbotci build (never reading `<name>.frozen.json`
as the graph under test). The frozen JSON is used only by (a) the completeness
divergence-report test, which compares this build's graph against it and
reports (never fails on) differences, and (b) generating the `<name>.lean3.txt`
oracle fixtures above. Because every corpus graph this build produces is
byte-identical (in meaning) to the frozen graph, the byte-parity test isolates
renderer correctness from any semantic drift.
