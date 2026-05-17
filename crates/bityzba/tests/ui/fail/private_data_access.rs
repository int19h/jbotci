use bityzba::{data, invariant};

mod model {
    use super::*;

    #[invariant(self.start <= self.end)]
    pub struct Span {
        start: usize,
        end: usize,
    }
}

fn main() {
    let _ = data!(model::Span { start: 0, end: 4 });
}
