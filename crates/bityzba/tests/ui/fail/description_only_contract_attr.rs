use bityzba::ensures;

#[ensures("result must be nonzero")]
fn value() -> usize {
    1
}

fn main() {
    let _ = value();
}
