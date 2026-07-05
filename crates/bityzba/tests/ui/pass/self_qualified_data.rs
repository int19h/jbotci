use bityzba::{data, invariant, new};

#[invariant(::Named => !name.is_empty())]
enum Choice {
    Named { name: String },
}

impl ChoiceData {
    fn name(&self) -> &str {
        match self {
            data!(Self::Named { name }) => name,
        }
    }
}

fn main() {
    let choice = new!(Choice::Named {
        name: String::from("cmavo"),
    });
    assert_eq!(choice.as_data().name(), "cmavo");
}
