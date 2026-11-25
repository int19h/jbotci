use bityzba::requires;

// The type error must point at the `old(...)` call, not at `requires`.
#[requires(true)]
#[ensures(ret == old(*value))]
fn ordinary(value: &mut u32) -> u64 {
    *value += 1;
    u64::from(*value)
}

#[requires(true)]
#[expensive_ensures(ret == old(*value))]
fn expensive(value: &mut u32) -> u64 {
    *value += 1;
    u64::from(*value)
}

fn main() {}
