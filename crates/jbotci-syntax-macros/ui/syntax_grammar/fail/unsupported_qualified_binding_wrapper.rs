#[bityzba::invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
struct Token;

mod foreign {
    #[bityzba::invariant(true)]
    #[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
    pub struct Option<T>(pub T);
}

jbotci_syntax_macros::syntax_grammar! {
    tree_model {}
    model;

    /// A model whose qualified wrapper collides with a supported basename.
    rule "qualified wrapper collision" qualified_wrapper_collision -> struct {
        /// A foreign generic must not acquire built-in optional cardinality.
        field collision: foreign::Option<Token> = unreachable!();
    }
}

fn main() {}
