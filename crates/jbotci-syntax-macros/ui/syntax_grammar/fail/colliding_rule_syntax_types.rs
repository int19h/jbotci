#[bityzba::invariant(true)]
struct SyntaxGrammarEnv;

#[bityzba::invariant(true)]
struct Token;

jbotci_syntax_macros::syntax_grammar! {
    env SyntaxGrammarEnv;

    recursive {
        item: Token;
    }

    rule "first" foo_bar(item) -> struct {
        field token <- item;
    }

    rule "second" foo__bar(item) -> struct {
        field token <- item;
    }
}

fn main() {}
