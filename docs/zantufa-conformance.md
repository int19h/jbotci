# Zantufa Parser Conformance Ledger

This ledger tracks v1 parser behavior against Guskant's frozen Zantufa fork.

- Grammar: https://raw.githubusercontent.com/guskant/gerna_cipra/master/zantufa-1.9999.peg
- Grammar SHA-256: `79e7a1daec2aaa9760af3c650c9e67c393c1c4c1b1e40e1ce905e0a9b652a312`
- Parser snapshot SHA-256: `127562f0f7b05bb805060ff9ba419b2cfadaa56e47b74aca30ab3bdb11787283`
- Checked locally on: 2026-07-01
- Captured fixture: [tests/fixtures/zantufa/upstream-parity.json](/home/int19h.linux/git/jbotci/tests/fixtures/zantufa/upstream-parity.json)

CLL forethought connectives are binary. Zantufa permits n-ary forethought
chains such as `ga P0 gi P1 gi ... gi Pn (gi'i)`. v1 represents those as
ordered branch vectors.

## Gate Policy

- `default-warning`: lexically or structurally distinguishable from baseline
  CLL+xorlo; parses without a dialect flag and emits an experimental warning.
- `feature-required`: changes behavior for a plausible baseline token stream;
  requires the specific Zantufa feature or `(zantufa)`.
- `compatibility-only`: public dialect/profile flag retained, but no syntax is
  hidden behind it.

There is no v1 public `ZantufaCmavo` feature. Fork-only cmavo are recognized by
default and warned as `ExperimentalZantufaCmavo`; surrounding experimental
constructs may emit their own construct-specific warning as well.

## Rule Ledger

| Upstream rule surface | Owner | Policy | Parser status |
| --- | --- | --- | --- |
| `gek_statement`, `sentence`, `gek_bridi_tail`, `sumti_3`, `tanru_unit_1`, `gek_term` n-ary `gik+` plus optional `GIhI` | `ZantufaConnectives` | `feature-required` | Accepted with ordered branch vectors and `GIhI` as `GIhI`, not baseline `GIhA` |
| Zantufa `gek` variants including `GI (joik/tag)`, `(joik/tag) GI`, and optional `BO` | `ZantufaConnectives`, `ZantufaTags` | `feature-required` | Accepted by the generated connective/tag grammar |
| `statement_terms <- statement IAU? terms?` | `ZantufaTerms` | mixed | Accepted under `ZantufaTerms`; `IhAU` preservation is dialect-sensitive so it is not swallowed as an indicator |
| `bridi_tail_3 <- KE !(selbri_2 KEhE) bridi_tail KEhE? tail_terms` | `ZantufaTerms` | `feature-required` | Accepted under `ZantufaTerms` as `ZantufaGroupedBridiTail`, now behind the completed-candidate guard `GroupedTanruKeTailRejection` that stands in for the source's `!(selbri_2 KEhE)` lookahead: a KE body that a baseline group could take over the identical extent goes back to the baseline group, so plain adjacency, a missing KEhE and a bare forethought body stay baseline and a CO-bearing body stays epoch 5's KE-CO tanru arm |
| `bridi_tail <- bridi_tail_1 (joik_gihek tag? CU_elidible bridi_tail_1)*` | `ZantufaConnectives` | `feature-required` | Accepted as `ZantufaContinuedBridiTail` behind an extension-first priority wrapper, with the completed-candidate classifier that returns any candidate whose every continuation is a GIhA carrying no tag to its baseline or adopted-camxes-exp owner |
| `joik_gihek <- joik / gihek` at every bridi-tail joint | `ZantufaConnectives` | `feature-required` | The shared `bridi_tail_connective` gains gated JOIK and JEK arms, which widens the flat joint and the BO joint together, including the `!(tag? BO)` and `!(tag? KE)` guards written over that node. The top KE join is deliberately NOT widened: rolling Zantufa spells no KE join at that level, its KE-led tail being a `bridi_tail_3` alternative |
| `bridi_tail_2 <- bridi_tail_3 (tag BO_clause CU_elidible bridi_tail_3 tail_terms)*`, connective ABSENT | `ZantufaConnectives` | `feature-required` | Accepted as the second arm of the new `bridi_tail_bo_joint` sum; the two arms are structurally disjoint, the connective being mandatory in the sourced one and absent here |
| `term_2 <- KE !(sumti KEhE) term+ KEhE?` | `ZantufaTerms` | `default-warning` | Accepted by default as KE term grouping |
| `gek_term <- gek term+ (gik term+)+ GIhI?` | `ZantufaTerms`, `ZantufaConnectives` | `feature-required` for extra branches and `GIhI` | Accepted through `forethought_termset` |
| `term_2 <- XOI statement SEhU?` and `FIhOI statement` | `ZantufaAdverbials` | `default-warning` | Accepted with full generated `statement` payloads |
| `brigahi <- POIhA free* selbri KU? / NA ... KU?` | `ZantufaTerms`, `ZantufaAdverbials` | `default-warning` where distinguishable | POIhA/NOIhA and NA briga'i forms are accepted by the generated term grammar |
| `tag_term` with `tag`, `FA (joik FA)*`, and `JAI tag?` | `ZantufaTags` | `feature-required` for conflicting tag behavior | Accepted under `ZantufaTags`, including the JOIK-chained place tag and the `(sumti / KU_elidible)` payload in both halves, with `!tanru_unit_1` asserted structurally so `jai broda` and `fa je fe broda` stay selbri |
| `term_1 <- term_2 (joik_ek? BO_clause term_2)*`, connective ABSENT | `ZantufaTerms` | `feature-required` | Accepted under `ZantufaTerms` as a continuation of the sourced BO connection, so one node may mix connectorless and connective-present joints as upstream does; the connective-present stag-less form stays the sourced camxes-exp arm's |
| `sumti_2 <- sumti_3 (joik_ek? tag? BO_clause sumti_3)*`, connective ABSENT | `ZantufaTerms` | `feature-required` | Accepted under `ZantufaTerms` at the BASELINE BO-precedence level, tag optional |
| `BO <- ce'e / bo` | — | documented gap | `ko'a ce'e ko'e broda` keeps the baseline CEhE termset group in every configuration, including `(zantufa)`; the Zantufa BO-connection reading is a fidelity-flag candidate, recorded and not minted |
| `tag <- tcita_selci+ (joik tcita_selci+)*`, recursive `tcita_selci` | `ZantufaTags` | `feature-required` | Accepted by v1 connected tag grammar plus Zantufa recursive prefix atoms |
| `relative_clause <- NOI statement KUhO?` | `ZantufaTerms` | `feature-required` | Accepted under `ZantufaTerms`; default keeps ordinary bridi relatives because an elided baseline `KUhO` can otherwise take the same prefix |
| `LOhOI (joik LOhOI)* statement KUhAU?` | `ZantufaQuotes` | `default-warning` | Accepted by default for bridi-description sumti |
| `RAhOI`, `MUhOI`, `GOhOI`, `LUhEI ... LIhAU?` quote surfaces | `ZantufaQuotes` | `default-warning` | Accepted by default |
| `NU (joik NU)* statement KEI?` | `ZantufaTerms` | `feature-required` | Accepted under `ZantufaTerms` before ordinary NU so baseline abstractions do not warn |
| `ME (sumti / operator+ / mex / tag) MEhU? MOI?`, `mex MOI` | `ZantufaMex` | mixed | Accepted; fork-only payloads warn as `ExperimentalZantufaMex` |
| `mex`, `mex_1`, `mex_2`, `mex_rp`, `mex_forethought` | `ZantufaMex` | mixed | Accepted for default-warning raw mex fragments and feature-required raw mex quantifiers, BO-grouped mex, KE-grouped operand sequences, reverse-Polish tails, operator-first mex, and optional trailing operator forms |
| `operator <- SE operator / NAhE operator / MAhO (mex/selbri/sumti) / VUhU / joik_ek !CU` | `ZantufaMex` | `feature-required` when it changes operator parsing | Accepted under `ZantufaMex`; connective operators are guarded from `CU` |
| `operand <- number / lerfu / VEI mex / MOhE (selbri/sumti) / LAhE|NAhE BO mex / NAhE operand` | `ZantufaMex` | `feature-required` where it conflicts | Accepted under `ZantufaMex`, including scalar `NAhE operand` and selbri `MOhE` |
| `free <- SEI statement / mex_2 MAI / xi_clause / ...` | `ZantufaTerms`, `ZantufaMex` | mixed | `SEI statement` is feature-required where it shadows ordinary SEI; `mex_2 MAI` and statement-term `XI` cases are feature-required when raw mex quantifier behavior would conflict with baseline quantification |
| Non-cmavo morphology differences | `ZantufaMorphology` | `compatibility-only` | No syntax is gated by this flag; concrete non-cmavo morphology work is tracked outside the syntax parser |

## Captured Expectations

The one-shot capture helper
[tests/support/capture-zantufa-expectations.sh](/home/int19h.linux/git/jbotci/tests/support/capture-zantufa-expectations.sh)
checks the reviewed fixture against Guskant's parser snapshot. Normal tests do
not invoke the upstream parser or use the network.

The fixture currently covers:

- default-warning bare mex fragments;
- feature-gated statement relative clauses;
- grouped bridi tails;
- n-ary forethought statement connectives with `gi'i`;
- statement abstractions;
- raw mex quantifiers with trailing operators and `MAI` free modifiers;
- feature-gated `XI` plus operator-first raw mex in a statement;
- the connectorless BO joints at the term and sumti tiers, including the
  tag-bearing one;
- the JAI term with an explicit KU, and the JAI selbri that the `!tanru_unit_1`
  guard keeps a selbri;
- the JOIK-chained place tag;
- `ce'e`, which upstream reads as a BO connection and v1 keeps as a CEhE termset
  group.

## Reference Analysis

This ledger covers parsing only. v1 no longer lowers any dialect to a semantic
model: the legacy semantic builder, its notation, and its
`Unsupported Zantufa ...` diagnostics were retired in #869, and the successor
semantics belongs to the separate smusni project.

The retained generated reference analysis
(`jbotci_semantics::references`, surfaced as Gentufa reference arrows) does
cover the accepted Zantufa shapes: a grouped bridi tail contributes its tail
terms to the same bridi frame as the tail it wraps, statement relative clauses
bind `ke'a` to the relative head exactly as baseline bridi relatives do, and
forethought termset branches, statement-terms statements, `FIhOI` adverbial
terms, `SEI` statement free modifiers, and Zantufa tag and mex payloads are
visited as ordinary nested syntax.
