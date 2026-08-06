# Smusni version-0 implementation status

This renderer is an experimental implementation candidate, not a minted or
exhaustive version-0 implementation. Its design inputs are this directory's
`spec.md` and `samples.md`, which are dual-homed with the smusni design
repository. The samples are design specimens, not output expectations.

**Issue #753 is closed.** An unproved projection is a product error rather
than a conforming raw document, in the implementation as well as in the
specification. The renderer entrypoints return the section-16.1 result shape:
a success carries one complete `(Smusni 0 ...)` document with its nonfatal
diagnostics and statistics, and a failure carries a nonempty ordered list of
registered projection errors with no document, no `Words` section, and no
serialized graph. Every reason row carries a reviewed section-16.2
`failure-class` and an explicit failure site; the two `WholeGraph` ids the
specification requires exist and are emitted where the renderer detects their
conditions; and the ledger records the section-14.2 `Failure` marker in place
of the retired `TypedFallback` value.

The specification remains ahead of this implementation in coverage, which is
what the support matrix and the limitations list below describe. It names
`tu'a`, `co'e`, and `do'e` as tracked spec gaps (section 14.4), and no compact
route may be attempted for them until the gap is closed. `tu'a` has its own
`TrackedSpecGap` reason, `smusni.projection.abstraction-about-unspecified`, so
its records are not confused with the renderer backlog it used to share a
reason with; `co'e` and `do'e` are still behaviorally part of the catch-all
below.

## The failure surface

A failed projection produces no document at all. Each failed edge is one
record carrying a registered `smusni.projection.` reason id, the stable message
its typed cause fixes, severity `error`, the section-16.2 class taken from that
reason's registry row, typed owner and use-site evidence where the graph
supplies them, and a source span resolved when the record is created by the
section-16.2 attribution order: the owner's own span, else the use site's span,
else the nearest source-bearing semantic ancestor, else the whole input with
the identities carried as notes. The record says which of those four rules
chose its span, so a host that owns the original text can present the
whole-input case as the whole input rather than as an approximation of it.

Two conditions have no smaller sound owner and use the registered `WholeGraph`
site: a root that denotes a value rather than a performable act
(`smusni.projection.graph.root-not-performable`), and a scope dependence naming
a binder that owns no scope anywhere in the graph
(`smusni.projection.graph.unbound-variable`). The second is a defensive route:
no graph the production builder emits reaches it today.

Nonfatal graph-attached diagnostics stay separate from projection errors. This
implementation deliberately does **not** promote a graph-attached diagnostic of
severity `error` into a projection failure: a graph carrying one still has a
faithful projection when every edge projects.

The host profiles present that one structured value and add nothing to it. The
command line converts records into standard labelled diagnostics in a
`semantic-projection` phase and writes them to stderr through the same
source-aware renderer the parser phases use, over the original Lojban, with
empty stdout and a nonzero exit; `--max-errors` truncates the printed records
and prints how many were omitted. HTTP returns an `application/problem+json`
server error carrying the stable `smusni-projection-failed` code, the requested
format, the records, the total/returned/truncated counts, and the statistics;
a malformed request keeps its ordinary client-error status. MCP returns the
tool error result with a readable summary first and the same envelope
serialized as one JSON text item. An explicit smusni request is never retried
in another format.

## Runnable support matrix

| Family | Current behavior | Boundary |
|---|---|---|
| Document packaging | One typed-grammar-parseable `(Smusni 0 ...)` datum with one trailing newline. The optional `Words` section carries one `(Word root definition)` card per content word that has a dictionary definition. A card whose word is not a bare lexical root — a defined zei-lujvo's multi-word surface such as `abu zei sance` — keeps that exact surface through the grammar's escaped spelling `|abu zei sance|`. Words with no dictionary definition have no card, because the grammar's card production has no place for a missing definition; the XML rendering states them as `KNOWN="false"` instead. | Spec sections 2.2 and 2.4 |
| Diagnostics | Nonfatal semantic diagnostics are collected once as structured `SmusniDiagnostic` values and kept out of the datum. Every failed projection edge is its own `SmusniProjectionFailure` record on the separate failure channel. An edge's identity is typed rather than textual: `(owner, declining boundary)` in the elaborator and `(kind, binder, use site)` in the scope planner. Re-entering one edge — for instance when a declining wrapper re-renders a child — records nothing further, while a second distinct boundary on the same owner is a second record. `SmusniRenderStats` retains exactly the three section-16.1 measurements: `failed_projection_edges`, the per-reason `failure_reasons` counts, and `failing_owners`. Ordering is deterministic from the typed channels; the internal sort key is not a public tuple contract. | Spec sections 2.4 and 16 |
| Variable spelling | Generated variables are composed from typed identity components through closed token tables, never by rewriting an identity's display text. Every token begins lowercase, so no generated name enters the PascalCase namespace that section 2.1 reserves for primitives, prelude names, types, and literals. Structural object kinds carry a `…Node` stem and referent sorts do not, which keeps the two namespaces injective without using letter case as the separator. The short `$x`/`$e`/`$p`/`$q`/`$u`/`$s`/`$v` alpha-renaming of section 15 item 3 is not yet implemented. | Spec sections 2.1 and 15 (item 3) |
| Predication | Named predicate terms, ordinary fills, `:n`, `:Eventuality`, numbered-only `DropPlace`, default closure omission, and explicit `Assert` are compact for their exact typed shapes. | Spec sections 4 and 5 |
| Relation formers | The canonical flat binary tanru graph projects to the registered former `(Tanru modifier head)` applied to the tertau's own places. The recognizer requires an implicit-juxtaposition `And` connective at predicate locus with exactly two children, a named tertau predication that is otherwise plain, a `Composition` link predication with exactly two plain arguments and no side fields, a fixed constant unary property abstraction over an exact `ce'u` entity parameter as the seltau, a `modifier-head` constructed relation label, and every supporting object private to this projection. Any other relation-former shape falls back, and the tanru-*like* relation question below is a separate unsupported family. | Spec section 4.6 |
| Logical composition | Registered ordinary truth-functional connectives render with `¬`, `∧`, `∨`, `→`, `↔`, and `⊕`. Unsupported connector metadata falls back. | Spec section 9.1 |
| Quantification | Restricted quantification over a planned binder renders `Every` or `Some`; unrestricted quantification renders bare `∀` or `∃`. Selection sources, plural quantifiers, inner/outer cardinality, witness export, and effect routing are not implemented and fall back. | Spec sections 9.2 through 9.5 |
| Shared values and recursion | A value used more than once is bound once and shared. Nonrecursive groups nest canonical single-binding `Let` forms; a recursive strongly connected component uses `LetRec` only when every initializer in the group is a top-level lambda. Any other recursive value falls back. | Spec sections 2.2 and 15 (item 2) |
| Contextual and deictic values | `Speaker`, `Audience`, `Now`, `Here`, and the proximity deictics `This`, `That`, and `Yonder` render as the declared atoms. A fixed context renders as the bare `Context` primitive and an underspecified one as `(Context deps…)` over its direct dependencies. A deictic with a non-current ground falls back. | Spec sections 3.5 and 6.1 |
| Fixed descriptions | One force-local, exact entity `lo`, `le`, or `la` reference becomes `Bind` plus `Refer`. For `lo` and `le` the descriptor body is the property: it retains ordinary filled conventional arguments, `le` retains the represented speaker/audience `skicu` property without asserting classification, and a veridical restrictive `poi` is conjoined inside the one property. The compact `la` shape is exactly the fixed name description — `la` with a cmevla name and no descriptor body and no relative clause — whose property is `(Named "name" $var)`; a `la` description over a selbri body carries no name at all and has no compact route. Nested reference effects, richer descriptors, incidental/nonveridical relatives, and shared placement fall back. | Spec sections 6.3, 8.3, and 8.4 |
| Modals and tense | Exact represented modal predicates are joined to the host by `Joi` and share the graph event under one lambda-shaped existential. The verified `before`/`at`/`after` event relations lower to `purci`/`cabna`/`balvi`. Other tag maps and event facets fall back. | Spec sections 10.1 through 10.4 |
| Discourse | Same-topic items use `Do`; exact paragraph provenance uses `NewTopic` or `Resume`, with explicit `Perform`/`PerformUtterance` crossings at the transition operand. | Spec sections 7.1 and 7.2 |
| Force | `Assert` and `Mention` are compact for their exact typed shapes. `Ask` is emitted only by the question projection below, so ask force over content that is not a typed question fails closed rather than applying `Ask` at the wrong type. Quote, parenthetical, subordinated, command, and vocative force fall back. | Spec sections 1.3 and 7.1 |
| Questions | Exact direct polar questions render `Ask` plus `Polar`; exact entity argument questions and atomic `ti mo`-style relation questions render `OpenQ`, with the latter retaining a typed open predicate row and explicit `Close`. Tanru-like `ti mo zdani`, embedded, multi-slot, and richer questions fall back. No answer family is projected: section 12.2's `Answer`, `PolarAnswer`, `TupleAnswer`, `ContextualAnswer`, and `UnresolvedAnswer` have no compact route. | Spec section 12 |
| Abstractions | Exact entity properties render as lambdas over the abstraction's own entity parameters, which may be more than one, and exact proposition crossings use `Reify` when their complete model shape is eligible. Event-valued and richer abstraction families fall back. | Spec section 11 |
| Utterance entries | Retained entries use the fresh `UtteranceToken` binder and registered `SpeakerOf`, `AudienceOf`, `LocutionOf`, deictic, and `Realizes` facts. Unsupported force and asides are projection errors. | Spec section 7.2 |
| Everything else | **Catch-all:** every specification family not named as compact above is a projection error. In particular that covers indicators and displayed content (7.4), sign and quotation constructors (7.3, 13.3), set/group descriptions and referential connections (8.5), simultaneous termsets (9.5), witness export (9.4), respectively-distribution, quantities, and math beyond exact integer literals and binary kernel arithmetic (13.1, 13.2), and answers (12.2). No compact head is emitted for any of them. | Whole specification |
| Failure attribution | Every failure is attributed to the smallest owner the planner or elaborator actually held, and its span is resolved at that moment. Attribution to the smallest owner whose *expected type* is also proved is still future work (see the limitations below). | Spec sections 14.4, 16.2 |

## What the current acceptance gate does and does not prove

Structural tests parse every rendered document with `parse_v0_document`. That
gate validates the closed serialization grammar of specification section 2.2
and the typed annotations those productions carry: document packaging, the
lexical token grammars, binder and declaration shapes, and the declared type
expressions that appear in the output.

The acceptance parser is the compact public grammar and nothing else. The
`Fallback`, `TypedGraph`, and `Raw*` productions of the internal debug codec
are not part of it and are rejected by it; the codec lives in
`notation::sexpr::internal_raw` with its own parser and its own tests, and
`internal-raw.md` is its description. Both parsers can receive untrusted text,
so both bound list nesting, document size, and integer digit length, and every
atom built from data-derived text goes through the fallible constructor rather
than the panicking one.

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

- Dynamic host planning: `ro da poi gerku cu bajra` currently fails at
  definition placement, and the tanru-like `ti mo zdani`
  question still has unlicensed relation/property crossings and event hosting.
  Implement graph-owned accessibility, force
  handlers, dependency lifting, and legal shared capture before enabling these
  compact paths (sections 6.2–6.4 and 12.1).
- Simultaneous termsets: both grammar-licensed prenex spellings retain their one
  semantic `QuantifierBundle`, but projection currently fails.
  Implement the approved equal-scope reduction rather than choosing a nested
  order or reviving one of the forbidden record shapes of section 14.3
  (section 9.5, and section 12 of the samples).
- Quantification: the compact lowering recognizes the simple `Every`/`Some`
  function shapes after scope planning and bare `∀`/`∃` for unrestricted
  quantification, but selection sources and plural quantifiers (section 9.2),
  inner and outer cardinality (section 9.3), witness export (section 9.4), and
  effect routing (section 6.4) are not yet complete.
- Signs and quotation: structured and opaque quotation currently fail; no
  transcript is synthesized and no retired `Quotation` record
  prints. Add `StructuredQuote`, `OpaqueQuote`, raw sign constructors, and sign
  token facts only from graph-owned identities (sections 7.3 and 13.3).
- Event abstractions and facets: unsupported actuality, aspect, recurrence,
  space, path, and abstraction details fail. Add only registered
  lowercase/prelude reductions with their actual shared event (sections
  10.3–10.4 and 11.2–11.3).
- Respectively, collections, and richer math: these all fail except for exact
  integer literals and binary kernel arithmetic.
  Generic composition records and non-exact quantities do not borrow callable
  names from the registry. Implement `ZipWith`, typed collection kernels,
  generalized-quantifier reductions, and registered math rows without generic
  `Math`, `Quantity`, or `Respectively` records; the generalized-quantifier
  reductions in that list belong to sections 9.2 and 9.5 rather than to
  section 13 (sections 13.1 and 13.2).
- Internal capture tuple payload coordinates currently use deterministic
  `item1`, `item2`, and so on because serde does not expose source field names.
  Replace them with projection-declared stable names before the internal codec
  is treated as settled (`internal-raw.md`).
- Internal capture numeric scalar type names currently expose the serializer's
  normalized `i128`, `u128`, and `f64` carriers. Bind them to declared model
  scalar types (`internal-raw.md`).
- Failure attribution: a record names the owner and use site the planner or
  elaborator held. It does not additionally prove that the owner is the
  smallest one whose expected v0 type is established, which is what sections
  14.4 and 16.2 ask of a registered `TypedPosition` site.
- Failure classes: every reason row carries a reviewed class, but the review is
  a first pass over one reason id per condition. Where several distinct causes
  share one registered id, the class states what that id's dominant runtime
  cause is; splitting such an id is registry work rather than renderer work.

## Registry status

The checked-in semantic-surface scanner, 882 authored dispositions, generated
runtime registry, and registry validation are retained from the interrupted CP2
work because they are useful executable defect detectors. They are not
described as immutable, exhaustive, or final: the current runnable slice still
fails on many rows classified for later direct lowering, and the runtime
application/signature registry requires further semantic review.
No renderer behavior is licensed merely by the existence of an inventory row.

The ledger records the five closed semantic dispositions of section 14.4 plus
the non-semantic `Failure` marker of section 14.2, which closes nothing and
counts toward nothing. Section 14.4 also splits a coordinate's normative
semantic disposition from one implementation's coverage of it; that coverage
value is not yet a separate recorded field, so a coordinate whose route this
renderer lacks is still distinguished only through its reason row's
`RouteUnavailable` class.
`sources/must-compact-witnesses.txt` is part of the retained material, and its
name overstates it: it is a structurally exercised witness corpus and a
registry-audit input, not a claim that each of its lines renders compactly.

## Regenerating and checking the registry

Every `cargo build -p jbotci-semantics` verifies the checked-in registry in
check mode through `build.rs`. The reviewer-facing entry point is the same
generator run directly:

```sh
cargo run -r -p jbotci-semantics --example smusni_v0_bundle -- --check
cargo run -r -p jbotci-semantics --example smusni_v0_bundle -- --generate
```

`JBOTCI_SMUSNI_V0_BUNDLE_MODE=generate cargo build -p jbotci-semantics` is the
equivalent through the build script. Generation is deterministic: running it on
an already-current tree changes no byte. The example writes only its compiled
policy table, into `$CARGO_TARGET_DIR/smusni-v0-scratch/` or the workspace
`target/` directory when that variable is unset.

The generator reads the checked-in registry sources under
`data/smusni-v0/sources/`, plus the vendored dictionary snapshot at its live
path in `jbotci-dictionary-data`. Every one of those files must survive
`cargo package`, because the Python source distribution rebuilds the workspace
from the extracted archive alone;
`crates/jbotci-semantics/tests/smusni_v0_bundle.rs` asks cargo itself rather
than restating its pruning rules.

`data/smusni-v0/sources/smusni/spec.md` is the registry's own copy of the
dual-homed specification. It exists because prelude signatures and canonical
definitions are extracted from the specification text and a distribution
carries no `docs/` tree. The test suite asserts that it is byte-identical to
`docs/smusni/spec.md`; `.gitattributes` marks both `-text` so a Windows
checkout cannot make them differ.

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

| Slice | Inputs | Documents | Projection failures | Render panics | Notes |
|---|---:|---:|---:|---:|---|
| `phaseb` | 48 | 16 | 32 | 0 | the frozen structural corpus |
| `cll` | 1,247 | 204 | 1,041 | 0 | 2 pre-render morphology failures |
| `focused` | 16 | 6 | 10 | 0 | |
| `alice-lines` | 2,436 | 48 | 1,036 | 0 | the remaining 1,352 inputs fail earlier parsing or building, mostly syntax |
| `alice-whole` | 1 | 0 | 1 | 0 | one failed projection over 49,172 objects |

The whole-Alice run is the memory reference point: 7,523,428 KiB RSS after the
graph build and a 7,861,288 KiB peak, so it needs a host with about 8 GiB free.
Those figures vary by a few MiB between runs. The pre-#753 peak was
9,523,660 KiB; the roughly 2 GiB difference was the internal whole-graph
capture, and the failure path no longer serializes a graph at all. That leaves
the 7.5 GiB graph-build baseline, which is separate work.

The same sweep reports the failed-edge channel and the owner channel
separately. `Failed edges` is `SmusniRenderStats::failed_projection_edges`,
which is also the number of failure records and the sum of `failure_reasons`.
`Failing owners` is `SmusniRenderStats::failing_owners`, the distinct owners
those records name, and `multi-edge owners` counts the owners named by more
than one record. Like the table above, these are observations of the current
slice rather than expectations.

| Slice | Failed edges | Failing owners | Multi-edge owners |
|---|---:|---:|---:|
| `phaseb` | 91 | 88 | 3 |
| `cll` | 2,806 | 2,643 | 113 |
| `focused` | 30 | 29 | 1 |
| `alice-lines` | 2,824 | 2,599 | 149 |
| `alice-whole` | 376 | 122 | 55 |

The edge and owner measurements differ because they count different things,
which is why they are reported side by side rather than derived from one
another. The per-edge identity in the code is justified by the typed law tests
over the channel representations, not by these numbers.

Of the four section-16.2 classes, three are reachable from real input today:
`RouteUnavailable` for most families, `TrackedSpecGap` for `tu'a` sumti raising
(`mi djica tu'a do`), and `InvalidGraph` for an ill-scoped binder (the frozen
`question-multiple-domains` witness `pau xo ma mo xu`).
`ImplementationInvariant` is not reachable, which is what it should mean, and
`crates/jbotci-semantics/tests/smusni_projection_failure.rs` asserts that
rather than leaving it to chance.

The five CLL inputs that previously panicked while manufacturing a variable
atom — `c11e12d2`, `c11e3d1`, `c11e3d3`, `c11e3d4`, and `c11e9d1` — no longer
panic. They are retained as structural regressions beside one witness per
eventuality subtype, so the sweep's zero-panic result has a cheap test-suite
counterpart.
