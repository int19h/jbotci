/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use std::collections::BTreeSet;

use proc_macro2::{Ident, TokenStream};
use quote::{ToTokens, quote};
use syn::parse::{Parse, ParseStream, Parser};
use syn::punctuated::Punctuated;
use syn::{
    Expr, ExprCall, ExprPath, ExprStruct, FieldPat, FieldValue, Member, Pat, PatStruct,
    PatTupleStruct, Path, Result, Token,
};

pub(crate) fn data(input: TokenStream) -> TokenStream {
    if let Ok(mut expression) = syn::parse2::<ExprStruct>(input.clone()) {
        rewrite_path_to_data_type(&mut expression.path);
        return expression.into_token_stream();
    }

    if let Ok(mut expression) = syn::parse2::<ExprCall>(input.clone()) {
        if let Expr::Path(path) = expression.func.as_mut() {
            rewrite_path_to_data_type(&mut path.path);
            return expression.into_token_stream();
        }
    }

    if let Ok(mut expression) = syn::parse2::<ExprPath>(input.clone()) {
        rewrite_path_to_data_type(&mut expression.path);
        return expression.into_token_stream();
    }

    if let Ok(pattern) = Pat::parse_single.parse2(input.clone()) {
        return match pattern {
            Pat::Struct(pattern) => data_struct_pattern(pattern),
            Pat::TupleStruct(pattern) => data_tuple_struct_pattern(pattern),
            Pat::Path(mut pattern) => {
                rewrite_path_to_data_type(&mut pattern.path);
                pattern.into_token_stream()
            }
            _ => syn::Error::new_spanned(
                pattern,
                "data! pattern aliases support struct, tuple struct, and path patterns",
            )
            .to_compile_error(),
        };
    }

    match syn::parse2::<DataAssignments>(input) {
        Ok(assignments) => data_expression(assignments),
        Err(error) => error.to_compile_error(),
    }
}

fn data_struct_pattern(mut pattern: PatStruct) -> TokenStream {
    rewrite_path_to_data_type(&mut pattern.path);
    pattern.into_token_stream()
}

fn data_tuple_struct_pattern(mut pattern: PatTupleStruct) -> TokenStream {
    rewrite_path_to_data_type(&mut pattern.path);
    pattern.into_token_stream()
}

fn data_expression(assignments: DataAssignments) -> TokenStream {
    let setters = match setters_for_assignments(assignments.assignments, "data!") {
        Ok(setters) => setters,
        Err(error) => return error.to_compile_error(),
    };

    quote! {
        |__bityzba_data| __bityzba_data #setters
    }
}

pub(crate) fn new_value(input: TokenStream) -> TokenStream {
    construct_value(input, ConstructionMode::Panicking)
}

pub(crate) fn try_new_value(input: TokenStream) -> TokenStream {
    construct_value(input, ConstructionMode::Fallible)
}

#[derive(Debug, Clone, Copy)]
enum ConstructionMode {
    Panicking,
    Fallible,
}

fn construct_value(input: TokenStream, mode: ConstructionMode) -> TokenStream {
    if let Ok(expression) = syn::parse2::<ExprStruct>(input.clone()) {
        return construct_named_value(expression, mode);
    }

    if let Ok(expression) = syn::parse2::<ExprCall>(input.clone()) {
        return construct_tuple_variant(expression, mode);
    }

    if let Ok(expression) = syn::parse2::<ExprPath>(input.clone()) {
        return construct_unit_variant(expression, mode);
    }

    syn::Error::new_spanned(
        input,
        "new! and try_new! expect a struct value or enum variant",
    )
    .to_compile_error()
}

fn construct_named_value(expression: ExprStruct, mode: ConstructionMode) -> TokenStream {
    if expression.rest.is_some() {
        return syn::Error::new_spanned(
            expression,
            "new! and try_new! require an explicit full field list",
        )
        .to_compile_error();
    }

    if path_has_variant_segment(&expression.path) {
        construct_named_variant(expression, mode)
    } else {
        construct_struct(expression, mode)
    }
}

fn construct_struct(expression: ExprStruct, mode: ConstructionMode) -> TokenStream {
    let type_path = expression.path;
    let assignments = match assignments_for_field_values(expression.fields, "new!") {
        Ok(assignments) => assignments,
        Err(error) => return error.to_compile_error(),
    };
    let setters = match setters_for_assignments(assignments, "new!") {
        Ok(setters) => setters,
        Err(error) => return error.to_compile_error(),
    };
    let method = match mode {
        ConstructionMode::Panicking => quote! { __bityzba_from_data_builder },
        ConstructionMode::Fallible => quote! { __bityzba_try_from_data_builder },
    };

    quote! {
        #type_path::#method(|__bityzba_data| __bityzba_data #setters)
    }
}

fn construct_named_variant(expression: ExprStruct, mode: ConstructionMode) -> TokenStream {
    let wrapper_path = match wrapper_path_for_variant(&expression.path) {
        Some(path) => path,
        None => {
            return syn::Error::new_spanned(
                expression.path,
                "new! and try_new! named variant construction requires Type::Variant syntax",
            )
            .to_compile_error();
        }
    };
    let mut data_path = expression.path;
    rewrite_path_to_data_type(&mut data_path);
    let fields = expression.fields;
    if let Err(error) = reject_duplicate_field_values(&fields, "new!") {
        return error.to_compile_error();
    }
    match mode {
        ConstructionMode::Panicking => quote! {
            #wrapper_path::from_data(#data_path { #fields })
        },
        ConstructionMode::Fallible => quote! {
            #wrapper_path::try_from_data(#data_path { #fields })
        },
    }
}

fn construct_tuple_variant(expression: ExprCall, mode: ConstructionMode) -> TokenStream {
    let func = expression.func;
    let args = expression.args;
    let Expr::Path(callee) = *func else {
        return syn::Error::new_spanned(
            func,
            "new! and try_new! tuple variant construction requires Type::Variant(...) syntax",
        )
        .to_compile_error();
    };
    let wrapper_path = match wrapper_path_for_variant(&callee.path) {
        Some(path) => path,
        None => {
            return syn::Error::new_spanned(
                callee.path,
                "new! and try_new! tuple variant construction requires Type::Variant(...) syntax",
            )
            .to_compile_error();
        }
    };
    let mut data_path = callee.path;
    rewrite_path_to_data_type(&mut data_path);
    match mode {
        ConstructionMode::Panicking => quote! {
            #wrapper_path::from_data(#data_path(#args))
        },
        ConstructionMode::Fallible => quote! {
            #wrapper_path::try_from_data(#data_path(#args))
        },
    }
}

fn construct_unit_variant(expression: ExprPath, mode: ConstructionMode) -> TokenStream {
    let wrapper_path = match wrapper_path_for_variant(&expression.path) {
        Some(path) => path,
        None => {
            return syn::Error::new_spanned(
                expression.path,
                "new! and try_new! unit variant construction requires Type::Variant syntax",
            )
            .to_compile_error();
        }
    };
    let mut data_path = expression.path;
    rewrite_path_to_data_type(&mut data_path);
    match mode {
        ConstructionMode::Panicking => quote! {
            #wrapper_path::from_data(#data_path)
        },
        ConstructionMode::Fallible => quote! {
            #wrapper_path::try_from_data(#data_path)
        },
    }
}

fn rewrite_path_to_data_type(path: &mut Path) {
    if path.segments.is_empty() {
        return;
    }

    let data_segment_index = data_type_segment_index(path);

    let segment = path
        .segments
        .iter_mut()
        .nth(data_segment_index)
        .expect("path segment index is in bounds");
    segment.ident = Ident::new(&format!("{}Data", segment.ident), segment.ident.span());
}

fn data_type_segment_index(path: &Path) -> usize {
    if path.segments.len() >= 2
        && path
            .segments
            .iter()
            .nth(path.segments.len() - 2)
            .is_some_and(|segment| starts_with_uppercase(&segment.ident))
    {
        path.segments.len() - 2
    } else {
        path.segments.len().saturating_sub(1)
    }
}

fn path_has_variant_segment(path: &Path) -> bool {
    data_type_segment_index(path) + 1 < path.segments.len()
}

fn wrapper_path_for_variant(path: &Path) -> Option<Path> {
    let type_segment_index = data_type_segment_index(path);
    if type_segment_index + 1 >= path.segments.len() {
        return None;
    }

    let mut segments = Punctuated::new();
    for segment in path.segments.iter().take(type_segment_index + 1) {
        segments.push(segment.clone());
    }
    Some(Path {
        leading_colon: path.leading_colon,
        segments,
    })
}

fn assignments_for_field_values(
    fields: Punctuated<FieldValue, Token![,]>,
    macro_name: &'static str,
) -> Result<Vec<DataAssignment>> {
    fields
        .into_iter()
        .map(|field| {
            let member = field.member;
            let expr = field.expr;
            let Member::Named(name) = member else {
                return Err(syn::Error::new_spanned(
                    member,
                    format!("{macro_name} struct construction requires named fields"),
                ));
            };
            Ok(DataAssignment { name, value: expr })
        })
        .collect()
}

fn reject_duplicate_field_values(
    fields: &Punctuated<FieldValue, Token![,]>,
    macro_name: &'static str,
) -> Result<()> {
    let mut seen = BTreeSet::new();
    for field in fields {
        let Member::Named(name) = &field.member else {
            continue;
        };
        if !seen.insert(name.to_string()) {
            return Err(syn::Error::new_spanned(
                name,
                format!("duplicate field in {macro_name} macro"),
            ));
        }
    }
    Ok(())
}

fn setters_for_assignments(
    assignments: Vec<DataAssignment>,
    macro_name: &'static str,
) -> Result<TokenStream> {
    let mut seen = BTreeSet::new();
    let mut setters = TokenStream::new();

    for assignment in assignments {
        let name = assignment.name;
        if !seen.insert(name.to_string()) {
            return Err(syn::Error::new_spanned(
                name,
                format!("duplicate field in {macro_name} macro"),
            ));
        }
        let value = assignment.value;
        setters.extend(quote! {
            .#name(#value)
        });
    }

    Ok(setters)
}

fn starts_with_uppercase(ident: &Ident) -> bool {
    ident
        .to_string()
        .chars()
        .next()
        .is_some_and(char::is_uppercase)
}

struct DataAssignments {
    assignments: Vec<DataAssignment>,
}

impl Parse for DataAssignments {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let punctuated = Punctuated::<DataAssignment, Token![,]>::parse_terminated(input)?;
        Ok(Self {
            assignments: punctuated.into_iter().collect(),
        })
    }
}

struct DataAssignment {
    name: Ident,
    value: Expr,
}

impl Parse for DataAssignment {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let name = input.parse()?;
        input.parse::<Token![:]>()?;
        let value = input.parse()?;
        Ok(Self { name, value })
    }
}

#[allow(dead_code)]
fn _assert_pattern_supports_field_patterns(_: FieldPat, _: Pat) {}
