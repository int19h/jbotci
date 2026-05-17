/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use std::collections::BTreeSet;

use proc_macro2::{Ident, TokenStream};
use quote::{ToTokens, quote};
use syn::parse::{Parse, ParseStream, Parser};
use syn::punctuated::Punctuated;
use syn::{Expr, FieldPat, Pat, PatStruct, Path, Result, Token};

pub(crate) fn fields(input: TokenStream) -> TokenStream {
    if let Ok(pattern) = Pat::parse_single.parse2(input.clone()) {
        return match pattern {
            Pat::Struct(pattern) => fields_struct_pattern(pattern),
            Pat::Path(mut pattern) => {
                rewrite_path_to_raw_type(&mut pattern.path);
                pattern.into_token_stream()
            }
            _ => syn::Error::new_spanned(
                pattern,
                "fields! pattern aliases support struct and path patterns",
            )
            .to_compile_error(),
        };
    }

    match syn::parse2::<FieldAssignments>(input) {
        Ok(assignments) => fields_expression(assignments),
        Err(error) => error.to_compile_error(),
    }
}

fn fields_struct_pattern(mut pattern: PatStruct) -> TokenStream {
    rewrite_path_to_raw_type(&mut pattern.path);
    pattern.into_token_stream()
}

fn fields_expression(assignments: FieldAssignments) -> TokenStream {
    let mut seen = BTreeSet::new();
    let mut setters = TokenStream::new();

    for assignment in assignments.assignments {
        let name = assignment.name;
        if !seen.insert(name.to_string()) {
            return syn::Error::new_spanned(name, "duplicate field in fields! macro")
                .to_compile_error();
        }
        let value = assignment.value;
        setters.extend(quote! {
            .#name(#value)
        });
    }

    quote! {
        |__bityzba_fields| __bityzba_fields #setters
    }
}

fn rewrite_path_to_raw_type(path: &mut Path) {
    if path.segments.is_empty() {
        return;
    }

    let raw_segment_index = if path.segments.len() >= 2
        && path
            .segments
            .iter()
            .nth(path.segments.len() - 2)
            .is_some_and(|segment| starts_with_uppercase(&segment.ident))
    {
        path.segments.len() - 2
    } else {
        path.segments.len() - 1
    };

    let segment = path
        .segments
        .iter_mut()
        .nth(raw_segment_index)
        .expect("path segment index is in bounds");
    segment.ident = Ident::new(&format!("{}Raw", segment.ident), segment.ident.span());
}

fn starts_with_uppercase(ident: &Ident) -> bool {
    ident
        .to_string()
        .chars()
        .next()
        .is_some_and(char::is_uppercase)
}

struct FieldAssignments {
    assignments: Vec<FieldAssignment>,
}

impl Parse for FieldAssignments {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let punctuated = Punctuated::<FieldAssignment, Token![,]>::parse_terminated(input)?;
        Ok(Self {
            assignments: punctuated.into_iter().collect(),
        })
    }
}

struct FieldAssignment {
    name: Ident,
    value: Expr,
}

impl Parse for FieldAssignment {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let name = input.parse()?;
        input.parse::<Token![:]>()?;
        let value = input.parse()?;
        Ok(Self { name, value })
    }
}

#[allow(dead_code)]
fn _assert_pattern_supports_field_patterns(_: FieldPat, _: Pat) {}
