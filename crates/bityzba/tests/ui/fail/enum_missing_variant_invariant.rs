use bityzba::invariant;

#[invariant(matches!(self.as_data(), ChoiceData::Named { name } if !name.is_empty()))]
enum Choice {
    Empty,
    Named { name: String },
}

fn main() {}
