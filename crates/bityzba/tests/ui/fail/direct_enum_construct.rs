use bityzba::invariant;

#[invariant(true)]
enum Choice {
    Named { name: String },
}

fn main() {
    let _ = Choice::Named {
        name: String::new(),
    };
}
