# Smusni version-0 implementation status

This renderer is an experimental implementation candidate, not a minted or
exhaustive version-0 implementation. Its candidate design inputs are the smusni
design repository at commit `86cbd9d1288d2c0232ba86cb214000a431b5db7c`, this
directory's `spec.md` (SHA-256
`c2c0616b0b0d8991251f5145be4985a8191a68704cff3ae10f6f85caa34dbdc1`),
and `samples.md` (SHA-256
`ee4cfe6c00009f2ca0387efd0dfa6551b4e6db61e6ff9bebf891f0c0346aa50b`).
The samples are design specimens, not output expectations.

## Runnable support matrix

| Family | Current behavior | Boundary |
|---|---|---|
| Document packaging | One typed-grammar-parseable `(Smusni 0 ...)` datum with one trailing newline. The optional `Words` section uses only `(Word root definition)` cards. | Spec sections 2.2 and 2.4 |
| Diagnostics | Collected once as structured `SmusniDiagnostic` values and kept out of the datum. Every failed projection edge gets its own `Fallback` record with a stable reason code, a stable message fixed by its typed cause, and the affected owner/use identities; `SmusniRenderStats::fallback_reasons` is a summary of those records, never a substitute. CLI display is deliberately deferred until these records can use the existing source-aware diagnostic renderer; provisional formatter strings are not printed. | Spec sections 2.4 and 16 |
| Predication | Named predicate terms, ordinary fills, `:n`, `:Eventuality`, numbered-only `DropPlace`, default closure omission, and explicit `Assert` are compact for their exact typed shapes. | Spec sections 4 and 5 |
| Logical composition | Registered ordinary truth-functional connectives render with their logical operators. Unsupported connector metadata falls back. | Spec section 6 |
| Fixed descriptions | One force-local, exact entity `lo`, `le`, or `la` reference becomes `Bind` plus `Refer`; its predicate property retains ordinary filled conventional arguments, `le` retains the represented speaker/audience `skicu` property without asserting classification, and a veridical restrictive `poi` is conjoined inside the one property. Nested reference effects, richer descriptors, incidental/nonveridical relatives, and shared placement fall back. | Spec sections 6.3, 8.3, and 8.4 |
| Modals and tense | Exact represented modal predicates are joined to the host by `Joi` and share the graph event under one lambda-shaped existential. The verified `before`/`at`/`after` event relations lower to `purci`/`cabna`/`balvi`. Other tag maps and event facets fall back. | Spec sections 10.1 through 10.4 |
| Discourse | Same-topic items use `Do`; exact paragraph provenance uses `NewTopic` or `Resume`, with explicit `Perform`/`PerformUtterance` crossings at the transition operand. | Spec sections 7.1 and 7.2 |
| Questions | Exact direct polar questions render `Ask` plus `Polar`; exact entity argument questions and atomic `ti mo`-style relation questions render `OpenQ`, with the latter retaining a typed open predicate row and explicit `Close`. Tanru-like `ti mo zdani`, embedded, multi-slot, and richer questions fall back. | Spec section 12 |
| Abstractions | Exact unary entity properties render as lambdas and exact proposition crossings use `Reify` when their complete model shape is eligible. Event-valued and richer abstraction families fall back. | Spec section 11 |
| Utterance entries | Retained entries use the fresh `UtteranceToken` binder and registered `SpeakerOf`, `AudienceOf`, `LocutionOf`, deictic, and `Realizes` facts. Unsupported force/asides use fallback. | Spec section 7.2 |
| Raw fallback | Unproved compact projection selects one typed `TypedGraph` document with graph-owned `%id` sharing and a registered reason. The current vertical slice does not yet emit typed local `Fallback`. | Spec section 20 |

## What the current acceptance gate does and does not prove

Structural tests parse every rendered document with `parse_v0_document`. That
gate validates the closed serialization grammar of specification section 2.2
and the typed annotations those productions carry: document packaging, the
lexical token grammars, binder and declaration shapes, and the declared type
expressions that appear in the output.

It is not a typechecker. There is no whole-expression static check proving that
every compact output is well-typed — that each application's operand type
matches its operator's domain, that each place fill respects the predicate
term's row, and that each closure boundary consumes the content type it
declares. That is a known post-slice gap. It is not authority to add a
superficial validator: a check that accepts everything the renderer currently
emits would prove nothing, so the real kernel-directed typechecker is the only
acceptable way to close it.

## Known limitations and next corrections

Each item names the observed construct, current honest boundary, and intended
specification destination.

- Dynamic host planning: `ro da poi gerku cu bajra` currently reaches
  `TypedGraph` at definition placement, and the tanru-like `ti mo zdani`
  question still has unlicensed relation/property crossings and event hosting.
  Implement graph-owned accessibility, force
  handlers, dependency lifting, and legal shared capture before enabling these
  compact paths (sections 6.2–6.5 and 12).
- Simultaneous termsets: both grammar-licensed prenex spellings retain their one
  semantic `QuantifierBundle`, but projection currently uses `TypedGraph`.
  Implement the approved equal-scope reduction rather than reviving
  `Quantify` or choosing a nested order (section 9 and section 12 of the
  samples).
- Quantification: the compact lowering recognizes the simple `Every`/`Some`
  function shapes after scope planning, but selection sources, plural
  quantifiers, cardinality, witness export, and effect routing are not yet
  complete (sections 9 and 10).
- Signs and quotation: structured and opaque quotation currently use
  `TypedGraph`; no transcript is synthesized and no retired `Quotation` record
  prints. Add `StructuredQuote`, `OpaqueQuote`, raw sign constructors, and sign
  token facts only from graph-owned identities (sections 7.3 and 13.3).
- Event abstractions and facets: unsupported actuality, aspect, recurrence,
  space, path, and abstraction details select `TypedGraph`. Add only registered
  lowercase/prelude reductions with their actual shared event (sections
  10.3–10.4 and 11.2–11.3).
- Respectively, collections, and richer math: these retain the graph through
  `TypedGraph` except for exact integer literals and binary kernel arithmetic.
  Generic composition records and non-exact quantities do not borrow callable
  names from the registry. Implement `ZipWith`, typed collection kernels,
  generalized-quantifier reductions, and registered math rows without generic
  `Math`, `Quantity`, or `Respectively` records (section 13).
- Raw tuple payload coordinates currently use deterministic `item1`, `item2`,
  and so on because serde does not expose source field names. Replace them with
  projection-declared stable names before claiming the raw schema final
  (section 20).
- Raw numeric scalar type names currently expose the serializer's normalized
  `i128`, `u128`, and `f64` carriers. Bind them to declared model scalar types
  before minting a stable raw schema (section 20).
- Local fallback: the current conservative implementation promotes every
  unproved local boundary to `TypedGraph`. Introduce local `(Fallback T reason
  raw)` only when the expected type and minimum graph-owned raw owner are both
  proved (section 20).

## Registry status

The checked-in semantic-surface scanner, 882 authored dispositions, generated
runtime registry, and bundle audit are retained from the interrupted CP2 work
because they are useful executable defect detectors. They are not described as
immutable, exhaustive, or final: the current runnable slice still exercises
whole-graph fallback for many rows classified for later direct lowering, and
the runtime application/signature registry requires further semantic review.
No renderer behavior is licensed merely by the existence of an inventory row.

## Reproducible observations

Smoke outputs and their separate stderr files are written under
`/build/jbotci/scratch/issue-741/runnable-smoke/`. Representative inputs cover
simple assertion, restrictive description, modal event sharing, paragraph
transition, direct polar question, open question, quantification, and
structured quotation. These files are observations only and are safe to wipe;
typed/structural tests, not rendered bytes, are the acceptance oracle.

`cargo run -r -p jbotci-semantics --example smusni_corpus_report -- <slice>`
reproduces the aggregate corpus measurements below. They are **observations of
what the current conservative slice does**, not expectations: no test asserts
them, and they will move whenever a compact recognizer is added.

| Slice | Inputs | Renders | Render panics | Notes |
|---|---:|---:|---:|---|
| `cll` | 1,247 | 1,245 | 0 | 2 pre-render morphology failures; 146 compact documents, 1,099 typed-graph documents |
| `alice-lines` | 2,436 | 1,084 | 0 | the remaining inputs fail earlier parsing or building, mostly syntax |
| `alice-whole` | 1 | 1 | 0 | one `TypedGraph` over 49,172 objects |

The whole-Alice run is the memory reference point: 7,523,544 KiB RSS after the
graph build and a 9,523,648 KiB peak after rendering, so it needs a host with
more than 10 GiB free.

The five CLL inputs that previously panicked while manufacturing a variable
atom — `c11e12d2`, `c11e3d1`, `c11e3d3`, `c11e3d4`, and `c11e9d1` — now render.
They are retained as structural regressions beside one witness per eventuality
subtype, so the sweep's zero-panic result has a cheap test-suite counterpart.
