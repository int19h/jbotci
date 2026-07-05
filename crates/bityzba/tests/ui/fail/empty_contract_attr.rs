use bityzba::requires;

#[requires()]
fn value() -> usize {
    1
}

fn main() {
    let _ = value();
}
