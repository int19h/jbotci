# Grammar parity epoch 6c: Zantufa term binding

Epoch 6c is the last slice of the epoch-6 arc and carries D5 (#827) alone: the
term binding rolling Zantufa has and neither camxes-standard nor camxes-exp
does. Its design is the epoch-6 plan v2-as-amended D5 section together with the
reconciliation's B3 and B5 adjudications, and the D5 groundwork table in
[the epoch 6b ledger](grammar-parity-epoch-06b-terms.md), which probed every
surface below against all three running parsers before this epoch began. The
implementation base is `2397912147`, the epoch-6b merge.

| Section | Issue | State |
| --- | --- | --- |
| Connectorless BO at the term and sumti tiers | #827 | complete |
| The JAI term: overt sumti, explicit KU, elision | #827 | complete |
| FA JOIK-chains in the Zantufa tag term | #827 | complete |
| `ce'e`-as-BO fidelity note | #827 | complete, documented gap |
| Consolidated expectations, comparer re-baseline, peak RSS | — | complete; results below |

## The four surfaces, re-probed at implementation

Every row was re-probed against the three running parsers at this epoch's head
rather than carried over from the 6b table:

| Surface | camxes-standard | camxes-exp | rolling Zantufa | jbotci before | jbotci now |
| --- | --- | --- | --- | --- | --- |
| `pu ko'a bo ca ko'e broda` | rejects | rejects | `term_1` | rejects | Zantufa term tier |
| `ko'a bo ko'e broda` | rejects | rejects | `sumti_2` | rejects | Zantufa sumti tier |
| `ko'a ba bo ko'e broda` | rejects | rejects | `sumti_2`, tag-bearing | rejects | Zantufa sumti tier |
| `ko'a goi pu ko'e bo ca ko'i broda` | rejects | rejects | GOI payload | rejects | Zantufa normal-flavour tier |
| `ko'a goi ba ko'e .e bo vi ko'i broda` | rejects | accepts | accepts | exp T4-normal | **unchanged** (B3 pin) |
| `jai ko'a broda` | rejects | rejects | `tag_term` | accepted, warned | unchanged |
| `jai pu ko'a broda` | rejects | rejects | `tag_term` | accepted, warned | unchanged |
| `jai ku broda` | rejects | rejects | `tag_term`, explicit KU | **rejects** | accepted, warned |
| `jai cu broda` | rejects | rejects | `tag_term`, elided KU | **rejects** | accepted, warned |
| `jai broda` | accepts | accepts | JAI **selbri** | JAI selbri | unchanged |
| `mi jai pu broda` | accepts | accepts | JAI **selbri** | JAI selbri | unchanged |
| `fa je fe ko'a broda` | rejects | accepts | `tag_term` chain | **rejects** | Zantufa chained place tag |
| `ko'a ce'e ko'e broda` | CEhE termset | CEhE termset | a sumti **BO** connection | CEhE termset | unchanged, gap row |

## D5-1: the connectorless BO arms

Zantufa writes both BO tiers with an optional connective —
`term_1 <- term_2 (joik_ek? BO_clause term_2)*` (zantufa-1.9999.peg:28) and
`sumti_2 <- sumti_3 (joik_ek? tag? BO_clause sumti_3)*` (:35) — where every
sourced BO joint requires one. jbotci therefore carries exactly the delta, the
connector-ABSENT joint, as an alternative of its own rather than by relaxing the
sourced shape (B3). Three arms land, one per BO position the ladder has:

| Arm | Position | Operand |
| --- | --- | --- |
| `zantufa_bound_sumti_tail` | the second arm of the new `sumti_bound_tail` sum | the recursive `sumti_bound`, as the sourced tail takes |
| `zantufa_bound_term_continuation` | the second arm of the new `bound_term_continuation` sum | the guarded `simple_term` |
| `zantufa_bound_normal_term_continuation` | the second arm of the new `normal_term_bo_continuation` sum | the unguarded `normal_term_atom` |

Both term-tier arms are continuations of the *existing* connection nodes rather
than connection nodes of their own, which is what Zantufa's own tree shape asks
for: its `term_1` is one node with a flat continuation list whose members may
individually carry or lack the connective, and `pu ko'a bo ca ko'e .e bo vi ko'i
broda` is one such mixed node (probed). Modelling the Zantufa joint as a
separate connection product would have made that surface unrepresentable, and
would have added a second BO connection per flavour to seven ladder levels for
no gain.

### What is not in the `recursive` block, and why

The three BO arms and the JOIK-chained place tag are referenced from the levels
that list them — nine leaf inventories in the place tag's case — and none of them
joins the grammar's `recursive` block. That is the same disposition
`place_tagged_sumti_term` and `tagged_or_elided_sumti` already have, and it rests
on the same reading of why the block exists: a rule outside it is rebuilt inline
at each reference site, which matters when the subgraph is large (a ladder level,
or `zantufa_gek_termset` with its whole `term+` machinery) and not when it is a
token, a chain of token pairs and references to nodes that are themselves
declared. Every child these rules reach — `sumti`, `normal_term`, `simple_term`,
`normal_term_atom`, `tanru_unit_atom` — is a block member already, so what an
inline rebuild copies is the leaf's own handful of combinators.

### Placement: the baseline BO tier, not Zantufa's rule number

Gemini-3's renumbering note is applied literally. Zantufa's `sumti_1` is its
loose tier and `sumti_2` its BO tier, while the baseline backbone jbotci
composes puts the loose tier at `sumti_2` and BO at `sumti_3`; the arm is placed
at the tier, so `ko'a bo ko'e` binds tighter than a loose continuation, exactly
as it does upstream. The term tier is the same question with the same answer:
the arm joins the BO-bound continuation lists, not the loose ones.

### The classifier, and what measuring it changed

B3 asks for a `#634`-pattern whole-candidate `reject_output` classifier on the
Zantufa arm on top of the connector-absent grammar shape, "wherever it could
still capture a connective-present candidate". `crate::grammar::baseline_bo`
answers that question rather than assuming it. The sourced owner can never take
a candidate of this arm's extent, because reproducing it needs a connective at
the very joint whose absence defines the arm and neither connective inventory
matches the empty string; each classifier states that proof by destructuring the
completed operand exhaustively and without `..`, so a later widening toward
Zantufa's literal `joik_ek?` cannot land without the proof being revisited.

One shape was measured rather than argued, because it is the only one where the
arm's extent contains a connective at all. jbotci spells a BO chain by recursion
(camxes.peg:143) where Zantufa spells it as a flat list, so
`ko'a bo ko'e .e bo ko'i broda` nests a sourced tail inside the connectorless
tail's own operand. The first cut rejected that candidate; the measurement is
that rejecting it does **not** hand the extent to the sourced owner — that owner
needs a connective directly after `ko'a`, and there is none — but pushes the
whole surface onto the term tier, where the sumti operand swallows `.e bo ko'i`
and the reading stops matching Zantufa's own `sumti_2`. Silently re-reading a
surface no source reads that way is what this epic's ownership policy forbids,
so the nested sourced tail is left where Zantufa puts it, and both nesting
orders are witnessed.

### Ownership rows

- Connective-present, stag-less: `ko'a goi ba ko'e .e bo vi ko'i broda` stays the
  exp T4-normal arm's in every configuration, including `(zantufa)`. That is the
  B3 witness, and it is the 6b fixture pair `goi-payload-stagless-bo` and
  `goi-payload-stagless-bo-zantufa`; the arm their trees select is unchanged, and
  the only edit either takes is the mechanical continuation-sum wrapper the
  regeneration applies everywhere.
- Connectorless: `ko'a goi pu ko'e bo ca ko'i broda` is Zantufa's alone;
  camxes-exp rejects it outright.

### The six-configuration family

The family is read over the zantufa axes, as epoch 6b established after
`DialectFeature::TermHierarchy` retired: omitted dialect, `()`,
`(+zantufa-terms)`, `(+zantufa-connectives)`, both, and `(zantufa)`. Measured at
this epoch's head:

| Surface | omitted | `()` | `(+zantufa-terms)` | `(+zantufa-connectives)` | both | `(zantufa)` |
| --- | --- | --- | --- | --- | --- | --- |
| `ko'a bo ko'e broda` | reject | reject | accept | reject | accept | accept |
| `pu ko'a bo ca ko'e broda` | reject | reject | accept | reject | accept | accept |
| `ko'a goi pu ko'e bo ca ko'i broda` | reject | reject | accept | reject | accept | accept |
| `ko'a goi ba ko'e .e bo vi ko'i broda` | accept | accept | accept | accept | accept | accept |

Both tiers witness all six rows: `zantufa-bo-term-connectorless-*` and
`zantufa-bo-sumti-connectorless-*` each carry the omitted, `()`, terms-axis,
connectives-axis, both-axes and `(zantufa)` configurations, and each fixture's
provenance names the row it is. Both arms name the same feature, so on the
current grammar the sumti tier's `()` and both-axes rows cannot say anything the
term tier's do not; they are pinned anyway, because naming one feature twice is
an implementation fact rather than a property of the surfaces, and if the two
ever diverge the sumti tier would otherwise move with nothing failing.

Both BO arms are keyed to `ZANTUFA-TERMS` rather than `ZANTUFA-CONNECTIVES`:
what they add is a term/sumti binding whose defining property is that it carries
no connective at all, and the connectives axis is where Zantufa's connective
*inventory* widenings live (the GIhI terminator, the n-ary GIK branches, the
NUhI-less GEK termset). The `(+zantufa-connectives)` rejection rows pin that
choice.

## D5-2: the JAI term

Zantufa's `tag_term` is
`(!gek (tag !(!tag selbri) !gek_bridi_tail !BO / (FA_clause (joik FA_clause)* /
JAI_clause tag?) !tanru_unit_1) (sumti / KU_elidible))` (zantufa-1.9999.peg:31).
jbotci's JAI term carried the JAI and its optional tag already; this epoch adds
the two halves it was missing.

- **The payload.** `(sumti / KU_elidible)` is the same payload every other
  tag-led term takes, so `jai_tagged_sumti_term.sumti` becomes the shared
  `tagged_or_elided_sumti`: the sumti may be overt, replaced by an explicit KU
  (`jai ku broda`), or elided outright (`jai cu broda`). Only one fixture in the
  tree carries a JAI tag term, so the re-typing is one file.
- **The boundary.** `!tanru_unit_1` is a structural negative predicate at the
  payload position, spelled `assert !tanru_unit_atom;` at the named DSL site —
  `tanru_unit_atom` is jbotci's name for that level. It is what keeps the
  elidable payload from swallowing the selbri: `jai broda` and `mi jai pu broda`
  are the JAI **selbri** in every parser including Zantufa (:52), and without the
  guard the term would take the JAI, elide its KU and leave `broda` to be found
  again as the sentence's selbri. The guard is structural rather than a token
  class because the boundary it draws is exactly where a tanru unit may begin.

Threading `tanru_unit_atom` into the JAI term means the nine leaf-listing term
levels carry it as a parameter; they are all members of the grammar's
`recursive` block, so no reference site changes.

## D5-3: FA JOIK-chains

The other half of the same Zantufa alternative is `FA_clause (joik FA_clause)*`.
It lands as `zantufa_joik_chained_place_tag_term`, a leaf of its own with the
chain REQUIRED, rather than as a `zero_or_more` continuation on the shared FA
term: an optional continuation list would re-type all 1,342 baseline FA fixtures
to record a list that is empty in every one of them, and requiring at least one
continuation also makes the arm structurally disjoint from the shared term, so
arm order cannot change which node a sourced surface gets. The `!tanru_unit_1`
guard applies here for the same reason it applies to the JAI term.

Two details are worth the ledger:

- **The connective inventory is JOIK-or-JEK.** Zantufa's `joik` is
  `GAhO? NA? SE? JOI GAhO?` (:68), and its JOI selma'o (:556) holds every JA word
  — `je`, `ja`, `jo`, `ju` — as well as the JOI ones, so the sourced domain at
  this position is what jbotci spells `standard_statement_connective`. Words
  Zantufa lexes into JOI and jbotci lexes elsewhere, `ji` being the A word among
  them, are a documented gap rather than a widening of this position.
- **The shared FA term declines the chained surface itself.** Zantufa consumes
  the chain greedily inside one alternative, before the payload position exists;
  jbotci's chain is a separate leaf, and a term that has already matched is never
  re-entered, so listing the chain arm later is not enough — `fa je fe ko'a broda`
  would match `fa` with an elided KU at the shared term and never reach the chain.
  A `ZANTUFA-TAGS`-gated negative lookahead on the shared term declines exactly
  the chained surface. The alternative — listing the chain arm first — was
  implemented and rejected: it changes the expected-set vocabulary of unrelated
  term diagnostics (`place tag` drops out of `mi ku i do ku i mi klama`'s
  expectation list), which the CLI recovery-diagnostics pins caught.

## D5-4: `ce'e` as BO

Zantufa lexes `BO <- ce'e / bo` (zantufa-1.9999.peg:529), so
`ko'a ce'e ko'e broda` is a sumti BO connection there and a CEhE termset group in
camxes-standard and camxes-exp alike. Both readings cover the identical extent
and mean different things, so under the standing reinterpretation ruling this is
a **documented gap plus a fidelity-flag candidate**, not a baseline re-pin: the
baseline CEhE reading is what jbotci keeps in every configuration including
`(zantufa)`, witnessed, and the flag is recorded as a follow-up candidate rather
than minted, exactly as epoch 6b's D2 rejections were.

## Gap ledger

| Gap | Surfaces | Disposition |
| --- | --- | --- |
| `ce'e` as BO | `ko'a ce'e ko'e broda` | baseline CEhE reading kept; fidelity-flag candidate recorded, not minted |
| Zantufa JOI words jbotci lexes elsewhere | `fa ji fe ko'a broda` | rejected; the chain takes JOIK-or-JEK, and `ji` is an A word here |
| Zantufa's one-ladder term flavour | `pu ko'a bo ca ko'e .e bo vi ko'i broda` at a sentence-leading position | rejected: the absorption-safe tier requires the stag before a connective-present BO (#796), which is pre-epoch and unchanged |

## The consolidated regeneration

Three joint positions moved from a single product to a two-arm sum, and one
payload widened, so every expectation that records one of them gains an arm
wrapper and nothing else:

| Position | Wrap | Files |
| --- | --- | --- |
| `SumtiBoundSyntax.bound_tail` | `BoundSumtiTail(..)` | 34 |
| `StagBoundTermConnectionSyntax.continuations` | `StagBoundTermContinuation(..)` | 5 |
| `BoundNormalTermConnectionSyntax.continuations` | `BoundNormalTermContinuation(..)` | 6 |
| `JaiTaggedSumtiTermSyntax.sumti` | `Sumti(..)` | 1 |

Forty-four pre-epoch fixtures move, which is the whole affected population rather
than a sample: 42 are regenerated by the project's own writer and two —
`corpus/camxes/1301` and `corpus/camxes/813`, whose recovered trees carry a BO
tail — are spliced, because `fixture-rewrite` refuses any fixture carrying an
xfail pin and must. Each is copied without its `xfail` table, regenerated, and
given its original `status` and `xfail` lines back, with the rebuilt accepted
status verified against `xfail.accepted-status`. The 34/5/6/1 split sums to 46
rather than 44 because two fixtures carry two of the wraps at once.

The comparer is re-baselined to `git archive 2397912147 tests/fixtures`, this
epoch's own implementation base, so it stays git-derivable. Epoch 6b's
`goi-payload-retyping` class is **retired** rather than left wired at zero: its
trigger was the narrow `relative_sumti` payload level, and 6b deleted that rule
outright, so it is absent from the baseline archive too and there is nothing left
for it to classify. The position it governed now reads `normal_term` on both
sides, where a stray narrow-payload arm still fails closed as manual residue.
The four earlier classes stay wired, and must find nothing.

| Category | Count |
| --- | --- |
| Changed pre-epoch fixtures | 44 |
| `bo-joint-sum-wrapper` | 43 |
| `jai-payload-widening` | 1 |
| `flat-sum-wrapper`, `goi-payload-retyping`, `pehe-cehe-retyping`, `stagless-bo-route-rejection`, `t3-loose-connection-warning` | 0 each |
| Manual residue | 0 |
| Prose-only provenance edits | 0 |
| Epoch-new witnesses (authored, unclassifiable) | 38 |

The level-inventory half of the re-baseline moves with the archive. Both
inventories are re-derived against the baseline grammar and re-checked by
`tools/tests/test_compare_term_hierarchy_expectations.py`, which now also sees
`when feature(...)`-gated enum arms — this epoch adds one, the JOIK-chained place
tag, at all nine levels — and gains a case asserting that the wrap table names
rules the baseline grammar actually has, so a typo there fails the test instead
of silently inflating manual residue.

## What pins an epoch-new witness

An epoch-new witness has no baseline entry, so no mechanical class inspects it:
what it pins is the whole audit. Every one of the 38 therefore carries an exact
`expectations.syntax.diagnostics` list — the full warning stream where warnings
fire, and an explicitly empty list where the expectation is silence. Five success
witnesses are silent, and each says something by being so: the two `ce'e`
baseline-CEhE readings, the two JAI selbri-absorbed forms and the unchanged plain
FA term all take arms this epoch did not touch, and a warning appearing on any of
them would mean a new arm had captured the surface. Omitting the key is not a
weaker pin but no pin
at all: the tree stays fixed while the construct is free to stop warning, or
start, with nothing failing. `fixture-rewrite` also fills the list only where the
key already exists, so an omission perpetuates itself.

The comparer enforces this rather than trusting it. It already derives the
epoch-new set from `git diff --diff-filter=A`, and it now reports every member of
that set that pins no diagnostics list and exits 1, alongside the unpaired-fixture
and witness-delta gates; a witness with no `expectations.syntax` at all is
reported for the same reason rather than skipped. Four cases in
`tools/tests/test_compare_term_hierarchy_expectations.py` exercise the check on a
synthetic tree from both sides and assert the property of the repository tree
itself, and the gate run below was repeated against a copy of the fixture tree
with one witness's pin deleted: the tool reports that witness by name and exits
1, so the check is fail-closed in the tool and not only in its unit test.

## The ASK carried to the lead

One deviation from D5's letter was raised with `lead-jbotci-801` before
submission rather than settled unilaterally: B3 asks for the classifier to
"reject any candidate whose completed extent carries a present connective", and
the classifier this epoch ships rejects nothing, because the one shape where the
question arises is the nested sourced tail measured above, where rejecting
reinterprets rather than returns. The assumption recorded with the ASK, and the
one this epoch proceeds under, is that the classifier stays wired at all three
arms as the extent proof — exhaustive destructure, no `..`, candidate returned to
the Zantufa arm because the sourced owner can never take its extent — and that
the nested sourced tail is left where Zantufa puts it, with both nesting orders
witnessed.

## Fixture counts

The tree moves 26,479 → 26,517 with this epoch's 38 witnesses; the xfail count is
unchanged at 513, because the two xfail fixtures this epoch touches keep their
pins and only their trees move.

## Pre-submission gate (round 1)

Run at `f436f0e61c`, the tip of the code, expectation and ledger commits; the
conformance-ledger commit above it changes only documentation, which nothing
under test reads.

| Gate | Result | Log |
| --- | --- | --- |
| `cargo fmt --all --check` | clean | `epoch06c-gate-fmt.log` |
| `cargo test -r --workspace` | 2,312 passed, 0 failed, 16 ignored | `epoch06c-gate-workspace.log` |
| `fixture-test --profile all` | 26,515 fixtures, 4 facets, 73,807 passed, 513 xfailed, **0 failed** | `epoch06c-gate-fixtures.log` |
| Tagged `term-hierarchy-epoch` facet | 129 fixtures, 135 passed, 0 failed | `epoch06c-gate-tagged-facet.log` |
| Frozen check set (tagged facet, syntax only) | 129 fixtures, 129 passed, 0 failed | `epoch06c-gate-frozen-facet.log` |
| Expensive contracts, all targets, release | 2,333 passed, 0 failed | `epoch06c-gate-expensive.log` |
| `semantics-coverage` | checked 22,659, panics 0, unsupported 0 | `epoch06c-gate-coverage.log` |
| Debug `jbotci` build | green | `epoch06c-gate-debug-jbotci.log` |
| Debug `dx build` | green | `epoch06c-gate-dx.log` |
| `maturin develop` + the four generated checks | all green | `epoch06c-maturin-develop.log` |
| Level-inventory unit test | 7 tests, green | `epoch06c-gate-inventory-test.log` |
| Comparer | 44 changed / 43 + 1 + 0 + 0 + 0 + 0 + 0 mechanical / 0 manual, prose 0, witness re-pins 0, epoch-new 36 | `epoch06c-gate-comparer.log` |
| Peak RSS, full profile | base 5,770,904 KB → 5,818,608 KB, **+0.83%** (gate +20%) | `epoch06c-base-fixtures.log`, `epoch06c-gate-fixtures.log` |

The peak-RSS base is measured rather than carried over: the epoch base
`2397912147` is checked out in its own worktree, built with its own target
directory, and run through the same one-volume `fixture-test --profile all`
under `/usr/bin/time -v`. Its fixture count is the pre-epoch 26,479.

`semantics-coverage` reports 156 `other-error` fixtures, which is where the
connectorless BO sumti connection lands: it is a principled
`semantic interpretation is undefined for …` refusal rather than the
`does not yet support` class the ratchet counts, exactly as the JAI tag term has
been since it was introduced. Nothing about a connective-less joint says which
connective it means, so the builder reports instead of guessing.

The artifact-size audit is not in this table: the owner retired the per-platform
size ratchets on 2026-08-16, and epoch 6b's ledger records the replacement — one
absolute 95 MiB per-file tripwire, the entry-count and member checks, and a
comparison band that is audit methodology rather than a gate. This epoch adds no
budgets and recalibrates none.

## CI at the round-1 head

All 22 checks pass at `d472c7b17a`, the head this epoch first submitted: `Cargo
tests`, `Generated syntax and stubs`, the five wheel builds, the
source-distribution round trip, `Python artifact acceptance`, the ten
per-interpreter wheel tests, `CLI release tooling` and the F2LLM goldens. The
wheel legs pass under the no-ratchet policy, with only the absolute 95 MiB
tripwire and the shape checks in force.

## Round 2

The round-1 review returned a formal PASS, and the lead directed a second round
on the code review's findings anyway. The round-2 delta is the diagnostics pins
and the check behind them, the two sumti-tier configuration rows, three comment
corrections, and this section; no grammar rule, no classifier and no expectation
tree moves.

| Change | Where |
| --- | --- |
| Exact `diagnostics` on the 23 success witnesses that omitted them | `tests/fixtures/adhoc/syntax/terms/zantufa-*.toml` |
| Fail-closed completeness check + 4 cases | `tools/compare-term-hierarchy-expectations.py`, `tools/tests/test_compare_term_hierarchy_expectations.py` |
| Sumti-tier configuration rows 2 and 5 | `zantufa-bo-sumti-connectorless-{no-features,both-axes}.toml` |
| Comparer prose named the 6a archive while its constants named the 6b one | `tools/compare-term-hierarchy-expectations.py`, its unit test |
| The `zantufa_bound_sumti_tail` comment stated the opposite of the classifier's ACKed disposition | `crates/jbotci-syntax/src/grammar/generated.rs` |

That last one is worth its own line, because the correction is the whole point of
measuring: the comment said `ConnectivePresentSumtiBoRejection` "returns that
extent to the sourced owner", and it does not. The classifier rejects nothing —
every `sourced_owner_takes_*` function carries `#[ensures(!ret)]` — because the
measurement above found that returning the nested sourced tail would not hand it
to the sourced owner at all, only push the surface onto the term tier and change
what it means. `baseline_bo.rs` and this ledger were already right; the DSL
comment was not, and it is the copy that reaches readers through the generated
Python model, which is regenerated with it.

| Gate | Result | Log |
| --- | --- | --- |
| `cargo fmt --all --check` | clean | `epoch06c-r2-fmt.log` |
| Frozen check set (tagged facet, syntax only) | 131 fixtures, 131 passed, 0 failed | `epoch06c-r2-frozen-facet.log` |
| Tagged `term-hierarchy-epoch` facet, all facets | 131 fixtures, 137 passed, 0 failed | `epoch06c-r2-tagged-facet.log` |
| Level-inventory + witness-pin unit tests | 11 tests, green | `epoch06c-r2-inventory-test.log` |
| Comparer | 44 changed / 43 + 1 + 0 + 0 + 0 + 0 + 0 mechanical / 0 manual, prose 0, witness re-pins 0, epoch-new 38, unpinned 0 | `epoch06c-r2-comparer.log` |
| `maturin develop` + the four generated checks | all green | `epoch06c-r2-maturin.log` |

The heavy suites in the round-1 table are not re-run and do not need to be: the
only Rust source this round touches is a doc comment, the fixture deltas are
additive `diagnostics` leaves plus two new adhoc witnesses, and the comparer and
its test are not in any of them. The comparer is re-run because its epoch-new
count moved, and the generated checks because the corrected comment is copied
into the Python model.
