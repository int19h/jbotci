use bityzba::{fields, invariant};

#[invariant(self.start <= self.end)]
struct Span {
    start: usize,
    end: usize,
}

fn main() {
    let mut span = Span::new(fields! {
        start: 0,
        end: 1,
    });

    span.end = 2;
}
