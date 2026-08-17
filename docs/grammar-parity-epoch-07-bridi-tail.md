# Grammar parity epoch 7: the bridi tail

Epoch 7 is the bridi-tail slice of the grammar-parity epic and carries three
issues at once, because they are three views of the same six productions:

| Section | Issue | State |
| --- | --- | --- |
| Zantufa forethought reconcile audit | #826 | complete, no grammar change |
| Zantufa JOIK/tag tail continuations and the KE ownership guard | #826 | complete |
| Standard tail boundaries | #805 | complete |
| camxes-exp CU and term prefixes | #815 | pending |
| Consolidated expectations, comparer, peak RSS | — | pending |

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
  `ZantufaConnectives`-gated arms. That one edit widens the flat joint, the BO
  joint and the KE join at once, which is exactly the source's own shape — one
  joint, a wider connective — and it leaves every existing expectation untouched,
  because a GIhA-led joint still selects the `GihekConnective` arm it selects
  today.
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
ships: the flat joint takes the JOIK connective, the top continuation takes the
tag. The measured consequence is the `mi broda je pu cu brode` class, where the
inner joint would otherwise pair a Zantufa connective with a camxes-exp term
prefix in its operand and warn twice for one construct; the D2 commit adds the
classifier that returns those candidates to the tag-bearing top continuation, so
the surface reads as one Zantufa construct and warns once.

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
