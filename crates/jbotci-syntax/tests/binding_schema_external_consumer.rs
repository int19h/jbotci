//! Cross-crate compile-time binding-schema consumer coverage.

use jbotci_syntax_binding_schema_test_macro::consume_syntax_binding_schema;

jbotci_syntax::__jbotci_syntax_binding_schema!(consume_syntax_binding_schema);

#[bityzba::requires(true)]
#[bityzba::ensures(true)]
#[test]
fn external_proc_macro_consumes_complete_schema_without_python_dependencies() {
    assert!(EXTERNAL_SCHEMA_REPRESENTATIVE_RECORDS_VALID);
    assert!(EXTERNAL_SCHEMA_PRODUCT_COUNT > 300);
    assert!(EXTERNAL_SCHEMA_SUM_COUNT > 50);
    assert!(EXTERNAL_SCHEMA_VARIANT_COUNT > 300);
    assert!(EXTERNAL_SCHEMA_FIELD_COUNT > 1_000);
}
