#[bityzba::invariant(true)]
struct SyntaxGrammarEnv;

#[bityzba::invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Token;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Cmavo {
    Be,
    Bo,
}

jbotci_syntax_macros::syntax_grammar! {
    tree_model {}
    model;
    env SyntaxGrammarEnv;

    /// Syntax model for first leaf parsed by the `first_leaf` grammar rule.
    rule "first leaf" first_leaf -> struct {
        /// The source-ordered `token` component retained by the `first_leaf` syntax node.
        field token <- cmavo(Be);
    }

    /// Syntax model for second leaf parsed by the `second_leaf` grammar rule.
    rule "second leaf" second_leaf -> struct {
        /// The source-ordered `token` component retained by the `second_leaf` syntax node.
        field token <- cmavo(Bo);
    }

    /// Syntax model for leaf parsed by the `leaf` grammar rule.
    rule "leaf" leaf -> enum {
        /// The first leaf.
        first_leaf,
        /// The second leaf.
        second_leaf,
    }

    /// Syntax model for parent parsed by the `parent` grammar rule.
    rule "parent" parent -> enum {
        /// The second leaf, written in the parent as well.
        second_leaf,
        splice leaf,
    }
}

fn main() {}
