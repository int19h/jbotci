#[bityzba::invariant(true)]
struct SyntaxGrammarEnv;

#[bityzba::invariant(true)]
struct Token;

jbotci_syntax_macros::syntax_grammar! {
    env SyntaxGrammarEnv;

    recursive {
        item: Token;
    }

    rule "bad enum" bad_enum -> enum {
        missing_rule,
    }
}

fn main() {}
