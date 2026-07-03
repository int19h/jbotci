#[invariant(true)]
#[requires(true)]
enum MisorderedEnum {
    Empty,
}

#[ensures(ret > 0)]
#[requires(!input.is_empty())]
fn parse_term(input: &str) -> usize {
    input.len()
}
