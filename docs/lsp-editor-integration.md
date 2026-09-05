# Lojban language server & editor integration

Status: design accepted 2026-07-15 (brainstorm thread with owner). This document
specifies **Milestone 1** of the jbotci language server in implementable detail,
sketches M1.5 and M2, and records the codebase findings the plan rests on.

Nothing like this exists for Lojban: prior art is regex-based syntax
highlighters that cannot even segment words correctly (Lojban word boundaries
require the morphology parser — spaces are not reliable). The value proposition,
in order: correct error reporting, dictionary-integrated editing, and
eventually the semantic layer (place structure, anaphora) surfaced live.

## Where this fits

```
editors/vscode/          TypeScript extension: LSP client, language wiring, gating UX
apps/jbotci              `jbotci lsp` subcommand: stdio JSON-RPC transport, lsp-types marshalling
crates/jbotci-ide        NEW: transport-agnostic analysis snapshot + queries (wasm-safe)
crates/jbotci-web-core   parse orchestration (parse_gentufa_for_web) — reused
```

Decisions already made:

- **`jbotci lsp` is a subcommand of the main CLI binary**, not a separate
  binary. Single self-contained binary is project doctrine; it also makes
  manual editor integration trivial for Vim/Helix/Kakoune users (point the
  editor at `jbotci lsp` over stdio, done).
- **`jbotci-ide` is a new wasm-safe crate** exposing pure queries over a
  document snapshot. No I/O, no async, no `lsp-types`. It is the single
  analysis surface for the LSP adapter, the future in-browser editor (SPA /
  vscode.dev), and the future SCIP corpus exporter. The LSP adapter in
  `apps/jbotci` is a thin marshalling layer and must stay that way.
- **Do not extend `jbotci-server`** — it is tokio/axum/native-only by design.
- LSP server library: lean toward `async-lsp` over `tower-lsp`; hand-rolled
  JSON-RPC (per the MCP-handler precedent in `apps/jbotci-server/src/mcp.rs`)
  is the fallback if neither fits. Decide at implementation time; the choice is
  contained entirely within the adapter layer.

## Milestone map

- **M1 — "correct where nothing else is"**: diagnostics, hover definitions,
  expectation-driven completion, morphology-driven semantic tokens, tier-1
  permissive lexer, VS Code extension + `jbotci lsp`. Everything in M1 runs on
  the recovery-capable morphology+syntax pipeline and the dictionary; **nothing
  in M1 depends on `jbotci-semantics`**, which is what makes it independently
  shippable while semantic work is ongoing.
- **M1.5 — "pure-syntax display & hosting"**: structure inlay hints (brackets
  profile), Markdown structural hosting, fenced-block injection grammar,
  morphology-based detection offer.
- **M2 — the semantic layer**: gated on porting the reference/place analysis to
  the recovered tree model (Appendix A). Place inlays, signature help, anaphora
  navigation/highlights, rename, code actions, pandi decoration profile.
- **Later (filed vaguely, by design)**: cukta sidebar, parametrized parse
  command UX, Markdown preview integration, SCIP corpus export.

---

# Milestone 1 specification

## Scope

**In:** live morphology/syntax diagnostics; hover word definitions; word
completion (content words *and* cmavo) driven by the parser's expected-token
machinery; morphology-driven semantic tokens; tier-1 permissive lexer; VS Code
extension; `jbotci lsp`.

**Out (M1.5/M2):** everything that consumes `jbotci-semantics` (place
resolution, anaphora, semantic inlays, signature help), structure inlays,
Markdown structural pass, pandi, formatting.

## Document pipeline

Per open document, `jbotci-ide` maintains:

```
DocumentSnapshot {
    text, version,
    line_index,          // LineIndex, see below
    words,               // RecoveredMorphologySegmentation
    parse,               // SyntaxRecoveryParse (recovered tree + errors)
    diagnostics,         // Vec<jbotci_diagnostics::Diagnostic>
}
```

- **Sync**: LSP incremental sync (`TextDocumentSyncKind.Incremental`), applied
  as a string splice via `LineIndex`. Incremental *sync* ≠ incremental
  *parsing*: we rebuild the text cheaply and reparse the whole document.
- **Reparse**: debounce ~200 ms; a generation counter cancels superseded
  parses. All requests answer from the latest *completed* snapshot; pull
  diagnostics may await the in-flight parse.
- **Producer**: the orchestration in `jbotci-web-core::parse_gentufa_for_web`
  (recovered morphology → diagnostics → recovered syntax → diagnostics) is the
  reference implementation; lift/share rather than duplicate, so the SPA and
  LSP cannot drift.
- **Perf budget**: the recovery-perf work (#355 chain) put pathological
  error-text parses at ~82 ms release — comfortably inside the debounce window.
  Completion performs one extra prefix parse per request (see below); budget
  ≤ 1 full parse + 50 ms.

## Position mapping (`LineIndex`)

New in `jbotci-ide`. Facts motivating it:

- `SourceSpan` (`crates/jbotci-source/src/lib.rs:45`) carries byte + char
  offsets; its `start`/`end: Option<LineColumn>` fields are **never populated**
  by the pipeline.
- The only line/column helper, `char_offset_to_line_column`
  (`crates/jbotci-syntax/src/lib.rs:2822`), counts Unicode scalars, not UTF-16
  code units; the only UTF-16 handling in the tree is a one-off in
  `crates/jbotci-ui/src/diagnostics.rs`.

`LineIndex` maps LSP `Position` ⇄ byte/char offsets ⇄ `SourceSpan`: line-start
table plus per-line UTF-8/UTF-16 conversion. Negotiate
`positionEncoding: "utf-8"` (LSP 3.17) and fall back to a *correct* UTF-16
path — Lojban text is mostly ASCII today, but ZOI payloads are arbitrary and
pandi glyphs (`«»`, `·`, `₂`, `𝙰` — astral) are coming. Contract-heavy
(bityzba) and property-tested over multi-byte/astral inputs: silent bugs here
poison every feature downstream.

## Feature: diagnostics

Producer exists; M1 is mapping. `Diagnostic`
(`crates/jbotci-diagnostics/src/lib.rs`) → LSP:

| jbotci | LSP |
|---|---|
| primary `DiagnosticLabel.span` | `range` |
| secondary labels | `relatedInformation` (same-URI locations) |
| severity `Error` / `Warning` / `Advice` | `Error` / `Warning` / `Hint` |
| `code` (stable strings, e.g. `syntax.unexpected-cmavo`) | `code` |
| `phase` | `source`: `"jbotci/morphology"` or `"jbotci/syntax"` |
| `notes` + `styled_notes` (incl. "expected one of:") | appended to `message` (multi-line); segments flattened via the shared segments→markdown/text helper |

- Implement **pull diagnostics** (`textDocument/diagnostic`) as primary and
  push (`publishDiagnostics`) as fallback for clients without pull support —
  one producer, two adapters.
- `codeDescription` (CLL links) is deferred; the breadcrumb laid now is that
  diagnostic codes stay stable and CLL section references become attachable in
  the ide layer.

## Feature: hover

- **Word at cursor** from the snapshot's segmented word stream via span
  containment — never from raw text. **Hover range = the rendered dictionary
  unit's full span**: normally one morphology word, the full attested cmavo
  sequence when that card replaces a constituent, or the full ZEI compound.
- Content by classification (`Word` / `ValsiAnalysis`):
  - **gismu** — definition with indexed place rendering (the vlacku
    `sumti_places`-style rendering), rafsi, gloss keywords.
  - **lujvo** — veljvo decomposition (already carried by `Word::lujvo(parts)`)
    rendered `rafsi·rafsi → gismu + gismu`; jbovlaste entry if present, else
    stacked component cards separated by horizontal rules.
  - **ZEI compounds** — the canonical compound headword and classification;
    its own dictionary card when attested, otherwise the attested component
    cards separated by horizontal rules.
  - **cmavo sequences** — replace the constituent card with only the longest
    dictionary-attested contiguous sequence that contains it. Contiguity
    follows the segmented word stream, with only whitespace and periods allowed
    in gaps between its source spans; equal-length candidates retain source order.
    Attestation uses a morphology-derived index independent of dictionary tags,
    so compact, spaced, and mixed headwords share canonical component keys.
    Web Blocks uses the same index with a global longest-first, leftmost partition.
    Hover remains cursor-local: in `ba pu ba`, Blocks groups `ba pu`, while hover
    over the final `ba` documents `pu ba`.
  - **fu'ivla** — dictionary entry if present, else morphological
    classification.
  - **cmevla** — classification only (binding info is M2).
  - **Quote payloads** — none inside `zoi`/`la'o` payloads; morphology-only
    inside `lo'u…le'u`.
- Card headings keep the classification compact on the headword line, including
  cmavo selma'o; pure layout labels are omitted.
- Lookup reuses the vlacku pipeline
  (`run_vlacku_requests(dictionary, requests, options) → VlackuSearchOutput`
  cards); the new work is a **card/segments → Markdown renderer** (the existing
  renderer targets terminals). This helper is shared with diagnostics notes and
  completion documentation. Semantic (embeddings) vlacku search is explicitly
  *not* part of hover.

## Feature: completion

### The expectation machinery (as found, 2026-07)

`SyntaxError::Parse` (`crates/jbotci-syntax/src/lib.rs:384`) carries
`expectations: Vec<SyntaxExpectation>`:

- `SyntaxExpectation { tokens: Vec<SyntaxExpectedToken>, reason: SyntaxExpectationReason }` (lib.rs:1726)
- `SyntaxExpectedToken` is typed: `Cmavo(Cmavo)` | `Selmaho(Selmaho)` |
  `WordCategory(Brivla|Cmevla|SelbriWord|ProSumti|LetterWord|Quote)` |
  `EndOfInput` | `Named(String)` (lib.rs:485)
- `SyntaxExpectationReason`: `ContinueCurrent{construct}` |
  `StartNested{construct}` | `EndThenStart{starts, ends}` (lib.rs:1706), with
  merge/dedup post-processing (`merge_expectations_by_reason`,
  `retain_innermost_continue_expectations`).

This is a completion *menu structure*, not just a token set. The gap:
expectations only materialize on a parse **error**. A valid-and-complete prefix
(`mi klama le zarci|`) parses cleanly and yields none, though many words may
legally follow.

### New parser API

```rust
// jbotci-syntax
pub fn expected_continuations(words: &[WordLike], options: &ParseOptions)
    -> Vec<SyntaxExpectation>;
```

Contract: the reason-grouped expectation set for "what may follow this word
sequence", **including** when the sequence is a complete valid text (then
continuations include further terms, `.i`, `vau`, `ni'o`, …). Implementation
belongs inside the grammar crate (e.g. parse with an unmatchable sentinel and
harvest expectations at the sentinel), reusing the existing
`expected_groups`/merge machinery — not simulated from outside. Must be
verified against the memoized recovery path: expectations must reflect the cut
point, not a recovery resumption.

### Cursor → (seed, preceding words)

All morphology/orthography-driven; no space-based word-boundary assumptions.

- **Seed** = longest trailing run of word-forming characters (Lojban letters,
  `'`, `,` — alphabet membership per the orthography layer; this is a
  character-class fact, not a boundary guess) in `text[..cursor]`.
- **Preceding words** = recovered segmentation of `text[..cursor − seed]`.
- Both interpretations are offered, because Lojban makes them genuinely
  ambiguous:
  - **Extend**: candidates with the seed as prefix (`ba|` → `barda`, `bajra`) —
    valid even when the seed is itself a complete word (`ba` the cmavo).
  - **Continue**: when the seed segments cleanly into complete word(s),
    additionally offer next-word candidates after `preceding + seed`
    (empty prefix). Completed-word Continue items rank above Extend items.
- **Morphology validity filter**: a candidate is offered only if
  `preceding + candidate` re-segments such that the candidate surfaces as an
  actual word (prevents proposing words that would fuse with adjacent text).

### Candidate generation

Each `SyntaxExpectedToken` from `expected_continuations(preceding)` expands,
prefix-filtered against the seed (sorted `word_index` +
`partition_point` range scan — a small addition to
`crates/jbotci-dictionary/src/lib.rs`; apply the same normalization as
`lookup_words`):

| Expected token | Candidates |
|---|---|
| `Cmavo(c)` | that cmavo |
| `Selmaho(s)` | all cmavo of selma'o `s` (`entries_by_selmaho` / `Cmavo` table) |
| `WordCategory(Brivla)` / `SelbriWord` | dictionary brivla (gismu, lujvo, fu'ivla) |
| `WordCategory(ProSumti)` / `LetterWord` | KOhA / BY sets |
| `WordCategory(Cmevla)` | cmevla harvested from the current document (open class; only useful source) |
| `Quote` | quote-opening cmavo |
| `EndOfInput` / `Named` | nothing |

**Magic-word contexts** come from segmentation, which already models quote
constructs: inside a `zoi`/`la'o` payload → no completion; after `zo` / inside
`lo'u…le'u` → unfiltered word list (anything is grammatical). Erasure
(`si`/`sa`/`su`) needs no special-casing — it happens upstream.

### Presentation

- **Ranking** via `sortText`, grouped by reason: `ContinueCurrent` <
  `StartNested` < `EndThenStart`.
- **`detail`** from the reason: `continues sumti`, `starts relative clause`,
  `ends sumti · starts bridi-tail`.
- **`kind`**: brivla → `Function` (they are predicates), pro-sumti →
  `Variable`, cmavo → `Keyword`, cmevla → `Value`, terminators → `Operator`.
- **`labelDetails.description`** = short gloss keyword; full definition with
  places resolved lazily via `completionItem/resolve`.
- Insert text = bare word; no automatic pause periods (BPFK: convention, not
  obligation). Whether cmevla completions include conventional periods
  (`.djan.`) is an open question to settle in review.

## Feature: semantic tokens

Morphology-driven highlighting off the snapshot word stream — nearly free and
the most visible "does it right where regex can't" demo (correct even with no
spaces between words). Token legend: word classes (gismu / lujvo / fu'ivla /
cmevla) + selma'o groups (sumti-words, selbri-words, connectives, terminators,
quotation, numbers, attitudinals/UI, tense/BAI). Modifiers reserved for later
(erased-by-`si` → `deprecated` renders strikethrough; elidable terminators).
Purely token-level in M1 — no tree-derived modifiers yet.

## Feature: tier-1 permissive lexer

Current behavior (corrected during #402 implementation): the segmenter already
has a **legacy hand-selected separator set** — `segment::is_separator`
(`crates/jbotci-morphology/src/segment.rs`) treats whitespace, periods, and a
fixed list (`?` `!` `;` `:` `-` brackets, guillemets, quote marks, …) as
word-boundary separators; characters outside that set are
`MorphologyErrorKind::InvalidCharacter` errors, which recovered segmentation
survives by erroring-and-continuing. This tier generalizes the legacy set
(pandi §7.1 envisioned the same); the legacy set's behavior is frozen in both
modes — fixtures pin it.

New: a **dialect-gated ignorable tier** — characters in Unicode
punctuation/symbol/emoji classes act as word-boundary whitespace. Makes
Discord paste, stray `?!`, emoji, and *simple* Markdown (`#`, `*`, `-`, `>`)
just work.

- Defined as "Unicode P*/S*/emoji **minus an explicit reserved list**":
  everything with morphology or future-substitutive meaning is reserved —
  `.` `,` `'` (morphology proper); digits, `$`, subscript characters (pandi
  substitutive families will claim them). `-` is already a legacy separator
  (and pandi's ignorable set includes between-word `-`), so it stays a
  separator rather than reserved. The reserved list constrains only
  *newly*-ignorable characters. A character-class rule, not a heuristic.
- Lives **in the segmenter with native spans** (option in `jbotci-dialect`),
  never as a preprocessing strip — spans must point into the user's real text.
- Masked-typo tradeoff: an accidental `@` mid-word now silently splits a word
  in permissive mode (legacy separators like `;` already did). Strict dialects
  keep `InvalidCharacter` for newly-covered characters; the permissive dialect
  emits a single Advice-level "N non-Lojban characters ignored" note (counting
  only newly-ignorable characters) rather than per-character spam.
- Explicitly **cannot** make real Markdown work by itself: link URLs, code
  fences, HTML blocks are *regions*, not characters (URLs are made of valid
  Lojban letters and would lex into garbage). That is M1.5's structural pass.

## VS Code extension

- Language `lojban` for pure files (`.jbo`; possibly `.lojban`), with a
  `language-configuration.json` whose **word pattern includes `'`** (double-
  click selection, word-based fallbacks).
- **Markdown hosting via provider stacking** (the cSpell/LTeX model): VS Code
  language features stack per language; an LSP documentSelector of
  `{ language: "markdown", pattern: "**/*.jbo.md" }` attaches all Lojban
  features while keeping every built-in Markdown feature (preview, outline,
  links). **Do not register `.jbo.md` as a separate language** — by suffix it
  already *is* `markdown`, and re-registering would strip those features.
- Plain `**/*.md` participation is **opt-in and labeled** in M1 (setting /
  front-matter key / status-bar toggle): simple Markdown works via tier 1, but
  URLs/fences produce noise until the M1.5 structural pass. Bad diagnostics on
  a URL would undercut the core value proposition; users who flip the switch
  accept the edge.
- Auto-detection reality: VS Code's ML language detection is not extensible
  and LSP has no detection concept. M1.5 adds our own offer based on running
  the real morphology over the document (see below).
- Server discovery: setting → `$PATH`. Bundled binaries/marketplace publishing
  is a later distribution task.

## Testing

- **`jbotci-ide` fixture tests** as the workhorse, mirroring the existing
  fixture discipline: source with a cursor marker → snapshotted
  hover/completion/diagnostics/semantic-token output; expectation updates
  follow the same manual-verification rules as parser fixtures.
- **Protocol integration tests**: spawn `jbotci lsp`, drive JSON-RPC over
  stdio through initialize → didOpen → didChange → each request; assert on
  wire-level responses, including position encoding both ways.
- **Corpus smoke + property tests**: snapshot pipeline over the fixture corpus
  (no panics, perf envelope); property tests for `LineIndex` round-trips over
  multi-byte/astral inputs.
- **No-op guard** (mandatory for completion): a test must fail if expectation
  filtering silently degrades to "all dictionary words".

## M1 open questions (settle in implementation review)

1. `Advice` → LSP `Hint` (chosen above; faded rendering) vs `Information`.
2. Document-local cmevla harvesting for completion — in (specced) unless it
   proves noisy.
3. Cmevla completions with conventional periods (`.djan.`) or bare.
4. `async-lsp` vs `tower-lsp` vs hand-rolled JSON-RPC.

---

# Milestone 1.5 — pure-syntax display & hosting

## Structure inlay hints (brackets profile)

The idea: what the brackets output format shows statically, inlaid dynamically
into the user's own text as they type. Pure syntax — and the substrate is
already **recovery-native**, unlike the semantic overlay:
`pretty_recovered_syntax_brackets_with_options`
(`crates/jbotci-output/src/recovered.rs:497`) walks the recovered tree
directly with `RecoveredBracketBuilder`, and
`pretty_recovered_syntax_bracket_source_fragments_with_options` returns
**source fragments** — structure elements tied to source positions, i.e.
inlay-shaped data. Works mid-error today.

- **Interaction model**: designed around VS Code's
  `editor.inlayHints.enabled: "offUnlessPressed"` — hold Ctrl+Alt, structure
  appears; release, clean text. Full always-on bracketing is unreadable;
  chord-peek is the primary UX. Also offer depth-limited and
  construct-filtered profiles (only sumti boundaries, only bridi-tails, …).
- **Architecture**: build as a *decoration profile over tree-anchored
  fragments*. pandi (M2+) is the same engine with a different glyph
  vocabulary — raw brackets are the debug/learner profile, pandi the reader
  profile. One inlay engine, N profiles.

The VS Code surface is one kind-keyed object, `jbotci.inlays`. The extension
passes the same object as `initializationOptions.inlays`; each kind is
independently enabled:

```json
{
  "structureBrackets": true,
  "wordBoundaries": false,
  "rafsiBoundaries": true
}
```

`structureBrackets` may instead contain its profile object (`profile`,
`maxNestingDepth`, and `constructs`). The pre-kind-keyed server shape,
`initializationOptions.structureInlays`, remains accepted for one release and
produces a `window/logMessage` deprecation warning.

## Markdown structural hosting

A CommonMark structure pass (pulldown-cmark, or owned — the spec is small)
classifies the document into **visible prose** (→ the Lojban stream) and
**syntax regions** (link destinations, fences + info strings, HTML blocks,
front matter → skipped, spans preserved). Not a heuristic — CommonMark is a
spec.

- Requires a **filtered-text overlay** abstraction (virtual text ⇄ original
  spans), in or near `jbotci-source`. The same abstraction serves fenced-block
  virtual documents and any future embedded-Lojban hosting: one mechanism,
  three consumers.
- **Mixed documents** (alternating English prose and Lojban) can never be
  whole-document Lojban regardless of lexer permissiveness — English is
  invalid morphology and would drown diagnostics. The Markdown-native answer
  is ` ```lojban ` fenced regions (+ opt-in inline code spans) riding the same
  overlay. Whole-doc mode is for "Lojban wearing Markdown clothing"; region
  mode is for tutorials and reference docs.

## Injection grammar & detection

- Registering a `lojban` TextMate grammar with markdown `embeddedLanguages`
  wiring makes ` ```lojban ` fences highlight inside *any* Markdown file via
  the existing injection mechanism.
- **Detection offer**: run the real morphology segmenter over opened `.md`
  documents and measure valid-word coverage — detection by the actual parser,
  not regex vibes. Surfaces as a status-bar offer ("This looks like Lojban —
  enable?"), persisted per file/workspace, always overridable; it only ever
  *offers*, never silently activates diagnostics on someone's English README.

---

# Milestone 2 — the semantic layer (sketch)

Gated on Appendix A (the recovered-tree analysis port). Contents, roughly in
order of value:

- **Place-structure features**: sumti place inlay hints (`x1`/`x2`/modal tags
  from `SumtiPlaceAssignment`); **signature help** with `activeParameter` =
  the place under the cursor (Lojban has no call parens; trigger from cursor
  position via enclosing frame, re-resolving as the cursor moves).
- **Anaphora features**: hover/inline referent display for `ri`/`ko'a`/letter
  anaphora (`ReferenceEdge` targets, with `ReferenceRule` as explanation),
  document highlights for co-referent clusters, find-references, go-to-
  definition for `goi`/`cei`/`da`-series bindings, rename over the binding
  graph.
- **Semantic diagnostics**: unbound anaphora (`ReferenceTarget::Unresolved`
  already exists), place over/under-fill (Advice-tier only — place counts are
  parsed from definition prose, an accepted limitation).
- **Code actions**: tanru ↔ lujvo (jvozba exists), insert/remove elidable
  terminators, `le`↔`lo`, explicit xorlo quantifiers, extract-to-`goi`,
  morphology fixups (pauses, cmevla endings).
- **pandi**: the additive-marks profile over the M1.5 inlay engine; document
  formatting (strip + re-decorate is idempotent since additive glyphs are
  lexer-whitespace); substitutive lexer macros (numeric islands, subscripts,
  `$…$`) as morphology work with span mapping through desugaring. See the
  pandi spec issue (migrated from Codeberg #1).
- **Ghost elidable terminators**: inlays for elided `ku`/`kei`/`vau`/`ku'o`
  from the valid tree's absent `Option` terminator fields (n.b. *not* from
  recovery slots — recovery items are for broken input, elision is normal).

## Later (vague by design)

- **Cukta sidebar** (`WebviewViewProvider`, cukta-only Dioxus wasm bundle),
  deep-linked from diagnostics/hovers via `command:` URIs;
  `Diagnostic.codeDescription.href` points at the hosted SPA as fallback.
- **Parametrized parse command**: `jbotci.parse(text?)` opening a virtual
  read-only result tab (`TextDocumentContentProvider`, `jbotci-parse:`
  scheme); command palette (input box), "Parse selection" context menu,
  per-paragraph code lens. Same pattern later for the semantic analysis that
  succeeds the retired tersmu implementation (#869). All of these consume
  `jbotci-ide` queries — never a second pipeline.
- **Markdown preview integration** via the built-in extension's
  `extendMarkdownIt` API: pandi-decorated or gentufa-rendered preview.
- **SCIP corpus export**: definition/reference/hover graph for static corpus
  browsing (dictionary entries get global monikers; assigned variables are
  document-local). A thin second emitter over `jbotci-ide` once M2 lands.
- Embeddings-backed semantic lookup in the editor: native builds use
  `jbotci-embeddings` (llama-cpp) directly — the LSP server is a native child
  process, so no Node/Electron WebGPU question arises; wasm builds keep the
  WebGPU delegate (works in browser workers for vscode.dev). Embeddings stay
  behind a trait in `jbotci-ide`.

---

# Appendix A — the recovered-tree analysis gap (M2 gate)

Findings (verified 2026-07-14):

- `GeneratedReferenceAnalysis::analyze` takes `&GeneratedTextSyntax` — the
  **valid** model only (`crates/jbotci-semantics/src/references.rs:486`).
  There is no mention of the recovered model anywhere in `jbotci-semantics`
  (8,713 lines in references.rs alone); the tersmu semantic builder, which has
  since been retired (#869), was likewise valid-only.
- The output layer confirms the gap is load-bearing:
  `pretty_recovered_generated_model_tree_with_options` tries
  `try_into_valid()` and renders references only if that succeeds — i.e. only
  when no recovery actually happened; the genuinely-recovered path constructs
  its builder with `syntax_index: None` (`crates/jbotci-output/src/tree.rs`).
- Consequence: **one recovery error anywhere ⇒ zero anaphora/place analysis,
  document-wide** — the common case during live editing. This gates all M2
  semantic features.

Options considered:

- **(a) Last-good-tree caching** — serve semantic features from the last
  cleanly-analyzed parse with spans mapped through edit deltas; diagnostics
  always fresh. Standard language-server practice; ships an M2 MVP; composes
  with (c). Weakness: stale answers exactly where the user is editing.
- **(b) Per-statement salvage** — rejected: discourse state (`ri`, `ko'a`,
  `da` bindings) threads across statements; this is a correctness-losing
  heuristic.
- **(c) The Right Thing: make the recovered model the canonical analysis
  substrate.** valid→recovered is a lossless embedding (exercised by
  `generated_model_recovered_round_trip_matches_valid`), so port the analysis
  to consume `recovered::TextSyntax` and feed valid trees through the
  embedding — **one codepath**, no genericity over two parallel models.
  Recovery slots become explicit discourse events with defined semantics.

Plan: (a) then (c). The genuinely open *design* question in (c) is recovery-slot
discourse semantics — proposal to be written before implementation:
`SkippedTokens` should conservatively poison anaphora state (subsequent `ri`
resolutions marked tentative/unresolved rather than confidently wrong);
`MissingRequiredField` (e.g. missing selbri) yields a frame with unknown place
structure rather than no frame. This is the anaphora analog of the
explicit-underspecification doctrine. The port should be largely mechanical
*if* the tree macros generate walkers for the recovered module — verify before
scoping.

# Appendix B — reusable infrastructure inventory (as of 2026-07)

For implementers; verified in the brainstorm thread.

- **Diagnostics**: `Diagnostic`/`DiagnosticLabel`/`DiagnosticTextSegment`
  (`crates/jbotci-diagnostics`); producer `parse_gentufa_for_web`
  (`crates/jbotci-web-core/src/lib.rs`).
- **Expectations**: `SyntaxExpectation`/`SyntaxExpectedToken`/
  `SyntaxExpectationReason` (`crates/jbotci-syntax/src/lib.rs:485,1706,1726`),
  populated on `SyntaxError::Parse` only.
- **Recovery**: `SyntaxRecoveryParse`/`RecoveredSyntaxParse` with error-slot
  invariants; `RecoveredMorphologySegmentation`; `try_into_valid()` generated
  for recovered types.
- **Brackets/fragments**: `pretty_recovered_syntax_brackets_with_options`,
  `pretty_recovered_syntax_bracket_source_fragments_with_options`
  (`crates/jbotci-output/src/recovered.rs`).
- **Dictionary**: sorted `word_index` (binary search; prefix ranges are a
  small addition), `entries_by_selmaho`, embedded via
  `jbotci-dictionary-data` (`include!` of build-time-generated statics).
- **Word/selma'o per token**: `Word`, `WordKind`, `Selmaho`
  (`crates/jbotci-morphology`).
- **Node indexing (M2)**: `GeneratedSyntaxIndex` + `SyntaxNodeMetadata`
  (parent/depth/preorder/first+last spans) in
  `crates/jbotci-semantics/src/references.rs` — data for offset→node lookup
  exists; the positional index does not.
- **Reference/place queries (M2)**: `ReferenceEdge`/`ReferenceTarget`/
  `ReferenceRule`; `SelbriPlaceFrame`/`SumtiPlaceAssignment`/`PlaceSlot`/
  `AssignmentSource` with query API (`assignments_for_sumti`,
  `frames_for_node`, `first_argument_for_place`).
- **Current lexer strictness**: non-Lojban characters are
  `MorphologyErrorKind::InvalidCharacter` errors; a legacy hand-selected
  separator set already exists (`segment::is_separator` — whitespace, periods,
  `?` `!` `;` `:` `-` brackets, guillemets, quotes) and is frozen by fixtures.
