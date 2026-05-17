use bityzba::invariant;

#[invariant(self.start <= self.end)]
struct Span {
    start: usize,
    end: usize,
}

fn main() {
    let _ = Span {
        start: 0,
        end: 1,
    };
}
