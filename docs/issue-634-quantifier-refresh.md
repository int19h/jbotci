# Refreshing the #634 baseline-quantifier fix onto current main

PR #843 carried the accepted fix for #634 — the Zantufa profile reporting
ordinary CLL quantifiers as experimental mex — at exact head `64ea0aa8`, on
branch `issue-634-zantufa-baseline-quantifier` branched from `23dda1c037`
(the #798 merge). It was deliberately held out of epochs 6b through 8 for
epoch-9 integration. By the time it was picked back up, main had moved 119
commits, and the exact-head acceptance was stale in every part of the change
except the defect it fixes.

This note records the re-derivation on branch `issue-634-refresh`: what the
fix's semantic content is, what it was measured against, and how every class
of divergence between `64ea0aa8` and main was classified and resolved. It is
the conflict ledger for that rebase, not a redesign of the fix.

## The defect is unchanged and still live

Measured on `0d791fd35c` (the #871 merge) with a release `jbotci` built from
that commit, `jbotci gentufa --dialect '(zantufa)'`:

| source | warning at base | ownership at base |
| --- | --- | --- |
| `tirna re cmalu se krixa` | `experimental-zantufa-mex` at `re` | `ZantufaPriorityRawMeksoQuantifier` |
| `tirna re boi cmalu se krixa` | `experimental-zantufa-mex` at `re` | `ZantufaPriorityRawMeksoQuantifier` |
| `tirna vei pa su'i re ve'o cmalu` | `experimental-zantufa-mex` at `vei` | `ZantufaPriorityRawMeksoQuantifier` |
| `tirna vei pa su'i re cmalu` | `experimental-zantufa-mex` at `vei` | `ZantufaPriorityRawMeksoQuantifier` |
| `re` (bare fragment) | `experimental-zantufa-mex` at `re` | `ZantufaPriorityRawMeksoQuantifier` |
| `vei pa ve'o` (bare fragment) | `experimental-zantufa-mex` at `vei` | `ZantufaPriorityRawMeksoQuantifier` |
| `tirna vei pa bo re ve'o cmalu` | two, at `vei` and at `bo` | `ZantufaPriorityRawMeksoQuantifier` |
| `tirna pa su'i re cmalu` | one, at `pa` — genuine | `ZantufaPriorityRawMeksoQuantifier` |
| `re moi broda`, `re roi klama` | none | not a quantifier |

`quantifier` is still the four-alternative sum
`zantufa_priority_raw_mekso_quantifier | mekso_quantifier | pa_run_quantifier |
zantufa_raw_mekso_quantifier`, with the priority raw alternative ordered first
under `feature(ZantufaMex)`. The warning itself is emitted post-parse by
`GeneratedConstructWarningVisitor`, which warns on the presence of a
`QuantifierSyntaxZantufaPriorityRawMeksoQuantifier` node; removing the false
warning therefore means removing the false ownership, which is exactly what the
accepted fix does.

## The fix's semantic content, restated

The priority raw route refuses a completed `mex` when its whole extent is
exactly one of the two surfaces baseline `quantifier` already owns:

* a single number operand, whose `number_mekso` payload is the very
  `pa_run_quantifier` rule the baseline alternative parses; and
* a single parenthesized operand, whose `parenthesized_mekso_operand` fields
  are the `mekso_quantifier` production's `VEI`, inner `mex`, optional `VEhO`
  field for field.

Strict ordered choice then reparses the same text through `mekso_quantifier` or
`pa_run_quantifier`. Because both surfaces are built from the component rules
the baseline alternatives use, the rejected raw match and its baseline reparse
consume the identical extent, so ownership, tree shape and diagnostics change
while the accepted language does not. Rejection is expressed by the grammar's
own `reject_output` refinement, not by warning suppression, mutable parser
state, or a post-parse rewrite.

## Conflict classes

Every divergence between `64ea0aa8` and `0d791fd35c` fell into one of these
seven classes. Each was classified before anything was resolved.

### C1 — The DSL and runtime combinator: already landed, hunks dropped

Round one of #843 invented the `reject_output` grammar combinator, the runtime
`OutputRejection` trait, its recovered-generation resolution, the memo-replay
evaluation, and the FIRST-set and elidable-terminator transparency, touching
`crates/jbotci-syntax-macros/src/lib.rs` (+63),
`crates/jbotci-syntax-macros/tests/syntax_grammar.rs` (+57) and
`crates/jbotci-syntax/src/grammar/generated_runtime.rs` (+67).

All of it is on main, landed independently by the intervening epochs, and is
used by six other classifiers (`baseline_bo`, `baseline_mex`,
`baseline_relative`, `baseline_selbri`, `baseline_tag`, `baseline_termset`).
`generated_runtime::reject_output` there rewinds to the inner parser's start
and restores the diagnostic-candidate snapshot, which is the behaviour the fix
depends on. **Resolution: take main; the refresh adds no DSL or runtime
machinery and only names the new refinement.** The `__syntax_model_generator_source`
digest row in the parity matrix is consequently untouched, because
`jbotci-syntax-macros/src/lib.rs` is not edited.

### C2 — The `mex` axis was rebuilt: ported by production name

At the branch point, `MeksoSyntax` was
`ZantufaReversePolishMekso | ZantufaInfixMekso | InfixMekso | ReversePolishMekso`,
with the Zantufa arms sharing `mekso_precedence`, `mekso_base` and
`mekso_operand` with the baseline arms. `MeksoOperandSyntax` was a sum of
`AfterthoughtMeksoOperand | SimpleMeksoOperand | BoundMeksoOperand`,
`MeksoBaseSyntax` carried `ZantufaBoGroupedMeksoBase` and
`ZantufaGroupedMeksoOperandSequence`, and `SimpleMeksoOperandSyntax` carried
`ZantufaScalarNegatedMeksoOperand` and `ZantufaSelbriMoheMeksoOperand`.

On main the Zantufa readings are a separate `zantufa_mex` rule family, and
`mekso` is
`reinterpret_zantufa_mex | zantufa_priority_mex | infix_mekso |
reverse_polish_mekso | zantufa_mex`. `mekso_base` is
`mekso_operand | forethought_call_mekso`. `mekso_operand` became a **product**
`{ connected_expression, grouped_continuation }`. `simple_mekso_operand` gained
`scalar_negated_mekso_operand` and `lahe_qualified_mekso_operand` and lost the
two Zantufa arms.

**Resolution: the descent is ported by production name against the current
model, and the classifier's scope narrows accordingly.** Only the baseline
`infix_mekso` reading classifies:

* `zantufa_priority_mex` is already held to Zantufa-only surfaces by
  `baseline_mex::BaselineMexRejection`, so a completed one is non-baseline by
  construction and must keep its raw-mex ownership. This subsumes the old
  branch's `ZantufaInfixMekso` arm, which existed only because that arm could
  still hold a baseline-shaped tree at the branch point.
* `reinterpret_zantufa_mex` is the deliberate meaning-changing Zantufa
  projection selected by `ZantufaMexReinterpretation`; under that flag the
  faithful Zantufa reading is the point, so a quantifier built on it keeps
  raw-mex ownership and its warning. `BaselineMexRejection` is likewise not
  attached to that alternative. This is pinned by
  `zantufa_quantifier_reinterpretation_keeps_the_faithful_zantufa_reading`, a
  witness the old branch had no axis for.
* the additive `zantufa_mex` fallback and `reverse_polish_mekso` are not lone
  baseline operands.

The new `grouped_continuation` slot on `mekso_operand` consumes tokens, so the
descent requires it absent, exactly as it already requires the precedence
`tail` absent, the infix `continuations` empty and the afterthought chain
`links` empty. The `..`-free exhaustive destructuring guard that the
extent-preservation argument rests on is preserved on both the strict and the
recovered spine.

### C3 — The semantic builder was deleted: hunks dropped

Round one shared the classifier's spine walker with
`crates/jbotci-semantics/src/generated_builder/mekso.rs` and
`generated_builder/mod.rs` (−63/+36) so that only one descent over the spine
existed per generated model. The tersmu retirement (#869, #870) deleted the
whole `generated_builder` tree; `jbotci-semantics` is now `lib.rs`,
`references.rs` and `generated_term_view.rs`.

**Resolution: drop those hunks. There is no second consumer, so the shared
descent has no reason to exist and nothing outside the module reads which of
the two surfaces matched.**

### C4 — The public API surface: retracted with its consumer

`BaselineQuantifierSurface`, its two variants and payload fields, and the three
public accessors were public, `grammar::baseline_quantifier` was a `pub mod`,
and `jbotci-syntax/src/lib.rs` carried `#[doc(hidden)] pub use
grammar::baseline_quantifier`, solely so the semantic builder could import them
across the crate boundary. That is why commit `5145fa5b` had to add a
`RUST_ONLY_CONCEPTS` entry and regenerate `bindings/python/docs/api-parity.tsv`,
and why `a88030d4` had to widen the parity scope (see C5).

With C3's consumer gone, the whole public surface is unreferenced. Main's own
convention, established by the seven `baseline_*` classifiers the intervening
epochs added, is a **private** module exporting only a `pub(crate)` rejection
marker. That is also what `5145fa5b` itself argued for — "shrink the surface
before classifying it".

**Resolution: `mod baseline_quantifier;` is private and exports only
`pub(crate) struct BaselineQuantifierRejection`; the strict classifier is a
private predicate rather than a public surface-returning accessor. No Rust
symbol is published, so `bindings/python/docs/api-parity.tsv`,
`bindings/python/tools/generate_api_matrix.py` and
`crates/jbotci-syntax/src/lib.rs` are untouched.**

### C5 — The parity-scope blind spot: out of scope, still latent

`a88030d4` extracted the hard-coded `jbotci_syntax` inventory file list into
`syntax_inventory_files`, added `grammar/baseline_quantifier.rs` to it, and
added a `syntax_inventory_covers_public_modules` test holding the list closed
under the public modules the listed files declare. It found a real gap: a
`pub mod name;` in a scanned file contributes exactly one `module` row and none
of the module's own items, so an unlisted public module's entire API silently
carries no parity disposition while `--check` stays green.

Main's scope is still the bare `["lib.rs", "tree.rs", "grammar/mod.rs"]` list
and has no such guard. But under C4 this refresh declares no public module, so
the classification the commit existed to enable is not needed, and porting the
guard alone would be a tool change with no rows behind it.

**Resolution: dropped from this branch. The gap is real and unfixed on main —
the next shared accessor added under `grammar/<new>.rs` as a `pub mod` would
repeat it silently — and belongs in its own item, not in a rebase of the #634
fix.**

### C6 — The round-three revert: preserved as the branch left it

`64ea0aa8` reverted the round-three relaxation of the recovered number arm
(back to `5145fa5b` byte for byte), on the ground that rejecting the priority
raw route for `NumberMekso(Valid(NumberMeksoSyntax(Error)))` does not establish
baseline ownership: the reparse can fail at the same token and ordered choice
falls through to the unrefined fourth `zantufa_raw_mekso_quantifier`
alternative, which emits the same warning, so the relaxation would relocate a
false warning rather than remove one. The recovered fallback was recorded as an
integration requirement on #830 instead.

The fourth alternative and its lack of a refinement are unchanged on main, so
that reasoning still holds verbatim.

**Resolution: the refreshed recovered descent requires `Recovered::Valid` at
every slot on the way down to the simple operand, and classifies the operand's
own payload as the exhaustive product it is — the accepted `64ea0aa8`
behaviour. The `zantufa_raw_mekso_quantifier` alternative, the recovery
directives and every #830 behaviour are untouched.**

### C7 — Fixtures: no conflict, but a measured footprint of its own

The accepted branch changed no fixture, so the tersmu-block deletions and
fixture reshaping that #869/#870 and the epochs performed produce no conflict
here — there is nothing on this branch to conflict with them.

The refresh does move expectations, because main's fixture corpus has grown
Zantufa-dialect cases since the branch point that record the very false
ownership the fix removes. `fixture-test --profile all` over 26,573 fixtures
and 72,515 facets reports exactly four failing facets, in four fixtures, and no
others:

| fixture | facet | delta |
| --- | --- | --- |
| `adhoc.syntax.selbri.issue-828-explicit-ku-zantufa` | syntax, gentufa-json | quantifier re-owned; the `re` mex warning dropped |
| `adhoc.syntax.selbri.issue-828-elided-ku-zantufa` | syntax, gentufa-json | quantifier re-owned; the `re` mex warning dropped |
| `adhoc.syntax.tags.issue-833-stag-position-zantufa-rejected` | syntax | still rejected at byte 28; winning error code moves from `syntax.incomplete-mekso` to `syntax.incomplete-term` |
| `corpus.alis.full-alice` | syntax, gentufa-json | 350 quantifiers re-owned |

Each was classified before it was regenerated, and the regeneration was
validated mechanically rather than trusted:

* The two #828 fixtures are epoch-5 witnesses for selbri reconstruction and
  relative-clause ownership under `(zantufa)`; their subject is `re broda poi
  brode`, whose leading `re` is an ordinary baseline quantifier. Line by line,
  the only changes are `quantifier: ZantufaPriorityRawMeksoQuantifier(...)` →
  `quantifier: PaRunQuantifier(<the same PaRunQuantifierSyntax payload>)` in
  `raw` and `json`, and the removal of the single
  `experimental-zantufa-mex` diagnostic at `[0, 2]`. Every other diagnostic and
  every other byte, including the whole relative-clause subtree and the
  `gentufa` `tree` rendering, is unchanged.
* The #833 fixture is a **negative** witness: rolling Zantufa has no tag slot
  between an operand connective and `BO`, so `li pa .e se na'e se pu bo re` must
  be rejected. It still is, at the same byte offset, with the same `status =
  "failure"`. Only the winning diagnostic candidate changes, because the trailing
  `re` is now reparsed through the baseline quantifier route and contributes a
  term-level candidate where it previously contributed a mex-level one.
* `corpus.alis.full-alice` is a 23.7 MB regression baseline under
  `(case-insensitive zantufa)`. Its delta was validated with a script that
  applies the ownership rewrite to the *old* expectation and compares the result
  to the regenerated one byte for byte
  (`/build/jbotci/scratch/lane843-validate.py`). All 350 differences are exactly
  `ZantufaPriorityRawMeksoQuantifier(ZantufaPriorityRawMeksoQuantifierSyntax(InfixMekso(<one NumberMekso operand>)))`
  → `PaRunQuantifier(<the same PaRunQuantifierSyntax payload>)`; the predicted
  text equals the regenerated text exactly, with zero unclassified sites. The
  `gentufa` `json` delta is the same 350 subtrees, checked structurally rather
  than textually. The fixture's `morphology`, `semantics-refs` and `gentufa`
  `tree` expectations are byte-identical.

One `ZantufaPriorityRawMeksoQuantifier` survives in full-alice, and it is the
right one: a `ForethoughtCallMekso` head built on an `.e` connective operator,
which is a genuine Zantufa raw mex and not a lone baseline operand.

## Verification

See the work-item submission for the exact commands and results. The witness
set is the branch's own, re-measured against the refreshed binary, plus the
reinterpretation-flag witness C2 adds. After the fix every baseline-owned row
in the table above is silent and re-owned by `PaRunQuantifier` or
`MeksoQuantifier`; the two genuine rows keep exactly the warnings they should,
`tirna vei pa bo re ve'o cmalu` dropping from two warnings to the one anchored
at `bo`; and the bracket renderings are byte-identical throughout, because only
ownership moved.
