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

### C4 Zantufa CEI and KE/CO dispositions

The three-parser reference trees are recorded in
`/build/jbotci/logs/epoch05-c4-reference-probes.log`. Full-selbri CEI uses an
extension-first candidate under `ZantufaTerms`, with
`ExperimentalZantufaSelbriAssignment` anchored on each CEI retained by the
extension owner. The completed-candidate classifier recognizes the actual
pre-C4 tree shape across the whole extent: each operand must begin with an
untagged L2 selbri, and neither the leading tree nor any operand may contain a
surviving C4-only node. Generated traversal checks every descendant, while the
candidate and assignment products are destructured without `..`; the recovered
mirror rejects only fully valid candidates with the same shape.

This is intentionally not a per-operand-width test. Splitting the first linked
unit from `brode brodi` leaves ordinary adjacency, so the complete S15 surface
has a same-extent baseline tree and is returned to the standard owner. By
contrast, the surviving nested assignment in S25 contains an NA-led operand,
which proves that no pre-C4 tree covers the complete extent.

| Surface | Current owner | Diagnostics and disposition |
| --- | --- | --- |
| `mi broda cei brode brodi` | Standard `TanruUnit` CEI plus outer adjacency | Warning-free. Baseline ownership is pinned; rolling Zantufa absorbs the adjacency into the CEI operand. This fidelity pin flips under `ZantufaSelbriReinterpretation` in C5. |
| `mi broda cei brode cei brodi` | Standard flat `TanruUnit` CEI chain | Warning-free. The nested rolling-Zantufa CEI tree is an A24 fidelity gap; C5's reinterpretation flag inherits the pin to flip. |
| `mi broda cei brode cei na brodi` | Whole Zantufa assigned-selbri candidate whose leading baseline prefix retains the first CEI | Exact `ExperimentalZantufaSelbriAssignment` warning on the extension-owned second CEI; the baseline-owned first CEI remains warning-free. Standard and camxes-exp reject. |
| `mi broda cei na brode` | Zantufa assigned-selbri candidate | Exact assignment warning on CEI; the full operand is NA-led. |
| `mi broda cei pu brode` | Zantufa assigned-selbri candidate | Exact assignment warning on CEI; the full operand is tagged. |

KE-with-CO is independently gated by `ZantufaConnectives`. Its atom contains a
flat list of L3 operands and requires at least one direct CO continuation, so a
non-CO KE group cannot enter the arm. `ExperimentalZantufaKeCoGrouping` is
anchored on KE. Semantic/reference lowering folds the flat list from the right,
preserving the multiple-CO meaning specified by CLL §5.8 while retaining the
rolling-Zantufa syntax topology.

| Surface | Current owner | Diagnostics and disposition |
| --- | --- | --- |
| `lo ke broda co brode co brodi ke'e ku` | Flat Zantufa KE/CO atom with two direct CO tails in a description selbri | Exact KE/CO grouping warning; camxes-standard and camxes-exp reject while rolling Zantufa accepts. |
| `lo ke broda co brode ke'e cei na brodi ku` | Flat KE/CO atom followed by a full-selbri CEI assignment in a description selbri | Exact KE/CO grouping warning on KE plus assignment warning on CEI; camxes-standard and camxes-exp reject while rolling Zantufa accepts. |
| `lo na'e ke broda co brode ke'e ku` | Standard NAhE atom wrapper around the flat KE/CO atom in a description selbri | Exact KE/CO grouping warning; this is S18's sourced wider Zantufa scope. Camxes-standard and camxes-exp reject while rolling Zantufa accepts. |
| `lo ke broda brode ke'e ku` | Standard grouped tanru atom in a description selbri | Warning-free isolating no-delta control; the C4 arm is structurally unavailable. All three running reference parsers accept this surface. |

The S14 gate matrix pins the two gates independently: no dialect, either
feature alone, and the standard profile reject the complete mixed surface;
explicitly enabling both features and the built-in Zantufa profile accept it
with exactly the KE/CO and full-selbri-CEI warnings. The default parse without
a dialect definition does not implicitly enable either warning-gated owner.

The top-level `mi ke broda brode ke'e` surface remains owned by the epoch-2/3
Zantufa grouped-bridi-tail arm and retains its
`ExperimentalZantufaGroupedBridiTail` warning. That existing pin is the outer
no-delta control: C4 does not reclassify it through the atom-level direct-CO
gate. The description row above separately proves the selbri-family behavior
without that bridi-tail escape.

### C5 relative-attachment and fidelity dispositions

Selbri-level relative attachment is an additive `ZantufaTerms` owner. Its
relative-list marker carries
`ExperimentalZantufaSelbriRelativePlacement`; the existing statement-body
warning remains independent. The owner occurs before CEI assignments, so the
explicit-KUhO witness `mi broda poi brode ku'o cei na brodi` retains a single
selbri-level relative list followed by the full NA-led assignment. The default
profile rejects that surface. The non-description control
`mi broda noi brode` is likewise rejected without Zantufa terms and accepted
with one selbri-relative-placement warning when the feature is enabled. The
elided-KUhO spelling is deliberately absent from the witness set because its
relative body absorbs the following words and does not isolate this owner.

Description and vocative consumers use a typed
`SelbriWithoutTerminalRelativeSyntax` entry. The restriction follows every
rightmost recursive edge rather than inspecting tokens after parsing: NA
recursion and the final full-selbri CEI operand remain restricted, while all
earlier CEI operands remain full width. The following independently pinned
surfaces prove the three steal paths and exact warning ownership:

| Surface | Pinned owner and warning disposition |
| --- | --- |
| `lo na broda poi brode ku` | Outer description relative after an NA selbri; statement-body warning only. |
| `lo broda cei brode poi brodi ku` | Outer description relative after the final CEI operand; the same-extent standard CEI owner remains warning-free, and POI has only the statement-body warning. |
| `lo na broda cei brode poi brodi ku` | The combined NA/final-CEI right spine; again only the statement-body warning is present. |

The quantifier-selbri consumer is split by actual terminator ownership.
`re broda poi brode ku` is rejected by the default profile and accepted under
Zantufa terms with the relative attached to the selbri before explicit KU.
`re broda poi brode` keeps the standard outer-relative tree in both profiles;
the elided KU is zero-width and no selbri-relative-placement warning is
introduced. Both Zantufa-profile pins retain the independent Zantufa-mex
warning on `re`; both also retain the statement-body warning on POI, and only
the explicit-KU pin adds the selbri-relative-placement warning. The terminator
distinction is structural: explicit KU selects the full selbri entry, while an
elided KU selects the restricted entry.

Description no-steal and continuation behavior is pinned in every applicable
feature configuration:

| Surface family | No dialect / `()` | `+zantufa-connectives` | `+zantufa-terms` | both features / `(zantufa)` |
| --- | --- | --- | --- | --- |
| Single POI: `lo broda poi brode ku` | Baseline outer relative, warning-free | Same baseline owner | Same baseline owner; statement-body warning only | Same baseline owner; statement-body warning only |
| ZIhE pair: `lo broda poi brode zi'e poi brodi ku` | Baseline joined outer list, warning-free | Same baseline owner | Same baseline owner; statement-body warnings only | Same baseline owner; statement-body warnings only |
| Bare pair: `lo broda poi brode poi brodi ku` | Reject | Reject | Baseline outer list plus one `ZantufaBareRelativeClauseTail` | Same gated continuation owner |
| Bare triple: `lo broda poi brode poi brodi poi brodo ku` | Reject | Reject | Baseline outer list plus two source-ordered bare tails | Same gated continuation owner |

The explicit `()` and omitted-dialect fixtures are separate pins in each
six-configuration family. Enabling only Zantufa connectives never enables a
bare continuation. Every accepted bare tail carries its own
`ExperimentalZantufaSelbriRelativePlacement` warning, in addition to the
statement-body warnings on its POI clauses. ZIhE remains the shared standard
continuation and does not acquire a placement warning.

`ZantufaSelbriReinterpretation` is a meaning-changing flag and is inert unless
the corresponding Zantufa terms owner is also enabled. It bypasses only the
two completed-candidate boundary classifiers specified by D6:

| Flag witness | Default `(zantufa)` | Reinterpretation enabled |
| --- | --- | --- |
| `lo broda cei brode brodi ku` | Baseline tanru-unit CEI plus outer adjacency, warning-free | One full-selbri assignment whose operand absorbs `brode brodi`; assignment warning on CEI |
| `lo broda poi brode ku` | Outer description relative | One `ZantufaRelativeSelbri` with a selbri-relative-placement warning |
| `lo broda poi brode poi brodi ku` | Outer description list plus gated bare continuation | One selbri-level relative list containing the bare continuation; placement warnings remain exact |
| `coi broda poi brode do'u` | Standard trailing vocative relative | Selbri-level relative ownership with a placement warning |

The separate default and flag-enabled vocative fixtures, plus the default
baseline fixture, pin all three ownership states. Nested CEI reinterpretation
uses the same assignment arm; the C4 nested-CEI gap is therefore covered by
the flag even though the compact witness matrix uses S15 as its primary CEI
pair. Single-GI branch width and CO associativity remain #858 and are not
changed by this flag.

Bare-relative parsing instantiates the existing statement/bridi family with
the restricted selbri entry, so the next POI stays visible to the containing
list. Full-selbri CEI parsing similarly instantiates the rebuilt ladder with
only its leading tanru-unit CEI repetition removed. Generated rule memos now
include a balanced typed `SyntaxMemoScope` (`Ordinary`, `CeiFree`,
`DescriptionRelative`, or their union), for both strict and recovery parses.
This prevents two parameterized instantiations with the same generated rule
name and token offset from replaying each other's result; it changes no
grammar ownership by itself. A focused test pins the collision and a malformed
description-relative-body recovery test pins conservation of the outer
relative-list structure.

Finally, `lo broda goi ko'a ku` remains a standard GOI relative under the
built-in Zantufa profile with no C5 ownership delta. Full-term GOI expansion is
#794/epoch 6 and receives no selbri-relative warning here.

## Fidelity gaps and reinterpretation ownership

The default dialect preserves baseline ownership on identical-extent surfaces.
`ZantufaSelbriReinterpretation` changes ownership only for full-selbri CEI
absorption and selbri-level description-relative attachment, using the same
gated grammar arms but disabling their baseline-boundary classifiers. The
single-gik branch-width and flat-versus-right-recursive CO divergences require
different grammar shapes and remain documented gaps owned by follow-up #858.

### C6 consolidated regeneration and comparer audit

The comparer was completed and exercised against the complete C5 baseline archive before the checked-in regeneration was accepted. It excludes the 110 C1–C5 witnesses added after epoch base `9fafb66d4a`, except for the one C4 CEI witness whose deferred dual-warning/Gentufa refresh is mandated here. The fail-closed ratchet covers 20,729 eligible C5-to-C6 fixture deltas (20,728 pre-epoch fixtures plus that mandated C4 refresh). Residue-free mechanical class incidences are 15,699 recursive single-unit-wrapper fixtures, 4,001 pure-adjacency fixtures, 6 pure actual-old-shape joik/jek fixtures, and 5 pure connectiveless/stagless plain-BO fixtures. A fixture can contain multiple independent selbri sites, while any exact non-tree delta keeps the whole fixture in manual residue rather than normalizing it.

The recursive simple-unit predicate excludes KE, CEI, forethought, tags, NAhE, relatives, linkargs, and warning-gated atoms. The comparer classifies connector and BO shapes from the actual old tree, validates exact source spans, and derives a zero-width position for every absent KEhE/FEhU/KEI owner on both sides. Diagnostics and all other non-tree leaves are exact. Anything outside the four classes is residue; none of the following 3,334 cases is normalized or silently accepted.

The checked-in syntax regeneration visited all 26,412 fixtures and rewrote 13,041 files. A second all-fixture Gentufa refresh visited all 26,412 fixtures and rewrote zero further files. The consolidated all-facet rewrite then visited all 26,412 fixtures and rewrote 554 files: 456 semantics-ref facets and 162 tersmu-json facets (some fixtures contain both). `corpus/camxes/5521.toml` is the sole standard acceptance flip; the running camxes standard parser independently rejects its source.

<details>
<summary>Individual manual-residue dispositions (3,334)</summary>

| Fixture | Disposition |
| --- | --- |
| `adhoc/issues/issue-729-cbm-la-cmevla-tanru-description.toml` | Downstream semantic-projection residue manually repinned (expectations.output.tersmu.json); exact refs/tersmu output retained, no normalization. |
| `adhoc/issues/issue-791/05-mixed-loose-and-bound-links.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `adhoc/issues/issue-791/06-loose-link-chain.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `adhoc/issues/issue-791/07-bei-bound-link.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `adhoc/issues/issue-791/08-bound-link-with-stag.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `adhoc/issues/issue-791/10-empty-link-preserved.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `adhoc/issues/issue-791/21-ke-termset-loose-connection-standard.toml` | Manual materialization/removal residue: expectation leaves added: expectations.output.gentufa.json, expectations.syntax.raw; exact generated syntax/Gentufa leaves and token/span projections pinned. |
| `adhoc/issues/issue-791/22-ke-termset-loose-connection-term-hierarchy.toml` | Manual materialization/removal residue: expectation leaves added: expectations.output.gentufa.json, expectations.syntax.raw; exact generated syntax/Gentufa leaves and token/span projections pinned. |
| `adhoc/output/gentufa-show-elided.toml` | Show-elided JSON wrapper ladder manually repinned; zero-width VAU [8,8] and token projection retained; no normalization. |
| `adhoc/syntax/forethought/issue-832-bihi-led-operand.toml` | Manual materialization/removal residue: expectation leaves added: expectations.output.gentufa.json, expectations.output.gentufa.tree, expectations.syntax.raw; exact generated syntax/Gentufa leaves and token/span projections pinned. |
| `adhoc/syntax/forethought/issue-832-ga-bo.toml` | Manual materialization/removal residue: expectation leaves added: expectations.output.gentufa.json, expectations.output.gentufa.tree; exact generated syntax/Gentufa leaves and token/span projections pinned. |
| `adhoc/syntax/forethought/issue-832-gaho-na-bihi-operand-unsupported.toml` | Manual materialization/removal residue: expectation leaves added: expectations.output.gentufa.json, expectations.output.gentufa.tree, expectations.syntax.raw; exact generated syntax/Gentufa leaves and token/span projections pinned. |
| `adhoc/syntax/forethought/issue-832-gaho-na-joi-unsupported.toml` | Manual materialization/removal residue: expectation leaves added: expectations.output.gentufa.json, expectations.output.gentufa.tree, expectations.syntax.raw; exact generated syntax/Gentufa leaves and token/span projections pinned. |
| `adhoc/syntax/forethought/issue-832-gi-left-gaho.toml` | Manual materialization/removal residue: expectation leaves added: expectations.output.gentufa.json, expectations.output.gentufa.tree; exact generated syntax/Gentufa leaves and token/span projections pinned. |
| `adhoc/syntax/forethought/issue-832-gi-na-joik.toml` | Manual materialization/removal residue: expectation leaves added: expectations.output.gentufa.json, expectations.output.gentufa.tree; exact generated syntax/Gentufa leaves and token/span projections pinned. |
| `adhoc/syntax/forethought/issue-832-gi-paired-gaho-joi.toml` | Manual materialization/removal residue: expectation leaves added: expectations.output.gentufa.json, expectations.output.gentufa.tree; exact generated syntax/Gentufa leaves and token/span projections pinned. |
| `adhoc/syntax/forethought/issue-832-gi-se-pu-ordering.toml` | Manual materialization/removal residue: expectation leaves added: expectations.output.gentufa.json, expectations.output.gentufa.tree, expectations.syntax.raw; exact generated syntax/Gentufa leaves and token/span projections pinned. |
| `adhoc/syntax/forethought/issue-832-gi-tag.toml` | Manual materialization/removal residue: expectation leaves added: expectations.output.gentufa.json, expectations.output.gentufa.tree; exact generated syntax/Gentufa leaves and token/span projections pinned. |
| `adhoc/syntax/forethought/issue-832-paragraph-na-joi.toml` | Manual materialization/removal residue: expectation leaves added: expectations.output.gentufa.json, expectations.output.gentufa.tree; exact generated syntax/Gentufa leaves and token/span projections pinned. |
| `adhoc/syntax/forethought/issue-832-recovered-se-joi-disjoint.toml` | Manual materialization/removal residue: expectation leaves added: expectations.output.gentufa.json, expectations.output.gentufa.tree, expectations.syntax.raw; expectation leaves removed: expectations.syntax.recovered.diagnostics; exact generated syntax/Gentufa leaves and token/span projections pinned. |
| `adhoc/syntax/forethought/issue-832-term-na-joi-baseline.toml` | Manual materialization/removal residue: expectation leaves added: expectations.output.gentufa.json, expectations.output.gentufa.tree; exact generated syntax/Gentufa leaves and token/span projections pinned. |
| `adhoc/syntax/forethought/issue-832-term-na-joi-both.toml` | Manual materialization/removal residue: expectation leaves added: expectations.output.gentufa.json, expectations.output.gentufa.tree; exact generated syntax/Gentufa leaves and token/span projections pinned. |
| `adhoc/syntax/forethought/issue-832-term-na-joi-connectives.toml` | Manual materialization/removal residue: expectation leaves added: expectations.output.gentufa.json, expectations.output.gentufa.tree; exact generated syntax/Gentufa leaves and token/span projections pinned. |
| `adhoc/syntax/forethought/issue-832-term-na-joi-default-union.toml` | Manual materialization/removal residue: expectation leaves added: expectations.output.gentufa.json, expectations.output.gentufa.tree; exact generated syntax/Gentufa leaves and token/span projections pinned. |
| `adhoc/syntax/forethought/issue-832-term-na-joi-tags.toml` | Manual materialization/removal residue: expectation leaves added: expectations.output.gentufa.json, expectations.output.gentufa.tree; exact generated syntax/Gentufa leaves and token/span projections pinned. |
| `adhoc/syntax/forethought/issue-832-term-na-joi-zantufa.toml` | Manual materialization/removal residue: expectation leaves added: expectations.output.gentufa.json, expectations.output.gentufa.tree; exact generated syntax/Gentufa leaves and token/span projections pinned. |
| `adhoc/syntax/mekso/issue-811-ek-ke-group.toml` | Manual materialization/removal residue: expectation leaves added: expectations.output.gentufa.json, expectations.output.gentufa.tree; exact generated syntax/Gentufa leaves and token/span projections pinned. |
| `adhoc/syntax/mekso/issue-811-forethought-right-width.toml` | Manual materialization/removal residue: expectation leaves added: expectations.output.gentufa.json, expectations.output.gentufa.tree; exact generated syntax/Gentufa leaves and token/span projections pinned. |
| `adhoc/syntax/mekso/issue-811-jek-operand-reowned-zantufa.toml` | Manual materialization/removal residue: expectation leaves added: expectations.output.gentufa.json, expectations.output.gentufa.tree; exact generated syntax/Gentufa leaves and token/span projections pinned. |
| `adhoc/syntax/mekso/issue-811-jek-operand-reowned.toml` | Manual materialization/removal residue: expectation leaves added: expectations.output.gentufa.json, expectations.output.gentufa.tree; exact generated syntax/Gentufa leaves and token/span projections pinned. |
| `adhoc/syntax/mekso/issue-811-joik-ke-group.toml` | Manual materialization/removal residue: expectation leaves added: expectations.output.gentufa.json, expectations.output.gentufa.tree; exact generated syntax/Gentufa leaves and token/span projections pinned. |
| `adhoc/syntax/mekso/issue-811-lahe-qualified-operand.toml` | Manual materialization/removal residue: expectation leaves added: expectations.output.gentufa.json, expectations.output.gentufa.tree; exact generated syntax/Gentufa leaves and token/span projections pinned. |
| `adhoc/syntax/mekso/issue-812-selbri-guhek-nahe-regression.toml` | Excluded actual-old-tree shape (forethought); exact regenerated tree pinned manually, no normalization. |
| `adhoc/syntax/mekso/issue-813-interval-lau-number.toml` | Manual materialization/removal residue: expectation leaves added: expectations.output.gentufa.json, expectations.output.gentufa.tree; exact generated syntax/Gentufa leaves and token/span projections pinned. |
| `adhoc/syntax/mekso/issue-813-interval-tei-foi-number.toml` | Manual materialization/removal residue: expectation leaves added: expectations.output.gentufa.json, expectations.output.gentufa.tree; exact generated syntax/Gentufa leaves and token/span projections pinned. |
| `adhoc/syntax/mekso/issue-813-johi-rp-payload-rejected.toml` | Failure-frontier/status residue manually repinned: failure [] → failure [syntax.unexpected-cmavo@[8,12] "fu'a"]; exact diagnostics retained, no normalization. |
| `adhoc/syntax/mekso/issue-813-johi-standard-width.toml` | Manual materialization/removal residue: expectation leaves added: expectations.output.gentufa.json, expectations.output.gentufa.tree; exact generated syntax/Gentufa leaves and token/span projections pinned. |
| `adhoc/syntax/mekso/issue-813-johi-zantufa-width.toml` | Failure-frontier/status residue manually repinned: failure [] → failure [syntax.unexpected-cmavo@[17,21] "te'u"]; exact diagnostics retained, no normalization. |
| `adhoc/syntax/mekso/issue-813-lerfu-moi-ownership-zantufa.toml` | Manual materialization/removal residue: expectation leaves added: expectations.output.gentufa.json, expectations.output.gentufa.tree; exact generated syntax/Gentufa leaves and token/span projections pinned. |
| `adhoc/syntax/mekso/issue-813-lerfu-moi-ownership.toml` | Manual materialization/removal residue: expectation leaves added: expectations.output.gentufa.json, expectations.output.gentufa.tree; exact generated syntax/Gentufa leaves and token/span projections pinned. |
| `adhoc/syntax/mekso/issue-813-pa-indicator-before-boi.toml` | Manual materialization/removal residue: expectation leaves added: expectations.output.gentufa.json, expectations.output.gentufa.tree; exact generated syntax/Gentufa leaves and token/span projections pinned. |
| `adhoc/syntax/mekso/issue-813-pa-moi-ownership-zantufa.toml` | Manual materialization/removal residue: expectation leaves added: expectations.output.gentufa.json, expectations.output.gentufa.tree; exact generated syntax/Gentufa leaves and token/span projections pinned. |
| `adhoc/syntax/mekso/issue-813-pa-moi-ownership.toml` | Manual materialization/removal residue: expectation leaves added: expectations.output.gentufa.json, expectations.output.gentufa.tree; exact generated syntax/Gentufa leaves and token/span projections pinned. |
| `adhoc/syntax/mekso/issue-835-zantufa-priority-bihe-omitted-right.toml` | Manual materialization/removal residue: expectation leaves added: expectations.output.gentufa.json, expectations.output.gentufa.tree; exact generated syntax/Gentufa leaves and token/span projections pinned. |
| `adhoc/syntax/mekso/issue-835-zantufa-priority-bo-group.toml` | Manual materialization/removal residue: expectation leaves added: expectations.output.gentufa.json, expectations.output.gentufa.tree; exact generated syntax/Gentufa leaves and token/span projections pinned. |
| `adhoc/syntax/mekso/issue-835-zantufa-priority-no-steal.toml` | Manual materialization/removal residue: expectation leaves added: expectations.output.gentufa.json, expectations.output.gentufa.tree; exact generated syntax/Gentufa leaves and token/span projections pinned. |
| `adhoc/syntax/mekso/issue-835-zantufa-priority-trailing-operator.toml` | Manual materialization/removal residue: expectation leaves added: expectations.output.gentufa.json, expectations.output.gentufa.tree; exact generated syntax/Gentufa leaves and token/span projections pinned. |
| `adhoc/syntax/mekso/issue-836-mai-qualified-gap-union.toml` | Manual materialization/removal residue: expectation leaves added: expectations.output.gentufa.json, expectations.output.gentufa.tree, expectations.syntax.raw; exact generated syntax/Gentufa leaves and token/span projections pinned. |
| `adhoc/syntax/mekso/issue-836-xi-forethought.toml` | Excluded actual-old-tree shape (forethought,warning-gated); exact regenerated tree pinned manually, no normalization. |
| `adhoc/syntax/mekso/issue-836-xi-mohe.toml` | Excluded actual-old-tree shape (warning-gated); exact regenerated tree pinned manually, no normalization. |
| `adhoc/syntax/mekso/issue-836-xi-nested-reverse-polish.toml` | Excluded actual-old-tree shape (warning-gated); exact regenerated tree pinned manually, no normalization. |
| `adhoc/syntax/mekso/issue-836-xi-reverse-polish.toml` | Excluded actual-old-tree shape (warning-gated); exact regenerated tree pinned manually, no normalization. |
| `adhoc/syntax/selbri/issue-829-cei-extension-reach-zantufa.toml` | Mandated C4 witness refresh: exact warning set changed from [syntax.warning.experimental-zantufa-selbri-assignment@[19,22] 'cei'] to [syntax.warning.experimental-zantufa-selbri-assignment@[9,12] 'cei', syntax.warning.experimental-zantufa-selbri-assignment@[19,22] 'cei']; regenerated Gentufa JSON/tree pins now cover both CEI owners. |
| `adhoc/syntax/sumti-continuation/baseline-zihe.toml` | Manual materialization/removal residue: expectation leaves added: expectations.output.gentufa.json, expectations.output.gentufa.tree; exact generated syntax/Gentufa leaves and token/span projections pinned. |
| `adhoc/syntax/sumti-continuation/cehe-term-isolation.toml` | Manual materialization/removal residue: expectation leaves added: expectations.output.gentufa.json, expectations.output.gentufa.tree; exact generated syntax/Gentufa leaves and token/span projections pinned. |
| `adhoc/syntax/sumti-continuation/exp-a.toml` | Manual materialization/removal residue: expectation leaves added: expectations.output.gentufa.json, expectations.output.gentufa.tree; exact generated syntax/Gentufa leaves and token/span projections pinned. |
| `adhoc/syntax/sumti-continuation/exp-free-after-connective.toml` | Manual materialization/removal residue: expectation leaves added: expectations.output.gentufa.json, expectations.output.gentufa.tree; exact generated syntax/Gentufa leaves and token/span projections pinned. |
| `adhoc/syntax/sumti-continuation/exp-ja.toml` | Manual materialization/removal residue: expectation leaves added: expectations.output.gentufa.json, expectations.output.gentufa.tree; exact generated syntax/Gentufa leaves and token/span projections pinned. |
| `adhoc/syntax/sumti-continuation/exp-na-ja-nai.toml` | Manual materialization/removal residue: expectation leaves added: expectations.output.gentufa.json, expectations.output.gentufa.tree; exact generated syntax/Gentufa leaves and token/span projections pinned. |
| `adhoc/syntax/sumti-continuation/exp-pooi-follower.toml` | Manual materialization/removal residue: expectation leaves added: expectations.output.gentufa.json, expectations.output.gentufa.tree; exact generated syntax/Gentufa leaves and token/span projections pinned. |
| `adhoc/syntax/sumti-continuation/exp-voie-follower.toml` | Manual materialization/removal residue: expectation leaves added: expectations.output.gentufa.json, expectations.output.gentufa.tree; exact generated syntax/Gentufa leaves and token/span projections pinned. |
| `adhoc/syntax/sumti-continuation/jehi-spellings.toml` | Manual materialization/removal residue: expectation leaves added: expectations.output.gentufa.json, expectations.output.gentufa.tree; exact generated syntax/Gentufa leaves and token/span projections pinned. |
| `adhoc/syntax/sumti-continuation/lahe-relative-twin.toml` | Manual materialization/removal residue: expectation leaves added: expectations.output.gentufa.json, expectations.output.gentufa.tree; exact generated syntax/Gentufa leaves and token/span projections pinned. |
| `adhoc/syntax/sumti-continuation/nahe-bo-relative.toml` | Manual materialization/removal residue: expectation leaves added: expectations.output.gentufa.json, expectations.output.gentufa.tree; exact generated syntax/Gentufa leaves and token/span projections pinned. |
| `adhoc/syntax/sumti-continuation/vuho-bare.toml` | Manual materialization/removal residue: expectation leaves added: expectations.output.gentufa.json, expectations.output.gentufa.tree; exact generated syntax/Gentufa leaves and token/span projections pinned. |
| `adhoc/syntax/sumti-continuation/vuho-baseline.toml` | Manual materialization/removal residue: expectation leaves added: expectations.output.gentufa.json, expectations.output.gentufa.tree; exact generated syntax/Gentufa leaves and token/span projections pinned. |
| `adhoc/syntax/sumti-continuation/vuho-lahe-elided-luhu.toml` | Manual materialization/removal residue: expectation leaves added: expectations.output.gentufa.json, expectations.output.gentufa.tree; exact generated syntax/Gentufa leaves and token/span projections pinned. |
| `adhoc/syntax/sumti-continuation/vuho-lahe-explicit-luhu.toml` | Manual materialization/removal residue: expectation leaves added: expectations.output.gentufa.json, expectations.output.gentufa.tree; exact generated syntax/Gentufa leaves and token/span projections pinned. |
| `adhoc/syntax/sumti-continuation/vuho-term-no-steal.toml` | Manual materialization/removal residue: expectation leaves added: expectations.output.gentufa.json, expectations.output.gentufa.tree; exact generated syntax/Gentufa leaves and token/span projections pinned. |
| `adhoc/syntax/sumti-continuation/vuhu-sumti.toml` | Manual materialization/removal residue: expectation leaves added: expectations.output.gentufa.json, expectations.output.gentufa.tree; exact generated syntax/Gentufa leaves and token/span projections pinned. |
| `adhoc/syntax/tags/issue-822-post-na-extension-gap.toml` | Failure-frontier/status residue manually repinned: failure [] → failure [syntax.unexpected-cmavo@[9,12] 'roi']; exact diagnostics retained, no normalization. |
| `adhoc/syntax/tags/issue-822-post-na-no-steal.toml` | Manual materialization/removal residue: expectation leaves added: expectations.output.gentufa.json, expectations.output.gentufa.tree; exact generated syntax/Gentufa leaves and token/span projections pinned. |
| `adhoc/syntax/tags/issue-822-vuhu-not-adopted.toml` | Failure-frontier/status residue manually repinned: failure [syntax.incomplete-selbri@[24,24] ''] → failure [syntax.unexpected-end@[24,24] '']; exact diagnostics retained, no normalization. |
| `adhoc/syntax/tags/issue-833-gihek-ke-zantufa-tag-rejected.toml` | Failure-frontier/status residue manually repinned: failure [] → failure [syntax.unexpected-brivla@[21,26] 'tavla']; exact diagnostics retained, no normalization. |
| `adhoc/syntax/tags/issue-833-post-na-zantufa-only.toml` | Manual materialization/removal residue: expectation leaves added: expectations.output.gentufa.json, expectations.output.gentufa.tree; exact generated syntax/Gentufa leaves and token/span projections pinned. |
| `adhoc/syntax/tags/issue-833-selbri-joik-tag-bo-bridi.toml` | Manual materialization/removal residue: expectation leaves added: expectations.output.gentufa.json, expectations.output.gentufa.tree; exact generated syntax/Gentufa leaves and token/span projections pinned. |
| `adhoc/syntax/tags/issue-833-selbri-joik-tag-bo-description.toml` | Manual materialization/removal residue: expectation leaves added: expectations.output.gentufa.json, expectations.output.gentufa.tree; exact generated syntax/Gentufa leaves and token/span projections pinned. |
| `adhoc/syntax/tags/issue-833-stag-position-zantufa-rejected.toml` | Failure-frontier/status residue manually repinned: failure [] → failure [syntax.incomplete-mekso@[28,28] '']; exact diagnostics retained, no normalization. |
| `adhoc/v0/syntax/basic/be-linkargs.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `adhoc/v0/warnings/cbm/cbm-la-cmevla-tanru-descriptor-reinterpretation.toml` | Downstream semantic-projection residue manually repinned (expectations.output.tersmu.json); exact refs/tersmu output retained, no normalization. |
| `adhoc/v0/warnings/experimental/broad-a-relation-connective.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `adhoc/v0/warnings/experimental/empty-be-before-bei-linkargs.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `adhoc/v0/warnings/experimental/empty-postposed-be-linkargs.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `adhoc/v0/warnings/experimental/empty-postposed-bei-linkargs.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `adhoc/v0/warnings/experimental/empty-preposed-be-linkargs-are-preposed.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `adhoc/v0/warnings/experimental/empty-preposed-be-linkargs.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `adhoc/v0/warnings/experimental/experimental-koha-nauhu.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `adhoc/v0/warnings/experimental/preposed-be-linkargs.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `adhoc/v0/warnings/standard-no-warning/standard-guha-relation-forethought-connective.toml` | Excluded actual-old-tree shape (forethought); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `adhoc/v0/warnings/standard-no-warning/standard-postposed-be-linkargs.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `adhoc/v0/warnings/standard-no-warning/standard-postposed-bei-linkargs.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `adhoc/v0/warnings/standard-no-warning/standard-selbri-connective.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `cll/chapter-05/section-5.10/c5e10d10.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-05/section-5.10/c5e10d11.toml` | Downstream semantic-projection residue manually repinned (expectations.output.tersmu.json); exact refs/tersmu output retained, no normalization. |
| `cll/chapter-05/section-5.11/c5e11d4.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-05/section-5.12/c5e12d1.toml` | Excluded actual-old-tree shape (KE,NAhE); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-05/section-5.12/c5e12d10.toml` | Excluded actual-old-tree shape (KE,NAhE,linkargs,tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.12/c5e12d11.toml` | Excluded actual-old-tree shape (KE,NAhE,linkargs,tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.12/c5e12d2.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-05/section-5.12/c5e12d3.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-05/section-5.12/c5e12d4.toml` | Excluded actual-old-tree shape (NAhE,linkargs,tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.12/c5e12d5.toml` | Excluded actual-old-tree shape (KE,NAhE,linkargs,tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.12/c5e12d6.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.12/c5e12d7.toml` | Excluded actual-old-tree shape (KE,linkargs,tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.12/c5e12d8.toml` | Excluded actual-old-tree shape (NAhE,linkargs,tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.12/c5e12d9.toml` | Excluded actual-old-tree shape (KE,NAhE,linkargs,tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.16/c5e16d1.toml` | Downstream semantic-projection residue manually repinned (expectations.output.tersmu.json); exact refs/tersmu output retained, no normalization. |
| `cll/chapter-05/section-5.16/c5e16d10.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.16/c5e16d11.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.16/c5e16d12.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.16/c5e16d13.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.16/c5e16d14.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.16/c5e16d15.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.16/c5e16d16.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.16/c5e16d17.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.16/c5e16d18.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.16/c5e16d19.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.16/c5e16d2.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.16/c5e16d20.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.16/c5e16d21.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.16/c5e16d22.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.16/c5e16d23.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.16/c5e16d24.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.16/c5e16d25.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.16/c5e16d26.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.16/c5e16d27.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.16/c5e16d28.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.16/c5e16d29.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.16/c5e16d3.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.16/c5e16d30.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.16/c5e16d31.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.16/c5e16d32.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.16/c5e16d33.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.16/c5e16d34.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.16/c5e16d35.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.16/c5e16d36.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.16/c5e16d37.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.16/c5e16d38.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.16/c5e16d39.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.16/c5e16d4.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.16/c5e16d40.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.16/c5e16d5.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.16/c5e16d6.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.16/c5e16d7.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.16/c5e16d8.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `cll/chapter-05/section-5.16/c5e16d9.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.3/c5e3d4.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.3/c5e3d5.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.3/c5e3d6.toml` | Downstream semantic-projection residue manually repinned (expectations.output.tersmu.json); exact refs/tersmu output retained, no normalization. |
| `cll/chapter-05/section-5.3/c5e3d7.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.3/c5e3d8.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.3/c5e3d9.toml` | Downstream semantic-projection residue manually repinned (expectations.output.tersmu.json); exact refs/tersmu output retained, no normalization. |
| `cll/chapter-05/section-5.4/c5e4d1.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.4/c5e4d2.toml` | Downstream semantic-projection residue manually repinned (expectations.output.tersmu.json); exact refs/tersmu output retained, no normalization. |
| `cll/chapter-05/section-5.4/c5e4d3.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.4/c5e4d4.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.4/c5e4d5.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.4/c5e4d6.toml` | Downstream semantic-projection residue manually repinned (expectations.output.tersmu.json); exact refs/tersmu output retained, no normalization. |
| `cll/chapter-05/section-5.5/c5e5d1.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.5/c5e5d2.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.5/c5e5d3.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.5/c5e5d4.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.5/c5e5d5.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.5/c5e5d6.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.5/c5e5d7.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.5/c5e5d8.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.6/c5e6d1.toml` | Downstream semantic-projection residue manually repinned (expectations.output.tersmu.json); exact refs/tersmu output retained, no normalization. |
| `cll/chapter-05/section-5.6/c5e6d11.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.6/c5e6d13.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw; expectations.output.tersmu.json); exact refs/tersmu output retained, no normalization. |
| `cll/chapter-05/section-5.6/c5e6d14.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.6/c5e6d15.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.6/c5e6d16.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.6/c5e6d17.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.6/c5e6d18.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.6/c5e6d19.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.6/c5e6d2.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.6/c5e6d20.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.6/c5e6d21.toml` | Downstream semantic-projection residue manually repinned (expectations.output.tersmu.json); exact refs/tersmu output retained, no normalization. |
| `cll/chapter-05/section-5.6/c5e6d22.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.6/c5e6d23.toml` | Excluded actual-old-tree shape (forethought); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.6/c5e6d24.toml` | Excluded actual-old-tree shape (forethought); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.6/c5e6d3.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.6/c5e6d4.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.6/c5e6d5.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.6/c5e6d6.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.6/c5e6d7.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw; expectations.output.tersmu.json); exact refs/tersmu output retained, no normalization. |
| `cll/chapter-05/section-5.6/c5e6d8.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.7/c5e7d10.toml` | Downstream semantic-projection residue manually repinned (expectations.output.tersmu.json); exact refs/tersmu output retained, no normalization. |
| `cll/chapter-05/section-5.7/c5e7d11.toml` | Excluded actual-old-tree shape (linkargs,relative); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-05/section-5.7/c5e7d12.toml` | Excluded actual-old-tree shape (linkargs,relative); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-05/section-5.7/c5e7d13.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.7/c5e7d2.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-05/section-5.7/c5e7d3.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.7/c5e7d4.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-05/section-5.7/c5e7d6.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.7/c5e7d7.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-05/section-5.7/c5e7d8.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-05/section-5.7/c5e7d9.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-05/section-5.8/c5e8d10.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.8/c5e8d11.toml` | Downstream semantic-projection residue manually repinned (expectations.output.tersmu.json); exact refs/tersmu output retained, no normalization. |
| `cll/chapter-05/section-5.8/c5e8d12.toml` | Excluded actual-old-tree shape (CO); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-05/section-5.8/c5e8d13.toml` | Excluded actual-old-tree shape (CO,linkargs); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-05/section-5.8/c5e8d2.toml` | Excluded actual-old-tree shape (CO); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-05/section-5.8/c5e8d3.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-05/section-5.8/c5e8d4.toml` | Excluded actual-old-tree shape (CO); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-05/section-5.8/c5e8d5.toml` | Excluded actual-old-tree shape (CO); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.8/c5e8d6.toml` | Excluded actual-old-tree shape (CO,KE); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.8/c5e8d7.toml` | Excluded actual-old-tree shape (CO); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.8/c5e8d8.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-05/section-5.8/c5e8d9.toml` | Excluded actual-old-tree shape (CO); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-05/section-5.9/c5e9d3.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-05/section-5.9/c5e9d5.toml` | Downstream semantic-projection residue manually repinned (expectations.output.tersmu.json); exact refs/tersmu output retained, no normalization. |
| `cll/chapter-05/section-5.9/c5e9d8.toml` | Downstream semantic-projection residue manually repinned (expectations.output.tersmu.json); exact refs/tersmu output retained, no normalization. |
| `cll/chapter-06/section-6.10/c6e10d1.toml` | Downstream semantic-projection residue manually repinned (expectations.output.tersmu.json); exact refs/tersmu output retained, no normalization. |
| `cll/chapter-06/section-6.10/c6e10d11.toml` | Downstream semantic-projection residue manually repinned (expectations.output.tersmu.json); exact refs/tersmu output retained, no normalization. |
| `cll/chapter-06/section-6.10/c6e10d2.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-06/section-6.10/c6e10d3.toml` | Downstream semantic-projection residue manually repinned (expectations.output.tersmu.json); exact refs/tersmu output retained, no normalization. |
| `cll/chapter-06/section-6.10/c6e10d5.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-06/section-6.11/c6e11d6.toml` | Downstream semantic-projection residue manually repinned (expectations.output.tersmu.json); exact refs/tersmu output retained, no normalization. |
| `cll/chapter-06/section-6.11/c6e11d8.toml` | Downstream semantic-projection residue manually repinned (expectations.output.tersmu.json); exact refs/tersmu output retained, no normalization. |
| `cll/chapter-06/section-6.5/c6e5d3.toml` | Downstream semantic-projection residue manually repinned (expectations.output.tersmu.json); exact refs/tersmu output retained, no normalization. |
| `cll/chapter-07/section-7.5/c7e5d5.toml` | Excluded actual-old-tree shape (CEI); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-07/section-7.5/c7e5d6.toml` | Excluded actual-old-tree shape (CEI); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-07/section-7.5/c7e5d7.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-07/section-7.6/c7e6d12.toml` | Downstream semantic-projection residue manually repinned (expectations.output.tersmu.json); exact refs/tersmu output retained, no normalization. |
| `cll/chapter-07/section-7.6/c7e6d14.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-07/section-7.7/c7e7d1.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-07/section-7.9/c7e9d5.toml` | Downstream semantic-projection residue manually repinned (expectations.output.tersmu.json); exact refs/tersmu output retained, no normalization. |
| `cll/chapter-08/section-8.2/c8e2d6.toml` | Downstream semantic-projection residue manually repinned (expectations.output.tersmu.json); exact refs/tersmu output retained, no normalization. |
| `cll/chapter-08/section-8.3/c8e3d16.toml` | Downstream semantic-projection residue manually repinned (expectations.output.tersmu.json); exact refs/tersmu output retained, no normalization. |
| `cll/chapter-08/section-8.3/c8e3d6.toml` | Excluded actual-old-tree shape (KE,relative); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-08/section-8.3/c8e3d8.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-08/section-8.7/c8e7d5.toml` | Downstream semantic-projection residue manually repinned (expectations.output.tersmu.json); exact refs/tersmu output retained, no normalization. |
| `cll/chapter-08/section-8.8/c8e8d1.toml` | Excluded actual-old-tree shape (NAhE,relative); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-08/section-8.8/c8e8d3.toml` | Downstream semantic-projection residue manually repinned (expectations.output.tersmu.json); exact refs/tersmu output retained, no normalization. |
| `cll/chapter-08/section-8.8/c8e8d4.toml` | Downstream semantic-projection residue manually repinned (expectations.output.tersmu.json); exact refs/tersmu output retained, no normalization. |
| `cll/chapter-09/section-9.13/c9e13d2.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-09/section-9.3/c9e3d2.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-09/section-9.4/c9e4d8.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-09/section-9.5/c9e5d2.toml` | Downstream semantic-projection residue manually repinned (expectations.output.tersmu.json); exact refs/tersmu output retained, no normalization. |
| `cll/chapter-09/section-9.6/c9e6d1.toml` | Downstream semantic-projection residue manually repinned (expectations.output.tersmu.json); exact refs/tersmu output retained, no normalization. |
| `cll/chapter-09/section-9.6/c9e6d2.toml` | Downstream semantic-projection residue manually repinned (expectations.output.tersmu.json); exact refs/tersmu output retained, no normalization. |
| `cll/chapter-09/section-9.6/c9e6d7.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-09/section-9.7/c9e7d1.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-09/section-9.7/c9e7d2.toml` | Downstream semantic-projection residue manually repinned (expectations.output.tersmu.json); exact refs/tersmu output retained, no normalization. |
| `cll/chapter-09/section-9.7/c9e7d4.toml` | Downstream semantic-projection residue manually repinned (expectations.output.tersmu.json); exact refs/tersmu output retained, no normalization. |
| `cll/chapter-09/section-9.7/c9e7d5.toml` | Downstream semantic-projection residue manually repinned (expectations.output.tersmu.json); exact refs/tersmu output retained, no normalization. |
| `cll/chapter-09/section-9.7/c9e7d6.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-09/section-9.7/c9e7d9.toml` | Downstream semantic-projection residue manually repinned (expectations.output.tersmu.json); exact refs/tersmu output retained, no normalization. |
| `cll/chapter-09/section-9.9/c9e9d6.toml` | Downstream semantic-projection residue manually repinned (expectations.output.tersmu.json); exact refs/tersmu output retained, no normalization. |
| `cll/chapter-09/section-9.9/c9e9d8.toml` | Excluded actual-old-tree shape (forethought,tagged); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-10/section-10.10/c10e10d10.toml` | Downstream semantic-projection residue manually repinned (expectations.output.tersmu.json); exact refs/tersmu output retained, no normalization. |
| `cll/chapter-10/section-10.12/c10e12d2.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-10/section-10.13/c10e13d11.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-10/section-10.14/c10e14d1 c10e14d2 c10e14d3 c10e14d4 c10e14d5 c10e14d6 c10e14d7.toml` | Downstream semantic-projection residue manually repinned (expectations.output.tersmu.json); exact refs/tersmu output retained, no normalization. |
| `cll/chapter-10/section-10.14/c10e14d2.toml` | Downstream semantic-projection residue manually repinned (expectations.output.tersmu.json); exact refs/tersmu output retained, no normalization. |
| `cll/chapter-10/section-10.15/c10e15d5.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-10/section-10.15/c10e15d6.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-10/section-10.15/c10e15d7.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-10/section-10.15/c10e15d8.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-10/section-10.15/c10e15d9.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-10/section-10.18/c10e18d9.toml` | Excluded actual-old-tree shape (NAhE,tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-10/section-10.22/c10e22d8.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-10/section-10.22/c10e22d9.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-10/section-10.25/c10e25d1.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-10/section-10.25/c10e25d2.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-10/section-10.26/c10e26d1.toml` | Downstream semantic-projection residue manually repinned (expectations.output.tersmu.json); exact refs/tersmu output retained, no normalization. |
| `cll/chapter-11/section-11.1/c11e1d3.toml` | Downstream semantic-projection residue manually repinned (expectations.output.tersmu.json); exact refs/tersmu output retained, no normalization. |
| `cll/chapter-11/section-11.10/c11e10d10.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-11/section-11.10/c11e10d9.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-11/section-11.3/c11e3d2.toml` | Downstream semantic-projection residue manually repinned (expectations.output.tersmu.json); exact refs/tersmu output retained, no normalization. |
| `cll/chapter-11/section-11.8/c11e8d3.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-11/section-11.8/c11e8d4.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-11/section-11.8/c11e8d5.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-11/section-11.8/c11e8d7.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-11/section-11.9/c11e9d5.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-11/section-11.9/c11e9d6.toml` | Excluded actual-old-tree shape (KE,linkargs); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-12/section-12.11/c12e11d1.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-12/section-12.11/c12e11d2.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-12/section-12.11/c12e11d3.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-12/section-12.11/c12e11d4.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-12/section-12.11/c12e11d5.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-12/section-12.11/c12e11d6.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-12/section-12.12/c12e12d4.toml` | Downstream semantic-projection residue manually repinned (expectations.output.tersmu.json); exact refs/tersmu output retained, no normalization. |
| `cll/chapter-12/section-12.14/c12e14d10.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-12/section-12.14/c12e14d11.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-12/section-12.14/c12e14d9.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-12/section-12.15/c12e15d3.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-12/section-12.15/c12e15d5.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-12/section-12.2/c12e2d3.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-13/section-13.11/c13e11d2.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-13/section-13.12/c13e12d5.toml` | Downstream semantic-projection residue manually repinned (expectations.output.tersmu.json); exact refs/tersmu output retained, no normalization. |
| `cll/chapter-13/section-13.12/c13e12d6.toml` | Downstream semantic-projection residue manually repinned (expectations.output.tersmu.json); exact refs/tersmu output retained, no normalization. |
| `cll/chapter-13/section-13.13/c13e13d2.toml` | Downstream semantic-projection residue manually repinned (expectations.output.tersmu.json); exact refs/tersmu output retained, no normalization. |
| `cll/chapter-13/section-13.3/c13e3d10.toml` | Downstream semantic-projection residue manually repinned (expectations.output.tersmu.json); exact refs/tersmu output retained, no normalization. |
| `cll/chapter-13/section-13.3/c13e3d15.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-14/section-14.12/c14e12d10.toml` | Excluded actual-old-tree shape (forethought); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-14/section-14.12/c14e12d11.toml` | Excluded actual-old-tree shape (forethought); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-14/section-14.12/c14e12d14.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw; expectations.output.tersmu.json); exact refs/tersmu output retained, no normalization. |
| `cll/chapter-14/section-14.12/c14e12d2.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw; expectations.output.tersmu.json); exact refs/tersmu output retained, no normalization. |
| `cll/chapter-14/section-14.12/c14e12d6.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-14/section-14.12/c14e12d8.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-14/section-14.12/c14e12d9.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-14/section-14.14/c14e14d5.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-14/section-14.15/c14e15d3.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw; expectations.output.tersmu.json); exact refs/tersmu output retained, no normalization. |
| `cll/chapter-14/section-14.15/c14e15d4.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw; expectations.output.tersmu.json); exact refs/tersmu output retained, no normalization. |
| `cll/chapter-14/section-14.16/c14e16d8.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-14/section-14.5/c14e5d7.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-15/section-15.11/c15e11d1.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-15/section-15.2/c15e2d10.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-15/section-15.2/c15e2d11.toml` | Excluded actual-old-tree shape (CO); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-15/section-15.2/c15e2d12.toml` | Excluded actual-old-tree shape (CO); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-15/section-15.2/c15e2d6.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-15/section-15.2/c15e2d7.toml` | Excluded actual-old-tree shape (CO); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-15/section-15.4/c15e4d10.toml` | Excluded actual-old-tree shape (KE,NAhE); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-15/section-15.4/c15e4d11.toml` | Excluded actual-old-tree shape (KE,NAhE,linkargs); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-15/section-15.4/c15e4d13.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-15/section-15.4/c15e4d15.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-15/section-15.4/c15e4d16.toml` | Excluded actual-old-tree shape (NAhE,linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-15/section-15.4/c15e4d17.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-15/section-15.4/c15e4d18.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-15/section-15.4/c15e4d2.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-15/section-15.4/c15e4d5.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-15/section-15.4/c15e4d6.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-15/section-15.4/c15e4d7.toml` | Excluded actual-old-tree shape (KE,NAhE); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-15/section-15.4/c15e4d8.toml` | Excluded actual-old-tree shape (KE,NAhE); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-15/section-15.4/c15e4d9.toml` | Excluded actual-old-tree shape (KE,NAhE,linkargs); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chapter-15/section-15.5/c15e5d1.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-15/section-15.5/c15e5d10.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-15/section-15.5/c15e5d11.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-15/section-15.5/c15e5d12.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-15/section-15.5/c15e5d2.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-15/section-15.5/c15e5d3.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-15/section-15.5/c15e5d4.toml` | Excluded actual-old-tree shape (NAhE,linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-15/section-15.5/c15e5d5.toml` | Excluded actual-old-tree shape (NAhE,linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-15/section-15.5/c15e5d6.toml` | Excluded actual-old-tree shape (NAhE,linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-15/section-15.5/c15e5d7.toml` | Excluded actual-old-tree shape (NAhE,linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-15/section-15.5/c15e5d8.toml` | Excluded actual-old-tree shape (NAhE,linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-15/section-15.6/c15e6d1.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-15/section-15.6/c15e6d2.toml` | Excluded actual-old-tree shape (NAhE,linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-15/section-15.8/c15e8d8.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-15/section-15.8/c15e8d9.toml` | Excluded actual-old-tree shape (KE,NAhE,tagged); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-15/section-15.9/c15e9d7.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-16/section-16.8/c16e8d7.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. |
| `cll/chapter-18/section-18.17/c18e17d3.toml` | Downstream semantic-projection residue manually repinned (expectations.output.tersmu.json); exact refs/tersmu output retained, no normalization. |
| `cll/chrestomathy/alice01.toml` | Excluded actual-old-tree shape (CO,KE,NAhE,forethought,linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chrestomathy/forest-nymph.toml` | Excluded actual-old-tree shape (CO,KE,NAhE,forethought,linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chrestomathy/in-xanadu.toml` | Excluded actual-old-tree shape (CO,linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chrestomathy/north-wind.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `cll/chrestomathy/terry.toml` | Excluded actual-old-tree shape (NAhE,linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/alis/full-alice.toml` | Manual materialization/removal residue: expectation leaves added: expectations.output.gentufa.json, expectations.output.gentufa.tree, expectations.syntax.raw; expectation leaves removed: expectations.output.tersmu.status, expectations.syntax.raw.sha256; exact generated syntax/Gentufa leaves and token/span projections pinned. |
| `corpus/camxes/10000.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10010.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10031.toml` | Excluded actual-old-tree shape (NAhE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10045.toml` | Excluded actual-old-tree shape (KE,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10046.toml` | Excluded actual-old-tree shape (KE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10051.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10053.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10061.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10067.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/10087.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10090.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10091.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10095.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10108.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/10129.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10153.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10157.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/10160.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10163.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10165.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/10166.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10168.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10176.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10180.toml` | Excluded actual-old-tree shape (CO); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10188.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10191.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/1020.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1021.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10210.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10215.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10216.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1022.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10225.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10228.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/1023.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10232.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/10234.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1024.toml` | Excluded actual-old-tree shape (NAhE,linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1025.toml` | Excluded actual-old-tree shape (NAhE,linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10256.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1026.toml` | Excluded actual-old-tree shape (NAhE,linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1027.toml` | Excluded actual-old-tree shape (NAhE,linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1028.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10285.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/10289.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1029.toml` | Excluded actual-old-tree shape (NAhE,linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10293.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10297.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10300.toml` | Excluded actual-old-tree shape (KE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10306.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10307.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10311.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10312.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10316.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10325.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/10332.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10335.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1034.toml` | Excluded actual-old-tree shape (KE,linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10344.toml` | Excluded actual-old-tree shape (NAhE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1035.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10355.toml` | Failure-frontier/status residue manually repinned: failure [syntax.incomplete-selbri@[50,50] ''] → failure [syntax.unexpected-end@[50,50] '']; exact diagnostics retained, no normalization. |
| `corpus/camxes/10379.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10387.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10393.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10399.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10412.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10427.toml` | Excluded actual-old-tree shape (CO,KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10450.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10451.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10455.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/10467.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10469.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/10470.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10483.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/10484.toml` | Excluded actual-old-tree shape (KE,linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10506.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1051.toml` | Excluded actual-old-tree shape (NAhE,tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/10515.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10525.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/10544.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10547.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10556.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10600.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10601.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10629.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1065.toml` | Excluded actual-old-tree shape (linkargs,relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10656.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1066.toml` | Excluded actual-old-tree shape (linkargs,relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10660.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1067.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10672.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10689.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10691.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10698.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/10703.toml` | Excluded actual-old-tree shape (KE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10708.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10714.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10719.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/10722.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/1076.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10770.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/10775.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10776.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10779.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10792.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10801.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10807.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10815.toml` | Excluded actual-old-tree shape (KE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10819.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10825.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/10842.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10848.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10896.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10909.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10913.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10914.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10916.toml` | Excluded actual-old-tree shape (linkargs,relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10917.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10924.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/10929.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10934.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10946.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/10967.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11004.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11006.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11012.toml` | Excluded actual-old-tree shape (KE,linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11021.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11027.toml` | Excluded actual-old-tree shape (linkargs,relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11034.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/11049.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11054.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11072.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11087.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11092.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11094.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11102.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11104.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11112.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11113.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11115.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11127.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11133.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11134.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11141.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11147.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11154.toml` | Failure-frontier/status residue manually repinned: failure [syntax.incomplete-selbri@[64,64] ''] → failure [syntax.incomplete-term@[64,64] '']; exact diagnostics retained, no normalization. |
| `corpus/camxes/11155.toml` | Excluded actual-old-tree shape (NAhE,tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/11163.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11164.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11182.toml` | Excluded actual-old-tree shape (linkargs,relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11184.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11185.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11200.toml` | Excluded actual-old-tree shape (linkargs,relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11203.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11204.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11219.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11220.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11222.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/11223.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11248.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11253.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/11257.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1126.toml` | Excluded actual-old-tree shape (NAhE,relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11266.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11272.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11273.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/11280.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11302.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11304.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11325.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/11326.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11333.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11334.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11336.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11337.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11338.toml` | Failure-frontier/status residue manually repinned: failure [syntax.incomplete-selbri@[73,73] ''] → failure [syntax.incomplete-term@[73,73] '']; exact diagnostics retained, no normalization. |
| `corpus/camxes/11350.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11362.toml` | Failure-frontier/status residue manually repinned: failure [syntax.incomplete-selbri@[22,22] ''] → failure [syntax.unexpected-end@[22,22] '']; exact diagnostics retained, no normalization. |
| `corpus/camxes/11369.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11372.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11374.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11378.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11407.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11408.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11417.toml` | Excluded actual-old-tree shape (linkargs,relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11422.toml` | Failure-frontier/status residue manually repinned: failure [syntax.unexpected-cmavo@[32,34] 'le'] → failure [syntax.unexpected-cmavo@[29,31] 'ca']; exact diagnostics retained, no normalization. |
| `corpus/camxes/11425.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11427.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11468.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11485.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11487.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11489.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11492.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11494.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11508.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11516.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11524.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11528.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/11532.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11537.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11538.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11539.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11542.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11544.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11561.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/1157.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1158.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1159.toml` | Excluded actual-old-tree shape (NAhE,linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11592.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11594.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11595.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1160.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11617.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11626.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1163.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11638.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11640.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11645.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11653.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11666.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/11667.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/11673.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11674.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11687.toml` | Failure-frontier/status residue manually repinned: failure [syntax.incomplete-selbri@[25,25] ''] → failure [syntax.incomplete-term@[25,25] '']; exact diagnostics retained, no normalization. |
| `corpus/camxes/11689.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11690.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11697.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/11714.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11717.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11729.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11733.toml` | Excluded actual-old-tree shape (linkargs,relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11743.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1176.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/11769.toml` | Failure-frontier/status residue manually repinned: failure [syntax.incomplete-selbri@[24,24] ''] → failure [syntax.unexpected-end@[24,24] '']; exact diagnostics retained, no normalization. |
| `corpus/camxes/1177.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/11775.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11794.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11795.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11797.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/11804.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11811.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11816.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11820.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11824.toml` | Excluded actual-old-tree shape (forethought,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11826.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11828.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11831.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11838.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11840.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11850.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11853.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11855.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11856.toml` | Failure-frontier/status residue manually repinned: failure [syntax.incomplete-selbri@[37,37] ''] → failure [syntax.incomplete-term@[37,37] '']; exact diagnostics retained, no normalization. |
| `corpus/camxes/11860.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11861.toml` | Excluded actual-old-tree shape (KE,linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11869.toml` | Excluded actual-old-tree shape (linkargs,relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11874.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11883.toml` | Excluded actual-old-tree shape (CO); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11893.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/11900.toml` | Excluded actual-old-tree shape (linkargs,relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11901.toml` | Excluded actual-old-tree shape (linkargs,relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11914.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11921.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11929.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11939.toml` | Excluded actual-old-tree shape (KE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11941.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11943.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11949.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1195.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11950.toml` | Excluded actual-old-tree shape (KE,linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11953.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11967.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1197.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11970.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11971.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11974.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11983.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11984.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11985.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11993.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/11996.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12000.toml` | Excluded actual-old-tree shape (NAhE,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12036.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12058.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12060.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12062.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12085.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12089.toml` | Excluded actual-old-tree shape (linkargs,relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12093.toml` | Excluded actual-old-tree shape (linkargs,relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12095.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12137.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12141.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12142.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12157.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12161.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12171.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12172.toml` | Excluded actual-old-tree shape (KE,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12173.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/12192.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12195.toml` | Excluded actual-old-tree shape (NAhE,linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1223.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/1224.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/12241.toml` | Failure-frontier/status residue manually repinned: failure [syntax.incomplete-selbri@[20,20] ''] → failure [syntax.incomplete-term@[20,20] '']; exact diagnostics retained, no normalization. |
| `corpus/camxes/12248.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1225.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/12258.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1226.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/12263.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12264.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1227.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12271.toml` | Excluded actual-old-tree shape (linkargs,relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1228.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1229.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12291.toml` | Excluded actual-old-tree shape (KE,NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1230.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/12308.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1231.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/12310.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12311.toml` | Excluded actual-old-tree shape (KE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12313.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12314.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12315.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1232.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/1233.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/12332.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12338.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/1234.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12345.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1236.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/12360.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12368.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12369.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/1237.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/12374.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1238.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/12380.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12383.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12384.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1239.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/12397.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1240.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/12400.toml` | Excluded actual-old-tree shape (NAhE,linkargs,relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12402.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12404.toml` | Excluded actual-old-tree shape (KE,NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1241.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/12414.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/12415.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1242.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/12423.toml` | Excluded actual-old-tree shape (CO); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1243.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/12437.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1244.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/12449.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1245.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/1246.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/12460.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12469.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1247.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/1248.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12480.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12486.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12487.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/1249.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12490.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12494.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1250.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12504.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1251.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12510.toml` | Excluded actual-old-tree shape (NAhE,relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1252.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12520.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12523.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1253.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12535.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12539.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1254.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12540.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12545.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1255.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/12553.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1256.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/12567.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1257.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/12575.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12576.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1258.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12587.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12589.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/1259.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12590.toml` | Excluded actual-old-tree shape (NAhE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12591.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12592.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12598.toml` | Excluded actual-old-tree shape (CO,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/126.toml` | Failure-frontier/status residue manually repinned: failure [syntax.incomplete-selbri@[32,32] ''] → failure [syntax.unexpected-end@[32,32] '']; exact diagnostics retained, no normalization. |
| `corpus/camxes/1260.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12608.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12609.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1261.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12614.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/12618.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/12619.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12622.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12632.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12634.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12637.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12649.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/12659.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12669.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12686.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12708.toml` | Excluded actual-old-tree shape (linkargs,relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12723.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12727.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12732.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12745.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12746.toml` | Excluded actual-old-tree shape (CO,relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12750.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12773.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12775.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/12784.toml` | Excluded actual-old-tree shape (linkargs,relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12801.toml` | Excluded actual-old-tree shape (CO,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12807.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12808.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12809.toml` | Excluded actual-old-tree shape (linkargs,relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12822.toml` | Excluded actual-old-tree shape (forethought,linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12834.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12840.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12842.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12844.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12848.toml` | Failure-frontier/status residue manually repinned: failure [syntax.incomplete-selbri@[30,30] ''] → failure [syntax.incomplete-term@[30,30] '']; exact diagnostics retained, no normalization. |
| `corpus/camxes/12858.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12914.toml` | Excluded actual-old-tree shape (CO); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12925.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12928.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12929.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12937.toml` | Excluded actual-old-tree shape (NAhE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1294.toml` | Excluded actual-old-tree shape (forethought,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12948.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12963.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/12966.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12981.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12982.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12984.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/12985.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13006.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13008.toml` | Excluded actual-old-tree shape (KE,NAhE,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13019.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13021.toml` | Excluded actual-old-tree shape (KE,relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13027.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13029.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13030.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13031.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13034.toml` | Excluded actual-old-tree shape (forethought,linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13038.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13046.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13047.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13054.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13055.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13069.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13075.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1309.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13093.toml` | Excluded actual-old-tree shape (NAhE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13094.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13097.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13114.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/13122.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13123.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13137.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13146.toml` | Excluded actual-old-tree shape (CO); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13165.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13177.toml` | Failure-frontier/status residue manually repinned: failure [syntax.incomplete-selbri@[126,126] ''] → failure [syntax.incomplete-term@[126,126] '']; exact diagnostics retained, no normalization. |
| `corpus/camxes/13181.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13193.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13207.toml` | Excluded actual-old-tree shape (CO,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13211.toml` | Excluded actual-old-tree shape (CO); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13213.toml` | Excluded actual-old-tree shape (CO); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13216.toml` | Excluded actual-old-tree shape (CO,linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13227.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1323.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13259.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13279.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13280.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13309.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13321.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13334.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13343.toml` | Excluded actual-old-tree shape (CO,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13357.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13365.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13377.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/13406.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13412.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13420.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13425.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13442.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13445.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13450.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13451.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13454.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13456.toml` | Excluded actual-old-tree shape (NAhE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13459.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13462.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13477.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1349.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13490.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13495.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13516.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13526.toml` | Failure-frontier/status residue manually repinned: failure [syntax.incomplete-selbri@[54,54] ''] → failure [syntax.incomplete-sumti@[54,54] '']; exact diagnostics retained, no normalization. |
| `corpus/camxes/13531.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13546.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/13551.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/13568.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/13569.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13577.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13590.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/13623.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13624.toml` | Excluded actual-old-tree shape (NAhE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13627.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13628.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13640.toml` | Excluded actual-old-tree shape (CO); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13641.toml` | Excluded actual-old-tree shape (NAhE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13645.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13646.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13649.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13669.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13671.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13677.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13680.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13685.toml` | Excluded actual-old-tree shape (NAhE,relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/137.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13711.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/1372.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13721.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13725.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1373.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13732.toml` | Excluded actual-old-tree shape (NAhE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13746.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13751.toml` | Excluded actual-old-tree shape (CO,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13759.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1376.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13774.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13777.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1378.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13786.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13789.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1379.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13825.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13832.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13840.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13851.toml` | Failure-frontier/status residue manually repinned: failure [syntax.incomplete-selbri@[47,47] ''] → failure [syntax.incomplete-term@[47,47] '']; exact diagnostics retained, no normalization. |
| `corpus/camxes/13868.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13873.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13879.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13899.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13902.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13927.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/13963.toml` | Excluded actual-old-tree shape (CO); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/14.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/14002.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/14010.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/14011.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/14012.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/14042.toml` | Excluded actual-old-tree shape (CEI); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/14045.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/14046.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/14049.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/14078.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/14082.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/14083.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/14089.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/14102.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/14124.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/14142.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/14146.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1415.toml` | Excluded actual-old-tree shape (KE,linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/14152.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/14166.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/14167.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/14170.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/14171.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1418.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1419.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/14193.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1420.toml` | Excluded actual-old-tree shape (CO,linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/14203.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1421.toml` | Excluded actual-old-tree shape (CEI); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/14210.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/14212.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/14213.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1422.toml` | Excluded actual-old-tree shape (CO); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/14226.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/14227.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/14253.toml` | Excluded actual-old-tree shape (KE,NAhE); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/14258.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/14272.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/14277.toml` | Excluded actual-old-tree shape (CO,linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1430.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/14314.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/14320.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/14323.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/14327.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/14329.toml` | Excluded actual-old-tree shape (CO); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/14353.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/14358.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/14360.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/14361.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/14394.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/14395.toml` | Failure-frontier/status residue manually repinned: failure [syntax.incomplete-selbri@[50,50] ''] → failure [syntax.unexpected-end@[50,50] '']; exact diagnostics retained, no normalization. |
| `corpus/camxes/14399.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/14425.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/14429.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/14435.toml` | Excluded actual-old-tree shape (linkargs,relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/14453.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/14458.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/14468.toml` | Excluded actual-old-tree shape (CO,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1447.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/14476.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/14500.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/14509.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/14510.toml` | Excluded actual-old-tree shape (CO,relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/14512.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/14520.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/14530.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/14532.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/14537.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/14541.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/14544.toml` | Excluded actual-old-tree shape (CO); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/14553.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/14557.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/14563.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/14576.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/14586.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/14603.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/14649.toml` | Excluded actual-old-tree shape (CO,NAhE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1465.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/14650.toml` | Excluded actual-old-tree shape (NAhE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/14673.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/14691.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/14695.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/14723.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/14744.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/14771.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/14772.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/14784.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/14790.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/14816.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/14823.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/14824.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1483.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/14832.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1484.toml` | Excluded actual-old-tree shape (KE,NAhE,linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/14842.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/14846.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/14847.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1485.toml` | Excluded actual-old-tree shape (KE,NAhE,linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/14853.toml` | Excluded actual-old-tree shape (NAhE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/14855.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1486.toml` | Excluded actual-old-tree shape (KE,NAhE,linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/14863.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/14864.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1487.toml` | Excluded actual-old-tree shape (KE,NAhE,linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/14876.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1488.toml` | Excluded actual-old-tree shape (KE,NAhE,linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1489.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1490.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1492.toml` | Excluded actual-old-tree shape (NAhE,linkargs,tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/14920.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/14921.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/14923.toml` | Failure-frontier/status residue manually repinned: failure [syntax.incomplete-selbri@[25,25] ''] → failure [syntax.unexpected-end@[25,25] '']; exact diagnostics retained, no normalization. |
| `corpus/camxes/1493.toml` | Excluded actual-old-tree shape (NAhE,linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/14939.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/14948.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/14951.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/15029.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/15064.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/15067.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/15068.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/15070.toml` | Excluded actual-old-tree shape (CO,relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/15073.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/15101.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/15104.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/15109.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/15122.toml` | Failure-frontier/status residue manually repinned: failure [syntax.incomplete-selbri@[22,22] ''] → failure [syntax.unexpected-end@[22,22] '']; exact diagnostics retained, no normalization. |
| `corpus/camxes/15140.toml` | Excluded actual-old-tree shape (CO); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/15149.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/15163.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/15168.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/15169.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/15191.toml` | Excluded actual-old-tree shape (CO,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1521.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/15223.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/15224.toml` | Failure-frontier/status residue manually repinned: failure [syntax.incomplete-selbri@[54,54] ''] → failure [syntax.incomplete-term@[54,54] '']; exact diagnostics retained, no normalization. |
| `corpus/camxes/15229.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/15232.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/15240.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1525.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/15261.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/15262.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/15280.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/15317.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/15323.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/15325.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/15338.toml` | Excluded actual-old-tree shape (NAhE,relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/15342.toml` | Excluded actual-old-tree shape (CO,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/15348.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/15359.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/15367.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/15373.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/15378.toml` | Excluded actual-old-tree shape (NAhE,linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/15379.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/15386.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/15392.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/15409.toml` | Failure-frontier/status residue manually repinned: failure [syntax.incomplete-selbri@[61,61] ''] → failure [syntax.unexpected-end@[61,61] '']; exact diagnostics retained, no normalization. |
| `corpus/camxes/15421.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/15441.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/15446.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/15456.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/15465.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/15467.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/15468.toml` | Excluded actual-old-tree shape (CO); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/15471.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/15479.toml` | Failure-frontier/status residue manually repinned: failure [syntax.incomplete-selbri@[75,75] ''] → failure [syntax.incomplete-term@[75,75] '']; exact diagnostics retained, no normalization. |
| `corpus/camxes/15484.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/15492.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/15498.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/15509.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1551.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/15512.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/15514.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/15525.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/15532.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/15539.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/15544.toml` | Failure-frontier/status residue manually repinned: failure [syntax.incomplete-selbri@[27,27] ''] → failure [syntax.incomplete-term@[27,27] '']; exact diagnostics retained, no normalization. |
| `corpus/camxes/15547.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/15563.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/15564.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/15566.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/15574.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/15577.toml` | Failure-frontier/status residue manually repinned: failure [syntax.incomplete-selbri@[131,131] ''] → failure [syntax.incomplete-term@[131,131] '']; exact diagnostics retained, no normalization. |
| `corpus/camxes/15583.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/15584.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/15586.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/15587.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/15611.toml` | Excluded actual-old-tree shape (NAhE,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/15613.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/15616.toml` | Failure-frontier/status residue manually repinned: failure [syntax.incomplete-selbri@[45,45] ''] → failure [syntax.incomplete-term@[45,45] '']; exact diagnostics retained, no normalization. |
| `corpus/camxes/15628.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/15647.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/15654.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/15665.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/15668.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/15671.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/15672.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/15697.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/157.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/15717.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/15729.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/15730.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/15739.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/15754.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/15767.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/15771.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/15797.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/15803.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/15805.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/15819.toml` | Excluded actual-old-tree shape (CO,NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/15848.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/15855.toml` | Excluded actual-old-tree shape (linkargs,relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1587.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1588.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/15899.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1592.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/15929.toml` | Excluded actual-old-tree shape (CO,KE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/15930.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/15953.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/15971.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/15976.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/15983.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/15989.toml` | Excluded actual-old-tree shape (linkargs,relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16012.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16027.toml` | Failure-frontier/status residue manually repinned: failure [syntax.incomplete-selbri@[26,26] ''] → failure [syntax.unexpected-end@[26,26] '']; exact diagnostics retained, no normalization. |
| `corpus/camxes/16036.toml` | Excluded actual-old-tree shape (CO,NAhE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16037.toml` | Excluded actual-old-tree shape (NAhE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16053.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16057.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16059.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/16074.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/16077.toml` | Failure-frontier/status residue manually repinned: failure [syntax.incomplete-selbri@[84,84] ''] → failure [syntax.incomplete-term@[84,84] '']; exact diagnostics retained, no normalization. |
| `corpus/camxes/1608.toml` | Excluded actual-old-tree shape (CO); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16089.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/16093.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/16112.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16125.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16126.toml` | Excluded actual-old-tree shape (CO,relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16137.toml` | Excluded actual-old-tree shape (NAhE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16154.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16160.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16163.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16165.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16169.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16172.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16178.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16181.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16188.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16192.toml` | Excluded actual-old-tree shape (CO,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16193.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16194.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16196.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16197.toml` | Excluded actual-old-tree shape (NAhE,linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16200.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16202.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16205.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16206.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16208.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16211.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/16212.toml` | Excluded actual-old-tree shape (KE,relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16215.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/16220.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16223.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16229.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16230.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16237.toml` | Excluded actual-old-tree shape (NAhE,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16244.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16246.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16250.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16255.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16261.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16262.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16266.toml` | Excluded actual-old-tree shape (forethought,linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16274.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16276.toml` | Excluded actual-old-tree shape (forethought,relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16277.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16278.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1628.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16281.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16282.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16286.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16293.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16298.toml` | Excluded actual-old-tree shape (CO); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16300.toml` | Excluded actual-old-tree shape (CO); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16323.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16324.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16325.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16326.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1634.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16345.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16354.toml` | Excluded actual-old-tree shape (NAhE,linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16376.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16385.toml` | Excluded actual-old-tree shape (KE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16391.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16399.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1641.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/16418.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16420.toml` | Excluded actual-old-tree shape (KE,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16424.toml` | Failure-frontier/status residue manually repinned: failure [morphology.warning.experimental-cgv@[31,34] 'nio', syntax.incomplete-selbri@[34,34] ''] → failure [morphology.warning.experimental-cgv@[31,34] 'nio', syntax.unexpected-end@[34,34] '']; exact diagnostics retained, no normalization. |
| `corpus/camxes/16431.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16446.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16481.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16488.toml` | Failure-frontier/status residue manually repinned: failure [syntax.incomplete-selbri@[19,19] ''] → failure [syntax.incomplete-term@[19,19] '']; exact diagnostics retained, no normalization. |
| `corpus/camxes/16495.toml` | Excluded actual-old-tree shape (CO); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1650.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16504.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16507.toml` | Excluded actual-old-tree shape (KE,NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16511.toml` | Failure-frontier/status residue manually repinned: failure [syntax.incomplete-term@[30,30] ''] → failure [syntax.incomplete-sumti@[30,30] '']; exact diagnostics retained, no normalization. |
| `corpus/camxes/16526.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16539.toml` | Excluded actual-old-tree shape (NAhE,linkargs); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/16541.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16557.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1656.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1657.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16574.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16586.toml` | Failure-frontier/status residue manually repinned: failure [syntax.incomplete-selbri@[46,46] ''] → failure [syntax.incomplete-term@[46,46] '']; exact diagnostics retained, no normalization. |
| `corpus/camxes/16590.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1660.toml` | Excluded actual-old-tree shape (NAhE,linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16606.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16611.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16630.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16632.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/16639.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16640.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16646.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16648.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16663.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16664.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16675.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16688.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16698.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16710.toml` | Excluded actual-old-tree shape (CO,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16712.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16716.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16718.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16720.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16722.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16724.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16729.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1673.toml` | Failure-frontier/status residue manually repinned: failure [syntax.incomplete-selbri@[69,69] ''] → failure [syntax.incomplete-term@[69,69] '']; exact diagnostics retained, no normalization. |
| `corpus/camxes/16756.toml` | Excluded actual-old-tree shape (KE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16761.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16781.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16794.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/16803.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/16812.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16829.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/1683.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1684.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16859.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16862.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16895.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16911.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16915.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16917.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16922.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16927.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/16956.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/17008.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/17022.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/17026.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/17039.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/17049.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/17069.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/17096.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/17104.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/17120.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/17121.toml` | Excluded actual-old-tree shape (NAhE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/17141.toml` | Manual materialization/removal residue: expectation leaves added: expectations.output.gentufa.json, expectations.output.gentufa.tree, expectations.syntax.raw; exact generated syntax/Gentufa leaves and token/span projections pinned. |
| `corpus/camxes/17170.toml` | Failure-frontier/status residue manually repinned: failure [syntax.incomplete-selbri@[17,17] ''] → failure [syntax.unexpected-end@[17,17] '']; exact diagnostics retained, no normalization. |
| `corpus/camxes/17173.toml` | Failure-frontier/status residue manually repinned: failure [syntax.incomplete-selbri@[47,47] ''] → failure [syntax.unexpected-end@[47,47] '']; exact diagnostics retained, no normalization. |
| `corpus/camxes/17197.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/17218.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/17224.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/17233.toml` | Failure-frontier/status residue manually repinned: failure [syntax.incomplete-selbri@[29,29] ''] → failure [syntax.unexpected-end@[29,29] '']; exact diagnostics retained, no normalization. |
| `corpus/camxes/17272.toml` | Excluded actual-old-tree shape (KE,tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/17282.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/17329.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/17341.toml` | Excluded actual-old-tree shape (NAhE,relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/17372.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/17373.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/17381.toml` | Excluded actual-old-tree shape (NAhE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/17382.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/17386.toml` | Excluded actual-old-tree shape (NAhE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/17394.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/17399.toml` | Excluded actual-old-tree shape (NAhE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/17406.toml` | Excluded actual-old-tree shape (NAhE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/17407.toml` | Excluded actual-old-tree shape (NAhE,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/17419.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/17433.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/17458.toml` | Excluded actual-old-tree shape (CO); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/17461.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/17464.toml` | Excluded actual-old-tree shape (CO,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/17466.toml` | Excluded actual-old-tree shape (CO,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/17483.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/17486.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/17493.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/17502.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/17512.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1754.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/1755.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/17554.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/17557.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1756.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/17574.toml` | Excluded actual-old-tree shape (KE,NAhE); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/17575.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/17576.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/17586.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/17599.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/17600.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/17653.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/17665.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/17699.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/17702.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/17704.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/17726.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/17737.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/17743.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/17747.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/17759.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/17773.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/17776.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/17780.toml` | Excluded actual-old-tree shape (CO,linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/17782.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/17792.toml` | Excluded actual-old-tree shape (CO,relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/17814.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/17830.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/17847.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/17854.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/17870.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/17880.toml` | Failure-frontier/status residue manually repinned: failure [syntax.incomplete-selbri@[37,37] ''] → failure [syntax.unexpected-end@[37,37] '']; exact diagnostics retained, no normalization. |
| `corpus/camxes/17887.toml` | Excluded actual-old-tree shape (KE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/17889.toml` | Failure-frontier/status residue manually repinned: failure [syntax.incomplete-selbri@[12,12] ''] → failure [syntax.incomplete-sumti@[12,12] '']; exact diagnostics retained, no normalization. |
| `corpus/camxes/17891.toml` | Excluded actual-old-tree shape (KE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/17905.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/17909.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/17936.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/17938.toml` | Excluded actual-old-tree shape (NAhE,linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18031.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/18066.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18072.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18146.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18148.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1816.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1817.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/18176.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18180.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18221.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18225.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18245.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18257.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/1826.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18263.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged,warning-gated); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18264.toml` | Excluded actual-old-tree shape (warning-gated); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18265.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18268.toml` | Excluded actual-old-tree shape (NAhE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1827.toml` | Excluded actual-old-tree shape (CO,KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18270.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1828.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18280.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1830.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18307.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18321.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/1833.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18332.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18338.toml` | Excluded actual-old-tree shape (NAhE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1834.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18344.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18349.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1835.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18353.toml` | Excluded actual-old-tree shape (CO); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1836.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18360.toml` | Excluded actual-old-tree shape (NAhE,relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18362.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18363.toml` | Failure-frontier/status residue manually repinned: failure [syntax.incomplete-selbri@[38,38] ''] → failure [syntax.incomplete-term@[38,38] '']; exact diagnostics retained, no normalization. |
| `corpus/camxes/18364.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18370.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18384.toml` | Excluded actual-old-tree shape (NAhE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1839.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18394.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18398.toml` | Excluded actual-old-tree shape (CO,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18403.toml` | Excluded actual-old-tree shape (CO,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18405.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/18406.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/18412.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18416.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/18417.toml` | Excluded actual-old-tree shape (linkargs,relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18426.toml` | Excluded actual-old-tree shape (forethought); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/1843.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18433.toml` | Excluded actual-old-tree shape (CO,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1844.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18440.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/1845.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18455.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1846.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18469.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1848.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/18486.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18487.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1849.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18491.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18492.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1850.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1851.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18514.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18515.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18518.toml` | Excluded actual-old-tree shape (CO,relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1852.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18524.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18525.toml` | Excluded actual-old-tree shape (NAhE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1853.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18530.toml` | Excluded actual-old-tree shape (CO,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18532.toml` | Excluded actual-old-tree shape (KE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18534.toml` | Excluded actual-old-tree shape (CO,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1854.toml` | Excluded actual-old-tree shape (CO); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18545.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18548.toml` | Excluded actual-old-tree shape (NAhE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1855.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18553.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18566.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18573.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18580.toml` | Excluded actual-old-tree shape (CO); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1861.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18619.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1862.toml` | Excluded actual-old-tree shape (CO); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18620.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18630.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18633.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/18635.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18637.toml` | Excluded actual-old-tree shape (CO); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18642.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18667.toml` | Excluded actual-old-tree shape (KE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18693.toml` | Excluded actual-old-tree shape (CEI); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18703.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18704.toml` | Excluded actual-old-tree shape (NAhE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18707.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18709.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/18711.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18713.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18715.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1872.toml` | Excluded actual-old-tree shape (CO); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18721.toml` | Excluded actual-old-tree shape (forethought,linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18722.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18729.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18733.toml` | Excluded actual-old-tree shape (linkargs,relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18740.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18742.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1875.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/1876.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/18764.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1877.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/18777.toml` | Excluded actual-old-tree shape (linkargs,relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18790.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1880.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18800.toml` | Excluded actual-old-tree shape (KE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18807.toml` | Excluded actual-old-tree shape (linkargs,relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18811.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18812.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18813.toml` | Excluded actual-old-tree shape (linkargs,relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18815.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18834.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18855.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18856.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18861.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1888.toml` | Excluded actual-old-tree shape (CO); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1889.toml` | Excluded actual-old-tree shape (CO); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18901.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18920.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18922.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18927.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/18940.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18942.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18952.toml` | Excluded actual-old-tree shape (NAhE,relative); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/18965.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/18967.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18968.toml` | Excluded actual-old-tree shape (CO); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18971.toml` | Excluded actual-old-tree shape (CO,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1898.toml` | Excluded actual-old-tree shape (CEI); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/18980.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18981.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/18986.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18988.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18993.toml` | Excluded actual-old-tree shape (CO); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18994.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/18996.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/19001.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/19005.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/19006.toml` | Excluded actual-old-tree shape (NAhE,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/19007.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1901.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/19010.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/19013.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/19018.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1902.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/19020.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/19023.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1903.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/19033.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/19041.toml` | Excluded actual-old-tree shape (CO); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/19049.toml` | Excluded actual-old-tree shape (CO,NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/19051.toml` | Excluded actual-old-tree shape (CEI); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/19071.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/19073.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/19093.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/19132.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/19160.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/19171.toml` | Excluded actual-old-tree shape (linkargs,relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/19179.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1918.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/19185.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/19195.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/19227.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/19233.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/19236.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/19258.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/19292.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/19293.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/19296.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/19313.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/19316.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1932.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/1933.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/19337.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/19341.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/19342.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/19343.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/19348.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/19352.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/19406.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/19410.toml` | Failure-frontier/status residue manually repinned: failure [syntax.incomplete-term@[46,46] ''] → failure [syntax.incomplete-sumti@[46,46] '']; exact diagnostics retained, no normalization. |
| `corpus/camxes/19441.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/19445.toml` | Excluded actual-old-tree shape (NAhE,linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/19447.toml` | Excluded actual-old-tree shape (NAhE,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/19448.toml` | Excluded actual-old-tree shape (NAhE,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/19449.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/19451.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/19463.toml` | Failure-frontier/status residue manually repinned: failure [syntax.incomplete-selbri@[19,19] ''] → failure [syntax.unexpected-end@[19,19] '']; exact diagnostics retained, no normalization. |
| `corpus/camxes/1948.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/19484.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/19505.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/19518.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/19520.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/19533.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/19545.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/19571.toml` | Excluded actual-old-tree shape (NAhE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/19572.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/19592.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/19593.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/19616.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/19657.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1968.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/19689.toml` | Excluded actual-old-tree shape (CO); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/19716.toml` | Excluded actual-old-tree shape (KE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/19733.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/19740.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/19749.toml` | Excluded actual-old-tree shape (warning-gated); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/19767.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/19783.toml` | Excluded actual-old-tree shape (NAhE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1979.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/19790.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1980.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1981.toml` | Excluded actual-old-tree shape (KE,NAhE,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/19810.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1982.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/19828.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/19829.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1983.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1984.toml` | Excluded actual-old-tree shape (NAhE,forethought,linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/19842.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/19846.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1985.toml` | Excluded actual-old-tree shape (NAhE,linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/19859.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1986.toml` | Excluded actual-old-tree shape (forethought,linkargs,tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/19868.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1987.toml` | Excluded actual-old-tree shape (KE,linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/19870.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/19871.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/19872.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/19873.toml` | Excluded actual-old-tree shape (KE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/19877.toml` | Excluded actual-old-tree shape (CO,NAhE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/19878.toml` | Excluded actual-old-tree shape (CO,NAhE,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1988.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/19882.toml` | Excluded actual-old-tree shape (NAhE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/19883.toml` | Excluded actual-old-tree shape (CO,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/19887.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1989.toml` | Excluded actual-old-tree shape (KE,relative,tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/1991.toml` | Excluded actual-old-tree shape (KE,forethought,linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/19910.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1992.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1993.toml` | Excluded actual-old-tree shape (CO,linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/19939.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1994.toml` | Excluded actual-old-tree shape (KE,linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/19940.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/1995.toml` | Excluded actual-old-tree shape (KE,linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/19964.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/19965.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/19968.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/1998.toml` | Excluded actual-old-tree shape (KE,linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/19980.toml` | Excluded actual-old-tree shape (CO); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/1999.toml` | Manual materialization/removal residue: expectation leaves added: expectations.output.gentufa.json, expectations.output.gentufa.tree, expectations.syntax.raw; exact generated syntax/Gentufa leaves and token/span projections pinned. |
| `corpus/camxes/19996.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/2001.toml` | Excluded actual-old-tree shape (NAhE,linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/20026.toml` | Excluded actual-old-tree shape (NAhE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2003.toml` | Excluded actual-old-tree shape (NAhE,tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/2004.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/20040.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/20045.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2005.toml` | Excluded actual-old-tree shape (KE,linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/20057.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/20059.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/20061.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2007.toml` | Excluded actual-old-tree shape (KE,linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/20070.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/20071.toml` | Excluded actual-old-tree shape (NAhE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/20076.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/20078.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2008.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/20083.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2009.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/20095.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/20096.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2010.toml` | Excluded actual-old-tree shape (KE,relative,tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/20103.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2011.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/20118.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2014.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/20141.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2015.toml` | Excluded actual-old-tree shape (KE,forethought,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/20168.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2017.toml` | Excluded actual-old-tree shape (KE,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2018.toml` | Excluded actual-old-tree shape (KE,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/20192.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/20195.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2020.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/20207.toml` | Excluded actual-old-tree shape (warning-gated); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/20217.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/20222.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/2024.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/20243.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/20245.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/20246.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/20247.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2025.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2026.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/20262.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/20264.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2028.toml` | Excluded actual-old-tree shape (forethought,tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/2029.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/2030.toml` | Excluded actual-old-tree shape (CO,NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/20308.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2031.toml` | Excluded actual-old-tree shape (CO,NAhE,linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2032.toml` | Excluded actual-old-tree shape (CO,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/20323.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2033.toml` | Excluded actual-old-tree shape (CO,linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/20330.toml` | Excluded actual-old-tree shape (NAhE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2035.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/20365.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2037.toml` | Excluded actual-old-tree shape (CO); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2038.toml` | Excluded actual-old-tree shape (KE,linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/20393.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/20397.toml` | Excluded actual-old-tree shape (NAhE,linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2040.toml` | Excluded actual-old-tree shape (CO,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2041.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/20417.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2042.toml` | Excluded actual-old-tree shape (CO,linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2043.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2044.toml` | Excluded actual-old-tree shape (CO,linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/20442.toml` | Excluded actual-old-tree shape (CO); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2045.toml` | Excluded actual-old-tree shape (CO,linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/20459.toml` | Excluded actual-old-tree shape (NAhE,linkargs,relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2046.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2047.toml` | Excluded actual-old-tree shape (CO,NAhE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2048.toml` | Excluded actual-old-tree shape (CO,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/20482.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2049.toml` | Excluded actual-old-tree shape (NAhE,linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/20492.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/20493.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2050.toml` | Excluded actual-old-tree shape (CO,linkargs,tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/20500.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/20501.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/20507.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2052.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/2053.toml` | Excluded actual-old-tree shape (CO,linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2055.toml` | Excluded actual-old-tree shape (CO,linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/20576.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2058.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/20585.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2059.toml` | Excluded actual-old-tree shape (CO,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2060.toml` | Excluded actual-old-tree shape (CO,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/20605.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2061.toml` | Excluded actual-old-tree shape (KE,linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/20613.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/20614.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/20617.toml` | Excluded actual-old-tree shape (linkargs,warning-gated); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2062.toml` | Excluded actual-old-tree shape (KE,linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/20628.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2063.toml` | Excluded actual-old-tree shape (linkargs,relative); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/20631.toml` | Failure-frontier/status residue manually repinned: failure [syntax.incomplete-selbri@[62,62] ''] → failure [syntax.incomplete-term@[62,62] '']; exact diagnostics retained, no normalization. |
| `corpus/camxes/20643.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/20645.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/2065.toml` | Excluded actual-old-tree shape (CO,linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2067.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2068.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/20682.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/20684.toml` | Excluded actual-old-tree shape (CO); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/20686.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2069.toml` | Excluded actual-old-tree shape (CO,KE,forethought); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/20701.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/20704.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2071.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/20711.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/20718.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/20729.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/20739.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2075.toml` | Excluded actual-old-tree shape (CO,linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/20750.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/20756.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2076.toml` | Excluded actual-old-tree shape (CO,NAhE,linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2077.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2079.toml` | Excluded actual-old-tree shape (forethought,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2080.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/20807.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/2081.toml` | Excluded actual-old-tree shape (CO,relative,tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/2082.toml` | Excluded actual-old-tree shape (KE,linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2083.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/20834.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2084.toml` | Excluded actual-old-tree shape (KE,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/20845.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2085.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/20852.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/20853.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2086.toml` | Excluded actual-old-tree shape (KE,linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/20887.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/20899.toml` | Excluded actual-old-tree shape (CO,NAhE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2090.toml` | Excluded actual-old-tree shape (KE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/20908.toml` | Excluded actual-old-tree shape (CO,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2091.toml` | Excluded actual-old-tree shape (forethought,tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/20915.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/20928.toml` | Excluded actual-old-tree shape (CO,NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/20929.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/20938.toml` | Excluded actual-old-tree shape (linkargs,relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/20956.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/20959.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2096.toml` | Excluded actual-old-tree shape (forethought,linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/20963.toml` | Excluded actual-old-tree shape (CO); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/20967.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2097.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/20973.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/20980.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/20985.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2099.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/20998.toml` | Excluded actual-old-tree shape (CO); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/21003.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/21010.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/21021.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/21027.toml` | Excluded actual-old-tree shape (CO,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/21036.toml` | Excluded actual-old-tree shape (KE,linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/21049.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/21053.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/21055.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/21058.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2106.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/21060.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/21063.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2108.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/21088.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/21089.toml` | Excluded actual-old-tree shape (CO,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/21093.toml` | Excluded actual-old-tree shape (CO,forethought,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/21095.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/21096.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/21097.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2110.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/21102.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/21127.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2113.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/21133.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/21134.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2114.toml` | Excluded actual-old-tree shape (KE,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/21140.toml` | Excluded actual-old-tree shape (CO,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/21147.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/21152.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/21155.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/2116.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/21160.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2117.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/21174.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/21175.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/21179.toml` | Failure-frontier/status residue manually repinned: failure [syntax.incomplete-selbri@[63,63] ''] → failure [syntax.incomplete-term@[63,63] '']; exact diagnostics retained, no normalization. |
| `corpus/camxes/2118.toml` | Excluded actual-old-tree shape (KE,relative,tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/2119.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/21195.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2120.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/21202.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/21203.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2121.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2122.toml` | Excluded actual-old-tree shape (KE,relative,tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/21227.toml` | Excluded actual-old-tree shape (CO); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2123.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/21234.toml` | Failure-frontier/status residue manually repinned: failure [syntax.incomplete-selbri@[37,37] ''] → failure [syntax.unexpected-end@[37,37] '']; exact diagnostics retained, no normalization. |
| `corpus/camxes/21239.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2124.toml` | Excluded actual-old-tree shape (forethought,linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/21243.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/21246.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/21268.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2127.toml` | Excluded actual-old-tree shape (NAhE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2131.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/21328.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2133.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/21331.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/21335.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/21348.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/21351.toml` | Failure-frontier/status residue manually repinned: failure [syntax.incomplete-selbri@[33,33] ''] → failure [syntax.unexpected-end@[33,33] '']; exact diagnostics retained, no normalization. |
| `corpus/camxes/21356.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2137.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/21371.toml` | Excluded actual-old-tree shape (KE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/21377.toml` | Excluded actual-old-tree shape (NAhE,linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/21387.toml` | Excluded actual-old-tree shape (forethought,linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/21389.toml` | Excluded actual-old-tree shape (forethought,linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2139.toml` | Excluded actual-old-tree shape (NAhE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/21390.toml` | Excluded actual-old-tree shape (forethought,linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2140.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/21414.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/21415.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/21424.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/21427.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2143.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/21438.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2144.toml` | Excluded actual-old-tree shape (forethought,linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2145.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/21465.toml` | Excluded actual-old-tree shape (NAhE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/21478.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2150.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/21510.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/21514.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2152.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/21520.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/21522.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/21527.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2154.toml` | Excluded actual-old-tree shape (CO,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2155.toml` | Excluded actual-old-tree shape (KE,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/21555.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/21557.toml` | Excluded actual-old-tree shape (NAhE,relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/21558.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2156.toml` | Excluded actual-old-tree shape (CO,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/21569.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/21597.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2161.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/21631.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/21633.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2164.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/21644.toml` | Excluded actual-old-tree shape (linkargs,relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/21668.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/21719.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2172.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/21722.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/21723.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/21724.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2173.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/21731.toml` | Failure-frontier/status residue manually repinned: failure [syntax.incomplete-selbri@[41,41] ''] → failure [syntax.incomplete-term@[41,41] '']; exact diagnostics retained, no normalization. |
| `corpus/camxes/21752.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2176.toml` | Excluded actual-old-tree shape (CO,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2177.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/2178.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/21784.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2179.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2180.toml` | Excluded actual-old-tree shape (KE,linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2181.toml` | Excluded actual-old-tree shape (NAhE,linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/21824.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/21828.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/21834.toml` | Excluded actual-old-tree shape (KE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/21843.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2185.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/21856.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2186.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/21865.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2187.toml` | Failure-frontier/status residue manually repinned: failure [syntax.incomplete-selbri@[118,118] ''] → failure [syntax.incomplete-bridi@[118,118] '']; exact diagnostics retained, no normalization. |
| `corpus/camxes/2188.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2189.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2190.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2191.toml` | Excluded actual-old-tree shape (KE,linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/21913.toml` | Excluded actual-old-tree shape (CO,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/21921.toml` | Failure-frontier/status residue manually repinned: failure [syntax.unexpected-cmavo@[30,32] 'la'] → failure [syntax.unexpected-cmavo@[27,29] 'fi']; exact diagnostics retained, no normalization. |
| `corpus/camxes/2193.toml` | Excluded actual-old-tree shape (CO); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2194.toml` | Excluded actual-old-tree shape (CO,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2195.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/21953.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/21954.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/21957.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/21958.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2196.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/21963.toml` | Excluded actual-old-tree shape (KE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/21966.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/21972.toml` | Excluded actual-old-tree shape (NAhE,relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/21975.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2198.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/21981.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/21984.toml` | Excluded actual-old-tree shape (linkargs,relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2199.toml` | Excluded actual-old-tree shape (NAhE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2200.toml` | Excluded actual-old-tree shape (NAhE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/22018.toml` | Excluded actual-old-tree shape (tagged,warning-gated); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/22029.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/22031.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/22033.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/22034.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/22035.toml` | Excluded actual-old-tree shape (NAhE,linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2204.toml` | Excluded actual-old-tree shape (CO,NAhE,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/22040.toml` | Excluded actual-old-tree shape (NAhE,forethought); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2206.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/22061.toml` | Excluded actual-old-tree shape (NAhE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/22066.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/22068.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2207.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/22070.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/22087.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/22088.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2209.toml` | Excluded actual-old-tree shape (forethought,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/22097.toml` | Excluded actual-old-tree shape (CO); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/22106.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/22107.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2211.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/22113.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/22128.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2213.toml` | Excluded actual-old-tree shape (CO,KE,forethought,linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/22149.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2215.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/22158.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/22172.toml` | Failure-frontier/status residue manually repinned: failure [syntax.incomplete-selbri@[67,67] ''] → failure [syntax.incomplete-term@[67,67] '']; exact diagnostics retained, no normalization. |
| `corpus/camxes/22180.toml` | Excluded actual-old-tree shape (linkargs,relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/22190.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/22197.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/22199.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2220.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/22207.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/2222.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/22220.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/22236.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/22254.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/22268.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2227.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/22273.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2228.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2230.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/22310.toml` | Failure-frontier/status residue manually repinned: failure [syntax.incomplete-selbri@[94,94] ''] → failure [syntax.incomplete-forethought-connection@[94,94] '']; exact diagnostics retained, no normalization. |
| `corpus/camxes/2232.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/22332.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/22335.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2234.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2236.toml` | Excluded actual-old-tree shape (CO,NAhE,linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/2237.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2239.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/22391.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/22395.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/22398.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/22399.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/22406.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2241.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/22418.toml` | Excluded actual-old-tree shape (CO,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/22419.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/22431.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/22432.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/22454.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2247.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/22475.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/22487.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2250.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2251.toml` | Excluded actual-old-tree shape (KE,linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2252.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/22533.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/22538.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/2254.toml` | Excluded actual-old-tree shape (forethought,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/22542.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/22547.toml` | Excluded actual-old-tree shape (NAhE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/22555.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/22556.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/22558.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2256.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/22564.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2257.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2258.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2266.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2267.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2279.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2282.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2284.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2288.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/2290.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2291.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2294.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2295.toml` | Excluded actual-old-tree shape (CO); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2296.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2299.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2302.toml` | Excluded actual-old-tree shape (CO,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2304.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2305.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2307.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2311.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2313.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2315.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2319.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2321.toml` | Excluded actual-old-tree shape (CO,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2323.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2326.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2330.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2331.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2332.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2337.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2342.toml` | Excluded actual-old-tree shape (KE,linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2346.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2351.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2353.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2358.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/236.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2360.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2364.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2365.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2366.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2369.toml` | Excluded actual-old-tree shape (KE,linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2370.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2375.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/2376.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2379.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2381.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2383.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2384.toml` | Excluded actual-old-tree shape (KE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2386.toml` | Excluded actual-old-tree shape (CO,linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2388.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2389.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2393.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2398.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/240.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2401.toml` | Excluded actual-old-tree shape (KE,linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2402.toml` | Excluded actual-old-tree shape (KE,linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2411.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2413.toml` | Excluded actual-old-tree shape (CO,KE,relative,tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/2416.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/242.toml` | Excluded actual-old-tree shape (CO); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2421.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2422.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2423.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2426.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2428.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2429.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2430.toml` | Excluded actual-old-tree shape (KE,linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/2434.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2436.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2437.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2439.toml` | Excluded actual-old-tree shape (CO,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2443.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2444.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/2447.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2448.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2449.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2450.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2451.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2459.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2462.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/2465.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2466.toml` | Excluded actual-old-tree shape (forethought,linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2467.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2468.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2470.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2471.toml` | Excluded actual-old-tree shape (forethought,linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2473.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2481.toml` | Excluded actual-old-tree shape (KE,forethought,linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2482.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2484.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2485.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2487.toml` | Excluded actual-old-tree shape (NAhE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2488.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2493.toml` | Excluded actual-old-tree shape (KE,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2495.toml` | Excluded actual-old-tree shape (NAhE,linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2497.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2498.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2499.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2500.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2501.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2502.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2503.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2504.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2505.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2508.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2509.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2510.toml` | Excluded actual-old-tree shape (NAhE,forethought,linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2513.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2516.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2517.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2518.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2522.toml` | Excluded actual-old-tree shape (KE,relative,tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/2523.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/2524.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2525.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2526.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2528.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/2529.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/2530.toml` | Excluded actual-old-tree shape (forethought,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2536.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2543.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2544.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2547.toml` | Excluded actual-old-tree shape (linkargs,relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2550.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2552.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2558.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2559.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2560.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2563.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2565.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2567.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2568.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2576.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2578.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2583.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/2587.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2589.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/2592.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2596.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2598.toml` | Excluded actual-old-tree shape (KE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2607.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2608.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2611.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2613.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2617.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2618.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2619.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2622.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/2623.toml` | Excluded actual-old-tree shape (CO,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2624.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2627.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2628.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2630.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2631.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2632.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2633.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2635.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2636.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2637.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2642.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2646.toml` | Excluded actual-old-tree shape (forethought,linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/2647.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2648.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2651.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2652.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2653.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2654.toml` | Excluded actual-old-tree shape (KE,linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2658.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2661.toml` | Excluded actual-old-tree shape (forethought,relative,tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/2663.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2666.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/2667.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2669.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2671.toml` | Manual materialization/removal residue: expectation leaves added: expectations.output.gentufa.json, expectations.output.gentufa.tree, expectations.syntax.raw; exact generated syntax/Gentufa leaves and token/span projections pinned. |
| `corpus/camxes/2673.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/2674.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2675.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2680.toml` | Excluded actual-old-tree shape (KE,linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2681.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2682.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/2686.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2691.toml` | Excluded actual-old-tree shape (KE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2692.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/2696.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/2698.toml` | Excluded actual-old-tree shape (KE,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/270.toml` | Excluded actual-old-tree shape (CO); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2701.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2706.toml` | Excluded actual-old-tree shape (linkargs,relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2708.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2714.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2716.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2718.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2720.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/2721.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2722.toml` | Excluded actual-old-tree shape (forethought,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2725.toml` | Excluded actual-old-tree shape (NAhE,forethought,linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2726.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2730.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2731.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2732.toml` | Excluded actual-old-tree shape (CO,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2741.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2742.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2744.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2751.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2753.toml` | Excluded actual-old-tree shape (CO,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2759.toml` | Excluded actual-old-tree shape (CO); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2765.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2767.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2770.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2773.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2776.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2778.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/2779.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2782.toml` | Excluded actual-old-tree shape (NAhE,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2787.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2788.toml` | Excluded actual-old-tree shape (KE,linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2789.toml` | Excluded actual-old-tree shape (KE,linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2790.toml` | Excluded actual-old-tree shape (KE,relative,tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/2802.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2811.toml` | Failure-frontier/status residue manually repinned: failure [syntax.unexpected-cmavo@[17,19] 'mo'] → failure [syntax.unexpected-cmavo@[14,16] 'fo']; exact diagnostics retained, no normalization. |
| `corpus/camxes/2840.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2862.toml` | Excluded actual-old-tree shape (KE,relative,tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/2901.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2910.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2966.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/2998.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3005.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/301.toml` | Excluded actual-old-tree shape (CO); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3014.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3027.toml` | Failure-frontier/status residue manually repinned: failure [syntax.incomplete-selbri@[31,31] ''] → failure [syntax.unexpected-end@[31,31] '']; exact diagnostics retained, no normalization. |
| `corpus/camxes/3032.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3034.toml` | Failure-frontier/status residue manually repinned: failure [syntax.incomplete-selbri@[17,17] ''] → failure [syntax.unexpected-end@[17,17] '']; exact diagnostics retained, no normalization. |
| `corpus/camxes/3037.toml` | Failure-frontier/status residue manually repinned: failure [morphology.warning.experimental-cgv@[5,8] 'dua', syntax.incomplete-selbri@[34,34] ''] → failure [morphology.warning.experimental-cgv@[5,8] 'dua', syntax.unexpected-end@[34,34] '']; exact diagnostics retained, no normalization. |
| `corpus/camxes/3044.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3049.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3071.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3076.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3089.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3093.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3099.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3100.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3101.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3106.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3107.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3109.toml` | Excluded actual-old-tree shape (NAhE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3111.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3116.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3137.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/3139.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/3156.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3185.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3194.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3198.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3218.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3234.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3242.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3252.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3265.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3266.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3270.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3289.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3290.toml` | Excluded actual-old-tree shape (NAhE,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3313.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3317.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3319.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3326.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3331.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3336.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3343.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/3352.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3356.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3359.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3363.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3365.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3366.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3367.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3384.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3386.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3388.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3395.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3398.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3399.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/34.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3404.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3407.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3417.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3421.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3422.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3425.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3426.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3432.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3444.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3462.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3463.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/347.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3472.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3484.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3500.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3505.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3529.toml` | Failure-frontier/status residue manually repinned: failure [syntax.incomplete-selbri@[39,39] ''] → failure [syntax.incomplete-term@[39,39] '']; exact diagnostics retained, no normalization. |
| `corpus/camxes/3542.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3580.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3622.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/3626.toml` | Excluded actual-old-tree shape (KE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3656.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3669.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3689.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3692.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3720.toml` | Manual materialization/removal residue: expectation leaves added: expectations.output.gentufa.json, expectations.output.gentufa.tree, expectations.syntax.raw; exact generated syntax/Gentufa leaves and token/span projections pinned. |
| `corpus/camxes/3742.toml` | Failure-frontier/status residue manually repinned: failure [syntax.incomplete-selbri@[42,42] ''] → failure [syntax.unexpected-end@[42,42] '']; exact diagnostics retained, no normalization. |
| `corpus/camxes/3744.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3755.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3767.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3788.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3825.toml` | Failure-frontier/status residue manually repinned: failure [syntax.incomplete-selbri@[39,39] ''] → failure [syntax.incomplete-term@[39,39] '']; exact diagnostics retained, no normalization. |
| `corpus/camxes/3860.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3872.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3873.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3875.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3919.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3927.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3940.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3942.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/395.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3951.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/3956.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3959.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3974.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3978.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/3986.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/40.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/4014.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/4029.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/4048.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/4050.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/4063.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/4064.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/4066.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/4069.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/4075.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/4094.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/4103.toml` | Excluded actual-old-tree shape (CO,linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/4136.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/4148.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/4149.toml` | Failure-frontier/status residue manually repinned: failure [syntax.incomplete-selbri@[50,50] ''] → failure [syntax.incomplete-term@[50,50] '']; exact diagnostics retained, no normalization. |
| `corpus/camxes/415.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/4155.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/4156.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/4159.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/4164.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/4165.toml` | Excluded actual-old-tree shape (NAhE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/4167.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/4200.toml` | Excluded actual-old-tree shape (linkargs,relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/4204.toml` | Failure-frontier/status residue manually repinned: failure [syntax.incomplete-selbri@[52,52] ''] → failure [syntax.incomplete-term@[52,52] '']; exact diagnostics retained, no normalization. |
| `corpus/camxes/4221.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/4224.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/4226.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/4256.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/4258.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/4259.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/4265.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/4267.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/4268.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/4270.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/4271.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/4274.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/4289.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/429.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/4295.toml` | Failure-frontier/status residue manually repinned: failure [syntax.incomplete-selbri@[29,29] ''] → failure [syntax.unexpected-end@[29,29] '']; exact diagnostics retained, no normalization. |
| `corpus/camxes/43.toml` | Excluded actual-old-tree shape (NAhE,linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/4302.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/4303.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/4306.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/4314.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/4319.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/4336.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/4339.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/4356.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/4378.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/4384.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/4405.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/4414.toml` | Excluded actual-old-tree shape (NAhE,linkargs,relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/4415.toml` | Excluded actual-old-tree shape (NAhE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/4446.toml` | Failure-frontier/status residue manually repinned: failure [syntax.incomplete-selbri@[57,57] ''] → failure [syntax.unexpected-end@[57,57] '']; exact diagnostics retained, no normalization. |
| `corpus/camxes/4449.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/4452.toml` | Excluded actual-old-tree shape (KE,NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/4462.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/4477.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/4518.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/4541.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/4543.toml` | Excluded actual-old-tree shape (KE,NAhE); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/4544.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/4551.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/4552.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/4558.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/4575.toml` | Excluded actual-old-tree shape (NAhE,relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/4576.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/4579.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/4590.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/4598.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/46.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/460.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/4604.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/461.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/462.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/463.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/464.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/4654.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/4673.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/4679.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/4684.toml` | Excluded actual-old-tree shape (linkargs,relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/4686.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/4687.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/47.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/470.toml` | Failure-frontier/status residue manually repinned: failure [syntax.unexpected-cmavo@[130,132] 'le'] → failure [syntax.unexpected-cmavo@[127,129] 'fi']; exact diagnostics retained, no normalization. |
| `corpus/camxes/4703.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/4704.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/4705.toml` | Excluded actual-old-tree shape (CO,NAhE,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/4707.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/4718.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/4723.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/4724.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/4733.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/4747.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/475.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/4761.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/4784.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/4788.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/4792.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/4796.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/481.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/4829.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/483.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/4830.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/484.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/485.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/4854.toml` | Excluded actual-old-tree shape (NAhE,relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/4856.toml` | Excluded actual-old-tree shape (KE,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/4859.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/486.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/4860.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/4865.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/487.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/4872.toml` | Excluded actual-old-tree shape (NAhE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/4873.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/4874.toml` | Failure-frontier/status residue manually repinned: failure [syntax.incomplete-selbri@[36,36] ''] → failure [syntax.incomplete-term@[36,36] '']; exact diagnostics retained, no normalization. |
| `corpus/camxes/4879.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/4884.toml` | Failure-frontier/status residue manually repinned: failure [syntax.incomplete-selbri@[20,20] ''] → failure [syntax.unexpected-end@[20,20] '']; exact diagnostics retained, no normalization. |
| `corpus/camxes/4893.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/4895.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/4929.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/4943.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/4944.toml` | Failure-frontier/status residue manually repinned: failure [syntax.incomplete-selbri@[21,21] ''] → failure [syntax.unexpected-end@[21,21] '']; exact diagnostics retained, no normalization. |
| `corpus/camxes/495.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/5005.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/5018.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/5019.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/502.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/5034.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/5049.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/5054.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/5084.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/5109.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/5110.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/5136.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/5137.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/5171.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/5172.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/5174.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/5176.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/5179.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/5201.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/521.toml` | Excluded actual-old-tree shape (CO); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/522.toml` | Excluded actual-old-tree shape (CO); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/5222.toml` | Excluded actual-old-tree shape (NAhE,relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/5241.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/5250.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/5252.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/5253.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/5257.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/5281.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/5286.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/5289.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/5292.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/5296.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/5301.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/5306.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/5313.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/5318.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/5321.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/5324.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/5325.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/5328.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/5335.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/5341.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/5342.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/5345.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/5349.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/5350.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/5355.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/5356.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/5358.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/5362.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/5403.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/5411.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/5425.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/5434.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/5451.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/5453.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/5454.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/5466.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/5467.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/5479.toml` | Excluded actual-old-tree shape (CO); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/5486.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/5502.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/5505.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/5520.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/5521.toml` | Standard acceptance flip manually reviewed: success → failure at LIhU; camxes-std running parser also rejects the surface. Exact diagnostics [syntax.unexpected-cmavo@[52,56] "li'u"] pinned; stale syntax tree, semantics refs, and Gentufa projections removed. |
| `corpus/camxes/5534.toml` | Failure-frontier/status residue manually repinned: failure [syntax.incomplete-selbri@[49,49] ''] → failure [syntax.incomplete-term@[49,49] '']; exact diagnostics retained, no normalization. |
| `corpus/camxes/5538.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/5539.toml` | Failure-frontier/status residue manually repinned: failure [syntax.incomplete-selbri@[23,23] ''] → failure [syntax.unexpected-end@[23,23] '']; exact diagnostics retained, no normalization. |
| `corpus/camxes/5548.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/5551.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/5554.toml` | Excluded actual-old-tree shape (KE,tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/5557.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/5559.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/5560.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/5561.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/5563.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/5564.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/5567.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/5569.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/5570.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/5571.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/5585.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/5590.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/5598.toml` | Failure-frontier/status residue manually repinned: failure [syntax.incomplete-selbri@[45,45] ''] → failure [syntax.unexpected-end@[45,45] '']; exact diagnostics retained, no normalization. |
| `corpus/camxes/5599.toml` | Failure-frontier/status residue manually repinned: failure [syntax.incomplete-selbri@[49,49] ''] → failure [syntax.unexpected-end@[49,49] '']; exact diagnostics retained, no normalization. |
| `corpus/camxes/5602.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/5604.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/5613.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/5618.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/5620.toml` | Manual materialization/removal residue: expectation leaves added: expectations.output.gentufa.json, expectations.output.gentufa.tree, expectations.syntax.raw; exact generated syntax/Gentufa leaves and token/span projections pinned. |
| `corpus/camxes/5624.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/5625.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/5630.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/5633.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/5645.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/5675.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/5679.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/5686.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/5687.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/5688.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/5700.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/5741.toml` | Excluded actual-old-tree shape (CO); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/577.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/5776.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/5783.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/5785.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/5792.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/5824.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/5849.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/5856.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/5864.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/5889.toml` | Excluded actual-old-tree shape (forethought); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/5909.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/5916.toml` | Failure-frontier/status residue manually repinned: failure [syntax.incomplete-selbri@[15,15] ''] → failure [syntax.incomplete-sumti@[15,15] '']; exact diagnostics retained, no normalization. |
| `corpus/camxes/5917.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/5918.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/5920.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/5922.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/593.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/5945.toml` | Failure-frontier/status residue manually repinned: failure [syntax.incomplete-selbri@[33,33] ''] → failure [syntax.incomplete-term@[33,33] '']; exact diagnostics retained, no normalization. |
| `corpus/camxes/5949.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/5955.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/5965.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/5967.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/5975.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/5994.toml` | Excluded actual-old-tree shape (CO,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/5997.toml` | Excluded actual-old-tree shape (NAhE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/6006.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/6036.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/6039.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/6051.toml` | Failure-frontier/status residue manually repinned: failure [syntax.incomplete-selbri@[33,33] ''] → failure [syntax.unexpected-end@[33,33] '']; exact diagnostics retained, no normalization. |
| `corpus/camxes/6057.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/6062.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/6072.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/6074.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/6091.toml` | Excluded actual-old-tree shape (KE,NAhE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/6092.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/6094.toml` | Excluded actual-old-tree shape (NAhE,forethought,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/6095.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/6097.toml` | Excluded actual-old-tree shape (KE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/6102.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/6112.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/6125.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/6134.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/6145.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/6149.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/6157.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/6158.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/6165.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/620.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/6227.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/6231.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/6237.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/6251.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/6254.toml` | Excluded actual-old-tree shape (forethought,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/6257.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/6268.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/6269.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/6271.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/6280.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/6284.toml` | Failure-frontier/status residue manually repinned: failure [syntax.incomplete-selbri@[72,72] ''] → failure [syntax.incomplete-term@[72,72] '']; exact diagnostics retained, no normalization. |
| `corpus/camxes/6290.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/6293.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/6297.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/6309.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/6311.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/6318.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/6322.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/6326.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/6328.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/6334.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/6335.toml` | Failure-frontier/status residue manually repinned: failure [syntax.incomplete-selbri@[28,28] ''] → failure [syntax.unexpected-end@[28,28] '']; exact diagnostics retained, no normalization. |
| `corpus/camxes/6338.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/6355.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/6361.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/6377.toml` | Excluded actual-old-tree shape (NAhE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/6433.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/6440.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/6472.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/6476.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/6480.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/6492.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/6495.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/6520.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/6531.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/6540.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/6560.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/6569.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/6570.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/6574.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/6579.toml` | Excluded actual-old-tree shape (linkargs,relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/6583.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/6591.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/6594.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/660.toml` | Excluded actual-old-tree shape (forethought); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/6604.toml` | Manual materialization/removal residue: expectation leaves added: expectations.output.gentufa.json, expectations.output.gentufa.tree, expectations.syntax.raw; exact generated syntax/Gentufa leaves and token/span projections pinned. |
| `corpus/camxes/6605.toml` | Excluded actual-old-tree shape (NAhE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/6606.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/6607.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/661.toml` | Excluded actual-old-tree shape (forethought); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/6628.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/664.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/6646.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/6653.toml` | Failure-frontier/status residue manually repinned: failure [syntax.incomplete-selbri@[18,18] ''] → failure [syntax.unexpected-end@[18,18] '']; exact diagnostics retained, no normalization. |
| `corpus/camxes/6679.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/6680.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/6696.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/6698.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/6711.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/6712.toml` | Excluded actual-old-tree shape (NAhE,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/6715.toml` | Excluded actual-old-tree shape (linkargs,relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/6728.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/674.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/675.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/676.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/677.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/678.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/679.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/6791.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/6822.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/6823.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/6829.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/6832.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/6841.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/6845.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/6850.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/6909.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/6915.toml` | Failure-frontier/status residue manually repinned: failure [syntax.unexpected-cmavo@[21,23] 'mi'] → failure [syntax.unexpected-cmavo@[18,20] 'fi']; exact diagnostics retained, no normalization. |
| `corpus/camxes/6924.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/6925.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/6928.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/6931.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/6943.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/6945.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/6955.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/6968.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/6973.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/6976.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/6978.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/7031.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/7035.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/7056.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/7066.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/7081.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/7088.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/7092.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/7109.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/7120.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/713.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/716.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/717.toml` | Excluded actual-old-tree shape (KE,NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/7183.toml` | Excluded actual-old-tree shape (KE,tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/719.toml` | Excluded actual-old-tree shape (forethought); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/720.toml` | Excluded actual-old-tree shape (forethought); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/7202.toml` | Failure-frontier/status residue manually repinned: failure [syntax.incomplete-selbri@[25,25] ''] → failure [syntax.unexpected-end@[25,25] '']; exact diagnostics retained, no normalization. |
| `corpus/camxes/7215.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/7216.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/7227.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/7231.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/7253.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/7288.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/7298.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/7360.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/738.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/7413.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/7437.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/744.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/7441.toml` | Excluded actual-old-tree shape (NAhE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/7443.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/7467.toml` | Excluded actual-old-tree shape (CO,relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/7469.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/7489.toml` | Failure-frontier/status residue manually repinned: failure [syntax.incomplete-selbri@[34,34] ''] → failure [syntax.unexpected-end@[34,34] '']; exact diagnostics retained, no normalization. |
| `corpus/camxes/7502.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/7508.toml` | Failure-frontier/status residue manually repinned: failure [syntax.incomplete-selbri@[44,44] ''] → failure [syntax.incomplete-term@[44,44] '']; exact diagnostics retained, no normalization. |
| `corpus/camxes/7510.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/7524.toml` | Excluded actual-old-tree shape (KE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/7527.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/7528.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/7538.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/7539.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/7542.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/7547.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/7555.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/7564.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/7565.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/7567.toml` | Excluded actual-old-tree shape (linkargs,relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/7588.toml` | Excluded actual-old-tree shape (CO); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/7590.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/760.toml` | Excluded actual-old-tree shape (KE,NAhE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/762.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/7624.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/7626.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/763.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/7634.toml` | Excluded actual-old-tree shape (CO,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/7638.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/7639.toml` | Excluded actual-old-tree shape (CO,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/7641.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/7652.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/7653.toml` | Excluded actual-old-tree shape (KE,relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/7658.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/7659.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/7665.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/7681.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/7724.toml` | Excluded actual-old-tree shape (KE,linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/7729.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/7733.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/7748.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/7755.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/7766.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/7783.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/7789.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/7796.toml` | Excluded actual-old-tree shape (KE,NAhE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/7803.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/7814.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/783.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/7835.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/7853.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/7855.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/7882.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/7896.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/79.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/7922.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/7938.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/7941.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/7945.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/7951.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/7980.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/7990.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8006.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/802.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8023.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/803.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8031.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8034.toml` | Excluded actual-old-tree shape (NAhE,linkargs,tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/8038.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/804.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8056.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8060.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8067.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8074.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8075.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8076.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/8077.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8081.toml` | Excluded actual-old-tree shape (CO); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8089.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8090.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8091.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8092.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/81.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8102.toml` | Excluded actual-old-tree shape (KE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8107.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/8110.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8111.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8112.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8113.toml` | Excluded actual-old-tree shape (CO,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8115.toml` | Excluded actual-old-tree shape (NAhE,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8118.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8136.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8151.toml` | Excluded actual-old-tree shape (CO); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8169.toml` | Excluded actual-old-tree shape (linkargs,relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8174.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8188.toml` | Excluded actual-old-tree shape (CO); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8190.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8201.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8206.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8208.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8210.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/822.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8276.toml` | Excluded actual-old-tree shape (CO); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8281.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8305.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8307.toml` | Excluded actual-old-tree shape (CEI); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8322.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8337.toml` | Excluded actual-old-tree shape (NAhE,relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8345.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8349.toml` | Excluded actual-old-tree shape (forethought,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8353.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8356.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8357.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8358.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8360.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/8364.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/8366.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8372.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8373.toml` | Excluded actual-old-tree shape (NAhE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8374.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/8379.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/8381.toml` | Excluded actual-old-tree shape (NAhE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8385.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/841.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8423.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8424.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8430.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8438.toml` | Excluded actual-old-tree shape (KE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8439.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8440.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8442.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8443.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/846.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8460.toml` | Failure-frontier/status residue manually repinned: failure [syntax.incomplete-selbri@[37,37] ''] → failure [syntax.incomplete-term@[37,37] '']; exact diagnostics retained, no normalization. |
| `corpus/camxes/8463.toml` | Failure-frontier/status residue manually repinned: failure [syntax.incomplete-selbri@[18,18] ''] → failure [syntax.incomplete-term@[18,18] '']; exact diagnostics retained, no normalization. |
| `corpus/camxes/8467.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/847.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8476.toml` | Failure-frontier/status residue manually repinned: failure [syntax.incomplete-selbri@[18,18] ''] → failure [syntax.incomplete-term@[18,18] '']; exact diagnostics retained, no normalization. |
| `corpus/camxes/8482.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/8487.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8489.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8513.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8523.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8529.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8530.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8532.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8533.toml` | Excluded actual-old-tree shape (KE,linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8534.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8536.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8537.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/8541.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8545.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8559.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/8562.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8565.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8566.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/8569.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8572.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8573.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8580.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8585.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8595.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8613.toml` | Excluded actual-old-tree shape (forethought,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8633.toml` | Excluded actual-old-tree shape (CEI); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8634.toml` | Excluded actual-old-tree shape (CEI); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8690.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8705.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/8727.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/873.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/8737.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8739.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8743.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8755.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/8764.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8775.toml` | Failure-frontier/status residue manually repinned: failure [syntax.incomplete-selbri@[45,45] ''] → failure [syntax.incomplete-term@[45,45] '']; exact diagnostics retained, no normalization. |
| `corpus/camxes/8778.toml` | Excluded actual-old-tree shape (CO,tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/8781.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8783.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8792.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8812.toml` | Excluded actual-old-tree shape (forethought,linkargs,relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8825.toml` | Excluded actual-old-tree shape (CO); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8830.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8841.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8852.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8858.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8859.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8866.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8882.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8888.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8891.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/890.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/891.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8914.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8915.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/893.toml` | Excluded actual-old-tree shape (KE,relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8950.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8972.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/8979.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/8999.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9016.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/9028.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9029.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9031.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9063.toml` | Failure-frontier/status residue manually repinned: failure [syntax.incomplete-selbri@[18,18] ''] → failure [syntax.unexpected-end@[18,18] '']; exact diagnostics retained, no normalization. |
| `corpus/camxes/9068.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9099.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9102.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9113.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9138.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9139.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9148.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9150.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9155.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9162.toml` | Excluded actual-old-tree shape (NAhE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9164.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9169.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/9184.toml` | Excluded actual-old-tree shape (KE,NAhE); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/9185.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9197.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/921.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/922.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9221.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/9223.toml` | Excluded actual-old-tree shape (NAhE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/923.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9270.toml` | Failure-frontier/status residue manually repinned: failure [syntax.incomplete-selbri@[20,20] ''] → failure [syntax.incomplete-statement@[20,20] '']; exact diagnostics retained, no normalization. |
| `corpus/camxes/9278.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9282.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9287.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9288.toml` | Excluded actual-old-tree shape (CO,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9290.toml` | Excluded actual-old-tree shape (CO,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9299.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/9300.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9304.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9310.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9311.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/9348.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9349.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9352.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9353.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9354.toml` | Excluded actual-old-tree shape (NAhE,tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/9357.toml` | Excluded actual-old-tree shape (NAhE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9358.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9359.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9368.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9369.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/9378.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9415.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9425.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/9427.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9441.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9444.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/9453.toml` | Excluded actual-old-tree shape (KE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9460.toml` | Excluded actual-old-tree shape (KE,tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/9464.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9468.toml` | Excluded actual-old-tree shape (KE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9471.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9473.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9481.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9492.toml` | Excluded actual-old-tree shape (CO); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9494.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9507.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9508.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9514.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/9517.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9518.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9523.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9532.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9533.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9534.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9537.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/9551.toml` | Excluded actual-old-tree shape (NAhE,linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9555.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9579.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/9609.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9612.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9624.toml` | Failure-frontier/status residue manually repinned: failure [syntax.incomplete-selbri@[38,38] ''] → failure [syntax.incomplete-term@[38,38] '']; exact diagnostics retained, no normalization. |
| `corpus/camxes/9648.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9677.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/9710.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9719.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/9724.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/9740.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9748.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9772.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9774.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/9775.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9778.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9785.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9794.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9795.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9796.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9809.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9819.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/982.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9824.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9827.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/983.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9835.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9840.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9843.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9844.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9846.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/9854.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9855.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9873.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9875.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9878.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9879.toml` | Excluded actual-old-tree shape (KE,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9892.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9893.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/991.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9912.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9917.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9920.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/993.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9933.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9935.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/9939.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9940.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9952.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9957.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9958.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9959.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9964.toml` | Excluded actual-old-tree shape (mixed-or-non-simple); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/9972.toml` | Downstream semantic-projection residue manually repinned (expectations.semantics.refs.raw); exact refs/tersmu output retained, no normalization. |
| `corpus/camxes/9973.toml` | Excluded actual-old-tree shape (CO); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9974.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/9979.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `corpus/camxes/9989.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9991.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `corpus/camxes/9996.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1002-canonical.toml` | Excluded actual-old-tree shape (KE,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1003-canonical.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1003-front.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1007-front.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1011-canonical.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1011-front.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1013-front.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1014-front.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1016-front.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1028-canonical.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1034-front.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1037-canonical.toml` | Excluded actual-old-tree shape (KE,NAhE); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1037-front.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1042-canonical.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1043-canonical.toml` | Excluded actual-old-tree shape (KE,forethought); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1045-canonical.toml` | Excluded actual-old-tree shape (KE,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1051-canonical.toml` | Excluded actual-old-tree shape (KE,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1052-canonical.toml` | Excluded actual-old-tree shape (KE,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1053-canonical.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1054-canonical.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1056-front.toml` | Excluded actual-old-tree shape (KE,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1057-canonical.toml` | Excluded actual-old-tree shape (KE,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1057-front.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1058-front.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1061-canonical.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1066-front.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1068-canonical.toml` | Excluded actual-old-tree shape (KE,tagged); exact regenerated tree pinned manually, no normalization. Downstream semantic projection is also exactly repinned. |
| `muplis/collection-18/1072-canonical.toml` | Excluded actual-old-tree shape (KE,forethought,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1072-front.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1073-canonical.toml` | Excluded actual-old-tree shape (KE,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1088-front.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1089-canonical.toml` | Excluded actual-old-tree shape (KE,relative); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1100-canonical.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1101-canonical.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1101-front.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1102-front.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1103-canonical.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1115-canonical.toml` | Excluded actual-old-tree shape (CEI); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1117-canonical.toml` | Excluded actual-old-tree shape (CEI); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1134-front.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1142-canonical.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1152-canonical.toml` | Excluded actual-old-tree shape (forethought,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1152-front.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1154-canonical.toml` | Excluded actual-old-tree shape (forethought,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1154-front.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1155-canonical.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1164-canonical.toml` | Excluded actual-old-tree shape (CEI); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1169-canonical.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1169-front.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1170-canonical.toml` | Excluded actual-old-tree shape (CEI,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1174-canonical.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1174-front.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1177-canonical.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1182-canonical.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1182-front.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1184-canonical.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1184-front.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1191-canonical.toml` | Excluded actual-old-tree shape (forethought,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1191-front.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1201-canonical.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1205-front.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1208-canonical.toml` | Excluded actual-old-tree shape (CEI,forethought); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1220-front.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1224-canonical.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1225-canonical.toml` | Excluded actual-old-tree shape (CEI,forethought); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1226-canonical.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1227-canonical.toml` | Excluded actual-old-tree shape (KE,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1235-canonical.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1241-canonical.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1242-front.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1245-front.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1247-front.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1249-canonical.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1283-canonical.toml` | Excluded actual-old-tree shape (KE,forethought,relative); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1284-canonical.toml` | Excluded actual-old-tree shape (NAhE,forethought); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1284-front.toml` | Excluded actual-old-tree shape (NAhE,forethought); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1286-front.toml` | Excluded actual-old-tree shape (forethought,linkargs,relative); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1287-canonical.toml` | Excluded actual-old-tree shape (forethought); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1289-canonical.toml` | Excluded actual-old-tree shape (KE,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1289-front.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1292-canonical.toml` | Excluded actual-old-tree shape (KE,NAhE); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1292-front.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1296-canonical.toml` | Excluded actual-old-tree shape (forethought); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1298-canonical.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1298-front.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1300-canonical.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1300-front.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1301-canonical.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1301-front.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1305-front.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1306-front.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1319-canonical.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1330-front.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1332-canonical.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1332-front.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1333-front.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1338-canonical.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1339-canonical.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1344-canonical.toml` | Excluded actual-old-tree shape (KE,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1346-front.toml` | Excluded actual-old-tree shape (warning-gated); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1349-canonical.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1350-front.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1355-front.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1356-front.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1357-canonical.toml` | Excluded actual-old-tree shape (CEI,forethought,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1363-front.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1364-canonical.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1364-front.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1377-front.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1379-front.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1380-canonical.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1380-front.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1382-canonical.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1382-front.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1391-canonical.toml` | Excluded actual-old-tree shape (forethought,relative); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1392-front.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1394-canonical.toml` | Excluded actual-old-tree shape (KE,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1394-front.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1398-front.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1399-front.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1400-front.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1402-canonical.toml` | Excluded actual-old-tree shape (forethought); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1403-canonical.toml` | Excluded actual-old-tree shape (KE,linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1403-front.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1404-canonical.toml` | Excluded actual-old-tree shape (forethought,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1404-front.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1405-front.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1407-front.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1410-canonical.toml` | Excluded actual-old-tree shape (KE,linkargs,relative); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1410-front.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1413-front.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1415-canonical.toml` | Excluded actual-old-tree shape (KE,relative); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1417-canonical.toml` | Excluded actual-old-tree shape (CEI); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1420-canonical.toml` | Excluded actual-old-tree shape (forethought,relative); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1427-canonical.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1439-front.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1441-canonical.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1441-front.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1442-front.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1443-front.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1444-canonical.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1444-front.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1445-canonical.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1445-front.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1454-canonical.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1454-front.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1456-canonical.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1458-front.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1460-front.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1461-front.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1463-canonical.toml` | Excluded actual-old-tree shape (NAhE,forethought); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1463-front.toml` | Excluded actual-old-tree shape (NAhE,relative); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1464-front.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1466-front.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1469-canonical.toml` | Excluded actual-old-tree shape (NAhE,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1469-front.toml` | Excluded actual-old-tree shape (NAhE,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1479-canonical.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1487-canonical.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1490-canonical.toml` | Excluded actual-old-tree shape (NAhE,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1490-front.toml` | Excluded actual-old-tree shape (NAhE,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1492-canonical.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1492-front.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1498-front.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1499-front.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1503-canonical.toml` | Excluded actual-old-tree shape (KE,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1504-canonical.toml` | Excluded actual-old-tree shape (KE,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1510-canonical.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1510-front.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1511-canonical.toml` | Excluded actual-old-tree shape (forethought,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1511-front.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1512-canonical.toml` | Excluded actual-old-tree shape (KE,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1512-front.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1517-canonical.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1518-canonical.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1519-canonical.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1521-canonical.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1521-front.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1522-canonical.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1522-front.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1523-canonical.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1523-front.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1524-canonical.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1524-front.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1527-canonical.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1527-front.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1531-front.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1540-canonical.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1541-canonical.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1546-canonical.toml` | Excluded actual-old-tree shape (KE,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1554-front.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1579-canonical.toml` | Excluded actual-old-tree shape (KE,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1580-canonical.toml` | Excluded actual-old-tree shape (KE,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1582-front.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1590-canonical.toml` | Excluded actual-old-tree shape (KE,relative); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1594-front.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1596-canonical.toml` | Excluded actual-old-tree shape (KE,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1597-canonical.toml` | Excluded actual-old-tree shape (forethought,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1597-front.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1607-canonical.toml` | Excluded actual-old-tree shape (forethought); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1608-canonical.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1626-canonical.toml` | Excluded actual-old-tree shape (KE,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1627-front.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1628-front.toml` | Excluded actual-old-tree shape (relative); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1630-canonical.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1630-front.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1637-canonical.toml` | Excluded actual-old-tree shape (CEI,forethought); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1640-canonical.toml` | Excluded actual-old-tree shape (NAhE,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1640-front.toml` | Excluded actual-old-tree shape (NAhE,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1644-canonical.toml` | Excluded actual-old-tree shape (KE,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1645-canonical.toml` | Excluded actual-old-tree shape (forethought,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1647-canonical.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1647-front.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1650-canonical.toml` | Excluded actual-old-tree shape (CEI); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1654-canonical.toml` | Excluded actual-old-tree shape (KE,linkargs); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1654-front.toml` | Excluded actual-old-tree shape (CO,linkargs); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1655-front.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1656-canonical.toml` | Excluded actual-old-tree shape (KE,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1656-front.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1657-front.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1667-canonical.toml` | Excluded actual-old-tree shape (KE,forethought); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1672-canonical.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1672-front.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1676-canonical.toml` | Excluded actual-old-tree shape (NAhE,forethought); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1676-front.toml` | Excluded actual-old-tree shape (NAhE,relative); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1678-front.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1679-front.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1705-canonical.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1705-front.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1708-canonical.toml` | Excluded actual-old-tree shape (KE,forethought,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1711-front.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1712-front.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1737-canonical.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1737-front.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1738-canonical.toml` | Excluded actual-old-tree shape (forethought,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1739-canonical.toml` | Excluded actual-old-tree shape (CEI,forethought,relative); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1745-front.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1746-front.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1748-front.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1755-canonical.toml` | Excluded actual-old-tree shape (forethought,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1755-front.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1761-canonical.toml` | Excluded actual-old-tree shape (KE,forethought,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1763-canonical.toml` | Excluded actual-old-tree shape (KE,forethought,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1764-canonical.toml` | Excluded actual-old-tree shape (KE,forethought); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1769-canonical.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1770-canonical.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1770-front.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1771-front.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1772-front.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1773-canonical.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1773-front.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1774-canonical.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1774-front.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1775-canonical.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1775-front.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1784-canonical.toml` | Excluded actual-old-tree shape (forethought,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1785-front.toml` | Excluded actual-old-tree shape (linkargs,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1790-front.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1799-canonical.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1800-front.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1801-front.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1805-front.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1811-front.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1812-front.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1814-front.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1815-canonical.toml` | Excluded actual-old-tree shape (KE,relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1815-front.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1816-canonical.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1816-front.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1822-front.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1824-canonical.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1826-canonical.toml` | Excluded actual-old-tree shape (forethought,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1826-front.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1829-front.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1834-canonical.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1834-front.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1836-front.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1838-front.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1840-front.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1841-canonical.toml` | Excluded actual-old-tree shape (KE,forethought); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1844-front.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1848-front.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1849-canonical.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1849-front.toml` | Excluded actual-old-tree shape (NAhE); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/1852-front.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/866-canonical.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/866-front.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/868-canonical.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/868-front.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/871-canonical.toml` | Excluded actual-old-tree shape (KE,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/881-canonical.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/881-front.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/890-canonical.toml` | Excluded actual-old-tree shape (KE,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/907-front.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/908-canonical.toml` | Excluded actual-old-tree shape (KE,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/908-front.toml` | Excluded actual-old-tree shape (linkargs,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/912-canonical.toml` | Excluded actual-old-tree shape (forethought,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/912-front.toml` | Excluded actual-old-tree shape (relative,tagged); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/947-front.toml` | Excluded actual-old-tree shape (linkargs); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/953-canonical.toml` | Excluded actual-old-tree shape (KE); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/981-canonical.toml` | Excluded actual-old-tree shape (KE,forethought); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/999-canonical.toml` | Excluded actual-old-tree shape (forethought); exact regenerated tree pinned manually, no normalization. |
| `muplis/collection-18/999-front.toml` | Excluded actual-old-tree shape (tagged); exact regenerated tree pinned manually, no normalization. |

</details>

The complete machine-readable run is `/build/jbotci/logs/epoch05-c6-comparer-post-semantics-inventory.log`. The ledger table above is the durable per-fixture disposition; the `/build` log remains disposable run output.

### C6 consolidated non-fixture pins and final gates

The same reconstruction surface changed a small set of exact non-fixture
expectations. These are reviewed pins rather than comparer normalizations:

- CLI syntax diagnostics now name the reconstructed statement connection and
  its `NAhE` recovery tag;
- the Gentufa flat-source-order test expects the generated `selbri connection`
  block label;
- the IDE recovery snapshot admits the now-reachable trailing-operator mex
  quantifier case;
- recovered-output width budgets reflect the shorter reconstructed context;
- the frozen semantic divergence count adds `b39`, `b40`, and `nd1`, whose
  connective locus is now the predicate rather than its description wrapper;
- the issue-778 structural witness pins the reconstructed graph-owned `ce'u`
  parameter number; and
- the enum/struct placeholder audits individually disposition every new
  borrowed leaf, stateless rejection policy, visitor, inspector, and collector.

The final comparer run against the preserved C5 baseline is
`/build/jbotci/logs/epoch05-c6-comparer-final2.log`; it reproduces all pinned
counts above, and its five focused boundary tests pass. Final release-mode
verification is green: workspace tests, the 26,412-fixture all-profile run
(73,702 passed, 515 xfailed, zero failed), the 110 tagged syntax fixtures, and
the single all-targets expensive-contract run. Formatting, the four Python
generated checks, the debug `jbotci` build, and the debug Dioxus web build are
also green.

The stripped manylinux 2.28 aarch64 wheel contains 24 entries and measures
22,707,929 archive bytes / 98,781,023 unpacked bytes. Epoch 4 measured
22,106,247 / 96,208,471, so epoch-5 growth is 2.72% / 2.67%, within the required
5% audit band. The compressed artifact exceeded the former 22.2 MB policy
ceiling by 2.29%; `artifact-policy.toml` therefore rounds only the Linux
aarch64 compressed baseline from 18.5 MB to 19.0 MB, producing a 22.8 MB
ceiling. The unpacked and entry ratchets remain unchanged. The final policy
receipt is `/build/jbotci/logs/epoch05-artifact-ratchet-inspect-final.log`.

### Round 2: debug LSP latency guard and description CEI reach

| Surface or gate | Disposition and evidence |
|---|---|
| `error_heavy_completion_returns_before_followup_diagnostics` | CI measured base `9fafb66d4a` at approximately 2.8–2.9 s against the former 3 s wall-clock bound and epoch-5 `06437145a2` at 3.083 s. On the profiling host, base/current totals were 5.30/5.79 s (+9.2%, within the accepted 10% margin), while the current release path was 1.16 s. A bounded `perf` pass found no dominant epoch-5 selbri helper: the measured work is primarily the concurrent full-document recovery and decoration analysis launched by `didOpen`, with individual reconstructed-selbri functions below sampling significance. No speculative parser optimization was taken. The debug-only bound is therefore 6.5 s, which restores comparable headroom on the slower profiling host; response-shape and follow-up-diagnostics assertions remain exact. Profile: `/build/jbotci/logs/epoch05-round2-perf-report.log`. |
| `lo broda cei brode cei na brodi ku` under `(zantufa)` | K3's description-position extension-reach residual is now pinned. The restricted description selbri preserves the baseline-shaped first CEI prefix inside one nested Zantufa assignment whose later operand is `na brodi`; exact syntax tree, two CEI warning diagnostics, Gentufa tree, and JSON are recorded in `adhoc/syntax/selbri/issue-829-description-cei-extension-reach-zantufa.toml`. Running camxes-standard and camxes-exp reject at `ku`; running Zantufa 1.9999 accepts with the second CEI nested inside the first assignment operand (`/build/jbotci/logs/epoch05-round2-description-cei-reference.log`). |
