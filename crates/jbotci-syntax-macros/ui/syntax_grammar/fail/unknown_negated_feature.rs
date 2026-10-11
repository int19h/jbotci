#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Cmavo { Bo }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Selmaho {}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SyntaxWordCategory {}
struct SyntaxGrammarEnv;

jbotci_syntax_macros::syntax_grammar! {
    env SyntaxGrammarEnv;
    rule "leaf" leaf -> struct {
        field value: usize = 0usize;
    }
    rule "choice" selection -> enum {
        when !feature(MisspelledFeature) leaf,
    }
}

fn main() {}
