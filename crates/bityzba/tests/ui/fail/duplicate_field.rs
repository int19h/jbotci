use bityzba::{fields, invariant};

#[invariant(self.start <= self.end)]
struct Span {
    start: usize,
    end: usize,
}

fn main() {
    let _ = Span::try_from_fields(fields! {
        start: 0,
        start: 1,
        end: 2,
    });
}
