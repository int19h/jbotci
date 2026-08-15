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
`corpus/alis/full-alice`. Their `semantics.refs` and `output` facets are
unaffected; only the pinned syntax tree changes. They are regenerated with the
epoch's single consolidated expectation update.

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

### Two unsourced widenings this shape exposes

Both belong to the optional-NUhI `forethought_termset` node and are recorded
here rather than closed, pending the disposition question below.

| Surface | camxes-standard | camxes-exp | rolling Zantufa | jbotci |
| --- | --- | --- | --- | --- |
| `ge ko'a nu'u gi ko'e broda` | rejects | rejects | rejects | **accepts** |
| `nu'i ge A gi B gi C nu'u broda` (n-ary NUhI-present) | rejects | rejects | no NUhI selma'o at all | **accepts** |

Rolling Zantufa has neither NUhI nor NUhU: `nu'i` lexes as KE there, and its own
NUhI-less termset is `gek_term <- gek term+ (gik term+)+ GIhI?`, with `term+`
branches and no terminator slot. So the optional-NUhI node's NUhU slots are
sourced by nothing, and its Zantufa n-ary branches and GIhI cannot be sourced in
the NUhI-present arm they currently sit in.

## Open: the branch-formula type boundary

D3's grammar half is complete and probe-exact. Its **semantic** half is blocked
on a boundary that the plan did not anticipate.

A NUhI-less GEK termset lowers as a logical *connection*:
`build_generated_forethought_termset_connection_formula` finds the termset in
the sentence's term list and splices each of its two branches into the
surrounding terms, producing two complete bridi term lists that are then lowered
separately and joined by the connective. That splice is typed
`Vec<&TermSyntax>`, and it reaches the general bridi lowering path
(`build_selbri_simple_bridi_tail_formula_with_preassigned_arguments`,
`build_term_assignments_for_terms`, `GeneratedTermAssignments<'syntax>`).

`gek_termset` operands are `NonabsTermSyntax`, which is not convertible by
reference — and they must be, because the unguarded atom is exactly what makes
`ge ko'a gi pu broda` parse. Making the bridi term list level-agnostic (a
`GeneratedBridiTermRef` threaded through roughly 112 signatures across
`generated_builder/*.rs`, plus the assignment payload types) is a refactor
comparable in size to the epoch itself.

The branch *membership* is unaffected by the re-shaping: the leading operands
read outermost-first are the branch before the GIK and the trailing operands
read innermost-first are the branch behind it, which for the symmetric corpus
cases is exactly the flat reading's membership. So the correct lowering is known;
only the type boundary is in the way.

At this commit the reference and place-analysis walkers cover the new node — the
`semantics.refs` and `output` facets of the three affected fixtures still pass —
but graph building reports the principled `invalid_graph` error "non-sumti term
reached sumti visible-place advancement" instead of a connection formula. That
is not a semantics-coverage ratchet failure (the ratchet fires on
`undefined_semantics` and panics, and the allowlist stays empty), but it is a
capability regression on the surfaces in the table above.

The disposition is a lead decision because every option changes what this epoch
delivers, and it is recorded as an open ASK on work item
`jbotci-epoch06b-terms`.

## Scope not yet delivered

| Section | Issue | State |
| --- | --- | --- |
| D3 termset shapes | #806 | grammar complete; semantics blocked as above; witnesses and the `forethought_termset` disposition pending |
| D4 GOI and flavour-context payload width | #794 | not started |
| D5 Zantufa term binding | #827 | not started |
| C7 consolidated expectations, comparer re-baseline, ratchet, peak RSS | — | not started |
