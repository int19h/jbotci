# Python bindings guide

The Python package is a typed, immutable view of the jbotci Rust API. This
guide describes the consumer contract. The API is pre-alpha and unstable:
pin exact versions, keep strict mypy or another capable type checker in the
upgrade path, and read generated-union changes as source-breaking changes for
exhaustive consumers.

For installation commands and the executable quick start, see the
[package README](../README.md). All programs linked from this guide are run by
pytest and again from a fresh installed wheel.

## Namespace map

| Namespace | Purpose |
| --- | --- |
| `jbotci` | Source-to-parse and source-to-reference-analysis pipelines |
| `jbotci.source` | Source IDs, spans, line/column values, and Unicode-aware offset conversion |
| `jbotci.diagnostics` | Structured diagnostics and configurable morphology/syntax traces |
| `jbotci.dialect` | Declarative dialect definitions used by morphology and syntax options |
| `jbotci.morphology` | Word segmentation, recovery, classification, and morphology trees |
| `jbotci.syntax` | Parser options/results, recovery values, warnings, and shared syntax helpers |
| `jbotci.syntax.strict` | Generated strict syntax node classes and closed unions |
| `jbotci.syntax.recovered` | Generated recovered syntax node classes and closed unions |
| `jbotci.dictionary` | Immutable embedded English dictionary and typed queries |
| `jbotci.jvozba` | Typed lujvo/cmevla composition and lujvo decomposition |
| `jbotci.semantics.references` | Syntax indexing, place assignment, and discourse references |

## Representation rules

Rust product types become immutable, final Python classes with named
properties. Payload enum alternatives become separate immutable variant
classes and a closed union. Fieldless domain enums become `StrEnum` classes
whose values are stable canonical strings. `Option<T>` becomes `T | None`.
Rust sequences become tuples; ordered inputs accept `Sequence` values after
validating every member. Unordered sets and arbitrary iterators are not
silently reordered.

Numeric IDs and indexes retain distinct semantic classes instead of becoming
bare integers. Exceptions retain typed error values rather than only formatted
messages. Rust ownership carriers, generic traits, lifetimes, caller-owned
accumulators, and raw path or pointer identity do not cross the boundary when
a typed Python operation carries the same information.

The exhaustive mapping, including deliberate Rust-only construction and
repository data-build machinery, is in
[`api-parity.tsv`](api-parity.tsv).

## Strict and recovered parsing

`jbotci.parse(text)` runs strict morphology and strict syntax parsing and
returns `ParsedText`. A morphology failure raises `MorphologyError`; a syntax
failure raises `SyntaxError`. Both exceptions retain typed Rust error values,
source context when supplied, and optional traces.

`jbotci.parse_recovered(text)` returns `RecoveredParsedText`. Its morphology
and syntax products contain typed recovery errors rather than replacing
invalid regions with dictionaries or untyped sentinels. Recovered generated
fields are closed unions of `RecoveredValid`, `RecoveredError`, and
`RecoveredPrefix`.

Callers that already own morphology values can use the lower-level functions
in `jbotci.syntax`. `syntax_tokens_with_options` consumes typed `WordLike`
values directly. Strict, recovered, strict-or-recovered, non-raising, and
completion operations all retain the same structured result and error types.
Cursor completion uses a Python Unicode-character index and parses exactly the
prefix before that index; it does not guess incomplete words.

`SyntaxRecoveryErrorPolicy` configures separate non-zero per-statement and
whole-input error limits. Pass it as `ParseOptions(recovery_error_policy=...)`;
the recovered and strict-or-recovered entry points consume those options
directly. Both values are immutable, and their `with_...` methods return
validated copies.

[`recovery_diagnostics.py`](../examples/recovery_diagnostics.py) executes both
strict tracing and recovered parsing.

## Typed traversal and structural matching

Generated syntax values support ordinary Python structural pattern matching.
Each concrete variant publishes `__match_args__`, and every child is a typed
property, tuple, optional value, or shared helper such as `Chain` and
`WithFreeModifiers`. No conversion to a dictionary is needed. The
[`end_to_end.py`](../examples/end_to_end.py) program matches
`TextSyntaxRegularText`, then follows typed syntax IDs back to `SumtiSyntax`
nodes.

Repeated projections of the same generated field retain owner/path identity;
`same_identity` distinguishes that identity from structural equality. Tree
nodes are immutable. Use their fields for recursive traversal, or use
`GeneratedSyntaxIndex` when parent/order/depth, raw IDs, or typed node-family
IDs are needed.

Grammar changes regenerate strict and recovered classes, unions, match
arguments, and packaged typing information from the canonical schema. A
capable type checker can therefore report an `assert_never` exhaustiveness
failure when a new alternative is added. Python itself does not enforce
exhaustiveness at runtime: a dynamic `case _` consumer can intentionally
ignore an unfamiliar alternative. Upgrading without running a type checker
cannot promise exhaustive-match breakage.

## Dictionary, classification, and jvozba

`jbotci.dictionary.english` is the validated embedded English Lensisku
snapshot. It has stable interpreter-local identity and lazily projects entries
and nested records. `lookup_word`, `lookup_words`, `lookup_rafsi`,
`entries_by_word_prefix`, and `entries_by_selmaho` use the Rust indexes and
return typed immutable results. `english_metadata` identifies the embedded
snapshot.

`jbotci.morphology.analyze_valsi` determines whether input is one valid valsi
and returns its typed classification, warning, word, or error products. The
canonical phoneme spelling from a `PlainWord` is suitable for dictionary
lookup; raw source spelling and accents remain available separately.

`jbotci.jvozba.Word` and `FixedRafsi` are distinct input variants. `build`
accepts an ordered sequence, a `JvozbaMode`, and a `Dictionary`; it defaults to
the embedded English dictionary. `decompose_lujvo_like` returns typed
morphology rafsi/hyphen segments and optional dictionary source words. Every
jvozba error alternative has a corresponding final exception retaining the
exact value.

[`dictionary_jvozba.py`](../examples/dictionary_jvozba.py) passes the public
dictionary object directly into jvozba and exercises classification, lookup,
and composition without an intermediate export.

## Place assignment and references

`jbotci.analyze(text)` runs the strict parse pipeline and reference analysis,
returning `AnalyzedText`. For an existing strict `SyntaxParse` or
`syntax.strict.TextSyntax`, call
`jbotci.semantics.references.analyze_references`.

`ReferenceAnalysis.syntax_index` maps retained generated nodes to raw or
grammar-family IDs and back. `place_analysis` exposes place frames,
sumti-to-place assignments, and indexed queries by frame, slot, sumti, term, or
node. `discourse_references` exposes typed edges whose target is a closed union
of resolved node, resolved frame, ambiguous nodes, unresolved, and
intentionally vague alternatives.

Raw node, typed node, frame, assignment, and edge IDs are different immutable
classes scoped to one analysis. Passing an ID from another analysis raises
`InvalidInputError`; strict typing also rejects mixing ID families before
execution. `SelbriPlaceFrame`, `SumtiPlaceAssignment`, and `ReferenceEdge`
representations include their public fields for useful tracebacks.

The end-to-end example exercises assignments and reference edges. Fixture
projection APIs also exist for corpus tests and debugging, but they are
secondary to the typed query model and should not be used as a substitute for
normal traversal.

## Spans, exceptions, warnings, and traces

`SourceSpan` uses half-open UTF-8 byte and Unicode-scalar ranges. Its optional
`LineColumn` endpoints are one-based. Use the functions in `jbotci.source` to
construct spans, convert between character and byte offsets, compute
line/column values, and slice validated source text. Invalid values raise
`SourceLocationException` or `DiagnosticSpanException` with a closed typed
error value such as `CharOffsetOutOfBounds`.

Morphology and syntax warnings contain stable codes, typed kinds, exact source
anchors, and conversion to structured `Diagnostic` values. Raising parse
exceptions retain the original typed error, source, spans, warnings where
applicable, and an optional trace.

Tracing is opt-in through immutable `diagnostics.TraceOptions`, passed through
`MorphologyOptions` and `syntax.ParseOptions`. `TraceReport` contains its
phase, events, and failure summary; filters and limits are applied by Rust
during the operation. The recovery/diagnostics example is the executable
witness for spans, structured exceptions, warnings, and both trace phases.

## Ownership and immutability

All consumer-facing trees, records, options, errors, and result collections
are immutable. Copy-on-update methods such as `with_trace` return a new options
value. Borrowed Rust data retains a strong owner internally, so a dictionary
entry, syntax child, node lookup, frame, assignment, edge, or target remains
valid after intermediate Python owners are dropped.

The main cross-module paths do not serialize or rebuild their inputs:
morphology values feed syntax, strict syntax feeds reference analysis, and the
embedded dictionary feeds jvozba. Syntax/reference wrappers share the original
owner where identity matters; detached strings and small scalar products are
owned before returning to Python.

## Current non-goals

These bindings expose reference analysis but no deep semantic graph, and no
cukta book search. They do not publish the underlying Rust crates to Python
packaging, and they do not promise API stability yet. Platform wheel and source-distribution
CI is tracked separately. Repository dictionary import/index generation,
generic Rust visitor traits, and ownership/lifetime adapters are build or
implementation machinery rather than alternate consumer APIs.
