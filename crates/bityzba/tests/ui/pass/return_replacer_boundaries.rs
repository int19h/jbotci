use bityzba::ensures;

#[ensures(ret == 1)]
fn outer() -> usize {
    fn nested() -> usize {
        return 2;
    }

    let future = async {
        return 3usize;
    };

    let closure = || {
        return 4usize;
    };

    let _ = (nested, future, closure);

    return 1;
}

fn main() {
    let _ = outer();
}
