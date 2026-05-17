# bityzba

`bityzba` is the project-local design-by-contract proc-macro crate. It is an
MPL-2.0 fork of `contracts 0.6.7` with first-class expensive contracts and
valid-by-construction type invariants.

## Function And Trait Contracts

Use cheap contracts for checks that should run in normal builds:

```rust
use bityzba::{ensures, requires};

#[requires(min <= max)]
#[ensures(min <= ret && ret <= max)]
fn clamp_to_range(value: usize, min: usize, max: usize) -> usize {
    value.clamp(min, max)
}
```

`old(expr)` is available in `ensures` clauses and captures a pre-call value:

```rust
use bityzba::ensures;

#[ensures(*count == old(*count) + 1)]
fn increment(count: &mut usize) {
    *count += 1;
}
```

Contract expressions also support implication with `->`:

```rust
use bityzba::ensures;

#[ensures(name.is_some() -> ret.contains(name.unwrap()))]
fn greeting(name: Option<&str>) -> String {
    name.map_or_else(|| "hello".to_owned(), |name| format!("hello {name}"))
}
```

Expensive contracts use the same syntax and are enabled only when the consuming
crate enables its `expensive_contracts` feature:

```rust
use bityzba::{contract_trait, expensive_ensures, expensive_requires, ensures};

#[contract_trait]
trait TokenSource {
    #[ensures(!ret.is_empty())]
    #[expensive_ensures(ret.chars().all(|ch| ch.is_ascii()))]
    fn token(&self) -> String;

    #[expensive_requires(expected.iter().all(|value| !value.is_empty()))]
    fn accepts(&self, expected: &[String]) -> bool;
}
```

Apply `#[contract_trait]` to both the trait and each trait implementation. Trait
methods can mix cheap and expensive contracts.

## Function And Impl Invariants

`#[invariant]` on a function is checked before and after the function body.
`#[invariant]` on an `impl` block is copied to every method that takes `self`.
`#[expensive_invariant]` is the feature-gated version.

```rust
use bityzba::invariant;

#[invariant(self.len <= self.capacity)]
impl Buffer {
    pub fn push(&mut self, byte: u8) {
        /* ... */
    }
}
```

## Type Invariants

`#[invariant]` on a named-field struct or enum creates a wrapper type and an
unchecked raw type named `TypeRaw`. Values of the wrapper type are valid by
construction. Public `is_valid()` is not the model for invariant-bearing types.

```rust
use bityzba::{fields, invariant};
use serde::{Deserialize, Serialize};

#[invariant(self.byte_start <= self.byte_end, "byte range must be ordered")]
#[invariant(self.char_start <= self.char_end, "char range must be ordered")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceSpan {
    pub byte_start: usize,
    pub byte_end: usize,
    pub char_start: usize,
    pub char_end: usize,
}
```

The macro generates:

- `SourceSpan`, the validated wrapper.
- `SourceSpanRaw`, the unchecked data shape.
- `SourceSpanInvariantError`.
- `SourceSpan::new(fields! { ... })` for full construction.
- `span.with_fields(fields! { ... })` for whole-value updates.
- `SourceSpan::try_from_raw(raw)`, `TryFrom<SourceSpanRaw>`, `as_raw()`, and
  `into_raw()`.
- Serde implementations that validate during deserialization when the original
  type derived serde traits.

Full construction requires every field exactly once. Missing fields fail at
compile time through builder typestate; duplicate fields are rejected by
`fields!`.

```rust
let span = SourceSpan::new(fields! {
    byte_start: 0,
    byte_end: 4,
    char_start: 0,
    char_end: 4,
});
```

`with_fields` consumes the old value, applies a partial field set, and
revalidates the whole result. Clone first if the old value must be retained.

```rust
let longer = span.with_fields(fields! {
    byte_end: 8,
    char_end: 8,
});
```

`Deref<Target = TypeRaw>` is implemented for read-only field access:

```rust
assert_eq!(longer.byte_end, 8);
```

There is no `DerefMut`. Mutate invariant-bearing values through
`with_fields` or through an explicit raw reconstruction followed by
`try_from_raw` or `from_raw`.

If a type needs to provide its own `new` constructor, add
`#[bityzba(no_new)]` after the invariant attributes. `from_raw`,
`try_from_raw`, raw conversion, serde validation, `Deref`, and
`with_fields` are still generated; only the field-builder `new` method is
omitted.

```rust
#[invariant(self.start <= self.end)]
#[bityzba(no_new)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Result<Self, SpanError> {
        if start > end {
            return Err(SpanError::Inverted);
        }
        Ok(Self::from_raw(fields!(Span {
            start: start,
            end: end,
        })))
    }
}
```

## Pattern Matching

Pattern-match through `as_raw()` and `fields!` aliases to avoid spelling raw
type names in normal code:

```rust
let fields!(SourceSpan { byte_start, byte_end, .. }) = longer.as_raw();
assert!(byte_start <= byte_end);
```

Enum variants use the same alias form:

```rust
match value.as_raw() {
    fields!(SyntaxValue::Node { node }) => visit(node),
    fields!(SyntaxValue::Word { word }) => visit_word(word),
    _ => {}
}
```

For path-qualified enum variants, `fields!` follows normal Rust naming
conventions: the enum type segment immediately before the variant is rewritten
to `TypeRaw`, as in `fields!(crate::model::Value::Node { node })`.

## Raw Escape Hatches

`TypeRaw` exists for serde internals, low-level tests, generated fixtures, and
rare advanced code that must represent unchecked data. Normal construction and
updates should use `fields!` and the generated validation APIs. Enum helper
constructors should use `fields!(Type::Variant { ... })` rather than spelling
`TypeRaw`.
