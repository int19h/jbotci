use bityzba::{fields, invariant};

#[invariant(self.start <= self.end)]
struct Span {
    start: usize,
    end: usize,
}

fn main() {
    let _ = Span::new(fields! {
        start: 0,
    });
}
