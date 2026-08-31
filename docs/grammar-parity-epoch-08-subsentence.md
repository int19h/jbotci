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
| S3 | `zantufa_relative_selbri` | `selbri_relative_clause_list` | S2's list plus the R2 return, and gated (below) |

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
baseline-owned. Gek over I-connected branches is a documented residual gap.

### The retained gated omission

D1.3 freezes the S3 parent as default-enabled-warn. Measurement says it cannot be, and the
disposition is the other one Sol's round-1 finding 4 offered: an explicitly declared retained
gated omission.

The arm is reached inside every nesting whose terminator may elide, and there the enclosing
description's own relative-clause field is the baseline's site for the very same clause:
`.uesai le ni mrilu poi srana la lojban. cu mutce caku` is *the [quantity of mailing] which
concerns Lojban* to camxes-standard and *the quantity of [mailing which concerns Lojban]* to the
arm, over an identical extent. Twenty-four corpus fixtures read that way, and R1 puts the
baseline first.

Two repairs were measured and rejected before the gate was kept. The boundary idiom the
description and the vocative use does not reach: `selbri_without_terminal_relative` restricts the
top spine, and the leak runs through an abstraction body, so closing it means following the
no-terminal-relative entry down a second ladder from selbri level 2, which this epoch does not
build. And a candidate-local classifier that returns baseline-owned lists over-rejects, because
the same list is rolling Zantufa's alone wherever no enclosing site exists — `re broda poi brode
ku` measures that, and it is what issue #828's own fixtures pin.

Three cells of the frozen table move with the gate: `broda poi mi brode`,
`broda no'oi mi brode ku'o` and their ZIhE-chained twin are the Zantufa profile's rather than the
default union's. Both sides are witnessed, and the abstraction population is witnessed as the
no-delta surface it stays. Everything else in D1 is default-enabled as frozen.

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

### The route boundary

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
negatives, the SEI no-delta pins and both documented gaps. Every witness pins its diagnostics,
empty where the expectation is silence.

## Consolidated expectations

