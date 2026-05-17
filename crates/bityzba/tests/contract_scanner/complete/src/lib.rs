#[invariant(true)]
struct Marker {
    value: usize,
}

#[expensive_invariant(true)]
enum Choice {
    Present,
}

#[contract_trait]
trait Provides {
    #[requires(true)]
    #[ensures(true)]
    fn get(&self) -> usize;
}

#[invariant(true)]
struct ImplType;

#[contract_trait]
impl Provides for ImplType {
    fn get(&self) -> usize {
        0
    }
}

#[requires(!input.is_empty())]
#[ensures(ret > 0)]
fn parse_term(input: &str) -> usize {
    input.len()
}

impl Marker {
    #[requires(true)]
    #[expensive_ensures(true)]
    fn value(&self) -> usize {
        self.value
    }
}
