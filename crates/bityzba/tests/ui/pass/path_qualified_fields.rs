use bityzba::{fields, invariant};

mod model {
    use super::*;

    #[invariant(self.start <= self.end)]
    pub struct Span {
        pub start: usize,
        pub end: usize,
    }

    #[invariant(matches!(self.as_raw(), ChoiceRaw::Named { name } if !name.is_empty()) || matches!(self.as_raw(), ChoiceRaw::Unset))]
    pub enum Choice {
        Unset,
        Named { name: String },
    }
}

fn main() {
    let span = model::Span::try_from_fields(fields! {
        start: 0,
        end: 4,
    })
    .unwrap();

    let fields!(model::Span { start, end }) = span.as_raw();
    assert_eq!((*start, *end), (0, 4));

    let choice = model::Choice::try_from_raw(model::ChoiceRaw::Named {
        name: String::from("cmavo"),
    })
    .unwrap();

    match choice.as_raw() {
        fields!(model::Choice::Named { name }) => assert_eq!(name, "cmavo"),
        fields!(model::Choice::Unset) => panic!("wrong variant"),
    }
}
