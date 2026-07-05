#[bityzba::invariant(true)]
struct SyntaxGrammarEnv;

#[bityzba::invariant(true)]
struct Token;

jbotci_syntax_macros::syntax_grammar! {
    env SyntaxGrammarEnv;

    recursive {
        first: Token;
        second: Token;
    }

    rule "bad arguments" bad(first second) -> struct {
        field token <- first;
    }
}

fn main() {}
