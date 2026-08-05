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
| Document packaging | One typed-grammar-parseable `(Smusni 0 ...)` datum with one trailing newline. The optional `Words` section carries one `(Word root definition)` card per content word that has a dictionary definition. A card whose word is not a bare lexical root — a defined zei-lujvo's multi-word surface such as `abu zei sance` — keeps that exact surface through the grammar's escaped spelling `|abu zei sance|`. Words with no dictionary definition have no card, because the grammar's card production has no place for a missing definition; the XML rendering states them as `KNOWN="false"` instead. | Spec sections 2.2 and 2.4 |
| Diagnostics | Collected once as structured `SmusniDiagnostic` values and kept out of the datum. Every failed projection edge gets its own `Fallback` record with a stable reason code and a stable message fixed by its typed cause. Owner and use-site identities travel as evidence where the failure site has them; they are optional and a consumer must not require them. An edge's identity is typed rather than textual: `(owner, declining boundary)` in the elaborator and `(kind, binder, use site)` in the scope planner. Re-entering one edge — for instance when a declining wrapper re-renders a child — records nothing further, while a second distinct boundary on the same owner is a second record. `SmusniRenderStats::failed_projection_edges` and `fallback_reasons` both count failed edges; `SmusniRenderStats::object_fallbacks` counts graph objects and is a different measurement. Ordering is deterministic from the typed channels; the internal sort key is not a public tuple contract. **Deliberate deviation from the specification:** section 16.1 describes the CLI profile as writing these records to stderr in the shared `gentufa` format, and this implementation does not. CLI display is deferred until the records can use the existing source-aware diagnostic renderer; the provisional formatter strings are not printed anywhere. | Spec sections 2.4 and 16.1 |
| Variable spelling | Generated variables are composed from typed identity components through closed token tables, never by rewriting an identity's display text. Every token begins lowercase, so no generated name enters the PascalCase namespace that section 2.1 reserves for primitives, prelude names, types, and literals. Structural object kinds carry a `…Node` stem and referent sorts do not, which keeps the two namespaces injective without using letter case as the separator. The short `$x`/`$e`/`$p`/`$q`/`$u`/`$s`/`$v` alpha-renaming of section 15 item 3 is not yet implemented. | Spec sections 2.1 and 15 (item 3) |
| Predication | Named predicate terms, ordinary fills, `:n`, `:Eventuality`, numbered-only `DropPlace`, default closure omission, and explicit `Assert` are compact for their exact typed shapes. | Spec sections 4 and 5 |
| Relation formers | The canonical flat binary tanru graph projects to the registered former `(Tanru modifier head)` applied to the tertau's own places. The recognizer requires an implicit-juxtaposition `And` connective at predicate locus with exactly two children, a named tertau predication that is otherwise plain, a `Composition` link predication with exactly two plain arguments and no side fields, a fixed constant unary property abstraction over an exact `ce'u` entity parameter as the seltau, a `modifier-head` constructed relation label, and every supporting object private to this projection. Any other relation-former shape falls back, and the tanru-*like* relation question below is a separate unsupported family. | Spec section 4.6 |
| Logical composition | Registered ordinary truth-functional connectives render with `¬`, `∧`, `∨`, `→`, `↔`, and `⊕`. Unsupported connector metadata falls back. | Spec section 9.1 |
| Quantification | Restricted quantification over a planned binder renders `Every` or `Some`; unrestricted quantification renders bare `∀` or `∃`. Selection sources, plural quantifiers, inner/outer cardinality, witness export, and effect routing are not implemented and fall back. | Spec sections 9.2 through 9.5 |
| Shared values and recursion | A value used more than once is bound once and shared. Nonrecursive groups nest canonical single-binding `Let` forms; a recursive strongly connected component uses `LetRec` only when every initializer in the group is a top-level lambda. Any other recursive value falls back. | Spec sections 2.2 and 15 (item 2) |
| Contextual and deictic values | `Speaker`, `Audience`, `Now`, `Here`, and the proximity deictics `This`, `That`, and `Yonder` render as the declared atoms. A fixed context renders as the bare `Context` primitive and an underspecified one as `(Context deps…)` over its direct dependencies. A deictic with a non-current ground falls back. | Spec sections 3.5 and 6.1 |
| Fixed descriptions | One force-local, exact entity `lo`, `le`, or `la` reference becomes `Bind` plus `Refer`; its predicate property retains ordinary filled conventional arguments, `le` retains the represented speaker/audience `skicu` property without asserting classification, and a veridical restrictive `poi` is conjoined inside the one property. Nested reference effects, richer descriptors, incidental/nonveridical relatives, and shared placement fall back. | Spec sections 6.3, 8.3, and 8.4 |
| Modals and tense | Exact represented modal predicates are joined to the host by `Joi` and share the graph event under one lambda-shaped existential. The verified `before`/`at`/`after` event relations lower to `purci`/`cabna`/`balvi`. Other tag maps and event facets fall back. | Spec sections 10.1 through 10.4 |
| Discourse | Same-topic items use `Do`; exact paragraph provenance uses `NewTopic` or `Resume`, with explicit `Perform`/`PerformUtterance` crossings at the transition operand. | Spec sections 7.1 and 7.2 |
| Force | `Assert` and `Mention` are compact for their exact typed shapes. `Ask` is emitted only by the question projection below, so ask force over content that is not a typed question fails closed rather than applying `Ask` at the wrong type. Quote, parenthetical, subordinated, command, and vocative force fall back. | Spec sections 1.3 and 7.1 |
| Questions | Exact direct polar questions render `Ask` plus `Polar`; exact entity argument questions and atomic `ti mo`-style relation questions render `OpenQ`, with the latter retaining a typed open predicate row and explicit `Close`. Tanru-like `ti mo zdani`, embedded, multi-slot, and richer questions fall back. No answer family is projected: section 12.2's `Answer`, `PolarAnswer`, `TupleAnswer`, `ContextualAnswer`, and `UnresolvedAnswer` have no compact route. | Spec section 12 |
| Abstractions | Exact entity properties render as lambdas over the abstraction's own entity parameters, which may be more than one, and exact proposition crossings use `Reify` when their complete model shape is eligible. Event-valued and richer abstraction families fall back. | Spec section 11 |
| Utterance entries | Retained entries use the fresh `UtteranceToken` binder and registered `SpeakerOf`, `AudienceOf`, `LocutionOf`, deictic, and `Realizes` facts. Unsupported force/asides use fallback. | Spec section 7.2 |
| Everything else | **Catch-all:** every specification family not named as compact above reaches the whole-document typed fallback. In particular that covers indicators and displayed content (7.4), sign and quotation constructors (7.3, 13.3), set/group descriptions and referential connections (8.5), simultaneous termsets (9.5), witness export (9.4), respectively-distribution, quantities, and math beyond exact integer literals and binary kernel arithmetic (13.1, 13.2), and answers (12.2). No compact head is emitted for any of them. | Whole specification |
| Raw fallback | Unproved compact projection selects one typed `TypedGraph` document with graph-owned `%id` sharing and a registered reason. The current vertical slice does not yet emit typed local `Fallback`. | Spec sections 16.2 and 16.3 |

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
  compact paths (sections 6.2–6.4 and 12.1).
- Simultaneous termsets: both grammar-licensed prenex spellings retain their one
  semantic `QuantifierBundle`, but projection currently uses `TypedGraph`.
  Implement the approved equal-scope reduction rather than choosing a nested
  order or reviving one of the forbidden record shapes of section 14.3
  (section 9.5, and section 12 of the samples).
- Quantification: the compact lowering recognizes the simple `Every`/`Some`
  function shapes after scope planning and bare `∀`/`∃` for unrestricted
  quantification, but selection sources and plural quantifiers (section 9.2),
  inner and outer cardinality (section 9.3), witness export (section 9.4), and
  effect routing (section 6.4) are not yet complete.
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
  `Math`, `Quantity`, or `Respectively` records; the generalized-quantifier
  reductions in that list belong to sections 9.2 and 9.5 rather than to
  section 13 (sections 13.1 and 13.2).
- Raw tuple payload coordinates currently use deterministic `item1`, `item2`,
  and so on because serde does not expose source field names. Replace them with
  projection-declared stable names before claiming the raw schema final
  (section 16.2).
- Raw numeric scalar type names currently expose the serializer's normalized
  `i128`, `u128`, and `f64` carriers. Bind them to declared model scalar types
  before minting a stable raw schema (section 16.2).
- Local fallback: the current conservative implementation promotes every
  unproved local boundary to `TypedGraph`. Introduce local `(Fallback T reason
  raw)` only when the expected type and minimum graph-owned raw owner are both
  proved (sections 16.2 and 14.4).

## Registry status

The checked-in semantic-surface scanner, 882 authored dispositions, generated
runtime registry, and bundle audit are retained from the interrupted CP2 work
because they are useful executable defect detectors. They are not described as
immutable, exhaustive, or final: the current runnable slice still exercises
whole-graph fallback for many rows classified for later direct lowering, and
the runtime application/signature registry requires further semantic review.
No renderer behavior is licensed merely by the existence of an inventory row.

## Regenerating and checking the bundle

Every `cargo build -p jbotci-semantics` verifies the checked-in bundle in check
mode through `build.rs`. The reviewer-facing entry point is the same generator
run directly:

```sh
cargo run -r -p jbotci-semantics --example smusni_v0_bundle -- --check
cargo run -r -p jbotci-semantics --example smusni_v0_bundle -- --generate
```

`JBOTCI_SMUSNI_V0_BUNDLE_MODE=generate cargo build -p jbotci-semantics` is the
equivalent through the build script. Generation is deterministic: running it on
an already-current tree changes no byte. The example writes only its compiled
policy table, into `$CARGO_TARGET_DIR/smusni-v0-scratch/` or the workspace
`target/` directory when that variable is unset.

The retained generator inputs under `data/smusni-v0/sources/generator-inputs/`
are all stored with a `.opaque` suffix. That is load-bearing rather than
cosmetic: `cargo package` treats any subdirectory containing a `Cargo.toml` as a
nested package and prunes the whole subtree, so a mirror filed under its natural
manifest name silently removes the entire retained closure from every source
distribution while leaving the working tree green.

## Reproducible observations

Smoke outputs and their separate stderr files are written under
`/build/jbotci/scratch/issue-741/runnable-smoke/` on the project's dev box; that
location is an observation area, not a path any test depends on. Representative
inputs cover
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
| `phaseb` | 48 | 48 | 0 | the frozen structural corpus; 16 compact documents, 32 typed-graph documents |
| `cll` | 1,247 | 1,245 | 0 | 2 pre-render morphology failures; 146 compact documents, 1,099 typed-graph documents |
| `focused` | 16 | 16 | 0 | 6 compact documents, 10 typed-graph documents |
| `alice-lines` | 2,436 | 1,084 | 0 | the remaining inputs fail earlier parsing or building, mostly syntax |
| `alice-whole` | 1 | 1 | 0 | one `TypedGraph` over 49,172 objects |

The whole-Alice run is the memory reference point: 7,523,428 KiB RSS after the
graph build and a 9,523,660 KiB peak after rendering, so it needs a host with
more than 10 GiB free. Those two figures vary by a few MiB between runs.

The same sweep reports the object statistic and the per-edge diagnostic channel
separately. `Objects not projected` is `SmusniRenderStats::object_fallbacks`;
`failed edges` is `SmusniRenderStats::failed_projection_edges`, which is also the
number of `Fallback` records and the sum of `fallback_reasons`. `Failing owners`
counts the distinct owners those records name, and `multi-edge owners` counts the
owners named by more than one record. Like the table above, these are
observations of the current slice rather than expectations.

| Slice | Objects not projected | Failed edges | Failing owners | Multi-edge owners |
|---|---:|---:|---:|---:|
| `phaseb` | 280 | 94 | 91 | 3 |
| `cll` | 12,840 | 3,063 | 2,900 | 113 |
| `focused` | 33 | 34 | 33 | 1 |
| `alice-lines` | 15,764 | 2,824 | 2,599 | 149 |
| `alice-whole` | 49,172 | 376 | 122 | 55 |

The two measurements differ because they count different things, which is why
they are reported side by side rather than derived from one another. The
per-edge identity in the code is justified by the typed law tests over the
channel representations, not by these numbers.

The five CLL inputs that previously panicked while manufacturing a variable
atom — `c11e12d2`, `c11e3d1`, `c11e3d3`, `c11e3d4`, and `c11e9d1` — now render.
They are retained as structural regressions beside one witness per eventuality
subtype, so the sweep's zero-panic result has a cheap test-suite counterpart.
