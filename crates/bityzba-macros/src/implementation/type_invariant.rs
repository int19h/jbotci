/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use std::collections::{BTreeMap, BTreeSet};

use proc_macro2::{Spacing, TokenStream, TokenTree};
use quote::{ToTokens, format_ident, quote};
use syn::{
    Attribute, Expr, ExprLit, Fields, FieldsNamed, GenericParam, Generics, Ident, ItemEnum,
    ItemStruct, Lit, Type, TypePath, Variant, Visibility, parse::Parser, visit, visit::Visit,
};

use crate::implementation::{Contract, ContractMode, ContractType, parse};

pub(crate) fn invariant_struct(
    mode: ContractMode,
    attr: TokenStream,
    mut item: ItemStruct,
) -> TokenStream {
    let contracts = collect_type_invariants(mode, attr, &mut item.attrs);
    let option_errors = collect_type_option_errors(&mut item.attrs);
    if contracts_are_true_marker(&contracts) {
        return quote! {
            #(#option_errors)*
            #item
        };
    }

    match &item.fields {
        Fields::Named(fields) => {
            generate_struct(contracts, option_errors, item.clone(), fields.clone())
        }
        _ => syn::Error::new_spanned(
            item.ident,
            "type-level #[invariant] currently requires a struct with named fields",
        )
        .to_compile_error(),
    }
}

pub(crate) fn invariant_enum(
    mode: ContractMode,
    attr: TokenStream,
    mut item: ItemEnum,
) -> TokenStream {
    let contracts = collect_enum_type_invariants(mode, attr, &mut item.attrs);
    let option_errors = collect_type_option_errors(&mut item.attrs);
    if enum_contracts_are_true_marker(&contracts) {
        let shape = TypeShape::new(&item.ident, &item.vis, &item.generics, &item.attrs);
        let contract_errors = &contracts.errors;
        let (_, variant_errors) = enum_variant_invariant_expression(
            &shape.data_ident,
            item.variants.iter(),
            &contracts.variant_arms,
        );
        if variant_errors.is_empty() {
            return quote! {
                #(#option_errors)*
                #(#contract_errors)*
                #item
            };
        }
        return quote! {
            #(#option_errors)*
            #(#contract_errors)*
            #(#variant_errors)*
            #item
        };
    }

    generate_enum(contracts, option_errors, item)
}

fn generate_struct(
    contracts: Vec<Contract>,
    option_errors: Vec<TokenStream>,
    item: ItemStruct,
    fields: FieldsNamed,
) -> TokenStream {
    let shape = TypeShape::new(&item.ident, &item.vis, &item.generics, &item.attrs);
    let generics = &item.generics;
    let data_attrs = item.attrs.clone();
    let data_ident = shape.data_ident.clone();
    let wrapper_ident = shape.wrapper_ident.clone();
    let error_ident = shape.error_ident.clone();
    let builder_ident = shape.builder_ident.clone();
    let update_builder_ident = format_ident!("{wrapper_ident}DataUpdate");
    let builder_unset_ident = format_ident!("{builder_ident}Unset");
    let builder_set_ident = format_ident!("{builder_ident}Set");
    let data_vis = shape.data_vis.clone();
    let wrapper_vis = shape.wrapper_vis.clone();
    let field_idents = fields
        .named
        .iter()
        .map(|field| field.ident.clone().expect("named field"))
        .collect::<Vec<_>>();
    let field_types = fields
        .named
        .iter()
        .map(|field| field.ty.clone())
        .collect::<Vec<_>>();
    let data_fields = fields.named.iter().collect::<Vec<_>>();
    let state_idents = field_idents
        .iter()
        .map(|ident| format_ident!("Bityzba{}State", snake_to_upper_camel(&ident.to_string())))
        .collect::<Vec<_>>();
    let type_args = generic_arguments(generics);
    let all_unset_states = field_idents
        .iter()
        .map(|_| quote! { #builder_unset_ident })
        .collect::<Vec<_>>();
    let all_set_states = field_idents
        .iter()
        .map(|_| quote! { #builder_set_ident })
        .collect::<Vec<_>>();
    let full_builder_generics = generics_with_state_params(generics, &state_idents);
    let full_builder_where_clause = &full_builder_generics.where_clause;
    let all_unset_builder_ty = type_with_args(
        &builder_ident,
        type_args
            .iter()
            .cloned()
            .chain(all_unset_states.iter().cloned())
            .collect(),
    );
    let all_set_builder_ty = type_with_args(
        &builder_ident,
        type_args
            .iter()
            .cloned()
            .chain(all_set_states.iter().cloned())
            .collect(),
    );
    let setter_impls = field_idents
        .iter()
        .zip(field_types.iter())
        .zip(state_idents.iter())
        .enumerate()
        .map(|(index, ((field_ident, field_type), _fixed_state_ident))| {
            let other_state_idents = state_idents
                .iter()
                .enumerate()
                .filter_map(|(state_index, state_ident)| {
                    (state_index != index).then_some(state_ident.clone())
                })
                .collect::<Vec<_>>();
            let other_field_idents = field_idents
                .iter()
                .enumerate()
                .filter_map(|(field_index, other_field_ident)| {
                    (field_index != index).then_some(other_field_ident.clone())
                })
                .collect::<Vec<_>>();
            let setter_generics = generics_with_state_params(generics, &other_state_idents);
            let (setter_impl_generics, _, setter_where_clause) = setter_generics.split_for_impl();
            let input_states = state_idents
                .iter()
                .enumerate()
                .map(|(state_index, state_ident)| {
                    if state_index == index {
                        quote! { #builder_unset_ident }
                    } else {
                        quote! { #state_ident }
                    }
                })
                .collect::<Vec<_>>();
            let output_states = state_idents
                .iter()
                .enumerate()
                .map(|(state_index, state_ident)| {
                    if state_index == index {
                        quote! { #builder_set_ident }
                    } else {
                        quote! { #state_ident }
                    }
                })
                .collect::<Vec<_>>();
            let input_ty = type_with_args(
                &builder_ident,
                type_args
                    .iter()
                    .cloned()
                    .chain(input_states.iter().cloned())
                    .collect(),
            );
            let output_ty = type_with_args(
                &builder_ident,
                type_args
                    .iter()
                    .cloned()
                    .chain(output_states.iter().cloned())
                    .collect(),
            );
            quote! {
                impl #setter_impl_generics #input_ty #setter_where_clause {
                    pub fn #field_ident(self, value: #field_type) -> #output_ty {
                        #builder_ident {
                            #field_ident: ::std::option::Option::Some(value),
                            #(
                                #other_field_idents: self.#other_field_idents,
                            )*
                            __state: ::std::marker::PhantomData,
                        }
                    }
                }
            }
        })
        .collect::<Vec<_>>();
    let wrapper_attrs = shape.wrapper_attrs();
    let debug_impl = shape.struct_debug_impl(&field_idents, &field_types);
    let serialize_impl = shape.serialize_impl();
    let deserialize_impl = shape.deserialize_impl();
    let default_impl = shape.default_impl();
    let invariant_expr = invariant_expression(&contracts, quote! { true });
    let invariant_docs = invariant_docs(&contracts);
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let turbofish = ty_generics.as_turbofish();

    quote! {
        #(#option_errors)*

        #(#data_attrs)*
        #data_vis struct #data_ident #generics #where_clause {
            #(#data_fields,)*
        }

        #(#wrapper_attrs)*
        #wrapper_vis struct #wrapper_ident #generics (#data_ident #ty_generics) #where_clause;

        #[derive(Debug, Clone, PartialEq, Eq)]
        #wrapper_vis struct #error_ident {
            message: ::std::string::String,
        }

        impl #impl_generics #error_ident #ty_generics #where_clause {
            fn invariant_violation() -> Self {
                Self {
                    message: ::std::format!("type invariant violated for `{}`: {}", ::std::stringify!(#wrapper_ident), #invariant_docs),
                }
            }
        }

        impl #impl_generics ::std::fmt::Display for #error_ident #ty_generics #where_clause {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                f.write_str(&self.message)
            }
        }

        impl #impl_generics ::std::error::Error for #error_ident #ty_generics #where_clause {}

        #[doc(hidden)]
        #wrapper_vis struct #builder_unset_ident;

        #[doc(hidden)]
        #wrapper_vis struct #builder_set_ident;

        #[doc(hidden)]
        #wrapper_vis struct #builder_ident #full_builder_generics #full_builder_where_clause {
            #(#field_idents: ::std::option::Option<#field_types>,)*
            __state: ::std::marker::PhantomData<(#(#state_idents,)*)>,
        }

        impl #impl_generics #all_unset_builder_ty #where_clause {
            fn new() -> Self {
                Self {
                    #(#field_idents: ::std::option::Option::None,)*
                    __state: ::std::marker::PhantomData,
                }
            }
        }

        #(#setter_impls)*

        impl #impl_generics #all_set_builder_ty #where_clause {
            fn build(self) -> #data_ident #ty_generics {
                #data_ident {
                    #(
                        #field_idents: self.#field_idents.expect("typestate builder guarantees every field is set"),
                    )*
                }
            }
        }

        #[doc(hidden)]
        #wrapper_vis struct #update_builder_ident #generics #where_clause {
            #(#field_idents: ::std::option::Option<#field_types>,)*
        }

        impl #impl_generics #update_builder_ident #ty_generics #where_clause {
            fn from_data(data: #data_ident #ty_generics) -> Self {
                Self {
                    #(#field_idents: ::std::option::Option::Some(data.#field_idents),)*
                }
            }

            #(
                pub fn #field_idents(mut self, value: #field_types) -> Self {
                    self.#field_idents = ::std::option::Option::Some(value);
                    self
                }
            )*

            fn build(self) -> #data_ident #ty_generics {
                #data_ident {
                    #(
                        #field_idents: self.#field_idents.expect("update builder starts with every field set"),
                    )*
                }
            }
        }

        impl #impl_generics #wrapper_ident #ty_generics #where_clause {
            #[doc(hidden)]
            pub fn __bityzba_from_data_builder<F>(data: F) -> Self
            where
                F: ::std::ops::FnOnce(#all_unset_builder_ty) -> #all_set_builder_ty,
            {
                Self::from_data(data(#builder_ident::new()).build())
            }

            #[doc(hidden)]
            pub fn __bityzba_try_from_data_builder<F>(data: F) -> ::std::result::Result<Self, #error_ident #ty_generics>
            where
                F: ::std::ops::FnOnce(#all_unset_builder_ty) -> #all_set_builder_ty,
            {
                Self::try_from_data(data(#builder_ident::new()).build())
            }

            pub fn try_from_data(data: #data_ident #ty_generics) -> ::std::result::Result<Self, #error_ident #ty_generics> {
                let value = Self(data);
                if value.__bityzba_invariant() {
                    ::std::result::Result::Ok(value)
                } else {
                    ::std::result::Result::Err(#error_ident #turbofish::invariant_violation())
                }
            }

            pub fn from_data(data: #data_ident #ty_generics) -> Self {
                Self::try_from_data(data)
                    .expect("data value must satisfy type invariant")
            }

            pub fn with_data<F>(self, data: F) -> Self
            where
                F: ::std::ops::FnOnce(#update_builder_ident #ty_generics) -> #update_builder_ident #ty_generics,
            {
                Self::from_data(data(#update_builder_ident::from_data(self.0)).build())
            }

            pub fn as_data(&self) -> &#data_ident #ty_generics {
                &self.0
            }

            pub fn into_data(self) -> #data_ident #ty_generics {
                self.0
            }

            fn __bityzba_invariant(&self) -> bool {
                {
                    let #data_ident { #(#field_idents,)* } = self.as_data();
                    let _ = (#(&#field_idents,)*);
                    #invariant_expr
                }
            }
        }

        impl #impl_generics ::std::convert::TryFrom<#data_ident #ty_generics> for #wrapper_ident #ty_generics #where_clause {
            type Error = #error_ident #ty_generics;

            fn try_from(data: #data_ident #ty_generics) -> ::std::result::Result<Self, Self::Error> {
                Self::try_from_data(data)
            }
        }

        impl #impl_generics ::std::ops::Deref for #wrapper_ident #ty_generics #where_clause {
            type Target = #data_ident #ty_generics;

            fn deref(&self) -> &Self::Target {
                self.as_data()
            }
        }

        #debug_impl
        #serialize_impl
        #deserialize_impl
        #default_impl
    }
}

fn generate_enum(
    contracts: EnumTypeInvariants,
    option_errors: Vec<TokenStream>,
    item: ItemEnum,
) -> TokenStream {
    let shape = TypeShape::new(&item.ident, &item.vis, &item.generics, &item.attrs);
    let data_attrs = item.attrs.clone();
    let data_ident = shape.data_ident.clone();
    let wrapper_ident = shape.wrapper_ident.clone();
    let error_ident = shape.error_ident.clone();
    let data_vis = shape.data_vis.clone();
    let wrapper_vis = shape.wrapper_vis.clone();
    let variants = item.variants;
    let wrapper_attrs = shape.wrapper_attrs();
    let debug_impl = shape.enum_debug_impl(variants.iter());
    let serialize_impl = shape.serialize_impl();
    let deserialize_impl = shape.deserialize_impl();
    let default_impl = shape.default_impl();
    let contract_errors = &contracts.errors;
    let (variant_invariant_expr, variant_errors) =
        enum_variant_invariant_expression(&data_ident, variants.iter(), &contracts.variant_arms);
    let invariant_expr = invariant_expression(&contracts.type_contracts, variant_invariant_expr);
    let invariant_docs = enum_invariant_docs(&contracts);
    let generics = &item.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let turbofish = ty_generics.as_turbofish();

    quote! {
        #(#option_errors)*
        #(#contract_errors)*
        #(#variant_errors)*

        #(#data_attrs)*
        #data_vis enum #data_ident #generics #where_clause {
            #variants
        }

        #(#wrapper_attrs)*
        #wrapper_vis struct #wrapper_ident #generics (#data_ident #ty_generics) #where_clause;

        #[derive(Debug, Clone, PartialEq, Eq)]
        #wrapper_vis struct #error_ident {
            message: ::std::string::String,
        }

        impl #impl_generics #error_ident #ty_generics #where_clause {
            fn invariant_violation() -> Self {
                Self {
                    message: ::std::format!("type invariant violated for `{}`: {}", ::std::stringify!(#wrapper_ident), #invariant_docs),
                }
            }
        }

        impl #impl_generics ::std::fmt::Display for #error_ident #ty_generics #where_clause {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                f.write_str(&self.message)
            }
        }

        impl #impl_generics ::std::error::Error for #error_ident #ty_generics #where_clause {}

        impl #impl_generics #wrapper_ident #ty_generics #where_clause {
            pub fn try_from_data(data: #data_ident #ty_generics) -> ::std::result::Result<Self, #error_ident #ty_generics> {
                let value = Self(data);
                if value.__bityzba_invariant() {
                    ::std::result::Result::Ok(value)
                } else {
                    ::std::result::Result::Err(#error_ident #turbofish::invariant_violation())
                }
            }

            pub fn from_data(data: #data_ident #ty_generics) -> Self {
                Self::try_from_data(data)
                    .expect("data value must satisfy type invariant")
            }

            pub fn as_data(&self) -> &#data_ident #ty_generics {
                &self.0
            }

            pub fn into_data(self) -> #data_ident #ty_generics {
                self.0
            }

            fn __bityzba_invariant(&self) -> bool {
                #invariant_expr
            }
        }

        impl #impl_generics ::std::convert::TryFrom<#data_ident #ty_generics> for #wrapper_ident #ty_generics #where_clause {
            type Error = #error_ident #ty_generics;

            fn try_from(data: #data_ident #ty_generics) -> ::std::result::Result<Self, Self::Error> {
                Self::try_from_data(data)
            }
        }

        impl #impl_generics ::std::ops::Deref for #wrapper_ident #ty_generics #where_clause {
            type Target = #data_ident #ty_generics;

            fn deref(&self) -> &Self::Target {
                self.as_data()
            }
        }

        #debug_impl
        #serialize_impl
        #deserialize_impl
        #default_impl
    }
}

fn collect_type_option_errors(attrs: &mut Vec<Attribute>) -> Vec<TokenStream> {
    let mut errors = Vec::new();
    let mut retained = Vec::new();

    for attr in std::mem::take(attrs) {
        if attr.path().is_ident("bityzba") {
            if let Err(error) = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("no_new") {
                    Err(meta.error(
                        "`#[bityzba(no_new)]` is obsolete; bityzba no longer generates `Type::new`",
                    ))
                } else {
                    Err(meta.error("unsupported bityzba type option"))
                }
            }) {
                errors.push(error.to_compile_error());
            }
        } else {
            retained.push(attr);
        }
    }

    *attrs = retained;
    errors
}

fn collect_type_invariants(
    initial_mode: ContractMode,
    initial_attr: TokenStream,
    attrs: &mut Vec<Attribute>,
) -> Vec<Contract> {
    let mut contracts = vec![Contract::from_toks(
        ContractType::Invariant,
        initial_mode,
        initial_attr,
    )];

    let mut retained = Vec::new();
    for attr in std::mem::take(attrs) {
        let name = attr
            .path()
            .segments
            .last()
            .expect("attribute path has at least one segment")
            .ident
            .to_string();
        if let Some((ContractType::Invariant, mode)) = ContractType::contract_type_and_mode(&name) {
            if let syn::Meta::List(list) = &attr.meta {
                contracts.push(Contract::from_toks(
                    ContractType::Invariant,
                    mode,
                    list.tokens.clone(),
                ));
            }
        } else {
            retained.push(attr);
        }
    }
    *attrs = retained;
    contracts
}

fn collect_enum_type_invariants(
    initial_mode: ContractMode,
    initial_attr: TokenStream,
    attrs: &mut Vec<Attribute>,
) -> EnumTypeInvariants {
    let mut contracts = EnumTypeInvariants::default();
    collect_enum_type_invariant_tokens(initial_mode, initial_attr, &mut contracts);

    let mut retained = Vec::new();
    for attr in std::mem::take(attrs) {
        let name = attr
            .path()
            .segments
            .last()
            .expect("attribute path has at least one segment")
            .ident
            .to_string();
        if let Some((ContractType::Invariant, mode)) = ContractType::contract_type_and_mode(&name) {
            if let syn::Meta::List(list) = &attr.meta {
                collect_enum_type_invariant_tokens(mode, list.tokens.clone(), &mut contracts);
            }
        } else {
            retained.push(attr);
        }
    }
    *attrs = retained;
    contracts
}

fn collect_enum_type_invariant_tokens(
    mode: ContractMode,
    tokens: TokenStream,
    contracts: &mut EnumTypeInvariants,
) {
    let segments = parse::parse_attribute_segments(tokens.clone());
    if segments
        .iter()
        .any(|segment| parse_enum_variant_invariant(mode, segment.clone()).is_some())
    {
        for segment in segments {
            match parse_enum_variant_invariant(mode, segment.clone()) {
                Some(Ok(variant_arm)) => contracts.variant_arms.push(variant_arm),
                Some(Err(error)) => contracts.errors.push(error.to_compile_error()),
                None => contracts.type_contracts.push(Contract::from_toks(
                    ContractType::Invariant,
                    mode,
                    segment,
                )),
            }
        }
    } else {
        contracts
            .type_contracts
            .push(Contract::from_toks(ContractType::Invariant, mode, tokens));
    }
}

fn invariant_expression(contracts: &[Contract], extra_check: TokenStream) -> TokenStream {
    let checks = contracts
        .iter()
        .flat_map(|contract| {
            let mode = contract.mode.final_mode();
            contract.assertions.iter().map(move |expr| {
                if mode == ContractMode::Expensive {
                    quote! { (!cfg!(feature = "expensive_contracts") || (#expr)) }
                } else {
                    quote! { (#expr) }
                }
            })
        })
        .collect::<Vec<_>>();
    quote! { true #(&& #checks)* && (#extra_check) }
}

fn invariant_docs(contracts: &[Contract]) -> String {
    contracts
        .iter()
        .flat_map(|contract| {
            let mut docs = contract
                .streams
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>();
            if let Some(desc) = &contract.desc {
                docs.push(desc.clone());
            }
            docs
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn enum_invariant_docs(contracts: &EnumTypeInvariants) -> String {
    contracts
        .type_contracts
        .iter()
        .flat_map(|contract| {
            let mut docs = contract
                .streams
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>();
            if let Some(desc) = &contract.desc {
                docs.push(desc.clone());
            }
            docs
        })
        .chain(
            contracts
                .variant_arms
                .iter()
                .map(|variant_arm| variant_arm.display.to_string()),
        )
        .collect::<Vec<_>>()
        .join(", ")
}

fn contracts_are_true_marker(contracts: &[Contract]) -> bool {
    contracts.iter().all(|contract| {
        !contract.assertions.is_empty() && contract.assertions.iter().all(is_true_literal)
    })
}

fn enum_contracts_are_true_marker(contracts: &EnumTypeInvariants) -> bool {
    contracts.errors.is_empty()
        && contracts_are_true_marker(&contracts.type_contracts)
        && contracts
            .variant_arms
            .iter()
            .all(|variant_arm| is_true_literal(&variant_arm.expr))
}

fn is_true_literal(expr: &Expr) -> bool {
    matches!(
        expr,
        Expr::Lit(ExprLit {
            lit: Lit::Bool(value),
            ..
        }) if value.value
    )
}

fn enum_variant_invariant_expression<'a>(
    data_ident: &Ident,
    variants: impl Iterator<Item = &'a Variant>,
    variant_arms: &[EnumVariantInvariant],
) -> (TokenStream, Vec<TokenStream>) {
    let variants_by_name = variants
        .map(|variant| (variant.ident.to_string(), variant))
        .collect::<BTreeMap<_, _>>();
    let mut errors = Vec::new();
    let mut seen = BTreeSet::new();
    let mut checks = Vec::new();

    for variant_arm in variant_arms {
        let variant_name = variant_arm.variant_ident.to_string();
        if !seen.insert(variant_name.clone()) {
            errors.push(
                syn::Error::new_spanned(
                    &variant_arm.variant_ident,
                    format!("duplicate invariant for enum variant `{variant_name}`"),
                )
                .to_compile_error(),
            );
            continue;
        }

        let Some(variant) = variants_by_name.get(&variant_name) else {
            errors.push(
                syn::Error::new_spanned(
                    &variant_arm.variant_ident,
                    format!("unknown enum variant `{variant_name}` in invariant"),
                )
                .to_compile_error(),
            );
            continue;
        };

        let (pattern, use_bound_fields) =
            match enum_variant_invariant_pattern(data_ident, variant, &variant_arm.tail) {
                Ok(pattern) => pattern,
                Err(error) => {
                    errors.push(error.to_compile_error());
                    continue;
                }
            };
        let expr = &variant_arm.expr;
        let check = quote! {
            {
                match self.as_data() {
                    #pattern => {
                        #use_bound_fields
                        (#expr)
                    }
                    _ => true,
                }
            }
        };

        if variant_arm.mode.final_mode() == ContractMode::Expensive {
            checks.push(quote! { (!cfg!(feature = "expensive_contracts") || (#check)) });
        } else {
            checks.push(quote! { (#check) });
        }
    }

    for (variant_name, variant) in &variants_by_name {
        if !matches!(variant.fields, Fields::Unit) && !seen.contains(variant_name) {
            errors.push(
                syn::Error::new_spanned(
                    &variant.ident,
                    format!(
                        "missing invariant for data-carrying enum variant `{variant_name}`; add `#[invariant(::{variant_name} => true)]` if the variant data already expresses the invariant"
                    ),
                )
                .to_compile_error(),
            );
        }
    }

    (quote! { true #(&& #checks)* }, errors)
}

fn enum_variant_invariant_pattern(
    data_ident: &Ident,
    variant: &Variant,
    tail: &TokenStream,
) -> syn::Result<(TokenStream, TokenStream)> {
    let variant_ident = &variant.ident;
    if tail.is_empty() {
        return match &variant.fields {
            Fields::Named(fields) => {
                let field_idents = fields
                    .named
                    .iter()
                    .map(|field| field.ident.clone().expect("named field"))
                    .collect::<Vec<_>>();
                Ok((
                    quote! { #data_ident::#variant_ident { #(#field_idents,)* } },
                    quote! { let _ = (#(&#field_idents,)*); },
                ))
            }
            Fields::Unnamed(_) => Err(syn::Error::new_spanned(
                variant_ident,
                "tuple variant invariant requires an explicit tuple pattern",
            )),
            Fields::Unit => Ok((quote! { #data_ident::#variant_ident }, quote! {})),
        };
    }

    let pattern_tokens = quote! { #data_ident::#variant_ident #tail };
    syn::Pat::parse_single
        .parse2(pattern_tokens.clone())
        .map(|_| (pattern_tokens, quote! {}))
}

fn parse_enum_variant_invariant(
    mode: ContractMode,
    segment: TokenStream,
) -> Option<syn::Result<EnumVariantInvariant>> {
    let tokens = segment.clone().into_iter().collect::<Vec<_>>();
    if !starts_with_double_colon(&tokens) {
        return None;
    }
    let arrow_index = top_level_fat_arrow_index(&tokens)?;

    let Some(TokenTree::Ident(variant_ident)) = tokens.get(2) else {
        return Some(Err(syn::Error::new_spanned(
            segment,
            "enum variant invariant must start with `::Variant`",
        )));
    };

    let tail = tokens[3..arrow_index]
        .iter()
        .cloned()
        .collect::<TokenStream>();
    let expr_tokens = tokens[arrow_index + 2..]
        .iter()
        .cloned()
        .collect::<TokenStream>();
    if expr_tokens.is_empty() {
        return Some(Err(syn::Error::new_spanned(
            segment,
            "enum variant invariant requires an expression after `=>`",
        )));
    }

    Some(Ok(EnumVariantInvariant {
        mode,
        variant_ident: variant_ident.clone(),
        tail,
        expr: parse::parse_contract_expr(expr_tokens),
        display: segment,
    }))
}

fn starts_with_double_colon(tokens: &[TokenTree]) -> bool {
    matches!(
        (tokens.first(), tokens.get(1)),
        (Some(TokenTree::Punct(first)), Some(TokenTree::Punct(second)))
            if first.as_char() == ':'
                && first.spacing() == Spacing::Joint
                && second.as_char() == ':'
    )
}

fn top_level_fat_arrow_index(tokens: &[TokenTree]) -> Option<usize> {
    tokens.windows(2).position(|window| {
        matches!(
            (&window[0], &window[1]),
            (TokenTree::Punct(first), TokenTree::Punct(second))
                if first.as_char() == '='
                    && first.spacing() == Spacing::Joint
                    && second.as_char() == '>'
        )
    })
}

#[derive(Default)]
struct EnumTypeInvariants {
    type_contracts: Vec<Contract>,
    variant_arms: Vec<EnumVariantInvariant>,
    errors: Vec<TokenStream>,
}

struct EnumVariantInvariant {
    mode: ContractMode,
    variant_ident: Ident,
    tail: TokenStream,
    expr: Expr,
    display: TokenStream,
}

fn generic_arguments(generics: &Generics) -> Vec<TokenStream> {
    generics
        .params
        .iter()
        .map(|param| match param {
            GenericParam::Lifetime(param) => param.lifetime.to_token_stream(),
            GenericParam::Type(param) => param.ident.to_token_stream(),
            GenericParam::Const(param) => param.ident.to_token_stream(),
        })
        .collect()
}

fn generics_with_state_params(generics: &Generics, state_idents: &[Ident]) -> Generics {
    let mut generics = generics.clone();
    for state_ident in state_idents {
        generics.params.push(syn::parse_quote!(#state_ident));
    }
    generics
}

fn type_with_args(ident: &Ident, args: Vec<TokenStream>) -> TokenStream {
    if args.is_empty() {
        quote! { #ident }
    } else {
        quote! { #ident < #(#args),* > }
    }
}

fn snake_to_upper_camel(name: &str) -> String {
    name.split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().chain(chars).collect::<String>(),
                None => String::new(),
            }
        })
        .collect()
}

fn type_mentions_type_param(field_type: &Type, type_params: &BTreeSet<String>) -> bool {
    struct TypeParamVisitor<'params> {
        type_params: &'params BTreeSet<String>,
        found: bool,
    }

    impl<'ast> Visit<'ast> for TypeParamVisitor<'_> {
        fn visit_type_path(&mut self, type_path: &'ast TypePath) {
            if type_path.qself.is_none()
                && type_path
                    .path
                    .segments
                    .first()
                    .is_some_and(|segment| self.type_params.contains(&segment.ident.to_string()))
            {
                self.found = true;
                return;
            }
            visit::visit_type_path(self, type_path);
        }
    }

    let mut visitor = TypeParamVisitor {
        type_params,
        found: false,
    };
    visitor.visit_type(field_type);
    visitor.found
}

struct TypeShape {
    wrapper_ident: Ident,
    data_ident: Ident,
    error_ident: Ident,
    builder_ident: Ident,
    wrapper_vis: Visibility,
    data_vis: Visibility,
    generics: Generics,
    derive_traits: Vec<Ident>,
    derives_debug: bool,
    derives_serialize: bool,
    derives_deserialize: bool,
    derives_default: bool,
    docs: Vec<Attribute>,
}

impl TypeShape {
    fn new(ident: &Ident, vis: &Visibility, generics: &Generics, attrs: &[Attribute]) -> Self {
        let mut derive_traits = Vec::new();
        let mut derives_debug = false;
        let mut derives_serialize = false;
        let mut derives_deserialize = false;
        let mut derives_default = false;
        let mut docs = Vec::new();

        for attr in attrs {
            if attr.path().is_ident("doc") {
                docs.push(attr.clone());
            }
            if !attr.path().is_ident("derive") {
                continue;
            }
            let _ = attr.parse_nested_meta(|meta| {
                if let Some(ident) = meta.path.get_ident() {
                    match ident.to_string().as_str() {
                        "Debug" => derives_debug = true,
                        "Serialize" => derives_serialize = true,
                        "Deserialize" => derives_deserialize = true,
                        "Default" => derives_default = true,
                        _ => derive_traits.push(ident.clone()),
                    }
                }
                Ok(())
            });
        }

        Self {
            wrapper_ident: ident.clone(),
            data_ident: format_ident!("{ident}Data"),
            error_ident: format_ident!("{ident}InvariantError"),
            builder_ident: format_ident!("{ident}DataBuilder"),
            wrapper_vis: vis.clone(),
            data_vis: vis.clone(),
            generics: generics.clone(),
            derive_traits,
            derives_debug,
            derives_serialize,
            derives_deserialize,
            derives_default,
            docs,
        }
    }

    fn wrapper_attrs(&self) -> Vec<Attribute> {
        let mut attrs = self.docs.clone();
        if !self.derive_traits.is_empty() {
            let traits = &self.derive_traits;
            attrs.push(syn::parse_quote!(#[derive(#(#traits),*)]));
        }
        attrs
    }

    fn struct_debug_impl(&self, field_idents: &[Ident], field_types: &[syn::Type]) -> TokenStream {
        if !self.derives_debug {
            return TokenStream::new();
        }
        let wrapper_ident = &self.wrapper_ident;
        let mut generics = self.generics.clone();
        let (_, ty_generics, _) = self.generics.split_for_impl();
        let where_clause = generics.make_where_clause();
        for field_type in self.generic_debug_bounds(field_types.iter()) {
            where_clause
                .predicates
                .push(syn::parse_quote!(#field_type: ::std::fmt::Debug));
        }
        let (impl_generics, _, where_clause) = generics.split_for_impl();
        quote! {
            impl #impl_generics ::std::fmt::Debug for #wrapper_ident #ty_generics #where_clause {
                fn fmt(&self, formatter: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                    let data = self.as_data();
                    let mut debug = formatter.debug_struct(::std::stringify!(#wrapper_ident));
                    #(
                        debug.field(::std::stringify!(#field_idents), &data.#field_idents);
                    )*
                    debug.finish()
                }
            }
        }
    }

    fn enum_debug_impl<'variant>(
        &self,
        variants: impl Iterator<Item = &'variant Variant>,
    ) -> TokenStream {
        if !self.derives_debug {
            return TokenStream::new();
        }
        let wrapper_ident = &self.wrapper_ident;
        let data_ident = &self.data_ident;
        let mut generics = self.generics.clone();
        let (_, ty_generics, _) = self.generics.split_for_impl();
        let mut arms = Vec::new();
        let mut field_types = Vec::new();
        for variant in variants {
            let variant_ident = &variant.ident;
            match &variant.fields {
                Fields::Unit => {
                    arms.push(quote! {
                        #data_ident::#variant_ident => formatter.write_str(::std::stringify!(#variant_ident))
                    });
                }
                Fields::Unnamed(fields) => {
                    let binding_idents = (0..fields.unnamed.len())
                        .map(|index| format_ident!("field_{index}"))
                        .collect::<Vec<_>>();
                    field_types.extend(fields.unnamed.iter().map(|field| field.ty.clone()));
                    arms.push(quote! {
                        #data_ident::#variant_ident(#(#binding_idents,)*) => {
                            let mut debug = formatter.debug_tuple(::std::stringify!(#variant_ident));
                            #(
                                debug.field(#binding_idents);
                            )*
                            debug.finish()
                        }
                    });
                }
                Fields::Named(fields) => {
                    let field_idents = fields
                        .named
                        .iter()
                        .map(|field| field.ident.clone().expect("named field"))
                        .collect::<Vec<_>>();
                    field_types.extend(fields.named.iter().map(|field| field.ty.clone()));
                    arms.push(quote! {
                        #data_ident::#variant_ident { #(#field_idents,)* } => {
                            let mut debug = formatter.debug_struct(::std::stringify!(#variant_ident));
                            #(
                                debug.field(::std::stringify!(#field_idents), #field_idents);
                            )*
                            debug.finish()
                        }
                    });
                }
            }
        }
        let where_clause = generics.make_where_clause();
        for field_type in self.generic_debug_bounds(field_types.iter()) {
            where_clause
                .predicates
                .push(syn::parse_quote!(#field_type: ::std::fmt::Debug));
        }
        let (impl_generics, _, where_clause) = generics.split_for_impl();
        quote! {
            impl #impl_generics ::std::fmt::Debug for #wrapper_ident #ty_generics #where_clause {
                fn fmt(&self, formatter: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                    match self.as_data() {
                        #(#arms,)*
                    }
                }
            }
        }
    }

    fn generic_debug_bounds<'ty>(&self, field_types: impl Iterator<Item = &'ty Type>) -> Vec<Type> {
        let type_params = self
            .generics
            .params
            .iter()
            .filter_map(|param| match param {
                GenericParam::Type(param) => Some(param.ident.to_string()),
                GenericParam::Lifetime(_) | GenericParam::Const(_) => None,
            })
            .collect::<BTreeSet<_>>();
        field_types
            .filter(|field_type| type_mentions_type_param(field_type, &type_params))
            .cloned()
            .collect()
    }

    fn serialize_impl(&self) -> TokenStream {
        if !self.derives_serialize {
            return TokenStream::new();
        }
        let wrapper_ident = &self.wrapper_ident;
        let data_ident = &self.data_ident;
        let mut generics = self.generics.clone();
        let (_, ty_generics, _) = self.generics.split_for_impl();
        generics
            .make_where_clause()
            .predicates
            .push(syn::parse_quote!(#data_ident #ty_generics: serde::Serialize));
        let (impl_generics, _, where_clause) = generics.split_for_impl();
        quote! {
            impl #impl_generics serde::Serialize for #wrapper_ident #ty_generics #where_clause
            {
                fn serialize<S>(&self, serializer: S) -> ::std::result::Result<S::Ok, S::Error>
                where
                    S: serde::Serializer,
                {
                    self.as_data().serialize(serializer)
                }
            }
        }
    }

    fn deserialize_impl(&self) -> TokenStream {
        if !self.derives_deserialize {
            return TokenStream::new();
        }
        let wrapper_ident = &self.wrapper_ident;
        let data_ident = &self.data_ident;
        let mut impl_generics_source = self.generics.clone();
        impl_generics_source
            .params
            .insert(0, syn::parse_quote!('de));
        let (_, ty_generics, _) = self.generics.split_for_impl();
        impl_generics_source
            .make_where_clause()
            .predicates
            .push(syn::parse_quote!(#data_ident #ty_generics: serde::Deserialize<'de>));
        let (impl_generics, _, where_clause) = impl_generics_source.split_for_impl();
        quote! {
            impl #impl_generics serde::Deserialize<'de> for #wrapper_ident #ty_generics #where_clause
            {
                fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
                where
                    D: serde::Deserializer<'de>,
                {
                    let data = #data_ident::deserialize(deserializer)?;
                    Self::try_from_data(data).map_err(serde::de::Error::custom)
                }
            }
        }
    }

    fn default_impl(&self) -> TokenStream {
        if !self.derives_default {
            return TokenStream::new();
        }
        let wrapper_ident = &self.wrapper_ident;
        let data_ident = &self.data_ident;
        let mut generics = self.generics.clone();
        let (_, ty_generics, _) = self.generics.split_for_impl();
        generics
            .make_where_clause()
            .predicates
            .push(syn::parse_quote!(#data_ident #ty_generics: ::std::default::Default));
        let (impl_generics, _, where_clause) = generics.split_for_impl();
        quote! {
            impl #impl_generics ::std::default::Default for #wrapper_ident #ty_generics #where_clause
            {
                fn default() -> Self {
                    Self::try_from_data(#data_ident::default())
                        .expect("default data value must satisfy type invariant")
                }
            }
        }
    }
}
