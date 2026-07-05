#[foo::invariant(true)]
struct SpoofedType {
    value: usize,
}

#[foo::contract_trait]
trait SpoofedTrait {
    #[foo::requires(true)]
    #[foo::ensures(true)]
    fn value(&self) -> usize;
}

#[foo::requires(true)]
#[foo::ensures(true)]
fn spoofed_function() {}

struct TuplePolicy(usize);

struct UnitPolicy;
