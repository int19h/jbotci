extern crate bityzba;

use bityzba::*;

#[requires(true)]
#[ensures(true)]
fn main() {
    bityzba::require_contracts().unwrap();
}
