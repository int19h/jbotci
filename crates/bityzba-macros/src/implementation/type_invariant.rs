/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use proc_macro2::TokenStream;
use quote::{ToTokens, format_ident, quote};
use syn::{
    Attribute, Expr, ExprLit, Fields, FieldsNamed, GenericParam, Generics, Ident, ItemEnum,
    ItemStruct, Lit, Visibility,
};

use crate::implementation::{Contract, ContractMode, ContractType};

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
    let contracts = collect_type_invariants(mode, attr, &mut item.attrs);
    let option_errors = collect_type_option_errors(&mut item.attrs);
    if contracts_are_true_marker(&contracts) {
        return quote! {
            #(#option_errors)*
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
    let serialize_impl = shape.serialize_impl();
    let deserialize_impl = shape.deserialize_impl();
    let default_impl = shape.default_impl();
    let invariant_expr = invariant_expression(&contracts);
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

        #serialize_impl
        #deserialize_impl
        #default_impl
    }
}

fn generate_enum(
    contracts: Vec<Contract>,
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
    let serialize_impl = shape.serialize_impl();
    let deserialize_impl = shape.deserialize_impl();
    let default_impl = shape.default_impl();
    let invariant_expr = invariant_expression(&contracts);
    let invariant_docs = invariant_docs(&contracts);
    let generics = &item.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let turbofish = ty_generics.as_turbofish();

    quote! {
        #(#option_errors)*

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

fn invariant_expression(contracts: &[Contract]) -> TokenStream {
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
    quote! { true #(&& #checks)* }
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

fn contracts_are_true_marker(contracts: &[Contract]) -> bool {
    contracts.iter().all(|contract| {
        !contract.assertions.is_empty() && contract.assertions.iter().all(is_true_literal)
    })
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

struct TypeShape {
    wrapper_ident: Ident,
    data_ident: Ident,
    error_ident: Ident,
    builder_ident: Ident,
    wrapper_vis: Visibility,
    data_vis: Visibility,
    generics: Generics,
    derive_traits: Vec<Ident>,
    derives_serialize: bool,
    derives_deserialize: bool,
    derives_default: bool,
    docs: Vec<Attribute>,
}

impl TypeShape {
    fn new(ident: &Ident, vis: &Visibility, generics: &Generics, attrs: &[Attribute]) -> Self {
        let mut derive_traits = Vec::new();
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
