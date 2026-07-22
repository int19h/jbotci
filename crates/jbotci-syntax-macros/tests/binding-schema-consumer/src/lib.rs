//! External compile-time consumer used to prove that the exported schema does
//! not depend on proc-macro implementation details or Python bindings.

use proc_macro::{TokenStream, TokenTree};

#[bityzba::invariant(true)]
#[derive(Default)]
struct SchemaSummary {
    saw_root: bool,
    saw_version: bool,
    saw_recovered_field: bool,
    products: usize,
    sums: usize,
    variants: usize,
    fields: usize,
}

#[bityzba::requires(true)]
#[bityzba::ensures(true)]
fn inspect(stream: TokenStream, summary: &mut SchemaSummary) {
    for token in stream {
        match token {
            TokenTree::Ident(ident) => match ident.to_string().as_str() {
                "syntax_binding_schema" => summary.saw_root = true,
                "version" => summary.saw_version = true,
                "recovered_field" => summary.saw_recovered_field = true,
                "product" => summary.products += 1,
                "sum" => summary.sums += 1,
                "variant" => summary.variants += 1,
                "field" => summary.fields += 1,
                _ => {}
            },
            TokenTree::Group(group) => inspect(group.stream(), summary),
            TokenTree::Punct(_) | TokenTree::Literal(_) => {}
        }
    }
}

#[bityzba::requires(true)]
#[bityzba::ensures(true)]
#[proc_macro]
pub fn consume_syntax_binding_schema(input: TokenStream) -> TokenStream {
    let mut summary = SchemaSummary::default();
    inspect(input, &mut summary);
    assert!(summary.saw_root, "schema root marker is missing");
    assert!(summary.saw_version, "schema version is missing");
    assert!(
        summary.saw_recovered_field,
        "recovered field cardinality is missing"
    );
    assert!(summary.products > 0, "schema has no product models");
    assert!(summary.sums > 0, "schema has no sum models");
    assert!(summary.variants > 0, "schema has no variants");
    assert!(summary.fields > 0, "schema has no fields");

    format!(
        "const EXTERNAL_SCHEMA_PRODUCT_COUNT: usize = {};\n\
         const EXTERNAL_SCHEMA_SUM_COUNT: usize = {};\n\
         const EXTERNAL_SCHEMA_VARIANT_COUNT: usize = {};\n\
         const EXTERNAL_SCHEMA_FIELD_COUNT: usize = {};",
        summary.products, summary.sums, summary.variants, summary.fields,
    )
    .parse()
    .expect("generated schema summary constants must parse")
}
