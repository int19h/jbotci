use jbotci_tree::tree_model;

#[derive(Debug)]
pub struct Bar;

#[derive(Debug)]
pub struct Foo<T>(T);

#[derive(Debug)]
pub struct FooBar;

tree_model! {
    #[derive(Debug)]
    pub struct Root {
        pub generic: Foo<Bar>,
        pub flat: FooBar,
    }
}

fn main() {
    let root = Root {
        generic: Foo(Bar),
        flat: FooBar,
    };
    let _ = root;
}
