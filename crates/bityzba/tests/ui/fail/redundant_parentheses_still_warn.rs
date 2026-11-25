#![deny(unused_parens)]

use bityzba::requires;

fn takes(value: bool) -> bool {
    value
}

// These parentheses are redundant, so the lint must still report them. The
// outer pair around an implication group is redundant too.
#[requires(takes((value > 0)))]
#[ensures(takes(((value > 0 -> ret))))]
#[ensures(if (value > 0) { ret } else { true })]
#[ensures([1u32].iter().all(|item| (*item > 0)))]
#[expensive_ensures(takes(((value > 0 -> ret))))]
fn redundant(value: u32) -> bool {
    value > 0
}

fn main() {}
