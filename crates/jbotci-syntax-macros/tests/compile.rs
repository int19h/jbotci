#[allow(unused_imports)]
use bityzba::{ensures, requires};

#[test]
#[requires(true)]
#[ensures(true)]
fn syntax_grammar_ui_tests() {
    let tests = trybuild::TestCases::new();
    tests.compile_fail("ui/syntax_grammar/fail/*.rs");
}
