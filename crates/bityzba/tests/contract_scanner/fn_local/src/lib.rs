#[requires(true)]
#[ensures(true)]
fn outer() {
    struct LocalStruct {
        value: usize,
    }

    #[invariant(true)]
    enum LocalEnum {
        Empty,
        Present { value: usize },
    }

    trait LocalTrait {
        fn run(&self);
    }

    fn inner() {}

    impl LocalStruct {
        fn local_method(&self) -> usize {
            self.value
        }
    }

    let _ = {
        fn inside_expression() {}
        0usize
    };

    impl Provides for Provider {
        fn provided(&self) {
            struct InsideTraitImpl;
            let _ = InsideTraitImpl;
        }
    }
}

#[contract_trait]
trait Provides {
    #[requires(true)]
    #[ensures(true)]
    fn provided(&self);
}

#[invariant(!name.is_empty())]
struct Provider {
    name: String,
}
