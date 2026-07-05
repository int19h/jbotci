#[allow(unused_imports)]
use bityzba::{ensures, requires};

#[test]
#[requires(true)]
#[ensures(true)]
fn tree_model_ui_tests() {
    let tests = trybuild::TestCases::new();
    tests.pass("ui/tree_model/pass/*.rs");
    tests.compile_fail("ui/tree_model/fail/*.rs");
}
