use jbotci_tree::tree_model;

tree_model! {
    pub struct Root {
        #[tree_child(primary)]
        #[tree_child(false)]
        pub text: String,
    }
}

fn main() {}
