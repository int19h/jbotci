//! Proc-macros for syntax-specific source traversal.

extern crate proc_macro;

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    Attribute, Data, DeriveInput, Fields, GenericParam, Generics, Ident, parse_macro_input,
    parse_quote,
};

#[proc_macro_derive(SourceTree, attributes(source))]
pub fn derive_source_tree(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand_source_tree(input).into()
}

fn expand_source_tree(input: DeriveInput) -> proc_macro2::TokenStream {
    let name = input.ident;
    let generics = add_source_tree_bounds(input.generics);
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let body = match input.data {
        Data::Struct(data) => struct_body(&data.fields),
        Data::Enum(data) => {
            let arms = data.variants.iter().map(|variant| {
                let variant_name = &variant.ident;
                enum_arm(&name, variant_name, &variant.fields)
            });
            quote! {
                match self {
                    #(#arms)*
                }
            }
        }
        Data::Union(_) => {
            return syn::Error::new_spanned(name, "SourceTree cannot be derived for unions")
                .to_compile_error();
        }
    };

    quote! {
        #[::bityzba::contract_trait]
        impl #impl_generics ::jbotci_syntax::source_tree::SourceTree
            for #name #ty_generics #where_clause
        {
            fn visit_source_words<'source_tree>(
                &'source_tree self,
                visitor: &mut dyn FnMut(&'source_tree ::jbotci_syntax::WithIndicators<::jbotci_morphology::WordLike>),
            ) {
                #body
            }
        }
    }
}

fn add_source_tree_bounds(mut generics: Generics) -> Generics {
    for param in &mut generics.params {
        if let GenericParam::Type(type_param) = param {
            type_param
                .bounds
                .push(parse_quote!(::jbotci_syntax::source_tree::SourceTree));
        }
    }
    generics
}

fn struct_body(fields: &Fields) -> proc_macro2::TokenStream {
    let visits = fields.iter().enumerate().filter_map(|(index, field)| {
        if source_skip(&field.attrs) {
            return None;
        }
        let access = field
            .ident
            .as_ref()
            .map(|ident| quote!(&self.#ident))
            .unwrap_or_else(|| {
                let index = syn::Index::from(index);
                quote!(&self.#index)
            });
        Some(quote! {
            ::jbotci_syntax::source_tree::SourceTree::visit_source_words(#access, visitor);
        })
    });
    quote! {
        #(#visits)*
    }
}

fn enum_arm(enum_name: &Ident, variant_name: &Ident, fields: &Fields) -> proc_macro2::TokenStream {
    match fields {
        Fields::Named(fields) => {
            let bindings = fields
                .named
                .iter()
                .map(|field| field.ident.as_ref().unwrap());
            let visits = fields.named.iter().filter_map(|field| {
                if source_skip(&field.attrs) {
                    return None;
                }
                let ident = field.ident.as_ref().unwrap();
                Some(quote! {
                    ::jbotci_syntax::source_tree::SourceTree::visit_source_words(#ident, visitor);
                })
            });
            quote! {
                #enum_name::#variant_name { #(#bindings,)* } => {
                    #(#visits)*
                }
            }
        }
        Fields::Unnamed(fields) => {
            let bindings = (0..fields.unnamed.len())
                .map(|index| format_ident!("field_{index}"))
                .collect::<Vec<_>>();
            let visits = fields.unnamed.iter().enumerate().filter_map(|(index, field)| {
                if source_skip(&field.attrs) {
                    return None;
                }
                let ident = &bindings[index];
                Some(quote! {
                    ::jbotci_syntax::source_tree::SourceTree::visit_source_words(#ident, visitor);
                })
            });
            quote! {
                #enum_name::#variant_name(#(#bindings,)*) => {
                    #(#visits)*
                }
            }
        }
        Fields::Unit => quote! {
            #enum_name::#variant_name => {}
        },
    }
}

fn source_skip(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| {
        attr.path().is_ident("source")
            && attr
                .parse_args::<Ident>()
                .is_ok_and(|ident| ident == "skip")
    })
}
