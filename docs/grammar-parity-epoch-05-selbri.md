# Grammar parity epoch 5: selbri reconstruction

This note records the durable implementation dispositions for GitHub issues
#840, #841, #842, #829, and #828. The implementation base is main
`9fafb66d4a8a3da5183d4beeba66396266348b5c`. The authoritative design is the
panel-reviewed epoch-5 plan v3 in the grammar-review repository. Running-parser
probes use `camxes.js`, `camxes-exp.js`, and rolling Zantufa
`zantufa-1.9999.js`; a pinned reference tree is accepted only after rerunning
the applicable parser, rather than transcribing a reviewer summary.

The reference snapshots used for the epoch witnesses are ilmentufa commit
`778ea138f7d150121ca722db7536ce3b123943ac` (`camxes.js` SHA-256
`8cde27ae785f06b23cef02ed81a4293e7c97e838a5c2258abda9ece8984df89d`,
`camxes-exp.js` SHA-256
`88d054407e15557b5058441838d77adc41cf005a80d0eafecc5dce44211fcee1`) and
gerna_cipra commit `d5a5065c924304cf5e9067ee6d41b584fbe1c099`
(`zantufa-1.9999.js` SHA-256
`42e40893082ee82c38407d4dc58f07b0a9fd09d4a73242486cfd13f6feee8cb7`).

## Pre-implementation selbri call-site ledger

The following table is the required mechanical inventory of every direct use of
the `selbri` parser parameter in `generated.rs` at the implementation base. It
was produced by scanning for `arc(selbri)`, the equivalent ignored lookahead,
and negative `selbri` assertions, then associating every match with its enclosing
rule. Merely forwarding `selbri` as a grammar-rule parameter is not a call site;
the eventual direct consumer is listed instead. The 25 rows below account for
all 23 `arc(selbri)` calls, one ignored lookahead, and one negative assertion.

`Normal` means that the consumer takes the full reconstructed selbri entry.
`No-terminal-relative` means that the description or vocative owns a following
relative-clause slot, so the right-spine-threaded entry suppresses only the
terminal selbri-relative slot while retaining CEI. `Warning-wrapper` means that
the consumer is reached through an already warning-gated experimental or
Zantufa construction; reconstruction must preserve that outer provenance.

| Base line | Direct consumer | Classification | Epoch-5 disposition |
| ---: | --- | --- | --- |
| 565 | `selbri_fragment` | Normal | Keep the full entry; fragment ownership is unchanged. |
| 792 | `selbri_simple_bridi_tail_without_tail_terms` | Normal | Keep the full entry; this is a non-description selbri position. |
| 800 | `selbri_simple_bridi_tail` | Normal | Keep the full entry, including the additive selbri-relative slot. |
| 1053 | `term_hierarchy_loose_connection_guard` | Normal lookahead | Point the ignored lookahead at the full entry; it remains an ownership guard only. |
| 1352 | `noiha_variable_adverbial_term` | Normal | Keep the full entry inside the NOIhA term. |
| 1362 | `noiha_relative_adverbial_term` | Normal | Keep the full entry inside the NOIhA term. |
| 1453 | `tagged_sumti_term` | Normal negative lookahead | Test the full entry; the assertion remains a term-boundary guard. |
| 1554 | `interval_property_leading_term_tag_tense` | Normal lookahead wrapper | Keep `fiho_tense(selbri)` in the interval-property follower lookahead; the nested FIhO body consumes the full entry and this outer call remains non-owning. |
| 2010 | `selbri_mekso_operator` (NAhU) | Normal | Keep the full entry; TEhU continues to delimit the consumer. |
| 2030 | `zantufa_maho_selbri_mekso_operator` | Warning-wrapper | Keep the full entry under the existing `ExperimentalZantufaMex` introducer warning. |
| 2209 | `zantufa_selbri_mohe_mekso_operand` | Warning-wrapper | Keep the full entry under the existing `ExperimentalZantufaMex` introducer warning. |
| 2219 | `selbri_mekso_operand` (NIhE) | Normal | Keep the full entry; TEhU continues to delimit the consumer. |
| 2916 | `descriptor_without_gadri_sumti` | No-terminal-relative when KU is elided; Normal when KU is explicit | Split by terminator ownership: the elided-KU path preserves the outer relative; the explicit-KU path admits the full entry because the following relative slot starts after KU. |
| 2959 | `relation_description_tail` | No-terminal-relative | Use the right-spine-threaded entry so the description retains its terminal relative slot. |
| 2970 | `quantifier_relation_description_tail` | No-terminal-relative | As above, while retaining the complete CEI repetition inside the description selbri. |
| 3060 | `selbri_vocative_sumti` | No-terminal-relative | Preserve the trailing vocative-relative slot by default; the reinterpretation flag selects the full entry for Zantufa fidelity. |
| 3170 | `sei_free_modifier` | Normal | Keep the full entry; SEhU delimits this non-description consumer. |
| 4142 | `fiho_tense` | Normal | Keep the full entry and preserve FEhU elision and zero-width span behavior. |
| 4338 | `exp_nihe_number_atom` | Warning-wrapper | Keep the full entry inside the baseline-rejected experimental tag run; preserve its run-level warning ownership and TEhU span. |
| 4374 | `exp_fiho_tag_atom` | Warning-wrapper | Keep the full entry inside the baseline-rejected experimental tag run; preserve FEhU elision. |
| 4473 | `zantufa_fiho_tcita_selci` | Warning-wrapper | Keep the full entry inside the feature-gated Zantufa tag model; this is the sourced NAhE/FIhO route used by the #842 fragment witness. |
| 4939 | `forethought_selbri_connection.leading_selbri` | Normal | Rehome as the full left operand of the single standard L6 guhek owner. |
| 4953 | `forethought_selbri_branch` | Normal | Rehome as the tight L6 right operand of the standard binary guhek owner. |
| 4962 | `zantufa_forethought_selbri_branch` | Warning-wrapper | Replace with the structurally disjoint n-ary/GIhI arm; all operands use Zantufa selbri-2 width and retain the existing warning categories. |
| 5024 | `forethought_selbri_group_tanru_unit.leading_selbri` | Normal | Remove this overlapping standard owner; standard ownership moves to L6, while the disjoint warning-gated Zantufa form is rebuilt separately. |

The JAI-inner mini-ladder is not a direct `selbri` call site: it has its own
`connected_jai_inner_selbri` family. Its continuation still consumes
`relation_afterthought_connective`, so it is explicitly retained for the
epoch-7/JAI-family reconstruction instead of being silently changed here.

## Binding A13 audit dispositions

| Audit | Disposition |
| --- | --- |
| Feature gates | Full-selbri CEI and selbri-layer relatives use `ZantufaTerms`; n-ary forethought and KE-with-CO use `ZantufaConnectives`. |
| Warning categories | N-ary forethought and GIhI reuse `ExperimentalZantufaNaryForethought` and `ExperimentalZantufaForethoughtGihi`. Dedicated categories are added for Zantufa selbri assignment, selbri-relative placement, and KE/CO grouping. `ExperimentalZantufaStatementRelativeClause` is not reused because it describes a relative body, not its attachment site. |
| `relation_afterthought_connective` | Keep the node for `ke_bridi_statement_continuation`, `relation_connective_as_bridi_tail`, and `connected_jai_inner_selbri_continuation`. Selbri-family consumers move to the new joik/jek-only nodes. |
| Grouped forethought bridi connection | Its only consumers are the with-tail-terms and without-tail-terms `forethought_bridi_connection` sums. Both grouped variants change their tag field from optional to repeated; no split is required. |
| Existing Zantufa atoms | Preserve `zantufa_me_tanru_unit`, `zantufa_mex_moi_tanru_unit`, and `zantufa_statement_abstraction_tanru_unit`; witness them in all six configurations without ownership changes. |
| Deferred experimental scope | Experimental selbri `tag*` is #857; NOhOI selbri relatives are #818; experimental forethought relative lists are #855; preposed linkargs remain in the linkargs epoch. |
| Standard connective partition | EK/A and VUhU are removed from selbri connective positions. Their camxes-exp provenance is intentionally unadopted under I02, whose site list is extended to include selbri level 4. Connectorless `tag BO` is independently unsourced in all three reference grammars. |

### C1 outer-consumer residuals inherited by epoch 7

The rejection witnesses use description selbri so that no outer bridi-tail
consumer can reclaim the connective. Each isolating surface was rerun against
the pinned camxes-standard parser and rejected. The corresponding complete
statements remain explicit success pins because their current ownership lies
outside the selbri family:

| Complete statement | Current post-C1 owner | Provenance and disposition |
| --- | --- | --- |
| `mi broda .e brode` | `RelationConnectiveAsBridiTail` | Plain-jbotci-only residual. I02 deliberately does not adopt camxes-exp's EK selbri connective; camxes-standard rejects the statement. Epoch 7 (#805/#815/#826) owns the outer consumer and inherits the pin to flip. |
| `mi broda su'i brode` | `RelationConnectiveAsBridiTail` | Plain-jbotci-only residual. I02 deliberately does not adopt camxes-exp's VUhU selbri connective; camxes-standard rejects the statement. Epoch 7 owns the outer consumer and inherits the pin to flip. |
| `mi broda je pu ke brode ke'e` | `BridiTailKeContinuation` | Plain-jbotci-only residual. The standard tagged grouped selbri continuation is JOIK-only and camxes-standard rejects the statement. Epoch 7 owns the outer consumer and inherits the pin to flip. |

These pins document rather than broaden C1: the description-isolated fixtures
prove the selbri-family restriction, while the complete-statement fixtures
make the deferred bridi-tail-family boundary mechanically visible.

### C2 forethought-domain dispositions

The grouped forethought-bridi owners with and without tail terms both carry a
source-ordered repeated tag field. The nested S2 witness
`mi ke bau bai ke ge broda gi brode ke'e ke'e` prevents the outer bridi-tail
term consumer from reclaiming either BAI; the top-level
`mi bau bai ke ge broda gi brode ke'e` control records that the same words are
ordinary terms when the isolating outer KE is absent.

Standard forethought selbri now have one binary owner at L6. Its left operand
is full selbri width and its sole GI branch is L6 width, so the pinned standard
trees put CO and adjacency outside the group:

| Surface | Standard disposition |
| --- | --- |
| `mi gu'e broda gi brode co brodi` | The complete GUhE/GI group is the left side of the outer right-recursive CO. |
| `mi gu'e broda gi brode brodi` | The complete GUhE/GI group is followed by outer tanru adjacency. |

The rolling-Zantufa owner is reachable only under `ZantufaConnectives` and
only when the completed surface proves disjointness from standard syntax:
one or more additional GI branches, or an explicit GIhI. Its leading operand
and every branch are uniformly L2 width. Each additional GI retains
`ExperimentalZantufaNaryForethought`; GIhI retains
`ExperimentalZantufaForethoughtGihi`. The six-configuration fixtures pin both
success trees and exact warning sets, and pin failures with exact diagnostics.

NAhE was unfolded from the connective leaf into the standard and Zantufa
forethought owners. A repeated free-modifier slot follows it independently;
S35 and S36 pin the SEI surface both without and with NAhE. Connector source
rendering still includes NAhE, while truth-table behavior remains determined
by GUhA/GI.

Single-GI branch-width reinterpretation is deliberately not warning-gated or
pinned to a Zantufa tree: both single-GI surfaces are already accepted by the
standard owner at identical extent. Their Zantufa CO-inside and
adjacency-inside trees remain the documented #858 fidelity gap. This avoids a
warning arm that silently reinterprets baseline-valid text.

### C3 NAhE atom-scope and tag-model dispositions

The standard scalar-negation owner now recurses over exactly
`tanru_unit_atom`. Its former tagged-connected-selbri and direct pro-bridi
alternatives were jbotci-only duplicates; pro-bridi remains available through
the ordinary atom inventory. The generated semantic consumers were reduced to
the now-single atom variant, preserving conversion, place-frame, reference,
KEhA, label, and rafsi traversal through the atom helpers.

The reference probes are recorded in
`/build/jbotci/logs/epoch05-c3-reference-probes.log`. Running camxes-standard
rejects `mi na'e fi'o brodo fe'u brode`, while camxes-exp and rolling Zantufa
accept it as a full bridi with the NAhE/FIhO form in the ordinary selbri tag
slot. Both experimental references accept the elided-FEhU surface
`mi na'e fi'o brodo brode` as a final tag-term fragment. Jbotci therefore pins
the following ownership, with no selbri-side widening:

| Surface and profile | Pinned owner | Diagnostics disposition |
| --- | --- | --- |
| Explicit FEhU, default/exp | `TaggedSelbri` containing `ExpTagAtomRun` | Exactly `ExperimentalFlattenedTag` on NAhE. |
| Explicit FEhU, built-in Zantufa | The same extension-first tag model inside `TaggedSelbri` | Exactly `ExperimentalFlattenedTag`; the earlier camxes-exp arm owns the common extent before the Zantufa tag fallback. |
| Elided FEhU, default/exp | `ElidedNaheFihoTagTerm` fragment containing `ExpTagAtomRun` | Exactly `ExperimentalFlattenedTag` on NAhE. |
| Elided FEhU, built-in Zantufa | The same final tag-term fragment | Exactly `ExperimentalFlattenedTag` for the common exp/Zantufa extent. |

The final-tag-term arm is extension-first and accepts only the typed completed
candidate consisting of one NAhE-prefixed FIhO atom with an absent FEhU. Its
strict and recovered classifiers are exhaustive over the relevant generated
tag nodes, so explicit FEhU and unrelated tags cannot be reowned. This is the
A21 tag-model route, not the removed scalar-negated selbri arm.

The positive controls `mi na'e broda`, `mi na'e se broda`, and
`mi na'e ke broda brode ke'e` were rerun through all three reference parsers
and are pinned warning-free. Rolling Zantufa's wider NAhE-over-KE/CO form is
untouched and remains part of the C4 KE/CO witness family.

## Fidelity gaps and reinterpretation ownership

The default dialect preserves baseline ownership on identical-extent surfaces.
`ZantufaSelbriReinterpretation` changes ownership only for full-selbri CEI
absorption and selbri-level description-relative attachment, using the same
gated grammar arms but disabling their baseline-boundary classifiers. The
single-gik branch-width and flat-versus-right-recursive CO divergences require
different grammar shapes and remain documented gaps owned by follow-up #858.

The expectation-comparison classes and the complete manual-residue disposition
will be added here before the consolidated pre-existing-fixture regeneration.
