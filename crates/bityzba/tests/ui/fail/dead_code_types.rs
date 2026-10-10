#![deny(dead_code)]

use bityzba::{expensive_invariant, invariant};

#[invariant(true)]
struct UnusedMarker;

#[expensive_invariant(true)]
struct UnusedExpensiveMarker;

#[invariant(*value > 0)]
struct UnusedChecked {
    value: usize,
}

#[expensive_invariant(*value > 0)]
struct UnusedExpensiveChecked {
    value: usize,
}

#[invariant(self.0.0 > 0)]
struct UnusedNewtype(usize);

#[invariant(::Value => *value > 0)]
enum UnusedEnum {
    Value { value: usize },
}

fn main() {}
