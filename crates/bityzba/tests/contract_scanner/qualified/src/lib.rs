#[bityzba::invariant(true)]
struct QualifiedType {
    value: usize,
}

#[bityzba::contract_trait]
trait QualifiedTrait {
    #[bityzba::requires(true)]
    #[bityzba::ensures(true)]
    fn value(&self) -> usize;
}

#[bityzba::contract_trait]
impl QualifiedTrait for QualifiedType {
    fn value(&self) -> usize {
        self.value
    }
}

#[bityzba::requires(true)]
#[bityzba::ensures(ret > 0)]
fn qualified_function() -> usize {
    1
}

struct TuplePolicy(usize);

struct UnitPolicy;
