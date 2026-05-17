//! Project-local contract attributes.

use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn expensive_requires(attr: TokenStream, item: TokenStream) -> TokenStream {
    gated_contract("requires", attr, item)
}

#[proc_macro_attribute]
pub fn expensive_ensures(attr: TokenStream, item: TokenStream) -> TokenStream {
    gated_contract("ensures", attr, item)
}

#[proc_macro_attribute]
pub fn expensive_invariant(attr: TokenStream, item: TokenStream) -> TokenStream {
    gated_contract("invariant", attr, item)
}

fn gated_contract(kind: &str, attr: TokenStream, item: TokenStream) -> TokenStream {
    let prefix = format!(
        "#[cfg_attr(feature = \"expensive_contracts\", contracts::{kind}({}))]",
        attr
    );
    let mut output: TokenStream = prefix
        .parse()
        .expect("generated expensive contract attribute must parse");
    output.extend(item);
    output
}
