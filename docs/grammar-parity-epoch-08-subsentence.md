# Grammar parity epoch 8: the subsentence

Epoch 8 is the subsentence slice of the grammar-parity epic and carries two issues, because
both are re-audit corrections to the same warning union over the same three sources:

| Section | Issue | State |
| --- | --- | --- |
| NOI-family statement relatives, per attachment site | #818 | complete |
| camxes-exp's tanru-unit relative clause | #818 | complete |
| The SOI/XOI/FIhOI adverbial trio | #823 | complete |
| Consolidated expectations, comparer, peak RSS | — | complete |

The design is `~/git/grammar-review/reports/implementation/epoch-08-subsentence/plan.md`
v5-CONFIRMED, with its frozen owner decision function R1-R4, its S1/S1f/S2/S3 attachment-site
partition and its 30-row representative-cell table. The implementation base is `0d791fd35c`,
the #866 full-alice merge.

## Deltas from plan v5

Four frozen mechanisms changed under measurement. Each was raised as an ASK and adjudicated by
the lead in `#jbotci-801`; none of them changes what the epoch adopts, only how the frozen
mechanism is realized or where a cell lands.

| # | frozen in plan v5 | as implemented | provenance | where |
| --- | --- | --- | --- | --- |
| 1 | S2's R2 return is a candidate-local return inside the S2 entry | the return is realized as a **D2 preemption**: the camxes-exp arm declines wherever an explicit-KUhO Zantufa clause parses from the same position | lead ANSWER 2026-08-31T03:31:06Z | [D2's route boundary](#the-route-boundary-plan-v5-delta-1) |
| 2 | S3 names the Zantufa statement arms and its body is a statement | S3 is the alias `selbri_relative_clause_list`, wrapping S2's list with the R2 return on the **whole completed list**; the inner arm may be the **baseline** relative whenever the content is baseline-shaped | lead ANSWER 2026-08-31T03:31:06Z | [the S3 annotation correction](#the-s3-annotation-correction-plan-v5-delta-2) |
| 3 | rolling Zantufa's `gek_statement` is covered | it is covered at the sentence level by `gek_sentence` inside `bridi`; **gek over I-connected branches is a documented residual gap**, pinned on both sides | lead ANSWER 2026-08-31T03:31:06Z | [the body](#the-body) |
| 4 | the S3 parent is default-enabled-warn | the S3 parent stays behind `ZANTUFA-TERMS` as an **explicitly declared retained gated omission**, because 24 measured reinterpretations of successful baseline parses put it under the epic's flag rule | lead ANSWER 2026-08-31T06:02:58Z | [the retained gated omission](#the-retained-gated-omission-plan-v5-delta-4) |

## What the base actually did

The route was half-implemented rather than missing. `bridi_relative_clause` already carried two
`ZantufaTerms`-gated statement-relative arms with the full source NOI inventory, and the two
baseline subbridi arms carried `po'oi`, `voi'i` and `no'oi` as well. Measured at the base:

| surface | default profile |
| --- | --- |
| `lo broda po'oi mi brode ku'o cu brodi` | ACCEPT, silent |
| `lo broda voi'i mi brode ku'o cu brodi` | ACCEPT, silent |
| `lo broda no'oi mi brode ku'o cu brodi` | REJECT |
| `mi klama no'oi bajra` | ACCEPT, `experimental-zantufa-cmavo` |
| `mi broda fi'oi mi brode se'u` | REJECT |
| `mi broda fi'oi mi brode i je do brodi fi'au` | ACCEPT |

So two rolling-Zantufa markers were accepted as baseline relatives with no warning at all, one
was unreachable in every profile because its morphology classed it as a UI indicator, the
camxes-exp adverbial's own terminator had no route, and the FIhOI arm accepted a shape no
reviewed source spells.

## D1: ownership is decided per attachment site

The narrow half of the fix is inventory. The baseline arms take camxes-standard's own NOI
(camxes.peg:1695, which camxes-exp shares at :1807) and nothing else, and `Nohoi` loses the
`[Ui, Ui3a]` classes: rolling Zantufa's NOI (zantufa-1.9999.peg:590) and camxes-exp's NOhOI
(camxes-exp.peg:1907) are its only classifications, and the relation word's `.wf()` was eating
it as an indicator before any relative arm could see it.

The wide half is that ownership cannot be decided from the clause. Three surfaces with the same
inner shape have three different owners:

| surface | std / exp / Zantufa | owner |
| --- | --- | --- |
| `broda poi mi brode` | R / R / A | rolling Zantufa, at the selbri parent |
| `ko'a no'oi mi brode broda` | R / R / A | rolling Zantufa, at an ordinary sumti site |
| `broda no'oi mi brode` | R / A / A | camxes-exp's tanru-unit relative |

What decides is the consuming field site, so the shared relative list is parameterized by the
site's own entry rather than naming the arms itself. The partition is over field sites, not line
spans, because the list, tail and atom machinery is shared:

| class | consuming field sites | entry | policy |
| --- | --- | --- | --- |
| S1 | the `relative_clause_list` consumers on non-description sumti — `vuho_relative_sumti_attachment_tail`, `experimental_vuho_scoped_sumti_attachment_tail`, `simple_sumti`, `lahe_sumti`, `scalar_negated_sumti_with_bo`, `name_sumti` | `statement_relative_clause` | baseline arms + Zantufa statement arms, the latter returning a standard marker over a subbridi-shaped body |
| S1f | the standalone `relative_clause_fragment` | the same entry | S1's, by being S1's instantiation |
| S2 | every `bare_continuable_relative_clause_list` consumer — the four description shapes, the two description-tail shapes and the four vocative slots | `description_relative_statement_relative_clause` | S1's policy over the description-boundary body flavour |
| S3 | `zantufa_relative_selbri` | `selbri_relative_clause_list` | S2's list plus the R2 return on the whole list, and gated (below). The entry is a wrapper, not a new arm inventory: whatever S2's list admits at this position, S3 admits, minus the lists R2 hands to camxes-exp |

The shared ZIhE and joik continuation machinery is connective machinery rather than an owner
class: it takes the enclosing site's entry as a parameter, so D1's new atoms appear in
continuation position under that site's policy and under no policy of their own. One
ZIhE-chained witness pins each of the four classes.

### The body

The arms' body is a tailored transcription of `statement` (zantufa-1.9999.peg:12-18) rather than
the shared statement node. Three measured deltas make the shared node the wrong body here:

| delta | shared `statement` | rolling Zantufa | disposition |
| --- | --- | --- | --- |
| connective | `joik_connective / jek_connective / ek_connective / vuhu_nonlogical_connective` | `joik` only (:556, one selma'o merging the standard JOI and JA inventories) | narrowed to `standard_statement_connective`, which is exactly that union |
| prenex | `zero_or_more term` | `terms ZOhU`, terms required | `one_or_more term` |
| preposed connection | `preposed_i_statement_connection` | none | absent, which is what makes the JACU trailer a rejection |

The admission set the witnesses pin, both directions: non-empty prenex A, `ije` A, `I … BO` A,
TUhE A, gek A; empty prenex R, bare-`i` R. Rolling Zantufa's statement-level `gek_statement`
(:16) is not a separate arm: its sentence-level twin is already inside `bridi` as `gek_sentence`,
which is what carries `poi ge mi brode gi do brodi ku'o` at this position and keeps it
baseline-owned.

**Gek over I-connected branches is a documented residual gap** (plan-v5 delta 3), and both sides
of it are pinned rather than asserted. `gek_sentence` takes *sentences* as branches, while
`gek_statement` takes *statements*, which may themselves be I-connected; the difference is
exactly one surface class:

| surface | result | witness |
| --- | --- | --- |
| `lo broda poi ge mi brode gi do brodi ku'o cu brodi` | A, silent — the whole gek is the baseline's, through the bridi route, and the pinned tree shows `ForethoughtSimpleBridiTail` inside a `BridiSubbridi` rather than any Zantufa statement node | `d1-baseline-gek-relative-body` |
| `lo broda poi ge mi brode ije do brodi gi da brodi ku'o cu brodi` | R in both profiles — a gek whose own branch is I-connected is `gek_statement`'s shape and has no route | `d1-gap-gek-i-connected-branch` |

The accepting witness pins the bridi-route tree specifically so the gap's boundary is measured:
what is missing is the statement-level gek, not gek in relative bodies as such.

### The S3 annotation correction (plan-v5 delta 2)

Plan v5's S3 cell annotates the entry as the Zantufa statement arms over a statement body. As
implemented, S3 is an alias that wraps S2's list whole:

```
alias "relative clauses" selbri_relative_clause_list(bare_continuable_relative_clause_list)
    = bare_continuable_relative_clause_list
        .reject_output(ExpSelbriRelativeListRejection)
```

Two consequences follow, and both are more faithful than the frozen annotation.

The R2 return is on the **whole completed list**, not on one clause. camxes-exp's chain is a
single node: a ZIhE-joined list with a `poi` in it is not an extent exp can form at all, so a
per-clause test would hand exp a list it cannot derive. `ExpSelbriRelativeListRejection` requires
every atom *and* every continuation to be exp-formable before the list returns.

And the inner arm is whatever S2's entry selects — which for baseline-shaped content is the
**baseline** relative clause, not a Zantufa one. That is R1 applied at every level rather than
only at the top: a baseline marker over a baseline-formable body stays the baseline's even when
the *placement* around it is rolling Zantufa's and warns as such. The `broda poi mi brode`
witness pins the tree, so the correction is measured:

| node | what it is |
| --- | --- |
| `ZantufaRelativeSelbri` | the S3 placement, warned `experimental-zantufa-selbri-relative-placement` on the `poi` |
| ↳ `BridiRelativeClause(RestrictiveBridiRelativeClause(..))` | the **baseline** relative inside it, silent |

There is no `ZantufaStatementRelativeClause` anywhere in that tree. The plan's "statement body"
annotation for the S3 cell is therefore corrected: the body flavour S3 instantiates is S2's, and
the arm chosen inside it is decided by R1 on the content, independently of the placement.
Witness: `d1-zantufa-s3-bare-selbri-poi`.

### The retained gated omission (plan-v5 delta 4)

D1.3 freezes the S3 parent as default-enabled-warn. Measurement says it cannot be, and the
disposition is the other one Sol's round-1 finding 4 offered — disposition (b), an explicitly
declared retained gated omission. The epic's flag rule decides it: *a sourced extension that can
reinterpret a successful baseline parse is controlled by an explicit feature flag*. Twenty-four
corpus fixtures are exactly such reinterpretations, so once the measurement exists the frozen
default-enabled-warn clause loses to the policy. Lead adjudication 2026-08-31T06:02:58Z in
`#jbotci-801` confirms the gating as mandatory rather than merely acceptable.

The arm is reached inside every nesting whose terminator may elide, and there the enclosing
description's own relative-clause field is the baseline's site for the very same clause. The
worked example is `corpus/camxes/11399`, `.uesai le ni mrilu poi srana la lojban. cu mutce caku`:

| reading | bracketing | gloss |
| --- | --- | --- |
| camxes-standard, and the default union | `le ni mrilu` + `poi srana la lojban.` on the description | *the [quantity of mailing] which concerns Lojban* — the relative restricts the quantity |
| the S3 arm, if default-enabled | `le ni` + [`mrilu` + `poi srana la lojban.`] on the selbri inside the abstraction | *the quantity of [mailing which concerns Lojban]* — the relative restricts the mailing |

Both readings cover an identical extent, so nothing in the surface distinguishes them; R1 puts
the baseline first.

The full 24-parse list, measured by building `zantufa_relative_selbri` ungated (the `when
feature(ZantufaTerms)` alternative and the rule's `assert feature(ZantufaTerms)` both removed)
and running `fixture-test --profile all --facet syntax` against the pinned trees. These are the
fixtures reporting `syntax raw mismatch` — successful parses whose *tree* the arm changes, as
distinct from the fourteen `expected syntax failure, got success` rows, which are new
acceptances rather than reinterpretations:

| # | fixture | # | fixture |
| --- | --- | --- | --- |
| 1 | `cll/chrestomathy/alice01` | 13 | `corpus/camxes/2179` |
| 2 | `corpus/camxes/11399` | 14 | `corpus/camxes/21821` |
| 3 | `corpus/camxes/11733` | 15 | `corpus/camxes/21828` |
| 4 | `corpus/camxes/12145` | 16 | `corpus/camxes/2290` |
| 5 | `corpus/camxes/12966` | 17 | `corpus/camxes/2291` |
| 6 | `corpus/camxes/12970` | 18 | `corpus/camxes/2700` |
| 7 | `corpus/camxes/14301` | 19 | `corpus/camxes/2787` |
| 8 | `corpus/camxes/14510` | 20 | `corpus/camxes/3902` |
| 9 | `corpus/camxes/17831` | 21 | `corpus/camxes/5333` |
| 10 | `corpus/camxes/19967` | 22 | `corpus/camxes/6546` |
| 11 | `corpus/camxes/2119` | 23 | `corpus/camxes/8000` |
| 12 | `corpus/camxes/21633` | 24 | `corpus/camxes/8859` |

Two repairs were measured and rejected before the gate was kept. The boundary idiom the
description and the vocative use does not reach: `selbri_without_terminal_relative` restricts the
top spine, and the leak runs through an abstraction body, so closing it means following the
no-terminal-relative entry down a second ladder from selbri level 2, which this epoch does not
build. And a candidate-local classifier that returns baseline-owned lists over-rejects, because
the same list is rolling Zantufa's alone wherever no enclosing site exists — `re broda poi brode
ku` measures that, and it is what issue #828's own fixtures pin.

The second boundary ladder is recorded here as a **named follow-up candidate and nothing more**:
*follow the no-terminal-relative entry down from selbri level 2 so the description's own
relative-clause field and the S3 placement can both hold*. It is real scope, it is not this
epoch's, and no issue is minted for it here — the lead's instruction is explicit that it is
recorded, not minted.

Three cells of the frozen table move with the gate. Each carries witnesses on **both** sides, and
each is marked below as a plan-v5 delta with the 2026-08-31T06:02:58Z lead ANSWER as provenance:

| cell surface | Zantufa profile | default union | plan-v5 status |
| --- | --- | --- | --- |
| `broda poi mi brode` | `d1-zantufa-s3-bare-selbri-poi` — A, warned | `d1-s3-gap-bare-selbri-default` — R | **delta**: frozen as default-union, measured as Zantufa-profile |
| `broda no'oi mi brode ku'o` | `d2-zantufa-kuho-reservation` — A, warned | `d1-s3-gap-kuho-default` — R | **delta**: same |
| `broda no'oi mi brode ku'o zi'e no'oi do brodi ku'o` | `d1-chain-s3-zihe` — A, warned | `d1-s3-gap-zihe-default` — R | **delta**: same |

The abstraction population is witnessed as the no-delta surface it stays
(`d1-s3-gap-abstraction-no-delta`, `le ni broda poi mi brode cu brodi`, silent at the default
union). Everything else in D1 is default-enabled as frozen.

## D2: camxes-exp's tanru-unit relative clause

`selbri_relative_clauses <- selbri_relative_clause ((ZIhE_clause / joik) free*
selbri_relative_clause)* / gek selbri_relative_clauses gik selbri_relative_clauses` over
`NOhOI_clause free* subsentence KUhOI_elidible` (camxes-exp.peg:214-218), attached where the
source attaches it: after the CEI chain on the tanru unit (:241), inside the level-6 BO.

It is a new arm of `plain_bo_selbri` rather than an optional field on `tanru_unit`. The tanru
unit is nearly every node in the corpus and an absent-case field would print on all of them; the
arm requires the chain, so it is structurally disjoint from `plain_bo_tanru_unit` and ordinary
units keep the shape they have. It runs first so a present chain is not left behind by the
shorter arm.

The chain's connective is the source's own `joik`, whole — `NA_clause? SE_clause? (JOI_clause /
JA_clause / A_clause) NAI_clause? / interval / GAhO_clause interval GAhO_clause` (:346-349), under
an explicit A-JA-JOI merge. **Round 2 corrects this**: it was first transcribed as jbotci's
`joik_connective`, which is neither narrower nor wider than the source but both, and
`broda no'oi mi brode je no'oi do brodi` was pinned as a rejection on that reading. See
[round 2, H1](#h1-the-d2-chain-connective-was-not-the-sources-joik). The SA-erasure prefixes at :215-217 are omitted
as every other adopted camxes-exp family omits them, and the two SA-shaped witnesses record what
actually accepts those surfaces — jbotci's own CLL-sourced general erasure, which reads one
relative clause and not two.

`exp_subsentence` is a consumer-specific entry that is the shared `subbridi` shape today. The one
delta between camxes-exp's `subsentence` and `subbridi` is exp's JACU sentence trailer, an
adjudicated non-adoption whose rejection witness is `lo broda poi mi brode je i do brodi ku'o cu
brodi`; naming the entry separately is what keeps a later JACU decision from having to widen
every abstraction and forethought consumer.

### The route boundary (plan-v5 delta 1)

R3 read literally: rolling Zantufa keeps the KUhO-terminated extents, and KUhO is a terminator
camxes-exp does not have. The clause therefore declines wherever a Zantufa statement relative
clause closed by an explicit `ku'o` parses from the same position.

That reservation is what makes all three description cells reachable at once. A completed-
candidate classifier cannot decide between them, because what separates the owners is entirely
what follows the shared prefix:

| surface | owner | what decides |
| --- | --- | --- |
| `lo broda po'oi mi brode ku'oi cu brodi` | camxes-exp | KUhOI is exp's own terminator |
| `lo broda po'oi mi brode cu brodi` | camxes-exp (R2) | both derive it; the adopted source owns it |
| `lo broda po'oi mi brode ku'o cu brodi` | rolling Zantufa (R3) | the explicit KUhO |
| `lo broda no'oi mi brode ije do brodi ku'o cu brodi` | rolling Zantufa (R3) | the body the reservation reaches past |

The description site parses its selbri before its relative-clause field and cannot reconsider, so
without the reservation the exp arm would take the shortest reading and leave the `ku'o` — or the
Zantufa-only body on the way to it — with nowhere to attach.

#### This reservation *is* S2's R2 return

Plan v5 freezes S2's R2 return as a candidate-local return inside the S2 entry: the Zantufa
statement arms would hand back any completed candidate camxes-exp could also derive. That shape
is parent-blind, and the parents it is blind to are the pre-selbri consumers. camxes-exp's
tanru-unit relative attaches *after* a tanru unit (camxes-exp.peg:241); a description's leading
relative-clause field and the four vocative slots sit where no tanru unit precedes, so no exp arm
can reach them. A candidate-local return there would return an extent to a route that does not
exist at that position, and the surface would be lost outright rather than owned.

So the R2 return is realized in the other direction — as a decline inside D2's own arm:

```
rule "selbri relative clause" exp_selbri_relative_clause(exp_subsentence, zantufa_relative_statement) {
    assert !zantufa_kuho_terminated_statement_relative_clause(zantufa_relative_statement);
    ...
}
```

`zantufa_kuho_terminated_statement_relative_clause` is the full rolling-Zantufa NOI inventory over
the tailored relative body closed by a **required** `Kuho`. The condition is measured, not
guessed: the decline fires exactly when that clause actually parses from the same position, and
because it is a grammar-level negative assertion rather than a shape test on a finished node, it
carries no strict/recovered asymmetry to get wrong. The three classifiers this epoch does express
as `OutputRejection` — `BaselineStatementRelativeRejection`,
`BaselineRelativeContinuationRejection` and `ExpSelbriRelativeListRejection` — each carry both
twins (`crates/jbotci-syntax/src/grammar/baseline_relative.rs`), and both twins are fail-closed.
At the universally quantified sites the recovered side is `valid(c).is_some_and(..)` inside
`.all(..)`, never `filter_map`, so an unparsed child can never be silently dropped out of the
test. At the one site that is not a universal quantification — `returns_to_baseline`, whose
answer is a three-fact composition rather than an `.all` — fail-closedness is carried by
`RelativeBodyShape`, which represents "known non-subbridi" and "did not parse" as different
values. **Round 2 corrects this too**: the blanket `.all`-shaped claim was false for that
ternary. See [round 2, M4](#m4-the-recovered-return-was-not-fail-closed).

The pre-selbri consumers are witnessed, because with the return moved they are load-bearing
evidence for the mechanism rather than incidental coverage:

| surface | consumer | result | witness |
| --- | --- | --- | --- |
| `doi ko'a po'oi mi brode` | a vocative relative slot (S2) | A, `experimental-zantufa-statement-relative-clause`; tree pins `ZantufaStatementRelativeClause(ZantufaRestrictiveStatementRelativeClause(..))` | `d1-preselbri-vocative-pohoi-elided` |
| `ko'a no'oi mi brode broda` | `simple_sumti` (S1) | A, same warning; tree pins the Zantufa incidental arm | `d1-preselbri-s1-nohoi-elided` |
| `lo broda po'oi mi brode cu brodi` | a description tail (S2), post-selbri | A, `experimental-nohoi-selbri-relative-clause` — the same elided extent, at a position exp *does* reach, is exp's under R2 | `d2-exp-s2-pohoi-elided` |

The first two are elided-terminator bodies camxes-exp could form; the third is the same shape at a
position exp reaches. Under the frozen candidate-local return the first two would have been
declined with nothing behind them. The third shows the return still happening where it should.

One correction to the ASK, measured rather than assumed. The surface named in the lead's rider,
`lo po'oi mi brode broda`, does accept at the default profile and does warn, but it is **not**
relative-owned: `po'oi` is classified LAhE as well as NOI (`cmavo.rs:640`, matching rolling
Zantufa's own pair), and at a description's leading tail the LAhE reading takes the prefix before
the relative-clause field is reached. Its pinned tree is `LaheSumti`, and its one diagnostic is
`experimental-zantufa-cmavo`, not a relative-clause warning. It is pinned anyway
(`d1-preselbri-pohoi-lahe`) because the boundary is real and no cell covered it; the
load-bearing pre-selbri evidence is the `doi ko'a po'oi mi brode` row above.

## D3: the adverbial trio

Three sources spell an adverbial at the term level and they do not agree:

| source | selma'o | body | terminator | positions |
| --- | --- | --- | --- | --- |
| camxes-exp | SOI = `soi / xoi / fi'oi` (:1842) | subsentence | SEhU | `tag_term` (:149), `abs_tag_term` (:160) |
| rolling Zantufa | XOI = `xoi / fi'oi` (:615) | statement | SEhU | `term_2` (:29) |
| New-FIhOI proposal | FIhOI = `ku'au / fi'oi` (selpahi-mex.peg:1993) | subsentence | FIhAU | term |

The arms are keyed on the exact cmavo rather than on one widened selma'o, so no morphology moves
and both of `xoi`'s source classifications survive. The shape the base carried — a statement body
closed by FIhAU — is the Cartesian product of two of them and is in none, so it retires and its
unit pin splits. `ku'au` is a retained source gap.

Arm order carries the boundaries. The proposal arm requires an explicit FIhAU and so is
structurally disjoint and runs first; the elided-terminator extent it shares with camxes-exp is
camxes-exp's under R2. The Zantufa arm runs before the camxes-exp one because its body is the
wider of the two — the shorter reading would otherwise succeed and leave the rest of an
I-connected body behind — and its classifier hands back everything camxes-exp can form, so it
keeps only the statement-width extents.

Warnings are marker-anchored and neutral, as the reconciliation requires: `soi` and `xoi` carry
`ExperimentalSoiAdverbial` on every arm, `fi'oi` carries `ExperimentalFihoiAdverbial` on every
arm, and no shared extent gets a Zantufa-attributed name.

### R1's two halves

`mi broda soi mi brode` is accepted by all three reference parsers and camxes-standard reads it
as the reciprocal `soi mi` with `brode` continuing the tanru outside. Keeping it there takes two
reservations, because the reciprocal is a free modifier and attaches inside `.wf()` before any
term-level arm is reached.

The adverbial arm returns any completed candidate that reparses as the reciprocal plus a tail:
marker `soi` — `xoi` and `fi'oi` are in no reciprocal — an elided SEhU, so the extent has no
terminator of its own to keep it whole, and a body opening with the term run the reciprocal would
take as its first sumti. And the reciprocal declines wherever that arm, classifier included,
would own the extent. So `mi broda soi mi brode` is the reciprocal's and silent, while
`mi broda soi mi brode se'u` is the adverbial's and warned.

`mi klama soi do se'u lo zdani` needs neither: camxes-exp rejects it because `do` alone is not a
subsentence, so nothing disputes the extent.

### Positions

All three arms occupy all nine leaf inventories the two they replace occupied — term, CEhE,
loose, nonabs, simple, bound, normal, bound-normal and normal-atom — a uniform 3x9 placement.
The sources' term-level constructs map across jbotci's composed term hierarchy, and non-uniform
placement would re-open the epoch-6 flavour axis for no sourced reason. Witnesses pin each arm at
a non-`term` leaf.

The downstream consumer sweep follows the same three-for-two substitution: `generated_term_view.rs`,
`baseline_bo.rs`, `baseline_termset.rs`, the semantic walkers and reference builders, the
enum-invariant audit rows, the recovery anchor metadata snapshot, and the generated Python model,
stubs and API-parity inventory.

### The SEI gap

Rolling Zantufa also reads bare `soi` as SEI, a statement free modifier (:598, :75). That family
is out of this epoch's scope and no issue covers it, so it is a documented gap: the surface
`soi mi brode se'u mi klama` is the camxes-exp adverbial's and warns as one, and the existing SEI
arm takes no delta in either profile.

## The cell table as measured

Every row of the plan's 30-row table measures as frozen except the three S3-parent rows recorded
above. The witnesses are `tests/fixtures/adhoc/syntax/subsentence/`, tagged `subsentence-epoch`,
one per cell plus the body admission set, the chain forms, the 3x9 uniformity pins, the SA-shaped
negatives, the SEI no-delta pins, both documented gaps, and the five witnesses the four plan-v5
deltas add: the three pre-selbri consumers behind delta 1
(`d1-preselbri-vocative-pohoi-elided`, `d1-preselbri-s1-nohoi-elided`, `d1-preselbri-pohoi-lahe`),
the gek gap's rejection side (`d1-gap-gek-i-connected-branch`) and the third moved cell's default
union side (`d1-s3-gap-zihe-default`). Every witness pins its diagnostics, empty where the
expectation is silence.

## Consolidated expectations

The regeneration is one `fixture-rewrite` pass over the whole tree, classified by
`tools/compare-subsentence-expectations.py` against a `git archive` of `tests/fixtures` at the
epoch base. The classifier reads the OLD tree and rewrites it with the shapes this epoch
approves; the rewritten old tree must then equal the new one structurally, so nothing is
inferred from the new tree and an ownership change cannot be laundered as a re-typing. Its
transcriptions are re-derived from the grammar at both commits by
`tools/tests/test_compare_subsentence_expectations.py` rather than asserted, because a class
keyed on a field tuple the baseline never had fails open: it simply never fires, and the
fixtures it should have classified land in residue looking like ordinary population.

122 changed pre-epoch fixtures. 93 classify:

| class | count | what it is |
| --- | --- | --- |
| `rejection-diagnostic-reclassification` | 86 | a surface that rejected before and rejects now, whose error frontier moved because the epoch adds arms at the position it fails in |
| `soi-adverbial-arm-split` | 6 | the SOI arm's split, with its body re-typed from a statement to a subsentence |
| `fihoi-adverbial-arm-split` | 1 | the FIhOI arm's split, on an explicit FIhAU |
| `zantufa-statement-relative-wrapper` | 0 | the sum the two Zantufa arms move inside |
| `relative-statement-body` | 0 | those arms' body, re-typed onto the tailored family |

The two zeroes are the measurement rather than an omission: every pre-epoch fixture that reached
a Zantufa statement relative arm reached it over a baseline marker and a subbridi-shaped body, so
the site classifier returns all of them and each is an owner change with its own disposition.
Nothing in the pre-epoch corpus carried a Zantufa-only body at one of those positions.

29 are manual residue in four populations, none of them a class:

| population | count | disposition |
| --- | --- | --- |
| the site classifier returns a baseline marker over a subbridi body to the baseline arm | 25 | R1. All 25 stay accepted and lose exactly the `experimental-zantufa-statement-relative-clause` warning they carried, one to three each; 24 are Zantufa-profile fixtures from issue #828's own boundary set plus `full-alice`, which pins no diagnostics |
| the `po'oi` leak closes | 1 | `exp-pooi-follower`, a default-profile fixture whose silently accepted `po'oi` continuation is now the Zantufa arm's and warns. This is the frozen silent-to-warned flip |
| the elided-FIhAU extent moves to the camxes-exp arm | 2 | R2. `mi broda fi'oi mi brode` is derivable by both the proposal and camxes-exp, and the adopted source takes precedence |
| `no'oi` comes back to life | 1 | the one corpus fixture that used the retired indicator reading, now the camxes-exp tanru-unit relative it always was |

89 epoch-new witnesses are authored rather than classified, and every one pins its diagnostics
(72 at the round-1 submission; round 2 retires one and adds eighteen). The xfail count is
unchanged at 519.

## The gate

Run at the submission tree with `/build/jbotci/logs/epoch08-gate.sh`, sequentially, with the
peak-RSS pair last and alone.

| component | result | log |
| --- | --- | --- |
| `cargo fmt --all --check` | clean | `epoch08-g-fmt.log` |
| `cargo test -r --workspace --features jbotci-dictionary/import --no-fail-fast` | 103 targets, 1,650 passed, 0 failed, 16 ignored | `epoch08-g-workspace.log` |
| `cargo test -r --workspace --all-targets --features expensive_contracts --no-fail-fast` | 70 targets, 1,649 passed, 0 failed | `epoch08-g-expensive.log` |
| `fixture-test --profile all` | 26,645 fixtures, 72,591 passed, 519 xfailed, 0 failed | `epoch08-rss-head.log` |
| tagged facet `subsentence-epoch` | 72 fixtures, 3 facets, 72 passed, 0 failed | `epoch08-g-tagged-facet.log` |
| frozen syntax facet, same tag | 72 fixtures, 72 passed, 0 failed | `epoch08-g-frozen-facet.log` |
| comparer | 122 changed / 86 + 6 + 1 + 0 + 0 mechanical / 29 manual / 0 prose / 72 epoch-new / 0 unpaired / 0 witnesses missing diagnostics | `epoch08-g4-comparer.log` |
| comparer unit tests | 27 tests, green | `epoch08-g4-comparer-test.log` |
| `cargo build -p jbotci` (debug) | green | `epoch08-g-debug-jbotci.log` |
| `dx build` | green | `epoch08-g-dx.log` |
| `maturin develop` | green | `epoch08-g3-maturin.log` |
| `generate_syntax_models.py --check` | green after regeneration | `epoch08-g3-generate_syntax_models.log` |
| `generate_domain_enum_stubs.py --check` | green | `epoch08-g3-generate_domain_enum_stubs.log` |
| `compose_stubs.py --check` | green after regeneration | `epoch08-g3-compose_stubs.log` |
| `generate_api_matrix.py --check` | green after regeneration | `epoch08-g3-generate_api_matrix.log` |
| peak RSS, full profile | base 5,851,452 KB -> 6,299,132 KB, **+7.65%** (gate: base +20%) | `epoch08-rss-base.log`, `epoch08-rss-head.log` |

Both `cargo test` components run `--no-fail-fast` deliberately. `cargo test` abandons the run at
the first failing target, so a red gate without it reports a lower bound on the failing set
rather than the set itself.

Three rows were red on the first pass and are recorded as such. `generate_syntax_models.py
--check`, `compose_stubs.py --check` and `generate_api_matrix.py --check` reported the four
generated Python model files stale and 31 non-resolving public Python paths -- every one of them
this epoch's own new walk functions (`exp_relative_tanru_unit`, the three adverbial arms and
their nine leaf placements, the `zantufa_relative_statement` family, the
`zantufa_statement_relative_clause` sum, `zantufa_kuho_terminated_statement_relative_clause`,
`zihe_selbri_relative_connective`). Running the four generators in order regenerated
`strict.py`, `strict.pyi`, `recovered.py`, `recovered.pyi` and `docs/api-parity.tsv`; the
regenerated diff adds and removes exactly this epoch's node inventory -- the SOI/FIhOI pair
replaced by the three-way split at all nine leaves, the camxes-exp tanru-unit relative family, and
the Zantufa statement-relative wrapper family -- and every classification resolves without a
manual row. The four checks are green at the submitted tree.

There is no `semantics-coverage` row. That xtask subcommand was removed by `4b7b4cb36b`
(*Retire the legacy tersmu implementation and every surface it reached*, #869), which is an
ancestor of this epoch's base; the gate script's call to it returns `unrecognized subcommand`
and the row is retired rather than failed.

The peak-RSS pair is measured twice and the second pair is the one reported. The first pair was
taken while the Python generators were running on the same box; peak RSS is the maximum resident
set of the measured process tree and cannot be raised by an unrelated process, and the two pairs
agree to within 0.02% (5,852,004 -> 6,300,124 KB against 5,851,452 -> 6,299,132 KB), but the
reported pair is the one where nothing else was running. Both sides build `xtask-full` **outside**
the timed window, back to back, on one volume, so no rustc peak is folded into either figure --
the epoch-7 failure mode this pattern exists to avoid.

The row ordering is recorded rather than glossed. Every row above except `cargo fmt`, the comparer
and its unit tests was produced before this gate section was written; those three were re-run at
the final commit, whose only delta from the gated tree is this file. No Rust source, fixture or
generated artefact differs between the two, and nothing in the remaining rows reads `docs/`.


## Round 2: the PR #876 review corrections

The round-1 submission `dc8cb21faf` drew a CHANGES verdict from Sol and Qwen 3.8-max with the
lead. Six findings and two cosmetics were returned; everything else in the epoch verified clean on
both reports and is untouched. Each finding below names the correction and the measurement behind
it. Every before/after figure is measured against three release binaries — the epoch base
`0d791fd35c`, the round-1 submission `dc8cb21faf`, and this head — because several of these
findings are round-1 regressions against the base rather than long-standing gaps, and only the
three-column form says which. `A` is a successful parse and `R` a rejection.

**Round 2 introduces no `A -> R` against the epoch base.** The consolidated picture, which is
the single strongest piece of evidence this round produced:

| surface | base | round 1 | head | what it is |
| --- | --- | --- | --- | --- |
| `lo broda po'oi to do brodi toi mi brode ku'o cu brodi` | A | **R** | A | round-1 regression, removed (H3) |
| `mi broda soi na ku brode` | A | **R** | A | round-1 regression, removed (M5) |
| `mi broda soi fi do brode` | A | **R** | A | round-1 regression, removed (M5) |
| `broda po'oi mi brode je po'oi do brodi`, `(+zantufa-terms)` | A | **R** | A | round-1 regression, removed (H1) |
| `broda no'oi mi brode ga'o joi no'oi do brodi`, `(+zt +zc)` | R | **A** | R | round-1 over-acceptance, withdrawn (H1) |
| `broda po'oi mi brode zi'e poi do brodi`, `(+zantufa-terms)` | A | **R** | **R** | round-1 regression, NOT fixed — issue #877 |
| `broda po'oi mi brode je poi do brodi`, `(+zantufa-terms)` | A | **R** | **R** | the same, joik-joined — issue #877 |

Three of the four removed regressions were found by neither reviewer; they fell out of fixing
what the reviewers did find. The only `A -> R` still standing against the base is the S3
leading-selbri class in the last two rows, which round 1 introduced, round 2 does not fix, and
issue #877 now owns.

### H1: the D2 chain connective was not the source's `joik`

Both reviewers found the S3 list classifier handing camxes-exp a list camxes-exp could not
derive. They proposed opposite corrections, and the lead's verdict asked for the source to be
measured. It was, and it settles the disagreement in Sol's favour:

| what | where |
| --- | --- |
| `relative_clauses <- relative_clause ((ZIhE_clause / joik) free* relative_clause)*` | camxes-exp.peg:199 |
| `selbri_relative_clauses <- selbri_relative_clause ((ZIhE_clause / joik) free* selbri_relative_clause)*` | camxes-exp.peg:214 |
| `#// EXP-MODIF: A-JA-JOI merge` | camxes-exp.peg:346 |
| `joik <- NA_clause? SE_clause? (JOI_clause / JA_clause / A_clause) NAI_clause? / interval / GAhO_clause interval GAhO_clause` | camxes-exp.peg:347 |
| `interval <- SE_clause? BIhI_clause NAI_clause?` | camxes-exp.peg:349 |
| `relative_clauses <- (relative_clause (joik? relative_clause)*)`, `selbri_1 <- ((...) relative_clauses? (CEI_clause selbri)*)`, `JOI <- ... / je / ja / ...` | zantufa-1.9999.peg:42, :45, :556 |

So JA and A **are** in camxes-exp's `joik`, the two relative chains name the **same** `joik`, and
rolling Zantufa spells the chain too. `broda po'oi mi brode je po'oi do brodi` is therefore
R / A / A, and R2 — adopted camxes-exp owns shared extension extents — gives it to camxes-exp in
**both** profiles. Qwen's proposed narrowing (constrain the classifier to the JOI family) would
have inverted that rule, and the round-1 pin `d2-reject-jek-chain` recorded the cell as a
rejection on the same mistaken premise. Lead ANSWER 2026-08-31T15:26:52Z accepts the source
reading over the verdict's own witness spec.

The correction is in the grammar, not the classifier:

```
rule "relative clause connective" exp_selbri_relative_clause_connective -> enum {
    zihe_selbri_relative_connective,   // the source's ZIhE_clause
    exp_relative_clause_connective,    // NA? SE? (JOI / JA / A) NAI?, shared with :199
    simple_interval_connective,        // SE? BIhI NAI?
    closed_interval_connective,        // GAhO interval GAhO
}
```

All four arms already existed; none is a new transcription. The first alternative is the exact
node the ordinary relative chain uses for the same source `joik`, and the two interval arms are
jbotci's own, previously reached through `joik_connective`. What is gone is `joik_connective`
itself, which was never this language: narrower, because jbotci splits camxes-exp's merged
inventory across `joik_connective`, `jek_connective` and `ek_connective`, and wider, because
three of its arms are `ZantufaConnectives`-gated rolling-Zantufa shapes camxes-exp does not
spell.

With the two chains now holding the same connective nodes, the continuation classifiers need no
narrowing at all: every connective an S3 list can present is one the D2 chain consumes, which is
what `is_exp_selbri_relative_continuation`'s doc comment already promised and now proves. Both
twins keep their `.all(|c| valid(c).is_some_and(..))` shape.

Measured over the whole connective inventory, default profile except where marked:

| surface | base | round 1 | head |
| --- | --- | --- | --- |
| `broda no'oi mi brode je no'oi do brodi` | R | R | A |
| `broda no'oi mi brode .a no'oi do brodi` | R | R | A |
| `broda no'oi mi brode na je nai no'oi do brodi` | R | R | A |
| `broda no'oi mi brode se je no'oi do brodi` | R | R | A |
| `broda no'oi mi brode joi no'oi do brodi` | R | A | A |
| `broda no'oi mi brode zi'e no'oi do brodi` | R | A | A |
| `broda no'oi mi brode bi'i no'oi do brodi` | R | A | A |
| `broda no'oi mi brode ga'o bi'i ga'o no'oi do brodi` | R | A | A |
| `broda po'oi mi brode je po'oi do brodi`, `(+zantufa-terms)` | A | R | A |
| `broda poi mi brode je po'oi do brodi`, `(+zantufa-terms)` | A | A | A |
| `broda no'oi mi brode ga'o joi no'oi do brodi`, `(+zantufa-terms +zantufa-connectives)` | R | A | R |

The interval rows are why this correction is four arms and not one. Reusing
`exp_relative_clause_connective` alone would have taken `bi'i` and `ga'o bi'i ga'o` from `A` back
to `R`, because round 1's `joik_connective` was carrying the interval arms; the source has them
(`:347`, `:349`) and so does the corrected enum.

The last row is the one deliberate narrowing, and it is what "excluding unrelated feature-gated
alternatives" means in practice: `GAhO JOI` is rolling Zantufa's connective, not one of
camxes-exp's three `joik` alternatives, and it reached the adopted chain in round 1 only because
that chain borrowed jbotci's `joik_connective`. The epoch base rejected it in every profile, the
ordinary relative chain at :199 always has, and the two chains now agree again. Pinned as
`d2-gap-zantufa-gaho-joik-chain`.

Witnesses: `d2-chain-jek` (the flipped cell, default profile, exp-owned),
`d2-chain-jek-zantufa` (the `(+zantufa-terms)` twin, proving the owner does not move),
`d2-chain-a-connective`, `d2-chain-negated-connective`, `d2-chain-interval`,
`d2-chain-gaho-interval`, `d2-gap-zantufa-gaho-joik-chain`, and `d1-s3-jek-mixed-list` — the
whole-list return exercised against the JA family, where the list is joined by a connective
camxes-exp spells but its first clause is a baseline `poi` the exp relative cannot form, so the
list stays at the S3 parent. `d2-chain-joik` re-pins onto the shared connective node and gains
the `experimental-relative-clause-connective` warning it always should have carried. The old
`d2-reject-jek-chain` is retired.

Retained source gaps, unchanged by this round: camxes-exp's `gek selbri_relative_clauses gik
selbri_relative_clauses` forethought half still uses jbotci's `modal_forethought_connective`
rather than camxes-exp's own `gek` (:361). Sol raised it; the lead's round-2 scope excludes it;
it is recorded here so the next epoch does not have to rediscover it.

### The S3 leading-selbri gap (issue #877)

The lead required this measured and pinned rather than left implicit. The S3 parent is

```
rule "Zantufa relative selbri" zantufa_relative_selbri(...) -> struct {
    field leading_selbri <- arc(cei_free_co_selbri);
    field relative_clauses <- arc(selbri_relative_clause_list);
    ...
}
```

and the level-2 selbri ladder that `cei_free_co_selbri` reaches contains
`exp_relative_tanru_unit`. So when the leading selbri is followed by a NOhOI clause, D2 takes
that clause as part of the leading selbri, `selbri_relative_clause_list` has to start at the
connective, and the S3 whole-list return never runs. The class this loses is a list whose FIRST
clause camxes-exp can form but whose list as a whole it cannot:

| surface, `(+zantufa-terms)` | base | round 1 | head |
| --- | --- | --- | --- |
| `broda po'oi mi brode zi'e poi do brodi` | A | R | R |
| `broda po'oi mi brode je poi do brodi` | A | R | R |
| `broda po'oi mi brode ku'o zi'e poi do brodi` | A | A | A |
| `broda poi mi brode je po'oi do brodi` | A | A | A |

The last two rows are the boundary: an explicit `ku'o` on the first clause fires the D2 KUhO
reservation, and a first clause camxes-exp cannot form at all keeps D2 out of the leading selbri;
either way the leading selbri stays bare and the list reaches S3 intact. The fix for the other
two is the same second boundary ladder [Delta 4](#the-retained-gated-omission-plan-v5-delta-4)
declined to build — follow the no-terminal-relative entry down from selbri level 2 so the
leading selbri cannot end in a relative of its own. It is filed as **issue #877**.

**The base accepts both, so this is a regression the epoch introduced, not a pre-existing
limitation.** That is the whole reason it is a gap row rather than a footnote, and it is stated
plainly here because the two readings call for different follow-up: a pre-existing limitation
could wait for whichever epoch reaches that ladder, while a regression is a debt this epoch
created. Round 2 does not repay it — the ladder is out of this round's scope — but it is pinned
on both shapes rather than argued: `d1-s3-gap-leading-selbri-zihe-mixed-list` for the ZIhE-joined
form and `d1-s3-gap-leading-selbri-jek-mixed-list` for the joik-joined one, one witness per
shape at the lead's instruction.

### H2: the S1/S2 return fired without a baseline owner

The frozen S1/S2 rule permits a return only for a baseline marker **and** a subbridi-compatible
body. `returns_to_baseline`'s longer-extent half tested only the elided terminator, so a
statement-width body under `po'oi`, `voi'i` or `no'oi` was declined — and those markers have no
baseline arm at all, so the extent went nowhere. Both halves now require the marker, and the
marker they require is the EXACT arm the candidate would reparse through rather than the union
of the two: `poi`/`voi` for `restrictive_bridi_relative_clause` (camxes.peg:1695), `noi` for
`incidental_bridi_relative_clause`.

The change is monotone: it only ever removes a return, and a return that does not fire leaves
the extent on the warned Zantufa arm, so nothing that parsed before stops parsing.

| surface | base | round 1 | head |
| --- | --- | --- | --- |
| `lo broda voi'i mi brode ije do brodi cu brodi` | R | R | A, `experimental-zantufa-statement-relative-clause` |
| `lo broda no'oi tu'e mi brode tu'u cu brodi` | R | R | A, same warning |
| `lo broda poi mi brode ku'o cu brodi` | A, silent | A, silent | A, silent (R1 unchanged) |

Witnesses `d1-zantufa-s2-voihi-elided-ije` and `d1-zantufa-s2-nohoi-elided-tuhe`. The retained
longer-extent half is unchanged in what it does for baseline markers, and its reason is
unchanged: a statement-width body with the KUhO elided would swallow the paragraph's own `.ije`,
which thirteen corpus fixtures depend on not happening.

### H3: the KUhO preemption was not the language it reserves

Zantufa's `NOI_clause` carries `post_clause`, whose `free*` belongs to the marker
(zantufa-1.9999.peg:325, :82), and the owning arms spell it `.wf()`. The negative assertion's
marker did not, so a free modifier after NOhOI made the reservation fail while D2's own marker
consumed it — the prefix-steal the reservation exists to prevent, happening exactly where the two
languages differed. The assertion's marker is now `.wf()`. The tailored body's `i` had the same
omission against Zantufa's `I_clause` (:217) and is now `.wf()` too.

The terminator is deliberately NOT `.wf()` in the assertion, and that is a measured decision
rather than an oversight: the reservation is a boolean, so free modifiers after `ku'o` cannot
change its answer, while probing an empty `free*` at end of input moves the recorded failure
frontier onto that probe. With `cmavo(Kuho).wf()` there, `d1-s3-gap-kuho-default`'s rejection
diagnostic degraded from `syntax.unexpected-cmavo` at `ku'o` to `syntax.incomplete-free-modifier`
at a zero-width span at EOF.

| surface | base | round 1 | head |
| --- | --- | --- | --- |
| `lo broda po'oi to do brodi toi mi brode ku'o cu brodi` | A | R | A, Zantufa-owned |
| `lo broda no'oi mi brode i to ri brodi toi je do brodi ku'o cu brodi` | R | R | A, Zantufa-owned |
| `lo broda po'oi mi brode ku'o to do brodi toi cu brodi` | A | A | A (the post-terminator boundary, unchanged) |

The first row is a round-1 regression against the base, not a new surface: the prefix-steal
Sol described was already costing an extent the epoch base accepted.

Witnesses `d2-kuho-reservation-free-modifier`, `d1-zantufa-s2-i-free-modifier-refs` and
`d1-zantufa-s2-kuho-free-modifier`. The `i` re-typing moves eight existing witnesses'
`i: Plain(..)` to `i: WithFreeModifiers { value: Plain(..), free_modifiers: [] }` and changes
nothing else in them.

### M4: the recovered return was not fail-closed

An invalid recovered body made `subbridi_body` false, and the longer-extent half then returned
the candidate on the elided terminator alone — a candidate that did not parse, handed to a
baseline arm that must reparse it. The two facts are now distinct values rather than one boolean:

```rust
enum RelativeBodyShape { Subbridi, StatementWidth, Unproven }
```

`Unproven` never returns, and `returns_to_baseline` carries
`#[ensures(!ret || baseline_marker)]`. The connected arm is the one shape read off the variant
tag rather than the payload, because being an I-connection is what that variant IS. Direct
recovered-classifier tests are in `baseline_relative.rs`'s own test module:
`returns_to_baseline_needs_every_fact_proven` (the whole 2x2x3 table),
`recovered_body_shape_is_unproven_when_the_body_did_not_parse`, and
`recovered_baseline_statement_relative_returns_only_proven_baseline_extents`, which builds
recovered clauses directly and covers the positive return, both extension-only markers, an
unparsed body and an unparsed clause.

### M5: the R1 no-steal proved the wrong constituent

`soi_free_modifier` spells `SOI free* sumti sumti? SEhU_elidible`, so the reparse the adverbial
arm returns needs a bare `sumti` in first position. The classifier tested only that the body was
a `BridiWithLeadingTerms`, whose first component is an arbitrary `term` — which also covers
tagged sumti, termsets, `na ku` and the adverbials themselves. It now proves `TermSyntax`'s one
arm that is exactly `sumti`.

| surface | base | round 1 | head |
| --- | --- | --- | --- |
| `mi broda soi mi brode` | A, silent baseline reciprocal | A | A, unchanged |
| `mi broda soi na ku brode` | A | R | A, `experimental-soi-adverbial` |
| `mi broda soi fi do brode` | A | R | A, same |

Both non-sumti-leading rows are round-1 regressions against the base: with the over-wide
classifier the adverbial arm declined and the reciprocal could not stand in, so the surface was
lost entirely.

Witness `d3-exp-soi-na-ku-body`, whose pinned tree is `ExpSoiAdverbialTerm` over a body whose
first leading term is `NaKuTerm`.

### M6: the reference visitor did not mirror the ordinary one

Two gaps, both now closed. The Zantufa relative statement visitor walked only each continuation's
trailing statement, skipping the `i` and the connective, so references inside them were never
reached; it now walks the whole continuation node, as `visit_statement` does. And its prenex arm
bound relation variables but not CEI predicate targets, so a CEI assigned in the body's own
prenex never got its `PrenexCeiAssignment` edge to the body's main predicate; it now calls
`bind_prenex_cei_predicate_targets_for_zantufa_relative_statement` over a
`zantufa_relative_statement_main_predicate_id` resolver that mirrors
`statement_main_predicate_id` exactly, with the same save/restore of `cei_bridi_bindings`.

Both witnesses pin `expectations.semantics.refs`, and both were verified non-vacuous by reverting
the two edits, rebuilding and re-running them:

| witness | references with the fix reverted | references at head |
| --- | --- | --- |
| `d1-zantufa-s2-i-free-modifier-refs` | `[]` | one `ri` edge at the `ri` inside the continuation's free modifier |
| `d1-zantufa-s2-prenex-cei-refs` | one `pro-bridi-assignment` edge, to the OUTER bridi | that edge plus the `PrenexCeiAssignment` edge to the relative body's own bridi |

The second row is the ordinary visitor's shape exactly: `visit_statement_base`'s prenex arm walks
the terms first, which is where the outer edge comes from, and then binds the prenex CEI targets.

### Cosmetics

`d1-gap-gek-i-connected-branch` keeps its default-profile pin and gains the
`(+zantufa-terms)` twin `d1-gap-gek-i-connected-branch-zantufa`, so the ledger's "no route in
either profile" is pinned on both sides rather than argued. That is Qwen's actual suggestion; a
`dialect` line on the existing file would have moved what it pins rather than adding to it.

`d1-zantufa-s2-voihi-kuho.toml` is **not** renamed, and the cell reference is correct as it
stands. `voihi` is this repository's h-for-apostrophe spelling of `voi'i` — the same convention
its siblings `d1-zantufa-s2-nohoi-kuho` (`no'oi`) and `d1-zantufa-s2-pohoi-kuho` (`po'oi`) use,
and the one `Cmavo::Voihi` itself uses. There is no separate `voihi` cmavo to confuse it with.

### The round-2 gate

Run at `691a9ec38f` with `/build/jbotci/logs/epoch08-r2-gate.sh`, sequentially. Both `cargo test`
components use `--no-fail-fast` for the same reason they did in round 1: without it a red gate
reports a lower bound on the failing set rather than the set itself.

| component | result | log |
| --- | --- | --- |
| `cargo fmt --all --check` | clean | `epoch08-r2-g6-fmt.log` |
| `cargo test -r --workspace --features jbotci-dictionary/import --no-fail-fast` | 103 targets, 1,653 passed, 0 failed, 16 ignored | `epoch08-r2-g-workspace.log` |
| `cargo test -r --workspace --all-targets --features expensive_contracts --no-fail-fast` | 70 targets, 1,652 passed, 0 failed | `epoch08-r2-g2-expensive.log` |
| `fixture-test --profile all` | 26,662 fixtures, 72,610 passed, 519 xfailed, 0 failed | `epoch08-r2-g6-fixtures-all.log` |
| tagged facet `subsentence-epoch` | 89 fixtures, 3 facets, 91 passed, 0 failed | `epoch08-r2-g6-tagged-facet.log` |
| frozen syntax facet, same tag | 89 fixtures, 89 passed, 0 failed | `epoch08-r2-g6-frozen-facet.log` |
| comparer | 122 changed / 86 + 6 + 1 + 0 + 0 mechanical / 29 manual / 0 prose / 89 epoch-new / 0 unpaired / 0 witness deltas / 0 witnesses missing diagnostics | `epoch08-r2-g6-comparer.log` |
| comparer unit tests | 27 tests, green | `epoch08-r2-g6-comparer-test.log` |
| `cargo build -p jbotci` (debug) | green | `epoch08-r2-g-debug-jbotci.log` |
| `dx build` | green | `epoch08-r2-g-dx.log` |
| `maturin develop` | green | `epoch08-r2-g-maturin.log` |
| `generate_syntax_models.py --check` | green | `epoch08-r2-g-generate_syntax_models.log` |
| `generate_domain_enum_stubs.py --check` | green | `epoch08-r2-g-generate_domain_enum_stubs.log` |
| `compose_stubs.py --check` | green | `epoch08-r2-g-compose_stubs.log` |
| `generate_api_matrix.py --check` | green | `epoch08-r2-g-generate_api_matrix.log` |
| peak RSS | not re-measured | — |

The tagged row reads 91 passed over 89 fixtures because the two reference witnesses carry a
`semantics-refs` facet as well as a syntax one. The pre-epoch comparer figures are byte-identical
to round 1's; only the epoch-new witness count moves, 72 to 89.

The four fixture-facing rows and `fmt` were re-run after the lead's ACK added the second #877
witness; the Rust, product-build and binding rows are unchanged from the `691a9ec38f` run,
because a fixture, a documentation section and one Python constant cannot reach them.

Two rows carry a note rather than a bare figure.

The peak-RSS pair is deliberately not re-measured. The lead's round-2 instruction ties it to
adding a production, and this round adds none: `exp_selbri_relative_clause_connective` gains two
arms that name rules the grammar already had, and two existing fields gain `.wf()`. No new rule,
no new node type -- `WithFreeModifiers` already wraps dozens of fields.

The expensive-contracts row is reported from a standalone re-run at the same commit
(`epoch08-r2-g2-expensive.log`) rather than from the gate's own `epoch08-r2-g-expensive.log`. The
gate's copy reported `exit=0` and no failing test, but the file itself is not trustworthy as a
record: an earlier aborted gate attempt left a `cargo test` process alive whose own output landed
in that path after the second attempt had truncated it, so the file mixes two runs and ends in
the aborted run's tail. Nothing about the result changes -- the standalone re-run is 70 targets,
1,652 passed, 0 failed -- but the log a reviewer would open had to be one run.

Three generated artefacts were regenerated during this round rather than found stale at the gate:
the recovery anchor metadata snapshot and the four Python model files plus `docs/api-parity.tsv`.
Their deltas are the D2 connective arm change and the two `.wf()` fields exactly, and they are
committed with that reasoning in `691a9ec38f`.


## Round 3: the #877 regression and two source-exactness corrections

The round-2 verdict at `2284b50691` was split -- Sol CHANGES, Qwen PASS -- and the lead upheld
Sol on the one High: the S3 mixed-list regression may not ship. Both reviewers confirmed H2, H3,
M4, M5, M6, both cosmetics and the H1 grammar direction, and this round does not touch them.

**Round 3 introduces no `A -> R` against the epoch base, and removes the last one that stood.**
The epoch's consolidated picture is now:

| surface | base | round 1 | round 2 | head | what it is |
| --- | --- | --- | --- | --- | --- |
| `broda po'oi mi brode zi'e poi do brodi`, `(+zt)` | A | **R** | **R** | A | #877, repaid |
| `broda po'oi mi brode je poi do brodi`, `(+zt)` | A | **R** | **R** | A | the same, joik-joined |
| `lo broda po'oi mi brode zi'e poi do brodi ku cu brodi`, `(+zt)` | A | **R** | **R** | A | the same class at a description site, found this round |
| `broda po'oi mi brode ku'o zi'e poi do brodi`, `(+zt)` | A | A | A | A | boundary: explicit `ku'o` |
| `broda poi mi brode je po'oi do brodi`, `(+zt)` | A | A | A | A | boundary: clause 1 is one exp cannot form |
| `lo broda no'oi mi brode zi'e poi do brodi ku cu brodi` | R | R | R | **A** | new accept: the default-profile description site |
| `lo broda no'oi mi brode je poi do brodi ku cu brodi` | R | R | R | **A** | the same, joik-joined |
| `mi viska lo broda no'oi mi brode zi'e poi do brodi` | R | R | R | **A** | the same, elided `ku` |
| `broda no'oi mi brode je to do brodi toi nai no'oi do brodi` | R | R | **A** | R | round-2 over-acceptance, withdrawn |
| `broda no'oi mi brode bi'i to do brodi toi nai no'oi do brodi` | R | R | **A** | R | the same, interval form |
| `lo broda poi mi brode je to do brodi toi nai poi do brodi ku cu brodi` | A | A | A | A | the base-reachable twin, deliberately untouched |
| `li pa bi'i to do brodi toi nai li re` | A | A | A | A | the baseline `joik` twin, deliberately untouched |

`(+zt)` is `(+zantufa-terms)`; every other row is the default profile. Measurements are three
release binaries as before, plus this head; the base binary is `0d791fd35c` built from the
epoch's own base worktree.

### The blocking finding: the S3 leading-selbri regression (issue #877)

The lead asked for measurement in a fixed order: (a) a narrow constraint on where
`exp_relative_tanru_unit` is reachable inside the S3 leading selbri, (b) failing that, the second
no-terminal-relative ladder from selbri level 2 down, (c) failing both, stop and report. **(a)
works, and it is narrower than the lead's own sketch: the fix is not in the leading selbri at
all.**

The loss was diagnosed in round 2 as the leading selbri swallowing clause 1. That is what
happens, but it is not where the defect is. What camxes-exp's chain does when it meets
`po'oi mi brode zi'e poi do brodi` is stop after clause 1, because
`selbri_relative_clause ((ZIhE_clause / joik) free* selbri_relative_clause)*`
(camxes-exp.peg:214) is greedy and `poi` is not a `NOhOI_clause`. It then RETURNS that partial
chain, and the surrounding grammar has no way to consume the `zi'e` -- or the `je` before a
relative marker -- that is left standing. Nothing in camxes-standard, camxes-exp or rolling
Zantufa begins with either. So the parse is already dead at the moment the arm commits, and the
list that the S3 parent, or a description's own relative-clause field, could have owned whole is
gone.

That makes it a **prefix steal**, and this epoch already has the mechanism for one: the KUhO
reservation on `exp_selbri_relative_clause`, which declines wherever a rolling-Zantufa clause
closed by an explicit `ku'o` parses from the same position. The correction is the same shape at
the other end of the same chain -- a boolean probed inside a rewinding lookahead, spelled from
tokens so it can never contribute a node or the connective's own warning:

```
rule "tanru unit" exp_relative_tanru_unit(...) -> struct {
    field leading_unit <- arc(tanru_unit);
    field relative_clauses <- arc(exp_selbri_relative_clauses);
    field bo_tail <- opt(arc(plain_bo_selbri_tail(plain_bo_selbri))).followed_by((
        choice((ZIhE, GAhO SE? BIhI NAI? GAhO, SE? BIhI NAI?, NA? SE? (JOI/JA/A) NAI?)),
        choice((poi, po'oi, voi, voi'i, noi, no'oi)),
    ).not());
}
```

The connective inventory is `joik` as camxes-exp.peg:347-349 spells it plus `ZIhE_clause`,
longest alternative first, and the marker inventory is rolling Zantufa's whole NOI
(zantufa-1.9999.peg:590) rather than only the four the exp clause cannot form. Both choices are
deliberate. Reading the whole connective inventory is why the `je`-joined shape is fixed and not
only the ZIhE one. Reading the whole marker inventory costs nothing because the chain is greedy:
if the next clause were one this route could form, the chain would already have taken it, so the
probe can only fire on a list this route cannot finish.

**This cannot remove an accepted surface, by construction.** The probe fires only when a
relative-clause connective stands in front of a relative marker after the arm's own extent, and
that is a token sequence no production in any of the three grammars can begin. What the arm does
instead is decline, which lets `plain_bo_tanru_unit` return the bare unit and lets the completed
list reach whichever site owns it. The full profile is the oracle and agrees: 26,666 fixtures,
72,614 passed, 519 xfailed, 0 failed, with only the two #877 pins moving.

Three things follow that were not in the round-2 picture.

**The class is wider than the two pinned shapes, and reaches the default profile.** The same
prefix steal happens at a description's own relative-clause field, where there is no S3 parent at
all: `lo broda no'oi mi brode zi'e poi do brodi ku cu brodi` is `R` at the epoch base only because
the base had no adopted exp chain to steal with, and D2 made it `R` for a new reason. It is `A`
now, the whole mixed list owned by the description, the `no'oi` clause warned and the `poi` clause
silent. That is why the reservation is on the exp arm rather than on the S3 parent: the S3 parent
is `ZANTUFA-TERMS`-gated and the defect is not.

**Sol's ladder (b) is not needed.** It would also have worked -- a leading selbri that cannot end
in a relative of its own cannot swallow clause 1 -- but it is a parallel ladder from selbri level
2 down, it only reaches the `(+zantufa-terms)` S3 site, and it would leave the default-profile
description site above still broken. Measurement (a) succeeded, so (b) was not built.

**Delta 4's gated omission is unchanged, and the S3 parent still cannot be default-enabled.**
The lead asked for the answer, not the change, and the answer is no. Delta 4's twenty-four
measured reinterpretations are `zantufa_relative_selbri` claiming a *baseline* `poi` list at
selbri level inside a description whose own relative-clause field is the baseline's site for it --
`.uesai le ni mrilu poi srana la lojban. cu mutce caku`. No camxes-exp tanru-unit relative appears
in any of them, so the reservation never runs, and nothing about the ownership contest between the
S3 parent and the enclosing description has moved. The full profile confirms it: not one of those
twenty-four fixtures changed. Default-enabling the S3 parent still requires the boundary ladder
Delta 4 declined, which is what issue #877's sibling scope was always about.

The reservation sits on the LAST field rather than immediately after the chain, and that is a
measured choice rather than a stylistic one. Both placements were built and run: the two give
identical results on the full profile (26,666 / 72,614 / 0 in both) and on the whole round-3 probe
battery, because a `bo` tail cannot itself be followed by a stranded relative connective in any
accepted surface. What differs is the recovery anchor metadata. `followed_by` has no recovery
classification of its own, so the field it guards becomes opaque and loses its resume rows; on
`relative_clauses` that cost fourteen rows, on `bo_tail` it costs three (`resume 2 origin
FieldFirst start [Cmavo(Bo)]`, once per field). The three are the whole snapshot delta this round.
That `followed_by` degrades anchors at all is pre-existing -- the grammar has eight other
`followed_by` sites with the same silence -- and is left alone here rather than fixed in the macro,
because making it transparent would add anchors at all eight and that is a recovery change no
review round has asked for.

Witnesses: `d1-s3-leading-selbri-zihe-mixed-list` and `d1-s3-leading-selbri-jek-mixed-list` are
the two #877 shapes, now owner/tree/diagnostics success pins showing a bare `leading_selbri` and
the completed mixed list under `ZantufaRelativeSelbri` (they replace the round-2 rejection pins,
whose `-gap-` names are no longer true). `d2-kuho-boundary-mixed-list` pins the explicit-`ku'o`
boundary that already held, so the two routes to the same S3 reading are witnessed side by side.
`d1-s3-jek-mixed-list`, the other boundary -- a clause 1 camxes-exp cannot form at all -- is
retained unchanged from round 2. `d2-mixed-list-description-site` pins the default-profile
description site.

### Sol Medium: the free-modifier placement camxes-exp's `joik` does not spell

Sol is right on the source, and the finding is confirmed at its root rather than at its
citation. camxes-exp puts the chain's frees OUTSIDE the connective -- `(ZIhE_clause / joik) free*`
at both :199 and :214 -- and `joik <- NA_clause? SE_clause? (JOI_clause / JA_clause / A_clause)
NAI_clause? / interval / GAhO_clause interval GAhO_clause` (:347) has no slot inside it. The
`_clause` wrappers do not put one back: `post_clause <- spaces? si_clause? !ZEI_clause !BU_clause
indicators*` (camxes-exp.peg:422) carries **indicators**, not `free*`. So `je to do brodi toi nai`
is not a connective camxes-exp derives, while `je nai to do brodi toi` and `je to do brodi toi`
both are.

jbotci spells that slot on the connective's head instead, before the optional `NAI`. What the
round-2 measurement adds is where that shape lives:

| node | also used by | `<head> free+ NAI` at the epoch base |
| --- | --- | --- |
| `exp_relative_clause_connective` | the ordinary relative chain at :199 | **A** -- `lo broda poi mi brode je to do brodi toi nai poi do brodi ku cu brodi` |
| `simple_interval_connective` | the baseline `joik_connective` | **A** -- `li pa bi'i to do brodi toi nai li re` |
| `closed_interval_connective` | the baseline `joik_connective` | source-exact already: its slot is on the closing GAhO, after the NAI |
| `zihe_selbri_relative_connective` | -- | source-exact already: `ZIhE_clause`, and the trailing frees are the chain's own |

Removing the slot from the two shared nodes would therefore withdraw surfaces the epoch base
accepts through routes this epoch does not own, and the baseline `joik` consumers supply no outer
`free*` at all, so `bi'i to do brodi toi` without a NAI would go with it. That is the unsourced
free-modifier-placement sweep filed as **#847**, whose standing policy is a `prohibited_wf`
warning or an `unrestricted-free` gate rather than a silent narrowing, and it is not this epoch's
family.

What this epoch does own is the chain it added, and there the shape is an epoch-introduced
over-acceptance: both rows above are `R` at the base and became `A` in round 2 when the D2 chain
took the shared connective nodes. So the placement is refused on this chain's completed
continuation --
`ProhibitedRelativeConnectiveFreeModifierRejection`, a typed `OutputRejection` on
`exp_selbri_relative_clause_continuation` that rejects a connective carrying free modifiers before
a present `NAI` -- and left alone everywhere else. The rejection is exhaustive over the four arms
and has the usual fail-closed recovered twin.

The result is exactly camxes-exp's language on this chain: `je to do brodi toi nai` and
`bi'i to do brodi toi nai` are rejections; `je nai to do brodi toi`, `je to do brodi toi`,
`bi'i nai to do brodi toi`, `bi'i to do brodi toi`, `ga'o bi'i nai ga'o to do brodi toi` and
`zi'e to do brodi toi` all still parse. Negative witnesses:
`d2-reject-connective-free-modifier-before-nai` for the merged-head form and
`d2-reject-interval-free-modifier-before-nai` for the interval one. The closed interval needs no
witness of its own because it never had the slot.

**A macro asymmetry was found on the way, and it is worth an issue.** The rejection was first
written on the connective field itself, `field connective <- exp_selbri_relative_clause_connective
.reject_output(R);` -- a postfix method on a bare, parameterless rule reference. The RECOVERED
parser applied it and the STRICT parser did not: `jbotci gentufa`, which parses with recovery,
rejected the surface, while the fixture harness, which uses
`parse_syntax_tree_with_source_and_options`, accepted it and `fixture-rewrite` wrote out a tree
containing the very node the rejection names. Moving the same rejection onto the parameterized
`exp_selbri_relative_clause_continuation(...)` call -- the form every existing `reject_output` in
the grammar uses -- made the two parsers agree. A guard that silently applies in one parser and
not the other is a defect class this epic cannot afford; it is reported to the lead rather than
fixed here, because the fix is in `jbotci-syntax-macros` and belongs to whoever owns that surface.

### Sol Low: the stale `d2-chain-joik` provenance

Corrected. The description said camxes-exp's chain uses "the JOI family and not the JA one",
which the round-2 adjudication had already replaced with the A-JA-JOI merge at :346-347. It now
says the JOI family is one of the three inventories the same node carries, and points at
`d2-chain-jek` and `d2-chain-a-connective` for the other two.

### Issue #877's disposition

The regression is repaid, so #877 closes with this round. Its body should not be taken forward
as written: it names the leading selbri's level-2 ladder as the root cause and says the fix
"requires the second no-terminal-relative ladder from selbri level 2 down". Both halves are
wrong, and the measurement above says why -- the arm that has to decline is the camxes-exp
tanru-unit relative, the class reaches a description site in a profile where the S3 parent does
not exist at all, and the fix is a reservation on that arm rather than a ladder. What #877's body
gets right is that the ladder is still owed: it is Delta 4's, it is the only thing that would let
the S3 parent be default-enabled, and it stays where the ledger already put it, as a retained
gated omission for whichever epoch reaches the descriptor-internal selbri spine.

### The round-3 gate

Run at `f0e609bf0d` with `/build/jbotci/logs/epoch08-r3-gate.sh`, sequentially. Both `cargo test`
components use `--no-fail-fast`, so green means the whole set is green rather than green up to the
first failing target.

| component | result | log |
| --- | --- | --- |
| `cargo fmt --all --check` | clean | `epoch08-r3-g-fmt.log` |
| `cargo test -r --workspace --features jbotci-dictionary/import --no-fail-fast` | 103 targets, 1,653 passed, 0 failed, 16 ignored | `epoch08-r3-g-workspace.log` |
| `cargo test -r --workspace --all-targets --features expensive_contracts --no-fail-fast` | 70 targets, 1,652 passed, 0 failed, 8 ignored | `epoch08-r3-g-expensive.log` |
| `fixture-test --profile all` | 26,666 fixtures, 72,614 passed, 519 xfailed, 0 failed | `epoch08-r3-g-fixtures.log` |
| tagged facet `subsentence-epoch` | 93 fixtures, 3 facets, 95 passed, 0 failed | `epoch08-r3-g-tagged-facet.log` |
| frozen syntax facet, same tag | 93 fixtures, 93 passed, 0 failed | `epoch08-r3-g-frozen-facet.log` |
| comparer | 122 changed / 86 + 6 + 1 + 0 + 0 mechanical / 29 manual / 0 prose / 93 epoch-new / 0 unpaired / 0 witnesses missing diagnostics | `epoch08-r3-g-comparer.log` |
| comparer unit tests | 27 tests, green | `epoch08-r3-g-comparer-test.log` |
| `cargo build -p jbotci` (debug) | green | `epoch08-r3-g-debug-jbotci.log` |
| `dx build` | green | `epoch08-r3-g-dx.log` |
| `maturin develop` | green | `epoch08-r3-g-maturin.log` |
| `generate_syntax_models.py --check` | green after regeneration | `epoch08-r3-g3-generate_syntax_models.log` |
| `generate_domain_enum_stubs.py --check` | green | `epoch08-r3-g3-generate_domain_enum_stubs.log` |
| `compose_stubs.py --check` | green | `epoch08-r3-g3-compose_stubs.log` |
| `generate_api_matrix.py --check` | green | `epoch08-r3-g3-generate_api_matrix.log` |

**There is no peak-RSS row, and that is the reported result rather than an omission.** The lead's
standing instruction ties the pair to adding a production. This round adds none: no rule, no node
type, no field. `exp_relative_tanru_unit` and `exp_afterthought_selbri_relative_clauses` gain a
lookahead and a typed output rejection on fields they already had, and the proof is the generated
artefacts themselves -- the four Python model files regenerate to a **docstring-only** diff (the
two field doc comments, four files, eight lines), `docs/api-parity.tsv` does not move at all, and
the recovery anchor snapshot loses three rows and gains none.

Two rows carry a note. `generate_syntax_models.py --check` was red on the gate's own pass and is
reported that way; the four model files were regenerated and all four Python checks were re-run
green at the tree that contains them (`epoch08-r3-g3-*.log`). Nothing else in the gate reads those
files. And the first attempt at this gate was abandoned after the `workspace` row went red on
`struct_placeholder_invariant_audit_is_current`: the new zero-sized rejection needs its
allowlist row in `tests/struct_invariant_audit.rs`, exactly as its five siblings in the same file
do. The row was added and the whole gate re-run from the top rather than resumed, so every figure
above comes from one run at one commit.

`fixture-test --profile all` is 26,666 fixtures rather than round 2's 26,645 because the count in
that table predates round 2's own witness additions; against `2284b50691` this round is +4 net
(six added, two renamed away).

The final commit adds only this section and the two above it to `docs/`, plus the regenerated
Python model files. `cargo fmt --all --check`, the comparer, its unit tests and the four Python
checks were re-run at that commit; no other row reads `docs/`.
