jbotci_syntax_macros::syntax_grammar! {
    warnings {
        pub enum ExperimentalConstruct {}
    }
    rule "item" item -> struct {
        field token <- cmavo(Be).warn(Unknown);
    }
}

fn main() {}
