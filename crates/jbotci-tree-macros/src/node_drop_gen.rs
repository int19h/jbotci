//! Generate heap-queue drops for `tree_model!` nodes.

use std::collections::{BTreeMap, BTreeSet};

#[allow(unused_imports)]
use bityzba::{ensures, invariant, requires};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Fields, GenericArgument, Ident, Item, ItemEnum, ItemStruct, PathArguments, Type};

#[invariant(true)]
struct Release {
    code: TokenStream,
    releases: bool,
}

#[requires(true)]
#[ensures(true)]
fn empty_release() -> Release {
    Release {
        code: TokenStream::new(),
        releases: false,
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn node_drop_impls(
    items: &[Item],
    node_names: &BTreeSet<String>,
    aliases: &BTreeMap<String, Type>,
) -> TokenStream {
    let impls = items.iter().filter_map(|item| match item {
        Item::Struct(item) => Some(struct_drop(item, node_names, aliases)),
        Item::Enum(item) => Some(enum_drop(item, node_names, aliases)),
        Item::Type(_) => None,
        _ => None,
    });
    quote!(#(#impls)*)
}

#[requires(true)]
#[ensures(true)]
fn struct_drop(
    item: &ItemStruct,
    node_names: &BTreeSet<String>,
    aliases: &BTreeMap<String, Type>,
) -> TokenStream {
    // A non-trivial invariant wraps the struct. Field reads belong on the
    // inner data type. This generator leaves that wrapper on the normal drop.
    if super::item_needs_new_macro(&Item::Struct(item.clone())) {
        return TokenStream::new();
    }
    let body = fields_release(&item.fields, quote!(self), node_names, aliases);
    if !body.releases {
        return TokenStream::new();
    }
    drop_impl(&item.ident, &body.code)
}

#[requires(true)]
#[ensures(true)]
fn enum_drop(
    item: &ItemEnum,
    node_names: &BTreeSet<String>,
    aliases: &BTreeMap<String, Type>,
) -> TokenStream {
    // A non-trivial invariant wraps the enum. Variant reads belong on the
    // inner data type. This generator leaves that wrapper on the normal drop.
    if super::enum_uses_data_patterns(item) {
        return TokenStream::new();
    }
    let mut releases = false;
    let arms = item.variants.iter().map(|variant| {
        let name = &variant.ident;
        let arm = match &variant.fields {
            Fields::Unit => quote!(Self::#name => {}),
            Fields::Named(fields) => {
                let bindings = fields
                    .named
                    .iter()
                    .map(|field| field.ident.clone().expect("named field"))
                    .collect::<Vec<_>>();
                let parts = fields.named.iter().zip(&bindings).map(|(field, binding)| {
                    release_type(&field.ty, quote!(#binding), node_names, aliases)
                });
                let mut code = TokenStream::new();
                for part in parts {
                    releases |= part.releases;
                    code.extend(part.code);
                }
                quote!(Self::#name { #(#bindings,)* } => { #code })
            }
            Fields::Unnamed(fields) => {
                let bindings = (0..fields.unnamed.len())
                    .map(|index| format_ident!("__field_{index}"))
                    .collect::<Vec<_>>();
                let parts = fields
                    .unnamed
                    .iter()
                    .zip(&bindings)
                    .map(|(field, binding)| {
                        release_type(&field.ty, quote!(#binding), node_names, aliases)
                    });
                let mut code = TokenStream::new();
                for part in parts {
                    releases |= part.releases;
                    code.extend(part.code);
                }
                quote!(Self::#name(#(#bindings),*) => { #code })
            }
        };
        arm
    });
    let body = quote!(match self { #(#arms)* });
    if !releases {
        return TokenStream::new();
    }
    drop_impl(&item.ident, &body)
}

#[requires(true)]
#[ensures(true)]
fn drop_impl(ident: &Ident, body: &TokenStream) -> TokenStream {
    quote! {
        impl #ident {
            fn __jbotci_release_owned_children(&mut self) {
                #body
            }
        }

        impl Drop for #ident {
            fn drop(&mut self) {
                ::jbotci_tree::node_drop::drop_tree_node(
                    self,
                    Self::__jbotci_release_owned_children,
                );
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn fields_release(
    fields: &Fields,
    receiver: TokenStream,
    node_names: &BTreeSet<String>,
    aliases: &BTreeMap<String, Type>,
) -> Release {
    let mut code = TokenStream::new();
    let mut releases = false;
    for (index, field) in fields.iter().enumerate() {
        let place = match &field.ident {
            Some(name) => quote!((&mut #receiver.#name)),
            None => {
                let index = syn::Index::from(index);
                quote!((&mut #receiver.#index))
            }
        };
        let part = release_type(&field.ty, place, node_names, aliases);
        releases |= part.releases;
        code.extend(part.code);
    }
    Release { code, releases }
}

#[requires(true)]
#[ensures(true)]
fn release_type(
    ty: &Type,
    place: TokenStream,
    node_names: &BTreeSet<String>,
    aliases: &BTreeMap<String, Type>,
) -> Release {
    let mut seen = BTreeSet::new();
    release_type_seen(ty, place, node_names, aliases, &mut seen)
}

#[requires(true)]
#[ensures(true)]
fn release_type_seen(
    ty: &Type,
    place: TokenStream,
    node_names: &BTreeSet<String>,
    aliases: &BTreeMap<String, Type>,
    seen_aliases: &mut BTreeSet<String>,
) -> Release {
    match ty {
        Type::Paren(paren) => {
            release_type_seen(&paren.elem, place, node_names, aliases, seen_aliases)
        }
        Type::Group(group) => {
            release_type_seen(&group.elem, place, node_names, aliases, seen_aliases)
        }
        Type::Array(array) => release_sequence(
            Some(&array.elem),
            false,
            place,
            node_names,
            aliases,
            seen_aliases,
        ),
        Type::Tuple(tuple) => release_projected(
            place,
            tuple
                .elems
                .iter()
                .enumerate()
                .map(|(index, elem)| {
                    let index = syn::Index::from(index);
                    (quote!(#index), elem)
                })
                .collect(),
            node_names,
            aliases,
            seen_aliases,
        ),
        Type::Path(path) => release_path(path, place, node_names, aliases, seen_aliases),
        _ => empty_release(),
    }
}

#[requires(true)]
#[ensures(true)]
fn release_path(
    path: &syn::TypePath,
    place: TokenStream,
    node_names: &BTreeSet<String>,
    aliases: &BTreeMap<String, Type>,
    seen_aliases: &mut BTreeSet<String>,
) -> Release {
    if path.qself.is_some() {
        return empty_release();
    }
    let Some(segment) = path.path.segments.last() else {
        return empty_release();
    };
    let args = type_arguments(&segment.arguments);
    if path.path.segments.len() == 1 && args.is_empty() {
        let name = segment.ident.to_string();
        if aliases.contains_key(&name) {
            if !seen_aliases.insert(name.clone()) {
                return empty_release();
            }
            let part = release_type_seen(&aliases[&name], place, node_names, aliases, seen_aliases);
            seen_aliases.remove(&name);
            return part;
        }
    }
    let name = segment.ident.to_string();
    if path.path.segments.len() == 1 && node_names.contains(&name) && args.is_empty() {
        // The child node drops itself. A by-value field has one extra frame.
        return empty_release();
    }
    match name.as_str() {
        "Arc" => release_arc_type(
            args.first().copied(),
            place,
            node_names,
            aliases,
            seen_aliases,
        ),
        "Option" => release_option(
            args.first().copied(),
            place,
            node_names,
            aliases,
            seen_aliases,
        ),
        "Vec" | "SmallVec" => release_sequence(
            sequence_elem(name.as_str(), &args),
            true,
            place,
            node_names,
            aliases,
            seen_aliases,
        ),
        "Vec1" | "SmallVec1" => release_sequence(
            sequence_elem(name.as_str(), &args),
            false,
            place,
            node_names,
            aliases,
            seen_aliases,
        ),
        "Box" => {
            let Some(inner) = args.first().copied() else {
                return empty_release();
            };
            if is_node(inner, node_names, aliases) {
                // A required `Box<Node>` has no empty stand-in. Its drop stays
                // one frame deeper than the node drop. `Option<Box<Node>>` and
                // `Vec<Box<Node>>` move the node out before that drop.
                return empty_release();
            }
            release_type_seen(
                inner,
                quote!(#place.as_mut()),
                node_names,
                aliases,
                seen_aliases,
            )
        }
        "WithFreeModifiers" => {
            release_with_free_modifiers(&args, place, node_names, aliases, seen_aliases)
        }
        "Chain" => {
            let mut fields = Vec::new();
            if let Some(first) = args.first().copied() {
                fields.push((quote!(first), first));
            }
            if let Some(links) = args.get(1).copied() {
                fields.push((quote!(links), links));
            }
            release_projected(place, fields, node_names, aliases, seen_aliases)
        }
        "Recovered" => release_recovered(
            args.first().copied(),
            place,
            node_names,
            aliases,
            seen_aliases,
        ),
        _ => empty_release(),
    }
}

#[requires(true)]
#[ensures(true)]
fn release_arc_type(
    inner: Option<&Type>,
    place: TokenStream,
    node_names: &BTreeSet<String>,
    aliases: &BTreeMap<String, Type>,
    seen_aliases: &mut BTreeSet<String>,
) -> Release {
    let Some(inner) = inner else {
        return empty_release();
    };
    if is_node(inner, node_names, aliases) {
        return Release {
            code: quote!(::jbotci_tree::node_drop::release_arc(#place);),
            releases: true,
        };
    }
    if recovered_node(inner, node_names, aliases).is_some() {
        return Release {
            code: quote!(::jbotci_tree::node_drop::release_arc_recovered(#place);),
            releases: true,
        };
    }
    if !contains_node(inner, node_names, aliases) {
        return empty_release();
    }
    let inner_release = release_type_seen(
        inner,
        quote!(__jbotci_arc_value),
        node_names,
        aliases,
        seen_aliases,
    );
    if !inner_release.releases {
        return empty_release();
    }
    let inner_code = inner_release.code;
    Release {
        code: quote! {
            ::jbotci_tree::node_drop::with_unique_arc(#place, |__jbotci_arc_value| {
                #inner_code
            });
        },
        releases: true,
    }
}

#[requires(true)]
#[ensures(true)]
fn release_option(
    inner: Option<&Type>,
    place: TokenStream,
    node_names: &BTreeSet<String>,
    aliases: &BTreeMap<String, Type>,
    seen_aliases: &mut BTreeSet<String>,
) -> Release {
    let Some(inner) = inner else {
        return empty_release();
    };
    if is_node(inner, node_names, aliases) {
        return Release {
            code: quote! {
                if let Some(__jbotci_node) = #place.take() {
                    ::jbotci_tree::node_drop::enqueue_node(__jbotci_node);
                }
            },
            releases: true,
        };
    }
    if let Some(boxed) = box_of_node(inner, node_names, aliases) {
        let _ = boxed;
        return Release {
            code: quote! {
                if let Some(__jbotci_boxed) = #place.take() {
                    ::jbotci_tree::node_drop::enqueue_node(*__jbotci_boxed);
                }
            },
            releases: true,
        };
    }
    if recovered_node(inner, node_names, aliases).is_some() {
        return Release {
            code: quote! {
                if let Some(__jbotci_recovered) = #place.take() {
                    ::jbotci_tree::node_drop::enqueue_taken_recovered(__jbotci_recovered);
                }
            },
            releases: true,
        };
    }
    if box_of_recovered_node(inner, node_names, aliases).is_some() {
        return Release {
            code: quote! {
                if let Some(__jbotci_boxed) = #place.take() {
                    ::jbotci_tree::node_drop::enqueue_taken_recovered(*__jbotci_boxed);
                }
            },
            releases: true,
        };
    }
    if recovered_of_boxed_node(inner, node_names, aliases).is_some() {
        return Release {
            code: quote! {
                if let Some(__jbotci_recovered) = #place.take() {
                    ::jbotci_tree::node_drop::enqueue_taken_boxed_recovered(__jbotci_recovered);
                }
            },
            releases: true,
        };
    }
    let inner_release = release_type_seen(
        inner,
        quote!(__jbotci_item),
        node_names,
        aliases,
        seen_aliases,
    );
    if !inner_release.releases {
        return empty_release();
    }
    let inner_code = inner_release.code;
    Release {
        code: quote! {
            if let Some(__jbotci_item) = #place.as_mut() {
                #inner_code
            }
        },
        releases: true,
    }
}

#[requires(true)]
#[ensures(true)]
fn release_projected(
    place: TokenStream,
    fields: Vec<(TokenStream, &Type)>,
    node_names: &BTreeSet<String>,
    aliases: &BTreeMap<String, Type>,
    seen_aliases: &mut BTreeSet<String>,
) -> Release {
    let mut code = TokenStream::new();
    let mut releases = false;
    for (field, ty) in fields {
        let part = release_type_seen(
            ty,
            quote!((&mut __jbotci_base.#field)),
            node_names,
            aliases,
            seen_aliases,
        );
        releases |= part.releases;
        code.extend(part.code);
    }
    if !releases {
        return empty_release();
    }
    Release {
        code: quote! {
            {
                let __jbotci_base = #place;
                #code
            }
        },
        releases: true,
    }
}

#[requires(true)]
#[ensures(true)]
fn release_with_free_modifiers(
    args: &[&Type],
    place: TokenStream,
    node_names: &BTreeSet<String>,
    aliases: &BTreeMap<String, Type>,
    seen_aliases: &mut BTreeSet<String>,
) -> Release {
    let mut code = TokenStream::new();
    let mut releases = false;
    if let Some(value) = args.first().copied() {
        let part = release_type_seen(
            value,
            quote!((&mut __jbotci_base.value)),
            node_names,
            aliases,
            seen_aliases,
        );
        releases |= part.releases;
        code.extend(part.code);
    }
    // `free_modifiers` stores `Vec<F>`. `F` is the second type argument.
    // A missing argument uses the recovered default element.
    let modifiers: Type = match args.get(1).copied() {
        Some(modifiers) => syn::parse_quote!(Vec<#modifiers>),
        None => syn::parse_quote!(Vec<Recovered<FreeModifierSyntax>>),
    };
    let part = release_type_seen(
        &modifiers,
        quote!((&mut __jbotci_base.free_modifiers)),
        node_names,
        aliases,
        seen_aliases,
    );
    releases |= part.releases;
    code.extend(part.code);
    if !releases {
        return empty_release();
    }
    Release {
        code: quote! {
            {
                let __jbotci_base = #place;
                #code
            }
        },
        releases: true,
    }
}

#[requires(true)]
#[ensures(true)]
fn release_sequence(
    elem: Option<&Type>,
    can_take: bool,
    place: TokenStream,
    node_names: &BTreeSet<String>,
    aliases: &BTreeMap<String, Type>,
    seen_aliases: &mut BTreeSet<String>,
) -> Release {
    let Some(elem) = elem else {
        return empty_release();
    };
    if can_take && is_node(elem, node_names, aliases) {
        return Release {
            code: quote! {
                for __jbotci_node in ::std::mem::take(#place) {
                    ::jbotci_tree::node_drop::enqueue_node(__jbotci_node);
                }
            },
            releases: true,
        };
    }
    if can_take && box_of_node(elem, node_names, aliases).is_some() {
        return Release {
            code: quote! {
                for __jbotci_boxed in ::std::mem::take(#place) {
                    ::jbotci_tree::node_drop::enqueue_node(*__jbotci_boxed);
                }
            },
            releases: true,
        };
    }
    if can_take && recovered_node(elem, node_names, aliases).is_some() {
        return Release {
            code: quote! {
                for __jbotci_recovered in ::std::mem::take(#place) {
                    ::jbotci_tree::node_drop::enqueue_taken_recovered(__jbotci_recovered);
                }
            },
            releases: true,
        };
    }
    if can_take && box_of_recovered_node(elem, node_names, aliases).is_some() {
        return Release {
            code: quote! {
                for __jbotci_boxed in ::std::mem::take(#place) {
                    ::jbotci_tree::node_drop::enqueue_taken_recovered(*__jbotci_boxed);
                }
            },
            releases: true,
        };
    }
    if can_take && recovered_of_boxed_node(elem, node_names, aliases).is_some() {
        return Release {
            code: quote! {
                for __jbotci_recovered in ::std::mem::take(#place) {
                    ::jbotci_tree::node_drop::enqueue_taken_boxed_recovered(__jbotci_recovered);
                }
            },
            releases: true,
        };
    }
    let inner = release_type_seen(
        elem,
        quote!(__jbotci_item),
        node_names,
        aliases,
        seen_aliases,
    );
    if !inner.releases {
        return empty_release();
    }
    let inner_code = inner.code;
    Release {
        code: quote! {
            for __jbotci_item in #place.iter_mut() {
                #inner_code
            }
        },
        releases: true,
    }
}

#[requires(true)]
#[ensures(true)]
fn release_recovered(
    inner: Option<&Type>,
    place: TokenStream,
    node_names: &BTreeSet<String>,
    aliases: &BTreeMap<String, Type>,
    seen_aliases: &mut BTreeSet<String>,
) -> Release {
    let Some(inner) = inner else {
        return empty_release();
    };
    if is_node(inner, node_names, aliases) {
        return Release {
            code: quote!(::jbotci_tree::node_drop::enqueue_recovered_node(#place);),
            releases: true,
        };
    }
    if box_of_node(inner, node_names, aliases).is_some() {
        return Release {
            code: quote!(::jbotci_tree::node_drop::enqueue_recovered_boxed_node(#place);),
            releases: true,
        };
    }
    let valid = release_type_seen(
        inner,
        quote!(__jbotci_recovered.as_mut()),
        node_names,
        aliases,
        seen_aliases,
    );
    let prefix = release_type_seen(
        inner,
        quote!(__jbotci_prefix.value.as_mut()),
        node_names,
        aliases,
        seen_aliases,
    );
    if !valid.releases && !prefix.releases {
        return empty_release();
    }
    let valid_code = valid.code;
    let prefix_code = prefix.code;
    Release {
        code: quote! {
            match #place {
                ::jbotci_tree::Recovered::Valid(__jbotci_recovered) => { #valid_code }
                ::jbotci_tree::Recovered::Prefix(__jbotci_prefix) => { #prefix_code }
                ::jbotci_tree::Recovered::Error(_) | ::jbotci_tree::Recovered::Released => {}
            }
        },
        releases: true,
    }
}

#[requires(true)]
#[ensures(true)]
fn sequence_elem<'a>(wrapper: &str, args: &[&'a Type]) -> Option<&'a Type> {
    let inner = args.first().copied()?;
    if matches!(wrapper, "SmallVec" | "SmallVec1")
        && let Type::Array(array) = inner
    {
        return Some(&array.elem);
    }
    Some(inner)
}

#[requires(true)]
#[ensures(true)]
fn type_arguments(arguments: &PathArguments) -> Vec<&Type> {
    let PathArguments::AngleBracketed(arguments) = arguments else {
        return Vec::new();
    };
    arguments
        .args
        .iter()
        .filter_map(|argument| match argument {
            GenericArgument::Type(ty) => Some(ty),
            _ => None,
        })
        .collect()
}

#[requires(true)]
#[ensures(true)]
fn is_node(ty: &Type, node_names: &BTreeSet<String>, aliases: &BTreeMap<String, Type>) -> bool {
    let mut seen = BTreeSet::new();
    is_node_seen(ty, node_names, aliases, &mut seen)
}

#[requires(true)]
#[ensures(true)]
fn is_node_seen(
    ty: &Type,
    node_names: &BTreeSet<String>,
    aliases: &BTreeMap<String, Type>,
    seen: &mut BTreeSet<String>,
) -> bool {
    let Type::Path(path) = ty else {
        return false;
    };
    if path.qself.is_some() || path.path.segments.len() != 1 {
        return false;
    }
    let Some(segment) = path.path.segments.last() else {
        return false;
    };
    if !matches!(segment.arguments, PathArguments::None) {
        return false;
    }
    let name = segment.ident.to_string();
    if node_names.contains(&name) {
        return true;
    }
    if !seen.insert(name.clone()) {
        return false;
    }
    aliases
        .get(&name)
        .is_some_and(|alias| is_node_seen(alias, node_names, aliases, seen))
}

#[requires(true)]
#[ensures(true)]
fn box_of_node<'a>(
    ty: &'a Type,
    node_names: &BTreeSet<String>,
    aliases: &BTreeMap<String, Type>,
) -> Option<&'a Type> {
    let Type::Path(path) = ty else {
        return None;
    };
    if path.qself.is_some() {
        return None;
    }
    let segment = path.path.segments.last()?;
    if segment.ident != "Box" {
        return None;
    }
    let inner = type_arguments(&segment.arguments).into_iter().next()?;
    is_node(inner, node_names, aliases).then_some(inner)
}

#[requires(true)]
#[ensures(true)]
fn box_of_recovered_node<'a>(
    ty: &'a Type,
    node_names: &BTreeSet<String>,
    aliases: &BTreeMap<String, Type>,
) -> Option<&'a Type> {
    let inner = wrapper_type_argument(ty, "Box")?;
    recovered_node(inner, node_names, aliases)
}

#[requires(true)]
#[ensures(true)]
fn recovered_of_boxed_node<'a>(
    ty: &'a Type,
    node_names: &BTreeSet<String>,
    aliases: &BTreeMap<String, Type>,
) -> Option<&'a Type> {
    let inner = wrapper_type_argument(ty, "Recovered")?;
    box_of_node(inner, node_names, aliases)
}

#[requires(!name.is_empty())]
#[ensures(true)]
fn wrapper_type_argument<'a>(ty: &'a Type, name: &str) -> Option<&'a Type> {
    let Type::Path(path) = ty else {
        return None;
    };
    if path.qself.is_some() {
        return None;
    }
    let segment = path.path.segments.last()?;
    if segment.ident != name {
        return None;
    }
    type_arguments(&segment.arguments).into_iter().next()
}

#[requires(true)]
#[ensures(true)]
fn recovered_node<'a>(
    ty: &'a Type,
    node_names: &BTreeSet<String>,
    aliases: &BTreeMap<String, Type>,
) -> Option<&'a Type> {
    let Type::Path(path) = ty else {
        return None;
    };
    if path.qself.is_some() {
        return None;
    }
    let segment = path.path.segments.last()?;
    if segment.ident != "Recovered" {
        return None;
    }
    let inner = type_arguments(&segment.arguments).into_iter().next()?;
    is_node(inner, node_names, aliases).then_some(inner)
}

#[requires(true)]
#[ensures(true)]
fn contains_node(
    ty: &Type,
    node_names: &BTreeSet<String>,
    aliases: &BTreeMap<String, Type>,
) -> bool {
    let mut seen = BTreeSet::new();
    contains_node_seen(ty, node_names, aliases, &mut seen)
}

#[requires(true)]
#[ensures(true)]
fn contains_node_seen(
    ty: &Type,
    node_names: &BTreeSet<String>,
    aliases: &BTreeMap<String, Type>,
    seen_aliases: &mut BTreeSet<String>,
) -> bool {
    match ty {
        Type::Paren(paren) => contains_node_seen(&paren.elem, node_names, aliases, seen_aliases),
        Type::Group(group) => contains_node_seen(&group.elem, node_names, aliases, seen_aliases),
        Type::Array(array) => contains_node_seen(&array.elem, node_names, aliases, seen_aliases),
        Type::Tuple(tuple) => tuple
            .elems
            .iter()
            .any(|elem| contains_node_seen(elem, node_names, aliases, seen_aliases)),
        Type::Path(path) => {
            if path.qself.is_some() {
                return false;
            }
            let Some(segment) = path.path.segments.last() else {
                return false;
            };
            let args = type_arguments(&segment.arguments);
            if path.path.segments.len() == 1 && args.is_empty() {
                let name = segment.ident.to_string();
                if node_names.contains(&name) {
                    return true;
                }
                if seen_aliases.insert(name.clone())
                    && let Some(alias) = aliases.get(&name)
                {
                    let found = contains_node_seen(alias, node_names, aliases, seen_aliases);
                    seen_aliases.remove(&name);
                    return found;
                }
                return false;
            }
            let bare_modifiers = segment.ident == "WithFreeModifiers" && args.is_empty();
            args.into_iter()
                .any(|arg| contains_node_seen(arg, node_names, aliases, seen_aliases))
                || bare_modifiers
        }
        _ => false,
    }
}
