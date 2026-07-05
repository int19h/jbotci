#[bityzba::requires(true)]
struct RequiresTarget;

#[bityzba::ensures(true)]
struct EnsuresTarget;

#[bityzba::invariant(true)]
const INVARIANT_TARGET: usize = 0;

#[bityzba::contract_trait]
struct ContractTraitTarget;

fn main() {}
