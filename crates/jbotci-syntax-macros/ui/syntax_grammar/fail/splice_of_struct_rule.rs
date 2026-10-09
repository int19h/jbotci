#[bityzba::invariant(true)]
struct SyntaxGrammarEnv;

#[bityzba::invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Token;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Cmavo {
    Be,
}

jbotci_syntax_macros::syntax_grammar! {
    tree_model {}
    model;
    env SyntaxGrammarEnv;

    /// Syntax model for leaf parsed by the `leaf` grammar rule.
    rule "leaf" leaf -> struct {
        /// The source-ordered `token` component retained by the `leaf` syntax node.
        field token <- cmavo(Be);
    }

    /// Syntax model for parent parsed by the `parent` grammar rule.
    rule "parent" parent -> enum {
        splice leaf,
    }
}

fn main() {}
