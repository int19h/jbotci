use bityzba::{invariant, new};

#[invariant(self.start <= self.end)]
struct Span {
    start: usize,
    end: usize,
}

fn main() {
    let mut span = new!(Span {
        start: 0,
        end: 1,
    });

    span.end = 2;
}
