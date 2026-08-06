# Smusni version-0 implementation status

This renderer is an experimental implementation candidate, not a minted or
exhaustive version-0 implementation. Its design inputs are this directory's
`spec.md` and `samples.md`, which are dual-homed with the smusni design
repository. The samples are design specimens, not output expectations.

**The specification is ahead of this implementation.** Issue #753 made an
unproved projection a product error rather than a conforming raw document, and
the specification now says so throughout. The renderer still returns a raw
`TypedGraph` document on an unproved projection; making the API fallible and
wiring the CLI, server, and MCP error surfaces is the second increment of that
issue. Every divergence this creates is listed below. The reason-id namespace
is already `smusni.projection.`, and the registry already calls its table
`ProjectionFailureReasonRow`. Three further declared shapes are ahead of the
registry and likewise belong to the second increment: the rows do not yet
carry the `failure-class` field the section-14.2 schema declares (so the
section-16.2 class breakdown cannot yet be reported); the
`smusni.projection.graph.root-not-performable` and
`smusni.projection.graph.unbound-variable` ids the specification requires do
not exist as rows, and no row uses a `WholeGraph` failure site — a whole-graph
capture is currently labelled with its first failed edge's reason; and the
disposition ledger still records the retired `TypedFallback` value where the
specification now requires the `Failure` marker with reason-row classes.
Separately, the specification names `tu'a`, `co'e`, and `do'e` as tracked spec
gaps (section 14.4); behaviorally they are part of the catch-all below, and no
compact route may be attempted for them until the gap is closed.

## Runnable support matrix

| Family | Current behavior | Boundary |
|---|---|---|
| Document packaging | One typed-grammar-parseable `(Smusni 0 ...)` datum with one trailing newline. The optional `Words` section carries one `(Word root definition)` card per content word that has a dictionary definition. A card whose word is not a bare lexical root — a defined zei-lujvo's multi-word surface such as `abu zei sance` — keeps that exact surface through the grammar's escaped spelling `|abu zei sance|`. Words with no dictionary definition have no card, because the grammar's card production has no place for a missing definition; the XML rendering states them as `KNOWN="false"` instead. | Spec sections 2.2 and 2.4 |
| Diagnostics | Collected once as structured `SmusniDiagnostic` values and kept out of the datum. Every failed projection edge gets its own `Fallback` record with a stable reason code and a stable message fixed by its typed cause. Owner and use-site identities travel as evidence where the failure site has them; they are optional and a consumer must not require them. An edge's identity is typed rather than textual: `(owner, declining boundary)` in the elaborator and `(kind, binder, use site)` in the scope planner. Re-entering one edge — for instance when a declining wrapper re-renders a child — records nothing further, while a second distinct boundary on the same owner is a second record. `SmusniRenderStats::failed_projection_edges` and `fallback_reasons` both count failed edges; `SmusniRenderStats::object_fallbacks` counts graph objects and is a different measurement. Ordering is deterministic from the typed channels; the internal sort key is not a public tuple contract. **Deliberate deviation from the specification:** section 16 makes a projection failure a result with no document and requires the CLI profile to write labelled records to stderr. This implementation returns a raw document instead and prints nothing; both are increment 2 of issue #753. The provisional formatter strings are not printed anywhere. | Spec sections 2.4 and 16 |
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
| Utterance entries | Retained entries use the fresh `UtteranceToken` binder and registered `SpeakerOf`, `AudienceOf`, `LocutionOf`, deictic, and `Realizes` facts. Unsupported force and asides fall back to the whole-graph internal capture (product projection errors from increment 2). | Spec section 7.2 |
| Everything else | **Catch-all:** every specification family not named as compact above falls back to the whole-graph internal capture (a product projection error once increment 2 lands). In particular that covers indicators and displayed content (7.4), sign and quotation constructors (7.3, 13.3), set/group descriptions and referential connections (8.5), simultaneous termsets (9.5), witness export (9.4), respectively-distribution, quantities, and math beyond exact integer literals and binary kernel arithmetic (13.1, 13.2), and answers (12.2). No compact head is emitted for any of them. | Whole specification |
| Failure surface | **Diverges from the specification.** An unproved projection should return no document. This slice still emits one whole-graph internal capture as the document, with graph-owned `%id` sharing and a registered `smusni.projection.` reason, and never emits a smaller local capture. Increment 2 of issue #753 replaces this with a fallible API. | `internal-raw.md` |

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
- Failure attribution: the current conservative implementation attributes every
  unproved projection to the whole graph. Attribute to the smallest owner only
  when the expected type and the minimum graph-owned owner are both proved
  (sections 14.4 and 16.2).

## Registry status

The checked-in semantic-surface scanner, 882 authored dispositions, generated
runtime registry, and registry validation are retained from the interrupted CP2
work because they are useful executable defect detectors. They are not
described as immutable, exhaustive, or final: the current runnable slice still
fails on many rows classified for later direct lowering, and the runtime
application/signature registry requires further semantic review.
No renderer behavior is licensed merely by the existence of an inventory row.

**Diverges from the specification.** Section 14.4 splits a coordinate's
normative semantic disposition from one implementation's coverage of it and
removes `TypedFallback` from the semantic dispositions. The checked-in ledger
still records `TypedFallback` as a sixth disposition value. Reclassifying 882
rows is semantic work on the coverage taxonomy rather than the increment-1
de-ceremony, so it is deferred; nothing in the current renderer depends on the
distinction, because the reason ids and their owners are already exact.
`sources/must-compact-witnesses.txt` is part of that retained material, and its
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

| Slice | Inputs | Renders | Render panics | Notes |
|---|---:|---:|---:|---|
| `phaseb` | 48 | 48 | 0 | the frozen structural corpus; 16 compact documents, 32 unproved projections |
| `cll` | 1,247 | 1,245 | 0 | 2 pre-render morphology failures; 204 compact documents, 1,041 unproved projections |
| `focused` | 16 | 16 | 0 | 6 compact documents, 10 unproved projections |
| `alice-lines` | 2,436 | 1,084 | 0 | the remaining inputs fail earlier parsing or building, mostly syntax |
| `alice-whole` | 1 | 1 | 0 | one unproved projection over 49,172 objects |

The whole-Alice run is the memory reference point: 7,523,428 KiB RSS after the
graph build and a 9,523,660 KiB peak after rendering, so it needs a host with
more than 10 GiB free. Those two figures vary by a few MiB between runs. The
2 GiB between them is the internal whole-graph capture; increment 2 removes it
by never serializing a graph on the failure path. It does not remove the 7.5 GiB
graph-build baseline, which is separate work.

The same sweep reports the object statistic and the per-edge diagnostic channel
separately. `Objects not projected` is `SmusniRenderStats::object_fallbacks`;
`failed edges` is `SmusniRenderStats::failed_projection_edges`, which is also the
number of failure records and the sum of `fallback_reasons`. `Failing owners`
counts the distinct owners those records name, and `multi-edge owners` counts the
owners named by more than one record. Those three statistic names still carry
the retired vocabulary; increment 2 renames them with the API. Like the table
above, these are observations of the current slice rather than expectations.

| Slice | Objects not projected | Failed edges | Failing owners | Multi-edge owners |
|---|---:|---:|---:|---:|
| `phaseb` | 277 | 91 | 88 | 3 |
| `cll` | 12,583 | 2,806 | 2,643 | 113 |
| `focused` | 29 | 30 | 29 | 1 |
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
