use bityzba::contract_trait;

#[contract_trait]
trait PatternTrait {
    #[bityzba::requires(value > 0)]
    #[bityzba::ensures(ret > value)]
    fn double(mut value: usize) -> usize {
        value += 1;
        value * 2
    }

    #[bityzba::requires(true)]
    #[bityzba::ensures(ret == 7)]
    fn wildcard(_: usize) -> usize;
}

struct Impl;

#[contract_trait]
impl PatternTrait for Impl {
    fn wildcard(_: usize) -> usize {
        7
    }
}

fn main() {
    assert_eq!(Impl::double(1), 4);
    assert_eq!(Impl::wildcard(0), 7);
}
