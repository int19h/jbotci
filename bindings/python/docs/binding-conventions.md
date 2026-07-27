# Python binding conventions

These developer-facing rules define the stable shape of the Python boundary.
Consumer documentation belongs in the [bindings guide](guide.md); implementation
work must preserve the conventions below.

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
  Numeric newtypes keep their semantic Python class and validate range or units
  in the constructor instead of leaking a bare integer or a lossy cast.
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
  `enum.StrEnum` classes with stable canonical string values. Payload enums and
  grammar ADTs instead become a closed set of frozen, non-subclassable
  variant/value classes with a typed union or base where appropriate. Neither
  form exposes Rust discriminants, declaration order, serde tags, or generated
  AST layout. The Rust grammar/schema generator is the only source of members
  and variants—Python never maintains a second list—and every generated value
  class is installed through the shared registration helper. The
  `PythonStringEnum` metadata and registration/conversion helpers are the
  reusable path for fieldless enums.
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
