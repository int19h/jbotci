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

The completeness tests **re-derive** each graph from `<name>.lojban` using
*this* jbotci build (never reading `<name>.frozen.json` as the graph under
test). The frozen JSON is used only by the divergence-report test, which
compares this build's graph against it and reports (never fails on)
differences.
