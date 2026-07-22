#[bityzba::invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
struct Token;

#[bityzba::invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
struct WithFreeModifiers<T, F> {
    value: T,
    free_modifiers: Vec<F>,
}

mod foreign {
    #[bityzba::invariant(true)]
    #[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
    pub struct FreeModifierSyntax;
}

jbotci_syntax_macros::syntax_grammar! {
    tree_model {}
    model;

    /// A model whose free-modifier payload cannot match recovered syntax.
    rule "noncanonical free modifiers" noncanonical_free_modifiers -> struct {
        /// A wrapper with a noncanonical free-modifier type argument.
        field collision: WithFreeModifiers<Token, foreign::FreeModifierSyntax> = unreachable!();
    }
}

fn main() {}
