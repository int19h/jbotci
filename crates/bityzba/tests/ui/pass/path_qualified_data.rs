use bityzba::{data, invariant, new};

mod model {
    use super::*;

    #[invariant(self.start <= self.end)]
    pub struct Span {
        pub start: usize,
        pub end: usize,
    }

    #[invariant(matches!(self.as_data(), ChoiceData::Named { name } if !name.is_empty()) || matches!(self.as_data(), ChoiceData::Unset))]
    pub enum Choice {
        Unset,
        Named { name: String },
    }
}

fn main() {
    let span = new!(model::Span {
        start: 0,
        end: 4,
    });

    let data!(model::Span { start, end }) = span.as_data();
    assert_eq!((*start, *end), (0, 4));

    let choice = new!(model::Choice::Named {
        name: String::from("cmavo"),
    });

    match choice.as_data() {
        data!(model::Choice::Named { name }) => assert_eq!(name, "cmavo"),
        data!(model::Choice::Unset) => panic!("wrong variant"),
    }
}
