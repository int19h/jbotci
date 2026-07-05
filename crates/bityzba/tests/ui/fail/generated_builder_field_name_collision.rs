use bityzba::invariant;

#[invariant(*build <= *from_data && *from_data <= *new)]
struct Collision {
    build: usize,
    from_data: usize,
    new: usize,
}

fn main() {}
