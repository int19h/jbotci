# jbotci Python bindings

This directory contains the pre-alpha `jbotci` Python distribution. Its public
API tracks the unstable Rust API and can change without compatibility notice.
The extension is private (`jbotci._native`); callers should import public names
from `jbotci` and its namespace modules.

## Local development

From this directory, create and populate a CPython 3.11-or-newer environment:

```sh
uv venv --python 3.11
uv sync --group dev
source .venv/bin/activate
```

Build and install the editable mixed package, then run its checks:

```sh
CARGO_TARGET_DIR=/var/tmp/jbotci-target uv run --group dev maturin develop
CARGO_TARGET_DIR=/var/tmp/jbotci-target cargo test -r -p jbotci-python
uv run --group dev python -m pytest -q tests
uv run --group dev mypy --strict python tests
```

Use a target directory on a local filesystem when the repository itself is on
a mounted filesystem. `maturin develop` installs both the Python sources and
the ABI3 native extension into the active environment. Imports should also be
smoke-tested with a working directory outside this repository so the editable
install, rather than the source tree, resolves `jbotci`.

## Binding conventions

- Public Python objects use explicit Python names and `module = "jbotci..."`.
  Pyclasses are frozen and non-subclassable by default. Value objects implement
  equality, and implement hashing only when the underlying Rust value is also
  hashable. Their `repr` is constructor-shaped and includes the public module.
- `Option<T>` maps to `T | None`. Ordered collection inputs use
  `collections.abc.Sequence`, so lists and tuples are accepted after every
  element is validated; unordered iterables such as sets are rejected. Rust
  sequences are materialized as immutable Python tuples, never mutable lists.
  The same validation applies to pure-Python public wrappers such as
  `MorphologyError`, including an immutable copy of each accepted sequence.
  Numeric newtypes keep their semantic Python class and validate range/units in
  the constructor instead of leaking a bare integer or a lossy cast.
- A Python wrapper for borrowed Rust data must retain a strong owner. The
  internal `OwnedReference` helper stores an `Arc` plus a projection function;
  wrappers must never expose Rust lifetimes or construct self-references. Static
  embedded aggregate data uses the same `Arc` owner plus domain-specific typed
  positions so children remain valid independently without copying whole
  source tables.
- Structured Rust binding errors convert in one place to subclasses of
  `JbotciError`. Python-visible exceptions use stable messages and arguments;
  Rust implementation types (`Box`, `Arc`, bityzba data wrappers, lifetimes,
  and serde payloads) never cross the boundary.
  `TraceLevel.from_number` delegates every value in Rust's `u8` domain to Rust;
  invalid values from 0 through 255 raise `TraceOptionError` with an
  `InvalidTraceLevel` payload. Integers outside that domain raise
  `InvalidInputError` (or Python `OverflowError` before binding conversion when
  they do not fit the signed native argument at all).
  Source constructors retain `SourceLocationError` variants in
  `SourceLocationException`; source-text offset helpers retain every
  `DiagnosticSpanError` variant in `DiagnosticSpanException`, including nested
  source-location payloads. Binding-side precondition guards construct those
  same Rust error variants before calling contracted core helpers.
- Fieldless Rust enums that represent finite choices become genuine
  `enum.Enum` classes, preferably with stable canonical string values. Payload
  enums and grammar ADTs instead become a closed set of frozen,
  non-subclassable variant/value classes with a typed union or base where
  appropriate. Neither form exposes Rust discriminants, declaration order,
  serde tags, or generated AST layout. The Rust grammar/schema generator is the
  only source of members and variants—Python never maintains a second list—and
  every generated value class is installed through the shared registration
  helper. The `PythonStringEnum` metadata and registration/conversion helpers
  are the reusable path for fieldless enums.
- Registration runs in a fixed order during module initialization. There is no
  mutable module-level Rust state; registrations and embedded data must be
  immutable or interpreter-local.
- Each public domain owns an ordered native export inventory. Classes and
  `StrEnum`s are stored on the private extension under deterministic
  domain-qualified keys such as `_dictionary_Dictionary`, while the class
  itself retains the public name `Dictionary` and module
  `jbotci.dictionary`. Python-to-Rust `StrEnum` conversion first verifies exact
  identity with that interpreter's registered class and then accepts only a
  declared canonical value; a plain string is never accepted as an enum.

## Embedded dictionary

`jbotci.dictionary.english` and `english_metadata` are immutable module objects
with stable identity for the lifetime of an interpreter. `english.entries` is a
lazy source-order sequence: importing the module does not create 17,415 Python
entry objects. Entries and nested keyword, rafsi, user, sound, IPA, pattern, and
lujvo-decomposition records retain the shared native owner and are projected on
demand.

Lookup methods delegate to the Rust dictionary indexes and return immutable
tuples in collision/index order. Integer indexing follows normal negative-index
rules; typed `EntryIndex` lookup remains optional for out-of-range values.
`Dictionary` and `DictionaryEntries` provide the concrete `len`, indexing, and
iteration protocol only; they do not claim `collections.abc.Sequence` methods
such as `count` or `index` that the native objects do not implement. The public
surface intentionally does not expose static-slice construction, importer-owned
index builders, serde bridges, or a sound-search operation.

Each `DictionarySoundEntry` exposes both its canonical display
`token_sequence` and the distinct `pronunciation_targets` used by sound
scoring. A `PronunciationTargetId` can admit several concrete `IpaSegmentId`
realizations (notably for Lojban `r`); `realizations` returns all of them in
Rust inventory order. The Rust-parity `realization(index)` method returns
`None` for every out-of-range index, including negative Python integers. The
convenience tuple itself retains normal Python negative-index behavior.

The checked-in `python/jbotci/_native.pyi` is composed from the ordered manual,
generated, and domain fragments under `stubs/_native/`. After changing a
fragment, run:

```sh
uv run python tools/compose_stubs.py
uv run python tools/compose_stubs.py --check
```

The explicit fragment manifest in the compositor is the ordering authority.
The test suite verifies that composition is reproducible and that the composed
public declarations match the native module's runtime exports.

## Jvozba composition and decomposition

`jbotci.jvozba` keeps dictionary words and exact rafsi as distinct input
variants. The ergonomic builder uses the embedded English dictionary and lujvo
mode by default:

```python
from jbotci import jvozba

result = jvozba.build(
    [jvozba.Word("lojbo"), jvozba.FixedRafsi("bau")],
    mode=jvozba.JvozbaMode.LUJVO,
)
assert result.word == "jbobau"
```

Use `build_best_jvozba_detailed(mode, dictionary, raw_inputs)` for the exact
low-level Rust argument order. Both functions accept ordered Python sequences
of `Word | FixedRafsi`; raw strings and arbitrary iterables are rejected.
`word_can_enter_jvozba_pane(dictionary, word)` preserves the direct Rust
predicate, while `can_use_word(word, dictionary=english)` supplies ergonomic
argument order and defaults.

`decompose_lujvo_like(word, dictionary=english)` returns exact
`morphology.LujvoRafsi` and `morphology.LujvoHyphen` values with optional
dictionary source words. All collections are immutable tuples, hyphens always
have `source=None`, and source strings are owned before detached Rust work
returns to Python.

Every Rust `JvozbaError` variant has a distinct immutable value class and a
final exception class. Exceptions retain the complete value in `.value`;
payload exceptions also expose fields such as `.offending` and
`.is_fixed_rafsi`.

## Syntax parsing and completion

Use the package-root conveniences when starting from source text:

```python
import jbotci

parsed = jbotci.parse("mi tavla do", source_id="example")
tree = parsed.parse_tree  # jbotci.syntax.strict.TextSyntax

recovered = jbotci.parse_recovered("mi tavla vau vau do")
for error in recovered.syntax_errors:
    print(error.code)
```

These functions run morphology first and pass its typed Rust word values
directly to the syntax parser. `jbotci.syntax` exposes the corresponding
low-level strict, recovered, strict-or-recovered, and non-raising APIs for
callers that already retain `WordLike` values. Parse trees are immutable lazy
projections over one retained native owner; repeated field projections preserve
owner/path identity.

## Place assignment and discourse references

`jbotci.analyze` runs the same strict morphology and syntax pipeline as
`jbotci.parse`, then runs the Rust place-assignment and discourse-reference
analysis over that typed tree:

```python
import jbotci

result = jbotci.analyze("mi tavla do")
analysis = result.reference_analysis

for assignment in analysis.place_analysis.assignments():
    frame = analysis.place_analysis.frame(assignment.frame)
    sumti = analysis.syntax_index.node(assignment.sumti.raw_id)
    assert frame is not None
    assert sumti is not None
```

`AnalyzedText` retains the morphology result, strict syntax result, warnings,
traces, and reference analysis. For an existing `SyntaxParse` or strict
`TextSyntax`, use
`jbotci.semantics.references.analyze_references(tree_or_parse)`. This lower-level
form consumes the typed Rust tree directly: it never serializes, reparses, or
constructs a `tersmu` semantic model.

`ReferenceAnalysis` owns one strong handle to the original strict syntax root
and keeps the core borrowed analysis in a Rust owning cell. Its syntax index,
frames, assignments, reference edges, and reference-target values retain that
same owner, so they remain valid after the original parse or tree Python object
is dropped. Node lookup projects lazily through the retained owner and original
tree path; it does not clone the tree or compare source spans as identity.

Syntax-node, place-frame, assignment, and edge IDs are distinct immutable
classes with an explicit integer `.value`; typed syntax IDs also expose
`.raw_id`. IDs are scoped to one analysis. Equality and hashing include that
analysis identity, and passing an ID to a different analysis raises
`jbotci.InvalidInputError`. All query collections are immutable tuples.

`fixture_projection(analysis)` and `fixture_projection_json(analysis)` are
secondary projections for corpus fixtures and debugging. They intentionally
use the core resolver's canonical fixture representation and are not the
primary object model; normal consumers should use `syntax_index`,
`place_analysis`, and `discourse_references`.

Completion accepts either typed morphology words or source text. Cursor-based
completion interprets `cursor` as a Python Unicode-character index and parses
exactly `text[:cursor]`:

```python
from jbotci import syntax

expected = syntax.expected_continuations_at_cursor("mi klama le", 11)
```

It does not guess partial words or scan backwards. An invalid exact prefix
therefore raises the ordinary typed morphology error. Time limits are seconds,
must be finite and nonnegative, and return only the expectations produced
within the Rust operation; there is no synthetic timeout flag.
