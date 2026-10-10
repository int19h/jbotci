#![deny(dead_code)]

use bityzba::contract_trait;

#[contract_trait]
trait UsedTrait {
    #[bityzba::requires(true)]
    #[bityzba::ensures(true)]
    fn used() {}

    #[bityzba::requires(true)]
    #[bityzba::ensures(true)]
    fn unused_default() {}

    #[bityzba::expensive_requires(true)]
    #[bityzba::expensive_ensures(true)]
    fn unused_required();
}

struct Implementation;

#[contract_trait]
impl UsedTrait for Implementation {
    fn unused_required() {}
}

fn main() {
    Implementation::used();
}
