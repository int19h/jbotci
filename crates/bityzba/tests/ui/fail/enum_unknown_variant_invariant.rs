use bityzba::invariant;

#[invariant(::Named => true)]
#[invariant(::Missing => true)]
enum Choice {
    Named { name: String },
}

fn main() {}
