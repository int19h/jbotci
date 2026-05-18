use bityzba::{expensive_ensures, expensive_requires};

#[expensive_requires(value.no_such_method())]
fn bad_precondition(value: i32) {}

#[expensive_ensures(ret.no_such_method())]
fn bad_postcondition() -> i32 {
    1
}

fn main() {}
