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

  The arm is not enough on its own, and round 1 shipped it without the second
  half. Rolling Zantufa's `tag_term` is
  `!gek (tag !(!tag selbri) !gek_bridi_tail !BO / ...)` (zantufa-1.9999.peg:31),
  and the `!BO` lookahead is what keeps a tail term from swallowing the tag that
  opens this joint. Without it the selbri tail's own `[zero_or_more term]` takes
  `pu` with an elided KU and the joint is never reached, so only an explicit VAU
  closing that term list — which the source does not require — got there. Round 3
  adds the reservation to the two tag-term leaves that feed tail terms,
  `tagged_sumti_term` and `nonabs_tagged_sumti_term`, as
  `zantufa_tag_bo_joint_reservation`, an inert guard in the
  `zantufa_place_tag_chain_guard` idiom, active whenever `ZantufaConnectives` or
  `ZantufaTerms` is on and inert only when both are off. It sits exactly where the
  source puts it, after the parsed tag and before the elidable payload, so an
  explicit KU is still a term and `mi broda pu ku bo brode` is still a rejection in
  every dialect. Full prose and the measured cells are in the round-3 section; the
  round-4 section records why the gate covers the terms axis too.

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
| rolling Zantufa's `!BO` reservation in `tag_term` was missing — CLOSED in round 3 | Round 1 landed the connectiveless `tag BO` joint arm without the `!BO` half of the source's own `tag_term` (zantufa-1.9999.peg:31), so the tail-term list absorbed the tag first and the joint was reachable only behind an explicit VAU the source does not require. Under the full preset the epoch-6c term-level connectorless BO arm reached the tag before the joint did and the parse then failed, which is a wrong-owner reach rather than a clean rejection. Round 3 adds the gated reservation to both tag-term leaves; ten dialect-axis witnesses plus three explicit-KU rejection rows pin the result. Recorded here because the round-1 D3-2 prose claimed the arm "lands" while it reached only one of its own canonical surfaces. |
| the `!BO` reservation was gated on `ZANTUFA-CONNECTIVES` alone — CLOSED in round 4 | Round 3 gated the reservation on the feature that creates the joint it reserves for, which left one axis diverging: with `ZANTUFA-TERMS` alone, `pu bo ko'a broda` still took epoch 6c's term-level connectorless BO reading where camxes-standard, camxes-exp *and* rolling Zantufa all reject. That is a Zantufa projection preserving an exposed defect, which the epic's acceptance policy forbids. Round 4 widens the gate to `ZANTUFA-CONNECTIVES` **or** `ZANTUFA-TERMS`: `ZANTUFA-TERMS` is exactly the feature that enables the 6c arm, so activating there removes the defect at its source rather than around it. The guard stays inert when both are off, which is the only configuration with no Zantufa arm behind it, so the default, `()`, omitted, std and exp projections are byte-identical — the frozen facets and the unchanged comparer class counts are the evidence. The terms-axis witness flips to the same `unexpected-cmavo` rejection at `bo` its three siblings pin, so all four configurations now agree with all three references. Zantufa's own `!BO` is still unconditional where jbotci's is two-feature gated; that residue is not observable, because the arms it would apply to exist only behind those two features. |
| rolling Zantufa's `!gek_bridi_tail` reservation in the same source line is NOT adopted | Same line, other half. For `pu ge broda gi brode` camxes-standard and rolling Zantufa both accept and camxes-exp rejects (measured against `camxes.js`, `camxes-exp.js` and `zantufa-1.9999.js`; camxes-exp fails at the GEK), but the two accepting references disagree on ownership: R1 keeps the baseline reading — a tagged leading term, then the GEK-led tail — while rolling Zantufa attaches the tag to the tail through `!gek_bridi_tail`. This is a *reinterpretation*, not an acceptance difference, so under the standing reinterpretation ruling it is recorded rather than minted: adopting it needs its own feature-gated reservation and its own decision, and combining it with the `!BO` repair would have changed ownership under cover of a reachability fix. The fidelity-flag candidate is named `ZANTUFA-GEK-TAIL-TAG-REINTERPRETATION` here for the follow-up to pick up; nothing is added to the flag inventory by this epoch. `adhoc/syntax/bridi-tail/zantufa-gek-tail-leading-tag-baseline-owned` measures the baseline ownership at the default dialect so the gap has a witness. |

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

The tree moves 26,517 → 26,553 with round 1's 36 witnesses, and 26,553 → 26,573
with round 3's twenty (the round-3 section lists them). The xfail count
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

## Round 3 — the four review corrections

Round 2 was reviewed at `fe3492cdc3` and came back CHANGES with four findings.
Round 3 is exactly those four corrections on top of that commit; the round-1 and
round-2 commits are untouched.

### H1 — the connectiveless `tag BO` joint was reachable only behind an explicit VAU

The finding is a reachability defect, not a missing arm. `bridi_tail_bo_joint`
already carried `zantufa_tag_bo_bridi_tail_continuation`, but the tail-term list
in front of it absorbed the tag first, so the only surface that reached the joint
was the one the round-1 witness used — `mi broda vau pu bo brode`, where the VAU
closes the term list. Rolling Zantufa needs no VAU: its `tag_term` is

```
!gek (tag !(!tag selbri) !gek_bridi_tail !BO / ...)      zantufa-1.9999.peg:31
```

and the `!BO` lookahead is exactly what reserves the BO that opens the joint at
`:22` from the term that would otherwise swallow the tag ahead of it. jbotci's
two tag-term leaves that feed tail terms — `tagged_sumti_term`, whose only
structural guard was `assert !selbri`, and `nonabs_tagged_sumti_term`, which has
none — carried no such reservation.

The correction adds `zantufa_tag_bo_joint_reservation` to both, spelled in the
DSL's existing inert-guard idiom (the one `zantufa_place_tag_chain_guard` uses):

```
alias "tag" zantufa_tag_bo_joint_reservation = choice((
    feature(ZantufaConnectives).not(),
    cmavo(Bo).not(),
)).ignored();
```

`assert`ed, it succeeds without consuming anything whenever the feature is off,
so every projection that does not enable `ZANTUFA-CONNECTIVES` is byte-identical.
(Round 4 widens that gate to `ZANTUFA-CONNECTIVES` **or** `ZANTUFA-TERMS`; the
round-4 section below has the reason and the re-measured cells. Everything else in
this subsection, including the placement argument, is unchanged by that.)
Placement matters and follows the source: the reservation sits **after** the
parsed tag and **before** the elidable payload, so an overt sumti payload is
untouched and an explicit KU still makes the tag a complete term. That is why
`mi broda pu ku bo brode` stays a rejection in every dialect, as it is in rolling
Zantufa itself.

The measured cells, against `camxes.js`, `camxes-exp.js` and `zantufa-1.9999.js`
on one side and the round-3 head binary on the other:

| surface | camxes | camxes-exp | rolling Zantufa | head `()` / default | head `(+zantufa-connectives)` | head `(+zantufa-connectives +zantufa-terms)` | head `(zantufa)` |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `mi broda pu bo brode` | R | R | A | R | **A** | **A** | **A** |
| `mi broda do pu bo brode` | R | R | A | R | **A** | **A** | **A** |
| `mi broda pu ku bo brode` | R | R | R | R | R | R | R |
| `mi broda vau pu bo brode` | R | R | A | R | A | A | A |

The three accepting rows carry exactly one warning,
`syntax.warning.experimental-zantufa-tail-continuation`, anchored on the tag.
Nothing about the warning matrix moves: the joint was already the warning's
owner, and the reservation only changes which surfaces reach it.

Ownership on the leading-term row is the point of the second surface. Before the
reservation, `mi broda do pu bo brode` under the full preset was first mis-owned
by epoch 6c's term-level connectorless BO arm — `do (pu [bo brode])`, a
wrong-owner reach — and then failed. After it, `do` stays a tail term of the
first bridi tail and the joint owns `pu bo brode`:
`(mi [{bróda do} {pu bo bróde}])`, with `ZantufaTagBoBridiTailContinuation` in
the raw tree. 6c's own witnesses are unaffected, and structurally cannot be: its
connectorless-BO surfaces (`pu ko'a bo ca ko'e broda` and its five dialect-axis
rows, `zantufa-bo-goi-payload-connectorless`, `zantufa-bo-normal-term-mixed-chain`)
all carry an overt sumti after the tag, so the token at the reservation's
position is never BO. The full profile confirms it: no pre-existing fixture
changed.

The reservation also has a term-level consequence, and it is a parity gain
rather than a cost. `pu bo ko'a broda` is rejected by camxes-standard,
camxes-exp *and* rolling Zantufa — the source's own unconditional `!BO` stops
`pu` from being a term with an elided KU before a BO — while epoch 6c's
connectorless term-level BO arm accepted it. With the reservation, head rejects
it wherever `ZANTUFA-CONNECTIVES` is on, which is the gate, both axes and the
full preset; with `ZANTUFA-TERMS` alone the 6c reading survives, because the
reservation is gated on the feature that creates the joint. Four
`zantufa-tag-bo-term-reservation-*` witnesses pin all four configurations and a
gap-ledger row records the one axis that still diverges. Nothing else in the
tree moves: the full profile reports no pre-existing fixture changed.

Ten new dialect-axis witnesses pin the two VAU-less surfaces across
`(+zantufa-connectives)`, `(zantufa)`, `(+zantufa-connectives +zantufa-terms)`,
`()` and the omitted default; three more pin the explicit-KU rejection across
the gate, the full preset and the no-feature control. There is no `(standard)`
builtin dialect in this repository — no fixture in the tree names one — so the
standard-side control is the pair epoch 6c uses for it, the explicitly empty
`()` and the omitted default, which measure identically here. The round-1 VAU
witness stays exactly as it was.

### M2 — the recovered continuation classifiers were fail-open

`recovered_continued_tail_has_owner` and its tail-terms-free twin filtered the
continuation list through `valid` before `.all(...)`, so a list of only
recovered prefix/error elements passed vacuously and a mixed list was judged on
its completed elements alone. Either way the priority wrapper then rejected a
candidate for which no complete owner had actually been established — the
opposite of what "every continuation is a tag-less GIhA continuation" asserts.
Both now use the fail-closed form `baseline_tag.rs` already uses at its own
recovered classifier: `.all(|c| valid(c).is_some_and(...))`.

Two recovery fixtures pin the populations, both under `(+zantufa-connectives)`:

| fixture | surface | recovered continuation list |
| --- | --- | --- |
| `adhoc/recovery/syntax/zantufa-tail-continuation-all-invalid` | `mi broda je cu gi brode` | one element, invalid |
| `adhoc/recovery/syntax/zantufa-tail-continuation-mixed-invalid` | `mi broda je cu gi brode gi'e brodi` | invalid, then a complete tag-less GIhA |

Both pin diagnostics on the strict and recovered facets and the recovered tree's
valid-token and recovery-item lists; the mixed one keeps `gi'e bródi` in its
valid tokens, which is the completed GIhA element the old form judged the whole
list by. The change is observable, not merely defensive: built from `HEAD` and
from the corrected tree back to back, the all-invalid surface's recovered tree
differs at the statement envelope (`IStatementConnection` before, plain
`StatementBase(BridiStatement(...))` after). The mixed surface's classifier
verdict differs by construction — fail-open true on the one completed element,
fail-closed false — and its tree is pinned at the corrected value.

### M3 — the `!gek_bridi_tail` half of the same source line, recorded not minted

Ledger only; no grammar change. `pu ge broda gi brode` is an ownership
difference rather than an acceptance one, and it is the sibling of H1's `!BO` on
the very same `tag_term` line. Measured:

| surface | camxes | camxes-exp | rolling Zantufa | head, default |
| --- | --- | --- | --- | --- |
| `pu ge broda gi brode` | A | **R** | A | A |
| `mi pu ge broda gi brode` | A | **R** | A | A |

The round-2 review recorded camxes-exp as accepting these; re-probed against
`camxes-exp.js` it rejects both at the GEK (`Expected [,] but "b" found`), so the
cell is recorded as measured. The two accepting references still disagree about
what the tag attaches to: R1 keeps the baseline reading — a tagged leading term,
then the GEK-led tail — while rolling Zantufa attaches the tag to the tail
through `!gek_bridi_tail`. Head reads it the baseline way,
`TaggedSumtiTermSyntax` before `DirectForethoughtBridiConnectionSyntax`, and
silently.

Under the standing reinterpretation ruling a differing-extent dialect overlap of
this kind is baseline-first plus a documented gap, with the fidelity flag named
rather than added; the gap-ledger row above does that and names
`ZANTUFA-GEK-TAIL-TAG-REINTERPRETATION` as the candidate. Adopting it needs its
own feature-gated reservation and its own decision, which is precisely why it is
not folded into the `!BO` repair.
`adhoc/syntax/bridi-tail/zantufa-gek-tail-leading-tag-baseline-owned` pins the
surface at the default dialect with `diagnostics = []`, so the baseline
ownership is measured rather than assumed.

### L4 — the dead `connective` view fields

`GeneratedBridiTailBoJointRef` and its tail-terms-free twin carried a
`connective` field whose only readers were in the generated-builder files #869
deleted. Every surviving consumer — `references.rs` at its two BO-joint analysis
sites and its two visitor sites — reads only the tag, the operand and the tail
terms. The field, its `ensures` clause and the doc prose describing it are
removed; `BridiTailConnectiveSyntax` is no longer imported by the file. The
postcondition is not weakened to `true`: what remains true of the view is that
the Zantufa arm's tag is mandatory, so both `from_joint` methods now pin
`matches!(joint, …ZantufaTagBoBridiTailContinuation(_)) -> ret.tense_modal.is_some()`.

### The round-3 gate

No peak-RSS pair this round. The only grammar change is a zero-width lookahead
that fails earlier than the term it guards; it allocates nothing and cannot
raise peak resident set. The round-2 pair stands.

| component | result | log |
| --- | --- | --- |
| `cargo fmt --all --check` | clean | `epoch07-r3-fmt.log` |
| `cargo test -r --workspace --features jbotci-dictionary/import --no-fail-fast` | 103 targets, 1,649 passed, 0 failed | `epoch07-r3-workspace.log` |
| `cargo test -r --workspace --all-targets --features expensive_contracts --no-fail-fast` | 70 targets, 1,648 passed, 0 failed | `epoch07-r3-expensive.log` |
| `fixture-test --profile all` | 26,573 fixtures, 3 facets, 72,519 passed, 519 xfailed, 0 failed | `epoch07-r3-fixtures-all.log` |
| tagged facet `bridi-tail-epoch` | 56 fixtures, 56 passed, 0 failed | `epoch07-r3-tagged-facet.log` |
| frozen syntax facet, same tag | 56 fixtures, 56 passed, 0 failed | `epoch07-r3-frozen-facet.log` |
| comparer | 19,769 changed / 19,286 + 19,695 + 456 + 33 + 0 mechanical / 73 manual / 0 prose / 56 epoch-new | `epoch07-r3-comparer2.log` |
| comparer unit tests | 11 tests, green | `epoch07-r3-comparer-test2.log` |
| `cargo build -p jbotci` (debug) | green | `epoch07-r3-debug-jbotci.log` |
| `dx build` (debug) | green | `epoch07-r3-dx.log` |
| `maturin develop` + the four generated checks | green | `epoch07-r3-maturin.log`, `epoch07-r3-gen-*.log` |
| peak RSS | **not measured — see above** | — |

The comparer and comparer-unit-test rows were re-run *after* the round-3 commit
(`epoch07-r3-comparer2.log`, `epoch07-r3-comparer-test2.log`), because the
comparer's pairing check reads the baseline out of git and therefore needs the
round's own commit in place; every other row in the table ran before that commit,
on a source tree identical to the committed one.

The comparer's class counts are unchanged from round 2 by construction: this
round adds no fixture that existed at the base, so every file it touches is
epoch-new and lands in the pinned added-witness list rather than in a class.
`EXPECTED_NEW_WITNESSES` therefore moves 36 → 56 and nothing else does. The
comparer's unit tests run under `unittest` here; `pytest` is not installed on
this box, and the eleven tests are the same eleven either way.

The first gate attempt failed one target,
`grammar::tests::generated_recovery_anchor_metadata_snapshot_matches`, and the
snapshot is regenerated rather than waived. Its whole diff is `rules: 601 → 602`
for the new `zantufa_tag_bo_joint_reservation` alias — which contributes an
empty anchor block, being inert — plus the field- and resume-index shift the
extra `assert` slot causes in the two guarded leaves. Every `first` token set in
the file is byte-identical as a multiset before and after, which is the check
that the reservation changed no anchor's reachable token set.

## Round 4 — the one residual on the `!BO` reservation

Round 3 was reviewed at `32a6503d87` and came back CHANGES with a single
residual. H1's placement, M2, M3 and L4 were all confirmed; round 4 changes
nothing about any of them.

### The gate was one feature too narrow

The reservation as round 3 shipped it was

```
alias "tag" zantufa_tag_bo_joint_reservation = choice((
    feature(ZantufaConnectives).not(),
    cmavo(Bo).not(),
)).ignored();
```

— active only where `ZANTUFA-CONNECTIVES` creates the tail joint it reserves
for. That reasoning is right about the joint and wrong about the reservation's
other consequence. The source's `!BO` in `tag_term` (zantufa-1.9999.peg:31) is
not written for the joint alone: it is also why `pu bo ko'a broda` is a
rejection in rolling Zantufa, exactly as it is in camxes-standard and
camxes-exp. The arm that accepts that surface in jbotci is epoch 6c's
connectorless term-level BO continuation, and that arm is enabled by
`ZANTUFA-TERMS`, not by `ZANTUFA-CONNECTIVES`. So the narrow gate left the
`(+zantufa-terms)` projection accepting a surface all three references reject —
a Zantufa projection preserving an exposed baseline defect, which the epic's
acceptance policy does not allow.

Round 4 widens the gate to cover the terms axis, keeping the same inert-lookahead
idiom and consuming nothing:

```
alias "tag" zantufa_tag_bo_joint_reservation = choice((
    choice((
        feature(ZantufaConnectives),
        feature(ZantufaTerms),
    )).not(),
    cmavo(Bo).not(),
)).ignored();
```

The inert branch is now the not-of-both: the guard stands aside only when
*neither* feature is on, which is the one configuration where no Zantufa arm sits
behind the BO it would reserve. Everything else about the reservation — its
placement after the parsed tag and before the elidable payload, the explicit-KU
rejection, the untouched overt-sumti payload — is unchanged, so nothing in the
round-3 H1 argument is re-opened.

### What moved, and what provably did not

The whole observable delta is the `(+zantufa-terms)` cell of one surface:

| surface | camxes | camxes-exp | rolling Zantufa | `()` / default | `(+zantufa-connectives)` | `(+zantufa-terms)` | both axes | `(zantufa)` |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `pu bo ko'a broda` (round 3) | R | R | R | R | R | **A** | R | R |
| `pu bo ko'a broda` (round 4) | R | R | R | R | R | **R** | R | R |

The round-4 rejection is byte-for-byte the one its three siblings already pin:
`syntax.unexpected-cmavo` at `bo`, byte span `[3, 5]`, and no other diagnostic.
`zantufa-tag-bo-term-reservation-terms-axis` is flipped to it by the project's
own writer and hand-checked against the sibling pins; the provenance note on all
four witnesses is corrected to say the gate now covers both axes.

The tail surface the reservation exists for is a rejection on the terms axis
either way, because the joint arm needs `ZANTUFA-CONNECTIVES`; what changes is
*how* it is rejected. Measured on the round-3 and round-4 binaries back to back,
`mi broda pu bo brode` under `(+zantufa-terms)` alone was

```
error[syntax.unexpected-brivla] at `brode`
warning[syntax.warning.experimental-zantufa-connectorless-bo] at `bo`
(mi [bróda {pu ([bo ‼brode‼] ‼‼)}])
```

— the 6c term arm reaching the tag first, warning for a connection it then could
not complete — and is now

```
error[syntax.unexpected-cmavo] at `bo`
‼mi broda pu bo brode‼
```

the clean reservation rejection at the BO, with no warning for a reading that
never happened. No fixture pins that cell, so nothing is regenerated for it. The
explicit-KU control is untouched on the same axis: `mi broda pu ku bo brode` is
byte-identical on both binaries, since an explicit KU closes the term before the
reservation's position is reached.

Nothing else in the corpus can move, and the full profile confirms that nothing
did. The reservation is reachable only where one of the two features is on, and
`ZANTUFA-CONNECTIVES` already had it in round 3, so the newly covered population
is exactly the eleven `(+zantufa-terms)`-only fixtures in the tree. Ten of them
have no tag-then-BO position at all, and epoch 6c's own connectorless-BO
witnesses — `zantufa-bo-term-connectorless-terms-axis`
(`pu ko'a bo ca ko'e broda`) and `zantufa-bo-sumti-connectorless-terms-axis`
(`ko'a bo ko'e broda`) — carry an overt sumti after every tag, so the token at
the reservation's position is never BO; both parse byte-identically on the two
binaries. The default, explicitly empty `()`, omitted, camxes-standard and
camxes-exp projections are byte-identical too, which is what the frozen facets
and the unchanged comparer class counts measure; the four canonical joint
surfaces (`pu bo ko'a broda`, `mi broda pu bo brode`, `mi broda do pu bo brode`,
`mi broda vau pu bo brode`) were also diffed directly across `()`, the omitted
default, `(+zantufa-connectives)`, both axes and `(zantufa)` and are identical on
every one.

The recovery-anchor snapshot does **not** move this round. Round 3's regeneration
already accounted for the alias and for the extra `assert` slot in the two
guarded leaves; round 4 rewrites only the alias body, adding no rule, no field
and no assert, and the alias's anchor block was already empty because the guard
is inert. `rules` stays at 602 and the file is unchanged on disk —
`generated_recovery_anchor_metadata_snapshot_matches` passes without an update,
and the multiset of all 3,493 `first` lines is trivially identical, being the
same bytes.

### The round-4 gate

No peak-RSS pair this round, for the round-3 reason: the only grammar change is a
zero-width lookahead that now fails earlier in one more configuration. It
allocates nothing and cannot raise peak resident set. The round-2 pair stands.

| component | result | log |
| --- | --- | --- |
| `cargo fmt --all --check` | clean | `epoch07-r4-fmt.log` |
| `cargo test -r --workspace --features jbotci-dictionary/import --no-fail-fast` | 103 targets, 1,649 passed, 0 failed | `epoch07-r4-workspace.log` |
| `cargo test -r --workspace --all-targets --features expensive_contracts --no-fail-fast` | 70 targets, 1,648 passed, 0 failed | `epoch07-r4-expensive.log` |
| `fixture-test --profile all` | 26,573 fixtures, 3 facets, 72,519 passed, 519 xfailed, 0 failed | `epoch07-r4-fixtures-all.log` |
| tagged facet `bridi-tail-epoch` | 56 fixtures, 56 passed, 0 failed | `epoch07-r4-tagged-facet.log` |
| frozen syntax facet, same tag | 56 fixtures, 56 passed, 0 failed | `epoch07-r4-frozen-facet.log` |
| comparer | 19,769 changed / 19,286 + 19,695 + 456 + 33 + 0 mechanical / 73 manual / 0 prose / 56 epoch-new | `epoch07-r4-comparer.log` |
| comparer unit tests | 11 tests, green | `epoch07-r4-comparer-test.log` |
| `cargo build -p jbotci` (debug) | green | `epoch07-r4-debug-jbotci.log` |
| `dx build` (debug) | green | `epoch07-r4-dx.log` |
| peak RSS | **not measured — see above** | — |

As in round 3, the comparer and its unit tests run after the round's commit,
because the comparer's pairing check reads the baseline out of git; every other
row ran before the commit on an identical source tree. Its output is *byte*-identical
to round 3's — same 19,769 changed, same five class counts, same 73-line manual
residue, same 56 epoch-new witnesses — which is the direct evidence that no
projection outside the terms axis moved: this round adds no fixture, and the one
witness it rewrites was already epoch-new and therefore already listed rather
than classified. The baseline root is `git archive 67cc7e4b5a tests/fixtures`,
re-derived for this round and verified byte-identical to the one round 3 used.
