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
    let span = model::Span::new(fields! {
        start: 0,
        end: 4,
    });

    let fields!(model::Span { start, end }) = span.as_raw();
    assert_eq!((*start, *end), (0, 4));

    let choice = model::Choice::from_raw(fields!(model::Choice::Named {
        name: String::from("cmavo"),
    }));

    match choice.as_raw() {
        fields!(model::Choice::Named { name }) => assert_eq!(name, "cmavo"),
        fields!(model::Choice::Unset) => panic!("wrong variant"),
    }
}
