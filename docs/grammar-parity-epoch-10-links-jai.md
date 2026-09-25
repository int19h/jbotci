# Grammar parity epoch 10: links, JAI, MEhOI and the Zantufa atom inventory

Epoch 10 of the grammar-parity epic (#801) covers six parity issues:
- #793: BE/BEI links take a full term;
- #807: empty BE/BEI payloads removed;
- #808: standard linkarg placement and complete JAI recursion;
- #820: sourced MEhOI tanru units;
- #831: the missing Zantufa tanru-unit and sumti atoms;
- #834: the Zantufa `selbri_2 KEhE linkargs` owner.

The branch is `epoch-10-links-jai`, with merge base 146554db1e. It also merged two recovery-driver fixes from main during the epoch: #927, the final-selector fallback, and #930, resume-at-end conservation.

## Scope, and the re-scope of 2026-09-21

The epic's acceptance criteria are:
- strict-parse source equivalence with camxes-standard;
- an additive, provenance-checked warning union;
- the Zantufa PEG as the reference for the Zantufa profiles.

Recovery is explicitly outside that goal.

The frozen plan (plan-v8, 553c774c63) had nonetheless mandated a recovered-parse contract:
- 227 frozen recovered rows;
- a "recovered witness law" requiring a strict/recovered classifier twin for every ownership decision;
- sealed measurement arms, ledgers, and a fail-closed comparer as a gate.

From 2026-09-14 the lane spent most of its time on that contract. On 2026-09-21 the PM re-scoped the epoch to strict parity only (`~/artifacts/jbotci/epoch10-ce-recovery/epoch10-rescope-2026-09-21.md`):
- **Acceptance.** Source-shaped fixtures for the accepted, rejected and grouping boundaries of each issue, against camxes-standard and the Zantufa PEG, with the reference parsers as the oracle; the additive-warning invariant (no successful baseline parse changes its tree); and a green full fixture profile.
- **Dropped.** The frozen recovered-row contract, the classifier-twin witness law, the sealed arms, the ledgers, and the comparer as a gate.
- **Recovered expectations.** These stay on the adhoc fixtures as regression pins. They are refreshed at consolidation (C-g) from actual output, and every movement is classified, as described below.
- **Architecture audit.** One was required before C-f. The branch had added a family of tree-inspecting ownership classifiers, and parity must come from grammar rules wherever a PEG can state them.

The collapse-driver defect the lane found was real and pre-existing on main. It was fixed on main as #927 and merged back (see "Recovery-driver fixes from main").

## The work, issue by issue

### #793 (C-a): BE/BEI links accept a full term

a05bd77220 adds a guarded full-term-first route for BE/BEI payloads next to the complete legacy link owners.

The guard is `LegacyLinkPayloadRejection`, an ordered-choice difference: the new full-term arm must not claim a shape the legacy link owners already produce. It is kept (see "The KEEP category"). A PEG cannot state a language difference over the whole term hierarchy without duplicating that hierarchy.

### #807 (C-b): empty BE/BEI payloads removed

d79e147507 removes `Empty` from both linked-argument hierarchies and from their Rust and Python consumers, and keeps the mandatory fields.

This produced 15 strict success→failure flips, and every one is correct parity:
- **Eight camxes-corpus entries** are English text ("to be", "befunge", "be.") that parsed only through the empty-BE hack.
- **Six adhoc fixtures** pinned the old empty-link behaviour.
- **`corpus.alis.full-alice`** now fails at `panzi be ny ci mei`, which camxes-standard rejects. Main's "success" there parsed an empty `be` and read `ny ci mei` as a MOI unit, which is a wrong tree. See "Alice" below.

At C-g, seven more corpus.camxes rows moved their strict first error onto the link payload for the same reason. They are listed under "Expectations re-pinned at C-g", class H.

Recovered structural-warning parity was deferred to #891.

### #808 (C-c): standard linkarg placement and complete JAI recursion

a19f165392:
- removes the JAI mini-family, the `JaiInner*` model types;
- makes JAI recurse over the shared tanru-unit atom;
- keeps CEI outside preposed linked atoms;
- retypes the preposed link's base to `LinkedTanruUnitSyntax`.

The semantic consumers are reused rather than duplicated.

### #820 (C-d): sourced MEhOI tanru units

5094feaf95 restores direct MEhOI predicate atoms. It removes the unsourced quoted-sumti ownership and with it the `ExperimentalMehoiCompoundQuote` model types.

### #831 (C-e): the Zantufa atom inventory

C-e adds the atoms the Zantufa PEG defines and jbotci lacked (`zantufa-1.9999.peg:59,75,94`):
- the FA-recursive tanru unit `(FA (joik FA)*) tanru_unit_1`;
- the GEK forethought tanru unit `NAhE? gek selbri_2 (gik selbri_2)+`;
- the atomic KE sumti `KE sumti KEhE?`;
- JAI recursion over that inventory.

Where these atoms overlap existing routes (FA terms, GUhA forethought, forethought-bridi, KE termsets), ownership is decided as follows:
- **Standalone GEK atom.** Adjudicated ownership tables, a strict-language partition of complete products that depends on dialect.
- **Enclosed GEK atom under JAI.** A GA-opener test.
- **Atom-bearing level-2 selbri.** A feature-gated priority route (D5), with a priority-tail refinement.
- **The grouped sumti at term position.** It requires an explicit KEhE.
- **Two negative lookaheads.** `!(sumti kehe)` and `!ke_termset` under ZantufaTerms. They are expressed with `strict_observe`, which runs its probe strictly and discards memo, recovery and diagnostic effects.

C-e also repaired strict memo replay of diagnostics (see "Diagnostics"). It added the `JBOTCI_TRACE_ZANTUFA_ATOMS` trace, which prints one line per recovered classification. It is gated debug output, useful for attributing a recovered tree to an atom decision.

### #834 (C-f): the Zantufa `selbri_2 KEhE linkargs` owner

Zantufa's first `selbri_1` alternative closes a whole level-2 selbri with an unmatched `ke'e` and links arguments to all of it:

```
selbri_1 <- (!KE selbri_2 KEhE_clause linkargs / selbri_2) relative_clauses? (CEI_clause selbri)*
```

(zantufa-1.9999.peg:45.) C-f implements it as ordinary grammar on the ZantufaSelbri axis, with the source's `!KE` guard and no classifier.

`zantufa_kehe_linked_selbri` holds:
- the level-2 selbri;
- the warned `ke'e` (ExperimentalZantufaKeheLinkargs);
- the linkargs;
- optional selbri-level relatives;
- the full-selbri CEI assignments.

Where it sits:
- **First gated variant of `untagged_selbri`.** This lets tags and NA reach it.
- **First in `zantufa_selbri_entry`, ahead of the D5 priority arm.** That arm completes an atom-bearing level-2 selbri and commits, which would leave `ke'e be …` unparseable.
- **One recursive handle per ladder.** This makes the second reach a memo hit.
- **Descriptions.** They use a second product, `zantufa_kehe_linked_selbri_without_terminal_relative`, which has no relatives and a restricted final CEI operand.

A recovered-only rejection (see `reject_recovered_output` below) stops recovery from claiming the owner on a `ke'e`, BE or link payload it did not parse. Without it, recovery claims the owner on synthesized content in 12 probe rows, for example a synthesized `ke'e` before ordinary tanru-unit linkargs (`mi broda be ko'a be'o ku`, pinned as `cf-recovered-synthesized-kehe`).

**Order-independence measurement.** In a measurement build I swapped only the owner's arm and the D5 arm, then re-parsed every C-f row on every pinned axis. Only the brodi-GEK row breaks: `mi brodi ga broda gi brode ke'e be ko'a be'o` on `+zantufa-selbri +zantufa-selbri-reinterpretation`. The leading FA and GEK rows never produce an atom-bearing level-2 selbri in jbotci (see #933 below), so they are unaffected. The atom-bearing set jbotci actually has breaks exactly as predicted, and no other row breaks.

**Reference parity.** All 99 C-f fixtures record their Zantufa 1.9999, camxes and camxes-exp result. Where jbotci accepts, the linked extent matches Zantufa's (selbri_2 before `KEhE_clause`, and the linkargs), with these exceptions:
- **CEI after the linkargs** needs ZantufaTerms, because `zantufa_selbri_assignment` asserts it; that is the C-e feature allocation. These rows fail on `+zantufa-selbri` alone and agree on `+zantufa-selbri +zantufa-terms`.
- **A GEK unit after a brivla** is reinterpretation-gated, as in ce-gr-s9.
- **Leading `fa` / `ga … gi`**: see #933 below.
- **A tag with a selbri-level relative** (`mi pu broda poi mi brodi ku'o`) is #932, outside C-f, and not fixtured.

## Dialect overlaps: what the union keeps, and why

The owner's standing ruling on Zantufa reinterpretation: where Zantufa reads an input that the baseline (camxes-standard) also accepts, and the readings differ, the union keeps the baseline reading, and a dedicated reinterpretation flag carries the faithful reading.

The epoch has cases on both sides of that line:
- **`mi fa broda …` and `mi ga broda gi brode …`** on the built-in `zantufa` axis. camxes-standard accepts `mi fa broda` as `[mi fa] broda` (a FA term with elided KU), and `mi ga broda gi brode` as a bridi-tail connection. So the union keeps those readings, and the C-f owner then links only the selbri after the prefix. That is correct union policy, not a defect. The gap is on the reinterpretation axis: with `zantufa-selbri-reinterpretation` on, Zantufa's reading (FA / GEK inside tanru_unit_1) should win, and it does not. That is **#933**.
- **`fa je fe broda`** on the `zantufa` axis. camxes-standard rejects it, so there is no baseline parse to protect, and on the Zantufa axis the dialect's source reading wins: C-e's FA atom, `(fa [je fe] bróda)`. The default configuration keeps camxes-exp's FA-as-tag reading. `adhoc.syntax.terms.zantufa-fa-joik-chain-selbri-boundary` states this contrast.
- **Zantufa selbri-level routes under a tense/modal tag** are unreachable, because `tagged_selbri`'s inner is `untagged_selbri`, not Zantufa's full selbri. That is **#932**.

## Diagnostics

### Strict memo replay (C-e, design R3)

Before C-e, a strict memo hit fabricated a failure with no diagnostic observations. So a rule's rolled-back diagnostic candidates, recorded when the rule was first evaluated, were lost on every later memo hit. The reported first error was then not the true farthest failure.

C-e records observations on every memo frame and replays them on a hit. The reported error is now the PEG farthest failure, which moves later and never earlier.

28 corpus.camxes rows pinned the old, lost-candidate errors. They are re-pinned at C-g (class F). A control build of 5094feaf95 with strict memo lookups disabled reproduces the head's first error on 26 of them; the other two time out without memoization.

A companion test, 3d4e4097e9, makes the memo-replay equivalence test non-vacuous across a diagnostic truncation that actually runs. The restore path truncates frame observations, and the earlier test could not observe that.

### The expected-alternatives contract

This contract is binding; it was settled while investigating the diagnostics regressions R1 and R2.
- **Completeness.** It is required of the underlying expectation set and of the detailed NOTE. The one-line summary may be a scoped summary of it. No rendering may be the only surface a user sees; the complete set is always adjacent.
- **Naming (granularity rule).** An expected alternative is named by the rule whose next unconsumed element is being expected, never by an ancestor whose earlier fields are already consumed. Both renderings use this rule. Under it, epoch 10's `term connection continuation` is correct where main said `term connection`, which is the parent construct.
- **The scope filter.** `syntax_expectation_summary_constructs` restricts the one-line summary to scope-relevant constructs, free modifiers and end of input, but only when at least one scope-relevant construct is present. It is a deliberate feature and exists only on the one-line path; `syntax_detailed_segments` renders the unfiltered set. So a single correct scope-relevant alternative can collapse a 15-item one-line summary to 3 items.
- **The summary/note divergence.** On `mi ku i do ku i mi klama` (with the #926 union applied) the one-line summary had 3 items against 91 in the note. That is pre-existing presentation behaviour, recorded here so it is not rediscovered as a defect.
- **Main's note omits constructs.** At that position main's note leaves out constructs that demonstrably parse there (`la .djan.`, `lo broda cu`, `noi broda ku'o`). Main's 16-bullet note is not a baseline to restore.

### Same-position selection (#926)

`select_parser_error` keeps one of several errors recorded at the same position and discards the rest, so which truthful alternative is offered depends on rule order. Commit 54f9c378f2 replaced that with a deduplicated union. It was extracted to #926 in 828a1fcc0d, because it is not this epoch's work and it exposes a separate pre-existing defect. `zantufa_raw_mekso_quantifier` has no leading marker and its guard does not exclude `ku`, so at a term position the whole mekso grammar is attempted, and every alternative records an expectation, including constructs that cannot be written there.

Separating truthful alternatives from unsatisfiable ones needs information the parser does not record. That is design work.

Epoch 10 changes the survivor at a few same-position sites, which is the order dependence #926 exists to remove:
- 23 refs errors (class I);
- 7 same-span code changes (class G);
- the CLI constant's `tanru unit (ZANTUFA-SELBRI feature)` entry (5e23ad704a).

### Explored and rejected: a completeness sink

An expectation sink (`sink-implementation.patch`, sha256 4b10f3481f9cfdceb5c2b08e29edc1402e7e0579e7cc4d0d76b71891e4a4538a) was built to collect every alternative tried at a failure position. It changed nothing observable: the reported expectation set was already complete, so a completeness sink was redundant. It was deliberately not committed anywhere.

### A standing hazard: strict-observe must own every piece of parser state

`strict_observe` suspends the parser's state for the duration of a probe by moving it into a `StrictObserveJournal`, then restores it. The journal holds:
- the memo stores and in-progress sets;
- the recovery trial;
- the memo rule frames;
- the diagnostic, replay and applied journals;
- the diagnostic and continuation candidates;
- warnings;
- the recovery counters and the active directive;
- the checkpoint collection;
- the continuation state.

Any new piece of diagnostic or recovery state added to `ParserState` must be added to the journal too. Otherwise a strict-observe probe leaks into, or is steered by, its parent parse. Nothing checks this mechanically; it is an invariant to keep in mind when extending `ParserState`.

## Recovery

### `reject_recovered_output`: a recovered-only rejection, and its contract

The classifier-layer audit added one DSL form for "recovery may not hand a construct a claim it did not parse": `.reject_recovered_output(X)`.
- **Strict flavour.** It lowers to its receiver unchanged, so the strict language is unaffected by construction, and the strict spine loses a combinator.
- **Recovered flavours.** It lowers to the shared `reject_output` mechanism through `RecoveredOutputRejection::rejects_uncertain`.
- **The contract.** `rejects_uncertain` carries `#[expensive_ensures(!ret || carries_recovery_uncertainty(value))]`: it may reject only a value with a recovery item (skipped, invalid or synthesized content) somewhere in its subtree. Rejecting a fully parsed value would be a strict-language decision made only in recovery. The expensive-contracts gate exercises this across the corpora.

Its users:
- **The FA atom and the grouped sumti.** Their strict hooks were tautologies (the product rules already state the inventory) and were deleted. At C-g their recovered classifiers were collapsed into one `recovered_inventory_presence`, defined exactly as "Unproven iff the candidate carries recovery uncertainty".
  - The old classifiers also answered `Absent` on a wrong-class token. No parse produces that: a bounded search of 15,596 damaged inputs never did, and the contract declares such hand-built values out of bounds. So the branch was removed rather than kept untestable.
- **C-f's KEhE-linked owner.**

**The engine-level question.** If "a recovered candidate that carries recovery uncertainty and no parsed token of its own does not complete as that construct" held for every rule, the per-construct recovered twins would be local patches over one engine rule. The remaining twins (Enclosed, Standalone, Priority, GroupedSumtiTerm) combine that test with real ownership decisions. For example, a fully parsed GI-opener product is rejected by the enclosed twin. So they stay `reject_output`, and the engine rule is not attempted in epoch 10.

### The KEEP category

The audit's rule: where a hook post-filters a broader parse to express something the grammar could state, the grammar states it. Hooks that could not be restated are kept, each with its reason:
- **Enclosed GEK atom, Priority atom and Priority tail.** The category: a tree-transparent eligibility refinement over a memoized product, shared with another route at the same position. Restating it as grammar would instantiate the product twice and add recursive handles on the descent path (#923), losing memo sharing.
  - Enclosed is reduced to the GA-opener variant test, with the head class as an expensive precondition.
  - The Priority arms are feature-gated (`feature(ZantufaSelbri).ignore_then(…)`), so on axes without ZantufaSelbri the route is absent rather than attempted and rejected. An absent route must not steer recovery or diagnostics, and gating it returned ce-sr2-terms and the CLI label to main's output.
- **Standalone GEK atom.** Adjudicated ownership tables, a dialect-dependent strict-language partition of complete products. A PEG cannot state that partition over a completed product without re-deriving it per cell.
- **LegacyLinkPayload.** An existential property over the connection spine ("some operand is a new-width term"). A PEG cannot state a language difference over the whole term hierarchy without duplicating it.
- **ZantufaGroupedSumtiTermRejection.** A property of the whole completed sumti at term position: a bare grouped sumti whose KEhE is elided. Stating it structurally means parameterising the whole sumti descent chain for the term site.
- **`strict_observe` ×2.** PEG negative lookaheads, not post-filters.

### Recovery-driver fixes from main

- **#927, the final-selector fallback (D1).** A first-phase candidate list whose trials were all rejected never reached the final selector, although an empty list did, so such parses collapsed to a single invalid item. All 21 collapse-pinned rows of the 2026-09-21 provenance ruling were frozen defects; 17 of them recover under D1.
- **#930, resume-at-end conservation.** A resume-at-end skip now consumes exactly the tokens it claims (no double-counted tokens), which removed zero-width invented markers from 18 lane rows.

### Known recovery gaps, filed and pinned as change detectors

The policy for a known gap is to pin its current output, with provenance naming the issue. The fixture then works as a change detector: nothing is silently baked in, and nothing is xfailed.
- **#925.** The final selector is consulted and exhausted: every candidate is rejected. This covers ce-sr2-zantufa: C-e's grouped-sumti rejection works, and the remaining recovery is a KE bridi-tail group with a missing body that loses `broda`.
- **#935.** A late natural-stop success is kept only `if !directives.is_empty()`, so at the first error it is discarded and the parse degrades, although the driver's own comment says such a success is preferable to degrading. This is the same shape of asymmetry as #927, and main shares it.
  - C-e exposed it on four epoch-9 descriptor witnesses (`d3c-recovered-r2-*`, `d3c-recovered-r4-*`). Its added checkpoints displace the winning `descriptor_with_outer_quantifier_sumti` candidate to position 31 or 41 of the final list, past the exact-phase trial budget of 8. The base tried it fourth.
  - It is not fixed here: a driver-policy change needs its own PR with the full differential, conservation and the expensive gate.
- **#934.** On Zantufa axes, main's recovery keeps a bare `ku` behind a synthesized missing sumti, `[‼‼ ku]`, inventing a description the input never started. 20 re-pinned rows cite it.
- **#931.** Resume-at-end skips swallow the enclosing construct's closer.
- **#928.** The final-selector fallback costs up to 3.7× on inputs that still collapse.

## Generated model naming

`EnumBranch::field_name` names every enum variant's public field for the product rule whose type the branch yields, never for the parser arm or alias that reaches it. The lane had leaked 8 alias names into the public model, including a rename of main's published `jai_modal_tanru_unit` to `jai_modal_tanru_unit_candidate`. All 8 now carry construct names, with 0 renames against main.

Main already published 4 alias-named fields: three quantifier `*_candidate` arms, and `bridi_relative_clause`'s `statement_relative_clause`. Each is pinned with an explicit `arm as field` override, the only way to depart from the rule. Renaming them is an API decision pending with the owner.

At C-g, `validate_unique_rules` also rejects two rules that denote the same syntax type (`foo_bar` / `foo__bar` → `FooBarSyntax`). That makes `field_name`'s choice unambiguous by construction; its precondition states this.

## Python API: break notice

Compared with origin/main, the regenerated Python models remove 22 whole classes. At the audit head (1a059274a6) they added 234 public names; C-f adds its own since. There are no renames inside surviving classes. The removals come from three reviewed model changes:
- the `JaiInner*` family, 17 classes (#808, shared JAI atom recursion);
- `EmptyLinkedSumti`, 3 classes (#807, empty BE/BEI payloads removed);
- `ExperimentalMehoiCompoundQuote`, 2 classes (#820, direct MEhOI predicate atoms).

## Alice

`corpus.alis.full-alice` (run under `(case-insensitive zantufa)`) stays a strict failure until #924. It contains `panzi be ny ci mei`, which is correct Zantufa: `be ny` followed by the tanru unit `ci mei`. Standard camxes and every jbotci profile keep the mixed lerfu-string form `lerfu_string <- lerfu_word (PA / lerfu_word)*`, so `ny ci` forms one string and the phrase is rejected. Main's earlier "success" was a wrong tree through the empty-`be` defect (#807).

The fixture regains success once #924 provides the split number/lerfu reading under a reinterpretation flag. The vendored text is not edited.

## Expectations re-pinned at C-g

At 92fedd55cd the full fixture profile had 128 failing facets, all attributed.

**Attribution.** I made 17 release builds along the lane's first-parent history (base, #793, #807, #808, #820, C-e, its follow-ups, the union and its extraction, the main merges, the audit, #930, C-f). Each failing row's input was probed on its axis, and the row was attributed to the interval where its output last changed after its fixture was pinned.

**Controls.**
- A memo-disabled 5094feaf95 build confirms class F.
- The harness at 29d7f6ae93 and at 4ab3ce5870, run against the current pins, isolates 14e2009218 for class D.

**Re-pinning.** Every re-pin comes from the head's complete regenerated dump and changes only fields that exist and fail. fixture-rewrite would also add never-pinned keys, drop comments and re-quote keys; all three were excluded. Every class was validated by script, and the final result validated as one set.

| class | rows | mechanism |
|---|---|---|
| A | 17 | #927: collapse → recovery (the provenance ruling's FD-pre rows) |
| B | 18 | #930: an invented zero-width missing marker removed |
| C | 1 | ce-gr-s7-zantufa raw digest: an empty IStatementConnection artifact became StatementBase |
| D | 16 | 14e2009218: the JAI-term reservation no longer drags the reported failure and anchor on Zantufa axes |
| E | 4 | lr-be/bei zantufa rows: the head equals the pre-epoch base exactly |
| F | 28 | C-e strict memo replay: the true farthest failure (camxes) |
| G | 7 | C-e: same span, a different winning context (#926 class) |
| H | 7 | #807: the first error moves to the link payload (camxes) |
| I | 23 | C-e: the surviving same-position alternative in refs errors (#926 class) |
| J | 4 | #935, pinned at current output with provenance |
| K | 1 | `fa je fe broda`: the FA atom reading on the Zantufa axis |
| Q1 | 1 | ce-sr2-zantufa, #925 |
| s9 | 1 | ce-gr-s9-zantufa: the head equals the pre-epoch base |

Rows that needed individual treatment:
- **Seven Prefix witnesses** (ce-gr-e7, ce-gr-s8, ce-gr-s10, ce-sr1b, ce-sr5, ce-zr-j6, ce-zr-s6, all on the -zantufa axis). They are the only coverage of the kept classifiers' behaviour under a Prefix-wrapped containing field. Their Prefix came from recovery skipping a trailing bare `ku`, which main's recovery now keeps (#934).
  - Only the trailing damage changed, `ku` → `ku'o`, measured per row over 18 candidates.
  - The head's raw tree again has a Prefix on `paragraphs.first`, and every traced decision the provenance states still holds.
  - Their -default, -selbri and -terms siblings keep `.i ku`.
- **The IDE provisional-diagnostic gate.** `trailing-operator-mex-quantifier` left the reviewed set. Since C-e (7a18ce022d), recovery reads its `su'i` as the experimental VUhU connective on a parsed token, which complies with the recovered-claim policy but carries a local experimental warning that the gate must reject.

The full fixture profile at c7e7946211: 27,248 fixtures, 73,295 passed, 509 xfailed, 0 failed, 7,940 skipped. The delta is exactly the 128 re-pinned rows.

## Harness notes

- **dx asset paths.** Release `dx build --web` now emits hashed assets under `public/assets/` (`jbotci-app-dxh*.js`, `jbotci-app_bg-dxh*.wasm`) instead of `public/wasm/`; the debug build still writes `public/wasm/`. Identify a release bundle by the revision embedded in its wasm.
- **The web debug build and jbotci-syntax.** `profile.wasm-dev.package.jbotci-syntax` now has `debug = false`. Even line tables name every monomorphized function in `.debug_str`, and the generated grammar's combinator types make those names enormous.
  - Before the fix, on main, the post-bindgen debug module was 1.66 GB. That is past V8's 1 GiB module limit, although it still linked.
  - The epoch's grammar pushed the merged `.debug_str` past 2^31, and rust-lld 22.1.2 crashed with SIGSEGV while linking.
  - With the fix, the module is 139 MB and loads in V8.
  - Cargo does not read package overrides from `CARGO_PROFILE_*` environment variables. The escape hatch is `--config 'profile.wasm-dev.package.jbotci-syntax.debug="line-tables-only"'`.
- **Python tools on aarch64.** Build with `CARGO_PROFILE_DEV_DEBUG=0` and a separate `CARGO_TARGET_DIR`. Debug info overflows R_AARCH64_ABS32 `.debug_str` relocations when linking `jbotci-python-api-parity`, and the full debug tree is large.
- **Test runs.** `cargo test` stops at the first failing target, so always pass `--no-fail-fast`. In piped gate scripts, take the exit code from `${PIPESTATUS[0]}`.
- **fixture-test.** `--path` without a profile selects nothing; use `--profile all --path-prefix`.
- **fixture-rewrite.** It fills only keys that are present. It adds keys that were never pinned, drops comments and re-quotes keys, so re-pin field by field and validate against the parsed TOML.

## Gate

To be completed on the final candidate. It runs once:
- the release expensive-contracts gate (all targets);
- the full fixture profile;
- `cargo test -r --workspace`;
- the debug `jbotci` build and the debug `dx build`;
- cliff.py;
- the four Python `--check` tools.

## Follow-up issues filed by this epoch

- #924: Zantufa/camxes-exp split number and lerfu-string productions (Alice).
- #925: the final-selector fallback exhausted (four Zantufa rows, and ce-sr2-zantufa).
- #926: expectation reporting, the same-position union and the unsatisfiable-alternative problem.
- #928: the final-selector fallback costs up to 3.7× on inputs that still collapse.
- #929: CI, the Python wheels "Generated syntax and stubs" job dies mid-build.
- #931: resume-at-end skips swallow the enclosing construct's closer.
- #932: Zantufa selbri-level routes unreachable under a tense/modal tag.
- #933: ZantufaSelbriReinterpretation does not cover leading FA/GEK tanru atoms.
- #934: recovery policy for a bare `ku` kept behind a synthesized missing sumti.
- #935: the recovery driver discards a late natural-stop success at the first error.
