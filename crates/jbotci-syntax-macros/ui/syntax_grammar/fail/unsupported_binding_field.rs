#[bityzba::invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
struct Token;

jbotci_syntax_macros::syntax_grammar! {
    tree_model {}
    model;

    /// A model with a field shape that bindings cannot represent canonically.
    rule "unsupported binding field" unsupported_binding_field -> struct {
        /// An unsupported associative container.
        field mapping: std::collections::HashMap<String, Token> = unreachable!();
    }
}

fn main() {}
