# Code quality and correctness audit — July 2026

This document is the consolidated result of a full-workspace audit of jbotci v1 (~217k lines of
Rust across 25 crates plus tooling), performed against the project's own standards in `AGENTS.md`:
correctness first, design-by-contract everywhere, no undiscussed heuristics, no string-level hacks
around the morphology parser, strong typing, ownership discipline, aggressive removal of dead code.

Every finding was verified by reading the code in context (not just grep). Severity scale:

- **HIGH** — correctness bug, user-reachable panic/DoS, silently broken feature, or a
  regression-masking pattern.
- **MEDIUM** — quality/architecture problem with concrete failure modes.
- **LOW** — polish.

Sections also record verified non-issues so future audits don't re-litigate them.

## Executive summary

The codebase is in good shape where it counts most: the fixture machinery compares exact strings,
the "never split words by whitespace" discipline is respected in the parsing core, server-side
security fundamentals (path traversal, HTML escaping, ed25519 verification) are correct, the ALINE
and CLL-4.14 scoring implementations were verified against their published definitions, jvozba
routes all hyphen/tosmabru logic through morphology, and the hand-rolled QR encoder was verified
line-by-line against ISO/IEC 18004 with no encoding bugs. The semantic builder's 202 explicit
`unsupported(...)` sites are a model of honest incompleteness.

That said, the audit found real correctness bugs reachable from ordinary user input, silently
broken features, one regression-masking tool mode, and systemic patterns that contradict the
project's own rules:

1. **A workspace-wide contract bug pattern**: `#[ensures(ret.as_ref().is_ok_and(...))]` on
   `Result`-returning functions *without* `|| ret.is_err()`. bityzba cheap contracts are always-on
   asserts, so every `Err` return becomes a contract panic — nullifying entire designed error
   paths in dictionary import, CLL loading, and build scripts, plus point instances elsewhere
   (§1.1).
2. **Morphology: an incomplete stringly selma'o table** (`ku`, `kei`, `ke`, `gi`, `doi`, `boi`…
   missing) makes `... ku sa ku` erase the *entire* preceding text instead of erasing to the
   previous KU, and reports `null` selma'o for ordinary cmavo in public output. Plus a false
   `ensures` on `strip_diacritics` that panics on combining-marks-only input (§Morphology).
3. **Syntax: a genuine grammar deviation** — the live rule for `sumti-5 = quantifier selbri /KU#/`
   lost its `ku` field (`mi viska ci gerku ku` should parse and doesn't); the correct rule
   survives only as dead code behind a blanket `#![allow(dead_code)]` (§Syntax).
4. **Search: user-input panics and DoS** — punctuation-only `vlacku` queries panic on a false
   postcondition; the glob matcher backtracks exponentially (`****…*x` hangs CLI/MCP/wasm)
   (§Search).
5. **Semantics: `ko'a goi le broda` is a silent stub** (the pro-sumti-head GOI branch is
   unreachable), and compound SE conversions (`se te klama`) are truncated to the last conversion
   in reference analysis — diverging from the semantic builder, which folds the chain correctly
   (§Semantics).
6. **`fixture-rewrite --migrate-morphology-diagnostics` blindly flips green fixtures to
   expected-failure** from current parser output — the exact regression-masking mode AGENTS.md
   forbids (§Build tooling).
7. **Silently dead features**: gentufa's `show_elided` option is plumbed from CLI/web but ignored
   since the legacy-AST removal; gentufa web dictionary annotations ship hard-disabled with 9
   `#[ignore]`d tests; the `grammar-debug` EBNF/SVG outputs return raw Rust source and their test
   cannot pass (§Gentufa, §Web core, §Syntax).
8. **A latent WebGPU q4 matmul indexing bug** (ignores per-row group padding → garbage embeddings
   on non-divisible shapes) plus zero WebGPU error capture (failures yield silent all-zero
   embeddings) (§UI).
9. **Systemic debt**: `#[invariant(true)]` on hundreds of types with real, relied-upon invariants;
   stringly-typed closed sets throughout; six monolithic files (44k/25k/12.6k/9.7k/8.4k/6.9k
   lines); 4–5 divergent copies of diacritic stripping; a private cmavo-morphology
   reimplementation inside `jbotci-dialect`; a greedy string-level lujvo splitter in jvozba
   bypassing morphology; heavy deep-cloning on AST hot paths.

## Part I — Cross-cutting systemic issues

### 1.1 HIGH — Unsound `ensures(…is_ok_and(…))` without an `is_err` escape

bityzba's `#[ensures]` compiles to an always-on `assert!` (verified in
`bityzba-macros/src/implementation/mod.rs:108-110`, `codegen.rs:168`). Confirmed instances:

| Location | Consequence |
|---|---|
| `jbotci-dictionary/src/import.rs:83-85, 99-108` | Any malformed Lensisku JSON panics instead of returning `LensiskuImportError`. The crate's own negative tests (import.rs:180-212) would fail if the `import` feature were ever enabled in a test run — they are effectively never exercised. |
| `jbotci-cll/src/lib.rs:642-649, 651-653, 707-709, 726-729, 826-829` | The entire `CllError::Load/Parse` path is unreachable; failures panic with a misleading message. |
| `jbotci-cll/build.rs:66-68`, `jbotci-dictionary-data/build.rs:410-412` | Build-script error paths panic inside the contract. |
| `jbotci-output/src/lib.rs:297-312` | `compact_json_value` ensures non-`Null`, but a root JSON `null` passes through unchanged — valid calls panic. |
| `jbotci-search/src/vlacku.rs:868-872` | Punctuation-only queries (`--valsi '!!!'`) violate the postcondition. **User-reachable panic** from CLI, MCP, and wasm (verified end-to-end). |
| `jbotci-gentufa/src/lib.rs:1624-1643` | `weighted_circular_mean_hue` ensures `0.0..360.0`; float rounding can yield exactly `360.0`. User-reachable via block coloring. |
| `jbotci-morphology/src/syntax_eq.rs:117` | `strip_diacritics` ensures non-empty output for non-empty input — false for input of only combining marks (`\u{0301}` etc. map to `None`). Public API panic. |
| `jbotci-embeddings/src/lib.rs:2490-2505` | `native_pack_remote_paths` uses `ensures(!ret.contains("manifest.json"))` to "validate" network-supplied manifest paths — a malicious/broken asset server crashes the client instead of getting `InvalidIndex`. |
| `jbotci-semantics/src/references.rs:1076-1087`, `jbotci-gentufa/src/lib.rs:1812-1825` | `saturating_sub`-based `length > 0` and `!ret.ends_with("Syntax")` overpromises. |

**Fix:** sweep the workspace for `is_ok_and` inside `#[ensures]` and append `|| ret.is_err()`
where `Err` is possible (jbotci-diagnostics already does this consistently); add a
contract-scanner rule to prevent recurrence. The search/gentufa/morphology/embeddings cases need
real fixes, not just weaker contracts: model empty normalization as an outcome, `rem_euclid` the
hue, state the true `strip_diacritics` property, and validate remote manifests with typed errors.
Enable the dictionary `import` feature in a CI test run.

### 1.2 MEDIUM — `#[invariant(true)]` on types with real, relied-upon invariants

The most widespread contract violation: constraints restated as scattered per-function `requires`,
`.expect()`s, or external `is_valid()` predicates instead of type invariants. Confirmed clusters
(each with downstream code relying on the constraint):

- `SemanticGraph` (`jbotci-semantics/src/model.rs:301-364`): six validations in
  `new() -> Result<_, String>`, but `pub` fields + `invariant(true)` = bypassable.
  `SemanticObject` (model.rs:388-610): ~100-field god struct for 14 kinds, shapes enforced only by
  ~130-line external validators. `SemanticObjectId::eventuality(0)` satisfies its own contract but
  violates its callee's (`model.rs:14-19`).
- `QrCode`/`QrLogoPlacement`/`QrBlock` (`jbotci-output/src/qr_code.rs:10-56`): a hand-built
  `version: 99` reaches an out-of-bounds table index; six functions compensate with per-call
  `requires`. Similarly `BracketSourceRange`, `DiagnosticRenderOptions`, `TraceRenderOptions`.
- `ServerConfig`/`AppState` (`apps/jbotci-server/src/lib.rs:40-54`): `base_path.starts_with('/')`
  restated in ~10 functions plus a defensive re-normalization in `router()` showing the invariant
  isn't trusted.
- `WebSourceRange`, `ReferenceLabel`, `GentufaBlock(sLayout)`, `GentufaPngOptions` (jbotci-gentufa)
  — including a slice-panic path on inconsistent deserialized layouts (`render.rs:384`).
- `VlackuCard`/spans/options/requests (jbotci-search); `CllSite`/`CllChapter`/`CllSection`/
  `CllExample` cross-map consistency (jbotci-cll:32-118); `WithIndicators<T>`
  (jbotci-syntax/src/tree.rs:88-107 — constructor preconditions not enforced on the `Deserialize`
  path); `ValsiClassification` (jbotci-morphology/src/lib.rs:177-195 — 12 optional fields spanning
  7 mutually exclusive shapes, invariants for only 3); `IpaSegmentId`/`IpaTokenSequence`
  (jbotci-phonetic — range invariant enforced only by `.expect()` panic sites plus a forbidden
  public `is_valid_ipa_segment_id`); `LujvoDecomposition`/`LujvoSegmentInfo` (jvozba);
  `LoadedCorpus` (embeddings); `ReferenceRect`/`AsyncActivityState` (UI) and
  `Q4Tensor`/`ModelConfig` etc. (WebGPU runtime — shaders divide/index by these fields).
- `CmavoDialectEntry` (`jbotci-dialect/src/lib.rs:104-145`): invariant-bearing type that **also**
  exposes `pub is_valid()` (explicitly forbidden), and `DialectDefinition`'s invariant
  re-validates every already-validated element, re-parsing every cmavo per construction.
- Fixture support types (`tests/support/fixtures/mod.rs:877-925`): `invariant(true)` + public
  `is_valid()` demanded via `requires` at dozens of xtask-full sites.
- Also: 188 tautological `ensures(ret.is_ok() || ret.is_err())` in `generated_builder.rs` (plus
  more in embeddings/UI) that read as reasoned contracts but assert nothing — replace with
  `ensures(true)` or real postconditions.

### 1.3 MEDIUM — Stringly-typed closed sets

- **Semantic relation labels** (`generated_builder.rs`): relations assembled from surface text
  (`"nu <x>"`, `"nu'a <op>"`, `"<mekso> moi"`) and structure *recovered by string sniffing*
  (`starts_with("nu ")`, `ends_with(" moi")`; `constructed_relation_place_count`:40763).
  Collision-prone (a ZEI compound starting with `nu` misclassifies). Needs a typed
  `RelationLabel` enum.
- **Argument places as `"xN"` keys** (`generated_builder.rs:43925`, model.rs): usize↔"xN"
  round-tripping at 83+ sites, `"x10" < "x2"` ordering trap, ~10× copy-pasted fill-elided-places
  block. Needs a `PlaceIndex` newtype.
- **Morphology's `selmaho(&str) -> Option<&'static str>`** (`lib.rs:2530-2599`): hand-maintained,
  incomplete, duplicating the typed `Cmavo`/`Selmaho` model — with the SA-erasure bug as a direct
  consequence (§Morphology).
- **Word-type filters** (`jbotci-search/vlacku.rs:214-305`) duplicating the `WordType` enum as
  normalized strings; **CLL line/row kinds** (`jbotci-cll:120-239`); **composition operators/math
  kinds** (`model.rs:2436+`: `"connectiveQuestion"`, `ends_with("Interval")` ×3); **gentufa marker
  kinds** (`kind` ∈ {"sumti","reference"}, `token_kind` always `"word"`); **web state**
  (`check_collisions: String` etc.); **experimental-cmavo warning tables keyed on parser label
  strings** with unreachable `"KOhA"`/`"BY"` arms (§Syntax); **syllable patterns as
  `Option<String>`** compared to `"CVCCV"` literals (lujvo.rs); **Debug-repr cache keys**
  (`format!("{:?}|{}…")`, generated_builder.rs:43601); **`GismuCollision.existing_word_type:
  String`**.

### 1.4 MEDIUM — Monolithic files

| File | Lines | Notes |
|---|---|---|
| `jbotci-semantics/src/generated_builder.rs` | 43,965 | **Hand-written** (verified — no generator; the name refers to consuming the generated syntax model). One 28.5k-line impl block, ~1,527 functions, **exactly one comment**. |
| `jbotci-ui/src/lib.rs` | 25,216 | God-component + all five pages + find engine + diagnostics lexer + tests. |
| `xtask-full/src/main.rs` | 12,576 | ~8 unrelated tools. |
| `apps/jbotci/src/lib.rs` | 9,713 | Clap + runners + MCP bridging + colorization + 3.7k lines of tests. |
| `jbotci-web-core/src/lib.rs` | 8,433 | |
| `jbotci-cll/src/lib.rs` | 6,886 | Model + XML import + links + search + two renderers + EBNF + MathML. |

All are pure-code-motion splits protected by the existing suite — the "refactor boldly" case.

### 1.5 MEDIUM — Duplicated correctness-critical logic

- **Diacritic stripping: 4–5 copies**, already divergent (morphology ×2 byte-identical private
  copies at `syntax_eq.rs:125` / `lib.rs:2507`, dictionary, dialect, orthography's overlapping
  map).
- **jbotci-dialect privately reimplements cmavo morphology** (`lib.rs:1510-1896`): own
  consonant/vowel/initial-pair/diphthong tables and parser, no morphology dependency — the exact
  circumvention AGENTS.md forbids; ~80 lines (the 45-entry initial-pair table + cluster-onset
  branches) are provably dead since cmavo never begin with clusters.
- **Morphology grammar.rs re-implements ~13 segment.rs phonology helpers** (diphthong/nucleus/
  stress rules encoded twice; grammar.rs's `digit_to_cmavo` returns `Option` while segment.rs's
  silently returns `""`).
- **The 17×17 consonant `PAIR_MATRIX` maintained twice** (`lujvo.rs:468` i32 vs
  `segment/phonotactics.rs:51` u8) with undocumented 0/1/2 rank codes compared as raw literals —
  needs one table + a `ConsonantPairClass` enum with CLL citation.
- **Parallel bool/String cmavo-form parsers** (`matches_cmavo_form_*` vs `parse_cmavo_form_*`,
  segment.rs:1666-1807).
- **references.rs: ~4 hand-rolled grammar-traversal copies** (~5–6k lines) that **already
  diverged** (CEI collector skips `GroupedTanruUnit`). **CllBlock: 5 divergent traversals** with an
  observable search gap. **generated_builder.rs: a ~450-line function pair + 4× verbatim
  connective dispatch.** **`next_segment` vs `next_display_segment`** ~95% duplicated
  (grammar.rs:234-389). **`"UI"`/`"UI3a"` arms**: two verbatim ~55-cmavo lists
  (tokens.rs:731-850).
- **xtask vs xtask-full: ~800 duplicated lines** incl. the byte-identical service-worker template
  (divergence would silently ship different service workers); xtask-full's `expectation_status`
  duplicates `TestCase::expectation_status` line-for-line.
- **`escape_html_text` duplicated** (server vs web-core); **period-character set** re-hardcoded in
  semantics; **leading-pause/vowel logic ×3** (gentufa/output/phonetic); **Rust/JS F2LLM model
  catalog** duplicated with hard-coded URLs in JS; **`hex_digest` duplicated**
  (embeddings/embedding-inputs); **the `::Variant => expr` invariant grammar hand-duplicated in 3
  crates** (scanner, bityzba-macros, tree-macros) — with a real divergence already (§Macros).

### 1.6 MEDIUM — Undiscussed heuristics (flagged for explicit decision)

- **jvozba's `sloppy_decompose`** (`lib.rs:686-835`): greedy, non-backtracking char-level rafsi
  splitter with hard-coded hyphen-drop rules — string-level morphology circumvention *and* it
  demonstrably misses valid inputs (see the r-hyphen bug, §Tool crates). Belongs in
  jbotci-morphology as a proper backtracking parser.
- **Span-key node identity** (`references.rs:4610-4683`): "deepest node with same first/last token
  span" — conflates wrapper nodes *and* is O(N²·depth) on a hot path; an exact O(1) `by_ref` map
  already exists.
- **Dictionary place-count derivation** from jbovlaste definition text (`builder.rs:116-193`) —
  the approach itself is accepted as unavoidable (no typed Lojban dictionaries exist;
  owner-confirmed), but the jbovlaste conventions it implements must be documented at the site
  and pinned by unit tests; there is also a dead-assignment bug (§Semantics).
- **BAI modal tables with silent `marker.replace(' ', "-")` fallback** for ~40 uncovered BAI
  (`generated_builder.rs:40726`).
- **Uppercase-first-letter "constructor" sniffing** in CLI JSON colorization
  (`apps/jbotci/src/lib.rs:5891-5987`) and output's compact-JSON DOM patching.
- **`data!`/`new!` path classification by identifier casing** (bityzba-macros) — breaks
  `data!(Self::Variant { … })` outright.
- **annotate-snippets gutter-width guess** (`diagnostics.rs:19`, hardcoded 12 — wrong for
  multi-digit line numbers).
- **Interlinear alignment by whitespace token counts** (jbotci-cll:2602) — safe fallback, but
  jbo-cell dictionary links then treat whitespace tokens as words; route through the segmenter.
- **`base_path_from_canonical` substring matching** (web-core:4188); **windowed-mean pooling of
  over-long embedding inputs** with equal weights and no documenting comment
  (`native.rs:196-220`).

### 1.7 Dead code (verified zero call sites)

- **jbotci-morphology: ~36 dead private functions in segment.rs** (uncached wrappers/entry
  points, listed in the crate section), a "raw" public API family behaviorally identical to the
  non-raw one (7 pub functions whose distinction was lost — either dead surface or a latent bug),
  identity wrapper `base_word_like`, single-variant `SyllablePolicy` threaded through 6
  signatures, ignored `_options` in `is_cmevla_with_options`.
- **jbotci-syntax: 13 dead grammar rules** hidden by a blanket `#![allow(dead_code)]`
  (`generated.rs:3`) — the abandoned description-parsing decomposition, one of which is the only
  rule with correct `ku` handling (masking the HIGH grammar bug); duplicate
  `SyntaxParse`/`GeneratedSyntaxParse` type pairs with byte-identical invariants; ~11 dead token
  helpers; `syntax_tree_partial_valid_round_trip` stub advertising nonexistent recovery; dead
  `ReplacementWord` category with an always-failing parser.
- **jbotci-semantics**: `PlaceCursor::new`/`merge_from_branch`, `record_wrapped_koha_reference`
  (likely the missing GOI wiring), the whole `V0CompatibilityProjection` subsystem + the
  `edge_ids_by_target_node` index maintained solely for it, unused pub `DiscourseReferences`
  accessors, and more.
- **jvozba**: `compose_lujvo`+`LujvoPlan`+`LujvoSource`, `build_best_jvozba`,
  `render_jvozba_error`, `word_like_type_key`. **gimfihi**: test-only `find_collision*` in
  production module. **phonetic**: dead aspiration feature (tokenizer rejects ʰ), unreachable
  empty-segment branch. **embeddings**: `NativeGemmaEmbeddingBackend` alias, dead
  `window_count == 0` branch, unused `anyhow`/`jbotci-search` deps.
- **jbotci-ui: all of platform.rs** (with observed drift vs live copies). **jbotci-gentufa**:
  `TransformInfo`/`transform`/`parent_color` (only ever `None`, bloating every serialized block),
  `render_loose_latin_surface`. **jbotci-output**: `with_indicators_value` cluster,
  `QR_LOGO_TEXT` + unused pub QR surface, trace `terminal_width` (populated, `requires`-guarded,
  never read). **web-core/server**: `trim_web_float`, identical-branch conditionals (×2), dead MCP
  match differentiation, test-only Discord registration payload. **bityzba-macros**: inherited
  dead modes/features/macros, `Contract::_span`, stale TODO. **xtask-full**: unreachable
  Check/Test/Clippy/Fmt arms. **Root tests**: `run_on_normal_stack` no-op.

### 1.8 Hot-path allocations and clones

- **Morphology**: `Word::phonemes()` clones the phoneme `String` per call and
  `is_simple_cmavo_text` runs ~8× per token with two `canonicalize_text` allocations each
  (`phonemes_ref()` exists, unused here); `streaming_*_candidate` re-collects and re-normalizes
  the whole prefix per candidate end (O(n²)) and constructs a fresh (n+1)²
  `LujvoRecognitionCache` per wrapper call, defeating memoization; `is_gismu_slice` String→Vec
  round-trip inside recursive rafsi analysis; `word_syntax_eq` allocates two Strings to test
  equality when `strip_diacritics_eq` exists.
- **Syntax**: every failed terminal match allocates a `String` reason + two `Vec`s + a cloned
  expected-token `Vec` (`parse_error.rs:110-142`, `tokens.rs:219-256`) — failures vastly
  outnumber successes in PEG choice; token pre-passes clone `Word`s for read-only checks.
- **gimfihi scoring loop**: ~200k String clones + ~600k Vec allocations per realistic
  `compose_gismu` (per-candidate×per-source `SourceScore` Strings and per-call LCS char-Vecs/DP
  rows). All candidates must of course still be *scored*; the fix is to compute only the numeric
  total in the hot loop (source chars pre-collected once per compose, reusable DP scratch as in
  jbotci-phonetic's `AlineSimilarityScratch`) and materialize the `SourceScore` detail records —
  the structs with cloned `language`/`word` Strings — only for the top `count+1` candidates
  actually displayed, after ranking.
- **Output tree renderer**: deep clone of every `TreeValue` subtree per entry per node via the
  redundant `RenderEntry` duplicate. **generated_builder**: whole-subtree syntax clones into
  builder state (missing `'syntax` lifetime). **references.rs/gentufa**: every token span cloned
  into every ancestor (only first/last used). **UI**: ~20 wholesale page-state clones per
  keystroke, page-find corpus rebuilt+hashed with empty query, zero `use_memo`. **search**:
  per-entry Strings in full-dictionary loops. **CLL**: every example deep-stored twice (with a
  real link-resolution divergence between copies) + 4-allocation `escape_html` chain.
  **dialect**: builtins re-parsed recursively per lookup. **diagnostics**: O(n)/O(n·m) scans
  inside always-on cheap contracts (`lib.rs:740-792`) — move to expensive tier.
  **embeddings native**: `Box::leak` of the whole 79–397 MB model per load (leaks multiply in the
  long-running server); Windows `rename` bug on force-redownload (`lib.rs:1780-1787`).

## Part II — Per-area findings

### Morphology (`jbotci-morphology`)

Well-built core: camxes-faithful segmentation through the parser, dense targeted tests (garden
paths, zbalermorna/Cyrillic, SA/SI/SU, ZOI), meaningful invariants on most model types. No debug
output, no TODOs, no string hacks in the parsing core.

**HIGH:**

- **Stringly, incomplete selma'o table with real SA misbehavior** (`lib.rs:2530-2599`):
  `selmaho(&str)` misses `ku`, `kei`, `ku'o`, `ke`, `ke'e`, `ga`, `gi`, `doi`, `boi`, `bai`…
  Consequences verified: `handle_sa` (`grammar.rs:973-977`) gets `None` → `unwrap_or_default()`
  truncates to 0, so `... ku sa ku` **erases the entire preceding text**; and
  `plain_word_classification` (`lib.rs:1846`) reports `null` selma'o for ordinary cmavo. Fix:
  delete the table; implement `Word::selmaho()` via `Cmavo::from_text` + a primary-Selmaho mapping
  in cmavo.rs; make `SAMatchTag::Selmaho` hold `Selmaho`.
- **False `ensures` on public `strip_diacritics`** (§1.1).

**MEDIUM:** ~36 dead segment.rs functions + vacuous "raw" API family (§1.7);
`next_segment`/`next_display_segment` duplication; grammar.rs phonology duplication +
`PAIR_MATRIX` ×2 with magic rank codes (§1.5); **`handle_sa` swallows all non-ZOI morphology
errors** (`grammar.rs:953-960`) — invalid garbage after `sa` makes the whole input "succeed" as an
empty word list with no diagnostic; hot-path allocation churn (§1.8); `word_syntax_eq`
equality-by-allocation; syllable patterns as strings (§1.3); flat `ValsiClassification` (§1.2);
`ensure_cmevla_word`/`is_cmevla` (`lujvo.rs:211-279`) suffix a literal `'s'` without re-validating
through morphology, and `is_cmevla` accepts `"abc7"`.

**LOW:** identity wrapper `base_word_like` + trivial single-use wrappers; ignored `_options`
param; `strip_diacritic` triplicated (§1.5); **no exhaustive `from_text`↔`canonical_text`
consistency test** over the ~950 hand-maintained cmavo pairs (add an `ALL_CMAVO` slice +
round-trip test — a single transposed arm would silently break canonicalization); parallel
bool/String form parsers + tuple-returning `parse_onsets`; inconsistent invariant-marker
conventions on C-like enums; single-variant `SyllablePolicy`; `starts_with_vowel_or_glide` doesn't
handle glides (misnamed; `ĭ`/`ŭ` wouldn't match).

### Syntax (`jbotci-syntax`)

`grammar/generated.rs` is **not machine-generated**: it's a hand-written declarative grammar DSL
expanded by `syntax_grammar!` (jbotci-syntax-macros); `generated_runtime.rs` is its hand-written
runtime. Expansion machinery (packrat memo with warning replay, left-recursion guard,
farthest-error diagnostics) is sound; EBNF fidelity is good in spot checks; field order matches
input order throughout.

**HIGH:**

- **Lost elidable `KU`** (`generated.rs:1579-1585`): the live rule for
  `sumti-5 = quantifier selbri /KU#/ [relative-clauses]` has no `ku` field; the only
  correctly-shaped rule (`gadri_elided_description_sumti`, :1673) is dead. No other `sumti_base`
  variant can consume trailing `ku` after bare `PA brivla`, so `mi viska ci gerku ku` (valid per
  the EBNF) should be rejected; no camxes fixture covers the gadri-less form, which is why it's
  untested. (Statically verified; sandbox denied running the parser.) Fix: add
  `field ku <- opt(cmavo(Ku).wf());`, delete the dead rule, add a fixture, run the full profile.

**MEDIUM:** `grammar-debug` EBNF/SVG stubs return `include_str!("generated.rs")` — raw Rust
source — for both formats, wired to the CLI; the feature-gated test asserting dialect-dependent
output containing `"mu'oi"` **cannot pass** (static include, zero occurrences), proving the
feature never compiles in CI. Implement real rendering or remove the feature + CLI formats; add
`--features grammar-debug` to CI. — 13 dead grammar rules under blanket `#![allow(dead_code)]`
(§1.7). — **Experimental-cmavo warning tables keyed on parser label strings with unreachable
`"KOhA"`/`"BY"` arms** (`tokens.rs:290-867`): KOhA/letter words match via word-category terminals
(`"PRO-SUMTI"`/`"LERFU"` labels), so experimental KOhA/BY cmavo parse **without** the experimental
warning. Key on typed Selmaho/category; add tests. — Duplicate `SyntaxParse`/
`GeneratedSyntaxParse` pairs (§1.7). — `WithIndicators` variant invariants all `=> true` while
constructors carry the real constraints (plus a BAhE-vs-ZAhE inconsistency between constructors)
— serde can build invalid values. — Per-failure allocations in the parse-error hot path (§1.8). —
Verbatim `"UI"`/`"UI3a"` duplication.

**LOW:** `panic!` as precondition in `syntax_construct_depth`/`_is_root` (make it
`#[requires(syntax_construct_is_known(…))]` + a completeness test for
`SYNTAX_CONSTRUCT_METADATA`); dead helpers (§1.7); `byte_range().unwrap_or(0..0)` masking a stated
invariant at two sites; inconsistent anchor-not-found fallbacks (index 0 vs `tokens.len()` →
`👉<EOF>`); `debug_assert!(false, …)` silently breaking repetition loops in release (make it a
contract or hard internal error); tree equality via serialize-to-JSON + delete-every-`"span"`-key
with swallowed serialization errors (backs an `expensive_ensures` — generate a typed
span-agnostic equality); weak `invariant(true)` on `ParserState`/`SyntaxMemoSuccess`;
elidable-terminator recovery keyed on DSL field-name strings with silent-`None` drift mode (emit
typed terminator metadata from the macro, or add a completeness test; rename `liau`→`lihau`);
token pre-pass clones; `feature_gate`/`policy_gate` and `complete_*` near-duplicates + 6× repeated
peek-token block despite `expected_found_at_current` existing.

Verified non-issues: the PA+ROI and Zantufa `i'au` indicator-attachment special cases are
deliberate, warned, dialect-gated, tested; `assert !selmaho(Roi)` guards match the EBNF ambiguity;
no debug output/TODOs/tuple constructors.

### Contract & macro infrastructure (`bityzba`, `bityzba-macros`, `jbotci-tree[-macros]`, `jbotci-syntax-macros`)

**MEDIUM (high-leverage):** scanner emits `rerun-if-changed` per discovered file, so **newly added
files are not tracked** and can ship contract-less (emit for directories); **items inside function
bodies are never scanned**; **description-only/empty contract attributes silently check nothing**
while satisfying the scanner (`#[requires("x must be positive")]` → zero assertions — make it a
spanned `compile_error!`); **`ReturnReplacer` rewrites `return` inside nested `fn`s and async
blocks** into `break 'run` (breaks legal code with a baffling label error); **`#[contract_trait]`
miscompiles `mut` params and panics on `_` patterns** with a Debug dump; **`data!` path
classification by casing** breaks `data!(Self::Variant …)` (§1.6); **qualified derive paths
silently dropped from the wrapper** (`#[derive(serde::Serialize)]` applies only to the Data type);
**tree-macros re-implements bityzba's true-marker predicate and they already diverge**
(`#[invariant(true, "reason")]` → tree-macros generates calls to nonexistent `from_data`) — share
one predicate (the `::Variant => expr` grammar is hand-duplicated in 3 crates); **syntax-macros
signals codegen failure via `Option`+`filter_map`** — a DSL typo silently skips generating that
rule's parser (masked by blanket `allow(dead_code)`) — convert to `syn::Result` with spanned
errors; **both proc-macro crates carry no contracts and don't run the scanner** despite the
workspace rule (no cycle obstacle — wire up or document the exemption).

**LOW:** scanner matches attributes by last path segment (spoofable) and demands invariants on
tuple/unit structs the macro can't expand; generated method-name collisions with fields named
`build`/`from_data`/`new`; relative `serde::` paths in generated impls (tree-macros correctly uses
`::serde::`); implication rewriter comment says `==>` but implements `->` and mangles legitimate
`->`/top-level commas in contract expressions; panic-based diagnostics with a stale "only works on
functions" message; `tree_model!` hardcodes `RecoveryTreeItem`/`FreeModifierSyntax`/`"Word"` in a
nominally generic macro; multi-segment atom paths silently assumed to have external impls; `&T`
tree fields silently lose the reference; `AtomRef` name-mangling collisions (`Foo<Bar>` vs
`FooBar`); token-string type identity with inconsistent `Vec` normalization; dead duplicate match
arm (`classify_call_recovery_expr:4632`); dangling `Rule(...)` recovery metadata for unknown
names; `(a b)` accepted as two arguments (comma not required).

### Semantics core (`model.rs`, `references.rs`, `builder.rs`)

Structurally disciplined (typed node-ID newtypes, cycle guards, deliberate "intentionally vague"
`ra`/`ru`/`go'a`); `builder.rs` is a live facade, **not** a dead legacy path.

**HIGH:**

- **`ko'a goi le broda` assignment is a stub** (`references.rs:10600-10607`, caller :8632-8640):
  `generated_argument_koha_cmavo_from_index(_index, _sumti)` unconditionally returns `None`,
  making the entire "GOI assigns the relative-clause head pro-sumti" branch unreachable. This word
  order is explicitly valid per CLL ch. 7; later `ko'a` mentions silently resolve to nothing. The
  dead `record_wrapped_koha_reference` (:9472) looks like the missing wiring. Implement +
  regression test for `ko'a goi le broda .i ko'a …`.
- **Compound SE conversions truncated** (`references.rs:2214-2231, 2239-2256`):
  `analyze_tanru_unit_atom` applies only `unit.conversions.last()`; `se te klama` maps x1 to inner
  x3 instead of the correct se∘te composition. `generated_builder.rs` folds the whole chain
  correctly (:31008-31018), so the two analyses **disagree**. Fold the chain (nested `Conversion`
  frames right-to-left).

**MEDIUM:** CEI collector skips `GroupedTanruUnit` (`:7037-7084` — prenex CEI inside `ke…ke'e`
silently unbound; audit sibling ignore-lists); span-key identity heuristic + O(N²·depth) (§1.6 —
generate an `as_node_ref()` impl so the exact O(1) `by_ref` map serves `raw_for_node`); four
traversal copies (§1.5 — unify on `TreeVisitor`, already used by the index builder);
`SemanticGraph`/`SemanticObject` god-struct + external validation (§1.2 — staged fix: per-kind
payload enum serializing to the current flat JSON; short-term, non-pub fields); **quote isolation
misses utterance history** (`:8454-8491` — `di'u` outside a quote can resolve to a quoted
statement, and `di'e` inside a quote to one outside it; verify against CLL 7.9, then include
utterance state in the swap set); **`first_definition_place_id` dead-assignment bug**
(`builder.rs:219-239` — written intent "continue scanning" is not what executes; scan aborts on
unparsable `$x_{…}$`) plus documenting and test-pinning the place-count conventions (§1.6 — the
approach itself is accepted as unavoidable); dead-code cluster (§1.7).

**LOW:** per-query mention Vec+sort in `ri`/`go'i` resolution (:9539-9587); global-max-place loops
with per-place HashSet walks (:4208-4302); `ReferenceEdge.rule: String` from `'static` literals,
`ReferenceKind` not `Copy`; dead `Dihe` re-match arms in `resolve_koha` (:9663-9684);
`(String, RawSyntaxNodeId)` tuples through ~15 collectors (needs `CeiAssignmentSource` +
`CeiLabel` enum); missing real invariants on `SyntaxNodeMetadata`/`SelbriPlaceFrame`/
`SyntaxSpanKey`/`Descriptor`/`Quotation`/`Connector`.

### Semantic builder (`generated_builder.rs`)

**Hand-written**, contract-annotated, honestly incomplete (202 `unsupported(...)` sites).
Structurally the workspace's worst file.

**HIGH:**

- **44k lines / one 28.5k-line impl / ~1,527 functions / one comment** — split along the natural
  seams in the method ordering (text plan; statements; bridi/selbri formulas; sumti connection;
  tense/modal; tanru; pro-bridi; referents/descriptions; mekso; letterals/numbers; labels; spans),
  adding CLL-cited why-comments at each semantic decision point.
- **The ~450-line near-verbatim function pair**
  (`build_generated_logical_sumti_connection_formula_for_terms` :10841-11283 vs the scalar variant
  :11411-11862), each containing the same ~45-line connective dispatch **twice** — four verbatim
  copies in the file (verified line-by-line, including mirrored `expect()` sites). Extract the
  per-sumti dispatch into one helper returning a small enum; merge the pair via an
  `Option<ScalarNegationContext>` parameter.
- **Stringly relation labels re-parsed by sniffing** (§1.3) — a typed `RelationLabel` enum is a
  model-level change to verify against fixture/Lean expectations (surface serialization can stay
  identical).

**MEDIUM:** `"xN"` keys + ~10× copy-pasted fill-elided-places block (§1.3); missing `'syntax`
lifetime forcing deep subtree clones (§1.8); 188 `is_ok() || is_err()` tautologies + 26 weak
invariants (§1.2); `token_list_text`'s false `ensures(!ret.is_empty())` for empty iterators
(:43675 — take a non-empty slice or return `Option`); BAI tables with string fallback (§1.6);
Debug-repr cache key (§1.3).

**LOW:** number/quantifier/letteral dispatch on joined-text round-trips instead of typed `Cmavo`
matches (:43294, :43573, :43614, :43503); duplicated period set (:43669); per-extraction String
churn in `token_text`/`token_list_text` (117 call sites — add push-into variants); 6×
`expect("bound distribution has tail")` re-checking what the helper proved (return the tail from
`generated_sumti_bound_for_distribution`); `is_lojban_period`'s ensures restates its body;
misleading `builder.rs` naming (contains no builder — rename to facade/errors when splitting).

### Search (`jbotci-search`)

Small and generally well structured — deterministic NaN-safe tie-breaking, correctly guarded
top-k, linear-time regex via the `regex` crate.

**HIGH:**

- **Contract panic on punctuation-only queries** (`vlacku.rs:868-872`; §1.1). Reachable from CLI
  (`--valsi '!!!'`), MCP, and the wasm client (verified end-to-end). Fix the contract *and* route
  empty normalization to `invalid_output` with the original query text.
- **Exponential glob backtracking = user-controlled CPU DoS** (`vlacku.rs:1642-1660`): naive
  recursion, no memoization, no collapsing of consecutive `*`; `--valsi '****…*x'` enumerates
  C(n+k, k) paths **per dictionary entry**. Replace with the standard O(tokens × chars) DP and
  collapse `AnyMany` runs in `compile_glob_pattern`.

**MEDIUM:** stringly word-type layer duplicating `WordType` (§1.3); per-entry allocations in
full-dictionary loops (§1.8); dead `entry_card_with_decomposition` + near-duplicate
card/decomposition builders (a field added to one is silently dropped by the other); **glob
compilation silently drops invalid characters** (`kl!ma` → `klma`, wrong results with no
diagnostic — :1576-1580; return an error like the regex path does).

**LOW:** identical-branch `if/else` for `VlackuOutcome` (:703-709); weak contracts (§1.2);
`filter_and_limit` alias + entry-vs-card filter duplication (cross-checked by a test — a symptom
the logic should exist once); dead `VlackuRequest::Meaning`; local consonant/vowel tables
duplicating morphology's with an uncommented deliberate `y` divergence; recomputed decompositions
and redundant `.take(count)` (:669-685, :826, :895-938); `(usize, f32, T)` sort triples should be
a named struct.

Verified non-issues: total sort comparator (no NaN panic), guarded `select_nth_unstable_by`,
char-indexed matching (no byte-slice panics), consistent case folding, no debug output/TODOs/
production unwraps.

### Gentufa (`jbotci-gentufa`)

Clean block-collector/visitor pipeline, correct tested XML escaping, panic-safe byte-range slicing
via `str::get`, no raw-string word splitting.

**HIGH:**

- **`show_elided` is silently a no-op** (`lib.rs:240`; hardcoded `is_elided: false` at :585, :603,
  :729). The option is plumbed from CLI (`apps/jbotci/src/lib.rs:2495`) and web
  (`jbotci-web-core/src/lib.rs:946`) but never read; git history confirms commit `bcd1ea0d16`
  ("Remove legacy syntax AST path") removed the collector that implemented it. A large body of
  elision machinery is now unreachable (the elided branch of `push_leaf_or_structural_block`,
  filters, sort-key dimension, annotation matching, strike-through rendering in render.rs:947-950).
  Either reimplement elided emission from the generated model or delete the option and all dead
  machinery — decide explicitly.

**MEDIUM:** whole-subtree `node.clone()` *before* the early-return checks in chain splitting
(:844-862, §1.8); `invariant(true)` cluster incl. the deserialized-layout slice-panic path (§1.2);
dead `TransformInfo`/`transform`/`parent_color` (§1.7); stringly `kind`/`token_kind` (§1.3);
`weighted_circular_mean_hue` boundary panic (§1.1).

**LOW:** redundant/confusing prefix condition in the leaf-part split (:879-887 — remnant of the
removed elision support); silent content drop in `push_payload` root fallback (:500-508);
triplicated leading-pause logic (§1.5); span-vector cloning up the tree (§1.8); duplicated
span-arithmetic helpers with divergent out-of-range policies (render.rs:394-410 vs :566-584);
`TextMeasurer` allocating a key `String` per cache probe and `TextSize` not `Copy` (:264-281);
fonts copied into fontdb twice per PNG render (use `fontdb::Source::Binary(Arc)` and share
`usvg::Options`); **exported SVGs depend on CDN fonts pinned to `@latest`** while Crisa is
embedded — at odds with the self-contained-binary philosophy (render.rs:1086-1120); `occurrence`
should be `Option<NonZeroUsize>` end-to-end and `base_key`'s digit-free-stem injectivity
assumption documented; overpromising constructor-name `ensures` (§1.1).

Verified non-issues: Unicode/byte-span safety, XML escaping, source-order preservation (incl. flat
chain rendering), visitor-protocol panics not user-reachable, no debug output.

### Other tool crates (`jvozba`, `gimfihi`, `phonetic`, `embeddings`, `embedding-inputs`)

Verified correct (no action): jvozba routes all hyphen/tosmabru/bonding rules through
`jbotci-morphology` with full candidate re-parses; gimfihi's CLL 4.14 scoring and
similar-consonant matrix match CLL exactly, candidates re-validated through morphology; ALINE
matches Kondrak's recurrence and published constants; embedding math is correct (L2-normalize,
dot==cosine, SHA-256-validated shards, bounded deterministic top-k, careful checkpoint resume).

**MEDIUM:**

- **jvozba r-hyphen bug** (`lib.rs:806-813`): `should_drop_hyphen`'s third condition is
  tautological (it tests the syllable pattern of `'r'` itself), so a leading `r` is always
  consumed as an r-hyphen; with the greedy non-backtracking splitter, any cmevla-form lujvo whose
  next rafsi genuinely starts with `r` (e.g. `jetrok` from jetce+rokci) fails decomposition —
  silent "No dictionary entry" errors and missing vlacku cards. Fix the following-char check;
  better, replace `sloppy_decompose` entirely (§1.6).
- **gimfihi scoring allocation storm** (§1.8).
- **phonetic tokenizer gap**: no U+0261 `ɡ` (the canonical IPA glyph), no NFD normalization, all
  modifiers rejected — real-world IPA queries fail with "Unsupported IPA segment" while the
  gimfihi transliterator accepts them; the two crates disagree on accepted IPA
  (`lib.rs:210-218, 331-360`).
- **`IpaSegmentId` correctness-by-construction gap** (§1.2 — panic-sites-as-enforcement plus a
  forbidden public `is_valid_ipa_segment_id`).
- **embeddings `Box::leak` per model load** (§1.8) and the **remote-manifest `ensures` panic**
  (§1.1).
- **embedding-inputs swallows embedded-CLL errors** (`lib.rs:228-236, 332-336`):
  `unwrap_or_default()` silently exports a corpus with an empty CLL section and wrong fingerprint
  hashes; `_json()` swallows serialization errors as `"{}"`. Return `Result` — these must be loud.

**LOW:** jvozba dead API + owned-value clones + O(n²) dedup duplicates; gimfihi `String` word-type
in collisions, `rafsi.is_empty()` as an undocumented "not computed" sentinel, const-fn invariant
markers uncommented; phonetic magic constants without Kondrak citation, dead aspiration feature,
inverted `derive_back_value` naming, stringly `PhoneticError::Message`; embeddings Windows
`rename` bug on `--force` re-download (use `rename_replacing`), `PathBuf::new()` sentinel, unused
deps, tautology-spelled ensures, undocumented windowed-mean pooling; embedding-inputs
`{title}`/`{text}` replace-templating fragility, duplicated `hex_digest`.

### Support crates (`cll`, `dialect`, `diagnostics`, `source`, `orthography`, `dictionary`, `dictionary-data`)

jbotci-cll uses real XML parsing (roxmltree) with TOML-driven chrestomathy special-casing and good
test pinning; jbotci-diagnostics has the best contract discipline in the workspace;
jbotci-orthography is clean; jbotci-dictionary-data's build.rs validates sha256/entry-count and
the full in-memory Dictionary before rendering code.

**HIGH:** the `ensures(is_ok_and)` cluster (§1.1), including the never-exercised dictionary import
negative tests.

**MEDIUM:** dialect's private morphology + dead initial-pair machinery (§1.5); builtins re-parsed
recursively per lookup (`lib.rs:241-270, 499-536` — use `LazyLock`; also converts a builtin typo
into a first-use panic instead of a deterministic startup/test failure); **silent character
laundering in dialect normalization** (`lib.rs:1512-1516` — `filter_map(normalize_dialect_char)`
turns `"c%3e"` into `"ce"`, which may validate as a different cmavo than the user wrote — error
instead); O(n)/O(n·m) work inside always-on diagnostics contracts (:740-792 — move to expensive
tier); CLL stringly kinds (§1.3); five divergent CllBlock traversals with a real search gap (§1.5
— decide whether CmavoList/InterlinearGloss content contributes tagged words; likely yes for
CmavoList); examples stored twice with a link-resolution behavioral divergence (§1.8 — resolve
links in `examples_by_id` too and add a standalone-vs-in-section rendering test);
`CmavoDialectEntry` invariant + `is_valid()` (§1.2); CLL model cross-map invariants (§1.2).

**LOW:** `SourceSpan`'s lossy serde round-trip silently conflates byte/char offsets for non-ASCII
sources (`jbotci-source/src/lib.rs:55-105` — documented as v0-compat but nothing marks
round-tripped spans byte-unreliable; consider a `CharOnlySpan` distinction); pre-parse string
replacement of three XML entities (add a why-comment; unknown entities do fail loudly);
content-keyed special cases (`"section-EBNF"`, `"volume-chrestomathy"`, hardcoded EBNF
symbol→section map) belong in vendor-adjacent metadata; chapter numbers derived from
lexicographic filename order with no validation (parse the numeric prefix and fail on mismatch);
duplicate paragraph anchor ids when a `<para>` is split (invalid HTML, ambiguous anchors);
admonition elements (`note`/`tip`/…) flattened to plain text losing links; chrestomathy TOML
`.expect()`ed lazily at first use instead of build-time validation; `escape_html` allocation
chain (§1.8); dialect formula-component equality by allocated `Atom` construction + two
near-identical hand-rolled tokenizers already divergent on stray `)`; dictionary `"brod"` rafsi
exception uncommented; three byte-identical `*_index_matches` helpers; O(n) `ptr::eq` scan where
pointer offset arithmetic is O(1); CLL lib.rs split (§1.4).

### UI (`jbotci-ui`)

Runtime hygiene is good (no `unsafe`, no user-reachable unwraps, sound guard-based task tracking,
idempotent head sync, honest client-side-only compute in workers).

**HIGH:**

- **25,216-line lib.rs** (§1.4) and **dead, drifting `platform.rs`** (§1.7 —
  `stable_jvozba_pane_top` duplicated verbatim; `platform::place_tooltip` is a *stale* copy
  missing the `viewport.top` handling the live one has). Either adopt the trait layer (it is the
  right shape to collapse the 145×wasm/86×desktop cfg-gate explosion) or delete it.
- **God-component with zero memoization**: every keystroke in the gentufa textarea (and TOC
  filter, and activity toggles) re-executes the entire `AppShell` build — ~20 wholesale page-state
  clones, page-find entries rebuilt and hashed from all visible text even with an empty find query
  (the empty-query early-out happens *after* collection). Gate entry collection on non-empty
  query, add `use_memo`, move per-page state into per-page `#[component]`s.
- **WebGPU q4 matmul stride bug** (`f2llm_webgpu_runtime.rs:137-143` vs the exporter's padded
  `groups * group_size` row stride in `tools/embedding-pack/f2llm/export-webgpu-from-onnx-q4.py:
  255-256`): wrong dequantization offsets for every row past the first on any
  `in_cols % group_size != 0` shape — silent garbage embeddings. Latent only while shipped shapes
  divide evenly. Fix the index or hard-reject non-divisible shapes at load.

**MEDIUM:** page-find highlight misattachment under memoization (lib.rs:650-685, 10467-10604 —
text keys assigned by a shared render-order counter; memoized components skip renders and consume
no ordinals, shifting every later key ⇒ wrong/missing `<mark>`s; this hidden invariant also blocks
the rest of the app from adopting memoization — derive keys from stable content-path identity);
WebGPU contracts panicking on untrusted network data (e.g. `score_f16_vectors` requires `"f16le"`
while the spec parser defaults absent `elementType` to `"f32le"` — demote to validation returning
`Err`); sequence-length caps combined with `.max()` instead of `.min()`
(`f2llm_webgpu_runtime.rs:631-635` — windows can exceed the trained RoPE range and desync from the
worker's cap); **no WebGPU error capture anywhere** (shader validation failures yield all-zero
"embeddings" that pass normalization — add error scopes around pipeline creation and submits);
shader-relied model invariants unvalidated (heads % kv_heads, token ids vs vocab bound);
**diagnostics re-lexing subsystem** (lib.rs:14028-15017 — recovers structure from rendered
diagnostic *strings* via `"needs one of:"` matching, ~250 hard-coded phrase/keyword entries, and
an ~80-entry construct→EBNF-href map with silent `None` fallback; any wording change in
jbotci-diagnostics silently breaks highlighting/links — emit structured segments/identifiers from
jbotci-diagnostics instead, as it already does for styled notes); Rust/JS model catalog
duplication (§1.5); compute-worker polling of wasm-bindgen internals (`__wasm`/`__dx_mainWasm`,
10ms × 30s — export an explicit readiness promise from the app module).

**LOW:** ~18 near-identical cfg-triplicated scheduling functions (one generic
`schedule_layout_pass` with per-platform backends); 35 ungated `console.info` calls in the JS
workers (AGENTS.md requires gating); vacuous contracts on `ReferenceRect`/`ElementSize`/
`AsyncActivityState` and tautological ensures across the runtime (§1.2); `PageFindContext::new`
cloning a map it could move; WebGPU misc (dead `encode_truncated`, `clone_for_load`, unreachable
empty-corpus branch, silent `""` for non-string worker input, unbounded GPU-side `vector_buffers`
cache, unvalidated 4-byte alignment of manifest chunk offsets, O(seq²·head_dim²) attention
recomputation + per-layer GPU syncs, WGSL workgroup sizes implicitly coupled to Rust `div_ceil`
constants); 14 desktop `document::eval` JS blobs (acceptable Dioxus-desktop workaround —
consolidate into named constants with comments, fold into the platform backend layer);
uncommented subscription-only reads `let _ = (…)`.

Verified non-issues: head sync removes previously-tagged nodes; guarded listener installs; correct
uniform layouts, dispatch rounding, RoPE, softmax, RMSNorm, and `map_async` ordering in the
runtime; clean build.rs; client-side-only rule honored.

### Web core & server (`jbotci-web-core`, `apps/jbotci-server`, `apps/jbotci-app`)

web-core is a well-typed view-model crate with correct escaping and extensively tested URL
round-tripping; the server is small and careful (path traversal guarded + tested, ed25519 Discord
verification, Origin-checked MCP with strict per-tool validation, heavy parses in
`spawn_blocking`); jbotci-app is a clean thin shell (no findings).

**MEDIUM:**

- **Gentufa web annotations ship hard-disabled** (`jbotci-web-core/src/lib.rs:524, 545, 556-559`):
  blocks never get glosses/definitions/tooltips; features default to false; **9 tests are
  `#[ignore]`d** with "temporarily disables" messages and no tracking issue. Restore the wiring or
  excise + file a Codeberg issue referenced from the code.
- **Tree rows reconstructed by string-scraping pretty-printed output** (`lib.rs:562-599,
  873-888`): depth = leading whitespace ÷ 2, label = scan-until-delimiter, `parent_id: None`/
  `has_children: false` for every row — the "operate on rendered strings instead of the model"
  hack the project forbids, producing a structurally degenerate tree. Walk the generated model
  instead (the blocks layout already does).
- **Blocking `ureq` call on a tokio worker thread** (`discord.rs:124-138, 589-611`), no explicit
  timeout — slow Discord API calls can starve the executor. Wrap in `spawn_blocking` + shared
  Agent with timeouts.
- **Consumer-less `/api/gentufa` endpoint** (`lib.rs:381, 452-474`): unauthenticated server-side
  parsing with no user anywhere in the repo — architecture drift from the client-side-only rule
  plus free DoS surface. Remove (also unused `/api/features`) or document as a deliberate public
  API with abuse protection.
- **Static assets read fully into memory per request + brotli negotiation without
  `Vary: Accept-Encoding`** on `public, max-age=31536000, immutable` responses
  (`lib.rs:695-719, 751-769, 881-893`): a shared cache/CDN can store the brotli variant and serve
  it to non-brotli clients for a year. Stream bodies (tower-http `ServeDir` with precompressed
  support handles both) or at minimum add `Vary`.

**LOW:** `percent_decode` slices the `&str` by raw byte offsets — panics on `%` within 2 bytes of
a multibyte UTF-8 char; wasm-reachable on route strings (decode over bytes; `lib.rs:5440-5464`);
`base_path` invariants (§1.2); hardcoded 23-entry CLL media dimension table silently dropping
og:image for unknown files (derive at build time); identical-branch conditional (:3290-3295); dead
`trim_web_float`; `base_path_from_canonical` substring heuristic (§1.6); stringly web state
(§1.3); duplicated `escape_html_text` and composition-piece mapping (§1.5); Discord
command-registration payload maintained solely for a test (make it a real `setup` subcommand or
remove); `run_embedding_request`'s blocking send/recv safe only by the implicit "all callers are
in spawn_blocking" invariant, plus redundant `Arc<Mutex<Sender>>` (Sender is Clone/Sync); dead
match differentiation in `notification_response`; hybrid 400-with-interaction-body Discord error +
unvalidated `application_id`/`token` interpolated into the outbound webhook URL (cheap
defense-in-depth); stale `#[allow(dead_code)]` on live aliases (:75-80).

Verified non-issues: path traversal (`safe_relative_path` + regression test), host-header escaping
into og:url, MathML XSS escaping (test at lib.rs:8208), MCP spawn_blocking discipline,
char-boundary-safe Discord truncation with `allowed_mentions: []`, env-overridable defaults.

### CLI & output (`apps/jbotci`, `jbotci-output`)

Terminal handling is done right (`terminal_size` + `concolor`, no width guessing); span math uses
`unicode-width` on pre-ANSI text; the QR encoder was verified line-by-line against ISO/IEC 18004
(mode/count bits, capacity, EC tables, GF(256)/RS, interleave, function patterns, format/version
BCH, zigzag placement, all 8 masks, penalties) with **no encoding bugs found**.

**HIGH:**

- **`compact_json_value`'s false postcondition** (§1.1).
- **9.7k-line CLI monolith** with a parallel `Tool*Request` MCP surface bridged by hand-written
  field-by-field `Command` reconstruction at 6 sites (lib.rs:2478, 2521, 2637, 2666, 2706, 2750 —
  drift-prone as options are added). Split into modules (`cli`, per-command modules, `tool`,
  `output`, tests into `tests/`); replace bridges with `From`/`TryFrom` impls colocated with each
  input type.

**MEDIUM:**

- **JSON colorization by re-lexing the serialized string** (`apps/jbotci/src/lib.rs:5891-5987`)
  with the uppercase-first-letter constructor heuristic (§1.6) and per-token allocations.
  Colorize during rendering from the structured value (jbotci-output already knows which keys are
  constructors).
- **Tree renderer deep-clones every subtree per entry per node** (tree.rs:2999, 3234-3241; §1.8)
  via the redundant `RenderEntry` tuple-variant duplicate of `TreeEntry`. Delete it; match on
  borrows.
- **QR types' missing invariants** (§1.2) and **wrong-diagnostic dead-error path**: any
  non-alphanumeric character makes `select_version` report "payload too large for version 40-H"
  while the precise character error is unreachable (`qr_code.rs:427-441, 472-479`) — validate
  characters once up front (also removes up-to-40× payload recomputation during version search).
- **QR tests verify no known-good matrix** (:1218-1263): a transposed mask formula, swapped
  format-info copy, or RS bug would pass every current assertion. Add a known-answer
  `dark_modules` test + `format_bits`/`version_bits`/Reed-Solomon spec vectors.
- **Compact-JSON `show_elided` path patches the serialized DOM** (key-shape sniffing, max-char_end
  scanning, hand-assembled nested-map synthetic tokens; lib.rs:399-527, 854-862, 1005-1019) while
  the tree/bracket renderers do the same job on the typed tree; the constructor→payload-field
  table is duplicated verbatim (lib.rs:948-978 vs json.rs:294-312). Build from the typed tree;
  extract one shared table.
- **annotate-snippets gutter-width guess** (§1.6).
- **Trace `terminal_width` dead but `requires`-guarded** (trace.rs:11-14; §1.7) — implement
  wrapping or delete.
- **Dead `with_indicators_value` cluster** duplicating live brackets.rs logic (§1.7).
- **`word_leaf` deep-clones the whole `Word` per bracket leaf** (brackets.rs:308-328) — add a
  borrowed surface-formatting helper.
- **Contract density dominated by no-op markers** (336 `requires(true)`/220 `ensures(true)` in the
  CLI; 161/16 in tree.rs, incl. `TreeNode`/`TreeEntry` with real constraints; §1.2).

**LOW:** `quoted_text_leaf` trims text but keeps the untrimmed span (brackets.rs:330-340 — breaks
text↔range correspondence for padded ZOI payloads); `let _ = joined_query_text(…)` dead work
masquerading as validation (lib.rs:3594); `minimum_by_penalty` cloning all 8 QR candidates +
re-evaluating penalties (use `min_by_key`), logo placement rebuilding function patterns per
candidate per render; N3 finder penalty undercounted at symbol edges (mask-choice quality only);
dead `QR_LOGO_TEXT`/pub QR surface + swallowed length mismatch via `unwrap_or_default` + missing
ISO provenance comments; `ReferenceName` invariant duplicated on both wrappers, by-value
annotation translation cloning every field, full render tree materialized to read one word's text
(references.rs); triple-computed `visible_segment_text`, per-key `Phonemes::from_canonical(clone)`,
`compact_single_payload` deep-cloning a payload its caller owns.

Coverage note: the CLI's ~6,000 product lines and tree.rs:400-2900 received verified
spot-findings, not an exhaustive function-by-function sweep; a follow-up pass could surface
additional medium/low items.

### Build & test tooling (`xtask`, `xtask-full`, root crate, scripts, deploy)

xtask is clean; xtask-full's individual pieces are careful (exact-string expectation comparison,
status-first xfail logic, ordered R2 uploads, staged promotion with rollback); deploy has no
secrets and validates commit hashes.

**HIGH:**

- **`fixture-rewrite --migrate-morphology-diagnostics` blindly re-derives fixture status from
  current parser output** (`xtask-full/src/main.rs:7167-7315, 7368-7399`): the
  `migrate_success_morphology_now_failure` trigger fires for *every* Success fixture — a
  morphology regression becomes silently enshrined as expected failures (raw cleared, vlasei
  output deleted), with no status-flip report, no dry-run, and no restriction to the legacy
  placeholders that are the flag's nominal purpose. Restrict to
  `is_legacy_morphology_placeholder` matches; move wholesale re-derivation behind a separate
  explicitly-named flag that prints old→new status per fixture and refuses a dirty git tree.
- **Hardcoded machine-specific ONNX paths** (`main.rs:90-127, 2368-2369, 2418`):
  `/home/int19h.linux/git/jbotci-f2llm-quant/...` as defaults and — for the 160m/330m/0.6b specs —
  non-overridable values. The f2llm asset pipelines are unusable on any other machine/CI. Make the
  artifact root a required arg/env; store relative names in `F2LlmAssetSpec`.

**MEDIUM:**

- **CLL brackets comparison normalizes away pause dots, stress accents, and glide breves**
  (`main.rs:11118-11161`): a renderer regression dropping those is invisible to the entire CLL
  corpus, and the normalization is inconsistent (the vlasei brackets facet compares exactly).
  Confirm approval; preferably regenerate expectations to the exact rendered form and delete the
  normalization.
- **xtask/xtask-full ~800-line duplication** incl. the service-worker template (§1.5) and the
  hand-maintained `should_run_light_command` routing list. Extract `xtask-common`.
- **Worker summary parsed before exit-status check** in `fixture_rewrite_subprocess_chunks`
  (:6979-6991): a crashed worker reports "did not print a summary" instead of its exit status (the
  sibling fixture-test path gets the order right).
- **Line-oriented TOML surgery on fixtures** (`replace_syntax_diagnostics_line` :8058-8095 lacks
  multiline-string skipping; `replace_gentufa_output_sections` terminates its skip on a contained
  delimiter; worker summaries scraped as `fixtures=N` lines with typo'd keys silently zeroing
  counters). Latent today (current fixtures keep `raw` as single-line literals) but the worst-case
  failure is expectation-file corruption. Round-trip through `toml_edit`; JSON-encode worker
  summaries.
- **Fixture support types: `invariant(true)` + public `is_valid()`** (§1.2).

**LOW:** legacy-format rejection by raw-text substring scan of whole files (a corpus fixture whose
translation contains `constructor =` is rejected with a misleading error — detect structurally;
`tests/support/fixtures/mod.rs:1055-1088`); `.jbotci-asset-sync/` temp dir leaks into the shipped
web bundle and Docker image; blunt remove-all-`"span"`-keys comparer + serialization errors
conflated with "trees differ" (:10487-10505); `run_on_normal_stack` no-op + last-range check gap
in `ranges_are_strictly_ordered` (tests/source_assignment.rs:130-225); xtask-full missing
`publish = false`; chunk `--path-prefix` exactness incidental on the `.toml` suffix (add an exact
`--path` selector).

Verified non-issues: all other expectation facets compare exact strings; xfail correctly fails
"unexpectedly passed" and refuses status flips; the default rewrite mode refreshes payloads only
for the recorded status; rayon failure accounting; R2 catalog-after-data ordering; wiki-vendorer
staged promotion/rollback; no TODO/FIXME or un-gated debug output; scripts and deploy are clean.

## Part III — Suggested roadmap

Ordered by (correctness impact × effort):

1. **Contract-soundness sweep** (mechanical, high value): fix every §1.1 instance; add a scanner
   rule; enable the dictionary `import` feature in CI.
2. **User-facing correctness**: morphology selma'o table → typed (fixes SA erasure); syntax `ku`
   grammar rule + fixture; glob DP (DoS); `ko'a goi` stub; SE-chain fold; CEI grouped-tanru gap;
   jvozba r-hyphen; glob/dialect silent character dropping; phonetic ɡ/NFD; `handle_sa` error
   swallowing.
3. **Tooling safety**: gate the fixture-rewrite migration; parameterize f2llm paths; decide the
   CLL brackets normalization.
4. **Feature triage**: `show_elided`, gentufa web annotations, `grammar-debug` — each is
   reimplement-or-delete, decided explicitly with an issue.
5. **WebGPU hardening**: q4 stride, error scopes, `.min()` caps, untrusted-input contract
   demotion; embedding-inputs loud errors.
6. **Contract quality campaign**: §1.2 invariants crate by crate (server base_path, QR, gentufa,
   search, CLL, fixtures, phonetic are small self-contained wins); replace tautology ensures.
7. **Structural refactors** (test-suite-protected code motion): split generated_builder.rs
   (+dedup its dispatch), UI lib.rs (+platform.rs decision + memoization strategy), CLI lib.rs,
   CLL lib.rs, xtask-common; unify references.rs traversals on `TreeVisitor`; one CllBlock
   visitor; delete the dead-rule set and narrow syntax's blanket `allow(dead_code)`.
8. **Typing campaign**: `RelationLabel`, `PlaceIndex`, word-type/CLL-kind/marker-kind/
   syllable-shape enums, typed `Selmaho` in SA tags, structured diagnostics segments for the UI
   (deletes the 250-entry tables), `ConsonantPairClass`.
9. **Consolidation**: one diacritic-stripping home; dialect on real morphology; jvozba
   decomposition into morphology; shared escape_html/hex_digest/true-marker predicate; Rust→JS
   catalog generation; morphology phonology helpers unified.
10. **Allocation/clone cleanups** (§1.8) as each area is touched, with the morphology cmavo-check
    path, gimfihi scoring loop, and tree renderer as the highest-value targets.
