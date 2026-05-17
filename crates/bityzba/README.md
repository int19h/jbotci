# bityzba

`bityzba` is the project-local design-by-contract crate. It re-exports the
proc macros implemented by the internal `bityzba-macros` crate and owns
ordinary support APIs such as the optional source scanner. It is an MPL-2.0
fork of `contracts 0.6.7` with first-class expensive contracts and
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
unchecked data type named `TypeData`. Values of the wrapper type are valid by
construction. Public `is_valid()` is not the model for invariant-bearing types.
On structs and enums, `#[invariant(true)]` and `#[expensive_invariant(true)]`
are explicit "audited no data invariant" markers and leave the type unchanged:
no wrapper, no `TypeData`, no `new!`, and no `data!` machinery are generated.

```rust
use bityzba::{data, invariant, new, try_new};
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
- `SourceSpanData`, the unchecked data shape.
- `SourceSpanInvariantError`.
- `new!(SourceSpan { ... })` and `try_new!(SourceSpan { ... })` for full
  construction.
- `span.with_data(data! { ... })` for whole-value updates.
- `SourceSpan::try_from_data(data)`, `TryFrom<SourceSpanData>`, `as_data()`, and
  `into_data()`.
- Serde implementations that validate during deserialization when the original
  type derived serde traits.

Full construction requires every field exactly once. Missing fields fail at
compile time through builder typestate; duplicate fields are rejected by
`data!`.

```rust
let span = new!(SourceSpan {
    byte_start: 0,
    byte_end: 4,
    char_start: 0,
    char_end: 4,
});

let fallible_span = try_new!(SourceSpan {
    byte_start: 0,
    byte_end: 4,
    char_start: 0,
    char_end: 4,
})?;
```

`with_data` consumes the old value, applies a partial field set, and
revalidates the whole result. Clone first if the old value must be retained.

```rust
let longer = span.with_data(data! {
    byte_end: 8,
    char_end: 8,
});
```

`Deref<Target = TypeData>` is implemented for read-only field access:

```rust
assert_eq!(longer.byte_end, 8);
```

There is no `DerefMut`. Mutate invariant-bearing values through `with_data` or
through explicit data reconstruction followed by `try_from_data` or
`from_data`.

`bityzba` does not generate `Type::new`, so a type can define its own `new`
constructor when construction does useful work beyond invariant validation.
Use `from_data` after explicit checks have proved the invariant, or
`try_from_data` when converting unchecked input directly.

```rust
#[invariant(self.start <= self.end)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Result<Self, SpanError> {
        if start > end {
            return Err(SpanError::Inverted);
        }
        Ok(Self::from_data(data!(Span {
            start: start,
            end: end,
        })))
    }
}
```

Enums use the same construction macros. Named variants use braces, tuple
variants use parentheses, and unit variants use a path:

```rust
let value = new!(SyntaxValue::Node { node });
let value = new!(SyntaxValue::Null);
let value = new!(Example::Pair(left, right));
```

For structs with private fields, `new!` expands through generated hidden
builder methods, so construction can be part of the public API without exposing
literal field access. `data!(Type { ... })` does not bypass Rust privacy.

## Pattern Matching

Pattern-match through `as_data()` and `data!` aliases to avoid spelling data
type names in normal code:

```rust
let data!(SourceSpan { byte_start, byte_end, .. }) = longer.as_data();
assert!(byte_start <= byte_end);
```

Enum variants use the same alias form:

```rust
match value.as_data() {
    data!(SyntaxValue::Node { node }) => visit(node),
    data!(SyntaxValue::Word { word }) => visit_word(word),
    data!(Example::Pair(left, right)) => visit_pair(left, right),
    _ => {}
}
```

For path-qualified enum variants, `data!` follows normal Rust naming
conventions: the enum type segment immediately before the variant is rewritten
to `TypeData`, as in `data!(crate::model::Value::Node { node })`.

## Data Escape Hatches

`TypeData` exists for serde internals, low-level tests, generated fixtures, and
rare advanced code that must represent unchecked data. Normal construction and
updates should use `new!`, `try_new!`, `with_data`, and the generated
validation APIs. Enum helper constructors should use `new!(Type::Variant {
... })` rather than spelling `TypeData`.

## Contract Scanner

Enable the `contract_scanner` feature from a build script to require explicit
contract decisions during normal development builds:

```toml
[build-dependencies]
bityzba = { workspace = true, features = ["contract_scanner"] }
```

```rust
#[bityzba::requires(true)]
#[bityzba::ensures(true)]
fn main() {
    bityzba::require_contracts().unwrap();
}
```

The scanner checks Rust source files under `src`, `tests`, `benches`, and
`examples`, plus `build.rs` when present. It requires every free function,
inherent method, and trait method to have both a precondition marker
(`requires` or `expensive_requires`) and a postcondition marker (`ensures` or
`expensive_ensures`). It requires every struct and enum to have `invariant` or
`expensive_invariant`, and every trait to use `contract_trait`.

Diagnostics are intentionally worded for coding agents: they ask for the real
contract to be reasoned through and describe `true` markers as a last resort,
not as the default response. The scanner is syntactic and stable-Rust
compatible; it does not inspect macro expansions and does not treat `cfg_attr`
as a visible contract in v1.
