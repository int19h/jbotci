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
