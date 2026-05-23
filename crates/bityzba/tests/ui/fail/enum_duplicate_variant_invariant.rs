use bityzba::invariant;

#[invariant(::Named => true)]
#[invariant(::Named => !name.is_empty())]
enum Choice {
    Named { name: String },
}

fn main() {}
