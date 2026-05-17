use bityzba::{invariant, new};

#[invariant(self.start <= self.end)]
struct Span {
    start: usize,
    end: usize,
}

fn main() {
    let _ = new!(Span {
        start: "zero",
        end: 2,
    });
}
