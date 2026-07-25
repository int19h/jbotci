# Phase-B frozen corpus (vendored)

These fixtures are the frozen Phase-B corpus of the tersmu notation program,
vendored from the research repo so the completeness tests are self-contained.

**Naming.** The product ships this notation as the `smusni` profile (from
*smuni simxa*). The research-repo Python oracle's internal profile name is
`--profile lean3` — a historical experiment label kept verbatim in the oracle
invocations and the research repo's `DESIGN-RECORD.md` / `FREEZE-PHASE-B.md`.
The two name the same rendering: research-repo `lean3` ≡ product `smusni`. The
vendored fixture files and the parity test use the product name (`*.smusni.txt`,
`*.smusni-prov.txt`, `tests/smusni_parity.rs`); only citations of the oracle's
own command line retain `--profile lean3`.

Provenance (see `FREEZE-PHASE-B.md` in the research repo):

- Oracle commit: `28c7d5f72ff1c1970f2c5568e3e73198207c4697` (corrected oracle;
  `FREEZE-PHASE-B.md` sections (e)/(f)/(g), Amendments 1–3). Supersedes `810c3bf`,
  `57a28c9`, and the original `cab176bcce9e35ba8a8646249f521fd84c2591a0`.
  Amendment 1 (round-1): three corpus-present fields (`Predication.tanruLink`,
  `Referent.assignedNames`, `Eventuality.intervalModifiers`) render
  (content-complete). Amendment 2 (round-2): nested value-struct sources
  (`AssignedName`, `ArgumentValue`, descriptor-attached `RelativeClause`) render
  under `--provenance` only. Amendment 3 (round-3): argument-attached
  `RelativeClause`s render (builder-selected branch (b) — the builder attaches
  them only to `ArgumentValue.relative_clauses` with no duplicating invariant),
  changing `b23`/`b24` in BOTH `*.smusni.txt` and `*.smusni-prov.txt`. The `*.json`
  inputs and the corpus hash below are unchanged throughout.
- Corpus: 33 `battery-docs/b*.json`/`nd*.json` + 4
  `notation-renderer-v0/samples/*.json` = 37 documents.
- Combined corpus hash (over the sorted `sha256sum` listing of the raw
  `*.json` bytes): `949ab9b86724068e1ff971f57718b772969dd5f4c45898efe5158369d3072d39`.
- Aggregate fixture hashes (pinned in `tests/smusni_parity.rs`
  `frozen_fixture_aggregate_hashes_are_pinned`, so the two fixture sets cannot
  silently drift together): `smusni.txt` →
  `4d3e18385ed48605019f0543c95598c45f7fe2a05cdf8bbac4e19c0a31569c81`;
  `smusni-prov.txt` →
  `d9c1fe502528d67246d3ba7cce9f25e4bd17726b24e564a1383217986bf968a7`.

For each document `<name>`:

- `<name>.lojban` — the Lojban input. For the 33 battery docs this is the
  frozen `.txt`; for the 4 samples (which ship only as `.json`) it is the
  root utterance's `source.text` extracted verbatim from the frozen graph.
- `<name>.frozen.json` — the frozen `lojban-semantics-json-1` graph (the
  Python oracle's input). Re-serialized with a stable 1-space indent; content
  is byte-identical in meaning to the frozen file (`json.load`/`json.dump`).
- `<name>.smusni.txt` — the byte-parity **expected output** of the Python
  oracle's `smusni` renderer (the research repo names this profile `lean3`),
  produced verbatim by
  `python3 experiments/notation-renderer-v0/render_v5.py <name>.frozen.json
  --profile lean3` at the corrected oracle commit `28c7d5f`. These are the
  fixtures the `tests/smusni_parity.rs` byte-parity test compares this build's
  `render_smusni` output against (graph re-derived from `<name>.lojban`, never
  from the frozen JSON — see below).
- `<name>.smusni-prov.txt` — the same, with `--provenance` (the source-span
  opt-in), compared against `render_smusni` with `SmusniConfig { provenance:
  true }`.

Regenerate both fixture sets by re-running the oracle over the
`<name>.frozen.json` files (`--profile lean3` and `--profile lean3
--provenance`); then update the two aggregate hashes pinned in
`tests/smusni_parity.rs` and the `FREEZE-PHASE-B.md` amendment in lockstep.

`hostile-quote.*` is a **separate** regression fixture (not one of the 37, and
excluded from the corpus/aggregate hashes): a `zoi` quotation whose text carries
notation metacharacters (`{ ( ; } )`), guarding the dense-flatten hardening
end-to-end.

The completeness and `smusni` byte-parity tests **re-derive** each graph from
`<name>.lojban` using *this* jbotci build (never reading `<name>.frozen.json`
as the graph under test). The frozen JSON is used only by (a) the completeness
divergence-report test, which compares this build's graph against it and
reports (never fails on) differences, and (b) generating the `<name>.smusni.txt`
oracle fixtures above. Because every corpus graph this build produces is
byte-identical (in meaning) to the frozen graph, the byte-parity test isolates
renderer correctness from any semantic drift.
