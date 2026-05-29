#[allow(unused_imports)]
use bityzba::{ensures, requires};

#[requires(true)]
#[ensures(true)]
fn main() {
    bityzba::require_contracts().unwrap();
}
