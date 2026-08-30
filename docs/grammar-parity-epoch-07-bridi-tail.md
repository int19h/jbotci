# Grammar parity epoch 7: the bridi tail

Epoch 7 is the bridi-tail slice of the grammar-parity epic and carries three
issues at once, because they are three views of the same six productions:

| Section | Issue | State |
| --- | --- | --- |
| Zantufa forethought reconcile audit | #826 | complete, no grammar change |
| Zantufa JOIK/tag tail continuations and the KE ownership guard | #826 | complete |
| Standard tail boundaries | #805 | complete |
| camxes-exp CU and term prefixes | #815 | complete |
| Consolidated expectations, comparer, peak RSS | — | complete |

The design is `~/git/grammar-review/reports/implementation/epoch-07-bridi-tail/plan.md`
v4-final, with its frozen owner decision function R1-R4 and the 22-row
representative-cell table. The implementation base is `48ad77a06c`, the #862
merge.

## The ladder was already there

The plan calls D1 a re-leveling and freezes a same-name comparer table for it.
Resolved by name against the base, that table describes the tree jbotci already
has: the four upstream levels exist under exactly those names, each with its
tail-terms-free mirror.

| upstream (camxes.peg:76-79) | jbotci production |
| --- | --- |
| `bridi_tail` (the gihek-stag-KE join) | `bridi_tail` sum → `bridi_tail_with_possible_tail_terms`, `.ke_continuation` |
| `bridi_tail_1` (the flat gihek chain) | `afterthought_bridi_tail`, a chain of `bridi_tail_continuation` |
| `bridi_tail_2` (the gihek-stag-BO joint) | `bo_grouped_bridi_tail`, `.bo_continuation` |
| `bridi_tail_3` (selbri tail-terms / gek) | `simple_bridi_tail` sum |

So no production in the same-name table is renamed, re-parented or re-leveled by
this epoch. What the three issues actually ask for is inventory surgery on those
existing nodes: D1 narrows what a joint may carry, D2 adds the camxes-exp CU and
term prefixes, D3 adds the rolling-Zantufa joints. The comparer's same-name rows
are therefore 1:1 by construction, and the mechanical classes this epoch needs
are field- and arm-level, not path-level.

## The C6 forethought reconcile audit

The plan sequences the forethought reconcile before any tail work, so that the
audit is not diffing against nodes the later commits have moved. Run at the base
against the C6 table, resolving every row by production name:

| rolling-Zantufa feature (`gek_bridi_tail`, zantufa-1.9999.peg:24-25) | jbotci production | status |
| --- | --- | --- |
| n-ary GIK branches | `direct_forethought_bridi_connection.additional_branches`, elements `zantufa_forethought_bridi_branch` | RETAINED, both families |
| GIhI terminator | `direct_forethought_bridi_connection.gihi`, gated `ZantufaConnectives`, warned `ExperimentalZantufaForethoughtGihi` | RETAINED, both families |
| `tag* KE` grouping | `grouped_forethought_bridi_connection.tense_modals`, a `zero_or_more` of `standard_forethought_tense_modal` | RETAINED, both families, and already the epoch-6a `tag*` |
| NA prenegation | `negated_forethought_bridi_connection` — the arm name the plan asked to verify — and `negated_forethought_bridi_connection_without_tail_terms` | RETAINED, both families, no shape change |
| operand width: Zantufa joins `bridi_tail`, jbotci joins `subbridi` | `.first`, `forethought_bridi_branch.branch`, `zantufa_forethought_bridi_branch.branch` | INTENTIONALLY DIFFERENT (ZI-10) |
| post-branch `!(gik (term / CU))` guard | not modelled | OUT OF SCOPE, rides the ZI-10 width difference |
| tail terms and VAU after the connection | `.tail_terms` and `.vau` on the with-tail-terms family; `.vau` alone on the without-tail-terms one | RETAINED |

The audit's finding is that the reconcile is a no-op on the grammar: every row is
either already present in both node families or already adjudicated. Epoch 7
therefore rewires no forethought reference, and the sequencing constraint the row
existed to protect — do not re-level under a pending audit — is satisfied
vacuously, because the levels do not move.

The ZI-10 width row is the one that needs witnesses rather than a citation: with
`subbridi` operands retained, a Zantufa additional branch may be led by a term or
by CU where the source would have stopped at `bridi_tail`. Those strict witnesses
are in the epoch's witness set.

## D3-1: the unbound top continuation (#826)

Rolling Zantufa's tail is `bridi_tail <- bridi_tail_1 (joik_gihek tag?
CU_elidible bridi_tail_1)*` (zantufa-1.9999.peg:20). The continuation is unbound,
its tag slot is what #826 names, and neither camxes-standard nor camxes-exp has
anything at that position: standard's top level is the gihek-stag-KE join and
nothing else.

It lands as `zantufa_continued_bridi_tail` and its tail-terms-free mirror, each a
non-empty list of `zantufa_tail_continuation` over a leading `afterthought_bridi_tail`.
Requiring at least one continuation is what makes the arm structurally distinct
from a bare flat tail; it does not make it *disjoint*, because a GIhA-led
continuation covers the same extent the flat chain does, which is what the
classifier below settles.

### Why it must run first, and what that costs

The arm's own operand is the flat level. If the flat chain ran first it would
succeed on the shorter extent and the longer JOIK-led or tag-bearing continuation
would never be reached — the classic extension-hidden-by-a-prefix trap epoch 5
recorded. So the arm is extension-first, wrapped in
`zantufa_priority_continued_bridi_tail`, and the price of running first is paid
in `crate::grammar::baseline_bridi_tail`: the completed candidate is returned to
its baseline or adopted-exp owner when every one of its continuations is a GIhA
carrying no tag. That shape is the sourced flat joint when its CU is absent, and
the adopted camxes-exp reading — the sourced joint over an operand carrying
camxes-exp's own leading `bridi_tail_2` CU — when it is present.

A single JOIK/JEK connective or a single tag anywhere in the list keeps the whole
candidate here, because no sourced or adopted arm spells either at this joint.
The list is judged whole rather than element by element because the arm is one
node: splitting a mixed candidate between two owners would change what the tree
says the sentence is.

## D3-2: the inner joints (#826)

Zantufa also widens the two inner joints:

```
bridi_tail_1 <- bridi_tail_2 (joik_gihek !(tag? BO) !(tag? KE) CU_elidible bridi_tail_2 tail_terms)*
bridi_tail_2 <- bridi_tail_3 ((tag / joik_gihek tag?) BO_clause CU_elidible bridi_tail_3 tail_terms)*
```

Read as deltas over what jbotci already has, both are *connective* widenings on
joints that exist, not joints of their own, and that is how they land:

- `bridi_tail_connective` gains `joik_connective` and `jek_connective` as
  `ZantufaConnectives`-gated arms. That one edit widens the flat joint and the BO
  joint together, which is exactly the source's own shape — one joint, a wider
  connective — and it leaves every existing expectation untouched, because a
  GIhA-led joint still selects the `GihekConnective` arm it selects today. It
  does **not** reach the top KE join, and should not: rolling Zantufa spells no
  KE join at that level, its KE-led tail being a `bridi_tail_3` alternative
  jbotci already carries as `zantufa_grouped_bridi_tail`. D1 makes that explicit
  by giving both families the GIhA-only join.
- The BO joint's other half, Zantufa's connectiveless `tag BO`, has no sourced
  counterpart at all, so it needs an arm: `zantufa_tag_bo_bridi_tail_continuation`,
  the second arm of the new `bridi_tail_bo_joint` sum. The two arms are
  structurally disjoint — the sourced one requires a connective, the Zantufa one
  forbids it — so arm order cannot change which node a sourced surface gets, and
  no classifier is needed.

The flat joint deliberately does **not** become a sum. It cannot: it is a chain
link, and the chain's element type is resolved through a named field of the link
production, so a sum link has no inferable element type. The widened connective
makes the sum unnecessary anyway.

The Zantufa arms are ordered ahead of `relation_connective_as_bridi_tail`, the
unsourced non-GIhA route D1 deletes, so that a JOIK-led joint selects the arm
that survives this epoch rather than the one that does not. A GIhA-led joint
still selects `gihek_connective`, which is what makes the widening churn no
expectation.

### What the widened connective costs downstream

Semantics reads the widened node through the same six answers it already gives
for the shared relation connective — operator, source, SE, both negations and the
truth table — because rolling Zantufa's `joik_gihek` contributes exactly the JOIK
and JEK payloads that node already holds. That is the borrowing epoch 5's selbri
family does through `relation_afterthought_connective_from_selbri`, and it is why
the widening adds no second copy of a connective reading.

The connectiveless `tag BO` arm is the one shape with nothing to borrow: it joins
two tails with no connective at all, and no source says what the result claims.
It is therefore reported rather than invented — `undefined_semantics`, the
disposition epoch 6c gave rolling Zantufa's connectorless BO sumti connection for
the same reason. Both BO-joint arms are read through one borrowed view,
`GeneratedBridiTailBoJointRef`, so every structural pass sees one shape and only
the lowering that needs a connective has to ask for one.

### What the level split costs, and the one shape it changes

Zantufa spells a JOIK-led tail at both the flat level and the top continuation;
jbotci's flat chain is greedy, so with a JOIK arm at both levels the inner one
always wins and the top continuation would fire only where the inner one cannot
reach — which is exactly the tag-bearing surfaces. That is the split this epoch
ships: the flat joint takes the bare JOIK connective, the top continuation takes
everything the flat joint would have to borrow a camxes-exp operand for. The
measured consequence is the `mi broda je pu cu brode` class, where the inner
joint would otherwise pair a Zantufa connective with a camxes-exp prefix in its
operand and warn twice for one construct; the D2 commit adds the classifier that
returns those candidates to the top continuation, whose own tag and CU slots
absorb them, so the surface reads as one Zantufa construct and warns once. The
witnesses measure it: `je cu brode` and `je pu cu brode` each carry exactly one
`ExperimentalZantufaTailContinuation` and no camxes-exp warning at all.

## D3-3: the KE ownership guard (#826)

Zantufa's KE-led tail is `bridi_tail_3 <- KE !(selbri_2 KEhE) bridi_tail KEhE?
tail_terms / …` (:23). jbotci has carried the tail since epoch 6 as
`zantufa_grouped_bridi_tail`, gated `ZantufaTerms`, and has carried it *without*
the guard: measured at the base under `--dialect zantufa`, all five of the
boundary surfaces below select it, including the three that belong to a baseline
group.

The guard cannot be the source's token lookahead — policy forbids one, and the
level it names is reachable through the tail itself, so no token prefix
distinguishes the readings. It is a completed-candidate classifier instead,
`GroupedTanruKeTailRejection`, asking the question the lookahead is really asking:
could this identical extent be a baseline group?

| boundary class | surface | std / exp / Zantufa | adopted owner |
| --- | --- | --- | --- |
| plain adjacency | `mi ke broda brode ke'e` | A / A / A | BASELINE `grouped_tanru_unit` (R1) |
| CO-bearing | `mi ke broda co brode ke'e` | R / R / A | epoch 5's `zantufa_ke_co_grouped_tanru_unit` |
| GE…GI… | `mi ke ge broda gi brode ke'e` | A / A / A | BASELINE `grouped_forethought_bridi_connection` (R1) |
| with a tail term | `mi ke broda do ke'e` | R / R / A | the Zantufa KE tail |
| with a GIhA connection | `mi ke broda gi'e brode ke'e` | R / R / A | the Zantufa KE tail |
| missing KEhE | `mi ke broda brode` | A / A / A | BASELINE (R1), though Zantufa's own guard hands it to the tail |

The last row is why the classifier is candidate-wide rather than keyed on the
KEhE token: Zantufa's lookahead passes when no KEhE follows, so the source reads
`mi ke broda brode` as a KE tail, while camxes-standard reads it as a grouped
tanru with an elided KEhE over the identical extent. R1 keeps it baseline.

The trailing-terms condition differs between the two baseline shapes because
their owners differ in where terms may attach: the selbri arm carries
`tail_terms` of its own, so `ke broda brode ke'e do` is still a grouped tanru
followed by the selbri tail's own term, while `grouped_forethought_bridi_connection`
has no place for a term after its KEhE, so `ke ge broda gi brode ke'e do` stays
the Zantufa tail's.

## Warnings

The three new joints own no token that is theirs alone: the connective is the
shared GIhA/JOI/JA spelling, the tag is the shared tag machinery, and the BO, the
CU and the top continuation's tag are all either sourced words or optional. Each
is therefore diagnosed post-parse by the standing
`GeneratedConstructWarningVisitor`, anchored at its own first token, under one
fixed category, `ExperimentalZantufaTailContinuation`
(`syntax.warning.experimental-zantufa-tail-continuation`).

Because the widening is on the shared connective rather than on joints of their
own, the sourced flat, BO and KE joints are diagnosed the same way whenever the
connective they selected is one of the arms Zantufa contributes. The top
continuation is deliberately left out of that list: it warns once for its whole
continuation list at the node above, and warning at its elements as well would
count one construct twice.

The unbound top continuation warns **once for the whole construct**, anchored at
the first continuation's connective, rather than once per element: the repeated
group is one node and one ownership decision. That is the plan's warning matrix
rule — one warning per maximal construct, repeated groups inside one construct
counting once — and nested constructs still warn once each at their own anchors,
because each is its own node.

## D1: the standard boundaries (#805)

camxes-standard's tail inventory is GIhA and nothing else — the flat chain, the
BO joint and the top KE join all spell `gihek` (camxes.peg:76-79) — and its only
statement join is an I (camxes.peg:20-22). jbotci carried four routes past those
boundaries that no source spells. D1 deletes all four.

| deleted route | what it admitted | what happens now |
| --- | --- | --- |
| `relation_connective_as_bridi_tail` | JOIK, JEK, EK and VUhU at every tail joint | GIhA plus the Zantufa arms D3 gates; `ganse su'i zukte nirna` and `ganse ji zukte nirna` reject |
| `bo_bridi_statement_continuation` | an I-less BO envelope over a full subbridi | the sourced tail BO joint and the I-led statement BO envelope keep every surface with an owner |
| `ke_bridi_statement_continuation` | an I-less KE envelope over a full subbridi | those surfaces reject |
| `bridi_tail_ke_continuation` | the KE join under a widened connective | both families take the GIhA-only `gihek_bridi_tail_ke_continuation` |
| the joints' own `cu` fields | a CU between a tail connective and its right operand | camxes-exp's CU is the operand's own leading `bridi_tail_2` CU, which D2 adds |

The KE row is the one worth stating plainly, because C-b's own note claimed the
widened connective reached the KE join: it does not, and it should not. Rolling
Zantufa spells no KE join at the top level at all. Its KE-led tail is a
`bridi_tail_3` alternative, which jbotci already carries separately as
`zantufa_grouped_bridi_tail`, so the KE join is GIhA's alone in every dialect.

Two consumers of the legacy shared `relation_afterthought_connective` die with
those routes, and the third — the JAI mini-ladder inside a selbri — is a selbri
connection rather than a bridi-tail one, so it converts to the selbri family's
JOIK/JEK inventory. With no consumer left, the node retires. Its reader family
retires with it into `GeneratedJoikJekConnectiveRef`, a borrowed view over the
two payloads every remaining tier spells, which also drops the per-connection
clone the selbri tier was paying to reach it.

`bridi_statement` keeps its name and its one child, and the macro renders a
single-child product transparently, so the node becomes `BridiStatementSyntax(bridi)`.
That is a rendering change in every fixture that has a statement, with no owner,
warning or cardinality change anywhere; it is the epoch's one bulk mechanical
class outside the frozen same-name table.

## D2: the camxes-exp CU and term prefixes (#815)

camxes-exp puts a CU in two places the standard grammar does not: at the head of
`bridi_tail_2` (`CU_elidible? free* bridi_tail_3 …`, camxes-exp.peg:107) and at
the end of each group of a repeated `(terms CU_elidible?)*` run in front of the
`bridi_tail_3` selbri (camxes-exp.peg:108). Both land where the source puts
them, which is what makes them available in the flat, BO and KE operands at once
without a single reference being rewired: the operands already *are* those
levels.

- `bo_grouped_bridi_tail` and its mirror gain the leading-CU field. Every adopted
  CU after a tail connective is this one — D1 deleted the joints' own CU slots,
  so `gi'e cu brode` is the sourced joint over an operand that opens with a CU,
  which is exactly the reading camxes-exp gives it.
- `exp_prefixed_simple_bridi_tail` and its mirror are the repeated-group arm, a
  non-empty run of `exp_tail_terms_prefix` before a `selbri_simple_bridi_tail`.
  The repetition belongs to the selbri alternative alone, never to the GEK one,
  which is the C2 finding the plan carries.
- The arm is **last** in the `bridi_tail_3` sum, which is where R1 is enforced
  for it: the sourced arms decide every extent they can cover. `gi'e pu brode` is
  a tagged selbri, reached by the selbri arm over the identical extent; only
  `gi'e pu cu brode`, where no tagged selbri can be built, falls through to the
  prefix. The four per-position classifiers the plan asks for reduce to that one
  ordering rule at the outer, flat, BO and KE positions, because the prefix arm
  cannot cover an extent a sourced arm covers without the sourced arm having been
  tried there first.
- The one place ordering is not enough is the flat joint under a Zantufa
  connective, and that is the classifier this commit adds:
  `ExpPrefixUnderZantufaConnectiveRejection` rejects a flat link whose connective
  is JOIK or JEK and whose operand opens with a camxes-exp CU or prefix. The
  candidate falls back to the unbound top continuation, whose own tag and CU
  slots absorb it, so `mi broda je cu brode` and `mi broda je pu cu brode` are
  one Zantufa node warning once rather than a Zantufa joint over an exp operand
  warning twice. A prefix that the top continuation cannot absorb — `je do cu
  brode`, whose `do` is a term and not a tag — stays a nested construct and warns
  once for each, which is the warning matrix's own rule for nesting.

The old single-outer-group shape retires into this one. `bridi_with_post_cu_terms`,
`bare_cu_terms_bridi` and their shared `cu_terms_bridi_tail` modelled exactly one
extra group, at the outer level, where camxes-exp has none: its second group is
the tail's. Retiring them also retires a lowering refusal —
`ca le nu mi klama le mi zdani cu mi tirna ra vau do` could not combine post-CU
terms with a statement-level suffix term and reported undefined semantics; under
the prefix shape it lowers, and the pin that recorded the refusal now records the
lowering.

## The consolidated regeneration

The epoch's shape moves are all field- and arm-level inside productions that keep
their names, so the comparer's classes are too. `tools/compare-bridi-tail-expectations.py`
is the fail-closed classifier: it reads the *baseline* Rust-Debug tree from a
`git archive` of `48ad77a06c`, applies exactly the five approved shapes, and then
requires the rewritten baseline tree to equal the regenerated one structurally.
Nothing is inferred from the new tree, so an ownership move can never be laundered
as a re-typing — every shape the rewrites do not produce is manual residue with
its own ledger disposition.

| class | what moves | fail-closed on |
| --- | --- | --- |
| `statement-continuation-collapse` | `BridiStatementSyntax { bridi, continuations: [] }` → `BridiStatementSyntax(bridi)` | the list must be EMPTY and the payload byte-identical; a non-empty list is a D1 flip and has no tree for the collapse to produce |
| `bt2-leading-cu-field` | `bo_grouped_bridi_tail` and its mirror gain `cu: None` at the head | the field is inserted, never matched: a regenerated `cu` the baseline could not have parsed diverges |
| `tail-joint-cu-drop` | the flat and BO joints lose their own `cu` field | only where the baseline value is `None`; a baseline tree that actually parsed a joint CU is a D1 flip |
| `bo-joint-sum-wrapper` | the sourced BO product gains its arm of the new two-arm joint sum | only the sourced product wraps; the Zantufa arm cannot appear on the baseline side |
| `ke-join-gihek-narrowing` | `BridiTailKeContinuation` → `GihekBridiTailKeContinuation` | only where the baseline selected the `GihekConnective` arm |

Three of the five are the shapes epoch 6c's own table blessed one tier down — a
field insert, a sum wrap and a narrowing — and the first is the class the lead
ACKed by name before the work began.

The classifier also refuses outright any regenerated tree that still contains a
node this epoch retires: `BridiStatementContinuation`, its two arms,
`RelationConnectiveAsBridiTail`, `RelationAfterthoughtConnective`,
`BridiTailKeContinuationSyntax`, `BridiWithPostCuTerms`, `BareCuTermsBridi` and
`CuTermsBridiTail`. A surviving instance would mean a population moved without
being dispositioned.

## The flips, and what they were

Every D1 deletion takes a population with it, and each one retires its old
expectation in the same commit as the code that moved it. The flipped surfaces
fall into three families, all jbotci-only readings with no source behind them:

- **Non-GIhA tail connectives.** `mi klama do je tavla ti` (JACU, I03),
  `mi prami do je cu djica lo nu do gleki`, the broad-A relation connective, and
  epoch 5's two `issue-840` residuals — a JEK-tag-KE join and a VUhU join — all
  reached a bridi-tail joint through `relation_connective_as_bridi_tail`. The
  epoch's own reject witnesses record the boundary that moved, and
  `ganse je zukte nirna` is pinned green beside them so the pin says *which*
  boundary rather than merely that something rejects now.
- **The I-less statement envelopes.** Their BO surfaces keep an owner — the
  sourced tail BO joint and the I-led statement BO envelope both already exist —
  and only their KE surfaces flip to reject.
- **The joints' own CU.** A CU between a tail connective and its right operand is
  now the operand's own leading `bridi_tail_2` CU, so the surfaces that parsed
  keep parsing and only the tree shape moves; that is the mechanical class, not a
  flip.

Two Rust unit pins retire with the first family and are replaced in the same
commit: `ganse su'i zukte nirna` and `ganse ji zukte nirna` become a rejection
pin, and the post-CU-terms lowering refusal becomes a lowering.

Thirty-six fixtures flip to rejection and six flip to acceptance. Every one of
the thirty-six pins its new frontier exactly: six carry a `syntax.diagnostics`
list, and the other thirty pin the byte and the expected-set through
`semantics.refs.error`, which is the same frontier read off the same parse.
Thirty-four of them also carried a Gentufa rendering of the tree they no longer
have; a rendering of a parse that no longer succeeds is an expectation the
writer can neither verify nor rebuild, so the block is deleted on exactly that
set and nowhere else.

### `corpus/alis/full-alice` and the one site that moved it

The long text flips on a single site. Under recovery the whole 152 KB yields
exactly one error, at `lo du'u py purdykurji ji kau sonci ji kau nolkansa ji kau
panzi be ny ci mei` (line 1408): an EK reaching a bridi-tail joint through
`relation_connective_as_bridi_tail`, the first flip family above. The camxes
corpus rejects that shape in its own right — entry `4148`,
`do prami ji gletu le do tanbo`, carries the corpus verdict `failure` — so
jbotci rejecting it is the boundary this epoch is for, not a regression against
the source. The cost is real and is recorded rather than softened: the text's
syntax tree, its Gentufa renderings and its Tersmu output all retire with it,
and the fixture now pins the rejection and its frontier. Repairing the
translation would keep the coverage, and the fixture's own provenance
(`alis-visible-diagnostic-repaired31`) shows the corpus has been repaired
before, but a vendored translation is not this epoch's to edit. The lead filed
that repair as **#866**: a one-site edit at line 1408 which restores the text's
syntax tree, its Gentufa renderings and its Tersmu output in full. The gap this
epoch books is therefore TEMPORARY, and the fixture carries the same reference in
its own frontier comment; the repaired form is settled against the CLL in #866
rather than asserted here.

## The camxes corpus verdict, and what the writer did to it

`expectations.syntax.status` on a `corpus/camxes` fixture is the *corpus's*
verdict, not jbotci's; `xfail` records jbotci's divergence from it. The
import-time fixtures settle this: `corpus/camxes/11224` was imported carrying
`status = "failure"` beside camxes's own `error = { position = 8 }`
(`0c9003162b`), and `corpus/camxes/1003` carries a v0 parse tree under
`status = "failure"` because camxes rejects the bracketed `[ku]` that jbotci
reads as an elidable marker.

That matters here because `fixture-rewrite` rewrites `status` from jbotci for
every fixture that does not already carry an `xfail`. On an acceptance flip in
either direction it therefore overwrites the corpus verdict instead of recording
the divergence, and the two directions this epoch produces need opposite
treatment.

- **Twenty-six fixtures flip to rejection, and the writer's output is right.**
  Their corpus verdict was `failure` all along: each was imported with an
  `xfail`, and both were dropped in May 2026 by `25e8e7d2ad`
  (*Remove corpus xfails for passing syntax cases*) and `bcbbcb40cb`
  (*Mark passing corpus syntax cases as success*), which reset the verdict to
  what the parser then did. Every one of the twenty-six is an EK, JEK or VUhU
  reaching a tail joint, which is exactly what the corpus rejected. The epoch
  restores agreement on all of them and no `xfail` is needed.
- **Six fixtures flip to acceptance, and the writer's output is wrong.**
  `11224`, `18634`, `21872`, `22256`, `5921` and `9245` are camxes-exp
  `(terms CU?)*` prefixes after a GIhA connector — `gi'anai py. xusra`,
  `gi'e baziku selcatra` and their kin — which #815 adopts with
  `ExperimentalCuTermsSelbri` and camxes-std rejects. The corpus verdict is
  restored to `failure` and the divergence pinned with
  `accepted-status = "success"`, which is also what makes the epoch's adoption
  visible in the corpus rather than silently absorbed. Each gains the accepted
  tree the runner requires at that status, so the xfail count moves 513 → 519 —
  the first epoch of this arc to add one rather than retire one.

## The xfail splice

`regenerate_syntax_fixture` refuses any fixture carrying
`expectations.syntax.xfail`, for the reason above: a writer that rebuilt those
would erase the corpus verdict. 421 of the 513 xfail fixtures fail their syntax
facet after this epoch and must be rebuilt anyway, so the population is
regenerated the way epochs 6/6b/6c did it — each fixture stripped of its `xfail`
line, rebuilt by the project's own writer, and given its original `status` and
`xfail` lines back. Two things fail the splice closed: the rebuilt accepted
status is compared against `xfail.accepted-status` for every fixture, and the
spliced document must equal the writer's own document in every value except the
two the splice restores. All 421 matched their pin, so no xfail retires or
inverts in this population, and each moved exactly one line — its `raw` tree.

The remaining 92 xfail fixtures need nothing: their syntax facet still passes,
which is the writer's own gate for attempting a rebuild at all.

## The manual residue, dispositioned

The comparer classifies 19,769 changed pre-epoch fixtures. 73 are manual
residue, in six populations, none of them a class:

| population | count | disposition |
| --- | --- | --- |
| acceptance flips (36) plus the `bare_cu` warning pin | 37 | The flip families above; each retires its tree, its Gentufa block and its accepted `semantics.refs` in the same commit as the code that moved it. `adhoc/v0/warnings/experimental/bare-cu.toml` is the C-e resolution's own fixture: `cu klama` now warns, so the fixture named for the surface gains the `diagnostics` list it never carried. |
| rejection-frontier drift | 24 | Fixtures that rejected before and reject now, whose error diagnostic moved. Nine go `syntax.incomplete-bridi` → `syntax.incomplete-statement` at the identical byte, which is the retired `"bridi continuation"` construct-metadata row showing through: with no statement-level continuation to attribute to, the incomplete frontier is the statement's. The rest exchange one unexpected-token code for another as the expected-set at the frontier changes; only `corpus/camxes/19938` (`stone age fuckers`, not Lojban) moves its byte, reporting at the `ge` inside `age` instead of at `fuckers`. No acceptance moves in the population. |
| the six new corpus xfails | 6 | Above. |
| the retired single-outer-group exp family | 4 | `cll/chrestomathy/alice01`, `corpus/camxes/22422`, `5970`, `6029`: `BridiWithPostCuTerms` becomes `BridiWithLeadingTerms` as the outer CU returns to the baseline arm and the post-CU terms become the tail's own `(terms CU?)*` prefix. The plan names this family's population a dedicated manual class. All three inline fixtures keep one `ExperimentalCuTermsSelbri`, re-anchored from the `cu` to the prefix's first token (`ti`, `mi`, `mi`) — the C8 rule that a warning sits on its construct's first token, not on the token that used to own the node. |
| the I-less statement BO envelope | 1 | `corpus/camxes/20846`, `… la lojban gi'ebo le sarji pe do sarcu`: the only fixture whose baseline tree used `BoBridiStatementContinuation`. It keeps parsing — the sourced tail BO joint takes it, and the new tree carries `BridiTailBoContinuation` — so this is the ownership move D1 predicted for the family's BO surfaces, not a flip. |
| a recovered-tree leaf | 1 | `adhoc/recovery/syntax/dialect-gated-anchor.toml`: recovered trees are outside every mechanical class by construction, so its recovered `diagnostics` leaf is listed rather than classified. |

`ke-join-gihek-narrowing` classifies nothing, and the zero is a measurement
rather than a gap: exactly one baseline tree in the whole corpus reaches the
tail-terms-free family's KE join, `issue-840-jek-tag-ke-bridi-tail-residual`,
and its join is JEK-led — an acceptance flip, which the class refuses by design.
The narrowing itself is covered by `baseline-gihek-ke-join` and by the
retired-shape invariant, which finds no `BridiTailKeContinuationSyntax`
anywhere in the regenerated tree.

That invariant needed a fix before it could say so. It matched retired names as
substrings, and `GihekBridiTailKeContinuationSyntax` — the node the narrowing
*produces* — ends with the retired `BridiTailKeContinuationSyntax`, so nine
fixtures were reported as carrying a node this epoch deletes while the class
that produces it was skipped for the same nine. The check now matches whole
identifiers in both renderings the macro emits, `Name(` and `NameSyntax {`, and
`tools/tests/test_compare_bridi_tail_expectations.py` pins both sides of it
along with every transcribed field tuple, re-derived from the DSL at HEAD and at
the archive commit rather than asserted.

## The diagnostic pins

Eleven tests across four packages pin rendered diagnostic text, a reviewed
diagnostic set, a witness source or a tree marker, and every one of their inputs
sits on this epoch's boundary. They are pins, not fixtures, so no comparer class
reaches them; each is dispositioned by hand against a base binary built from
`48ad77a06c`.

They did not surface together. `cargo test` abandons the run at the first failing
target, so the gate reported only `-p jbotci --test cli` — and each fix uncovered
the next target behind it. The population was closed with a `--no-fail-fast` pass
rather than by iterating, which is the lesson worth keeping: a red gate reports a
lower bound on the failing set, never the set.

The eight CLI pins reduce to one grammar move. D2 retires `bridi_with_post_cu_terms`,
`bare_cu_terms_bridi` and their shared `cu_terms_bridi_tail`, so the second outer
term group is no longer a field of `bridi`; it is `exp_tail_terms_prefix`, a field
of the tail's own `exp_prefixed_simple_bridi_tail`, whose sibling field is the
selbri. A frontier that used to sit inside `bridi`, one field short of a
`bridi_tail`, now sits inside `bridi tail`, one field short of a `selbri`.

| population | tests | pin | base → head | why |
| --- | --- | --- | --- | --- |
| `mi cu`, the outer post-CU frontier | `cli::gentufa_syntax_error_uses_explicit_diagnostic_width` | `apps/jbotci/tests/support/cli.rs` | `expected: free modifier, terms` → `expected: free modifier, space interval` | The umbrella `terms` label was the retired node's own field. With the group inside the tail, the frontier reports the tail's first set expanded — `space interval, time interval, space tense, time tense, place tag, tag` — and the capability is unchanged: `mi cu do cu broda` still parses, as the matrix's outer multi-group row requires. |
| | `cli::gentufa_detailed_syntax_errors_show_expectation_breakdown` | same | `[continues bridi]` → `[continues bridi tail]` | The construct being continued is the tail, not the bridi. The set widens by `{cu}` in the same move: the tail's own leading CU is D2's `bt_2` slot. |
| | `cli::gentufa_syntax_error_labels_unique_current_construct` | same | `syntax.incomplete-bridi` → `syntax.incomplete-statement` | Mechanical from the construct name: `bridi tail` is a `statement` descendant where `bridi` was its own kind. The same row as the nine fixtures under rejection-frontier drift, reached by a different retirement. |
| `mi ku i do ku i mi klama`, the term-connection frontier | `recovery_diagnostics::gentufa_renders_both_syntax_errors_exactly`, `…::gentufa_blocks_svg_renders_recovered_regions_without_changing_diagnostics`, `…::tersmu_renders_all_syntax_errors_without_semantic_output`, `…::max_errors_one_caps_both_recovery_phases_at_the_first_diagnostic`, `…::run_tool_gentufa_returns_partial_stdout_and_full_stderr_for_structural_formats` | `apps/jbotci/tests/support/recovery_diagnostics.rs`, the two shared constants `SYNTAX_EXPECTED_LABEL` and `SYNTAX_DETAILED_NOTE` | one entry added: `selbri (ZANTUFA-SELBRI-REINTERPRETATION feature)`, after `paragraph statement` | The five tests render from those two constants, so one insert answers all five. Leading terms now feed the tail's prefix repetition, whose sibling is the selbri, so the gated construct offered at a term frontier is a `selbri` where the base offered a `bridi tail` under `ZANTUFA-TERMS`. Probed base-against-head, the swap is exactly where the prefix reaches and nowhere else: `mi cu do ku` shows `bridi tail (ZANTUFA-TERMS)` → `selbri (ZANTUFA-SELBRI-REINTERPRETATION)`, while `mi klama mi ku` (a tail-terms frontier) and `lo broda be mi ku cu brode` (a frontier inside a `be` link) are byte-identical across the epoch. |
| the incremental-diagnostics sample gate | `jbotci_ide::snapshot::incremental_diagnostics::tests::fixture_sample_gate_passes_match_the_reviewed_set_and_imply_confirmation_equivalence` | `crates/jbotci-ide/src/snapshot/incremental_diagnostics.rs` | `nary-forethought-statement` joins the reviewed passing set | D1 deletes `bridi_statement_continuation` and its BO and KE arms. In the gate's own wrapper document the sample's trailing `gi'i` used to be an incomplete `bridi continuation` whose context label ran from inside the sample's paragraph across the following `ni'o`; a boundary-crossing diagnostic is exactly what the gate is conservative about. With the node gone the sample rejects with plain in-paragraph diagnostics and no local warning, so it joins for the same reason epoch 6b's `nary-gek-termset` and epoch 6c's five samples did. The pin carries that reasoning inline, as the rows above it do. |
| the smusni structural connective witness | `jbotci_semantics::smusni_structural::focused_semantic_families_are_exercised_without_output_goldens` | `crates/jbotci-semantics/tests/smusni_structural.rs` | `ganai broda gi brode .a brodi` → `ganai broda gi brode gi'a brodi` | Not a rendering change but a flipped surface: at the base the `.a` built an `AfterthoughtBridiTail`, the broad-A relation connective reaching a tail joint, which is the first flip family above. The witness is repaired rather than retired — the head tree under `gi'a` is the same shape with the sourced connector in the same joint, so the family stays exercised. Same treatment as the two `ganse …` unit pins. |
| the Zantufa parity shape marker | `jbotci_syntax::zantufa_parity::captured_zantufa_cases_match_parser_policy` | `tests/fixtures/zantufa/upstream-parity.json`, case `grouped-bridi-tail` | `ZantufaGroupedBridiTail` → `ZantufaPriorityGroupedBridiTail` | D3-3's KE ownership guard is a completed-candidate classifier, so `zantufa_grouped_bridi_tail` is now always reached through `zantufa_priority_grouped_bridi_tail`, which wraps it and carries the rejection. The inner rule is not renamed and its fields are untouched — the wrapper is transparent in the serialised tree, so the outer name is what a substring marker can see. Re-probed across all fifteen parity cases, this is the only marker the epoch moves. |

No pin needed a new rule to explain it, so none is a stop-and-ask.

## Gap ledger

| gap | why it stays open |
| --- | --- |
| rolling Zantufa's post-branch `!(gik (term / CU))` guard is not modelled | It rides the ZI-10 operand-width difference: jbotci joins `subbridi` where the source joins `bridi_tail`, so the guard has nothing to exclude at jbotci's width. The two `zi10-*` witnesses pin what the retained width admits — a term-led and a CU-led additional branch — so the difference is measured rather than assumed. |
| the plan's four per-position prefix classifiers are one ordering rule | At the outer, flat, BO and KE positions the camxes-exp prefix arm is last in its sum, so a sourced arm decides every extent it can cover before the prefix is tried, which is what R1 asks for. A classifier that can never fire would be worse than none: it would claim a boundary the grammar is already drawing. The one position where ordering is not enough — a Zantufa connective over a camxes-exp operand — does carry a classifier. |
| `je do cu brode` is a nested construct, not a rejection | The top continuation's slots are a tag and a CU, so a prefix group whose term is not a tag cannot be absorbed into one Zantufa node. The surface stays a Zantufa joint over a camxes-exp operand and warns once for each, which is the warning matrix's own rule for nesting rather than an exception to it. |
| `corpus/alis/full-alice` loses its syntax, Gentufa and Tersmu coverage | TEMPORARY, tracked by **#866**. The 152 KB text flips on exactly one site, and the flip is parity rather than regression — camxes rejects the same EK-over-tail shape in its own right (`corpus/camxes/4148`). Nothing in the grammar waits on it: the fixture pins the rejection and its frontier now, and the one-site text repair in #866 restores all three expectation blocks. Booked as a gap rather than softened, because the coverage is real and is genuinely absent until #866 lands. |
| the KE join is not widened by rolling Zantufa | Rolling Zantufa spells no KE join at the top level at all; its KE-led tail is a `bridi_tail_3` alternative, which jbotci carries separately. The C-b note that claimed otherwise is corrected in the D1 section. |

## The C-e stop-and-ask

C-e is fixture realization only: a surface that matches no rule of the decision
function is a stop-and-ask rather than a new classification. One surface hit it.

The matrix's outer row for `cu broda` (R|A|A) reads ADOPTED-EXP and calls it "the
existing `bare_cu_bridi` warn node — already adopted at base". Measured at the
base, `bare_cu_bridi` does not warn: its CU field carries no `.warn`, so jbotci
accepts a camxes-exp-only surface silently. R2 and the warning matrix both give
every non-baseline cell a warning, so the parenthetical is a belief about the
base rather than a measurement of it.

The resolution is to make the row true rather than to widen scope: the warning
goes on `bare_cu_bridi`'s CU, and `mi cu broda` is untouched because leading
terms send it to `bridi_with_leading_terms`, the baseline arm. Only the
leading-termless cell moves. Because that is a warning change on a pre-existing
node, every fixture in the affected population is a manual disposition and never
mechanical.

## The gate, and the peak-RSS number that was not one

| component | result | log |
| --- | --- | --- |
| `cargo fmt --all --check` | clean | `epoch07-s5g-fmt.log` |
| `cargo test -r --workspace --features jbotci-dictionary/import --no-fail-fast` | green | `epoch07-s5g-workspace.log` |
| `cargo test -r --workspace --all-targets --features expensive_contracts --no-fail-fast` | green | `epoch07-s5g-expensive.log` |
| `fixture-test --profile all` | 26,553 fixtures, 73,839 passed, 519 xfailed, 0 failed | `epoch07-s5-rss-head.log` |
| tagged facet `bridi-tail-epoch` | 36 fixtures, 36 passed | `epoch07-s5g-tagged-facet.log` |
| frozen syntax facet, same tag | 36 fixtures, 36 passed | `epoch07-s5g-frozen-facet.log` |
| `semantics-coverage` | checked 22,655, panic 0, unsupported 0, other-error 163 | `epoch07-s5g-coverage.log` |
| comparer | 19,769 changed / 19,286 + 19,695 + 456 + 33 + 0 mechanical / 73 manual | `epoch07-s5g-comparer.log` |
| comparer unit tests | 11 tests, green | `epoch07-s5g-comparer-test.log` |
| `cargo build -p jbotci` (debug) | green | `epoch07-s5g-debug-jbotci.log` |
| `dx build` | green | `epoch07-s5g-dx.log` |
| peak RSS, full profile | base 5,841,348 KB → 3,471,488 KB, **-40.6%** (gate: base +20%) | `epoch07-s5-rss-base.log`, `epoch07-s5-rss-head.log` |

Both test components run `--no-fail-fast` deliberately. `cargo test` abandons the
run at the first failing target, so a red gate reports a lower bound on the
failing set rather than the set; that is how this epoch's eleven boundary pins
came to surface one target at a time.

The peak-RSS pair is re-measured rather than carried over, because the number
first recorded for the base was not a measurement of the base. That run
(`epoch07-base-fixtures.log`, 9,704,892 KB) let `cargo run` compile inside the
timed window: it records 3,974,248 filesystem output blocks against 72 for a warm
run, 1,770s of user time against 472s, and the base `xtask-full` binary's mtime
falls five minutes into the measured interval. `/usr/bin/time -v` reports the
maximum resident set over the whole process tree, so what it had actually
measured was rustc.

Both sides are therefore re-measured identically: each tree checked out with its
own target directory, its `xtask-full` built *outside* the window, then the same
one-volume `/usr/bin/time -v cargo run -q -r -p xtask-full -- fixture-test
--profile all` at default parallelism, back to back, with nothing else on the
box. Warm, the base is 5,841,348 KB — in line with epoch 6c's 5,770,904 KB for
the same corpus, which is the sanity check the 9.7 GB figure failed. The head
number reproduces the pre-existing head measurement to within 0.14%
(3,466,632 KB), so it was the base and only the base that was wrong. The honest
reduction is **-40.6%**, not the -64% the contaminated pair implied. Wall clock
is flat within noise, 4:42 to 4:35 across a slightly larger corpus; no runtime
claim is made from it.

## Fixture counts

The tree moves 26,517 → 26,553 with this epoch's 36 witnesses. The xfail count
moves 513 → 519: no xfail retires or inverts, and the six added are the corpus
entries this epoch's D2 adoption newly accepts, dispositioned above. That is the
first increase in the arc — epochs 6 and 6b each retired one — and it is what
recording an adoption against a rejecting reference parser looks like.

## Round 2 — rebase onto the tersmu retirement

Everything above describes round 1, whose base was `48ad77a06c` and whose head
was `4582ad0b11`. While the pull request was open, `main` retired tersmu
(#869/#870), and the epoch was rebased onto that retirement. The base is now
`67cc7e4b5a`, the #870 merge. Nothing in the epoch's grammar, ownership matrix,
warnings, pins or dispositions changed; this section records the rebase and the
re-run gate, and the numbers above are re-verified against the new base rather
than restated on trust.

### Method and conflict classes

A plain `git rebase --onto 67cc7e4b5a 48ad77a06c` replayed all five commits and
preserved the C-b..C-f shape (`epoch07-r2-rebase.log`; the reflog records the
`rebase (finish) ... onto 67cc7e4b5a` entry). C-f is now `8ab6098449`.

Main's retirement touched `tests/fixtures` in exactly one way. It edits 1,342
fixtures and adds or deletes none, and every added or removed line across all
1,342 files mentions `tersmu` — that is, each edit is the removal of an
`[expectations.output.tersmu]` block and nothing else. Against that:

| class | count | resolution |
| --- | --- | --- |
| fixtures touched by the branch only | 18,587 | branch content replays unchanged |
| fixtures touched by both | 1,219 | branch content with the tersmu block removed |
| fixtures touched by main only | 123 | main verbatim |
| fixtures touched by neither | 6,635 | untouched |
| `jbotci-semantics` files the branch edited that main deleted | 10 | main's deletion taken |
| `jbotci-semantics` files the branch edited that main kept | 2 | branch edits ported |
| every other path | — | replayed clean |

The 10 deleted files are the nine `crates/jbotci-semantics/src/generated_builder/`
readers (`connectives.rs`, `formulas.rs`, `mod.rs`, `pro_bridi.rs`,
`relations.rs`, `statements.rs`, `tanru_property.rs`, `tense_modal.rs`,
`text_plan.rs`) plus `crates/jbotci-semantics/tests/smusni_structural.rs`. The
epoch's edits to all ten were mechanical follow-ons to the renamed bridi-tail
view constructors — they carried no decision of this epoch's own — so taking
main's deletion loses nothing. `references.rs` and `generated_term_view.rs`
survive the retirement, and the epoch's node-name rewires were ported onto
main's post-retirement versions of both.

Post-conditions, checked mechanically rather than assumed: the 26,564 files
under `tests/fixtures` account for 19,806 branch-touched files (resolved as
above) plus 6,758 files the branch never touched (main verbatim), with none
unexplained; and no `[expectations.output.tersmu]` block survives anywhere in
the tree. The three remaining case-insensitive `tersmu` hits in the tree are a
Lojban word (`midytersmu.` in `corpus/camxes/14768`), main's own test that the
retired facet names now fail to parse, and this epoch's documentation.

### What the rebase did not change

The comparer's baseline is git-derivable, so it moves with the base:
`ARCHIVE_COMMIT` and `EPOCH_BASE` are now `67cc7e4b5a`, and the docstring
records that the tersmu block is absent on *both* sides and is therefore out of
the comparison entirely rather than being classified. Re-run against the new
archive, the comparer reproduces round 1's report byte for byte except for the
two lines that named tersmu leaves: the same 19,769 changed fixtures, the same
19,286 + 19,695 + 456 + 33 + 0 mechanical classes, and the same 73 manual rows.
That is the strongest evidence available that the conflict resolution preserved
the epoch: had any expectation been lost or altered in the 1,219 conflicted
files, a class count would have moved.

The fixture counts above also stand unchanged. The re-measured base run reports
26,517 fixtures and 513 xfails at `67cc7e4b5a` — the same numbers `48ad77a06c`
reported — so the epoch's delta is still 26,517 → 26,553 and 513 → 519. What
does move is the *facet* count, 4 → 3: the `tersmu-json` facet no longer exists,
so at an unchanged 26,553 fixtures the full profile now reports 72,499 passed
(was 73,839) and 6,641 skipped (was 31,854). The skip count collapses because
almost every fixture used to be counted as skipping the tersmu facet.

One prose correction was made in the tree rather than only here:
`corpus/alis/full-alice` carried a comment describing the temporary #866
coverage gap as three retired renderings including Tersmu output. Two remain,
so the comment now names two and notes that #869 removed the third facet
repo-wide. The change is a TOML comment; the parsed fixture data is identical
and the fixture passes unchanged.

Ten of the eleven diagnostic pins survive. `smusni_structural.rs` is one of the
ten files #869 deleted, so the smusni structural connective witness — the
`ganai broda gi brode .a brodi` surface repaired to `gi'a` — no longer exists to
pin. Its disposition above stands as the record of what round 1 did; there is
simply nothing left to re-verify. The other ten pins replay unchanged and are
green in the round-2 workspace and expensive-contract runs.

### The round-2 gate

| component | result | log |
| --- | --- | --- |
| `cargo fmt --all --check` | clean | `epoch07-r2-fmt.log` |
| `cargo test -r --workspace --features jbotci-dictionary/import --no-fail-fast` | 103 targets, 1,649 passed, 0 failed | `epoch07-r2-workspace.log` |
| `cargo test -r --workspace --all-targets --features expensive_contracts --no-fail-fast` | 70 targets, 0 failed | `epoch07-r2-expensive.log` |
| `fixture-test --profile all` | 26,553 fixtures, 3 facets, 72,499 passed, 519 xfailed, 0 failed | `epoch07-r2-fixtures-all.log` |
| tagged facet `bridi-tail-epoch` | 36 fixtures, 36 passed, 0 failed | `epoch07-r2-tagged-facet.log` |
| frozen syntax facet, same tag | 36 fixtures, 36 passed, 0 failed | `epoch07-r2-frozen-facet.log` |
| `semantics-coverage` | **not run — the subcommand no longer exists**; #869 retired it from `xtask-full` with tersmu | — |
| comparer | 19,769 changed / 19,286 + 19,695 + 456 + 33 + 0 mechanical / 73 manual | `epoch07-r2-comparer3.log` |
| comparer unit tests | 11 tests, green | `epoch07-r2-comparer-test.log` |
| `cargo build -p jbotci` (debug) | green | `epoch07-r2-debug-jbotci.log` |
| `dx build` (debug) | green | `epoch07-r2-dx.log` |
| peak RSS, full profile | base 5,767,348 KB → head 3,455,884 KB, **-40.1%** (gate: base +20%) | `epoch07-r2-rss-base.log`, `epoch07-r2-rss-head.log` |

The peak-RSS pair follows round 1's corrected protocol exactly: two trees on one
volume, each with its own target directory, each `xtask-full` built *outside*
the timed window, then `/usr/bin/time -v cargo run -q -r -p xtask-full --
fixture-test --profile all` back to back at default parallelism. The base tree
is `67cc7e4b5a` checked out at `/build/jbotci/scratch/epoch07-r2/base-worktree`
and got an untimed warm-up pass first, since the head tree's page cache was
already warm from the profile run. `/usr/bin/time` reports 48 and 512 filesystem
output blocks for the two timed runs, which is how a run with no rustc inside it
looks; the contaminated round-1 base recorded 3,974,248.

Both numbers land where the previous measurement says they should: the base is
5,767,348 KB against round 1's 5,841,348 KB for the same corpus (-1.3%, the
tersmu blocks it no longer reads), and the head is 3,455,884 KB against
3,471,488 KB (-0.4%). No runtime claim is made from the pair — user time is in
fact *lower* on the head (470.40s against 495.34s) while wall clock is higher
(4:17 against 3:33), which is contention from other work on a shared box, not a
property of the change.
