//! Stack-guarded `Clone`, `PartialEq`, `Eq`, and `Debug` for tree nodes.
//!
//! A derived impl walks each child on the native stack. One `with_tree_stack`
//! frame at this node lets the child impl grow before the frame runs out.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    Attribute, Fields, Ident, Item, ItemEnum, ItemStruct, Path, Token, punctuated::Punctuated,
};

#[allow(unused_imports)]
use bityzba::{ensures, invariant, requires};

#[invariant(true)]
#[derive(Clone, Copy, Debug)]
struct SelectedTraits {
    clone: bool,
    partial_eq: bool,
    eq: bool,
    debug: bool,
}

#[requires(true)]
#[ensures(true)]
pub(super) fn emit_guarded_traits(items: &[Item], force_all: bool) -> TokenStream {
    let impls = items.iter().map(|item| emit_item(item, force_all));
    quote!(#(#impls)*)
}

#[requires(true)]
#[ensures(true)]
pub(super) fn strip_guarded_derives(item: &mut Item) {
    let attrs = match item {
        Item::Struct(item) => &mut item.attrs,
        Item::Enum(item) => &mut item.attrs,
        _ => return,
    };
    let mut kept = Vec::new();
    for attr in std::mem::take(attrs) {
        if let Some(stripped) = strip_derive_attr(&attr) {
            kept.push(stripped);
        }
    }
    *attrs = kept;
}

#[requires(true)]
#[ensures(true)]
fn emit_item(item: &Item, force_all: bool) -> TokenStream {
    match item {
        Item::Struct(item) => emit_struct(item, force_all),
        Item::Enum(item) => emit_enum(item, force_all),
        _ => TokenStream::new(),
    }
}

#[requires(true)]
#[ensures(true)]
fn emit_struct(item: &ItemStruct, force_all: bool) -> TokenStream {
    let selected = selected_traits(&item.attrs, force_all);
    if !selected.any() {
        return TokenStream::new();
    }
    let ident = &item.ident;
    let uses_data = super::item_needs_new_macro(&Item::Struct(item.clone()));
    let data_ident = format_ident!("{}Data", ident);
    let mut impls = TokenStream::new();
    if selected.clone {
        let body = struct_clone_body(ident, &data_ident, &item.fields, uses_data);
        impls.extend(clone_impl(ident, &body));
    }
    if selected.partial_eq {
        let body = struct_eq_body(&data_ident, &item.fields, uses_data);
        impls.extend(partial_eq_impl(ident, &body));
    }
    if selected.eq {
        impls.extend(eq_impl(ident));
    }
    if selected.debug {
        let body = struct_debug_body(ident, &data_ident, &item.fields, uses_data);
        impls.extend(debug_impl(ident, &body));
    }
    impls
}

#[requires(true)]
#[ensures(true)]
fn emit_enum(item: &ItemEnum, force_all: bool) -> TokenStream {
    let selected = selected_traits(&item.attrs, force_all);
    if !selected.any() {
        return TokenStream::new();
    }
    let ident = &item.ident;
    let uses_data = super::enum_uses_data_patterns(item);
    let data_ident = format_ident!("{}Data", ident);
    let mut impls = TokenStream::new();
    if selected.clone {
        let body = enum_clone_body(ident, &data_ident, item, uses_data);
        impls.extend(clone_impl(ident, &body));
    }
    if selected.partial_eq {
        let body = enum_eq_body(ident, &data_ident, item, uses_data);
        impls.extend(partial_eq_impl(ident, &body));
    }
    if selected.eq {
        impls.extend(eq_impl(ident));
    }
    if selected.debug {
        let body = enum_debug_body(ident, &data_ident, item, uses_data);
        impls.extend(debug_impl(ident, &body));
    }
    impls
}

#[requires(true)]
#[ensures(true)]
fn selected_traits(attrs: &[Attribute], force_all: bool) -> SelectedTraits {
    if force_all {
        return SelectedTraits {
            clone: true,
            partial_eq: true,
            eq: true,
            debug: true,
        };
    }
    let eq = derives_trait(attrs, "Eq");
    SelectedTraits {
        clone: derives_trait(attrs, "Clone"),
        partial_eq: derives_trait(attrs, "PartialEq") || eq,
        eq,
        debug: derives_trait(attrs, "Debug"),
    }
}

impl SelectedTraits {
    #[requires(true)]
    #[ensures(true)]
    fn any(self) -> bool {
        self.clone || self.partial_eq || self.eq || self.debug
    }
}

#[requires(true)]
#[ensures(true)]
fn derives_trait(attrs: &[Attribute], trait_name: &str) -> bool {
    attrs.iter().any(|attr| {
        derive_paths(attr)
            .is_some_and(|paths| paths.iter().any(|path| path_is_trait(path, trait_name)))
    })
}

#[requires(true)]
#[ensures(true)]
fn derive_paths(attr: &Attribute) -> Option<Vec<Path>> {
    if !attr.path().is_ident("derive") {
        return None;
    }
    attr.parse_args_with(Punctuated::<Path, Token![,]>::parse_terminated)
        .ok()
        .map(|paths| paths.into_iter().collect())
}

#[requires(!trait_name.is_empty())]
#[ensures(true)]
fn path_is_trait(path: &Path, trait_name: &str) -> bool {
    path.segments
        .last()
        .is_some_and(|segment| segment.ident == trait_name)
}

#[requires(true)]
#[ensures(true)]
fn strip_derive_attr(attr: &Attribute) -> Option<Attribute> {
    let Some(paths) = derive_paths(attr) else {
        return Some(attr.clone());
    };
    let kept: Vec<_> = paths
        .into_iter()
        .filter(|path| {
            !path_is_trait(path, "Clone")
                && !path_is_trait(path, "Debug")
                && !path_is_trait(path, "Eq")
                && !path_is_trait(path, "PartialEq")
        })
        .collect();
    if kept.is_empty() {
        return None;
    }
    Some(syn::parse_quote!(#[derive(#(#kept),*)]))
}

#[requires(true)]
#[ensures(true)]
fn clone_impl(ident: &Ident, body: &TokenStream) -> TokenStream {
    quote! {
        impl ::core::clone::Clone for #ident {
            fn clone(&self) -> Self {
                ::jbotci_tree::with_tree_stack(|| #body)
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn partial_eq_impl(ident: &Ident, body: &TokenStream) -> TokenStream {
    quote! {
        impl ::core::cmp::PartialEq for #ident {
            fn eq(&self, other: &Self) -> bool {
                ::jbotci_tree::with_tree_stack(|| #body)
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn eq_impl(ident: &Ident) -> TokenStream {
    quote! {
        impl ::core::cmp::Eq for #ident {}
    }
}

#[requires(true)]
#[ensures(true)]
fn debug_impl(ident: &Ident, body: &TokenStream) -> TokenStream {
    quote! {
        impl ::core::fmt::Debug for #ident {
            fn fmt(&self, formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                ::jbotci_tree::with_tree_stack(|| #body)
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn struct_clone_body(
    ident: &Ident,
    data_ident: &Ident,
    fields: &Fields,
    uses_data: bool,
) -> TokenStream {
    match fields {
        Fields::Unit => {
            if uses_data {
                quote!(#ident::from_data(#data_ident))
            } else {
                quote!(#ident)
            }
        }
        Fields::Unnamed(unnamed) if unnamed.unnamed.is_empty() => {
            if uses_data {
                quote!(#ident::from_data(#data_ident()))
            } else {
                quote!(#ident())
            }
        }
        Fields::Unnamed(unnamed) => {
            let clones = unnamed.unnamed.iter().enumerate().map(|(index, _)| {
                let index = syn::Index::from(index);
                if uses_data {
                    quote!(__data.#index.clone())
                } else {
                    quote!(self.#index.clone())
                }
            });
            if uses_data {
                quote! {
                    {
                        let __data = self.as_data();
                        #ident::from_data(#data_ident(#(#clones),*))
                    }
                }
            } else {
                quote!(#ident(#(#clones),*))
            }
        }
        Fields::Named(named) => {
            let clones = named.named.iter().map(|field| {
                let name = field.ident.as_ref().expect("named field");
                if uses_data {
                    quote!(#name: __data.#name.clone())
                } else {
                    quote!(#name: self.#name.clone())
                }
            });
            if uses_data {
                quote! {
                    {
                        let __data = self.as_data();
                        #ident::from_data(#data_ident { #(#clones,)* })
                    }
                }
            } else {
                quote!(#ident { #(#clones,)* })
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn struct_eq_body(data_ident: &Ident, fields: &Fields, uses_data: bool) -> TokenStream {
    let comparisons = match fields {
        Fields::Unit => Vec::new(),
        Fields::Unnamed(unnamed) if unnamed.unnamed.is_empty() => Vec::new(),
        Fields::Unnamed(unnamed) => unnamed
            .unnamed
            .iter()
            .enumerate()
            .map(|(index, _)| {
                let index = syn::Index::from(index);
                if uses_data {
                    quote!(__left.#index == __right.#index)
                } else {
                    quote!(self.#index == other.#index)
                }
            })
            .collect(),
        Fields::Named(named) => named
            .named
            .iter()
            .map(|field| {
                let name = field.ident.as_ref().expect("named field");
                if uses_data {
                    quote!(__left.#name == __right.#name)
                } else {
                    quote!(self.#name == other.#name)
                }
            })
            .collect(),
    };
    let chain = and_chain(comparisons);
    if uses_data {
        let _ = data_ident;
        quote! {
            {
                let __left = self.as_data();
                let __right = other.as_data();
                #chain
            }
        }
    } else {
        chain
    }
}

#[requires(true)]
#[ensures(true)]
fn struct_debug_body(
    ident: &Ident,
    data_ident: &Ident,
    fields: &Fields,
    uses_data: bool,
) -> TokenStream {
    let _ = data_ident;
    match fields {
        Fields::Unit => quote!(formatter.write_str(::core::stringify!(#ident))),
        Fields::Unnamed(unnamed) if unnamed.unnamed.is_empty() => {
            quote!(formatter.debug_tuple(::core::stringify!(#ident)).finish())
        }
        Fields::Unnamed(unnamed) => {
            let fields = unnamed.unnamed.iter().enumerate().map(|(index, _)| {
                let index = syn::Index::from(index);
                if uses_data {
                    quote!(.field(&__data.#index))
                } else {
                    quote!(.field(&self.#index))
                }
            });
            if uses_data {
                quote! {
                    {
                        let __data = self.as_data();
                        formatter.debug_tuple(::core::stringify!(#ident)) #(#fields)* .finish()
                    }
                }
            } else {
                quote!(formatter.debug_tuple(::core::stringify!(#ident)) #(#fields)* .finish())
            }
        }
        Fields::Named(named) if named.named.is_empty() => {
            quote!(formatter.debug_struct(::core::stringify!(#ident)).finish())
        }
        Fields::Named(named) => {
            let fields = named.named.iter().map(|field| {
                let name = field.ident.as_ref().expect("named field");
                let label = name.to_string();
                if uses_data {
                    quote!(.field(#label, &__data.#name))
                } else {
                    quote!(.field(#label, &self.#name))
                }
            });
            if uses_data {
                quote! {
                    {
                        let __data = self.as_data();
                        formatter.debug_struct(::core::stringify!(#ident)) #(#fields)* .finish()
                    }
                }
            } else {
                quote!(formatter.debug_struct(::core::stringify!(#ident)) #(#fields)* .finish())
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn enum_clone_body(
    ident: &Ident,
    data_ident: &Ident,
    item: &ItemEnum,
    uses_data: bool,
) -> TokenStream {
    if item.variants.is_empty() {
        let scrutinee = if uses_data {
            quote!(*self.as_data())
        } else {
            quote!(*self)
        };
        return quote!(match #scrutinee {});
    }
    let arms = item
        .variants
        .iter()
        .map(|variant| enum_clone_arm(ident, data_ident, variant, uses_data));
    let scrutinee = if uses_data {
        quote!(self.as_data())
    } else {
        quote!(self)
    };
    quote!(match #scrutinee { #(#arms,)* })
}

#[requires(true)]
#[ensures(true)]
fn enum_clone_arm(
    ident: &Ident,
    data_ident: &Ident,
    variant: &syn::Variant,
    uses_data: bool,
) -> TokenStream {
    let variant_ident = &variant.ident;
    let pattern_ty = if uses_data { data_ident } else { ident };
    match &variant.fields {
        Fields::Unit => {
            if uses_data {
                quote!(#data_ident::#variant_ident => #ident::from_data(#data_ident::#variant_ident))
            } else {
                quote!(#ident::#variant_ident => #ident::#variant_ident)
            }
        }
        Fields::Unnamed(fields) => {
            let binds: Vec<_> = (0..fields.unnamed.len())
                .map(|index| format_ident!("__field_{index}"))
                .collect();
            let clones = binds.iter().map(|bind| quote!(#bind.clone()));
            if uses_data {
                quote! {
                    #data_ident::#variant_ident(#(#binds),*) => {
                        #ident::from_data(#data_ident::#variant_ident(#(#clones),*))
                    }
                }
            } else {
                quote!(#pattern_ty::#variant_ident(#(#binds),*) => #ident::#variant_ident(#(#clones),*))
            }
        }
        Fields::Named(fields) => {
            let names: Vec<_> = fields
                .named
                .iter()
                .map(|field| field.ident.clone().expect("named field"))
                .collect();
            let pats = names.iter().map(|name| quote!(#name));
            let clones = names.iter().map(|name| quote!(#name: #name.clone()));
            if uses_data {
                quote! {
                    #data_ident::#variant_ident { #(#pats,)* } => {
                        #ident::from_data(#data_ident::#variant_ident { #(#clones,)* })
                    }
                }
            } else {
                quote!(#ident::#variant_ident { #(#pats,)* } => #ident::#variant_ident { #(#clones,)* })
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn enum_eq_body(
    ident: &Ident,
    data_ident: &Ident,
    item: &ItemEnum,
    uses_data: bool,
) -> TokenStream {
    if item.variants.is_empty() {
        let scrutinee = if uses_data {
            quote!(*self.as_data())
        } else {
            quote!(*self)
        };
        return quote!(match #scrutinee {});
    }
    let pattern_ty: &Ident = if uses_data { data_ident } else { ident };
    let arms = item
        .variants
        .iter()
        .map(|variant| enum_eq_arm(pattern_ty, variant));
    let wildcard = (item.variants.len() > 1).then(|| quote!(_ => false));
    let scrutinee = if uses_data {
        quote!((self.as_data(), other.as_data()))
    } else {
        quote!((self, other))
    };
    quote!(match #scrutinee { #(#arms,)* #wildcard })
}

#[requires(true)]
#[ensures(true)]
fn enum_eq_arm(pattern_ty: &Ident, variant: &syn::Variant) -> TokenStream {
    let variant_ident = &variant.ident;
    match &variant.fields {
        Fields::Unit => {
            quote! {
                (#pattern_ty::#variant_ident, #pattern_ty::#variant_ident) => true
            }
        }
        Fields::Unnamed(fields) => {
            let left: Vec<_> = (0..fields.unnamed.len())
                .map(|index| format_ident!("__left_{index}"))
                .collect();
            let right: Vec<_> = (0..fields.unnamed.len())
                .map(|index| format_ident!("__right_{index}"))
                .collect();
            let comparisons = left
                .iter()
                .zip(&right)
                .map(|(left, right)| quote!(#left == #right))
                .collect();
            let body = and_chain(comparisons);
            quote! {
                (
                    #pattern_ty::#variant_ident(#(#left),*),
                    #pattern_ty::#variant_ident(#(#right),*),
                ) => #body
            }
        }
        Fields::Named(fields) => {
            let names: Vec<_> = fields
                .named
                .iter()
                .map(|field| field.ident.clone().expect("named field"))
                .collect();
            let left_pats = names.iter().map(|name| {
                let bind = format_ident!("__left_{name}");
                quote!(#name: #bind)
            });
            let right_pats = names.iter().map(|name| {
                let bind = format_ident!("__right_{name}");
                quote!(#name: #bind)
            });
            let comparisons = names
                .iter()
                .map(|name| {
                    let left = format_ident!("__left_{name}");
                    let right = format_ident!("__right_{name}");
                    quote!(#left == #right)
                })
                .collect();
            let body = and_chain(comparisons);
            quote! {
                (
                    #pattern_ty::#variant_ident { #(#left_pats,)* },
                    #pattern_ty::#variant_ident { #(#right_pats,)* },
                ) => #body
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn enum_debug_body(
    ident: &Ident,
    data_ident: &Ident,
    item: &ItemEnum,
    uses_data: bool,
) -> TokenStream {
    if item.variants.is_empty() {
        let scrutinee = if uses_data {
            quote!(*self.as_data())
        } else {
            quote!(*self)
        };
        return quote!(match #scrutinee {});
    }
    let arms = item
        .variants
        .iter()
        .map(|variant| enum_debug_arm(ident, data_ident, variant, uses_data));
    let scrutinee = if uses_data {
        quote!(self.as_data())
    } else {
        quote!(self)
    };
    quote!(match #scrutinee { #(#arms,)* })
}

#[requires(true)]
#[ensures(true)]
fn enum_debug_arm(
    ident: &Ident,
    data_ident: &Ident,
    variant: &syn::Variant,
    uses_data: bool,
) -> TokenStream {
    let variant_ident = &variant.ident;
    let pattern_ty = if uses_data { data_ident } else { ident };
    let label = if uses_data {
        quote!(::core::stringify!(#variant_ident))
    } else {
        quote!(::core::concat!(
            ::core::stringify!(#ident),
            "::",
            ::core::stringify!(#variant_ident)
        ))
    };
    match &variant.fields {
        Fields::Unit => {
            quote!(#pattern_ty::#variant_ident => formatter.write_str(#label))
        }
        Fields::Unnamed(fields) => {
            let binds: Vec<_> = (0..fields.unnamed.len())
                .map(|index| format_ident!("__field_{index}"))
                .collect();
            let debug_fields = binds.iter().map(|bind| quote!(.field(#bind)));
            quote! {
                #pattern_ty::#variant_ident(#(#binds),*) => {
                    formatter.debug_tuple(#label) #(#debug_fields)* .finish()
                }
            }
        }
        Fields::Named(fields) => {
            let names: Vec<_> = fields
                .named
                .iter()
                .map(|field| field.ident.clone().expect("named field"))
                .collect();
            let pats = names.iter().map(|name| quote!(#name));
            let debug_fields = names.iter().map(|name| {
                let label = name.to_string();
                quote!(.field(#label, #name))
            });
            quote! {
                #pattern_ty::#variant_ident { #(#pats,)* } => {
                    formatter.debug_struct(#label) #(#debug_fields)* .finish()
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn and_chain(parts: Vec<TokenStream>) -> TokenStream {
    let mut parts = parts.into_iter();
    let Some(first) = parts.next() else {
        return quote!(true);
    };
    parts.fold(first, |left, right| quote!((#left) && (#right)))
}
