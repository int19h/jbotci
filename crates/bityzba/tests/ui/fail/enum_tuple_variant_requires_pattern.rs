use bityzba::invariant;

#[invariant(::Pair => true)]
enum Choice {
    Pair(String, usize),
}

fn main() {}
