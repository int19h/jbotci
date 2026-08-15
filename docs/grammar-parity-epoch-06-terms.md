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

### Mechanism E's cost, and the recursive-family fix

Leaf-listing keeps the public shape stable, and the first cut of the ladder paid
a large, wrongly-attributed price for it. The ladder was measured at +55% on the
full fixture profile and +63–77% on isolated parse throughput, and that cost was
read as intrinsic to leaf-listing: five levels each re-listing a fifteen-rule
leaf inventory, so every term in every text pays roughly five times the
per-level memo traffic.

Profiling refuted that reading. The regression was a **fixed per-parse cost**,
not a per-token one: the delta was +8.1 ms on a two-word text, +8.5 ms on a
100-character sentence and +11 ms on an 8 KB text. A per-token cost scales with
length; this one did not. Over 65% of samples sat in `malloc`/`free`, and the
inclusive call graph put the term-ladder *parser constructors* — not the parse
— at the top of the profile.

The cause is that the ladder levels were never added to the grammar's recursive
parser family. `strict_generated_parser_family()` builds the combinator graph
once per parse and hands out cheap clones of its members; a rule outside the
family is instead re-constructed inline at every reference site. `term` was in
the family, but `cehe_term`, `loose_term`, `nonabs_term`, `bound_term` and
`simple_term` were not, so each of the ~2–3 reference sites per level rebuilt
its whole subtree, and the levels nest — `term` → `termset_group` →
`loose_term` → `connected_term` → `bound_term` → `stag_bound_term_connection` →
`simple_term` — which multiplies the rebuilds through the ladder. Every other
ladder in the grammar (sumti, selbri, mekso) already declares each level in the
`recursive` block; the term ladder simply omitted it.

The fix is that omission repaired: the five levels join the `recursive` block,
their reference sites take them as parameters instead of calling them, and each
level is constructed once per parse. No rule, no alternative, no alternative
*order* and no output type changes, so the panel-approved ladder is untouched
and every tree is byte-identical — the full profile passes with the same
73,733 / 514 / 0 pass-xfail-fail counts as before the fix.

Measured on this host, release binaries, all three states against the same
fixture tree on the same filesystem with a warm page cache:

| Full release fixture profile | Base `a8b4f06227` | Ladder, unfixed | Ladder + fix |
| --- | ---: | ---: | ---: |
| Wall clock | 344.9 s | 468.3 s (+35.8%) | **315.3 s (−8.6%)** |
| CPU (user + sys) | 1,074 s | 1,844 s (+71.6%) | **859 s (−20.0%)** |
| Peak RSS | 5,773,984 KB | 5,774,252 KB (+0.005%) | 5,764,824 KB (−0.16%) |

Isolated parse throughput, `jbotci gentufa --benchmark`, unfixed column as
recorded at first submission:

| Text | Base | Ladder, unfixed | Ladder + fix |
| --- | ---: | ---: | ---: |
| `--benchmark 500 "mi klama le zarci"` | 5.02 s | 8.83 s (+77%) | **3.86 s (−23%)** |
| `--benchmark 500 "lo nu mi citka lo plise cu se pluka mi"` | 6.47 s | 10.06 s (+63%) | **5.07 s (−22%)** |
| `--benchmark 50 "mi klama"`, median | 8.63 ms | 16.45 ms (+91%) | **6.21 ms (−28%)** |
| 8 KB Alice slice, median | 103.90 ms | 115.19 ms (+11%) | 103.95 ms (±0%) |
| Full Alice, 154 KB, median | 1,545 ms | — | **1,531 ms (−1%)** |

The fixed ladder is faster than the pre-epoch base everywhere, by the most on
short texts, because the base paid the same defect at lower multiplicity:
`connected_term` and `simple_term` were already being re-constructed at several
sites before the epoch deepened the ladder. Long texts sit at parity, which is
the expected shape — construction is a fixed cost, so it vanishes into a long
parse either way.

One measurement note for anyone re-running these: the fixture-tree filesystem
dominates comparisons of this suite. The long-text fixtures are ~50 MB of TOML,
and reading them from the repository volume rather than the scratch volume moved
the *same binary* by up to +23% wall clock — enough to invent or hide a
regression on its own. Every figure above puts both trees on one volume with the
cache warmed.

The IDE completion guard `memoized_mid_size_recovery_remains_grammar_filtered`
was disclosed at first submission as moving 1.48 s → 1.63 s against its 2 s
literal bound, with one flake at 2.019 s under full-suite parallel load. It is
carried by the same construction cost: it now reports 1.30 s, stable across
three consecutive isolated runs, which is below the pre-epoch base as well as
the bound.

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

### Diagnosed tiers and the two warning mechanisms

Both extension tiers are diagnosed. camxes-standard has no term-level connective
at all, so every BO (T4) *and* every loose (T3) term connection is an extension
and warns in every profile. There is no documented gap.

The two tiers reach that outcome through different mechanisms, because they own
different amounts of the source text:

| Tier | Warning | Mechanism | Anchor |
| --- | --- | --- | --- |
| T4, BO-bound | `syntax.warning.experimental-term-bo-connection` | In-parser token warn: `cmavo(Bo).warn(ExperimentalTermBoConnection)` in the rule body (`generated.rs` `stag_bound_term_continuation`, `bound_linked_term_continuation`) | The `bo` token |
| T3, loose | `syntax.warning.experimental-term-loose-connection` | Post-parse construct visitor: `GeneratedConstructWarningVisitor::warn_first_token` on `ConnectedTermContinuationSyntax` and `ConnectedLinkedTermContinuationSyntax` (`grammar/mod.rs`) | The continuation's first token, which is its connective |

The split is forced by the grammar, not chosen for convenience. `ParserState::warn`
takes a `&Token`, so an in-parser warn has to name a token the rule itself
matches. T4 matches its own `bo`, so it warns there. A T3 continuation matches
no token of its own: its connective is the shared `joik_connective` /
`ek_connective` node that the sumti and statement tiers also use, so an
in-parser warn at that site would fire on every tier that shares the inventory.
The alternative — duplicating the joik|ek inventory, including the GAhO interval
forms, as term-tier node types — carries real fidelity risk for no gain.

The construct visitor already solves exactly this problem for the rule-level
constructs that own no distinguishing token (`ExperimentalFlattenedTag`,
`ExperimentalZantufaTag`, `ExperimentalZantufaSelbriRelativePlacement`). It runs
over the completed tree, so it can anchor on a node's first token; for a loose
continuation that first token *is* the connective, because the continuation is
`connective trailing_term`. No inventory is duplicated and no DSL change is
needed, which is why #860 — filed to extend the macro for this case — is
superseded rather than implemented.

One warning is emitted per continuation, so an n-operand loose chain carries
n − 1 warnings; `ba ko'a .e ca ko'e .a vi ko'i broda` carries two. The warnings
join the parser-attached stream and are re-sorted by anchor index, so a mixed
chain reports its T3 and T4 warnings in source order.

The four positions where the epoch newly makes the loose tier *reachable* each
carry the warning. Each row was re-probed against the base binary
(`a8b4f06227`) and the epoch binary in the default profile.

| Newly reachable surface | Position that gained T3 | Base | Epoch | Witness |
| --- | --- | --- | --- | --- |
| `ba ko'a ce'e ca ko'e .e vi ko'i broda` | CEhE leading operand (`loose_term`, camxes-exp.peg:122) | rejects | accepts + T3 warning on `.e` | `issue-792-cehe-loose-composition` |
| `ko'a ce'e ba ko'e .e ca ko'i broda` | CEhE continuation (`nonabs_term`, camxes-exp.peg:122) | rejects | accepts + T3 warning on `.e` | `issue-792-cehe-continuation-loose` |
| `ko'a pe'e je ba ko'e .e ca ko'i broda` | PEhE operand (`cehe_term`, camxes-exp.peg:121) | rejects | accepts + T3 warning on `.e` | `issue-792-pehe-operand-loose` |
| `mi broda be ba ko'a .e ca ko'e be'o brode` | BE/BEI link (`linked_term`, camxes-exp.peg:200) — the tier lost its dialect gate | rejects | accepts + T3 warning on `.e` | `adhoc/issues/issue-791/06-loose-link-chain` |

The warning is not limited to those four: it fires wherever the loose tier is
selected, including the positions that were already reachable at the
implementation base. `ba ko'a .e ca ko'e broda` parsed warning-free at the base
and now carries the T3 warning; that is the intended behaviour change and is the
reason the epoch re-pins fixtures it did not otherwise touch (see the re-pin
table below).

The BO tier at the same four positions warns as before:
`ba ko'a ce'e ca ko'e .e ba bo vi ko'i broda` and
`mi broda be ba ko'a .e ba bo ca ko'e be'o brode` carry
`syntax.warning.experimental-term-bo-connection` on their `bo`, and no T3
warning — the `.e` there belongs to the BO continuation, not to a loose one.
`ko'a ce'e pu broda` is a new acceptance that carries *neither* warning: the
unguarded CEhE continuation is camxes-standard's own `nonabs_term`, so no
extension is being diagnosed.

### T3 warning re-pins

The warning changes no tree, no status and no other diagnostic. Every affected
expectation gains T3 warning entries and keeps every pre-existing diagnostic in
place and in order, which is the property
`tools/compare-term-hierarchy-expectations.py` enforces as its class (iv)
`t3-loose-connection-warning`.

| Re-pinned expectations | Fixtures | Warnings added |
| --- | --- | --- |
| Epoch witnesses (C1–C6, exact commit-local pins) | 9 | 10 |
| Pre-epoch fixtures the tier already reached | 13 | 15 |
| **Total** | **22** | **25** |

Both rows are the comparer's own report rather than a hand count: the 13 are its class-(iv)
incidence and the 9 are its `epoch-witness T3 re-pins` list, each named in
`epoch06-comparer-round3.txt`.

Only one of the 22 is a corpus fixture (`corpus/camxes/5226`). Term-level
connectives are rare in running text because the sumti tier's greedy connective
absorbs almost every `.e` before the term tier can offer one; the loose tier is
overwhelmingly exercised by the adhoc suites that target it.

Because the warning lands on surfaces the epoch's own witnesses pin, the
comparer's zero-witness-delta guarantee is restated rather than dropped: a
witness may now take the additive class-(iv) re-pin and nothing else. Any other
witness delta, or a witness that mixes class (iv) with any other class, is still
a hard error.

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
`adhoc/v0/syntax/basic/cache-vuhu-connective-after-joik-miss` (VUhU). Two corpus
fixtures carry the flip as well, and the consolidated regeneration below
disposes of both individually: `corpus/camxes/811` is a PEhE + EK surface that
now rejects, and `corpus/camxes/20100` is a term-level JEK whose rejection
*retires* an xfail, because the camxes corpus expected the rejection all along.
The
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

## Consolidated regeneration, comparer, and manual residue

The epoch carries exactly one expectation update, as #792 requires ("do not refresh
intermediate `TermSyntax` shapes"): the C1-C6 commits pin only their own witnesses, and
every pre-existing expectation is regenerated once, here.

`fixture-rewrite` visited all 26,444 fixtures. The baseline they are measured against is
`git archive 667178f5a7 tests/fixtures` — the fixture tree at the C1-C6 tip, immediately
before the regeneration — so the archive is reproducible from git rather than hand-assembled.
Against it, 18,249 pre-existing fixtures changed. Of the 35 witnesses added by C1-C6, 9 take
the additive class-(iv) T3 re-pin and the other 26 changed in **zero** leaves; the comparer
verifies both as hard errors rather than skips. Pairing is itself fail-closed in both
directions: a candidate fixture with no archive entry, or an archive entry with no candidate,
is reported as a hard error and never skipped, so no re-pin can leave the audit silently.

`regenerate_syntax_fixture` refuses any fixture carrying `expectations.syntax.xfail`
(`xtask-full/src/main.rs:8813`), because an xfail pin records a corpus-expected status that
differs from the accepted one. The re-leveling changes the accepted tree of every such
fixture, so those 394 trees were regenerated through the project's own writer rather than by
hand: each fixture was copied without its `xfail` table, rewritten, and had its original
`status` and `xfail` lines spliced back. The pipeline is validated by the 120 xfail fixtures
whose trees did *not* change — every one of them round-trips byte for byte — and by checking
the regenerated status against `xfail.accepted-status` for all 515. Exactly one fixture
failed that check, and it is the acceptance flip ledgered below.

### The comparer

`tools/compare-term-hierarchy-expectations.py` rewrites the *old* tree with the mechanical
shapes the plan approves and then requires byte equality with the new tree. It never infers
a class from the new tree, from fixture text, or from a span comparison, so an ownership
change cannot be laundered as a re-leveling. Class (iii), sumti-term pass-through, is
prohibited and not implemented.

| Class | Incidence | What it accepts |
| --- | ---: | --- |
| `flat-sum-wrapper` (i) | 18,192 | The wrapper paths of the five former `term` sum siblings: the degenerate `ConnectedTerm { leading_term, continuations: [] }` produced by the old `zero_or_more` list, and the nested-sum `SimpleTerm(..)` variant. Atom-only: never a connection, never a termset or GEK atom, never a VUhO-carrying payload, never a recovered tree. |
| `pehe-cehe-retyping` (ii) | 5 | The PEhE operand's level change and the CEhE continuation's `TaggedSumtiTerm` → `NonabsTaggedSumtiTerm` rename, each only with the parent node *and* the governing connective exhaustively proven. |
| `stagless-bo-route-rejection` (#796) | 0 | An accept→reject flip whose old tree contains the deleted `BoundTermConnection`. No pre-existing fixture used the route; its witnesses landed with C1. |
| `t3-loose-connection-warning` (iv) | 13 | The additive T3 warning on a pre-epoch fixture, with the tree, the status and every other leaf byte-identical and every pre-existing diagnostic kept in place and in order. The 9 witness re-pins are classified by the same rule and reported separately. |
| manual residue | 49 | Individually dispositioned below. |

Every other leaf is compared exactly, with one recorded exception: the `description` prose of
a `provenance` entry, which carries no expectation. The entries must still correspond one for
one and agree on every other field, and the fixtures whose prose moved are listed and counted
in the report — 2, the witnesses whose text the T3 ruling reversal re-described — so an
unreviewed prose edit still fails the run.

Per-level nesting-depth validation is the plan's guard against a term landing at the wrong
ladder depth. Every strip happens at a position whose old and new levels are named in the
comparer's `POSITIONS` table — the 25 consumer sites plus the PEhE, CEhE, BO and link
operands — and the surviving leaf's variant name must belong to that position's *new* level
inventory, transcribed from the rebuilt grammar. A `NonabsTaggedSumtiTerm` outside a CEhE
continuation, or a `TaggedSumtiTerm` inside one, is residue rather than a re-leveling. Two
whole-tree invariants back the table: no regenerated tree may contain a `SimpleTerm(..)`
wrapper, and no regenerated `ConnectedTermSyntax` may carry an empty continuation list.

Any divergence at or inside a `ConnectedTerm`, `ConnectedLinkedTerm`, `LinkedSumti` or
`Linkargs` node is excluded from every mechanical class by name; the degenerate
zero-continuation wrapper is the single enumerated exception, which is why it is spelled out
in class (i). The exclusion is precise rather than textual: the walk is a paired traversal,
so an unchanged `Linkargs` subtree — present in every `be` fixture — is not residue, while a
`Linkargs` subtree that *moved* is.

One reading is recorded for review. Reconciliation B9 spells out "excluding connectives,
VUhO, relatives, termsets, GEK, BO, recovered trees". All of those are implemented except a
blanket exclusion of terms whose sumti carries a relative clause: a relative clause is
sumti-internal structure rather than a term-tier shape, class (i) already requires the
unwrapped payload to match byte for byte, and excluding them would move 1,769 pre-epoch
fixtures into hand-written ledger rows without adding any safety. The VUhO attachment *is*
excluded, because that is a term-versus-sumti ownership surface (the epoch-4 residual above).

### Manual residue: 49 individual dispositions

Nothing below is normalized or silently accepted. The 32 fixtures excluded by old-tree shape
were each re-run through the identical classifier with only the payload exclusion under
review lifted; 31 then classify as `flat-sum-wrapper` with no residue at all, and the
thirty-second (`corpus/alis/full-alice.toml`) leaves only its refs digest, dispositioned on
its own row.

| Fixture | Disposition |
| --- | --- |
| `adhoc/output/gentufa-show-elided.toml` | Gentufa-only fixture with no pinned syntax tree, so no mechanical class can attach and the comparer fails closed. The delta is the retired zero-continuation wrapper, visible as `ConnectedTerm` → `SumtiTerm` in the `show-elided` JSON; the token and span projection of every Gentufa leaf is unchanged. |
| `adhoc/syntax/selbri/issue-840-ek-rejected.toml` | Failure-frontier residue on an input that rejects before and after: `syntax.unexpected-brivla@[12,17] "brode"` → `syntax.unexpected-word@[12,17] "brode"`. The frontier moves because the term tier no longer offers the retired connective alternatives. Exact diagnostics pinned. |
| `adhoc/syntax/selbri/issue-840-jek-tag-ke-rejected.toml` | Failure-frontier residue on an input that rejects before and after: `syntax.unexpected-cmavo@[29,31] "ku"` → `syntax.unexpected-brivla@[18,23] "brode"`. The frontier moves because the term tier no longer offers the retired connective alternatives. Exact diagnostics pinned. |
| `adhoc/syntax/sumti-continuation/vuho-bare.toml` | Excluded old-tree shape: the zero-continuation wrapper covers a VUhO-carrying payload, which class (i) refuses by construction. Individually reviewed: re-running the identical classifier with only that payload exclusion lifted classifies the fixture as `flat-sum-wrapper` with no residue, so the delta is the wrapper removal alone. Exact regenerated tree pinned, no normalization. |
| `adhoc/syntax/sumti-continuation/vuho-baseline.toml` | Excluded old-tree shape: the zero-continuation wrapper covers a VUhO-carrying payload, which class (i) refuses by construction. Individually reviewed: re-running the identical classifier with only that payload exclusion lifted classifies the fixture as `flat-sum-wrapper` with no residue, so the delta is the wrapper removal alone. Exact regenerated tree pinned, no normalization. |
| `adhoc/syntax/tags/issue-822-vuhu-not-adopted.toml` | Failure-frontier residue on an input that rejects before and after: `syntax.unexpected-end@[24,24]` → `syntax.unexpected-cmavo@[11,15] "su'i"`. The frontier moves because the term tier no longer offers the retired connective alternatives. Exact diagnostics pinned. |
| `adhoc/syntax/tags/issue-833-je-waiver.toml` | Failure-frontier residue on an input that rejects before and after: `syntax.unexpected-brivla@[18,23] "broda"` → `syntax.unexpected-cmavo@[14,17] "roi"`. The frontier moves because the term tier no longer offers the retired connective alternatives. Exact diagnostics pinned. |
| `adhoc/v0/warnings/standard-no-warning/standard-vuho-relative-clause.toml` | Excluded old-tree shape: the zero-continuation wrapper covers a VUhO-carrying payload, which class (i) refuses by construction. Individually reviewed: re-running the identical classifier with only that payload exclusion lifted classifies the fixture as `flat-sum-wrapper` with no residue, so the delta is the wrapper removal alone. Exact regenerated tree pinned, no normalization. |
| `cll/chapter-08/section-8.8/c8e8d6.toml` | Excluded old-tree shape: the zero-continuation wrapper covers a VUhO-carrying payload, which class (i) refuses by construction. Individually reviewed: re-running the identical classifier with only that payload exclusion lifted classifies the fixture as `flat-sum-wrapper` with no residue, so the delta is the wrapper removal alone. Exact regenerated tree pinned, no normalization. |
| `cll/chapter-08/section-8.8/c8e8d8.toml` | Excluded old-tree shape: the zero-continuation wrapper covers a VUhO-carrying payload, which class (i) refuses by construction. Individually reviewed: re-running the identical classifier with only that payload exclusion lifted classifies the fixture as `flat-sum-wrapper` with no residue, so the delta is the wrapper removal alone. Exact regenerated tree pinned, no normalization. |
| `cll/chapter-09/section-9.8/c9e8d6.toml` | Excluded old-tree shape: the zero-continuation wrapper covers the `ForethoughtTermset` termset/GEK atom, which class (i) refuses by construction. Individually reviewed: re-running the identical classifier with only that payload exclusion lifted classifies the fixture as `flat-sum-wrapper` with no residue, so the delta is the wrapper removal alone. Exact regenerated tree pinned, no normalization. |
| `cll/chapter-10/section-10.25/c10e25d1.toml` | Excluded old-tree shape: the zero-continuation wrapper covers the `NuhiTermset` termset/GEK atom, which class (i) refuses by construction. Individually reviewed: re-running the identical classifier with only that payload exclusion lifted classifies the fixture as `flat-sum-wrapper` with no residue, so the delta is the wrapper removal alone. Exact regenerated tree pinned, no normalization. |
| `cll/chapter-10/section-10.25/c10e25d2.toml` | Excluded old-tree shape: the zero-continuation wrapper covers the `NuhiTermset` termset/GEK atom, which class (i) refuses by construction. Individually reviewed: re-running the identical classifier with only that payload exclusion lifted classifies the fixture as `flat-sum-wrapper` with no residue, so the delta is the wrapper removal alone. Exact regenerated tree pinned, no normalization. |
| `cll/chapter-14/section-14.11/c14e11d7.toml` | Excluded old-tree shape: the zero-continuation wrapper covers the `ForethoughtTermset` termset/GEK atom, which class (i) refuses by construction. Individually reviewed: re-running the identical classifier with only that payload exclusion lifted classifies the fixture as `flat-sum-wrapper` with no residue, so the delta is the wrapper removal alone. Exact regenerated tree pinned, no normalization. |
| `cll/chapter-14/section-14.15/c14e15d8.toml` | Excluded old-tree shape: the zero-continuation wrapper covers the `ForethoughtTermset` termset/GEK atom, which class (i) refuses by construction. Individually reviewed: re-running the identical classifier with only that payload exclusion lifted classifies the fixture as `flat-sum-wrapper` with no residue, so the delta is the wrapper removal alone. Exact regenerated tree pinned, no normalization. |
| `cll/chapter-14/section-14.15/c14e15d9.toml` | Excluded old-tree shape: the zero-continuation wrapper covers the `ForethoughtTermset` termset/GEK atom, which class (i) refuses by construction. Individually reviewed: re-running the identical classifier with only that payload exclusion lifted classifies the fixture as `flat-sum-wrapper` with no residue, so the delta is the wrapper removal alone. Exact regenerated tree pinned, no normalization. |
| `cll/chapter-16/section-16.7/c16e7d5.toml` | Excluded old-tree shape: the zero-continuation wrapper covers the `NuhiTermset` termset/GEK atom, which class (i) refuses by construction. Individually reviewed: re-running the identical classifier with only that payload exclusion lifted classifies the fixture as `flat-sum-wrapper` with no residue, so the delta is the wrapper removal alone. Exact regenerated tree pinned, no normalization. |
| `cll/chrestomathy/alice01.toml` | Excluded old-tree shape: the zero-continuation wrapper covers the `KeTermset` termset/GEK atom, which class (i) refuses by construction. Individually reviewed: re-running the identical classifier with only that payload exclusion lifted classifies the fixture as `flat-sum-wrapper` with no residue, so the delta is the wrapper removal alone. Exact regenerated tree pinned, no normalization. |
| `cll/chrestomathy/forest-nymph.toml` | Excluded old-tree shape: the zero-continuation wrapper covers the `ForethoughtTermset` termset/GEK atom, which class (i) refuses by construction. Individually reviewed: re-running the identical classifier with only that payload exclusion lifted classifies the fixture as `flat-sum-wrapper` with no residue, so the delta is the wrapper removal alone. Exact regenerated tree pinned, no normalization. |
| `corpus/alis/full-alice.toml` | Both the excluded VUhO payload shape and the digest-pinned refs projection. The tree is proven to be the class-(i) wrapper removal alone (audit below), and the refs digest moves for exactly the `corpus/camxes/2391` cause: Alice contains the same `lu ki'u ma na ku da'i sei la cibmasti cicyractu cu cusku li'u` sentence. Diffing the regenerated projections of both binaries gives 15 added frames and 0 removed, all inside that clause's byte range. `output.tersmu.json` is unchanged. |
| `corpus/camxes/11154.toml` | Failure-frontier residue on an input that rejects before and after: `syntax.incomplete-term@[64,64]` → `syntax.incomplete-selbri@[64,64]`. The frontier moves because the term tier no longer offers the retired connective alternatives. Exact diagnostics pinned. |
| `corpus/camxes/11856.toml` | Failure-frontier residue on an input that rejects before and after: `syntax.incomplete-term@[37,37]` → `syntax.incomplete-selbri@[37,37]`. The frontier moves because the term tier no longer offers the retired connective alternatives. Exact diagnostics pinned. |
| `corpus/camxes/12023.toml` | Excluded old-tree shape: the zero-continuation wrapper covers the `ForethoughtTermset` termset/GEK atom, which class (i) refuses by construction. Individually reviewed: re-running the identical classifier with only that payload exclusion lifted classifies the fixture as `flat-sum-wrapper` with no residue, so the delta is the wrapper removal alone. Exact regenerated tree pinned, no normalization. |
| `corpus/camxes/1451.toml` | Excluded old-tree shape: the zero-continuation wrapper covers the `ForethoughtTermset` termset/GEK atom, which class (i) refuses by construction. Individually reviewed: re-running the identical classifier with only that payload exclusion lifted classifies the fixture as `flat-sum-wrapper` with no residue, so the delta is the wrapper removal alone. Exact regenerated tree pinned, no normalization. |
| `corpus/camxes/16184.toml` | Failure-frontier residue on an input that rejects before and after: `syntax.unexpected-brivla@[31,36] "lerci"` → `syntax.unexpected-word@[31,36] "lerci"`. The frontier moves because the term tier no longer offers the retired connective alternatives. Exact diagnostics pinned. |
| `corpus/camxes/16271.toml` | Failure-frontier residue on an input that rejects before and after: `syntax.unexpected-brivla@[21,26] "jinvi"` → `syntax.unexpected-cmavo@[7,9] "bo"`. The frontier moves because the term tier no longer offers the retired connective alternatives. Exact diagnostics pinned. |
| `corpus/camxes/1692.toml` | Excluded old-tree shape: the zero-continuation wrapper covers the `ForethoughtTermset` termset/GEK atom, which class (i) refuses by construction. Individually reviewed: re-running the identical classifier with only that payload exclusion lifted classifies the fixture as `flat-sum-wrapper` with no residue, so the delta is the wrapper removal alone. Exact regenerated tree pinned, no normalization. |
| `corpus/camxes/16937.toml` | Excluded old-tree shape: the zero-continuation wrapper covers the `ForethoughtTermset` termset/GEK atom, which class (i) refuses by construction. Individually reviewed: re-running the identical classifier with only that payload exclusion lifted classifies the fixture as `flat-sum-wrapper` with no residue, so the delta is the wrapper removal alone. Exact regenerated tree pinned, no normalization. |
| `corpus/camxes/1984.toml` | Excluded old-tree shape: the zero-continuation wrapper covers a VUhO-carrying payload, which class (i) refuses by construction. Individually reviewed: re-running the identical classifier with only that payload exclusion lifted classifies the fixture as `flat-sum-wrapper` with no residue, so the delta is the wrapper removal alone. Exact regenerated tree pinned, no normalization. |
| `corpus/camxes/20100.toml` | Acceptance flip that **retires an xfail**: `ca le purlamnanca ku je ca le cabnanca ku na go'i` used a term-level JEK, which the corrected #795 domain rejects. The camxes corpus expected `failure` all along, so the fixture drops its `xfail` table, its accepted tree and its Gentufa renderings, and pins the new failure frontier exactly (`syntax.unexpected-cmavo@[27,29] "le"`). |
| `corpus/camxes/2029.toml` | Excluded old-tree shape: the zero-continuation wrapper covers a VUhO-carrying payload, which class (i) refuses by construction. Individually reviewed: re-running the identical classifier with only that payload exclusion lifted classifies the fixture as `flat-sum-wrapper` with no residue, so the delta is the wrapper removal alone. Exact regenerated tree pinned, no normalization. |
| `corpus/camxes/2033.toml` | Excluded old-tree shape: the zero-continuation wrapper covers a VUhO-carrying payload, which class (i) refuses by construction. Individually reviewed: re-running the identical classifier with only that payload exclusion lifted classifies the fixture as `flat-sum-wrapper` with no residue, so the delta is the wrapper removal alone. Exact regenerated tree pinned, no normalization. |
| `corpus/camxes/2038.toml` | Excluded old-tree shape: the zero-continuation wrapper covers a VUhO-carrying payload, which class (i) refuses by construction. Individually reviewed: re-running the identical classifier with only that payload exclusion lifted classifies the fixture as `flat-sum-wrapper` with no residue, so the delta is the wrapper removal alone. Exact regenerated tree pinned, no normalization. |
| `corpus/camxes/21600.toml` | Failure-frontier residue on an input that rejects before and after: `syntax.unexpected-brivla@[158,163] "finti"` → `syntax.unexpected-word@[158,163] "finti"`. The frontier moves because the term tier no longer offers the retired connective alternatives. Exact diagnostics pinned. |
| `corpus/camxes/2391.toml` | Semantic-projection residue with a verified cause: the syntax tree is the class-(i) wrapper removal alone, and the reference collector now reaches the free modifiers of the NA-KU term leaf, so the `sei la cibmasti cicyractu cu cusku` clause inside the fragment contributes its frames where the wrapper path produced none. Isolated by probe: `naku sei la djan cu cusku` yields 0 frames on base `a8b4f06227` and 4 here, while `mi klama sei la djan cu cusku` is unchanged at 9. Exact refs pinned. |
| `corpus/camxes/2467.toml` | Excluded old-tree shape: the zero-continuation wrapper covers a VUhO-carrying payload, which class (i) refuses by construction. Individually reviewed: re-running the identical classifier with only that payload exclusion lifted classifies the fixture as `flat-sum-wrapper` with no residue, so the delta is the wrapper removal alone. Exact regenerated tree pinned, no normalization. |
| `corpus/camxes/2481.toml` | Excluded old-tree shape: the zero-continuation wrapper covers the `ForethoughtTermset` termset/GEK atom, which class (i) refuses by construction. Individually reviewed: re-running the identical classifier with only that payload exclusion lifted classifies the fixture as `flat-sum-wrapper` with no residue, so the delta is the wrapper removal alone. Exact regenerated tree pinned, no normalization. |
| `corpus/camxes/2646.toml` | Excluded old-tree shape: the zero-continuation wrapper covers the `ForethoughtTermset` termset/GEK atom, which class (i) refuses by construction. Individually reviewed: re-running the identical classifier with only that payload exclusion lifted classifies the fixture as `flat-sum-wrapper` with no residue, so the delta is the wrapper removal alone. Exact regenerated tree pinned, no normalization. |
| `corpus/camxes/2661.toml` | Excluded old-tree shape: the zero-continuation wrapper covers the `ForethoughtTermset` termset/GEK atom, which class (i) refuses by construction. Individually reviewed: re-running the identical classifier with only that payload exclusion lifted classifies the fixture as `flat-sum-wrapper` with no residue, so the delta is the wrapper removal alone. Exact regenerated tree pinned, no normalization. |
| `corpus/camxes/2881.toml` | Failure-frontier residue on an input that rejects before and after: `syntax.incomplete-selbri@[7,7]` → `syntax.incomplete-sumti@[7,7]`. The frontier moves because the term tier no longer offers the retired connective alternatives. Exact diagnostics pinned. |
| `corpus/camxes/5290.toml` | Failure-frontier residue on an input that rejects before and after: `syntax.unexpected-brivla@[10,18] "zirbolci"` → `syntax.unexpected-word@[10,18] "zirbolci"`. The frontier moves because the term tier no longer offers the retired connective alternatives. Exact diagnostics pinned. |
| `corpus/camxes/5339.toml` | Failure-frontier residue on an input that rejects before and after: `syntax.unexpected-cmavo@[30,32] "cu"` → `syntax.unexpected-brivla@[23,29] "terpli"`. The frontier moves because the term tier no longer offers the retired connective alternatives. Exact diagnostics pinned. |
| `corpus/camxes/6378.toml` | Failure-frontier residue on an input that rejects before and after: `syntax.unexpected-cmavo@[22,24] "le"` → `syntax.unexpected-cmavo@[8,10] "na"`. The frontier moves because the term tier no longer offers the retired connective alternatives. Exact diagnostics pinned. |
| `corpus/camxes/644.toml` | Excluded old-tree shape: the zero-continuation wrapper covers the `ForethoughtTermset` termset/GEK atom, which class (i) refuses by construction. Individually reviewed: re-running the identical classifier with only that payload exclusion lifted classifies the fixture as `flat-sum-wrapper` with no residue, so the delta is the wrapper removal alone. Exact regenerated tree pinned, no normalization. |
| `corpus/camxes/811.toml` | Acceptance flip pinned as a **corpus witness of the #806 PEhE domain**: `la djeimyz. ce'e la meris. pe'e .e la djordj. ce'e la martas. prami` joins two CEhE termsets with `pe'e .e`, and the corrected PEhE level takes JOIK or JEK only. The fixture now pins the rejection at byte 34 and the semantic projection's matching error; camxes-exp still accepts it, which is the documented gap already recorded for `ko'a pe'e .e ko'e broda`. |
| `corpus/camxes/832.toml` | Excluded old-tree shape: the zero-continuation wrapper covers a VUhO-carrying payload, which class (i) refuses by construction. Individually reviewed: re-running the identical classifier with only that payload exclusion lifted classifies the fixture as `flat-sum-wrapper` with no residue, so the delta is the wrapper removal alone. Exact regenerated tree pinned, no normalization. |
| `corpus/camxes/833.toml` | Excluded old-tree shape: the zero-continuation wrapper covers a VUhO-carrying payload, which class (i) refuses by construction. Individually reviewed: re-running the identical classifier with only that payload exclusion lifted classifies the fixture as `flat-sum-wrapper` with no residue, so the delta is the wrapper removal alone. Exact regenerated tree pinned, no normalization. |
| `corpus/camxes/846.toml` | Excluded old-tree shape: the zero-continuation wrapper covers the `NuhiTermset` termset/GEK atom, which class (i) refuses by construction. Individually reviewed: re-running the identical classifier with only that payload exclusion lifted classifies the fixture as `flat-sum-wrapper` with no residue, so the delta is the wrapper removal alone. Exact regenerated tree pinned, no normalization. |
| `corpus/camxes/847.toml` | Excluded old-tree shape: the zero-continuation wrapper covers the `NuhiTermset` termset/GEK atom, which class (i) refuses by construction. Individually reviewed: re-running the identical classifier with only that payload exclusion lifted classifies the fixture as `flat-sum-wrapper` with no residue, so the delta is the wrapper removal alone. Exact regenerated tree pinned, no normalization. |

## Scope delivered, and the epoch 6b boundary

This round — epoch 6a — delivers the composed ladder and the dispositions that
depend on it. It closes #792 (level composition, including the standard CEhE
flavour divergence), #795 (the term connective domain), #796 (route removal),
and #816 (the BO tier's stag policy: mandatory in the absorption-safe flavour,
optional at BE/BEI links), and it discharges the epoch-4 VUhO residual.

#806 is **half** delivered and stays open. Its PEhE half — the JOIK/JEK domain
of the PEhE level, its operand level, and the CEhE token site — is in this
round; its standard-termset half is epoch 6b. #794 and #827 are entirely
epoch 6b.

| Epoch 6b | Issue | Why it is not here |
| --- | --- | --- |
| NUhI-less `gek_termset` rebuild: balanced `nonabs (gik / recursion) nonabs` operands, no NUhU slots, with a whole-candidate baseline-gek-sumti classifier | #806 (stays open) | Needs a new recursive termset node and the camxes-exp normal-flavour ladder as its operand level, plus semantic lowering for both. `nu'i` termsets keep their guarded `terms` operands, which is already the sourced NUhI-present shape; `ge ko'a gi pu broda` therefore still rejects. |
| GOI payload width across the three profiles, and the `abs` axis on the payload flavour | #794 | Needs the normal-flavour ladder threaded through the whole relative-clause subtree in place of the narrow `relative_sumti` family. |
| Zantufa connectorless BO at the term and sumti tiers, the JAI structural predicate, FA joik-chains, and the Zantufa GOI payload | #827 | Depends on the same normal-flavour ladder plus its own whole-candidate classifier. |

### Why 6b does not rewrite the expectation shapes this round re-levels

The consolidated regeneration below re-levels every term in the corpus. That
work is not provisional, because the deferred sections add arms and operand
levels at positions the ladder does not currently reach; they do not renumber it.

*The ladder's levels and their leaf inventories are fixed.* Mechanism E means a
level re-lists leaves rather than nesting the level below it, so adding an arm to
a level adds one variant to that level's inventory and changes the Debug shape of
no tree that does not select the new arm. 6b adds arms — a recursive
`gek_termset` operand, connectorless BO at the term and sumti tiers, a widened
JAI atom — and each of them is placed at the baseline precedence level it already
belongs to, which is the rule this round's own `stag_bound_term_connection`
placement follows.

*Every 6b arm is unreachable on a surface that parses today.* The NUhI-less
`gek_termset` arm only engages where a GEK sits at a term position with no NUhI,
which this round rejects (`ge ko'a gi pu broda`); Zantufa's connectorless BO only
engages on a BO with no connective, which every profile rejects here
(`pu ko'a bo ca ko'e broda`); the JAI predicate only widens where the current
overt-sumti requirement fails. Each therefore turns a rejection into an
acceptance rather than re-owning an accepted tree, and each lands with its own
whole-candidate classifier proving exactly that.

*The one subtree 6b does re-shape is not this round's.* #794 replaces the narrow
`relative_sumti` payload of a GOI relative with a full normal-flavour term. That
changes GOI payload expectations, which this round does not re-level: the
regeneration's term positions are the 25 consumer sites plus the PEhE, CEhE, BO
and link operands, and a GOI payload is none of them. `lo broda goi ko'a ku`
keeps the tree it has here.

*The ownership question 6b could reopen is already ledgered as a gap, not as a
pin to revisit.* camxes-exp's inner VUhO attachment stays a documented fidelity
gap under the standing reinterpretation ruling, as does Zantufa's `ce'e`-as-BO
lexing, which 6b inherits with #827. Adopting either changes the meaning of a
surface that already parses, so by that ruling it arrives behind a
meaning-changing dialect flag rather than by re-pinning the baseline
expectations regenerated here.

## Verification

Release mode, on the implementation host, at the submitted tree.

| Gate | Result | Log |
| --- | --- | --- |
| `cargo test -r --workspace` | 2,310 passed, 0 failed, 16 ignored | `epoch06-gate-workspace-tests4.log` |
| `fixture-test --profile all` | 26,446 fixtures, 4 facets, 73,733 passed, 514 xfailed, **0 failed** | `epoch06-gate-fixture-profile.log` |
| Expensive contracts, all targets, release | 2,331 passed, 0 failed | `epoch06-gate-expensive-contracts.log` |
| `semantics-coverage` | checked 22,608, panics 0, unsupported 0 | `epoch06-gate-semantics-coverage.log` |
| Debug `jbotci` build | green | `epoch06-gate-debug-jbotci.log` |
| Debug `dx build` | green | `epoch06-gate-debug-dx.log` |
| Four Python generated checks | all green | `epoch06-gate-generate_*.log`, `epoch06-gate-compose_stubs.log` |
| `cargo fmt --all --check` | clean | `epoch06-fmt4.log` |
| Frozen tagged syntax facet | 60/60 | — |
| Comparer, ratcheted | 18,249 changed / 18,192 / 5 / 0 / 13 mechanical / 49 manual, prose-only provenance edits 2, epoch-witness T3 re-pins 9, witness deltas 0, unpaired 0 | `epoch06-comparer-round3.txt` |
| Peak RSS, full profile | base 5,738,932 KB → 5,774,252 KB, **+0.62%** (gate +20%) | `epoch06-gate-fixture-profile*.log` |
| Artifact ratchet | archive +0.67%, unpacked +0.92% versus a base-built control | `epoch06-artifact-ratchet.log` |

The xfail count moves 515 → 514 because `corpus/camxes/20100`'s xfail retires.

Four breakages left by the earlier commits are fixed here rather than deferred:
`cargo fmt` in three `jbotci-semantics` files; the stale `su'i` case in
`nonlogical_direct_term_connections_are_principled_errors`, which #795 turned
from an undefined lowering into a syntax rejection; three stale and one missing
invariant-audit allowlist row for the deleted `GeneratedDirectTermConnective` /
`GeneratedDirectTermOperand::Simple` variants and the new
`GeneratedTaggedTermRef`; and the recovery-anchor metadata snapshot, whose delta
is exactly this epoch's rule rename and narrowed connective inventories.

Both wheels in the artifact row were built natively as manylinux 2.34, because
local container tooling could not produce a 2.28 artifact on this host. The base
control lands within 0.01% of epoch 5's container measurement, which is what
makes the delta meaningful; `artifact-policy.toml` records the same caveat, and
the python-wheels workflow remains the acceptance authority for the 2.28
artifact.
