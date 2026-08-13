# Grammar parity epoch 4: sumti continuations

This note records the durable implementation dispositions for GitHub issues
#838 and #839. The source snapshot is grammar-review commit
`4f97db48a7a84dc54b1f167d9a0f6990884cb466`; its generated
`camxes-std.syntax.peg` and `camxes-exp.syntax.peg` SHA-256 values are,
respectively, `f9685f52685fb07e1f2cd2381a912c3f50de2ff1808cc242d2b5f2ee2391db55`
and `1999484d3fc64f2a1ed45863709b011b9816b4768ef03565f9b813d74780d30e`.
The rolling-Zantufa source is gerna_cipra commit
`d5a5065c924304cf5e9067ee6d41b584fbe1c099`; its
`zantufa-1.9999.peg` SHA-256 is
`79e7a1daec2aaa9760af3c650c9e67c393c1c4c1b1e40e1ce905e0a9b652a312`.

## Relative-clause-list consumer dispositions

The experimental afterthought continuation is part of the shared
`relative_clause_list`, so every consumer of that nonterminal includes the
warning-gated `NA? SE? (JOI|JA|A) NAI?` continuation. The table enumerates the
consumers rather than relying on that fact implicitly.

| Consumer | Disposition | Grammar enforcement / owner |
| --- | --- | --- |
| Sumti-5 position | Include | `simple_sumti.relative_clauses` consumes the shared list. |
| LAhE pre-inner slot | Include | `lahe_sumti.relative_clauses` consumes the shared list before its inner sumti. |
| NAhE-BO pre-inner slot | Include | `scalar_negated_sumti_with_bo.relative_clauses` mirrors the sourced LAhE slot. |
| LA-name position | Include | `name_sumti.relative_clauses` consumes the shared list before the cmevla run. |
| No-gadri description position | Include | `descriptor_without_gadri_sumti.relative_clauses` consumes the shared list after KU. |
| Description leading-tail position | Include | `leading_description_tail_elements.relative_clauses` consumes the shared list. |
| Relation description tail | Include | `relation_description_tail.relative_clauses` consumes the shared list. |
| Quantified relation description tail | Include | `quantifier_relation_description_tail.relative_clauses` consumes the shared list. |
| VUhO baseline attachment | Include | `vuho_relative_sumti_attachment_tail` requires the shared list. |
| VUhO experimental scoped attachment | Include | `experimental_vuho_scoped_sumti_attachment_tail` requires the shared list before its required continuation; the separate bare arm represents the optional outer group from camxes-exp. |
| Selbri vocative, leading position | Include | `selbri_vocative_sumti.leading_relative_clauses` consumes the shared list. |
| Selbri vocative, trailing position | Include | `selbri_vocative_sumti.trailing_relative_clauses` consumes the shared list. |
| Cmevla vocative, leading position | Include | `cmevla_vocative_sumti.leading_relative_clauses` consumes the shared list. |
| Cmevla vocative, trailing position | Include | `cmevla_vocative_sumti.trailing_relative_clauses` consumes the shared list. |
| Relative fragment | Include | `relative_clause_fragment` wraps the shared list directly. |
| Bare relative-clause adjacency | Exclude / defer | Rolling Zantufa's connectorless repetition is owned by #828; no heuristic adjacency route is introduced here. |
| Forethought GEK relative list | Exclude / defer | The distinct camxes-exp forethought structure is owned by #855 and remains rejected. |

## VUhO ownership rule

The structurally closed scoped-continuation alternative is ordered first but
requires immediate explicit LUhU lookahead after its completed sumti. This
guard makes it impossible for that arm to steal a top-level term connection;
there the required lookahead fails, `VUhO` plus a required relative list stays
warning-free, and the generic term-connection layer owns the continuation. The
warning-gated scoped alternative is selected only in an explicitly closed
consumer such as `LAhE ... LUhU`. Consequently,
an elided LUhU and its explicit twin may intentionally have different owners:
the elided form retains the baseline inner attachment and an outer generic
connection, while the explicit form gives the connection to the experimental
VUhO attachment. This is the recorded camxes-exp inner-attachment waiver.

Bare VUhO is the third alternative. It denotes an empty relative attachment,
emits `ExperimentalVuhoScopedAttachment`, and otherwise leaves the underlying
sumti referent unchanged. Experimental VUhU in a scoped continuation retains
the existing explicit unsupported-semantics result from connective lowering.
The residual generic term-layer ownership is queued for the epoch-6 rebuild in
#792.

## Relative continuation ownership

The experimental continuation route parses the entire connective and following
relative-clause atom before ownership is classified. A completed candidate is
rejected only when it is baseline-representable: no NA/SE/NAI affixes and a
ZIhE head. Baseline reparsing then consumes exactly the same connective and
relative atom. This whole-candidate rule deliberately does not use token-class
lookahead, because vocabulary entries such as `voi'e` and `po'oi` belong to
multiple selmaho and can begin the following atom through a non-obvious class.

The pinned running parsers establish the adopted boundaries: camxes-exp accepts
JA/A afterthought continuations and the VUhO scoped/bare shapes, while standard
camxes keeps ZIhE-only relative repetition and requires a relative list after
VUhO. Rolling Zantufa also accepts ZIhE through lexical JOI. Its bare adjacency
and other wider repetitions remain deferred to #828 rather than being folded
into this exact camxes-exp route.

## C3 expectation ledger

The single consolidated rewrite scanned 26,302 fixtures and changed 69. The
committed comparer, `tools/compare-sumti-continuation-expectations.py`, compares
the pre-C3 tree with the regenerated tree and accepts exactly two mechanical
classes at identical spans:

- **Sumti-connective re-wrapping (2):** `corpus/camxes/17625` and
  `corpus/camxes/18858`. The old VUhU argument-connective wrapper becomes the
  warning-gated VUhU sumti-connective wrapper; its token, span, and enclosing
  sumti continuation are unchanged. The corresponding unsupported-semantics
  diagnostic changes only “argument connective” to “sumti connective”.
- **Relative-continuation re-typing (2):**
  `adhoc/v0/warnings/experimental/simpler-joi-relative-clause-connective` and
  `adhoc/v0/warnings/experimental/simpler-relative-clause-connective`. The old
  JOIK/JEK connected-relative node becomes the experimental relative
  continuation with the same connective head, following relative atom, and
  spans.

The other 65 fixtures were reviewed manually. Each row below names every
fixture in that behavioral class; paths are relative to `tests/fixtures`.

| Behavior | Fixtures | Manual justification |
| --- | --- | --- |
| CEhE removal from sumti continuation (10) | `cll/chapter-14/section-14.11/c14e11d2`, `cll/chapter-14/section-14.11/c14e11d4`, `cll/chapter-14/section-14.14/c14e14d15`, `cll/chapter-16/section-16.7/c16e7d5`, `corpus/camxes/1440`, `corpus/camxes/1533`, `corpus/camxes/3`, `corpus/camxes/811`, `corpus/camxes/812`, `muplis/collection-18/1323-front` | Each CEhE pair moves, at the same token spans, from an invalid sumti afterthought connection into the existing typed `TermsetGroup`/`TermsetGroupContinuation` owner. The larger reference deltas are the intended consequence: all terms of the termset are now assigned instead of treating the second member as a connected sumti. |
| Empty NAhE-BO relative slot (41) | `adhoc/v0/warnings/standard-no-warning/standard-nahe-bo-argument`, `cll/chapter-06/section-6.10/c6e10d10`, `cll/chapter-06/section-6.10/c6e10d11`, `cll/chapter-15/section-15.6/c15e6d2`, `cll/chapter-15/section-15.6/c15e6d3`, `corpus/camxes/11165`, `corpus/camxes/11478`, `corpus/camxes/12195`, `corpus/camxes/12797`, `corpus/camxes/1456`, `corpus/camxes/1522`, `corpus/camxes/15757`, `corpus/camxes/1636`, `corpus/camxes/16427`, `corpus/camxes/16583`, `corpus/camxes/1660`, `corpus/camxes/17746`, `corpus/camxes/17808`, `corpus/camxes/18301`, `corpus/camxes/18499`, `corpus/camxes/18544`, `corpus/camxes/18611`, `corpus/camxes/19308`, `corpus/camxes/19445`, `corpus/camxes/19717`, `corpus/camxes/19738`, `corpus/camxes/2003`, `corpus/camxes/20071`, `corpus/camxes/20427`, `corpus/camxes/20459`, `corpus/camxes/20560`, `corpus/camxes/21328`, `corpus/camxes/21377`, `corpus/camxes/21466`, `corpus/camxes/2204`, `corpus/camxes/2495`, `corpus/camxes/2510`, `corpus/camxes/2725`, `corpus/camxes/7148`, `corpus/camxes/7796`, `muplis/collection-18/865-front` | Each tree differs only by `relative_clauses: None` in `ScalarNegatedSumtiWithBoSyntax`, immediately before the unchanged inner sumti. This is the new sourced slot in its empty state; acceptance, ownership, diagnostics, spans, and semantics remain unchanged. |
| Baseline VUhO required-relative variant (12) | `adhoc/v0/warnings/standard-no-warning/standard-vuho-relative-clause`, `cll/chapter-08/section-8.8/c8e8d6`, `cll/chapter-08/section-8.8/c8e8d8`, `cll/chrestomathy/alice01`, `corpus/camxes/11182`, `corpus/camxes/1984`, `corpus/camxes/2029`, `corpus/camxes/2033`, `corpus/camxes/2038`, `corpus/camxes/2467`, `corpus/camxes/832`, `corpus/camxes/833` | Each warning-free baseline VUhO tree loses only the obsolete `sumti_connection: None` field. VUhO, its required relative list, all spans, and the semantic attachment are unchanged. |
| Bare VUhO term-continuation ownership (1) | `adhoc/v0/warnings/experimental/vuho-scoped-attachment` | For bare VUhO with no LUhU and no relative clause, `.e ko'e` moves out of the old unsourced VUhO-connected tail and into the ordinary term-level `ConnectedTerm` continuation; VUhO itself is represented by `ExperimentalBareVuhoSumtiAttachmentTail`. This matches camxes-exp :175, where a VUhO-owned continuation is sourced only inside the optional relative-clause group. The token spans are unchanged. |
| Hash-only full corpus (1) | `corpus/alis/full-alice` | The syntax digest changes from the same explicit empty NAhE-BO slots and removed baseline-VUhO empty continuation fields audited above. Its morphology digest, reference digest, and tersmu digest remain byte-for-byte unchanged, ruling out tokenization or semantic drift. |

The comparer reports `2 + 2` mechanical fixtures and all 65 manual fixtures;
it does not normalize CEhE acceptance, VUhO ownership, optional-field additions,
warnings, or hashes.
