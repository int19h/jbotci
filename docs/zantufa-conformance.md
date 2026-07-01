# Zantufa Parser Conformance Ledger

This ledger tracks parser conformance against Guskant's Zantufa fork, not v0
compatibility. The checked upstream grammar is:

- Source: https://raw.githubusercontent.com/guskant/gerna_cipra/master/zantufa-1.9999.peg
- Header date in file: 2019-08-20 UTC
- Checked locally on: 2026-07-01
- SHA-256: `79e7a1daec2aaa9760af3c650c9e67c393c1c4c1b1e40e1ce905e0a9b652a312`

CLL forethought connectives remain binary (`gek P gik Q`). The Zantufa
forethought-connective semantics page defines n-ary chains such as
`ga P0 gi P1 gi ... gi Pn (gi'i)`, so the parser treats extra `gi` branches
and `gi'i` as Zantufa connective syntax.

## Gate Policy

- `default-warning`: syntax is lexically or structurally distinguishable from
  baseline CLL+xorlo, so it parses by default and emits an experimental warning.
- `feature-required`: syntax changes the behavior of existing baseline
  connective/tag grammar or otherwise has a plausible baseline parse, so the
  specific Zantufa feature or `(zantufa)` is required.
- `compatibility-only`: the public dialect feature exists for dialect formulas
  and v0 compatibility, but it does not gate parser syntax.

There is intentionally no public `ZantufaCmavo` feature in v1. Fork-only cmavo
are recognized by default and warned as experimental words.

## Rule Ledger

| Zantufa rule(s) | Owner | Policy | v1 status | Warning(s) | Notes |
| --- | --- | --- | --- | --- | --- |
| `gek`, `gik`, `GIhI_elidible` inside `sentence`, `gek_bridi_tail`, `sumti_3`, `tanru_unit_1`, `gek_term` | `ZantufaConnectives` | `feature-required` | Partially implemented | `ExperimentalZantufaNaryForethought`, `ExperimentalZantufaForethoughtGihi` | Bridi, termset, sumti, selbri, and tanru-unit forethought forms now accept ordered extra `gi` branches under `ZantufaConnectives`. Extra branch separators match the fork's plain `GI`, not `GI NAI`. Semantics rejects n-ary lowering explicitly for now instead of dropping branches. |
| `gek <- GI (joik / tag) / (joik / tag) GI / ... BO?` | `ZantufaConnectives` | `feature-required` | Partially implemented | `ExperimentalZantufaGek` | Existing parser has `gi`-initial, `joik gi`, `jek gi`, modal `gi`, and `bo`-extended forms. Full parity still needs a direct audit against the fork's `tag` operand because our tag grammar is not yet the fork's `tcita_selci` grammar. |
| `term_2 <- KE !(sumti KEhE) term+ KEhE_elidible` | `ZantufaTerms` | `default-warning` | Implemented | `ExperimentalKeTermset` | Distinguishable KE term grouping parses by default. |
| `gek_term <- gek term+ (gik term+)+ GIhI_elidible` | `ZantufaTerms`, `ZantufaConnectives` | `feature-required` for extra branches/`gi'i` | Implemented through `forethought_termset` | `ExperimentalZantufaNaryForethought`, `ExperimentalZantufaForethoughtGihi` | `nu'i` is accepted when present. Tests cover binary, n-ary, optional `nu'u`, and optional `gi'i`. |
| `term_2 <- XOI_clause statement SEhU_elidible` | `ZantufaAdverbials` | `default-warning` | Implemented | `ExperimentalSoiAdverbial` | `xoi` is accepted through the SOI adverbial path with a full generated `statement` payload, including statement-level `I` connective forms. |
| `FIhOI_clause statement` adverbial behavior | `ZantufaAdverbials` | `default-warning` | Implemented | `ExperimentalFihoiAdverbial` | FIhOI accepts a full generated `statement` payload. The legacy FIhAU terminator remains accepted for compatibility with existing v1/v0 examples. |
| `brigahi <- POIhA free* selbri KU? / NA !bridi_tail !joik_gihek KU?` | `ZantufaTerms`, `ZantufaAdverbials` | `default-warning` where distinguishable | Partial | `ExperimentalZantufaPoihaBrigahi` | POIhA/NOIhA selbri term with `ku` parses by default, and the fork's `free*` slot before the selbri is implemented. TODO: the `NA` briga'i branch still needs a direct fork-conformance audit against existing bare-NA term behavior. |
| `tag_term` with `JAI_clause tag?` | `ZantufaTags` | `feature-required` | Implemented | `ExperimentalZantufaJaiTagTerm` | Existing gate matches the conflict policy. Full fork guards around `tanru_unit_1`, `BO`, and `gek_bridi_tail` need a separate audit. |
| `sumti_5 <- RAhOI_clause` | `ZantufaQuotes` | `default-warning` | Implemented | `ExperimentalZantufaRahoiQuote` | Fork-only quote cmavo parses by default. |
| `sumti_5 <- LOhOI_clause ... statement KUhAU_elidible` | Zantufa quote/cmavo surface | `default-warning` | Implemented for current bridi-description surface | `ExperimentalLohOiBridiDescription`, `ExperimentalZantufaCmavo` for table-only variants | Current parser accepts the known LOhOI bridi-description forms and warns. A full audit against `statement` payload remains open. |
| `tanru_unit_1 <- MUhOI_clause / GOhOI_clause / LUhEI_clause text LIhAU?` | `ZantufaQuotes` | `default-warning` | Implemented | `ExperimentalZantufaMuhoiSelbriUnit`, `ExperimentalGohoiSelbriUnit`, `ExperimentalZantufaLuheiSelbriUnit` | Distinguishable quote-as-selbri-unit forms parse by default. |
| `relative_clause <- NOI_clause statement KUhO?` | Baseline relative clauses plus Zantufa statement widening | `feature-required` if it changes baseline parse | Not implemented | TBD | Current relative-clause parser does not intentionally widen NOI payloads to full Zantufa `statement`. |
| `NU_clause (joik NU_clause)* statement KEI?` | Zantufa abstraction payload widening | `feature-required` if it changes baseline parse | Not implemented | TBD | Current NU payload remains the v1/CLL-shaped parser. This is needed for complete `{tu'e ... tu'u}`-style statement abstractions. |
| `mex`, `mex_1`, `mex_2`, `mex_rp`, `mex_forethought`, `operator`, `operand` | `ZantufaMex` | `feature-required` for conflicting grammar, `default-warning` for fork-only cmavo | Partial | `ExperimentalZantufaMex` | Implemented gated parsing for MOhE-selbri operands, MAhO-selbri and MAhO-sumti operators, NAhE operands without BO, and KE-grouped mex operand sequences. Existing v1 forethought-call and reverse-Polish mex remain close to the fork but still need a rule-by-rule audit for `mex_2+`, recursive `mex_forethought`, and ME-as-selbri-unit payloads. |
| `tag <- tcita_selci+ (joik tcita_selci+)*` and recursive `tcita_selci` | `ZantufaTags` | `feature-required` | Partial | `ExperimentalZantufaRecursiveTag` | Current parser only has the older limited recursive SE/NAhE tag prefix. TODO: replace with the fork's `tcita_selci` list shape. |
| `free <- SEI statement ... / mex_2 MAI / xi_clause ...` | Mixed terms/mex/free surfaces | Mixed | Not fully audited | TBD | Existing free-modifier grammar is still CLL/v1-shaped. Zantufa statement and mex widenings should be implemented after `statement` and `ZantufaMex` parity. |
| Fork-only cmavo table entries | No feature | `default-warning` | Implemented | `ExperimentalZantufaCmavo` | Word-level warning is emitted by token classification. Construct-specific warnings are still emitted when a surrounding experimental construct consumes the word. |
| Non-cmavo morphology differences | `ZantufaMorphology` | `compatibility-only` | Not syntax-gated | None | Keep the public feature for dialect formulas. Concrete non-cmavo morphology differences should be tracked in morphology work, not hidden behind parser gates. |

## Current Implementation Checkpoints

- N-ary Zantufa forethought branches are represented in generated syntax as a
  first branch plus ordered `additional_branches`, preserving input order.
- `gi'i` is consumed as `GIhI` only in Zantufa forethought contexts, under
  `ZantufaConnectives`.
- `ZantufaAdverbials`, `ZantufaMex`, `ZantufaQuotes`, and `ZantufaTerms` are
  now available to generated parser feature gates, not just dialect formulas.
- XOI/FIhOI adverbial terms now carry generated `statement` payloads.
- Zantufa mex now has gated parser coverage for the highest-confidence fork
  operand/operator deltas listed above.
- N-ary forethought semantic lowering is intentionally unsupported until the
  semantic model defines n-ary truth-functional behavior.
- Unit tests cover bridi and termset branch-count grids, optional `gi'i`, and a
  sourced Guskant `ju'e gi ...` example.
