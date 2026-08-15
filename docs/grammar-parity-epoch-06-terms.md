# Grammar parity epoch 6: term hierarchy

This note records the durable implementation dispositions for GitHub issues
#792, #794, #795, #796, #806, #816, and #827, together with the epoch-4 VUhO
residual that #792 inherited. The implementation base is main
`a8b4f062279a4d9e22722ca45822f4b4b3885fbf`. The authoritative design is the
panel-reviewed epoch-6 plan v2 in the grammar-review repository. Running-parser
probes use `camxes.js`, `camxes-exp.js`, and rolling Zantufa
`zantufa-1.9999.js`; a pinned reference tree is accepted only after rerunning
the applicable parser rather than transcribing a reviewer summary.

The reference snapshots used for the epoch witnesses are ilmentufa commit
`778ea138f7d150121ca722db7536ce3b123943ac` and gerna_cipra commit
`d5a5065c924304cf5e9067ee6d41b584fbe1c099`, the same snapshots epochs 4 and 5
recorded.

## The composed ladder

Standard camxes composes three term levels and camxes-exp composes five. The
epoch rebuilds `term` as the union of both compositions, one level per upstream
production:

| Level | Rule | Upstream source | Operand of |
| --- | --- | --- | --- |
| T1 PEhE | `pehe_termset_connection` over `term` | `terms_1 <- terms_2 (PEhE free* joik_jek terms_2)*` (camxes.peg:114) | every consumer of a term sequence |
| T2 CEhE | `termset_group` over `cehe_term` | `terms_2 <- term (CEhE free* nonabs_term)*` (camxes.peg:116) | the PEhE connection |
| T3 loose | `connected_term` over `loose_term` | `abs_term_1 <- abs_term_2 (joik_ek …)*` (camxes-exp.peg:153) | the CEhE connection's leading operand |
| T4 BO | `stag_bound_term_connection` over `bound_term` | `abs_term_2 <- abs_term_3 (joik_ek stag BO abs_term_3)*` (camxes-exp.peg:154) | the loose connection |
| T5 atoms | `simple_term` | `abs_term_3` / `term_1` leaves | the BO connection |

`terms <- terms_1+` is expressed, as before, by consumers repeating `term`; the
epoch changes no consumer's arity.

### Mechanism E and the drift guard

#791 settled the leaf-listing mechanism: a hierarchy level re-lists the leaf
inventory instead of nesting the level below it as a sum branch, because a
nested branch would add a public wrapper variant to Debug and serde output.
Every new level follows that rule, and the binding-schema drift guard in
`crates/jbotci-syntax-macros/tests/binding-schema-consumer` is extended from the
two #791 levels to the whole ladder: `BoundTerm = SimpleTerm + BO`,
`LooseTerm = BoundTerm + loose`, `CeheTerm = LooseTerm + CEhE`,
`Term = CeheTerm + PEhE`, and `NonabsTerm = LooseTerm` with exactly the
absorption-guarded tag leaf swapped for its unguarded twin. Adding a leaf to
`simple_term` without extending every level now fails the guard.

Because each connection requires at least one continuation, a term with no
connection selects its leaf directly at the level that offered it. The former
`ConnectedTerm { leading_term, continuations: [] }` wrapper, which the old
`zero_or_more` shape produced for every term in the corpus, is gone.

### Two parameters, never conflated

The plan separates *guard content* from *operand width*; the implementation
keeps them separate too.

*Guard content.* The absorption guard lives in the tag-led atom.
`tagged_sumti_term` asserts `!selbri` after the tag, which is camxes-exp's
`abs_tag_term` axis (camxes-exp.peg:160) and — because jbotci's `selbri`
parameter is the untagged selbri, with tags owned by an outer layer — also
matches camxes-standard's `tag !(!tag selbri)` (camxes.peg:126) at every
position where the two can differ. Probe: `mi bai bau broda` assigns `bai` with
an elided KU and reads `bau broda` as the tagged selbri in camxes-standard and
in jbotci alike. `nonabs_tagged_sumti_term` is the same rule without that
assertion, which is exactly camxes-standard's `nonabs_term` (camxes.peg:128).
The two nodes differ by nothing else, so semantic lowering sees one shape
(`GeneratedTaggedTermRef`).

*Operand width.* Each level names its own operand rule, so a context selects a
width by naming a level rather than by relaxing a guard:

| Context | Operand | Source |
| --- | --- | --- |
| Ordinary term positions | `term` (T1) | std `terms`, exp abs flavour |
| PEhE operands | `cehe_term` (T2) | camxes.peg:114 |
| CEhE leading operand | `loose_term` (T3) | camxes.peg:116, camxes-exp.peg:122 |
| CEhE continuations | `nonabs_term` | std `nonabs_term` ∪ exp `abs_term` |
| loose operands | `bound_term` (T4) | camxes-exp.peg:153 |
| BO operands | `simple_term` (T5) | camxes-exp.peg:154 |
| BE/BEI links | `linked_term` ladder | camxes-exp.peg:200, normal flavour |

`nonabs_term` is the union of the two sources at one position: camxes-standard
reads a CEhE continuation as a single unguarded `nonabs_term`, camxes-exp reads
it as a full absorption-safe `abs_term`. The level therefore carries the
absorption-safe connection tiers over guarded operands, and the unguarded leaf
inventory for the connectionless case. No surface outside the two sources is
admitted: the absorption guard can only fire when a selbri follows the atom
directly, and no connective tier can occupy that position.

## Consumer inventory (C1 prerequisite)

Every direct consumer of the `term` family at the implementation base, and its
disposition. `term` is consumed at 25 sites; all of them take the composed T1
level, so the table records the level each site now sees rather than a per-site
rewrite.

| Base line | Consumer | Disposition |
| ---: | --- | --- |
| 329 | `zantufa_iau_statement_terms_tail` | T1 sequence; unchanged arity |
| 335 | `zantufa_bare_statement_terms_tail` | T1 sequence; unchanged arity |
| 489 | `prenex_fragment` | T1 sequence; prenex collector reworked for leaf-listed levels |
| 497 | `prenex_statement` | T1 sequence; prenex collector reworked |
| 592 | `terms_fragment` | T1 sequence |
| 825 | `bridi_with_leading_terms` | T1 sequence |
| 835 | `bridi_with_post_cu_terms` | T1 sequence |
| 867 | `cu_terms_bridi_tail` | T1 sequence |
| 891 | `zantufa_grouped_bridi_tail` | T1 tail terms |
| 990 | `selbri_simple_bridi_tail` | T1 tail terms |
| 1028 | `direct_forethought_bridi_connection` | T1 tail terms |
| 1119 | `bridi_tail_ke_continuation` | T1 tail terms |
| 1137 | `gihek_bridi_tail_ke_continuation` | T1 tail terms |
| 1169 | `bridi_tail_bo_continuation` | T1 tail terms |
| 1195 | `bridi_tail_continuation` | T1 tail terms |
| 1217 | `prenex_subbridi` | T1 sequence; prenex collector reworked |
| 1477 | `forethought_termset.terms` | NUhI-present operands stay full guarded `terms` (B1) |
| 1493 | `forethought_termset_branch.terms` | as above |
| 1504 | `zantufa_forethought_termset_branch.terms` | as above |
| 1514 | `nuhi_termset.termset` | as above |
| 1525 | `ke_termset.termset` | as above; `ke_termset` itself is unchanged (ungated experimental, Zantufa-sourced) |
| 2975 | `lahe_term_wrapper.inner_term` | T1 payload |
| 2992 | `scalar_negated_term_wrapper_with_bo.inner_term` | T1 payload |
| 3008 | `scalar_negated_term_wrapper.inner_term` | T1 payload |
| 3399 | `sei_free_modifier.terms` | T1 sequence |

The loose-connection guard has exactly two call sites, and both are converted in
the same commit: ordinary terms (`connected_term_continuation`) and BE/BEI links
(`connected_linked_term_continuation`). The BE/BEI ladder keeps its own leaf
inventory and its optional-stag BO continuation, which is camxes-exp's *normal*
term flavour (camxes-exp.peg:143, :200); only its feature gate is removed, so
the #791 link witnesses keep their trees and gain only the BO warning.

## TermHierarchy retirement

`DialectFeature::TermHierarchy` is removed. The camxes-exp term tiers are not a
dialect: they are default-enabled extensions of one composed grammar, diagnosed
where camxes-standard rejects them. The owner-visible consequences:

| Surface | Disposition |
| --- | --- |
| `DialectFeature::TermHierarchy` | Removed from the public dialect feature enum; the Python binding's feature list loses the matching entry and the API matrix rows are regenerated. |
| `(term-hierarchy)` dialect string | Retained as a deprecation no-op that resolves to the empty dialect, so existing dialect strings keep parsing. `adhoc/v0/warnings/term-hierarchy/*` exercise that path. |
| `zantufa` builtin preset | Drops `term-hierarchy` from its definition; the tiers it used to enable are now unconditional. |
| `ExperimentalTermHierarchyBoConnection` | Renamed `ExperimentalTermBoConnection` (`syntax.warning.experimental-term-bo-connection`). The variant existed but was never emitted; it is now wired to the `bo` token of every BO-bound term and linked-argument connection. |

### Diagnosed tiers and one documented gap

The BO tier warns on its `bo` token. The loose (T3) tier does **not** warn, and
that is a recorded gap rather than a decision that term-level connectives are
standard: camxes-standard has no term-level connective at all, so every loose
term connection is an extension. The syntax DSL anchors a warning on a `Token`
parser (`ParserState::warn` takes `&Token`), and a loose continuation owns no
token of its own — its only tokens belong to the shared `joik_connective` /
`ek_connective` nodes that the sumti and statement tiers also use. Warning the
tier therefore requires either duplicating the joik|ek inventory as term-tier
node types, including the GAhO interval forms, or extending the syntax macro to
anchor a warning on a node's primary token. Both are outside this epoch's
scope, and the first carries real fidelity risk. The state is not a regression:
the default profile already accepted term-level connectives warning-free at the
implementation base. Recorded as a follow-up-issue candidate alongside the
fidelity-flag candidates below.

## Removed routes

| Route | Disposition |
| --- | --- |
| Standard stag-less BO term connection (`bound_term_connection`, #796) | Deleted. camxes-standard has no term-level BO and camxes-exp requires the stag in absorption-safe positions, so `pu ko'a .e bo ba ko'e broda` now rejects in every profile. The owner ruling on #796 is "do what camxes-std does". |
| `pehe_termset_operand` | Deleted; the PEhE connection now takes the `cehe_term` level directly. |
| `GeneratedDirectTermConnective::Bound`, `GeneratedDirectTermOperand::Simple` | Deleted with the route that constructed them. |

Semantic lowering of a BO-bound term connection now assigns every operand
through the termset-branch cursor, which is what the deleted stag-less route did
for its two operands and what the CEhE termset already did for its branches. The
previous behaviour — silently skipping the operands of a `stag_bound_term_connection`
— was an artefact of the feature-gated route, not a disposition.

## Connective inventories (#795, #806)

`term_afterthought_connective` (JOIK or EK) is the connective of both term
tiers and of the BE/BEI loose tier; the PEhE level takes the standard
`standard_statement_connective` (JOIK or JEK). The wide
`connected_term_connective`, which admitted JEK and VUhU at the loose tier,
retires with the flips.

Both domains are owner-corrected rather than ported: camxes-exp spells both
term tiers `joik_ek` and the PEhE tier `joik_jek`, and its `joik_ek_1` is
`joik / ek / VUhU` while its `joik` also reaches JA spellings
(camxes-exp.peg:347, :354, :358). The narrowed domains follow #795 ("corrected
joik/ek domain") and #806 ("PEhE uses joik/jek only"), which is the I02
adjudication applied to the term site — the same precedence epochs 4 and 5 used
for their CEhE and selbri EK/VUhU flips.

### Documented gaps

The warning grammar now rejects four shapes that running camxes-exp accepts.
Each is witnessed, and the standing reinterpretation ruling's fidelity-flag
question is recorded as a follow-up-issue candidate rather than minted here.

| Surface | camxes-exp | camxes-standard | jbotci | Witness |
| --- | --- | --- | --- | --- |
| `ba ko'a je ca ko'e broda` | accepts (`joik_ek` reaches JA) | rejects | rejects | `adhoc/syntax/terms/issue-795-term-jek-rejected` |
| `ba ko'a vu'u ca ko'e broda` | accepts (`joik_ek_1` includes VUhU) | rejects | rejects | `adhoc/syntax/terms/issue-795-term-vuhu-rejected` |
| `ko'a pe'e .e ko'e broda` | accepts | rejects | rejects | `adhoc/syntax/terms/issue-806-pehe-ek-rejected` |
| `ko'a pe'e vu'u ko'e broda` | accepts | rejects | rejects | `adhoc/syntax/terms/issue-806-pehe-vuhu-rejected` |

Three pre-existing v0 fixtures carry the same flip and are re-pinned with the
epoch: `adhoc/v0/warnings/experimental/simpler-term-connective` (JA),
`adhoc/v0/warnings/experimental/vuhu-term-connective` and
`adhoc/v0/syntax/basic/cache-vuhu-connective-after-joik-miss` (VUhU). The
retained halves of both domains keep their acceptance:
`issue-795-term-ek-accepted`, `issue-806-pehe-jek-accepted`,
`adhoc/v0/warnings/experimental/broad-a-term-connective`,
`broad-joi-term-connective`, and the two `experimental-ji-*-term-connective`
fixtures, whose `ji` is an A-family question word rather than a JEK.

## VUhO ownership matrix (epoch-4 residual)

Epoch 4 left the owner of a connective continuation after a VUhO tail to this
epoch. The rule is baseline-first: wherever the complete outer candidate
succeeds, the outer term connection owns the continuation, which is what the
retained epoch-4 fixture
`adhoc/syntax/sumti-continuation/vuho-term-no-steal` already pinned. The
composed ladder does not change that owner.

| Surface | camxes-standard | camxes-exp | rolling Zantufa | jbotci owner | Witness |
| --- | --- | --- | --- | --- | --- |
| `… vu'o poi brode ku'o .e lo mlatu …` | rejects | accepts, inside the VUhO tail | accepts, outer | outer term connection | `vuho-sumti-continuation-outer` |
| `… vu'o poi brode ku'o .e ba ko'a …` | rejects | accepts at the term tier | accepts, outer | outer term connection | `vuho-tag-continuation-outer` |
| `… vu'o poi brode ku'o .e bo lo mlatu …` | rejects | rejects | accepts via connectorless BO | rejects | `vuho-bo-continuation-rejected` |
| `… vu'o .e lo mlatu …` (bare VUhO) | rejects | accepts, outer | rejects | outer term connection, bare-VUhO warning | `vuho-bare-continuation-outer` |

Two fidelity gaps are recorded rather than closed. camxes-exp reads the
sumti-operand continuation *inside* the VUhO tail
(`relative_clauses (joik_ek sumti)?`, camxes-exp.peg:175); adopting that reading
would reinterpret a successful baseline parse, so it stays a documented gap with
a reinterpretation-flag candidate, exactly as the standing ruling requires.
Rolling Zantufa accepts the BO-bound continuation through its connectorless BO
term tier, which #827 owns; until that lands, the Zantufa configuration keeps
the rejection, witnessed by `vuho-bo-continuation-rejected-zantufa`.
