use bityzba::invariant;

#[invariant(matches!(self.as_data(), ChoiceData::Named { name } if !name.is_empty()))]
enum Choice {
    Named { name: String },
}

fn main() {
    let _ = Choice::Named {
        name: String::new(),
    };
}
