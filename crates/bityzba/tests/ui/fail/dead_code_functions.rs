#![deny(dead_code)]

use bityzba::{
    ensures, expensive_ensures, expensive_invariant, expensive_requires, invariant, requires,
};

#[requires(true)]
fn unused_requires() {}

#[ensures(true)]
fn unused_ensures() {}

#[invariant(true)]
fn unused_invariant() {}

#[expensive_requires(true)]
fn unused_expensive_requires() {}

#[expensive_ensures(true)]
fn unused_expensive_ensures() {}

#[expensive_invariant(true)]
fn unused_expensive_invariant() {}

#[requires(helper())]
#[ensures(helper())]
fn used_contracts() {}

fn helper() -> bool {
    true
}

fn main() {
    used_contracts();
}
