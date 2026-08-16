# Grammar parity epoch 6b: remaining term scope

This note records the durable implementation dispositions for the half of the
term-hierarchy epoch that epoch 6a deferred: #794 (GOI and flavour-context
payload width), the standard-termset half of #806, and #827 (rolling-Zantufa
term binding). The implementation base is main
`3c3b84a5bae715c18ae221433f258733cfa0ee69`, the epoch-6a merge. The
authoritative design is sections D3, D4, D5 and C7 of the panel-reviewed epoch-6
plan v2 in the grammar-review repository, as amended by its reconciliation
(B1/B3/B5/B6 and Kimi's R2 qualification) and by the 6b addendum.

Running-parser probes use `camxes.js`, `camxes-exp.js`, and rolling Zantufa
`zantufa-1.9999.js` at the snapshots epoch 6a recorded (ilmentufa
`778ea138f7d150121ca722db7536ce3b123943ac`, gerna_cipra
`d5a5065c924304cf5e9067ee6d41b584fbe1c099`). A pinned reference tree is accepted
only after rerunning the applicable parser.

## D3 (#806): the three standard termset shapes

camxes-standard composes three distinct termset shapes and camxes-exp composes
the same three over its normal-flavour term:

| Shape | Upstream | Operands | NUhU slots |
| --- | --- | --- | --- |
| NUhI-gek | `NUhI free* gek terms NUhU? free* gik terms NUhU? free*` (camxes.peg:136) | full **guarded** `terms` sequences (B1) | yes |
| NUhI-plain | `NUhI free* terms NUhU? free*` (camxes.peg:136) | full guarded `terms` | yes |
| NUhI-less `gek_termset` | `gek_termset <- gek terms_gik_terms` (camxes.peg:136-138, camxes-exp.peg:191-193) | one **unguarded** term per position | **no** |

`terms_gik_terms <- nonabs_term (gik / terms_gik_terms) nonabs_term` pairs by
nesting rather than by concatenation: each level contributes one operand before
its centre and one after it, so an n-operand termset nests n/2 deep and the
outermost operands are the outermost pair. camxes-standard spells the operand
`nonabs_term`; camxes-exp spells it its normal-flavour `term`, which is the same
unguarded leaf inventory with the loose and optional-stag BO tiers over it. The
jbotci level is the union, which at this commit is the 6a `nonabs_term` level;
D4 widens the BO tier of that position to the normal flavour.

The GIK alternative is listed before the recursive one, exactly as upstream
orders it, so the innermost pair is the one that finds the GIK.

### What this corrects

The NUhI-less shape was already *reachable* at the 6a base, but through the
optional-NUhI path of `forethought_termset`, which reads it with a flat branch
shape no upstream parser produces. The 6a ledger's boundary argument — that
"every 6b arm is unreachable on a surface that parses today" — is therefore not
correct for this arm, and the correction is a tree change on surfaces that
already parse rather than a new acceptance. It is nevertheless not a
reinterpretation under the standing ruling, which protects sourced baseline
parses: no parser sources the flat reading.

Probed at base `3c3b84a5ba` and after the change:

| Surface | camxes-standard | jbotci at base | jbotci now |
| --- | --- | --- | --- |
| `ge ko'a gi pu broda` | `gek_termset`, unguarded tag operand with elided KU | rejects | parses, same shape |
| `ge ba ko'a gi ca ko'e broda` | `gek_termset`, two tag operands | flat `ForethoughtTermset` | `gek_termset` |
| `ge pu ko'a pu ko'e gi pu ko'i pu ko'u broda` | nested `A (B gik C) D` | flat 2 + 2 | nested `A (B gik C) D` |
| `ge ko'a gi ko'e broda` | baseline GEK **sumti** connection at `sumti_4` | sumti-owned | sumti-owned |
| `ge ko'a gi ko'e .e ko'i broda` | sumti-owned, `.e` outside the connection | sumti-owned | sumti-owned |

Exactly three pre-existing expectations move to the sourced shape, all of them
NUhI-less GEK termsets: `corpus/camxes/644`, `corpus/camxes/2481` and
`corpus/alis/full-alice`. Their `semantics.refs`, `tersmu-json` and `output`
facets are unaffected; only the pinned syntax tree changes. They are regenerated
with the epoch's single consolidated expectation update.

### Witnesses

| Fixture | Surface | What it pins |
| --- | --- | --- |
| `gek-termset-nuhi-less-tag-operand` | `ge ko'a gi pu broda` | the unguarded operand: a bare tag with an elided KU, which the guarded `terms` sequence would refuse |
| `gek-termset-balanced-nesting` | `ge pu ko'a pu ko'e gi pu ko'i pu ko'u broda` | `A (B gik C) D`, not the flat 2 + 2 jbotci used to build |
| `gek-termset-baseline-sumti-owned` | `ge ko'a gi ko'e broda` | the whole-candidate classifier: the baseline GEK **sumti** connection keeps the extent |
| `gek-termset-baseline-sumti-owned-continuation` | `ge ko'a gi ko'e .e ko'i broda` | the same, with `.e` outside the connection |
| `gek-termset-baseline-sumti-owned-zantufa` | `ge ko'a gi ko'e broda` `(zantufa)` | the classifier is not dialect-gated |
| `nuhi-plain-over-nested-gek-termset` | `nu'i ge ko'a gi pu broda` | the arm-fallback rule (B1/Kimi-3): the pinned tree is `NuhiTermset { nuhi, termset: [GekTermset(..)] }`, because the NUhI-gek arm's guarded operands refuse the surface |
| `gek-termset-branch-connection-formula` | `ge ba ko'a gi ca ko'e broda` | the branch formula: a `termSet`-locus connective formula over two branches, not `invalid_graph` |

The plan asks for a camxes-exp analogue of the arm-fallback pin. Epoch 6a
retired `DialectFeature::TermHierarchy` and made the exp tiers unconditional, so
the default configuration *is* the std/exp union and the single pin covers both;
there is no separate exp configuration left to write.

### The gek-termset versus gek-sumti mechanism (B6)

`gek_termset <- gek terms_gik_terms` and the baseline GEK sumti connection
`sumti_4 <- sumti_5 / gek sumti gik sumti_4` both begin `GEK … GIK …` and cover
the identical extent on `ge ko'a gi ko'e broda`, which both upstream parsers
give to the sumti connection. The termset arm therefore carries a
whole-candidate `reject_output` classifier in the `baseline_*.rs` pattern
(`crates/jbotci-syntax/src/grammar/baseline_termset.rs`), not merely a later
position in the arm order: the sumti term is listed earlier at every level, but
a locally failing outer parse backtracks into the termset arm, which would then
reclaim an extent the baseline had already covered.

A candidate is baseline-owned exactly when its operand tree is one GIK-paired
level whose two operands are both bare sumti terms. The extent proof and the
exhaustive `..`-free destructuring are recorded in the module's own
documentation.

## The `forethought_termset` split (lead ruling: option B)

The optional-NUhI `forethought_termset` node carried two widenings that nothing
sources. Both were measured against all three running parsers:

| Surface | camxes-standard | camxes-exp | rolling Zantufa | jbotci at `1a4a8914bb` | jbotci now |
| --- | --- | --- | --- | --- | --- |
| `ge ko'a nu'u gi ko'e broda` | rejects | rejects | rejects | **accepts** | rejects, every profile |
| `nu'i ge A gi B gi C broda` (n-ary NUhI-present) | rejects | rejects | no NUhI selma'o at all | **accepts** (`+zantufa-connectives`) | rejects, every profile |

Rolling Zantufa has neither NUhI nor NUhU: `nu'i` lexes as KE there, and its own
NUhI-less termset is `gek_term <- gek term+ (gik term+)+ GIhI?`
(zantufa-1.9999.peg:32), with `term+` branches and no terminator slot. So the
optional-NUhI node's NUhU slots were sourced by nothing, and its Zantufa n-ary
branches and GIhI could not be sourced in the NUhI-present arm they sat in.

The lead ruled option B. `forethought_termset` is now NUhI-**mandatory** — the
sourced NUhI-gek arm and nothing else, its `m_nuhi` field replaced by a
mandatory `nuhi` and its `additional_branches` and `gihi` fields removed — and
rolling Zantufa's own shape becomes its own `ZantufaConnectives`-gated arm,
`zantufa_gek_termset`, ordered behind the sourced `gek_termset` at every level
that offers both. That is D3's "three distinct shapes" read literally, with the
dialect's fourth shape gated as a dialect.

### What the Zantufa arm is, and what it is not

| Property | sourced `gek_termset` | `zantufa_gek_termset` |
| --- | --- | --- |
| Gate | none (union default) | `ZantufaConnectives` |
| Operand position | one **unguarded** term | a whole `term+` run |
| Branch count | two, paired by nesting | n-ary, in source order |
| Terminator | none | optional GIhI |
| Warning | none | `syntax.warning.experimental-zantufa-gek-termset` on the arm, plus the shared n-ary GI warning per branch beyond the first, plus the shared GIhI warning |

The arm owns no token of its own — its GEK and its first GIK are the shapes every
other forethought connection spells, and its GIhI is elidable — so it is
diagnosed post-parse by `GeneratedConstructWarningVisitor`, anchored at the GEK
that opens it, which is the T3 mechanism epoch 6a established.

Zantufa's `gik <- GI_clause` (zantufa-1.9999.peg:72) carries no NAI, because
Zantufa has no NAI selma'o at all. `ge ko'a ko'e gi nai ko'i broda` nevertheless
parses there, with `nai` absorbed as a UI free modifier, so the first branch
spells its connective the shared `gik_connective` and jbotci reads the NAI as a
NAI. That is a reading difference on a surface both accept, not an acceptance
widening; the gap row is below.

### The Zantufa GEK-sumti ownership question (B6, one branch wider)

Zantufa spells the GEK sumti connection n-ary as
`sumti_3 <- (… / gek sumti (gik sumti)+ GIhI?) relative_clauses?`
(zantufa-1.9999.peg:36), and gives `ge ko'a gi ko'e gi ko'i broda` to it rather
than to `gek_term` — probed, not assumed. The new arm therefore carries its own
whole-candidate `reject_output` classifier, `ZantufaBaselineGekSumtiRejection`,
built on the same extent argument as the binary one: a candidate is
sumti-connection-owned exactly when every operand position holds one bare sumti
term, because `gek sumti (gik sumti)+` then reconstructs the identical extent,
and no other shape has a counterpart in the sumti connection's branches. The
GIhI slot does not enter the argument, since the sumti connection carries one
too. Both classifiers are destructured exhaustively and without `..`.

### Moved surfaces

| Surface | Before | After |
| --- | --- | --- |
| `ge ko'a nu'u gi ko'e broda` | flat `ForethoughtTermset` | rejects (`syntax.unexpected-cmavo` at `nu'u`) |
| `nu'i ge ko'a gi ko'e gi ko'i broda` | `ForethoughtTermset` with `additional_branches`, `+zantufa-connectives` | rejects |
| `ge ko'a ko'e gi ko'i broda` (default) | flat `ForethoughtTermset` | rejects, as both camxes parsers do |
| `ge ko'a ko'e gi ko'i broda` (`+zantufa-connectives`) | flat `ForethoughtTermset`, no warning | `ZantufaGekTermset`, warned |
| `ge ko'a gi ko'e gi ko'i broda` (`zantufa`) | baseline n-ary GEK **sumti** connection | unchanged; now protected by the classifier rather than by arm order alone |
| 14 fixture files carrying a `ForethoughtTermset` expectation | `m_nuhi`, `additional_branches`, `gihi` in the Debug shape | `nuhi`; the two removed fields gone. Individually-reviewed manual residue in C7, not a mechanical comparer class |

### Witnesses

| Fixture | Surface | What it pins |
| --- | --- | --- |
| `nuhi-less-flat-termset-rejected` | `ge ko'a ko'e gi ko'i broda` | an unbalanced NUhI-less run has no sourced reading; both camxes parsers reject it |
| `nuhi-less-nuhu-rejected` | `ge ko'a nu'u gi ko'e broda` | the NUhU widening is gone |
| `nuhi-less-nuhu-rejected-zantufa` | the same `(zantufa)` | and enabling the Zantufa arm does not restore it |
| `nuhi-nary-termset-rejected-zantufa` | `nu'i ge ko'a gi ko'e gi ko'i broda` `(zantufa)` | the n-ary NUhI-present widening is gone |
| `zantufa-gek-termset-unbalanced` | `ge ko'a ko'e gi ko'i broda` `(+zantufa-connectives)` | the arm, its `term+` runs, and its construct warning |
| `zantufa-gek-termset-terms-feature-off-rejected` | the same `(+zantufa-terms)` | the gate is `ZantufaConnectives`, not `ZantufaTerms` |
| `zantufa-gek-termset-nary-gihi` | `ge ko'a ko'e gi ko'i gi ko'u gi'i broda` `(zantufa)` | the n-ary branch sequence and the GIhI terminator together |
| `zantufa-gek-termset-baseline-sumti-owned` | `ge ko'a gi ko'e gi ko'i broda` `(zantufa)` | the new classifier: the n-ary GEK sumti connection keeps the extent |
| `zantufa-gek-termset-connection-formula` | `ge ba ko'a ca ko'e gi vi ko'i broda` `(zantufa)` | the arm lowers through the shared branch-splicing connection path: one `termSet`-locus `and` over the two spliced bridi |

### Documented gap

| Surface | Rolling Zantufa | jbotci | Disposition |
| --- | --- | --- | --- |
| `ge ko'a ko'e gi nai ko'i broda` | accepts; `nai` is a UI free modifier, since Zantufa has no NAI selma'o | accepts; `nai` is the baseline NAI on the GIK | Documented reading gap, not a flag candidate: the extent and the acceptance agree, and reading NAI as NAI is what jbotci does at every other GIK. No dialect can make `nai` a UI in jbotci's lexer. |

## The branch-formula type boundary, and the refactor that removed it

A NUhI-less GEK termset lowers as a logical *connection*:
`build_generated_forethought_termset_connection_formula` finds the termset in
the sentence's term list and splices each of its two branches into the
surrounding terms, producing two complete bridi term lists that are lowered
separately and joined by the connective. That splice was typed
`Vec<&TermSyntax>`, and it reaches the general bridi lowering path
(`build_selbri_simple_bridi_tail_formula_with_preassigned_arguments`,
`build_term_assignments_for_terms`, `GeneratedTermAssignments<'syntax>`).

`gek_termset` operands are `NonabsTermSyntax`, and they must be: the unguarded
atom is exactly what makes `ge ko'a gi pu broda` parse. There is no reference
conversion between the level enums and there must not be one — mechanism E
re-lists the same leaves at every level precisely so that no level is a wrapper
around another, so a conversion would be a copy. The lead's ruling was to fix the
boundary rather than ship a capability gap or defer it into epoch 7, which
reworks these same lowering paths.

The bridi term list is therefore level-agnostic as of `38b6348c53`, which lands
before any D3 semantic wiring and changes no surface's meaning.
`GeneratedBridiTermRef` is a `Copy` view over the six levels holding each level's
own node, with two projections — `simple` (the leaf) and `grouping` (the
connection tier) — and a `visit_in_order` that dispatches to the underlying node
so nothing that traverses a term list observes a different event stream. Both
projections are level-independent, because the tiers a level admits differ but
the product node a given tier builds is one type across every level offering it;
`simple().is_none() == grouping().is_some()` is written as an invariant. 122
signature sites moved, and the conversions sit where a `Vec` of references was
already being built, so nothing is allocated or cloned that was not before.

Two duplications collapsed as a direct consequence: the five per-level
term-formula-scope walkers became one, and the twice-written "is this term a
forethought termset, directly or wrapped in a degenerate `ConnectedTerm`"
matcher became `generated_forethought_termset_in_term`.

### The branch-membership rule

`GeneratedForethoughtTermsetRef` abstracts the two GEK/GIK-joining shapes behind
`gek()`, `gik()`, `branches()` and `additional_branches()`, so the connection
formula body is shared verbatim between them. For the NUhI-present arm both
branches are written out in source order. For the NUhI-less arm the operand tree
is walked pushing each level's leading operand *before* recursing and its
trailing operand *after*:

> the branch before the GIK is the leading operands read **outermost-first**,
> and the branch behind it is the trailing operands read **innermost-first**.

`ge pu ko'a pu ko'e gi pu ko'i pu ko'u broda` nests `A (B gik C) D` and yields
branches `[A, B]` and `[C, D]`, which is exactly what the flat reading produced
on symmetric termsets. That is not only an argument: `corpus/alis/full-alice`'s
`tersmu-json` expectation was pinned from the *old flat reading*, and it passes
unchanged after the re-shaping. Re-shaping the syntax tree left the semantic
graph byte-identical, which is the strongest available evidence that the
membership rule is the sourced one.

`ge ba ko'a gi ca ko'e broda` now lowers to a connective formula (operator `and`,
connector `ge gi`, locus `termSet`, two atom branches) where it previously
reported the principled `invalid_graph` error "non-sumti term reached sumti
visible-place advancement".

## D4 (#794): the normal-flavour payload constituent

The three profiles spell the GOI payload as

| Profile | Rule | Payload |
| --- | --- | --- |
| camxes-standard | `relative_clause_1 <- GOI_clause free* nonabs_term GEhU?` (camxes.peg:168) | ONE unguarded leaf — camxes-standard has no term-level connective tier at all |
| camxes-exp | `relative_clause_1 <- GOI_clause free* term GEhU?` (camxes-exp.peg:207) | the **normal** flavour: `term_1 <- term_2 (joik_ek … term_2)*` over `term_2 <- term_3 (joik_ek stag? BO term_3)*` over the unguarded `tag_term` leaves (camxes-exp.peg:136-149) |
| rolling Zantufa | `relative_clause <- GOI_clause term GEhU?` (zantufa-1.9999.peg:43) | its own `term <- term_1 (joik_ek term_1)*` over `term_1 <- term_2 (joik_ek? BO term_2)*` (zantufa-1.9999.peg:27-28) — the same shape with the connective ALSO optional, which is D5's connectorless BO |

So the union constituent is one rule family, not three: **the loose tier over a
BO tier whose stag is optional over the unguarded leaf inventory**, and it is now
`normal_term` / `bound_normal_term` / `normal_term_atom` with the two connection
products `connected_normal_term` and `bound_normal_term_connection` between them.
It differs from `nonabs_term` in exactly one place — `nonabs_term`'s BO tier is
`stag_bound_term_connection`, whose stag is MANDATORY because it models
camxes-exp's absorption-safe `abs_term_2 <- abs_term_3 (joik_ek stag BO
abs_term_3)*` (camxes-exp.peg:155). The leaf inventory and the T3 guard are the
ones `simple_term` already lists, with the unguarded `nonabs_tagged_sumti_term`
in place of its absorption-guarded twin.

`bound_linked_term` was the structural template, being the same optional-stag
tier one site over (camxes-exp.peg:255 spells the BE payload with the same normal
`term`). It stays a separate family rather than being unified with this one: its
leaf inventory is the four `linked_sumti` forms, and widening the BE/BEI site to
the shared term inventory is #816's half of the same upstream rule, not #794's.

### Why it is a second ladder rather than a widening of `nonabs_term`

`nonabs_term` is also consumed by the CEhE continuation, and that position is
sourced by camxes-standard's `nonabs_term` and camxes-exp's `abs_term` alone
(camxes.peg:116, camxes-exp.peg:125). Giving the whole ladder an optional-stag BO
tier would admit a surface no parser accepts there. The two ladders share every
leaf and nothing else, which is exactly what mechanism E already does between
`simple_term` and `bound_term`.

All three levels join the `recursive` block, and here that is not only the
combinator-graph economy epoch lesson 11 records: `normal_term`'s own leaf
inventory contains `gek_termset`, whose operands are `normal_term`, so the family
is genuinely cyclic and cannot be reconstructed inline at all.

### What the two sites now take

`sumti_association_relative_clause.sumti` is the constituent, and so are all four
operand positions of `balanced_termset_operands`. The GOI payload is deliberately
ONE term rather than a `terms` run: on `ko'a goi ko'e ce'e ko'i broda`
camxes-standard gives the payload only `ko'e` and leaves `ce'e ko'i` at the
enclosing `terms_2` level with GEhU elided, so neither the CEhE nor the PEhE tier
belongs inside it.

Measured delta — every row re-probed against all three running parsers:

| Surface | camxes-standard | camxes-exp | rolling Zantufa | jbotci at `d80c6a5b57` | jbotci now |
| --- | --- | --- | --- | --- | --- |
| `ko'a goi ko'e broda` | accepts | accepts | accepts | `PlainRelativeSumti` | `SumtiTerm`, same payload |
| `ko'a goi pu broda` (bare tag, elided KU) | accepts, `nonabs_term` tag leaf | accepts | rejects | `TenseTaggedRelativeSumti` | `NonabsTaggedSumtiTerm` |
| `ko'a goi na ku broda` | accepts | accepts | accepts | `NaKuRelativeSumti` | `NaKuTerm` |
| `ko'a goi fa ko'e broda` | accepts, FA is its own `term` arm | accepts, FA is inside `tense_modal` | accepts | `TenseTaggedRelativeSumti` **plus** `syntax.warning.experimental-fa-as-tag` | `PlaceTaggedSumtiTerm`, no warning |
| `ko'a goi ge ko'e gi pu broda` (termset payload) | accepts | accepts | rejects | **rejects** | accepts |
| `ko'a goi ba ko'e .e bo vi ko'i broda` | rejects | accepts | accepts | **rejects** | accepts, warned |
| `ge ko'a gi ba ko'e .e bo vi ko'i broda` | rejects | accepts | accepts | **rejects** | accepts, warned |
| `ge ba ko'a .e bo vi ko'e gi ko'i broda` | rejects | accepts | accepts | **rejects** | accepts, warned |
| `ko'a goi ko'e ce'e ko'i broda` | payload is `ko'e`; `ce'e ko'i` outside | same | — | same | unchanged |
| `ko'a goi ko'e pe'e je ko'i broda` | PEhE outside | same | — | same | unchanged |
| `ko'a goi ko'e .e ko'i broda` | `.e` inside the payload's `sumti_2` | same | — | same | unchanged |
| `ko'a goi ko'e .e bo ko'i broda` | `.e bo` inside the payload's `sumti_3` | same | — | same | unchanged |
| `ko'a goi ge ko'e gi ko'i broda` | baseline GEK **sumti** connection | same | — | sumti-owned | sumti-owned |
| `ko'a goi ko'e ge'u broda` | GEhU closes the payload | same | same | same | unchanged |

The FA row is the one the 6b groundwork recorded as a no-delta pin and is not:
acceptance is unchanged, but the tree and the diagnostics both move, because the
narrow `relative_sumti` node had no FA arm and routed the surface through the
epoch-5 FA-as-tag extension instead. Adopting the shared inventory replaces a
diagnosed extension reading with the sourced one.

### The re-typing, and the lead's option-1 ruling

The `relative_sumti` family the payload replaces had three arms, each the same
*content* as a leaf of the shared inventory under a different product name, and
**646 fixture files** carry a `SumtiAssociationRelativeClause` expectation. The
lead ruled option 1, adopt the shared constituent: #794's acceptance criteria
(SUM-05) explicitly include termset payloads, which extending `relative_sumti` in
place would leave rejected, so that option fails the issue by construction.

The re-typing lands with C7's consolidated regeneration, as a new mechanical
comparer class defined as a one-to-one product-name mapping. Measured populations:
`PlainRelativeSumti` in 606 files, `TenseTaggedRelativeSumti` in 46,
`NaKuRelativeSumti` in 1.

| Old product | New product | Payload relation |
| --- | --- | --- |
| `plain_relative_sumti` | `sumti_term` | identical: both are transparent one-field wrappers over the same `sumti` |
| `na_ku_relative_sumti` | `na_ku_term` | identical values; the second field is named `na_ku` rather than `ku` |
| `tense_tagged_relative_sumti` | `nonabs_tagged_sumti_term` | identical `sumti`; the `tense_modal` gains the term flavour's `LeadingTermTagTenseModalSyntax` wrapper, which is a `TenseModal(..)` layer when the leading-term tag split selects its fallback arm and a DIFFERENT arm otherwise. Only the exact wrapper is mechanical; anything else is manual residue |
| — | `place_tagged_sumti_term` | not a re-typing at all: a FA payload changes arm and loses a warning, so every such fixture is manual residue |

### Semantic lowering

Nothing about GOI's meaning changed. `GeneratedAssociationPayloadRef` projects
the widened payload back onto the four shapes a sumti-association phrase can
read — a plain sumti, a tag-led sumti, a FA-led sumti, and `NA KU` — and every
other leaf of the shared inventory reaches it as `None` and is reported with
`relative phrase payload is not a sumti-association term and is not semantically
lowered yet` rather than silently associating nothing. The FA arm reads the same
payload sumti the tag arm does, so `ko'a goi fa ko'e broda` keeps the assigned
name it had.

The `gek_termset` operands moved from `NonabsTermSyntax` to `NormalTermSyntax`
without touching a single lowering path: `GeneratedBridiTermRef` gained the three
new levels and `GeneratedTermGroupingRef` the two new connection tiers, which is
precisely what the level-agnostic bridi term list from `38b6348c53` exists for.
The six per-level KEhA scans collapsed into one over the view as a direct
consequence, the same way the five term-formula-scope walkers did in D3.

### Witnesses

| Fixture | Surface | What it pins |
| --- | --- | --- |
| `goi-payload-fa-tagged` | `ko'a goi fa ko'e broda` | the sourced FA arm replacing the FA-as-tag extension reading |
| `goi-payload-bare-tag` | `ko'a goi pu broda` | the unguarded tag leaf with an elided KU |
| `goi-payload-na-ku` | `ko'a goi na ku broda` | the shared `NA KU` leaf, still lowering to a negated relative phrase |
| `goi-payload-termset` | `ko'a goi ge ko'e gi pu broda` | the leaf-inventory delta SUM-05 names |
| `goi-payload-stagless-bo` | `ko'a goi ba ko'e .e bo vi ko'i broda` | the optional-stag BO tier, and its construct warning |
| `goi-payload-stagless-bo-zantufa` | the same `(zantufa)` | the constituent carries no dialect gate |
| `goi-payload-cehe-outside` | `ko'a goi ko'e ce'e ko'i broda` | the payload is one term: CEhE stays outside |
| `goi-payload-pehe-outside` | `ko'a goi ko'e pe'e je ko'i broda` | the same, one level up |
| `goi-payload-ek-inside-sumti` | `ko'a goi ko'e .e ko'i broda` | sumti greediness keeps the `.e` connection inside the payload sumti |
| `goi-payload-ek-bo-inside-sumti` | `ko'a goi ko'e .e bo ko'i broda` | the same with a BO: the payload's own BO tier engages only on non-sumti operands |
| `goi-payload-baseline-gek-sumti-owned` | `ko'a goi ge ko'e gi ko'i broda` | the NUhI-less termset classifier reaches inside the widened payload |
| `goi-payload-gehu-terminated` | `ko'a goi ko'e ge'u broda` | the explicit GEhU still closes the payload |
| `gek-termset-operand-stagless-bo-trailing` | `ge ko'a gi ba ko'e .e bo vi ko'i broda` | the operand position takes the same constituent |
| `gek-termset-operand-stagless-bo-leading` | `ge ba ko'a .e bo vi ko'e gi ko'i broda` | the same at the leading operand |

The 6a deferral row for the rolling-Zantufa GOI payload is discharged except for
its connectorless BO, which is D5's arm at the same tier.

## Residue this session cleaned up from the `forethought_termset` split

Four checked-in tests were already failing at `d80c6a5b57`, all of them
consequences of the split that the commit did not carry:

| Test | What moved | Disposition |
| --- | --- | --- |
| `cli::gentufa_detailed_syntax_errors_show_expectation_breakdown` | `{nu'i}` left the detailed expectation vocabulary | assertion dropped |
| `cli::gentufa_syntax_error_labels_unique_current_construct` | the same | assertion dropped |
| `recovery_diagnostics` `SYNTAX_DETAILED_NOTE` | the same, inside a pinned note | note re-pinned |
| `incremental_diagnostics::fixture_sample_gate_passes_…` | `nary-gek-termset` now passes the cross-paragraph gate | reviewed set extended, with the reason recorded at the assertion |

`forethought_termset`'s optional `m_nuhi` was the only source of `{nu'i}` in the
detailed "needs one of" vocabulary; with the NUhI mandatory the arm no longer
records it. The hint list is already a distinctive-marker summary rather than a
complete first set — it does not name KOhA or LE where a sumti is required
either — so this is a vocabulary change and not a diagnostic-quality regression;
`nu'i` remains a valid continuation and still parses.

`"tagged sumti"` left `SYNTAX_CONSTRUCT_METADATA` with `tense_tagged_relative_sumti`,
which was its only parser-wired rule.

## Scope not yet delivered

| Section | Issue | State |
| --- | --- | --- |
| D3 termset shapes | #806 | complete, including the `forethought_termset` split |
| D4 GOI and flavour-context payload width | #794 | complete |
| D5 Zantufa term binding | #827 | **rescoped to epoch 6c**; groundwork probed and recorded below |
| C7 consolidated expectations, comparer re-baseline, ratchet, peak RSS | — | complete; results below |

What each remaining section needs, concretely:

- **D5 (#827).** Connectorless BO at the term and sumti tiers, placed at the
  baseline BO-precedence levels; the arm grammar admits only the
  connector-ABSENT form AND a `#634`-pattern whole-candidate `reject_output`
  classifier rejects any completed candidate carrying a present connective
  (belt on top of the grammar shape — B3). The JAI term (overt sumti / explicit
  KU / elision) with a structural rule-level negative predicate at the named DSL
  site, its three mandatory configuration fixtures, and recovered/elided-KU
  diagnostics. FA joik-chains in the zantufa tag_term atom. The `ce'e`-as-BO
  fidelity note needs one witness pinning baseline CEhE ownership of
  `ko'a ce'e ko'e broda` in the zantufa profile plus a gap ledger row.

  **D5 is rescoped to epoch 6c** (lead ruling). It needs its own re-typing
  regeneration — 34 files for the sumti tier's `bound_sumti_tail` sum and 5 for
  the term tier's `StagBoundTermConnectionSyntax`, measured below — and this
  epoch's expectation update is a *single consolidated* one. Half-landing D5
  would either split that update in two or hold C7 open across another
  implementation section, so #827 carries forward whole rather than partly. The
  groundwork below is probed against all three running parsers at the snapshots
  this note pins, and is the 6c seed: the next epoch starts from measured facts
  rather than re-probing.

### D5 groundwork: the four arms, measured

| Surface | camxes-standard | camxes-exp | rolling Zantufa | jbotci now |
| --- | --- | --- | --- | --- |
| `pu ko'a bo ca ko'e broda` | rejects | rejects | accepts: `term_1 <- term_2 (joik_ek? BO_clause term_2)*`, connective absent, no stag (zantufa-1.9999.peg:28) | rejects, every profile |
| `ko'a bo ko'e broda` | rejects | rejects | accepts: `sumti_2 <- sumti_3 (joik_ek? tag? BO_clause sumti_3)*` (zantufa-1.9999.peg:35) | rejects, every profile |
| `jai ko'a broda` | rejects | rejects | accepts, `tag_term` | accepts `(zantufa)`, warned |
| `jai pu ko'a broda` | rejects | rejects | accepts, `JAI_clause tag? sumti` | accepts `(zantufa)`, warned |
| `jai ku broda` | rejects | rejects | accepts: `tag_term`'s payload is `(sumti / KU_elidible)`, so an EXPLICIT KU is a payload | **rejects** ← D5 delta |
| `jai cu broda` | rejects | rejects | accepts, the same payload ELIDED | **rejects** ← D5 delta |
| `jai broda` | accepts | accepts | accepts as the JAI **selbri** `tanru_unit_1`, not a term | accepts as the JAI selbri |
| `fa je fe ko'a broda` | rejects | accepts | accepts: `FA_clause (joik FA_clause)*` inside `tag_term` (zantufa-1.9999.peg:31) | **rejects** ← D5 delta |
| `ko'a ce'e ko'e broda` | CEhE termset group | CEhE termset group | a **sumti BO connection**: Zantufa lexes `BO <- ce'e / bo` (zantufa-1.9999.peg:529) | CEhE termset group, both profiles |

`jai broda` is why the JAI term needs the structural negative predicate: Zantufa
writes it `(FA_clause (joik FA_clause)* / JAI_clause tag?) !tanru_unit_1 (sumti /
KU_elidible)` (zantufa-1.9999.peg:31), and without the `!tanru_unit_1` guard the
elided-KU payload would swallow the selbri. The guard is the named DSL site the
plan asks for.

The `ce'e`-as-BO row is a meaning-changing reinterpretation of a surface that
already parses, so under the standing ruling it is a documented gap plus a
dedicated flag rather than a baseline re-pin, exactly as the 6a ledger forecast.

One implementation cost is measured rather than estimated: the sumti tier's
connectorless arm cannot be added without re-typing `sumti_bound.bound_tail`,
whose single product `bound_sumti_tail` has to become a sum once a second tail
shape exists — mechanism E forbids the nested-wrapper alternative. That is **34
fixture files** (`BoundSumtiTailSyntax`), a bounded second regeneration rather
than a bulk one; the term tier's arm adds a variant to seven ladder levels and
re-types **5** (`StagBoundTermConnectionSyntax`). Neither number changes the
design; both are recorded so the next session can size its own C7 pass.

## C7: the consolidated regeneration

The comparer is re-baselined to `git archive 3c3b84a5ba tests/fixtures` — epoch
6b's own implementation base, which is the epoch-6a merge — so it stays
git-derivable, which was the 6a round-3 fix. Two consequences follow from moving
the baseline forward rather than reusing 6a's:

- **The four epoch-6a classes must now find nothing.** Their work is already in
  the baseline tree. They stay wired in rather than being deleted, because a
  nonzero incidence would mean the archive is not the tree it claims to be.
- **This epoch's own added fixtures have no baseline entry.** 6a's archive sat
  at its C1-C6 tip, after its witnesses landed, so it had no such population.
  6b's does. They are identified from `git diff --diff-filter=A EPOCH_BASE..HEAD`
  rather than from mere absence, listed in their own pinned category, and never
  classified — there is nothing to classify them against. A candidate the archive
  lacks that git does not record as added by this epoch is still a hard error.

One new mechanical class, `goi-payload-retyping`, carries the #794 payload swap.
It is the exact one-to-one product-name mapping the ledger records above and
nothing else: the three retired arms, each with its payload carried across
verbatim, plus the `TenseModal(..)` wrapper for the tag-led arm. Everything else
at that position — a FA payload changing arm, a leading-term tag split selecting
a different arm, a widened leaf the old node could not spell, or any change to
the payload's own content — diverges into manual residue. The class was exercised
against synthetic positives and five negatives before the refresh.

### The level-inventory half of the re-baseline

Moving the archive forward also moves what the *old* side of every comparison is,
and the first run after the refresh exposed the half that had not moved with it.
`OLD_LEVEL_INVENTORY` and the old-level column of `POSITIONS` still described the
pre-6a grammar — the flat `term` sum, `pehe_termset_operand`, `SimpleTerm`
wrappers — while the archive is the 6a-composed ladder. Every fixture holding a
plain `SumtiTerm` at a term position was therefore rejected as "not a member of
the old term level" before the walk ever reached its GOI payload, which put 645
of the 646 re-typed fixtures into manual residue and left the new class carrying
7.

Both inventories are now transcribed from the `rule "term" … -> enum` arm lists,
and the transcription is checked arm-for-arm against `generated.rs` at each
commit rather than asserted. That check is what licenses the shorthand: across
`term`, `cehe_term`, `loose_term`, `nonabs_term`, `bound_term` and `simple_term`,
the *only* difference between the grammar at `3c3b84a5ba` and the grammar now is
D3's two new leaves, `gek_termset` and `zantufa_gek_termset`, so the old
inventory is written as the new one minus that pair instead of transcribed a
second time. Three positions lose a stale old level with it: the PEhE operand
reads `cehe_term` on both sides, and the TermsetGroup operands `loose_term` and
`nonabs_term`, because 6a's re-levelings are already applied in the baseline.

### C7 result

The comparer is green with the pins re-measured against the corrected classifier:

| Category | Count |
| --- | --- |
| Changed pre-epoch fixtures | 665 |
| `goi-payload-retyping` | 644 |
| `flat-sum-wrapper`, `pehe-cehe-retyping`, `stagless-bo-route-rejection`, `t3-loose-connection-warning` | 0 each |
| Manual residue | 21 |
| Prose-only provenance edits | 0 |
| Epoch-witness T3 re-pins / witness deltas / unpaired | 0 / 0 / 0 |
| Epoch-new witnesses (authored, unclassifiable) | 30 |

The four epoch-6a classes finding exactly nothing is the archive's own check:
their work is in the baseline tree, so a nonzero incidence would mean the archive
is not the tree it claims to be. The 646 baseline files carrying a
`SumtiAssociationRelativeClause` expectation reconcile as 644 mechanical plus
`cll/chrestomathy/forest-nymph` and `corpus/alis/full-alice`, which are manual for
a co-occurring `ForethoughtTermset` change rather than for anything at the GOI
position.

### The 21 manual residue fixtures, individually

All 14 baseline files carrying a `ForethoughtTermset` expectation are residue, as
the D3 section forecast, and they split three ways:

| Group | Count | Fixtures | Disposition |
| --- | --- | --- | --- |
| `ForethoughtTermsetSyntax` field reshape | 10 | `cll/chapter-09/section-9.8/c9e8d6`, `cll/chapter-14/section-14.11/c14e11d7`, `cll/chapter-14/section-14.15/c14e15d8`, `cll/chapter-14/section-14.15/c14e15d9`, `cll/chrestomathy/forest-nymph`, `corpus/camxes/12023`, `corpus/camxes/1451`, `corpus/camxes/1692`, `corpus/camxes/2646`, `corpus/camxes/2661` | `('m_nuhi', 'gek', 'terms', 'nuhu', 'first_branch', 'additional_branches', 'gihi')` became `('nuhi', 'gek', 'terms', 'nuhu', 'first_branch')` — the option-B split, NUhI now mandatory and the two unsourced fields gone. Every one of the ten keeps its NUhI, which is why the arm still matches |
| `ForethoughtTermset` became `GekTermset` | 3 | `corpus/camxes/644`, `corpus/camxes/2481`, `corpus/alis/full-alice` | The three pre-existing NUhI-less GEK termsets D3 names, moving from the flat reading to the sourced nested one. `full-alice`'s `tersmu-json` is unchanged, which is the membership-rule evidence D3 records |
| Rejection diagnostics moved | 8 | `corpus/camxes/12492`, `17294`, `19333`, `3095`, `3762`, `3784`, `6105`, `16937` | below |

The eight diagnostic moves are all acceptance-preserving — every one of these
surfaces was rejected before and is rejected now — and seven of them are the D4
payload widening showing through the error vocabulary:

- Six (`12492`, `17294`, `3095`, `3762`, `3784`, `6105`) keep their exact byte
  span and source text and only generalise `syntax.unexpected-brivla` to
  `syntax.unexpected-word`. All six are GOI-family surfaces — `mi ne sanji …`,
  `vi ma pe gugde …`, `… no'u dunli …`, `… po skina` — where the payload position
  used to admit only a sumti, so a brivla there was reported against the narrow
  expectation. The shared constituent's expected set is the term inventory, and
  the message follows it.
- `19333` is the text `negatively`, which lexes as `ne ga ti ve ly`. The `ne`
  opens a sumti-association phrase and the `ga` a GEK, so with termset payloads
  admitted (SUM-05) the parse now reaches the end of the input before failing:
  `syntax.unexpected-cmavo` at `ly` becomes `syntax.incomplete-free-modifier` at
  EOF. A deeper parse of the same rejected surface, not a new acceptance.
- `16937` is the retiring xfail, dispositioned above.

### The xfail splice, and what validates it

`fixture-rewrite` refuses any fixture carrying `expectations.syntax.xfail`, so the
514 xfail fixtures cannot be regenerated in the consolidated pass; an xfail pin
records a corpus-expected status that differs from the accepted one and must not
be silently re-derived. Their trees are spliced instead: the fixture is copied
without its `xfail` table, the copy is regenerated by the project's own writer,
and the original `status` and `xfail` lines are put back, with the accepted status
verified against `xfail.accepted-status` for every fixture.

The pipeline is validated by the fixtures it must *not* change: 506 of the 514
round-trip byte for byte through copy, regenerate and splice. 7 carried a stale
tree and are re-pinned; `corpus/camxes/16937` is the eighth and is the one
`accepted-status` mismatch the pass reports rather than writes.

### Regeneration reconciliation

The detached regeneration run reported seven of its twelve workers exiting 123.
All seven are accounted for:

| Worker | Cause | Disposition |
| --- | --- | --- |
| six workers | 12 unsupported facets across 9 fixtures | `corpus/camxes/16937` (syntax + both gentufa facets), `corpus/camxes/811` (both gentufa facets), and the 7 stale-tree xfails' syntax facets — the whole expected population |
| `paths-11` | the driver's `find tests/fixtures -name '*.toml'` swept in the three **profile definition** files, which are not fixtures | Sorted last, so the abort fell after all 1,735 real fixtures in that shard. Re-running the shard without them rewrites 0 and exits 0 |

`corpus/camxes/811` is pre-existing and takes a ledger note only: it pins an
`[expectations.output.gentufa]` section on a text its own `[expectations.syntax]`
records as a failure, so the derived facets cannot be rebuilt. Rebuilt at the
epoch base `3c3b84a5ba` with the base binary it fails identically, same two
facets and the same byte 34, so nothing in this epoch moved it.

The whole 26,476-fixture tree was then re-run facet by facet as an idempotency
check: every batch reports `rewrote 0`, and the only worker still exiting 123
carries `811`'s two facets and nothing else.

## The six-configuration substitution

The six-configuration substitution this epoch applies is recorded here as well:
epoch 6a retired `DialectFeature::TermHierarchy`, so the plan's "exp-off"
configuration no longer exists and the exp T3/T4 tiers are unconditional. The
configuration family for D3/D4/D5 is therefore read over the zantufa axes —
omitted dialect, `()`, `(+zantufa-terms)`, `(+zantufa-connectives)`, both, and
`(zantufa)` — with the plan's "the zantufa arm must not widen" intent discharged
by the `ZantufaTerms`-off rejection rows plus a zantufa-**on** row pinning that
the connective-present stag-less form stays owned by the exp T4-normal arm. The
lead ACKed this substitution.

## Pre-submission gate

Run at `4e11fddb47`, the tip of the code and expectation commits; the ledger commit
above it changes only this file, which nothing under test reads.

| Gate | Result | Log |
| --- | --- | --- |
| `cargo fmt --all --check` | clean | `epoch06b-gate-fmt.log` |
| `cargo test -r --workspace` | 2,312 passed, 0 failed, 16 ignored | `epoch06b-gate-workspace.log` |
| `fixture-test --profile all` | 26,476 fixtures, 4 facets, 73,766 passed, 513 xfailed, **0 failed** | `epoch06b-gate-fixtures.log` |
| Frozen tagged `term-hierarchy-epoch` facet | 90 fixtures, 94 passed, 0 failed | `epoch06b-gate-tagged-facet.log` |
| Expensive contracts, all targets, release | 2,333 passed, 0 failed | `epoch06b-gate-expensive.log` |
| `semantics-coverage` | checked 22,633, panics 0, unsupported 0 | `epoch06b-gate-coverage.log` |
| Debug `jbotci` build | green | `epoch06b-gate-debug-jbotci.log` |
| Debug `dx build` | green | `epoch06b-gate-dx.log` |
| `maturin develop` + the four generated checks | all green | `epoch06b-gate-maturin-develop.log`, `epoch06b-gate-generate_*.log`, `epoch06b-gate-compose_stubs.log` |
| Comparer, ratcheted | 665 changed / 644 + 0 + 0 + 0 + 0 mechanical / 21 manual, prose 0, witness re-pins 0, witness deltas 0, unpaired 0, epoch-new 30 | `comparer-final.txt` |
| Peak RSS, full profile | base 5,731,944 KB → 5,782,268 KB, **+0.88%** (gate +20%) | `epoch06b-gate-fixtures*.log` |
| Artifact ratchet | archive **+2.27%**, unpacked **+2.64%**, native `.so` **+2.47%** versus a base-built control (gate 5%) | `epoch06b-gate-wheel-build-*.log` |

The fixture count moves 26,446 → 26,476 with this epoch's 30 witnesses, and the
xfail count 514 → 513 because `corpus/camxes/16937`'s xfail retires — the same
shape as epoch 6a's 515 → 514 for `corpus/camxes/20100`.

Both wheels in the artifact row were built natively as manylinux 2.34, matching
epoch 6a's caveat; the python-wheels workflow remains the acceptance authority for
the 2.28 artifact. The first control pair was discarded and rebuilt: the epoch
wheel had been produced after a `maturin develop`, so it packaged 15 `__pycache__`
entries the base control did not, inflating the archive delta to +5.24% and the
unpacked delta to +7.10% for purely local reasons. `__pycache__` is gitignored and
CI builds from a clean checkout, so the shipped artifact was never affected; the
rebuilt control is stripped and byte-compilation-free on both sides, and its
native `.so` is byte-for-byte the same size as the contaminated build's, which
confirms only packaging differed.
