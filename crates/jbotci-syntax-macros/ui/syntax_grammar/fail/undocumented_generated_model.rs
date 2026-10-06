#[bityzba::invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
struct Token;

jbotci_syntax_macros::syntax_grammar! {
    tree_model {}
    model;

    rule "undocumented type" undocumented_type -> struct {
        /// A documented field of an undocumented type.
        field token: Token = Token;
    }

    /// A documented type with an undocumented field.
    rule "undocumented field" undocumented_field -> struct {
        field token: Token = Token;
    }

    /// A documented enum with an undocumented variant.
    rule "undocumented variant" undocumented_variant -> enum {
        undocumented_field,
    }
}

fn main() {}
