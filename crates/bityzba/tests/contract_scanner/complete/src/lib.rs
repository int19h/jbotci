#[invariant(true)]
struct Marker {
    value: usize,
}

enum Choice {
    Present,
}

#[invariant(::Present => true)]
enum DataChoice {
    Empty,
    Present { value: usize },
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

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|value| *value > 0) || ret.is_err())]
fn parse_result(input: &str) -> Result<usize, String> {
    input.parse().map_err(|error| format!("{error}"))
}

#[requires(true)]
#[expensive_ensures(ret.as_ref().is_ok_and(|value| *value > 0) || ret.is_err())]
fn parse_result_expensively(input: &str) -> Result<usize, String> {
    input.parse().map_err(|error| format!("{error}"))
}

impl Marker {
    #[requires(true)]
    #[expensive_ensures(true)]
    fn value(&self) -> usize {
        self.value
    }
}
