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

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|value| *value > 0) || ret.as_ref().err().is_some_and(|message| !message.is_empty()))]
fn parse_result_with_error_contract(input: &str) -> Result<usize, String> {
    input.parse().map_err(|error| format!("{error}"))
}

#[requires(true)]
#[ensures(!ret.is_err() && ret.as_ref().is_ok_and(|value| *value > 0))]
fn infallible_result_contract() -> Result<usize, String> {
    Ok(1)
}

#[requires(true)]
#[ensures(probe.as_ref().is_ok_and(|value| *value > 0) || probe.is_err())]
fn unrelated_result_probe_contract(probe: Result<usize, String>) -> Result<(), String> {
    let _ = probe;
    Ok(())
}

impl Marker {
    #[requires(true)]
    #[expensive_ensures(true)]
    fn value(&self) -> usize {
        self.value
    }
}
