# Phase-B frozen structural corpus

This directory retains the Phase-B Lojban sources and frozen
`lojban-semantics-json-1` graphs as structural inputs for renderer completeness
and regression testing. The original 37 graphs came from the notation research
repository at corrected oracle commit
`28c7d5f72ff1c1970f2c5568e3e73198207c4697`; later jbotci issues added focused
graph witnesses for relation questions, place questions, question families,
adjuncts, and structural deixis.

The corpus currently contains 48 documents:

- 33 `battery-docs` documents and four research samples;
- `ti-mo` and `mi-klama-fia`, covering relation- and place-question shapes;
- five question-family witnesses;
- four adjunct witnesses.

The combined hash of the original 37 raw research graphs remains
`949ab9b86724068e1ff971f57718b772969dd5f4c45898efe5158369d3072d39`.

For each document `<name>`:

- `<name>.lojban` is the source reprocessed by the current production
  morphology, syntax, and semantic builder;
- `<name>.frozen.json` is the retained structural comparison graph.

Issue #741 retired the old flat-smusni byte expectations, their separate
provenance profile, and the parity test. The experimental typed S-expression
renderer intentionally has no golden output corpus. Its tests rebuild graphs
from the `.lojban` files and assert parseability, totality, determinism, lexical
binding, graph-reference integrity, diagnostic preservation, modal place-map
fidelity, field-disposition completeness, and the single-document/newline
contract. Frozen JSON remains an evidence and completeness oracle, never the
semantic authority for compact recognition.

`hostile-quote.lojban` and `relation-question-indirect.lojban` remain focused
sources outside `CORPUS_DOCS`. The former exercises quotation metacharacters;
the latter is the live-traffic `lo se jalge cu mo kau` relation-question shape.
