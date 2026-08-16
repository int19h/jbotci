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
| D5 Zantufa term binding | #827 | not started |
| C7 consolidated expectations, comparer re-baseline, ratchet, peak RSS | — | not started |

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
- **C7.** Re-baseline `tools/compare-term-hierarchy-expectations.py` to
  `git archive 3c3b84a5ba tests/fixtures` (the tool is fail-closed both ways and
  its baseline must stay git-derivable — that was the 6a round-3 fix), extend the
  classes only per the plan's C7 rules, review the comparer before the refresh,
  then regenerate. The three deferred syntax expectations plus whatever the
  `forethought_termset` split moves are the manual residue. Then the
  semantics-coverage ratchet and the peak-RSS gate (epoch-vs-base ≤ +20% on the
  full release fixture profile, measured AFTER the bulk regeneration, one volume).

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
