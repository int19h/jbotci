use bityzba::{fields, invariant};

#[invariant(self.start <= self.end)]
struct Span {
    start: usize,
    end: usize,
}

fn main() {
    let mut span = Span::try_from_fields(fields! {
        start: 0,
        end: 1,
    })
    .unwrap();

    span.end = 2;
}
