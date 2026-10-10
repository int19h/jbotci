#![deny(dead_code)]

use bityzba::{invariant, new, requires};

fn cheap_helper() -> bool {
    true
}

fn expensive_helper() -> bool {
    true
}

#[requires(cheap_helper())]
#[bityzba::ensures(cheap_helper())]
#[invariant(cheap_helper())]
#[bityzba::expensive_requires(expensive_helper())]
#[bityzba::expensive_ensures(expensive_helper())]
#[bityzba::expensive_invariant(expensive_helper())]
fn live() {}

#[invariant(cheap_helper())]
#[bityzba::expensive_invariant(expensive_helper())]
struct Checked {
    value: usize,
}

fn main() {
    live();
    let checked = new!(Checked { value: 1 });
    assert_eq!(checked.value, 1);
}
