# Grammar parity epoch 9: descriptions and quantifiers

Epoch 9 is the description/quantifier slice of the grammar-parity epic and carries four issues,
because all four are consequences of one missing boundary or of the routes that boundary opens:

| Section | Issue | State |
| --- | --- | --- |
| The `sumti_6` operand tier at the description and quantifier sites | #552, #837 | complete |
| Baseline quantifiers keep baseline ownership | #634 | complete |
| Zantufa description-leading sumti and quantifier relatives | #830 | complete |
| Consolidated expectations, comparer, peak RSS | — | complete |

The design is
`~/git/grammar-review/reports/implementation/epoch-09-descriptions/plan-v7.md` at grammar-review
`40c024bb95`, confirmed by Sol after six rounds with zero remaining findings. The implementation
base is `9ec321d530315fd68a41a7c1c9dc51472366d010`, the epoch-8 merge's follow-up commit.

## The finding that shapes the epoch

jbotci's `sumti_base` was not camxes `sumti_6`. It was `sumti_6` PLUS two `sumti_5`-tier arms:
`descriptor_without_gadri_sumti` (camxes `sumti_5` arm 2, `quantifier selbri KU`) and
`descriptor_with_outer_quantifier_sumti` (a specialization of `sumti_5` arm 1,
`quantifier? sumti_6`). Two consumers that camxes spells with `sumti_6` were therefore reaching
quantifier-bearing forms:

- the LEADING element of a description tail, camxes
  `sumti_tail <- (sumti_6 relative_clauses?)? sumti_tail_1` (camxes.peg:156). A `vei ... ve'o`
  quantifier walked straight into it, the tail body then found `cu`, and the whole description
  failed. That is #552.
- the operand of an outer quantifier, camxes `sumti_5 <- quantifier? sumti_6 relative_clauses?`
  (camxes.peg:150). Its operand could itself be an outer-quantified description or a no-gadri
  quantified selbri. That is #837 SUM-02 — plus a third stacking shape the issue's prose does not
  name, `quantifier` over `descriptor_without_gadri_sumti`.

So #552 and #837 SUM-02 are one fix, and the fix is the missing tier boundary.

## D0: the restriction is one named rule, consumed twice

`description_leading_operand` is a NAMED grammar rule with its own entry in the `recursive {}`
block, and therefore its own parser identity, FIRST set, elidable-terminator analysis and
recovery metadata. It is `sumti_base` refined by a completed-candidate classifier, and it is
consumed BY NAME at exactly the two restricted field sites — `description_tail_sumti.sumti` and
`quantified_sumti.inner_sumti`. The restriction is expressed once and consumed twice.

The classifier is `crates/jbotci-syntax/src/grammar/sumti_operand_tier.rs`, a private module
exporting only `pub(crate) struct QuantifierBearingSumtiRejection`, in the convention of the seven
`baseline_*` classifiers already on main. It has strict and recovered twins, both fail-closed, and
its matches over `SumtiBaseSyntax` are exhaustive and `..`-free, so a future arm is a compile
error rather than a silent classification. That mechanism worked twice inside this epoch: D3's two
new descriptor arms had to be classified there before they could compile, and both are
`sumti_6`-tier descriptor forms, so both are permitted.

Its answer is three-valued, not boolean:

```rust
enum SumtiOperandTier { Sumti6, Sumti5, Unproven }
```

`Unproven` is reachable only on the recovered spine, where a candidate's wrapper carries no
selected arm at all. It is refused alongside `Sumti5`: "did not parse" is not "known to be a
permitted tier", and an unproven candidate never occupies a restricted `sumti_6` slot. Both
consumers read the answer as permission, which is exactly the condition under which epoch 8's
`RelativeBodyShape` lesson requires a third value.

### What the restriction is NOT

`description_tail_sumti`'s `assert !pa_word()` is DELETED. It blocked only the PA spelling and let
`vei ... ve'o` through, which is #552 itself; and #552 requires the exclusion to be structural
rather than a first-token test. With the two quantifier-bearing arms excluded by the classifier no
permitted arm can begin with PA anyway (`number_sumti` starts with LI, `lerfu_string_sumti` with
BY). The guard's job is witnessed as surviving — `lo re lo mlatu ku cu blabi` keeps its
`GAD/q+sum/PaRun/sel` ownership — and its failure path is pinned by two genuine-error witnesses.

`descriptor_with_outer_quantifier_sumti` SURVIVES as the LE/LA specialization of `sumti_5` arm 1,
tried before general `quantified_sumti` exactly as before. Absorbing it into `quantified_sumti`
would be tidier and would re-type 291 fixture files for zero behavioural gain. Three control pins
hold it: `re lo mlatu cu blabi`, `re la djan cu blabi`, `re lo ci mlatu cu blabi`.

### The C-a recovered-delta enumeration

The two restricted field sites were UNRESTRICTED at the base, so reject-on-`Unproven` can change a
winning RECOVERED owner without changing strict acceptance anywhere. That population was
genuinely unreported, so it was measured rather than assumed: every fixture input in
`tests/fixtures` (26,678 texts, each in its declared dialect) was run through
`gentufa --max-errors 20 --turtai raw` against the base binary and against C-a, which prints the
strict tree when the strict parse succeeds and the diagnostics plus the RECOVERED tree when it
does not.

**19 of 26,678 differ, and every one is delta kind 4** — owner-only / diagnostic-only /
recovery-item-only, with strict acceptance unchanged. Zero withdrawals to the no-leading-operand
parse, zero fall-throughs to `sumti_atom.sumti_base`, and zero kind-3 disappearances or
degradations, which is the kind the plan escalates on every occurrence.

| what moved | count | why |
| --- | --- | --- |
| the reported candidate set at a failure offset | 18 | `description_leading_operand` is a named rule, so `descriptor, or description` joins the expected list there; on three of them the rendered `while parsing` context frame also resolves to a different enclosing rule |
| recovery items | 1 | `corpus.camxes.21844` (`coile le ro prenu`) recovered at the base by skipping the WHOLE input into `leading_nai`; with the leading operand carrying its own recovery metadata the machinery anchors differently and now builds the real `VocativeFreeModifier`, leaving only the description tail synthesized. The parse still rejects with the same diagnostic; the recovered tree gains structure rather than losing it |

The error code, the severity, the primary byte span, acceptance and the recovered tree are
identical on all 18; none of the moved leaves is a fixture expectation, which is why the full
fixture profile is 0 failed at C-a.

#### Which restricted site each row reached, and with what answer

"Permitted, or the slot was never reached" is a disjunction, not a measurement, and the delta kind
of a row depends on which of the two it is. The classifier therefore carries an env-gated trace,
`JBOTCI_TRACE_SUMTI_OPERAND_TIER`, documented at `sumti_operand_tier::trace_enabled`: it prints
one line per RECOVERED classification with the consuming SITE — read from the parser's active rule
stack through the `output_rejection_site` seam in `reject_output`, because a classifier is handed a
completed candidate and nothing else — the candidate's byte extent, the recovered wrapper shape,
the tier and the decision. The whole corpus was re-run at C-a with it on.

**The population.** 225 of 26,678 fixtures reach a restricted site on the recovered spine at all,
producing 678 classifications:

| site | fixtures reaching it | classifications | `Sumti6` permit | `Sumti5` reject | `Unproven` | fixtures with a C-a delta | with NO delta |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `description_tail_sumti.sumti` | 184 | 617 | 603 | 14 | **0** | 4 | **180** |
| `quantified_sumti.inner_sumti` | 50 | 61 | 61 | 0 | **0** | 0 | **50** |

Nine fixtures reach both sites. The 180 + 50 rows that reach a restricted site and take no delta
at all are the population D0.5 scopes: the restriction fires on them and nothing observable moves.

**The 19 rows, attributed.** Fifteen of the nineteen reach NEITHER restricted site (traced), so
their delta cannot be a classification at all: it is the named rule's own parser identity widening
the reported candidate set, or moving the rendered `while parsing` frame, at a failure offset
inside a description tail. The remaining four are:

| # | fixture | site reached | classifier result | extent | consequence |
| --- | --- | --- | --- | --- | --- |
| 7 | `corpus.camxes.16852` | `description_tail_sumti.sumti` | `Sumti5` -> REJECT | 3..11 | the optional slot stays empty and the tail body re-parses the same text; acceptance, code, severity and primary span unchanged, only the `while parsing` frame moves (kind 4) |
| 8 | `corpus.camxes.2000` | `description_tail_sumti.sumti` | `Sumti5` -> REJECT | 425..433 | the same, at a description deep inside a long text (kind 4) |
| 15 | `corpus.camxes.21844` | `description_tail_sumti.sumti` | `Sumti6` -> permit (x2, 6..17) and `Sumti5` -> REJECT (9..17) | 6..17 / 9..17 | the recovery-item row: the recovered tree gains structure (kind 4) |
| 18 | `corpus.camxes.5916` | `description_tail_sumti.sumti` | `Sumti5` -> REJECT | 11..15 | as rows 7 and 8 (kind 4) |

Every row is still delta kind 4, and now by attribution rather than by exclusion: no row is a
withdrawal that loses a parse (kind 1 with a degraded result), no row falls through at
`quantified_sumti` (kind 2 — that site takes no delta at all), and no row is a kind-3
disappearance.

**`Unproven` is measured at zero, for a structural reason.** All 678 classifications are
`wrapper=valid`. The classifier is handed whatever `sumti_base`'s own recovered parser returned,
and that returns `Ok` only with a selected arm; the `Error` and `Prefix` wrappers at a field are
built by `recovered_field_parser`, which runs strictly outside `reject_output`. The `Unproven` arm
is a fail-closed guard for a state the current runtime cannot reach at this seam — see "Recovered
witnesses" below — and it is pinned by unit tests that construct the value directly, because no
surface can.

Two corpus-shaped R7 witnesses pin the permitted recovered descent at each restricted site
(`lo mi mlatu cu` keeps its recovered `DescriptionTailSumti` owner, `re la cu blabi` keeps its
recovered `QuantifiedSumti` owner), and two more pin the REFUSAL at each site
(`lo re mlatu cu`, `re vei pa ve'o lo mlatu ku cu`). All four pin the whole recovered tree by
digest as well as the full diagnostic set.

## D1 (#552): the VEI quantifier reaches the tail body

#552 is discharged entirely by D0. With the leading operand restricted, `vei ... ve'o` can no
longer be consumed there, and the tail body's `quantifier selbri` / `quantifier sumti` arms are
reached. No `!VEI` lookahead, no arm reordering, no new node. Seven surfaces flip reject ->
accept, each owned as camxes owns it — `sumti_tail_1` arm 2 gives
`GAD/q+sel(Mekso(PaRun))`.

Two surfaces measured correct at the base must stay byte-identical, and do:
`lo vei pa su'i re ve'o mi cu blabi` (`GAD/q+sum`) and
`lo vei pa ve'o vei re ve'o mlatu cu blabi` (`GAD/q+sum/.../NOGAD`, the `sumti_tail_1` arm-3 route
whose operand IS a full sumti including the no-gadri quantified selbri). D0 removes the no-gadri
form from the LEADING operand only, never from arm 3's operand.

## D2 (#634): baseline quantifiers keep baseline ownership

The accepted #634 fix was carried in as C-b. It is a REBASE of lane A's already-reviewed
re-derivation (`origin/issue-634-refresh` at `284147baa4`), not a cherry-pick, and it landed
through the plan's two-stage workflow: stage 1 rebased a copy of the payload branch onto the epoch
base, surfacing and resolving the conflicts and proving the payload builds there; stage 2 replayed
the two REWRITTEN validation commits onto C-a. The original SHAs are provenance identifiers only.

| role | SHA |
| --- | --- |
| lane A's frozen payload | `f408cff986`, `284147baa4` |
| stage-1 rewritten validation commits | `54193b38d1`, `9cf2ac039e` |
| C-b as landed | `4015ee9c78`, `4fb8b8a921` |

The code commit applied cleanly; the expectation commit conflicted in exactly three files, two
hunks each, all of them expectation payload, because epoch 8 re-owned the same relative-clause
machinery those payloads render after lane A froze. That is the plan's residual risk 4. Carrying
the frozen payloads across would have re-introduced pre-epoch-8 relative ownership into three
fixtures, so C7's METHOD was re-applied instead of its results: every conflicted hunk resolved to
the epoch-base side, the four affected fixtures regenerated on top of the applied code change, and
the regeneration validated mechanically rather than trusted. The lead ACKed that resolution with
four binding conditions on 2026-09-02; `docs/issue-634-quantifier-refresh.md` records it.

The validation rewrites the OLD epoch-base expectation with the two declared #634 shapes and
requires the result to equal the regenerated expectation structurally, so an ownership change
cannot be laundered as a re-typing. **Zero residue:**

| fixture | positions re-typed | genuine raw-mex survivors | `experimental-zantufa-mex` warnings removed |
| --- | --- | --- | --- |
| `corpus/alis/full-alice` | 350 | 1 (`ForethoughtCallMekso`) | 0 (pins no diagnostics) |
| `adhoc/syntax/selbri/issue-828-explicit-ku-zantufa` | 1 | 0 | 1 |
| `adhoc/syntax/selbri/issue-828-elided-ku-zantufa` | 1 | 0 | 1 |
| `adhoc/syntax/tags/issue-833-stag-position-zantufa-rejected` | 0 | 0 | 0 (rejection-diagnostic reclassification) |
| three epoch-9 witnesses C-a had added | 4 | 0 | 4 |

352 + 4 re-typed positions and one surviving genuine raw mex, which is exactly the census the
pre-C-a base ledger measured at the epoch base before any of it landed.

`crates/jbotci-syntax/tests/recovery-anchor-metadata.snapshot.txt` is byte-identical across the
C-a -> C-b replay. That is measured, not asserted: the snapshot was regenerated at both commits
and compared. The whole 35-line delta against the epoch base is C-a's — the named rule's own
metadata block, the rules count, and the field-index shift `description_tail_sumti` takes when its
`assert !pa_word()` retires.

## D3 (#830): three source-distinct routes, three separate owners

### D3a — camxes-exp's full-sumti leading element

camxes-exp's `sumti_tail` arm 3, `sumti sumti_tail_1` (camxes-exp.peg:194), admits a FULL sumti at
the connection level where the baseline admits a `sumti_6`. jbotci adopts it as a sibling
top-level descriptor variant, `exp_descriptor_with_leading_sumti_sumti`, ordered AFTER
`descriptor_with_gadri_sumti` so R1 keeps every extent the baseline route derives — not as a
widening of `description_tail`'s leading field, because a connected sumti cannot fit
`DescriptionTailSumtiSyntax` and widening that field would move baseline trees.

It is default-enabled and warned, under
`syntax.warning.experimental-exp-description-leading-sumti`, because a route ordered after the
baseline cannot reinterpret a successful baseline parse.

`ExpDescriptionLeadingSumtiRejection` is what makes R1 true on the RECOVERED spine, where ordered
choice alone does not: it refuses any completed candidate whose leading sumti is an extent the
baseline leading operand derives. Its answer is three-valued for the same reason D0's is, and
`Unproven` is refused, so an unproven leading sumti never takes an extent away from the baseline.
Four unit tests pin the three rows and the `Prefix` case.

### D3b — rolling Zantufa's relatives-first ordering

`sumti_tail <- relative_clauses? (!quantifier sumti)? sumti_tail_1` (zantufa-1.9999.peg:40). The
Zantufa-only content is the ORDER, so `zantufa_relatives_first_description_tail` makes BOTH the
relatives and the leading sumti mandatory: with either optional the arm could structurally reach
the baseline relatives-only surface or D3a's, and ownership would stop being decidable from the
shape. It is gated on the new `ZANTUFA-DESCRIPTIONS` feature — no existing flag covers it — which
the `zantufa` dialect set now carries, and warned under
`syntax.warning.experimental-zantufa-description-leading-sumti`.

### The `!quantifier` guard, and the one class it excludes

BOTH new rules carry the same real negative lookahead on the `quantifier` production, at the same
position. It is retained because rolling Zantufa spells it literally at exactly this position
(`sumti_tail <- relative_clauses? (!quantifier sumti)? sumti_tail_1`, zantufa-1.9999.peg:40) and
because it states the ownership boundary in the grammar instead of leaving it to ordering. What it
is NOT is the thing that keeps `lo vei pa su'i re ve'o mlatu cu blabi` baseline-owned. That is
plan-v7 F5's premise, and it is **measured false** — see below.

#### The guard is measured inert, and that is recorded rather than assumed

F5 says that without the guard the default axis re-opens #552 through D3a. It does not. Measured,
in the round-1 fix round:

- A guard-DELETED binary was built with both `assert !quantifier` lines removed and swept against
  the guarded binary over all **26,678 fixture inputs** at their declared dialects, comparing the
  full rendered output — errors, warnings, brackets and the recovered spine. **Zero differences.**
- Every constructed quantifier-leading candidate stays R with the guard deleted:

  | surface | std / exp / zan | jbotci, guarded | jbotci, guard deleted |
  | --- | --- | --- | --- |
  | `lo re lo gerku ku pa mlatu cu blabi` (Qwen's proposal) | A / A / A | A | A |
  | `lo re lo gerku ku mlatu cu blabi` | R / R / R | R | R |
  | `lo re lo gerku ku joi lo mlatu ku mlatu ku cu blabi` | R / R / R | R | R |
  | `mi viska lo re lo gerku ku mlatu ku` | — | R | R |
  | `lo poi mi zgana ku'o re lo gerku ku mlatu cu blabi` (D3b twin) | R / R / R | R | R |
  | `lo vei pa su'i re ve'o mlatu cu blabi` (F5's own surface) | A / A / A | A, BASELINE-owned, silent | A, BASELINE-owned, silent |

  Qwen's proposed probe measures A/A/A at the three references, so it is an ordinary baseline
  surface rather than a guard probe at all.
- The structural reason: D3a's leading sumti at that position is exactly `quantifier sumti_6...`,
  which is the IDENTICAL extent D1's restored `sumti_tail_1 <- quantifier sumti` arm consumes
  inside `descriptor_with_gadri_sumti` — and `sumti_base` orders that arm FIRST. Ordered choice
  commits on its success, and an outer failure never re-enters a committed inner choice, so for
  any quantifier-opening extent the baseline route claims it before the camxes-exp or Zantufa arm
  is reached. Guard or no guard.

So no surface exists on which ONLY the guard prevents acceptance, and no guard-critical witness
pair can be authored. The lead ruled (2026-09-02) that this recorded argument stands in place of
the pair, that F5's premise is recorded as measured-false here, and that the guard is RETAINED as
Zantufa's literal spelling and as defence in depth: it is what makes the ownership boundary hold
by construction rather than by an ordering property that a later epoch could disturb without
noticing. `d3a-quantifier-guard-keeps-baseline` is re-worded to say what it actually pins — D1's
ownership, held by ordering — rather than claiming to isolate the guard.

camxes-exp spells no such guard. Its only protection is ordering — arm 1 with the optional group
empty reduces to `sumti_tail_1`, which takes the ordinary quantifier-leading tails — and #552 and
B1 forbid resting an ownership boundary solely on ordering. Rolling Zantufa spells the guard
literally at exactly this position. The narrowing is therefore an intentional, recorded
exp-language non-adoption, and it is not always mere re-ownership: exp's `sumti_5` has a
`quantifier gek_sentence` arm that `sumti_tail_1` does not cover, so for that one class the guard
removes a genuinely exp-only accepted extent. Measured witness
`lo re ge mi klama gi do bajra ku mlatu cu blabi` (std R / **exp A** / zan R), pinned jbotci-R on
every axis, and filed as #886.

### D3c — trailing relatives on the raw-mex quantifier routes

`quantifier <- (!sumti_5 !selbri mex relative_clauses?)` (zantufa-1.9999.peg:55). The optional list
is spelled as SEPARATE with-relatives sibling variants rather than as an optional field, so every
surviving no-relative node stays byte-identical and no absent field is added to the existing
quantifier expectations. Both ride the existing `ZANTUFA-MEX` gate and the existing
`experimental-zantufa-mex` category: the trailing list is rolling Zantufa's own quantifier shape,
not a second construct.

The strict truth table, each row a separate witness:

| priority-route match | relatives | owner | outcome |
| --- | --- | --- | --- |
| baseline PA-run / `VEI mex VEhO` | absent | classifier REJECTS | baseline reparse, no warning (#634) |
| baseline PA-run / `VEI mex VEhO` | present | with-relatives variant | zantufa-owned, warns |
| genuine raw mex | absent | priority route | zantufa-owned, warns (unchanged) |
| genuine raw mex | present | with-relatives variant | zantufa-owned, warns |

The six-arm order is what makes it hold: a matching with-relatives priority arm wins BEFORE the
no-relatives arm can match and be rejected, so a no-relatives rejection can never withdraw a
surface the with-relatives variant would have taken, and the two recovered-fallback arms stay
strictly unreachable on a strict parse.

#### The classifier re-hook, and its tree-transparency proof

#634's rejection was attached to the inner `mekso` FIELD, and a field-level predicate cannot see a
relatives slot parsed after it. It moves to RULE level, through a tree-transparent eligibility
alias the `quantifier` sum consumes, with its `OutputRejection` impls retargeted from `MeksoSyntax`
to the wrapper output. The bare rule is reached only THROUGH the alias, so the classifier cannot be
bypassed by naming the rule in the sum.

Tree transparency is MEASURED, not assumed: the four genuine raw-mex positions in the corpus are
byte-identical across the re-hook, so an accepted no-relative quantifier still serializes as its
pre-epoch `ZantufaPriorityRawMeksoQuantifier` with no wrapper layer, no changed node name and no
changed field path. The comparer's declared multiplicity for an absent-field rewrite is therefore
**zero**, and it is zero because the frozen sibling-variant shape shipped rather than the fallback.

#### The started-production entry invariant

The joint enforcement rests on one fact the strict spine gives for free and the recovered spine
does not: that a with-relatives candidate really has relatives. The recovery runtime can satisfy a
mandatory field by synthesizing an error item having consumed no input and having never entered
the relative-list production, and it can also resynchronize and hand back skipped tokens that need
not begin with any relative opener. Either would let an absent-relative surface buy Zantufa
ownership with an error item, violating the recovered policy's row 1 with no strict-path symptom
at all.

`UnstartedRelativeListRejection` tests ENTRY EVIDENCE on the completed product — not slot
presence, and not slot extent:

| recovered `relative_clauses` slot | started? |
| --- | --- |
| `Valid(list)` whose `first` atom's opening marker is proven parsed, by recursive descent | yes |
| `Valid(list)` whose nested `first` was itself synthesized | **no** — a wrapper can be `Valid` over a placeholder |
| `Prefix { value, .. }` whose parsed value proves an opener the same way | yes |
| `Error(MissingRequiredField)` — zero extent, no attempt | no |
| `Error(SkippedTokens)` whose FIRST skipped token is in FIRST(`relative_clause_atom`) | yes |
| `Error(SkippedTokens)` opening outside that inventory | **no** — extent alone would pass it |
| any unenumerated wrapper shape | no, fail-closed |

The inventory is enumerated from the grammar by descending the production, not probed. Every
alternative's first constituent is a single marker word, so FIRST(`relative_clause_atom`) is
selma'o GOI in full — `goi`, `ne`, `no'u`, `pe`, `po`, `po'e`, `po'u`, `voi'e` — plus the six
relative markers `poi`, `po'oi`, `voi`, `voi'i`, `noi`, `no'oi`. ZIhE is deliberately absent: it is
the continuation connective of `relative_clause_list`, reached only after the list's `first` atom,
so it can never open the list. Eight unit tests pin the table row by row, including the two
controls that motivate the two halves of the predicate.

## Recovered witnesses, and the shapes the runtime will not produce

The three classifiers this epoch adds all answer a question that only exists on the RECOVERED
spine, and a unit test over a hand-built value proves the predicate rather than the wiring. Every
row below therefore has a parser-level witness — a fixture whose `[expectations.syntax.recovered]`
pins the recovered status, the FULL diagnostic set, and the whole recovered tree as a `sha256`
digest — or, where the runtime provably will not produce the shape from any surface, a recorded
argument in its place. The unit tests stay: they pin the predicate, the fixtures pin the wiring,
and neither discharges the other.

### How the classifier answer was measured

Each classifier carries an env-gated trace, documented at its `trace_enabled` function:
`JBOTCI_TRACE_SUMTI_OPERAND_TIER`, `JBOTCI_TRACE_DESCRIPTION_LEADING` and
`JBOTCI_TRACE_QUANTIFIER_RELATIVES`. Each prints one line per recovered classification with the
consuming SITE — read from the parser's active rule stack, which a classifier cannot otherwise
see, through the `output_rejection_site` seam in `reject_output` — and the answer. A refused
candidate leaves nothing behind in the tree, so "refused" and "never reached" are
indistinguishable without them; they are how every row here was attributed rather than assumed.
The frozen pre-epoch fixture corpus was swept with all three on (26,678 inputs, each in its
declared dialect), and every count below is from that sweep. The recovered spine intentionally
has no generated construct warnings: `recovered_success` forwards only warnings attached by the
parser and does not call `add_generated_construct_warnings`. Plan-v7 D3c B8 [PM] froze adding that
generation as out of scope; #891 tracks the follow-up. The strict-spine fixtures are therefore
the witnesses that pin construct warnings, while recovered fixtures pin ownership, diagnostics,
and recovered trees without claiming one.

Two classifier populations below use that same 26,678-input manifest at different commits. The
C-a-only enumeration recorded 678 restricted-site calls (617 `description_tail_sumti`, 61
`quantified_sumti`). The post-fix-round HEAD sweep records 702 (641 and 61): later description
grammar and recovered-routing commits add 24 transient calls on the SAME frozen inputs. The 702
does not include any of the series' 106 newly added witness fixture files; those are outside the
frozen manifest. Both counts are retained below and labelled by commit population.

### D0: the two restricted operand sites

| row | witness | measured classification |
| --- | --- | --- |
| permitted descent, `description_tail_sumti.sumti` | `d0-recovered-through-leading-operand` (`lo mi mlatu cu`) | `site=description_tail_sumti bytes=3..5 wrapper=valid tier=Sumti6 decision=permit` |
| permitted descent, `quantified_sumti.inner_sumti` | `d0-recovered-through-quantified-operand` (`re la cu blabi`) | `site=quantified_sumti bytes=3..14 wrapper=valid tier=Sumti6 decision=permit` |
| **refusal**, `description_tail_sumti.sumti` | `d0-recovered-leading-operand-refuses-sumti5` (`lo re mlatu cu`) | `site=description_tail_sumti bytes=3..11 wrapper=valid tier=Sumti5 decision=reject` |
| **refusal**, `quantified_sumti.inner_sumti` | `d0-recovered-quantified-operand-refuses-sumti5` (`re vei pa ve'o lo mlatu ku cu`) | `site=quantified_sumti bytes=3..26 wrapper=valid tier=Sumti5 decision=reject` |
| `Unproven` at either site | **not producible — argument below** | 0 of 702 HEAD classifications over the frozen 26,678-input manifest (the C-a-only population was 0 of 678) |

**Why `Unproven` has no surface witness at these two sites.** The wrapper the classifier is handed
is whatever `sumti_base`'s own recovered parser returned, and that parser returns `Ok` only with a
selected arm, always as `Recovered::Valid`. The `Error` and `Prefix` wrappers at a field are built
by `recovered_field_parser`, which runs strictly OUTSIDE `reject_output`: on a field failure it
calls `O::from_recovery_item` after the refinement has already returned, and `prepend_recovery_item`
turns a `Valid` into a `Prefix` after it as well. So no surface can hand this classifier a
non-`Valid` candidate. Measured against that argument: of the 702 recovered classifications the
HEAD sweep records at these two sites, **702 are `wrapper=valid`** and none is `Unproven`. The
`Unproven` arm is a fail-closed guard for a state the current runtime cannot reach here — which is
exactly why it is written and why the three unit tests in `sumti_operand_tier.rs` construct the
value directly. If the recovery runtime ever gains a path that wraps a whole rule product, the arm
already answers correctly instead of admitting an unproven candidate.

### D3a: the three recovered rows of the no-steal table

| row | witness | measured classification |
| --- | --- | --- |
| baseline-derivable -> REJECT, stays baseline, no warning | `d3a-recovered-baseline-derivable-leading-sumti` (`le le pe le broda ku brode`) | `slot=valid origin=BaselineDerivable decision=reject` |
| exp-only -> ACCEPT, EXP-owned | `d3a-recovered-exp-only-leading-sumti` (`lo mi je do mlatu cu`) | `slot=valid origin=ExpOnly decision=keep` |
| `Unproven` under a **Valid** slot -> REJECT | `d3a-recovered-unproven-leading-sumti` (`le broda pe la gy cu na broda`) | `slot=valid origin=Unproven decision=reject` |
| `Unproven` at the slot wrapper -> REJECT | `d3a-recovered-unproven-prefix-leading-sumti` (`lo mi cu`) | `slot=prefix decision=reject` |
| the `!quantifier` control | `d3a-recovered-quantifier-guard-control` (`lo re lo gerku ku mlatu cu`) | no D3a classification at all; the operand-tier classifier answers `Sumti5 decision=reject` at `description_tail_sumti` |

Corpus population of the three D3a rows: 46 `BaselineDerivable`, 7 `Unproven` under a `Valid`
slot, 11 `Prefix` slots, 0 `ExpOnly` — the accepting row is the one the corpus does not contain,
which is why it has a constructed witness.

### D3c: the recovered truth table and its two controls

| row | witness | measured classification |
| --- | --- | --- |
| R1, truly absent | `d3c-recovered-r1-absent-relatives-{zantufa,flag}` (`mi ku tirna re cmalu`) | no with-relatives classification at all; the baseline route re-owns the quantifier |
| R2, `Recovered::Valid` with a parsed opener | `d3c-recovered-r2-valid-relatives-{zantufa,flag}` (`tirna re poi cmalu`) | `slot=valid entry=parsed-opener started=yes decision=accept`; no generated construct warning on the recovered spine |
| R3, `Recovered::Error(SkippedTokens)` opening IN `FIRST(relative_clause_atom)` | **not producible — argument below** | 0 of the frozen 26,678-input manifest |
| R4, `Recovered::Prefix` | `d3c-recovered-r4-prefix-trial-relatives-{zantufa,flag}` (`tirna re poi cmalu kei`) | classified in an intermediate, globally unsuccessful trial on both axes: `slot=prefix entry=parsed-opener started=yes decision=accept`; witnessed by that trace and `prefix_slot_is_started_only_through_a_parsed_opener`; no winning-tree witness was found in the recorded 12,668-surface search (measured-bounded deviation from F4); no generated construct warning on the recovered spine |
| R5, synthesized zero-width `Error(MissingRequiredField)` | `d3c-recovered-r5-synthesized-relatives-{zantufa,flag}` (`tirna vei re ve'o`) | `slot=missing entry=none started=no decision=reject` |
| the H1 SIXTH control, `SkippedTokens` opening OUTSIDE `FIRST` | **not producible — argument below** | 0 of the frozen 26,678-input manifest |
| the I1 SEVENTH control, `Valid` over a synthesized opener | `d3c-recovered-i1-synthesized-opener-{zantufa,flag}` (`tirna re zi'e noi cmalu`) | `slot=valid entry=synthesized-opener started=no decision=reject` |

**Why R3 and the sixth control cannot reach a completed with-relatives candidate.** The ordinary
selector cannot target this field: its sole metadata entry is `resume 2 origin FieldFirst start
[Cmavo(Nohoi), Cmavo(Noi), Cmavo(Pohoi), Cmavo(Poi), Cmavo(Voi), Cmavo(Voihi), Selmaho(Goi)]`, and
recovery v1 deliberately excludes `FieldFirst`. The only remaining way to build an outer
`Error(SkippedTokens)` is therefore an exact FINAL directive for the with-relatives product. Such
a directive has `resume_field = usize::MAX`; the apparent H1 path is for it to fire at the
`relative_clauses` field start and make `recovery_field_action_with_match` abandon that required
field. The generated recovery wrapper also checks for a trailing directive immediately after
each preceding field succeeds, however, before it starts the next field. That ordering closes the
apparent path.

That exact directive cannot fire for either row:

- For R3, every token in FIRST(`relative_clause_atom`) is a one-word first constituent. The
  relative-list parser consumes it before any body failure, so this arm's failure is strictly to
  its right. Farthest-failure selection keeps only branches at the greatest span start; a sibling
  arm can move that selected point still farther right, but cannot move it back to field 1's
  start. The FINAL directive consequently cannot exact-match the field start. A natural-stop
  replay is required to fire strictly left of the declared failure; when it advances to EOF and
  reparses this required field, the field cannot succeed, so no completed product reaches
  `reject_output` with an outer skipped slot.
- For the sixth control, a token outside FIRST makes the relative field fail at its start. The
  main loop's exact phase may stop before the with-relatives claim, but that fact is not the
  exclusion: after an earlier accepted recovery, `try_final_recovery_from_current_failure` tries
  the COMPLETE FINAL list. Its non-empty `directives` precondition means a prior trial made
  progress, whose acceptance required the new failure to be strictly right of both the prior
  failure and that directive's resume point. A final replay is accepted only after every prior
  directive and the new one are consumed, so it must reproduce that later failure position.
  Farthest-failure selection gives the with-relatives branch only two cases. If a sibling moves
  the selected failure right of the relatives boundary, the exact directive cannot fire there.
  If the selected failure equals the boundary, the preceding required `mekso` field has just
  succeeded at exactly that location. Before `relative_clauses` starts, `recovered_field_parser`
  calls `trailing_recovery_field_action` for `mekso`; that hook consumes the `usize::MAX` FINAL
  directive, prepends its `SkippedTokens` item to `mekso`, advances to EOF, and records that the
  skipped item was emitted. The following required `relative_clauses` field is therefore
  abandoned with `MissingRequiredField`, not a second skipped item, so `reject_output` sees R5
  (`slot=missing entry=none started=no`) rather than H1. If `mekso` does not succeed to that
  boundary, the product never reaches the classifier. The eligibility-alias FINAL claim has a
  different rule identity and cannot target either product field, so it cannot bypass this
  ordering. The main loop's natural replay still requires a strictly-left firing point; the
  post-progress helper runs exact directives only. Thus no completed candidate carrying the H1
  outer slot reaches `reject_output`.

This is a structural exclusion, not an anchor-only inference. The 26,678-input trace sweep, the
targeted `VEI mex VEhO` / PA-run matrices, an exhaustive canonical-cmavo sweep at the token after
`VEhO`, and targeted post-progress replays (both feature forms, raised error cap) corroborate it:
on that frozen manifest the only observed outer slots are `Valid` with a parsed opener, `Valid`
over a synthesized opener, and `Error(MissingRequiredField)`. The separately constructed R4
surface exercises another classifier path: after the list parses a valid first relative, its
`zero_or_more` tail records the failed attempt on trailing `kei` while the with-relatives product
is still active; an intermediate FINAL replay attaches that skipped input to the completed
`relative_clauses` field through its trailing hook, and the classifier accepts the resulting
started `Prefix`. That trial is globally unsuccessful. Recovery continues, and the winning tree
instead places `Prefix` on `outer_quantifier` around a with-relatives value whose
`relative_clauses` field is `Valid`.

A bounded search tested 5,880 generated surfaces on each feature axis, plus 530 targeted
Zantufa relative-body probes and 378 targeted connective/context probes: 12,668 surfaces in all,
plus hand probes. No winning tree carried `relative_clauses: Prefix(`. Representative losers put
the winning `Prefix` on `outer_quantifier`, the nested `subbridi`, a GOI payload `sumti`, or
enclosing `text`, while keeping `relative_clauses` `Valid`. This is a measured-bounded result, not
an impossibility claim, and is the recorded reason for deviating from F4's winning-rule witness
requirement. The full generator sets and representative trees are archived at
`~/git/grammar-review/reports/implementation/epoch-09-descriptions/reviews/pr-r5-r4-winning-prefix-search.md`.

The predicate answers all three recovered wrapper shapes, and the unit tests in
`zantufa_quantifier_relatives.rs` construct them directly:
`prefix_slot_is_started_only_through_a_parsed_opener` (R4, both directions),
`skipped_relative_opener_is_started` (R3) and `skipped_non_relative_input_is_not_started` (the
sixth control). The renamed R4 fixtures pin the live surface's winning tree and diagnostics on
both feature axes, while their provenance records the intermediate R4 classification; they do
not claim that the winning tree carries the R4 slot. R3 and the sixth control stay model-level
tests backed by the structural exclusion above.

Residual risk: the H1 exclusion is a structural proof rather than a fixture and is therefore
fragile to future recovery-runtime changes; the predicate and its unit tests already answer that
shape fail-closed if the runtime begins producing it.

## D4 (#837): the connected head, and the stacked operand

### SUM-01: the head connective is deleted outright

`description_connection_sumti` and `description_head_connective` are gone — the rule, its
`sumti_base` arm, the connective production, the CBM name-form warning arm that reached them, the
two semantic descents, and the now-unreachable `"descriptor connective"` parser-context metadata
row. R6 governs: `lo je le mlatu cu blabi` is rejected by camxes, camxes-exp and rolling Zantufa
alike; no `.peg` in `~/git/ilmentufa` or `upstream/gerna_cipra` contains a connective between
description heads; and the `ExperimentalSimplerDescriptorHeadConnective` category the route would
have warned under has ZERO producers, so an existing warning category is not provenance. Epoch 6
set the precedent by removing the structurally identical `simpler-term-connective` route.

Cost: exactly one fixture,
`adhoc/v0/warnings/experimental/simpler-descriptor-head-connective`, the only
`DescriptionConnectionSumti`-bearing fixture in the corpus, re-pinned as a rejection. An SA-shaped
negative pins that erasure cannot resurrect the route (`lo je le sa le mlatu cu blabi`, R/R/R,
accepted at the base and rejected now), with its control `lo je sa lo mlatu cu blabi`, where SA
erases the connective itself, unchanged.

### SUM-02: stacked outer quantification, and the third shape

A pure narrowing with zero fixture flips: `inner_sumti: DescriptorWithOuterQuantifierSumti`,
`inner_sumti: DescriptorWithoutGadriSumti` and `inner_sumti: DescriptionConnectionSumti` are all
zero files at the epoch base. It costs only negative witnesses — `re boi ci lo mlatu cu blabi`,
`vei re ve'o vei ci ve'o lo mlatu cu blabi`, `vei re ve'o re lo mlatu cu blabi`, and
`re boi ci mlatu cu blabi`, the shape #837's prose does not name — all R/R/R at the references,
all accepted at the base, all rejected now.

### The four dead variants are tombstones, not deletions

`ExperimentalSimplerSumtiConnective`, `ExperimentalSimplerForethoughtConnective`,
`ExperimentalSimplerTermConnective` and `ExperimentalSimplerDescriptorHeadConnective` are RETAINED
with tombstone doc comments naming each one never-emitted and pointing at #885, which this epoch
filed for their removal together. Deleting them is a public-API change across the Rust enum, the
Python mapping and the parity inventory, which would materially expand a syntax epoch; epoch 6 set
that precedent too.

### The double relative-list over-acceptance is #884, not epoch 9

`re mlatu ku poi mi zgana ku'o poi do zgana ku'o cu blabi` is R/R/A at the references and accepted
SILENTLY at jbotci's default profile, because `descriptor_without_gadri_sumti` carries its own
relative list AND the enclosing `simple_sumti` adds a second one. The defect is real, its cause is
duplicated relative-list ownership rather than the operand tier, its zantufa-axis answer pulls in
a `sumti_3`-level trailing `relative_clauses?` this epoch does not adopt, and it intersects #828.
Epoch 9 carries two dependency pins asserting NO DELTA — the default-negative (A, silent) and the
zantufa-positive (A + warn, the behaviour #884 must preserve) — plus the already-correct GAD twin
and the single-list control.

## Retained source gaps

Every row is witnessed and unimplemented.

| gap | source | witness | status |
| --- | --- | --- | --- |
| zantufa `sumti_tail_1` `tag?` prefix | `zantufa-1.9999.peg:41` | `lo ba re mlatu cu blabi`, R/R/A, jbotci R on both axes | not adopted; outside #830's criteria |
| zantufa `NAhE_clause sumti_3` arm of `sumti_5` | `:38` | no jbotci route | not adopted |
| zantufa `sumti_3`-level trailing `relative_clauses?` | `:36` | D4.3's two pins | deferred to #884 / #828 |
| camxes-exp `sumti_tail` arm 4, `gek_sentence` as the WHOLE tail | `camxes-exp.peg:194` | `lo ge mi klama gi do bajra cu blabi`, std R / **exp A** / zan R, jbotci R | not adopted; the row exists so the production is neither lost silently nor adopted unreviewed |
| camxes-exp's raw-mex quantifier gate `!selbri !sumti_6 mex` | `camxes-exp.peg:273` | `tirna pa su'i re cmalu`, `lo pa su'i re mlatu cu blabi` | gate NOT widened; exp's `!sumti_6` and Zantufa's `!sumti_5` exclude different tiers and need their own probe battery — filed as **#888** |
| D3a's `!quantifier` guard has no camxes-exp counterpart | `camxes-exp.peg:194` vs `zantufa-1.9999.peg:40` | `lo re ge mi klama gi do bajra ku mlatu cu blabi`, std R / **exp A** / zan R | guard KEPT; documented fidelity narrowing, filed as **#886** |
| **emergent union composite** (NEW) | std name-leading `sumti_6` × exp connection-level leading sumti | `le mi jo'u la djan. selmrilu cu blabi`, R/R/R, jbotci **A** + the exp warning | accepted as a recorded union composite by lead ruling 2026-09-02; the CLASS is filed as **#887** |

### The union composite, in detail

D3a lifts the leading position from `sumti_6` to a connection-level `sumti`. jbotci separately
accepts a bare NAME as that leading element, from camxes-standard: `lo la djan mlatu cu blabi` is
std A / exp R / zan R and was already accepted at the base — the cell table's own no-delta CBM
row. Because jbotci holds BOTH components and no single reference parser does, the composite is
accepted by jbotci and by nothing else. Its whole corpus population is one fixture,
`corpus/camxes/18322`.

The lead ruled to accept it: the union grammar is jbotci's own coherent grammar, every constituent
has provenance, the acceptance is purely additive (reject -> accept, no baseline parse
reinterpreted and no extent stolen — names are not quantifiers, so #552 is untouched), the extent
is R2 exp-owned and carries the camxes-exp warning, and narrowing it would need an UNSOURCED
structural restriction, which B1 and #552 doctrine make the implementer refuse to spell. Three
witnesses pin the composite, its name-free control (`le do jo'u mi selmrilu cu blabi`, which
camxes-exp accepts outright) and the pre-existing component. Note the composite exists only at the
default profile: under `--dialect zantufa` the CBM name handling removes the component and the
surface rejects.

## Genuine-error witnesses

The retired `assert !pa_word()` and the newly rejected stacked attempts change what the FAILURE
path looks like, because rejection rewinds and discards the abandoned attempt's diagnostic
candidates. Two witnesses pin it, at both axes:

| witness | surface | pins |
| --- | --- | --- |
| missing tail after a VEI-shaped quantifier | `lo vei pa su'i re ve'o cu blabi` | rejection with a genuine missing-tail diagnostic — not a silent mis-parse, and not a diagnostic naming the retired guard |
| PA-leading control | `lo re cu blabi` | the diagnostic set the deleted guard used to produce, re-derived structurally |

## The cell table, base | head

`std` / `exp` / `zan` are `camxes.js`, `camxes-exp.js` and `js/zantufa-1.9999.js`. Node
abbreviations: GAD = `DescriptorWithGadriSumti`, NOGAD = `DescriptorWithoutGadriSumti`, OUTQ =
`DescriptorWithOuterQuantifierSumti`, CONN = `DescriptionConnectionSumti`, QSUM =
`QuantifiedSumti`, LEAD = `DescriptionTailSumti`, EXPLEAD =
`ExpDescriptorWithLeadingSumtiSumti`, ZANLEAD = `ZantufaDescriptorWithRelativesFirstSumti`,
`q+sel` / `q+sum` / `sel` = the three description-tail bodies, PaRun / Mekso / ZPri / ZPriRel /
ZRaw = the quantifier variants, `!x` = `syntax.warning.experimental-x`.

| surface | std | exp | zan | base default | base zantufa | head default | head zantufa |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `lo vei pa su'i re ve'o mlatu cu blabi` | A | A | A | R | R | A `GAD/q+sel/Mekso/PaRun` | A `GAD/q+sel/Mekso/PaRun` |
| `lo vei pa ve'o mlatu cu blabi` | A | A | A | R | R | A `GAD/q+sel/Mekso/PaRun` | A `GAD/q+sel/Mekso/PaRun` |
| `lo vei pa su'i re mlatu cu blabi` | A | A | A | R | R | A `GAD/q+sel/Mekso/PaRun` | A `GAD/q+sel/Mekso/PaRun` |
| `lo vei vei pa ve'o su'i re ve'o mlatu cu blabi` | A | A | A | R | R | A `GAD/q+sel/Mekso/PaRun` | A `GAD/q+sel/Mekso/PaRun` |
| `lo vei pe'o su'i pa re ku'e ve'o mlatu cu blabi` | A | A | A | R | R | A `GAD/q+sel/Mekso/PaRun` | A `GAD/q+sel/Mekso/PaRun` |
| `lo vei pa su'i re ve'o mlatu poi blabi ku'o cu blabi` | A | A | A | R | R | A `GAD/q+sel/Mekso/PaRun` | A `GAD/q+sel/Mekso/PaRun` |
| `lo vei pa su'i re ve'o lo gerku cu blabi` | A | A | A | R | R | A `GAD/q+sum/Mekso/PaRun/sel` | A `GAD/q+sum/Mekso/PaRun/sel` |
| `lo vei pa su'i re ve'o mi cu blabi` | A | A | A | A `GAD/q+sum/Mekso/PaRun` | A `GAD/q+sum/ZPri/PaRun` !zantufa-mex | A `GAD/q+sum/Mekso/PaRun` | A `GAD/q+sum/Mekso/PaRun` |
| `lo vei pa ve'o vei re ve'o mlatu cu blabi` | A | A | A | A `GAD/q+sum/Mekso/PaRun/NOGAD` | A `GAD/q+sum/ZPri/PaRun/NOGAD` !zantufa-mex | A `GAD/q+sum/Mekso/PaRun/NOGAD` | A `GAD/q+sum/Mekso/PaRun/NOGAD` |
| `lo ci mlatu cu blabi` | A | A | A | A `GAD/q+sel/PaRun` | A `GAD/q+sel/ZPri/PaRun` !zantufa-mex | A `GAD/q+sel/PaRun` | A `GAD/q+sel/PaRun` |
| `lo re mlatu cu blabi` | A | A | A | A `GAD/q+sel/PaRun` | A `GAD/q+sel/ZPri/PaRun` !zantufa-mex | A `GAD/q+sel/PaRun` | A `GAD/q+sel/PaRun` |
| `lo mi ci mlatu cu blabi` | A | A | A | A `GAD/LEAD/q+sel/PaRun` | A `GAD/LEAD/q+sel/ZPri/PaRun` !zantufa-mex | A `GAD/LEAD/q+sel/PaRun` | A `GAD/LEAD/q+sel/PaRun` |
| `lo mi vei pa su'i re ve'o mlatu cu blabi` | A | A | A | A `GAD/LEAD/q+sel/Mekso/PaRun` | A `GAD/LEAD/q+sel/ZPri/PaRun` !zantufa-mex | A `GAD/LEAD/q+sel/Mekso/PaRun` | A `GAD/LEAD/q+sel/Mekso/PaRun` |
| `lo mi poi blabi ku'o vei pa su'i re ve'o mlatu cu blabi` | A | A | A | A `GAD/LEAD/q+sel/Mekso/PaRun` | A `GAD/LEAD/q+sel/ZPri/PaRun` !zantufa-mex | A `GAD/LEAD/q+sel/Mekso/PaRun` | A `GAD/LEAD/q+sel/Mekso/PaRun` |
| `tirna re cmalu se krixa` | A | A | A | A `NOGAD/PaRun` | A `NOGAD/ZPri/PaRun` !zantufa-mex | A `NOGAD/PaRun` | A `NOGAD/PaRun` |
| `tirna vei pa su'i re ve'o cmalu` | A | A | A | A `NOGAD/Mekso/PaRun` | A `NOGAD/ZPri/PaRun` !zantufa-mex | A `NOGAD/Mekso/PaRun` | A `NOGAD/Mekso/PaRun` |
| `tirna re boi cmalu` | A | A | A | A `NOGAD/PaRun` | A `NOGAD/ZPri/PaRun` !zantufa-mex | A `NOGAD/PaRun` | A `NOGAD/PaRun` |
| `mi tirna vei pa ve'o cmalu` | A | A | A | A `NOGAD/Mekso/PaRun` | A `NOGAD/ZPri/PaRun` !zantufa-mex | A `NOGAD/Mekso/PaRun` | A `NOGAD/Mekso/PaRun` |
| `re moi broda` | A | A | A | A | A | A | A |
| `re roi klama` | A | A | A | A | A | A | A |
| `tirna pa su'i re cmalu` | R | A | A | R | A `NOGAD/ZPri/PaRun` !zantufa-mex | R | A `NOGAD/ZPri/PaRun` !zantufa-mex |
| `tirna vei pa bo re ve'o cmalu` | R | R | A | R | A `NOGAD/ZPri/PaRun` !zantufa-mex | R | A `NOGAD/Mekso/PaRun` !zantufa-mex |
| `tirna pa su'i re mi` | R | A | A | R | A `QSUM/ZPri/PaRun` !zantufa-mex | R | A `QSUM/ZPri/PaRun` !zantufa-mex |
| `lo pa su'i re mlatu cu blabi` | R | A | A | R | A `GAD/q+sel/ZPri/PaRun` !zantufa-mex | R | A `GAD/q+sel/ZPri/PaRun` !zantufa-mex |
| `lo poi mi zgana ku'o mlatu cu blabi` | A | A | A | A `GAD/sel` | A `GAD/sel` | A `GAD/sel` | A `GAD/sel` |
| `lo mi poi blabi ku'o mlatu cu blabi` | A | A | A | A `GAD/LEAD/sel` | A `GAD/LEAD/sel` | A `GAD/LEAD/sel` | A `GAD/LEAD/sel` |
| `lo mi mlatu cu blabi` | A | A | A | A `GAD/LEAD/sel` | A `GAD/LEAD/sel` | A `GAD/LEAD/sel` | A `GAD/LEAD/sel` |
| `lo lo gerku ku mlatu cu blabi` | A | A | A | A `GAD/LEAD/sel` | A `GAD/LEAD/sel` | A `GAD/LEAD/sel` | A `GAD/LEAD/sel` |
| `lo lo gerku ku poi mi zgana ku'o mlatu cu blabi` | A | A | A | A `GAD/LEAD/sel` | A `GAD/LEAD/sel` | A `GAD/LEAD/sel` | A `GAD/LEAD/sel` |
| `lo poi mi zgana ku'o re mlatu cu blabi` | A | A | A | A `GAD/q+sel/PaRun` | A `GAD/q+sel/ZPri/PaRun` !zantufa-mex | A `GAD/q+sel/PaRun` | A `GAD/q+sel/PaRun` |
| `lo lo mlatu ku joi lo gerku ku mlatu cu blabi` | R | A | A | R | R | A `EXPLEAD/GAD/sel` !exp-description-leading-sumti | A `EXPLEAD/GAD/sel` !exp-description-leading-sumti |
| `lo mi je do mlatu cu blabi` | R | A | A | R | R | A `EXPLEAD/sel` !exp-description-leading-sumti | A `EXPLEAD/sel` !exp-description-leading-sumti |
| `lo poi mi zgana ku'o mi mlatu cu blabi` | R | R | A | R | R | R | A `ZANLEAD/sel` !zantufa-description-leading-sumti |
| `lo poi mi zgana ku'o lo gerku ku mlatu cu blabi` | R | R | A | R | R | R | A `ZANLEAD/GAD/sel` !zantufa-description-leading-sumti |
| `tirna pa su'i re poi mi zgana ku'o cmalu` | R | R | A | R | R | R | A `NOGAD/ZPriRel/PaRun` !zantufa-mex |
| `tirna re poi mi zgana ku'o cmalu` | R | R | A | R | R | R | A `NOGAD/ZPriRel/PaRun` !zantufa-mex |
| `tirna vei pa su'i re ve'o poi mi zgana ku'o cmalu` | R | R | A | R | R | R | A `NOGAD/ZPriRel/PaRun` !zantufa-mex |
| `tirna pa su'i re poi mi zgana cmalu` | R | R | R | R | R | R | R |
| `lo mi ku'o mlatu cu blabi` | R | R | R | R | R | R | R |
| `lo poi mi zgana ku'o ku mlatu cu blabi` | R | R | R | R | R | R | R |
| `lo mi lo gerku ku mlatu cu blabi` | R | R | R | R | R | R | R |
| `lo ba re mlatu cu blabi` | R | R | A | R | R | R | R |
| `lo re ba mlatu cu blabi` | A | A | A | A `GAD/q+sel/PaRun` | A `GAD/q+sel/ZPri/PaRun` !zantufa-mex | A `GAD/q+sel/PaRun` | A `GAD/q+sel/PaRun` |
| `lo je le mlatu cu blabi` | R | R | R | A `CONN/sel` | A `CONN/sel` | R | R |
| `lo ja le mlatu cu blabi` | R | R | R | A `CONN/sel` | A `CONN/sel` | R | R |
| `re boi ci lo mlatu cu blabi` | R | R | R | A `QSUM/PaRun/OUTQ/sel` | A `QSUM/ZPri/PaRun/OUTQ/sel` !zantufa-mex | R | R |
| `vei re ve'o vei ci ve'o lo mlatu cu blabi` | R | R | R | A `QSUM/Mekso/PaRun/OUTQ/sel` | A `QSUM/ZPri/PaRun/OUTQ/sel` !zantufa-mex | R | R |
| `vei re ve'o re lo mlatu cu blabi` | R | R | R | A `QSUM/Mekso/PaRun/OUTQ/sel` | A `QSUM/ZPri/PaRun/OUTQ/sel` !zantufa-mex | R | R |
| `re boi ci mlatu cu blabi` | R | R | R | A `QSUM/PaRun/NOGAD` | A `QSUM/ZPri/PaRun/NOGAD` !zantufa-mex | R | R |
| `re lo mlatu cu blabi` | A | A | A | A `OUTQ/PaRun/sel` | A `OUTQ/ZPri/PaRun/sel` !zantufa-mex | A `OUTQ/PaRun/sel` | A `OUTQ/PaRun/sel` |
| `re re lo mlatu cu blabi` | A | A | A | A `OUTQ/PaRun/sel` | A `OUTQ/ZPri/PaRun/sel` !zantufa-mex | A `OUTQ/PaRun/sel` | A `OUTQ/PaRun/sel` |
| `so'i lo mlatu cu blabi` | A | A | A | A `OUTQ/PaRun/sel` | A `OUTQ/ZPri/PaRun/sel` !zantufa-mex | A `OUTQ/PaRun/sel` | A `OUTQ/PaRun/sel` |
| `re lo ci mlatu cu blabi` | A | A | A | A `OUTQ/PaRun/q+sel` | A `OUTQ/ZPri/PaRun/q+sel` !zantufa-mex | A `OUTQ/PaRun/q+sel` | A `OUTQ/PaRun/q+sel` |
| `re la djan cu blabi` | A | A | A | A `QSUM/PaRun` | A `OUTQ/ZPri/PaRun/sel` !cbm-cmevla-selbri-word,!cbm-la-name-as-description,!zantufa-mex | A `QSUM/PaRun` | A `OUTQ/PaRun/sel` !cbm-cmevla-selbri-word,!cbm-la-name-as-description |
| `lo re lo mlatu ku cu blabi` | A | A | A | A `GAD/q+sum/PaRun/sel` | A `GAD/q+sum/ZPri/PaRun/sel` !zantufa-mex | A `GAD/q+sum/PaRun/sel` | A `GAD/q+sum/PaRun/sel` |
| `lo su'o lo mlatu ku cu blabi` | A | A | A | A `GAD/q+sum/PaRun/sel` | A `GAD/q+sum/ZPri/PaRun/sel` !zantufa-mex | A `GAD/q+sum/PaRun/sel` | A `GAD/q+sum/PaRun/sel` |
| `vei pa su'i re ve'o lo mlatu cu blabi` | A | A | A | A `OUTQ/Mekso/PaRun/sel` | A `OUTQ/ZPri/PaRun/sel` !zantufa-mex | A `OUTQ/Mekso/PaRun/sel` | A `OUTQ/Mekso/PaRun/sel` |
| `vei pa su'i re ve'o mlatu cu blabi` | A | A | A | A `NOGAD/Mekso/PaRun` | A `NOGAD/ZPri/PaRun` !zantufa-mex | A `NOGAD/Mekso/PaRun` | A `NOGAD/Mekso/PaRun` |
| `lo vei pa ve'o vei re ve'o lo gerku cu blabi` | A | A | A | A `GAD/q+sum/Mekso/PaRun/OUTQ/sel` | A `GAD/q+sum/ZPri/PaRun/OUTQ/sel` !zantufa-mex | A `GAD/q+sum/Mekso/PaRun/OUTQ/sel` | A `GAD/q+sum/Mekso/PaRun/OUTQ/sel` |
| `re mlatu ku poi mi zgana ku'o poi do zgana ku'o cu blabi` | R | R | A | A `NOGAD/PaRun` | A `NOGAD/ZPri/PaRun` !zantufa-mex,!zantufa-selbri-relative-placement | A `NOGAD/PaRun` | A `NOGAD/PaRun` !zantufa-selbri-relative-placement |
| `re mlatu ku poi mi zgana ku'o cu blabi` | A | A | A | A `NOGAD/PaRun` | A `NOGAD/ZPri/PaRun` !zantufa-mex | A `NOGAD/PaRun` | A `NOGAD/PaRun` |
| `lo mlatu ku poi mi zgana ku'o poi do zgana ku'o cu blabi` | R | R | A | R | A `GAD/sel` !zantufa-selbri-relative-placement | R | A `GAD/sel` !zantufa-selbri-relative-placement |
| `lo mlatu poi blabi ku'o cu blabi` | A | A | A | A `GAD/sel` | A `GAD/sel` | A `GAD/sel` | A `GAD/sel` |
| `lo goi ko'a ge'u mlatu cu blabi` | A | A | A | A `GAD/sel` | A `GAD/sel` | A `GAD/sel` | A `GAD/sel` |
| `lo mi goi ko'a ge'u mlatu cu blabi` | A | A | A | A `GAD/LEAD/sel` | A `GAD/LEAD/sel` | A `GAD/LEAD/sel` | A `GAD/LEAD/sel` |
| `lo li pa mlatu cu blabi` | A | A | A | A `GAD/LEAD/PaRun/sel` | A `GAD/LEAD/PaRun/sel` | A `GAD/LEAD/PaRun/sel` | A `GAD/LEAD/PaRun/sel` |
| `lo by mlatu cu blabi` | A | A | A | A `GAD/LEAD/sel` | A `GAD/LEAD/sel` | A `GAD/LEAD/sel` | A `GAD/LEAD/sel` |
| `lo zo ba mlatu cu blabi` | A | A | A | A `GAD/LEAD/sel` | A `GAD/LEAD/sel` | A `GAD/LEAD/sel` | A `GAD/LEAD/sel` |
| `lo lu mi klama li'u mlatu cu blabi` | A | A | A | A `GAD/LEAD/sel` | A `GAD/LEAD/sel` | A `GAD/LEAD/sel` | A `GAD/LEAD/sel` |
| `lo la'e di'u mlatu cu blabi` | A | A | A | A `GAD/LEAD/sel` | A `GAD/LEAD/sel` | A `GAD/LEAD/sel` | A `GAD/LEAD/sel` |
| `lo na'e bo mi mlatu cu blabi` | A | A | A | A `GAD/LEAD/sel` | A `GAD/LEAD/sel` | A `GAD/LEAD/sel` | A `GAD/LEAD/sel` |
| `lo la djan mlatu cu blabi` | A | R | R | A `GAD/LEAD/sel` | R | A `GAD/LEAD/sel` | R |
| `lo lo'oi mi klama ku'au mlatu cu blabi` | R | A | A | A `GAD/LEAD/sel` !loh-oi-bridi-description | A `GAD/LEAD/sel` !loh-oi-bridi-description | A `GAD/LEAD/sel` !loh-oi-bridi-description | A `GAD/LEAD/sel` !loh-oi-bridi-description |
| `lo re moi mlatu cu blabi` | A | A | A | A `GAD/sel` | A `GAD/sel` | A `GAD/sel` | A `GAD/sel` |
| `lo re roi mlatu cu blabi` | A | A | A | A `GAD/sel` | A `GAD/sel` | A `GAD/sel` | A `GAD/sel` |
| `la djan cu blabi` | A | A | A | A | A `GAD/sel` !cbm-cmevla-selbri-word,!cbm-la-name-as-description | A | A `GAD/sel` !cbm-cmevla-selbri-word,!cbm-la-name-as-description |
| `li fu'a pa boi re su'i lo'o` | A | A | A | A `PaRun` | A `PaRun` | A `PaRun` | A `PaRun` |
| `li lu'e fu'a pa boi re su'i lo'o` | R | A | A | R | A `PaRun` !zantufa-mex | R | A `PaRun` !zantufa-mex |
| `li lu'e fu'a pa boi re su'i pi'i ci lo'o` | R | A | R | R | R | R | R |
| `lo ge mi klama gi do bajra cu blabi` | R | A | R | R | R | R | R |
| `lo re ge mi klama gi do bajra ku mlatu cu blabi` | R | A | R | R | R | R | R |
| `lo vei pa su'i re ve'o cu blabi` | R | R | R | R | R | R | R |
| `lo re cu blabi` | R | R | R | R | R | R | R |

Every acceptance cell of the base column was re-measured at `9ec321d530` before the first line of
C-a and matched plan v7's table on all 79 rows, so no cell moved between the plan's measurement
base and this epoch's. **Twenty cells flip, and they are exactly the twenty the plan predicts**:
D1's seven, D3a's two, D3b's two, D3c's three, D4.1's two and D4.2's four. Every other row keeps
its acceptance; the visible node and warning changes on unflipped rows are #634's, which is D2's
whole job.

## Commit map

| commit | SHA | content |
| --- | --- | --- |
| base | `9ec321d530` | the epoch-8 merge's follow-up commit |
| C-a | `b7eb8d019f` | D0: the operand tier, its classifier, both restricted sites, the retired guard, the tombstones, the #884 pins, the genuine-error and R7 witnesses |
| C-b | `4015ee9c78`, `4fb8b8a921` | D2 (#634), replayed from the stage-1 rewritten validation commits `54193b38d1` and `9cf2ac039e` |
| C-c | `486c25f21e` | D4.1's deletion, D4.2's and D1's witnesses, the one corpus reclassification |
| C-d | `0d66e0ccff` | D3a and D3b, their classifier, the guard, the union composite |
| C-e | `539eda3a0a` | D3c, the classifier re-hook, the entry invariant, the populations enumeration |

## Populations

Whole-corpus measurement, base vs head, over both spines (26,678 fixture inputs):

| quantity | measured |
| --- | --- |
| fixtures whose strict-or-recovered output differs at all | 183 |
| of those: acceptance flips | **3** |
| of those: a diagnostic code moves | 5 |
| of those: a rendered tree or context frame moves | 62 |
| of those: only the reported candidate set at a failure offset moves | 113 |
| pre-epoch fixture EXPECTATIONS that change | **8** |
| epoch-new witness fixtures | 91 |

The three acceptance flips are `adhoc/v0/warnings/experimental/simpler-descriptor-head-connective`
(A -> R, D4.1) and `corpus/camxes/8587` and `corpus/camxes/18322` (R -> A, D3a). D1's, D3b's and
D3c's corpus accept populations are all ZERO: their surfaces do not occur in the corpus, and
D3b/D3c are gated besides. That D3c count is scoped to the frozen 26,678-input manifest. The
constructed R4 surface `tirna re poi cmalu kei`, now recorded beside its truth-table row, reaches
the started-`Prefix` classifier row on both feature axes only in an intermediate, globally
unsuccessful trial; its winning tree keeps the relatives field `Valid`.

## Comparer

Same-name, fail-closed both ways, baseline archived from `tests/fixtures` at the epoch base.

| class | old -> new | multiplicity |
| --- | --- | --- |
| `quantifier-retyping` (#634) | `ZantufaPriorityRawMeksoQuantifier` over a lone baseline surface -> `PaRunQuantifier` / `MeksoQuantifier` | 3 fixtures |
| `quantifier-warning-removal` (#634) | the `experimental-zantufa-mex` diagnostics those positions anchored | 2 fixtures |
| D0 tier restriction (TOPOLOGY) | none | **0, and the zero is STRUCTURAL only** — `description_leading_operand` produces `SumtiBaseSyntax` and adds no tree layer, so no serialized node can change SHAPE because of it. What it does behaviourally is the separately measured C-a recovered-delta enumeration above |
| D3c absent-field rewrite | none | 0 — the frozen sibling-variant shape shipped, so no existing quantifier node gains a field |

Everything else is manual residue with an individual disposition:

| fixture | disposition |
| --- | --- |
| `adhoc/v0/warnings/experimental/simpler-descriptor-head-connective` | acceptance flip A -> R with the deleted route (D4.1); the fixture is re-pinned as a rejection and its tree, semantics and gentufa expectations go with the acceptance |
| `corpus/camxes/18322` | acceptance flip R -> A through D3a, as the recorded union composite (#887) |
| `corpus/camxes/8587` | acceptance flip R -> A through D3a; std R / **exp A** / zan A, so a direct adoption |
| `corpus/camxes/21995` | still rejects; the winning diagnostic moves `syntax.unexpected-cmavo @29..31` -> `syntax.incomplete-mekso @31..31`, detailed below |
| `corpus/camxes/5170` | still rejects; the winning diagnostic moves `syntax.unexpected-cmavo @41..42` -> `syntax.incomplete-sumti @52..52`, detailed below |

`corpus/camxes/22368` and `adhoc/syntax/tags/issue-833-stag-position-zantufa-rejected` each move
their winning diagnostic mid-epoch and return to the base code by the head, so they carry no
net delta and do not appear.

### The two moved error frontiers, in detail

A moved frontier is exactly what a count cannot police, so both signatures are PINNED in the
comparer, on both sides, as `PINNED_DIAGNOSTIC_MOVES`: a different move at the same fixture is a
comparer failure rather than unnamed residue. Both moves were bisected by running the per-commit
binaries over the two texts, and both land at C-d on the DEFAULT axis, which makes D3a the route
that moves them — D3b is gated off there.

**`corpus/camxes/21995`**, `noltroni'u               palace`, which morphology segments as
`noltroní'u pa la ce`. The base reports TWO diagnostics: `syntax.unexpected-cmavo @29..31` at
`ce`, raised inside the description opened at `la`, whose expected set reads *free modifier or
quantifier* — the `quantifier` candidate is exactly what D0 removes from the leading operand — and
`syntax.incomplete-mekso @31..31` at end of input, raised inside a forethought connective, because
`ce` is a JOI word and a `joik GI` gek wants its GI. The head reports the SECOND of those two,
unchanged in code and in byte offset. No frontier is invented: with C-d adding a sibling
description-tail route at the same position, the shallower description candidate stops being the
reported one, and what remains is the deeper frontier the base already reported. Acceptance is
unchanged (reject at both), and the fixture's morphology and vlasei expectations do not move.

**`corpus/camxes/5170`**, `to mi nelci le tercru be me'e la bysydy .e la xypapa`. The base stops at
the `.e` connective, `syntax.unexpected-cmavo @41..42`, raised *while parsing description tail*:
the baseline leading operand is one `sumti_6` and cannot carry a connection, so `la bysydy .e la
xypapa` is not a leading element the baseline route can take. D3a's
`exp_full_sumti_description_tail` admits a FULL sumti there, the connection is consumed, and the
frontier moves to end of input — `syntax.incomplete-sumti @52..52`, still *while parsing
description tail*, now with `sumti relative phrase` in its expected set because the parser is
inside a whole `sumti`. The new frontier is correct precisely because the extent that used to be
unparsable now parses as far as the missing tail body; the surface still rejects, because D3a's
tail body is mandatory and there is none.

## The recovery-anchor snapshot, dispositioned by rule block

`crates/jbotci-syntax/tests/recovery-anchor-metadata.snapshot.txt` is byte-identical across the
C-a -> C-b replay, which is #634's standing rule. For C-a, C-c, C-d and C-e it takes a reviewed,
source-derived delta instead, and every changed line is covered by a row below. Propagated
FIRST-set lines under an existing rule are dispositioned per rule block rather than per line;
added and removed BLOCKS are dispositioned individually, and each new anchor rule names the
recovered witness that exercises it.

### C-a — rules 628 -> 629

| rule block | old -> new | lines | source-derived reason | recovered witness |
| --- | --- | --- | --- | --- |
| `description_leading_operand` | **added** | +10 | the named rule gets its own parser identity, and therefore its own FIRST set, elidable-terminator analysis and recovery metadata — the identity #552 asks for | `d0-recovered-through-leading-operand`, `d0-recovered-through-quantified-operand`, `d0-recovered-leading-operand-refuses-sumti5`, `d0-recovered-quantified-operand-refuses-sumti5` |
| `description_tail_sumti` | changed | +1 `field` / -1 `field`, +10 `resume` / -10 `resume` | the field INDEX shifts by one as `assert !pa_word()` retires; the field's name, its resume origins and its start sets are unchanged, only the index they hang from moves | the same four |

No other block changes, and no FIRST-set line moves anywhere: the restricted alias accepts a
SUBSET of `sumti_base`'s arms, and a subset cannot widen a FIRST set.

### C-c — rules 629 -> 627

| rule block | old -> new | lines | source-derived reason | witness |
| --- | --- | --- | --- | --- |
| `description_head_connective` | **removed** | -3 | D4.1 deletes the unsourced head-connective production outright | `d4-description-head-connective-rejected{,-zantufa}` |
| `description_connection_sumti` | **removed** | -89 | the `sumti_base` arm the deleted production fed | `d4-description-head-ja-connective-rejected{,-zantufa}`, `d4-description-head-connective-sa-erasure-rejected{,-zantufa}` |

Removal-only: no existing block gains or loses a line, which is the shape a deletion of two
productions with no other consumer must have.

### C-d — rules 627 -> 631

| rule block | old -> new | lines | source-derived reason | recovered witness |
| --- | --- | --- | --- | --- |
| `exp_full_sumti_description_tail` | **added** | +72 (17 `first`, 2 `field`, 53 `resume`) | D3a's tail production; its FIRST set is the FIRST set of `sumti`, which is why this block alone carries 17 conditioned FIRST lines | `d3a-recovered-exp-only-leading-sumti`, `d3a-recovered-baseline-derivable-leading-sumti`, `d3a-recovered-unproven-leading-sumti`, `d3a-recovered-unproven-prefix-leading-sumti` |
| `exp_descriptor_with_leading_sumti_sumti` | **added** | +42 (1 `first`, 3 `field`, 38 `resume`) | D3a's `sumti_base` arm; one FIRST line because a descriptor opens on `description_head()` alone | the same four, plus `d3a-recovered-quantifier-guard-control` |
| `zantufa_relatives_first_description_tail` | **added** | +93 (1 `first`, 3 `field`, 89 `resume`) | D3b's tail production, whose mandatory leading relatives give it the relative-list FIRST set and its resume anchors | `d3b-zantufa-relatives-first-leading-sumti{,-zantufa,-flag}` |
| `zantufa_descriptor_with_relatives_first_sumti` | **added** | +10 (1 `first`, 3 `field`, 6 `resume`) | D3b's gated `sumti_base` arm | the same three |
| 198 existing blocks | changed | +699 lines, **-0** | FIRST-set propagation of the gated D3b arm's opener through the `sumti_base` cone: **every one of the 699 added lines carries `Feature(ZantufaDescriptions)` in its conditions** (465 alone, 152 with `ZantufaConnectives`, 41 with `ZantufaTerms`, 41 with both), and not one line is removed or altered | the D3b witnesses above |

The 198-block propagation is the mechanism working rather than a surprise: a new `when
feature(..)` arm of `sumti_base` adds a conditioned FIRST line at every rule that can reach
`sumti_base`, and nothing else. D3a's arm adds no unconditioned opener anywhere, because
`description_head()` is already in `sumti_base`'s FIRST set through `descriptor_with_gadri_sumti`.

### C-e — rules 631 -> 636

| rule block | old -> new | lines | source-derived reason | recovered witness |
| --- | --- | --- | --- | --- |
| `zantufa_priority_raw_mekso_quantifier_with_relatives` | **added** | +26 | D3c's priority with-relatives sibling variant | `d3c-recovered-r2-valid-relatives-{zantufa,flag}`, `d3c-recovered-r4-prefix-trial-relatives-{zantufa,flag}`, `d3c-recovered-i1-synthesized-opener-{zantufa,flag}`, `d3c-recovered-r5-synthesized-relatives-{zantufa,flag}` |
| `zantufa_raw_mekso_quantifier_with_relatives` | **added** | +26 | the recovered-fallback twin, so a repaired candidate keeps the source shape | the same eight, through the fallback route |
| `zantufa_priority_raw_mekso_quantifier_candidate` | **added** | +11 | F3's tree-transparent eligibility alias, which is what gives the #634 classifier a RULE-level product to inspect | `d3c-recovered-r1-absent-relatives-{zantufa,flag}` |
| `zantufa_priority_raw_mekso_quantifier_with_relatives_candidate` | **added** | +11 | G3's eligibility alias carrying the startedness test | `d3c-recovered-r4-prefix-trial-relatives-{zantufa,flag}`, `d3c-recovered-r5-synthesized-relatives-{zantufa,flag}`, `d3c-recovered-i1-synthesized-opener-{zantufa,flag}` |
| `zantufa_raw_mekso_quantifier_with_relatives_candidate` | **added** | +11 | the same alias over the fallback variant | the same six |
| existing blocks | unchanged | **+0 / -0** | the new productions are reached only through the `quantifier` sum, whose FIRST set already contains every opener they can take | — |

Additive-only, with zero propagation: C-e adds five blocks and touches no existing line.

## Gate

| row | result |
| --- | --- |
| `cargo fmt --all --check` | green |
| `cargo test -r --workspace --no-fail-fast` | green, 103 targets, 0 failed |
| expensive contracts, all targets, release, `--no-fail-fast` | green at the earlier full-series candidate, 70 targets, 0 failed; owner-ruling final-candidate rerun belongs to the lead's single heavy gate |
| `fixture-test --profile all` | 26,786 fixtures, 3 facets, 72,731 passed, 519 xfailed, 0 failed at the earlier full-series candidate; owner-ruling final-candidate rerun belongs to the lead's single heavy gate |
| tagged facet `descriptions-epoch` | 108 fixtures, 3 facets, 108 passed, 0 failed |
| tagged facet `descriptions-epoch`, `--facet syntax` | 108 fixtures, 1 facet, 108 passed, 0 failed |
| comparer | 8 changed / 3 + 2 mechanical / 5 manual / 0 prose / 108 epoch-new / 0 unpaired / 0 witness deltas; both pinned diagnostic moves verified on both sides |
| comparer unit tests | 18 tests, green |
| round-5 light re-gate after the E1 correction | fmt green; workspace release tests green with `--no-fail-fast`; tagged facets 108/108 in both forms; comparer 8 changed / 3 + 2 mechanical / 5 manual / 0 prose / 108 epoch-new / 0 unpaired / 0 witness deltas; 18 comparer unit tests green |
| `cargo build -p jbotci` (debug) | green |
| `dx build` | green |
| `maturin develop` | green |
| `generate_syntax_models.py --check` | green after regeneration |
| `generate_domain_enum_stubs.py --check` | green |
| `compose_stubs.py --check` | green after regeneration |
| `generate_api_matrix.py --check` | green after regeneration |
| `recovery-anchor-metadata.snapshot.txt` across the C-a -> C-b replay | byte-identical, regenerated at both commits and compared; re-verified in the fix round, which changes no grammar rule, so the committed file is identical at C-a and at both C-b commits |
| recovery-metadata delta, C-a | rules 628 -> 629: the named rule's own block, and `description_tail_sumti`'s field index shifting as its `assert !pa_word()` retires |
| recovery-metadata delta, C-c | rules 629 -> 627, removal-only: the two deleted productions and nothing else |
| recovery-metadata delta, C-d | rules 627 -> 631: the four new productions, and the gated arm's `Selmaho(La), Selmaho(Le)` opener propagated under `Feature(ZantufaDescriptions)` to the sites the `sumti_base` sum reaches |
| recovery-metadata delta, C-e | rules 631 -> 636, additive-only: the two with-relatives rules and the three candidate aliases |
| peak RSS, full profile | base 6,312,804 KB -> 6,415,332 KB, **+1.62%** (gate: base +20%) |
| GitHub `Test` workflow on the pushed branch | all 22 checks pass, `Cargo tests` included, at the shipped `CARGO_BUILD_JOBS: 2` and with no OOM; measured over two pushes of this series at 1h10m42s and 50m52s. One macOS wheel leg failed once on a `files.pythonhosted.org` read timeout and passed on re-run |
| peak COMPILE RSS of the CI `Cargo tests` command, `-j4`, cold | largest single process 9.55 GB (base) -> 9.62 GB (head), **+0.7%**; peak concurrent 14.52 GB -> 17.40 GB, which is a scheduling difference against a 16 GB runner rather than a per-unit regression (see the CI section) |

Both `cargo test` components run `--no-fail-fast` deliberately: `cargo test` abandons the run at
the first failing target, so a red gate without it reports a lower bound on the failing set rather
than the set itself.

The peak-RSS pair is one volume, both sides warm, with the writers built outside the measured
window and the base side built from `9ec321d530` into its own target directory.

Four rows were red on the first pass and are recorded as such. The struct-invariant audit and
`jbotci-gentufa`'s recovered-slot test both move with the epoch and are updated with their reasons
in C-f. The two `zantufa_quantifier_relatives` unit tests that build a `SkippedTokens` item failed
only under the expensive-contracts feature, because the helper segmented each word separately and
the item's own expensive invariant requires ordered source attribution; the helper now segments
one text in one pass. `dx build` and `maturin develop` failed once on `No space left on device`
and passed after two retired lanes' build trees were purged from `/build`; the round-1 fix round
needed the same remedy and purged the retired `issue-666` and `issue-869` trees, 78 GB, which is
what its own gate and RSS figures were measured against.

## CI: why the `Cargo tests` job ran out of memory, and what fixed it

The GitHub `Test` workflow's `Cargo tests` job died with exit 137 — a runner OOM kill — twice at
the round-1 candidate, about two and a half minutes into
`cargo test -r --workspace --features jbotci-dictionary/import`, i.e. during COMPILATION, while
the epoch base passes the same job. It was measured rather than assumed.

Both figures below are of that exact command with a cold target directory and
`CARGO_BUILD_JOBS=4`, which is what the workflow sets. `/usr/bin/time -v` reports the peak of the
largest single process; a two-second sampler over every process in the target directory reports
the peak CONCURRENT total, which is what an OOM kill actually turns on.

| measurement | epoch base `9ec321d530` | round-1 candidate `567cec376e` | delta |
| --- | --- | --- | --- |
| `/usr/bin/time -v` maximum RSS | 10,014,912 KB (9.55 GB) | 10,089,444 KB (9.62 GB) | **+0.7%** |
| peak CONCURRENT RSS, cold, `-j4` | 14.52 GB | 17.40 GB | +2.88 GB |
| peak per-crate RSS, warm re-build of the syntax cone | `jbotci_syntax` 8.79 GB | `jbotci_syntax` 8.85 GB | **+0.7%** |
| peak CONCURRENT RSS, warm re-build of the syntax cone | 10.38 GB | 9.57 GB | -0.81 GB |

**`quantifier` joining the `recursive {}` block is not the cause.** No compilation unit gets
materially bigger: the dominant one, `jbotci-syntax`, moves 8.79 -> 8.85 GB, and the largest
single rustc process anywhere in the workspace moves 9.55 -> 9.62 GB. Outside `jbotci-syntax` the
largest per-crate delta is 0.13 GB, on the `zantufa_parity` test target. Removing the entry would
cost either five more threaded parameters through the same cone or the loss of a FROZEN
`recursive_output` declaration, for a measured benefit indistinguishable from zero.

**What the OOM turns on** is that this workspace compiles a ~8.8 GB rustc unit — at the base as
much as at the head — on a 16 GB runner, four jobs at a time. Whether the total crosses 16 GB is
then a scheduling question: the cold base run happened to keep that unit away from the heavier
dependents and peaked at 14.5 GB; the cold head run did not and peaked at 17.4 GB. The epoch's
+0.7% did not create that margin, it consumed what was left of it.

**The fix is where the cause is.** `CARGO_BUILD_JOBS` for the `Cargo tests` job drops from 4 to 2,
with the measurement recorded in the workflow beside it. Verified by re-running the same cold
command at the fixed head with the new setting:

| measurement | head, `-j4` | head, `-j2` (as shipped) |
| --- | --- | --- |
| `/usr/bin/time -v` maximum RSS | 9.62 GB | **8.48 GB** |
| peak CONCURRENT RSS | 17.40 GB | **14.07 GB** |
| exit status | 0 (on this 62 GB box) | 0 |
| wall clock | 18:22 | 31:12 |

14.07 GB is below the 14.52 GB the epoch base peaks at under `-j4`, which is the schedule CI
passes today, so the shipped setting is inside the envelope the runner is already known to
tolerate. The two units that dominate are `jbotci-syntax` and `bindings/python`'s `_native`
extension, each near 8.8 GB; with two jobs they can still partially overlap, which is why the
number does not fall to one unit. Reducing either unit is a real and separate problem, and it is a
pre-epoch-8 property of the generated parser and its binding rather than anything this epoch
introduces. The ~13 minutes of extra wall clock is the price of not being schedule-dependent.

## Follow-up issues filed by this epoch

| issue | subject |
| --- | --- |
| **#885** | remove the four never-emitted `Experimental*SimplerConnective` categories together, with the Python mapping and the parity inventory |
| **#886** | camxes-exp's `quantifier gek_sentence` leading element, excluded by D3a's `!quantifier` guard |
| **#887** | emergent union composites: extents jbotci accepts that no single reference parser accepts |
| **#888** | camxes-exp's raw-mex quantifier gate `!selbri !sumti_6 mex`, unadopted, with a guard that differs from Zantufa's |

#884 was filed during planning and is carried by two dependency pins rather than fixed here.
