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

The chain's connective is `joik`, the JOI family, not `joik_jek`: `broda no'oi mi brode je no'oi
do brodi` has no route, which the witnesses pin. The SA-erasure prefixes at :215-217 are omitted
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
twins (`crates/jbotci-syntax/src/grammar/baseline_relative.rs`), and both twins are fail-closed:
the recovered side is `valid(c).is_some_and(..)` inside `.all(..)`, never `filter_map`, so an
unparsed child can never be silently dropped out of a universally quantified test.

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

72 epoch-new witnesses are authored rather than classified, and every one pins its diagnostics.
The xfail count is unchanged at 519.

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
