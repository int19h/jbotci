# Zantufa Parser Conformance Ledger

This ledger tracks v1 parser behavior against Guskant's frozen Zantufa fork.

- Grammar: https://raw.githubusercontent.com/guskant/gerna_cipra/master/zantufa-1.9999.peg
- Grammar SHA-256: `79e7a1daec2aaa9760af3c650c9e67c393c1c4c1b1e40e1ce905e0a9b652a312`
- Parser snapshot SHA-256: `127562f0f7b05bb805060ff9ba419b2cfadaa56e47b74aca30ab3bdb11787283`
- Checked locally on: 2026-07-01
- Captured fixture: [tests/fixtures/zantufa/upstream-parity.json](/home/int19h.linux/git/jbotci/tests/fixtures/zantufa/upstream-parity.json)

CLL forethought connectives are binary. Zantufa permits n-ary forethought
chains such as `ga P0 gi P1 gi ... gi Pn (gi'i)`. v1 represents those as
ordered branch vectors and lowers logical n-ary forms left-to-right where the
connective has an existing semantic operator.

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

| Upstream rule surface | Owner | Policy | Parser status | Semantic status |
| --- | --- | --- | --- | --- |
| `gek_statement`, `sentence`, `gek_bridi_tail`, `sumti_3`, `tanru_unit_1`, `gek_term` n-ary `gik+` plus optional `GIhI` | `ZantufaConnectives` | `feature-required` | Accepted with ordered branch vectors and `GIhI` as `GIhI`, not baseline `GIhA` | Logical statement, bridi, selbri, sumti, and termset forethoughts lower left-to-right or to existing composite/distributed forms; modal/nonlogical n-ary forms that lack a specified target diagnose explicitly |
| Zantufa `gek` variants including `GI (joik/tag)`, `(joik/tag) GI`, and optional `BO` | `ZantufaConnectives`, `ZantufaTags` | `feature-required` | Accepted by the generated connective/tag grammar | Logical forms lower through existing connective semantics; unsupported modal forms diagnose explicitly |
| `statement_terms <- statement IAU? terms?` | `ZantufaTerms` | mixed | Accepted under `ZantufaTerms`; `IhAU` preservation is dialect-sensitive so it is not swallowed as an indicator | Empty `IhAU` delegates to the inner statement; trailing terms append to bridi-tail semantics where the bridi shape is supported, while non-bridi and post-`CU` edge cases diagnose explicitly |
| `bridi_tail_3 <- KE bridi_tail KEhE? tail_terms` | `ZantufaTerms` | `feature-required` | Accepted under `ZantufaTerms` as `ZantufaGroupedBridiTail`; gated because it conflicts with baseline `KE` tanru grouping | Traversal/reference handling recurses into inner bridi-tail and tail terms; semantic construction uses the same bridi-tail path when no unsupported tail-term shape is introduced |
| `term_2 <- KE !(sumti KEhE) term+ KEhE?` | `ZantufaTerms` | `default-warning` | Accepted by default as KE term grouping | Uses existing termset semantics where representable |
| `gek_term <- gek term+ (gik term+)+ GIhI?` | `ZantufaTerms`, `ZantufaConnectives` | `feature-required` for extra branches and `GIhI` | Accepted through `forethought_termset` | Logical n-ary forms lower by folding termset branch formulas; modal, nonlogical, and `FAhU` n-ary cases diagnose explicitly |
| `term_2 <- XOI statement SEhU?` and `FIhOI statement` | `ZantufaAdverbials` | `default-warning` | Accepted with full generated `statement` payloads | Statement payloads are visited and bound like ordinary nested statements |
| `brigahi <- POIhA free* selbri KU? / NA ... KU?` | `ZantufaTerms`, `ZantufaAdverbials` | `default-warning` where distinguishable | POIhA/NOIhA and NA briga'i forms are accepted by the generated term grammar | POIhA/NOIhA lower through existing relation/adverbial paths; unsupported NA briga'i semantics diagnose explicitly |
| `tag_term` with `tag`, `FA (joik FA)*`, and `JAI tag?` | `ZantufaTags` | `feature-required` for conflicting tag behavior | Accepted under `ZantufaTags`, including the JOIK-chained place tag and the `(sumti / KU_elidible)` payload in both halves, with `!tanru_unit_1` asserted structurally so `jai broda` and `fa je fe broda` stay selbri | Tag terms feed existing tagged-term semantics where the tag has a semantic modal target; the JOIK-chained place tag names several places at once and diagnoses explicitly |
| `term_1 <- term_2 (joik_ek? BO_clause term_2)*`, connective ABSENT | `ZantufaTerms` | `feature-required` | Accepted under `ZantufaTerms` as a continuation of the sourced BO connection, so one node may mix connectorless and connective-present joints as upstream does; the connective-present stag-less form stays the sourced camxes-exp arm's | Term-level BO connections reuse the existing term-grouping traversal, which does not read the connective |
| `sumti_2 <- sumti_3 (joik_ek? tag? BO_clause sumti_3)*`, connective ABSENT | `ZantufaTerms` | `feature-required` | Accepted under `ZantufaTerms` at the BASELINE BO-precedence level, tag optional | No source says what a connective-less sumti connection means, so the referent builder diagnoses it explicitly rather than reading it as any particular connective |
| `BO <- ce'e / bo` | — | documented gap | `ko'a ce'e ko'e broda` keeps the baseline CEhE termset group in every configuration, including `(zantufa)` | Baseline termset semantics; the Zantufa BO-connection reading is a fidelity-flag candidate, recorded and not minted |
| `tag <- tcita_selci+ (joik tcita_selci+)*`, recursive `tcita_selci` | `ZantufaTags` | `feature-required` | Accepted by v1 connected tag grammar plus Zantufa recursive prefix atoms | Existing modal/tense semantics are reused; unsupported compound tags diagnose explicitly |
| `relative_clause <- NOI statement KUhO?` | `ZantufaTerms` | `feature-required` | Accepted under `ZantufaTerms`; default keeps ordinary bridi relatives because an elided baseline `KUhO` can otherwise take the same prefix | Statement restrictions lower through the generated relative-clause builder; `ke'a` handling is preserved |
| `LOhOI (joik LOhOI)* statement KUhAU?` | `ZantufaQuotes` | `default-warning` | Accepted by default for bridi-description sumti | Semantic lowering uses existing description/referent machinery where available |
| `RAhOI`, `MUhOI`, `GOhOI`, `LUhEI ... LIhAU?` quote surfaces | `ZantufaQuotes` | `default-warning` | Accepted by default | Quote-as-sumti and quote-as-selbri-unit forms lower to existing sign/quote concepts where representable |
| `NU (joik NU)* statement KEI?` | `ZantufaTerms` | `feature-required` | Accepted under `ZantufaTerms` before ordinary NU so baseline abstractions do not warn | Reducible single-bridi statements lower to relation labels; connected/text/forethought statement payloads diagnose unsupported Zantufa abstraction semantics |
| `ME (sumti / operator+ / mex / tag) MEhU? MOI?`, `mex MOI` | `ZantufaMex` | mixed | Accepted; fork-only payloads warn as `ExperimentalZantufaMex` | Sumti-compatible `ME` forms reuse existing semantics; operator/mex/tag and `mex MOI` relation-label forms lower to stable surface relation labels |
| `mex`, `mex_1`, `mex_2`, `mex_rp`, `mex_forethought` | `ZantufaMex` | mixed | Accepted for default-warning raw mex fragments and feature-required raw mex quantifiers, BO-grouped mex, KE-grouped operand sequences, reverse-Polish tails, operator-first mex, and optional trailing operator forms | Ordinary binary/operator-call forms lower to existing math expressions; grouped mex and operator-sequence/trailing-operator forms lower to structural math-expression nodes preserving operand and operator order |
| `operator <- SE operator / NAhE operator / MAhO (mex/selbri/sumti) / VUhU / joik_ek !CU` | `ZantufaMex` | `feature-required` when it changes operator parsing | Accepted under `ZantufaMex`; connective operators are guarded from `CU` | Existing math-operator semantics are reused where the operator has a model label |
| `operand <- number / lerfu / VEI mex / MOhE (selbri/sumti) / LAhE|NAhE BO mex / NAhE operand` | `ZantufaMex` | `feature-required` where it conflicts | Accepted under `ZantufaMex`, including scalar `NAhE operand` and selbri `MOhE` | Sumti/selbri operands lower where an existing math operand model exists; scalar negation and grouped operand sequences preserve their structure in math-expression nodes |
| `free <- SEI statement / mex_2 MAI / xi_clause / ...` | `ZantufaTerms`, `ZantufaMex` | mixed | `SEI statement` is feature-required where it shadows ordinary SEI; `mex_2 MAI` and statement-term `XI` cases are feature-required when raw mex quantifier behavior would conflict with baseline quantification | Nested statement and mex payloads are traversed; unsupported dangling mex semantics remain explicit |
| Non-cmavo morphology differences | `ZantufaMorphology` | `compatibility-only` | No syntax is gated by this flag | Concrete non-cmavo morphology work is tracked outside the syntax parser |

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

## Semantic Boundary

Zantufa semantics are implemented by desugaring into existing semantic model
objects where the fork gives a well-defined reading: logical n-ary forethought
folding, statement-level term attachment, statement relative clauses, raw mex
quantities, bare mex fragments as number mentions, quote/relation labels, and
structural math/operator forms. For parser-accepted grammar shapes without a
sound semantic target in the Zantufa materials, the builder returns explicit
`Unsupported Zantufa ...`
diagnostics with source context rather than silently dropping syntax or
guessing.
