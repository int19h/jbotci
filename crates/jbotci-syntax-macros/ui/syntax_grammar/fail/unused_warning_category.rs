jbotci_syntax_macros::syntax_grammar! {
    warnings {
        pub enum ExperimentalConstruct {
            Unused => ("syntax.warning.unused", "unused warning"),
        }
    }
}

fn main() {}
