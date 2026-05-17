use bityzba::{invariant, new};

mod model {
    use super::*;

    #[invariant(self.start <= self.end)]
    pub struct Span {
        start: usize,
        end: usize,
    }

    impl Span {
        pub fn bounds(&self) -> (usize, usize) {
            (self.start, self.end)
        }
    }
}

fn main() {
    let span = new!(model::Span { start: 0, end: 4 });
    assert_eq!(span.bounds(), (0, 4));
}
