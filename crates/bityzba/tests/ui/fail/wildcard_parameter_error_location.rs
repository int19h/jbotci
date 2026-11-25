use bityzba::{contract_trait, requires};

// The unsized parameter error must point at the parameter, not at
// `contract_trait`.
#[contract_trait]
trait Unsized {
    #[requires(true)]
    #[ensures(true)]
    fn check(&self, _: str) -> bool;
}

fn main() {}
