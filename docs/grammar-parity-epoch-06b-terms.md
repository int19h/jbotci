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

## Scope not yet delivered

| Section | Issue | State |
| --- | --- | --- |
| D3 termset shapes | #806 | complete, including the `forethought_termset` split above |
| D4 GOI and flavour-context payload width | #794 | not started |
| D5 Zantufa term binding | #827 | not started |
| C7 consolidated expectations, comparer re-baseline, ratchet, peak RSS | — | not started |

The next session picks these up in that order. What each needs, concretely:

- **D4 (#794).** A named normal-flavour payload constituent: the loose joik_ek
  tier over an OPTIONAL-stag BO tier over the unguarded leaves. The BE/BEI link
  ladder that epoch 6a built (`generated.rs`, the `linked_term` family) is the
  exact structural template, including its `recursive` block declarations —
  every new ladder level 6b adds must join that block, which is epoch lesson 11.
  Then the GOI payload widens from `relative_sumti` to it, and the `gek_termset`
  operand widens to it as well. Witnesses: std FA-tagged and termset payloads,
  the std negative set from Sol's GOI-width matrix (CEhE, PEhE and `.e bo` stay
  outside or reject), exp payload positives (`goi ba ko'e .e bo vi ko'i`), and
  GEhU/relative no-delta rows. The 6a ledger's intentional-partial row for the
  zantufa GOI payload retires here.
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
