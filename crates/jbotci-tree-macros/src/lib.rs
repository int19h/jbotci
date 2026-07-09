//! Proc-macro implementation for generic jbotci tree models.

extern crate proc_macro;

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

#[allow(unused_imports)]
use bityzba::{ensures, invariant, requires};
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    Attribute, Fields, GenericArgument, Ident, Item, ItemEnum, ItemStruct, ItemType, PathArguments,
    Type, parse_macro_input, parse_quote, punctuated::Punctuated,
};

#[requires(true)]
#[ensures(true)]
#[proc_macro]
pub fn tree_model(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::File);
    expand_tree_model(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

#[requires(true)]
#[ensures(true)]
fn expand_tree_model(input: syn::File) -> syn::Result<proc_macro2::TokenStream> {
    let options = tree_model_options(&input.attrs)?;
    let mut items = input.items;
    let node_names = collect_node_names(&items)?;
    let aliases = collect_type_aliases(&items);
    let atom_types = collect_atom_types(&items, &node_names, &aliases)?;
    let node_ref = node_ref_enum(&items)?;
    let atom_ref = atom_ref_enum(&atom_types);
    let walk_api = walk_api(
        &items,
        &atom_types,
        false,
        options.generate_with_free_modifiers,
    )?;
    let trait_impls = tree_node_trait_impls(&items, &node_names)?;
    let atom_impls = atom_trait_impls(&atom_types);
    let wrapper_impls = wrapper_trait_impls(false, options.generate_with_free_modifiers);
    let valid_state_impls = valid_field_state_impls(&items)?;
    let valid_module = valid_module(&items);
    let recovered_module = if options.generate_recovered {
        recovered_module(&items, &node_names, &aliases)?
    } else {
        quote!()
    };
    let cleaned_items = items
        .iter_mut()
        .map(strip_tree_attrs_from_item)
        .collect::<syn::Result<Vec<_>>>()?;

    Ok(quote! {
        #(#cleaned_items)*

        #valid_module

        #recovered_module

        #node_ref
        #atom_ref

        pub trait TreeNode {
            fn as_node_ref<'tree>(&'tree self) -> Option<NodeRef<'tree>> {
                None
            }

            fn visit_in_order<'tree, V>(&'tree self, visitor: &mut V)
            where
                V: ::jbotci_tree::TreeVisitor<'tree, Node = NodeRef<'tree>, Atom = AtomRef<'tree>>;

            fn path_to_node<'tree>(
                &'tree self,
                target: NodeRef<'tree>,
            ) -> Option<::jbotci_tree::TreePath> {
                let mut path = ::jbotci_tree::TreePath::new();
                if self.path_to_node_from(target, &mut path) {
                    Some(path)
                } else {
                    None
                }
            }

            fn node_at_path<'tree>(
                &'tree self,
                path: &::jbotci_tree::TreePath,
            ) -> Option<NodeRef<'tree>> {
                self.node_at_path_steps(path.steps())
            }

            fn path_to_node_from<'tree>(
                &'tree self,
                target: NodeRef<'tree>,
                path: &mut ::jbotci_tree::TreePath,
            ) -> bool;

            fn node_at_path_steps<'tree>(
                &'tree self,
                steps: &[::jbotci_tree::TreePathStep],
            ) -> Option<NodeRef<'tree>>;
        }

        #walk_api
        #wrapper_impls
        #atom_impls
        #trait_impls
        #valid_state_impls
    })
}

#[invariant(true)]
struct TreeModelOptions {
    generate_recovered: bool,
    generate_with_free_modifiers: bool,
}

#[requires(true)]
#[ensures(true)]
fn tree_model_options(attrs: &[Attribute]) -> syn::Result<TreeModelOptions> {
    let mut generate_recovered = false;
    let mut generate_with_free_modifiers = false;
    for attr in attrs {
        if attr.path().is_ident("tree_recovered") {
            if !matches!(attr.meta, syn::Meta::Path(_)) {
                return Err(syn::Error::new_spanned(
                    attr,
                    "`tree_recovered` does not take arguments",
                ));
            }
            generate_recovered = true;
        } else if attr.path().is_ident("tree_with_free_modifiers") {
            if !matches!(attr.meta, syn::Meta::Path(_)) {
                return Err(syn::Error::new_spanned(
                    attr,
                    "`tree_with_free_modifiers` does not take arguments",
                ));
            }
            generate_with_free_modifiers = true;
        } else {
            return Err(syn::Error::new_spanned(
                attr,
                "tree_model! accepts only the inner attributes `#![tree_recovered]` and `#![tree_with_free_modifiers]`",
            ));
        }
    }
    Ok(TreeModelOptions {
        generate_recovered,
        generate_with_free_modifiers,
    })
}

#[requires(true)]
#[ensures(true)]
fn valid_module(items: &[Item]) -> proc_macro2::TokenStream {
    let names = items.iter().filter_map(item_ident);
    quote! {
        pub mod valid {
            pub use super::{#(#names,)* AtomRef, NodeRef, TreeNode, TreeWalkable, TreeWalker};
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn item_ident(item: &Item) -> Option<&Ident> {
    match item {
        Item::Struct(item) => Some(&item.ident),
        Item::Enum(item) => Some(&item.ident),
        Item::Type(item) => Some(&item.ident),
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn recovered_module(
    items: &[Item],
    node_names: &BTreeSet<String>,
    aliases: &BTreeMap<String, Type>,
) -> syn::Result<proc_macro2::TokenStream> {
    let mut recovered_items = recovered_items(items, node_names, aliases)?;
    let recovered_aliases = collect_type_aliases(&recovered_items);
    let recovered_atom_types =
        collect_atom_types(&recovered_items, node_names, &recovered_aliases)?;
    let node_ref = node_ref_enum(&recovered_items)?;
    let atom_ref = atom_ref_enum(&recovered_atom_types);
    let has_with_free_modifiers = items_use_wrapper(items, "WithFreeModifiers");
    let walk_api = walk_api(
        &recovered_items,
        &recovered_atom_types,
        true,
        has_with_free_modifiers,
    )?;
    let trait_impls = tree_node_trait_impls(&recovered_items, node_names)?;
    let atom_impls = atom_trait_impls(&recovered_atom_types);
    let wrapper_impls = wrapper_trait_impls(true, has_with_free_modifiers);
    let with_free_modifiers = recovered_with_free_modifiers(has_with_free_modifiers);
    let conversion_impls =
        recovered_conversion_impls(&recovered_items, items, node_names, aliases)?;
    let state_impls = recovered_field_state_impls(
        &recovered_items,
        &recovered_atom_types,
        has_with_free_modifiers,
    )?;
    let cleaned_items = recovered_items
        .iter_mut()
        .map(strip_tree_attrs_from_item)
        .collect::<syn::Result<Vec<_>>>()?;

    Ok(quote! {
        pub mod recovered {
            use super::*;

            // The recovery module is intentionally tied to the jbotci syntax
            // model conventions: callers provide a `RecoveryTreeItem` item
            // type in the same tree model, and generated recovered values use
            // that type for every recovery wrapper.
            pub type Recovered<T> = ::jbotci_tree::Recovered<T, super::RecoveryTreeItem>;
            pub type RecoveryError = ::jbotci_tree::RecoveryError<super::RecoveryTreeItem>;

            #with_free_modifiers

            #(#cleaned_items)*

            #node_ref
            #atom_ref

            pub trait TreeNode {
                fn as_node_ref<'tree>(&'tree self) -> Option<NodeRef<'tree>> {
                    None
                }

                fn visit_in_order<'tree, V>(&'tree self, visitor: &mut V)
                where
                    V: ::jbotci_tree::TreeVisitor<'tree, Node = NodeRef<'tree>, Atom = AtomRef<'tree>>;

                fn path_to_node<'tree>(
                    &'tree self,
                    target: NodeRef<'tree>,
                ) -> Option<::jbotci_tree::TreePath> {
                    let mut path = ::jbotci_tree::TreePath::new();
                    if self.path_to_node_from(target, &mut path) {
                        Some(path)
                    } else {
                        None
                    }
                }

                fn node_at_path<'tree>(
                    &'tree self,
                    path: &::jbotci_tree::TreePath,
                ) -> Option<NodeRef<'tree>> {
                    self.node_at_path_steps(path.steps())
                }

                fn path_to_node_from<'tree>(
                    &'tree self,
                    target: NodeRef<'tree>,
                    path: &mut ::jbotci_tree::TreePath,
                ) -> bool;

                fn node_at_path_steps<'tree>(
                    &'tree self,
                    steps: &[::jbotci_tree::TreePathStep],
                ) -> Option<NodeRef<'tree>>;
            }

            #walk_api
            #wrapper_impls
            #atom_impls
            #trait_impls
            #state_impls
            #conversion_impls
        }
    })
}

#[requires(true)]
#[ensures(true)]
fn collect_node_names(items: &[Item]) -> syn::Result<BTreeSet<String>> {
    let mut names = BTreeSet::new();
    for item in items {
        match item {
            Item::Struct(item) => {
                reject_generic_node(&item.ident, &item.generics)?;
                names.insert(item.ident.to_string());
            }
            Item::Enum(item) => {
                reject_generic_node(&item.ident, &item.generics)?;
                names.insert(item.ident.to_string());
            }
            Item::Type(_) => {}
            other => {
                return Err(syn::Error::new_spanned(
                    other,
                    "tree_model! currently accepts only struct, enum, and type alias items",
                ));
            }
        }
    }
    Ok(names)
}

#[requires(true)]
#[ensures(true)]
fn collect_type_aliases(items: &[Item]) -> BTreeMap<String, Type> {
    items
        .iter()
        .filter_map(|item| match item {
            Item::Type(item) => Some((item.ident.to_string(), (*item.ty).clone())),
            _ => None,
        })
        .collect()
}

#[requires(true)]
#[ensures(true)]
fn reject_generic_node(ident: &Ident, generics: &syn::Generics) -> syn::Result<()> {
    if generics.params.is_empty() {
        return Ok(());
    }
    Err(syn::Error::new_spanned(
        ident,
        "tree_model! node declarations must be concrete; use a transparent wrapper impl for generic helpers",
    ))
}

#[requires(true)]
#[ensures(true)]
fn collect_atom_types(
    items: &[Item],
    node_names: &BTreeSet<String>,
    aliases: &BTreeMap<String, Type>,
) -> syn::Result<BTreeMap<String, Type>> {
    let mut atoms = BTreeMap::new();
    for item in items {
        match item {
            Item::Struct(item) => {
                collect_atoms_from_fields(&item.fields, node_names, aliases, &mut atoms)?
            }
            Item::Enum(item) => {
                for variant in &item.variants {
                    collect_atoms_from_fields(&variant.fields, node_names, aliases, &mut atoms)?;
                }
            }
            _ => {}
        }
    }
    Ok(atoms)
}

#[requires(true)]
#[ensures(true)]
fn collect_atoms_from_fields(
    fields: &Fields,
    node_names: &BTreeSet<String>,
    aliases: &BTreeMap<String, Type>,
    atoms: &mut BTreeMap<String, Type>,
) -> syn::Result<()> {
    for field in fields {
        let flags = tree_child_flags(&field.attrs)?;
        if flags.skip {
            continue;
        }
        collect_atom_type(&field.ty, node_names, aliases, atoms)?;
    }
    Ok(())
}

#[requires(true)]
#[ensures(true)]
fn collect_atom_type(
    ty: &Type,
    node_names: &BTreeSet<String>,
    aliases: &BTreeMap<String, Type>,
    atoms: &mut BTreeMap<String, Type>,
) -> syn::Result<()> {
    reject_reference_tree_type(ty, aliases)?;
    match unwrap_tree_type(ty, node_names, aliases) {
        UnwrappedTreeType::Node => {}
        UnwrappedTreeType::Atom(atom) => {
            let key = quote!(#atom).to_string();
            atoms.entry(key).or_insert_with(|| atom.clone());
        }
        UnwrappedTreeType::Children(children) => {
            for child in children {
                collect_atom_type(child, node_names, aliases, atoms)?;
            }
        }
    }
    Ok(())
}

#[invariant(true)]
#[invariant(::Atom(_) => true)]
#[invariant(::Children(_) => true)]
enum UnwrappedTreeType<'a> {
    Node,
    Atom(&'a Type),
    Children(Vec<&'a Type>),
}

#[requires(true)]
#[ensures(true)]
fn unwrap_tree_type<'a>(
    ty: &'a Type,
    node_names: &BTreeSet<String>,
    aliases: &'a BTreeMap<String, Type>,
) -> UnwrappedTreeType<'a> {
    unwrap_tree_type_with_seen(ty, node_names, aliases, &mut BTreeSet::new())
}

#[requires(true)]
#[ensures(true)]
fn unwrap_tree_type_with_seen<'a>(
    ty: &'a Type,
    node_names: &BTreeSet<String>,
    aliases: &'a BTreeMap<String, Type>,
    seen_aliases: &mut BTreeSet<String>,
) -> UnwrappedTreeType<'a> {
    match ty {
        Type::Path(path) => {
            if path.qself.is_none()
                && path.path.segments.last().is_some_and(|segment| {
                    WRAPPER_TYPES.contains(&segment.ident.to_string().as_str())
                })
            {
                let segment = path.path.segments.last().unwrap();
                if segment.ident == "Chain" {
                    let children = type_arguments(&segment.arguments);
                    return if children.is_empty() {
                        UnwrappedTreeType::Atom(ty)
                    } else {
                        UnwrappedTreeType::Children(children)
                    };
                }
                let Some(inner) = first_type_argument(&segment.arguments) else {
                    return UnwrappedTreeType::Atom(ty);
                };
                return match inner {
                    Type::Array(array) => UnwrappedTreeType::Children(vec![&array.elem]),
                    other => UnwrappedTreeType::Children(vec![other]),
                };
            }
            let Some(last) = path.path.segments.last() else {
                return UnwrappedTreeType::Atom(ty);
            };
            if path.qself.is_none()
                && path.path.segments.len() == 1
                && let Some(alias) = aliases.get(&last.ident.to_string())
                && seen_aliases.insert(last.ident.to_string())
            {
                return unwrap_tree_type_with_seen(alias, node_names, aliases, seen_aliases);
            }
            if path.path.segments.len() == 1 && node_names.contains(&last.ident.to_string()) {
                UnwrappedTreeType::Node
            } else {
                UnwrappedTreeType::Atom(ty)
            }
        }
        Type::Reference(reference) => {
            unwrap_tree_type_with_seen(&reference.elem, node_names, aliases, seen_aliases)
        }
        Type::Array(array) => UnwrappedTreeType::Children(vec![&array.elem]),
        Type::Tuple(tuple) => UnwrappedTreeType::Children(tuple.elems.iter().collect()),
        _ => UnwrappedTreeType::Atom(ty),
    }
}

const WRAPPER_TYPES: &[&str] = &[
    "Box",
    "Arc",
    "Option",
    "Vec",
    "Vec1",
    "SmallVec",
    "SmallVec1",
    "WithFreeModifiers",
    "Recovered",
    "Chain",
];

#[requires(true)]
#[ensures(ret.is_none_or(|_| !type_arguments(arguments).is_empty()))]
fn first_type_argument(arguments: &PathArguments) -> Option<&Type> {
    type_arguments(arguments).into_iter().next()
}

#[requires(true)]
#[ensures(ret.is_none_or(|_| index < type_arguments(arguments).len()))]
fn nth_type_argument(arguments: &PathArguments, index: usize) -> Option<&Type> {
    type_arguments(arguments).into_iter().nth(index)
}

#[requires(true)]
#[ensures(true)]
fn reject_reference_tree_type(ty: &Type, aliases: &BTreeMap<String, Type>) -> syn::Result<()> {
    reject_reference_tree_type_with_seen(ty, aliases, &mut BTreeSet::new())
}

#[requires(true)]
#[ensures(true)]
fn reject_reference_tree_type_with_seen(
    ty: &Type,
    aliases: &BTreeMap<String, Type>,
    seen_aliases: &mut BTreeSet<String>,
) -> syn::Result<()> {
    match ty {
        Type::Reference(reference) => Err(reference_tree_type_error(reference)),
        Type::Path(path) if path.qself.is_none() => {
            if path.path.segments.len() == 1
                && let Some(segment) = path.path.segments.last()
                && let Some(alias) = aliases.get(&segment.ident.to_string())
                && seen_aliases.insert(segment.ident.to_string())
            {
                return reject_reference_tree_type_with_seen(alias, aliases, seen_aliases);
            }
            for argument in path
                .path
                .segments
                .iter()
                .flat_map(|segment| type_arguments(&segment.arguments))
            {
                reject_reference_tree_type_with_seen(argument, aliases, seen_aliases)?;
            }
            Ok(())
        }
        Type::Array(array) => {
            reject_reference_tree_type_with_seen(&array.elem, aliases, seen_aliases)
        }
        Type::Tuple(tuple) => {
            for elem in &tuple.elems {
                reject_reference_tree_type_with_seen(elem, aliases, seen_aliases)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

#[requires(true)]
#[ensures(true)]
fn reference_tree_type_error(reference: &syn::TypeReference) -> syn::Error {
    syn::Error::new_spanned(
        reference,
        "tree_model! tree fields cannot use reference types; use an owned field type",
    )
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
fn strip_tree_attrs_from_item(item: &mut Item) -> syn::Result<Item> {
    match item {
        Item::Struct(item) => {
            strip_tree_attrs_from_fields(&mut item.fields)?;
            Ok(Item::Struct(item.clone()))
        }
        Item::Enum(item) => {
            for variant in &mut item.variants {
                strip_tree_attrs_from_fields(&mut variant.fields)?;
            }
            Ok(Item::Enum(item.clone()))
        }
        Item::Type(item) => Ok(Item::Type(ItemType {
            attrs: item.attrs.clone(),
            vis: item.vis.clone(),
            type_token: item.type_token,
            ident: item.ident.clone(),
            generics: item.generics.clone(),
            eq_token: item.eq_token,
            ty: item.ty.clone(),
            semi_token: item.semi_token,
        })),
        other => Err(syn::Error::new_spanned(
            other,
            "tree_model! currently accepts only struct, enum, and type alias items",
        )),
    }
}

#[requires(true)]
#[ensures(true)]
fn recovered_items(
    items: &[Item],
    node_names: &BTreeSet<String>,
    aliases: &BTreeMap<String, Type>,
) -> syn::Result<Vec<Item>> {
    items
        .iter()
        .map(|item| recovered_item(item, node_names, aliases))
        .collect()
}

#[requires(true)]
#[ensures(true)]
fn recovered_item(
    item: &Item,
    node_names: &BTreeSet<String>,
    aliases: &BTreeMap<String, Type>,
) -> syn::Result<Item> {
    match item {
        Item::Struct(item) => {
            let mut item = item.clone();
            item.attrs = recovered_attrs(&item.attrs);
            transform_fields_for_recovery(&mut item.fields, node_names, aliases)?;
            Ok(Item::Struct(item))
        }
        Item::Enum(item) => {
            let mut item = item.clone();
            item.attrs = recovered_attrs(&item.attrs);
            for variant in &mut item.variants {
                transform_fields_for_recovery(&mut variant.fields, node_names, aliases)?;
            }
            Ok(Item::Enum(item))
        }
        Item::Type(item) => {
            let mut item = item.clone();
            item.ty = Box::new(transform_type_for_recovery(&item.ty, node_names, aliases)?);
            Ok(Item::Type(item))
        }
        other => Err(syn::Error::new_spanned(
            other,
            "tree_model! currently accepts only struct, enum, and type alias items",
        )),
    }
}

#[requires(true)]
#[ensures(true)]
fn recovered_attrs(attrs: &[Attribute]) -> Vec<Attribute> {
    let mut recovered_attrs: Vec<_> = attrs
        .iter()
        .filter(|attr| {
            !attr.path().is_ident("invariant") && !attr.path().is_ident("expensive_invariant")
        })
        .cloned()
        .collect();
    if !attrs_derive_trait(&recovered_attrs, "Deserialize") {
        recovered_attrs.push(parse_quote!(#[derive(::serde::Deserialize)]));
    }
    recovered_attrs
}

#[requires(!trait_name.is_empty())]
#[ensures(true)]
fn attrs_derive_trait(attrs: &[Attribute], trait_name: &str) -> bool {
    attrs.iter().any(|attr| {
        if !attr.path().is_ident("derive") {
            return false;
        }
        attr.parse_args_with(Punctuated::<syn::Path, syn::Token![,]>::parse_terminated)
            .ok()
            .is_some_and(|paths| {
                paths.iter().any(|path| {
                    path.segments
                        .last()
                        .is_some_and(|segment| segment.ident == trait_name)
                })
            })
    })
}

#[requires(true)]
#[ensures(true)]
fn transform_fields_for_recovery(
    fields: &mut Fields,
    node_names: &BTreeSet<String>,
    aliases: &BTreeMap<String, Type>,
) -> syn::Result<()> {
    for field in fields {
        reject_reference_tree_type(&field.ty, aliases)?;
        field.ty = transform_type_for_recovery(&field.ty, node_names, aliases)?;
    }
    Ok(())
}

#[requires(true)]
#[ensures(true)]
fn transform_type_for_recovery(
    ty: &Type,
    node_names: &BTreeSet<String>,
    aliases: &BTreeMap<String, Type>,
) -> syn::Result<Type> {
    match ty {
        Type::Path(path) if path.qself.is_none() => {
            let Some(last) = path.path.segments.last() else {
                return Ok(wrap_recovered(ty.clone()));
            };
            if path.path.segments.len() == 1 && aliases.contains_key(&last.ident.to_string()) {
                return Ok(ty.clone());
            }
            if is_wrapper_ident(&last.ident) {
                return transform_wrapper_type_for_recovery(ty, node_names, aliases);
            }
            Ok(wrap_recovered(ty.clone()))
        }
        Type::Reference(reference) => Err(reference_tree_type_error(reference)),
        Type::Array(array) => {
            let mut array = array.clone();
            array.elem = Box::new(transform_type_for_recovery(
                &array.elem,
                node_names,
                aliases,
            )?);
            Ok(Type::Array(array))
        }
        Type::Tuple(tuple) => {
            let mut tuple = tuple.clone();
            for elem in &mut tuple.elems {
                *elem = transform_type_for_recovery(elem, node_names, aliases)?;
            }
            Ok(Type::Tuple(tuple))
        }
        _ => {
            let _ = node_names;
            Ok(wrap_recovered(ty.clone()))
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn transform_wrapper_type_for_recovery(
    ty: &Type,
    node_names: &BTreeSet<String>,
    aliases: &BTreeMap<String, Type>,
) -> syn::Result<Type> {
    let mut ty = ty.clone();
    let Type::Path(path) = &mut ty else {
        return Ok(ty);
    };
    let Some(segment) = path.path.segments.last_mut() else {
        return Ok(ty);
    };
    if segment.ident == "WithFreeModifiers" {
        let inner = first_type_argument(&segment.arguments)
            .cloned()
            .ok_or_else(|| {
                syn::Error::new_spanned(
                    &*segment,
                    "`WithFreeModifiers` needs a value type argument",
                )
            })?;
        let inner = transform_type_for_recovery(&inner, node_names, aliases)?;
        segment.arguments = PathArguments::AngleBracketed(parse_quote!(<#inner>));
        return Ok(ty);
    }
    let PathArguments::AngleBracketed(arguments) = &mut segment.arguments else {
        return Ok(ty);
    };
    let transform_all = segment.ident == "Chain";
    for argument in &mut arguments.args {
        if let GenericArgument::Type(inner) = argument {
            *inner = transform_type_for_recovery(inner, node_names, aliases)?;
            if !transform_all {
                break;
            }
        }
    }
    Ok(ty)
}

#[requires(true)]
#[ensures(true)]
fn wrap_recovered(ty: Type) -> Type {
    parse_quote!(Recovered<#ty>)
}

#[requires(true)]
#[ensures(true)]
fn is_wrapper_ident(ident: &Ident) -> bool {
    WRAPPER_TYPES.contains(&ident.to_string().as_str())
}

#[requires(true)]
#[ensures(true)]
fn recovered_with_free_modifiers(emit: bool) -> proc_macro2::TokenStream {
    if !emit {
        return quote!();
    }
    quote! {
        // `WithFreeModifiers` and `FreeModifierSyntax` are jbotci syntax-model
        // conventions used by generated recovered syntax trees.
        #[derive(Debug, Clone, PartialEq, Eq, ::serde::Serialize, ::serde::Deserialize)]
        pub struct WithFreeModifiers<T> {
            pub value: T,
            pub free_modifiers: Vec<Recovered<FreeModifierSyntax>>,
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn recovered_field_state_impls(
    recovered_items: &[Item],
    atom_types: &BTreeMap<String, Type>,
    has_with_free_modifiers: bool,
) -> syn::Result<proc_macro2::TokenStream> {
    let node_impls = recovered_items
        .iter()
        .filter_map(|item| match item {
            Item::Struct(item) => Some(recovered_struct_field_state_impl(item)),
            Item::Enum(item) => Some(recovered_enum_field_state_impl(item)),
            Item::Type(_) => None,
            other => Some(Err(syn::Error::new_spanned(
                other,
                "tree_model! currently accepts only struct, enum, and type alias items",
            ))),
        })
        .collect::<syn::Result<Vec<_>>>()?;
    let atom_impls = atom_types
        .values()
        .filter(|ty| atom_needs_generated_recovered_field_state_impl(ty))
        .map(|ty| {
            quote! {
                #[::bityzba::contract_trait]
                impl ::jbotci_tree::RecoveredFieldState for #ty {
                    #[::bityzba::requires(true)]
                    #[::bityzba::ensures(ret == 0)]
                    fn recovery_error_slots(&self) -> usize {
                        0
                    }
                }
            }
        });
    let external_atom_checks = atom_types
        .values()
        .enumerate()
        .filter(|(_index, ty)| atom_requires_external_recovered_field_state(ty))
        .map(|(index, ty)| {
            let assert_fn = format_ident!("__jbotci_tree_assert_recovered_field_state_{index}");
            let check_fn = format_ident!("__jbotci_tree_check_recovered_field_state_{index}");
            quote! {
                #[allow(dead_code)]
                fn #assert_fn<T: ::jbotci_tree::RecoveredFieldState>() {}

                #[allow(dead_code)]
                fn #check_fn() {
                    #assert_fn::<#ty>();
                }
            }
        });
    let with_free_modifiers_impl = has_with_free_modifiers.then(|| {
        quote! {
            #[::bityzba::contract_trait]
            impl<T> ::jbotci_tree::RecoveredFieldState for WithFreeModifiers<T>
            where
                T: ::jbotci_tree::RecoveredFieldState,
            {
                #[::bityzba::requires(true)]
                #[::bityzba::ensures(true)]
                fn recovery_error_slots(&self) -> usize {
                    ::jbotci_tree::RecoveredFieldState::recovery_error_slots(&self.value)
                        + ::jbotci_tree::RecoveredFieldState::recovery_error_slots(&self.free_modifiers)
                }

                #[::bityzba::requires(true)]
                #[::bityzba::ensures(true)]
                fn missing_error_slots(&self) -> usize {
                    ::jbotci_tree::RecoveredFieldState::missing_error_slots(&self.value)
                        + ::jbotci_tree::RecoveredFieldState::missing_error_slots(&self.free_modifiers)
                }
            }
        }
    });
    Ok(quote! {
        #(#node_impls)*
        #(#atom_impls)*
        #(#external_atom_checks)*
        #with_free_modifiers_impl
    })
}

#[requires(true)]
#[ensures(true)]
fn valid_field_state_impls(items: &[Item]) -> syn::Result<proc_macro2::TokenStream> {
    let impls = items
        .iter()
        .filter_map(|item| match item {
            Item::Struct(item) => {
                let ident = &item.ident;
                Some(Ok(quote! {
                    #[::bityzba::contract_trait]
                    impl ::jbotci_tree::RecoveredFieldState for #ident {
                        #[::bityzba::requires(true)]
                        #[::bityzba::ensures(ret == 0)]
                        fn recovery_error_slots(&self) -> usize {
                            0
                        }
                    }
                }))
            }
            Item::Enum(item) => {
                let ident = &item.ident;
                Some(Ok(quote! {
                    #[::bityzba::contract_trait]
                    impl ::jbotci_tree::RecoveredFieldState for #ident {
                        #[::bityzba::requires(true)]
                        #[::bityzba::ensures(ret == 0)]
                        fn recovery_error_slots(&self) -> usize {
                            0
                        }
                    }
                }))
            }
            Item::Type(_) => None,
            other => Some(Err(syn::Error::new_spanned(
                other,
                "tree_model! currently accepts only struct, enum, and type alias items",
            ))),
        })
        .collect::<syn::Result<Vec<_>>>()?;
    Ok(quote!(#(#impls)*))
}

#[requires(true)]
#[ensures(true)]
fn atom_needs_generated_recovered_field_state_impl(ty: &Type) -> bool {
    let Type::Path(path) = ty else {
        return true;
    };
    if path.qself.is_some() {
        return true;
    }
    if path.path.segments.len() > 1 {
        return false;
    }
    !path.path.segments.last().is_some_and(|segment| {
        // `Word` is a jbotci syntax-model convention: generated syntax models
        // provide the corresponding RecoveredFieldState implementation outside
        // this generic tree macro.
        matches!(
            segment.ident.to_string().as_str(),
            "String" | "SourceSpan" | "Word"
        )
    })
}

#[requires(true)]
#[ensures(true)]
fn atom_requires_external_recovered_field_state(ty: &Type) -> bool {
    matches!(
        ty,
        Type::Path(path) if path.qself.is_none() && path.path.segments.len() > 1
    )
}

#[requires(true)]
#[ensures(true)]
fn recovered_struct_field_state_impl(item: &ItemStruct) -> syn::Result<proc_macro2::TokenStream> {
    let ident = &item.ident;
    let sum = recovered_field_state_sum(&item.fields, |index, field| {
        field
            .ident
            .as_ref()
            .map(|ident| quote!(&self.#ident))
            .unwrap_or_else(|| {
                let index = syn::Index::from(index);
                quote!(&self.#index)
            })
    })?;
    let missing_sum =
        recovered_field_state_unconsumed_missing_sum(&item.fields, |index, field| {
            field
                .ident
                .as_ref()
                .map(|ident| quote!(&self.#ident))
                .unwrap_or_else(|| {
                    let index = syn::Index::from(index);
                    quote!(&self.#index)
                })
        })?;
    Ok(quote! {
        #[::bityzba::contract_trait]
        impl ::jbotci_tree::RecoveredFieldState for #ident {
            #[::bityzba::requires(true)]
            #[::bityzba::ensures(true)]
            fn recovery_error_slots(&self) -> usize {
                #sum
            }

            #[::bityzba::requires(true)]
            #[::bityzba::ensures(true)]
            fn missing_error_slots(&self) -> usize {
                #missing_sum
            }
        }
    })
}

#[requires(true)]
#[ensures(true)]
fn recovered_enum_field_state_impl(item: &ItemEnum) -> syn::Result<proc_macro2::TokenStream> {
    let enum_ident = &item.ident;
    let mut error_slot_arms = Vec::new();
    let mut missing_slot_arms = Vec::new();
    for variant in &item.variants {
        let variant_ident = &variant.ident;
        match &variant.fields {
            Fields::Named(fields) => {
                let bindings = fields
                    .named
                    .iter()
                    .map(|field| field.ident.as_ref().unwrap())
                    .collect::<Vec<_>>();
                let pattern_bindings = bindings.clone();
                let sum = recovered_field_state_sum(&variant.fields, |_index, field| {
                    let ident = field.ident.as_ref().unwrap();
                    quote!(#ident)
                })?;
                let missing_sum = recovered_field_state_unconsumed_missing_sum(
                    &variant.fields,
                    |_index, field| {
                        let ident = field.ident.as_ref().unwrap();
                        quote!(#ident)
                    },
                )?;
                error_slot_arms.push(quote! {
                    Self::#variant_ident { #(#pattern_bindings,)* } => #sum
                });
                let pattern_bindings = bindings;
                missing_slot_arms.push(quote! {
                    Self::#variant_ident { #(#pattern_bindings,)* } => #missing_sum
                });
            }
            Fields::Unnamed(fields) => {
                let bindings = (0..fields.unnamed.len())
                    .map(|index| format_ident!("field_{index}"))
                    .collect::<Vec<_>>();
                let pattern_bindings = bindings.clone();
                let sum = recovered_field_state_sum(&variant.fields, |index, _field| {
                    let ident = &bindings[index];
                    quote!(#ident)
                })?;
                let missing_sum = recovered_field_state_unconsumed_missing_sum(
                    &variant.fields,
                    |index, _field| {
                        let ident = &bindings[index];
                        quote!(#ident)
                    },
                )?;
                error_slot_arms.push(quote! {
                    Self::#variant_ident(#(#pattern_bindings,)*) => #sum
                });
                let pattern_bindings = bindings;
                missing_slot_arms.push(quote! {
                    Self::#variant_ident(#(#pattern_bindings,)*) => #missing_sum
                });
            }
            Fields::Unit => {
                error_slot_arms.push(quote!(Self::#variant_ident => 0));
                missing_slot_arms.push(quote!(Self::#variant_ident => 0));
            }
        }
    }
    Ok(quote! {
        #[::bityzba::contract_trait]
        impl ::jbotci_tree::RecoveredFieldState for #enum_ident {
            #[::bityzba::requires(true)]
            #[::bityzba::ensures(true)]
            fn recovery_error_slots(&self) -> usize {
                match self {
                    #(#error_slot_arms,)*
                }
            }

            #[::bityzba::requires(true)]
            #[::bityzba::ensures(true)]
            fn missing_error_slots(&self) -> usize {
                match self {
                    #(#missing_slot_arms,)*
                }
            }
        }
    })
}

#[requires(true)]
#[ensures(true)]
fn recovered_field_state_sum<F>(fields: &Fields, access: F) -> syn::Result<proc_macro2::TokenStream>
where
    F: Fn(usize, &syn::Field) -> proc_macro2::TokenStream,
{
    let terms = fields
        .iter()
        .enumerate()
        .filter_map(|(index, field)| match tree_child_flags(&field.attrs) {
            Ok(flags) if flags.skip => None,
            Ok(_) => {
                let access = access(index, field);
                Some(Ok(quote! {
                    + ::jbotci_tree::RecoveredFieldState::recovery_error_slots(#access)
                }))
            }
            Err(error) => Some(Err(error)),
        })
        .collect::<syn::Result<Vec<_>>>()?;
    Ok(quote!(0 #(#terms)*))
}

#[requires(true)]
#[ensures(true)]
fn recovered_field_state_unconsumed_missing_sum<F>(
    fields: &Fields,
    access: F,
) -> syn::Result<proc_macro2::TokenStream>
where
    F: Fn(usize, &syn::Field) -> proc_macro2::TokenStream,
{
    let terms = fields
        .iter()
        .enumerate()
        .filter_map(|(index, field)| match tree_child_flags(&field.attrs) {
            Ok(flags) if flags.skip => None,
            Ok(_) => {
                let access = access(index, field);
                Some(Ok(quote! {
                    + ::jbotci_tree::RecoveredFieldState::missing_error_slots(#access)
                }))
            }
            Err(error) => Some(Err(error)),
        })
        .collect::<syn::Result<Vec<_>>>()?;
    Ok(quote!(0 #(#terms)*))
}

#[requires(true)]
#[ensures(true)]
fn recovered_conversion_impls(
    recovered_items: &[Item],
    valid_items: &[Item],
    node_names: &BTreeSet<String>,
    aliases: &BTreeMap<String, Type>,
) -> syn::Result<proc_macro2::TokenStream> {
    let _ = recovered_items;
    let try_into_impls = valid_items
        .iter()
        .filter_map(|item| match item {
            Item::Struct(item) => Some(recovered_struct_conversion_impl(item, node_names, aliases)),
            Item::Enum(item) => Some(recovered_enum_conversion_impl(item, node_names, aliases)),
            Item::Type(_) => None,
            other => Some(Err(syn::Error::new_spanned(
                other,
                "tree_model! currently accepts only struct, enum, and type alias items",
            ))),
        })
        .collect::<syn::Result<Vec<_>>>()?;
    let from_valid_impls = valid_items
        .iter()
        .filter_map(|item| match item {
            Item::Struct(item) => Some(recovered_struct_from_valid_impl(item, node_names, aliases)),
            Item::Enum(item) => Some(recovered_enum_from_valid_impl(item, node_names, aliases)),
            Item::Type(_) => None,
            other => Some(Err(syn::Error::new_spanned(
                other,
                "tree_model! currently accepts only struct, enum, and type alias items",
            ))),
        })
        .collect::<syn::Result<Vec<_>>>()?;
    Ok(quote!(#(#try_into_impls)* #(#from_valid_impls)*))
}

#[requires(true)]
#[ensures(true)]
fn recovered_struct_conversion_impl(
    item: &ItemStruct,
    node_names: &BTreeSet<String>,
    aliases: &BTreeMap<String, Type>,
) -> syn::Result<proc_macro2::TokenStream> {
    let ident = &item.ident;
    let valid_ty = quote!(super::#ident);
    let mut field_conversions = Vec::new();
    let mut field_values = Vec::new();
    let mut raw_bindings = Vec::new();
    for (index, field) in item.fields.iter().enumerate() {
        let binding = field_binding_ident(index, field);
        let raw_binding = format_ident!("recovered_field_{index}");
        raw_bindings.push(raw_binding.clone());
        let path_name = field_name_tokens(field);
        let conversion =
            convert_value_for_type(&field.ty, quote!(#raw_binding), node_names, aliases)?;
        field_conversions.push(quote! {
            path.push(::jbotci_tree::TreePathStep::field(#path_name, #index));
            let #binding = #conversion?;
            path.pop();
        });
        if let Some(field_ident) = &field.ident {
            field_values.push(quote!(#field_ident: #binding));
        } else {
            field_values.push(quote!(#binding));
        }
    }
    let construct = match &item.fields {
        Fields::Named(_) => {
            if item_needs_new_macro(&Item::Struct(item.clone())) {
                quote!(#valid_ty::from_data(::bityzba::data!(#valid_ty { #(#field_values,)* })))
            } else {
                quote!(#valid_ty { #(#field_values,)* })
            }
        }
        Fields::Unnamed(_) => {
            if item_needs_new_macro(&Item::Struct(item.clone())) {
                quote!(#valid_ty::from_data(::bityzba::data!(#valid_ty(#(#field_values,)*))))
            } else {
                quote!(#valid_ty(#(#field_values,)*))
            }
        }
        Fields::Unit => {
            if item_needs_new_macro(&Item::Struct(item.clone())) {
                quote!(#valid_ty::from_data(::bityzba::data!(#valid_ty)))
            } else {
                quote!(#valid_ty)
            }
        }
    };
    let destructure = recovered_struct_conversion_destructure(item, &raw_bindings);
    Ok(quote! {
        impl #ident {
            pub fn try_into_valid(self) -> Result<#valid_ty, RecoveryError> {
                let mut path = ::jbotci_tree::TreePath::new();
                Box::new(self).try_into_valid_boxed_at_path(&mut path)
            }

            pub(crate) fn try_into_valid_at_path(
                self,
                path: &mut ::jbotci_tree::TreePath,
            ) -> Result<#valid_ty, RecoveryError> {
                Box::new(self).try_into_valid_boxed_at_path(path)
            }

            pub(crate) fn try_into_valid_boxed_at_path(
                self: Box<Self>,
                path: &mut ::jbotci_tree::TreePath,
            ) -> Result<#valid_ty, RecoveryError> {
                #destructure
                #(#field_conversions)*
                Ok(#construct)
            }
        }
    })
}

#[requires(true)]
#[ensures(true)]
fn recovered_struct_conversion_destructure(
    item: &ItemStruct,
    bindings: &[Ident],
) -> proc_macro2::TokenStream {
    match &item.fields {
        Fields::Named(fields) => {
            let names = fields
                .named
                .iter()
                .map(|field| field.ident.as_ref().unwrap())
                .collect::<Vec<_>>();
            quote!(let Self { #(#names: #bindings,)* } = *self;)
        }
        Fields::Unnamed(_) => quote!(let Self(#(#bindings,)*) = *self;),
        Fields::Unit => quote!(let _ = self;),
    }
}

#[requires(true)]
#[ensures(true)]
fn recovered_enum_conversion_impl(
    item: &ItemEnum,
    node_names: &BTreeSet<String>,
    aliases: &BTreeMap<String, Type>,
) -> syn::Result<proc_macro2::TokenStream> {
    let enum_ident = &item.ident;
    let valid_ty = quote!(super::#enum_ident);
    let needs_new = item_needs_new_macro(&Item::Enum(item.clone()));
    let arms = item
        .variants
        .iter()
        .map(|variant| {
            let variant_ident = &variant.ident;
            match &variant.fields {
                Fields::Named(fields) => {
                    let bindings = fields
                        .named
                        .iter()
                        .map(|field| field.ident.as_ref().unwrap())
                        .collect::<Vec<_>>();
                    let mut field_conversions = Vec::new();
                    let mut field_values = Vec::new();
                    for (index, field) in fields.named.iter().enumerate() {
                        let field_ident = field.ident.as_ref().unwrap();
                        let binding = format_ident!("converted_{field_ident}");
                        let path_name = field_name_tokens(field);
                        let conversion =
                            convert_value_for_type(&field.ty, quote!(#field_ident), node_names, aliases)?;
                        field_conversions.push(quote! {
                            path.push(::jbotci_tree::TreePathStep::field(#path_name, #index));
                            let #binding = #conversion?;
                            path.pop();
                        });
                        field_values.push(quote!(#field_ident: #binding));
                    }
                    let construct = if needs_new {
                        quote!(#valid_ty::from_data(::bityzba::data!(#valid_ty::#variant_ident { #(#field_values,)* })))
                    } else {
                        quote!(#valid_ty::#variant_ident { #(#field_values,)* })
                    };
                    Ok(quote! {
                        Self::#variant_ident { #(#bindings,)* } => {
                            #(#field_conversions)*
                            Ok(#construct)
                        }
                    })
                }
                Fields::Unnamed(fields) => {
                    let bindings = (0..fields.unnamed.len())
                        .map(|index| format_ident!("field_{index}"))
                        .collect::<Vec<_>>();
                    let mut field_conversions = Vec::new();
                    let mut field_values = Vec::new();
                    for (index, field) in fields.unnamed.iter().enumerate() {
                        let field_ident = &bindings[index];
                        let binding = format_ident!("converted_{index}");
                        let path_name = field_name_tokens(field);
                        let conversion =
                            convert_value_for_type(&field.ty, quote!(#field_ident), node_names, aliases)?;
                        field_conversions.push(quote! {
                            path.push(::jbotci_tree::TreePathStep::field(#path_name, #index));
                            let #binding = #conversion?;
                            path.pop();
                        });
                        field_values.push(quote!(#binding));
                    }
                    let construct = if needs_new {
                        quote!(#valid_ty::from_data(::bityzba::data!(#valid_ty::#variant_ident(#(#field_values,)*))))
                    } else {
                        quote!(#valid_ty::#variant_ident(#(#field_values,)*))
                    };
                    Ok(quote! {
                        Self::#variant_ident(#(#bindings,)*) => {
                            #(#field_conversions)*
                            Ok(#construct)
                        }
                    })
                }
                Fields::Unit => {
                    let construct = if needs_new {
                        quote!(#valid_ty::from_data(::bityzba::data!(#valid_ty::#variant_ident)))
                    } else {
                        quote!(#valid_ty::#variant_ident)
                    };
                    Ok(quote! {
                        Self::#variant_ident => Ok(#construct)
                    })
                }
            }
        })
        .collect::<syn::Result<Vec<_>>>()?;
    Ok(quote! {
        impl #enum_ident {
            pub fn try_into_valid(self) -> Result<#valid_ty, RecoveryError> {
                let mut path = ::jbotci_tree::TreePath::new();
                Box::new(self).try_into_valid_boxed_at_path(&mut path)
            }

            pub(crate) fn try_into_valid_at_path(
                self,
                path: &mut ::jbotci_tree::TreePath,
            ) -> Result<#valid_ty, RecoveryError> {
                Box::new(self).try_into_valid_boxed_at_path(path)
            }

            pub(crate) fn try_into_valid_boxed_at_path(
                self: Box<Self>,
                path: &mut ::jbotci_tree::TreePath,
            ) -> Result<#valid_ty, RecoveryError> {
                match *self {
                    #(#arms,)*
                }
            }
        }
    })
}

#[requires(true)]
#[ensures(true)]
fn recovered_struct_from_valid_impl(
    item: &ItemStruct,
    node_names: &BTreeSet<String>,
    aliases: &BTreeMap<String, Type>,
) -> syn::Result<proc_macro2::TokenStream> {
    let ident = &item.ident;
    let valid_ty = quote!(super::#ident);
    let bindings = item
        .fields
        .iter()
        .enumerate()
        .map(|(index, field)| field_binding_ident(index, field))
        .collect::<Vec<_>>();
    let conversions = item
        .fields
        .iter()
        .enumerate()
        .map(|(index, field)| {
            let binding = &bindings[index];
            let converted =
                convert_valid_value_for_type(&field.ty, quote!(#binding), node_names, aliases)?;
            Ok(quote!(let #binding = #converted;))
        })
        .collect::<syn::Result<Vec<_>>>()?;
    let field_values = item
        .fields
        .iter()
        .enumerate()
        .map(|(index, field)| {
            let binding = &bindings[index];
            field
                .ident
                .as_ref()
                .map(|ident| quote!(#ident: #binding))
                .unwrap_or_else(|| quote!(#binding))
        })
        .collect::<Vec<_>>();
    let destructure = struct_from_valid_destructure(item, &bindings);
    let boxed_destructure = struct_from_valid_boxed_destructure(item, &bindings);
    let construct = match &item.fields {
        Fields::Named(_) => quote!(Self { #(#field_values,)* }),
        Fields::Unnamed(_) => quote!(Self(#(#field_values,)*)),
        Fields::Unit => quote!(Self),
    };
    Ok(quote! {
        impl #ident {
            pub fn from_valid(value: #valid_ty) -> Self {
                *Self::from_valid_boxed(Box::new(value))
            }

            pub fn from_valid_boxed(value: Box<#valid_ty>) -> Box<Self> {
                #boxed_destructure
                #(#conversions)*
                Box::new(#construct)
            }

            pub(crate) fn from_valid_unboxed(value: #valid_ty) -> Self {
                #destructure
                #(#conversions)*
                #construct
            }
        }
    })
}

#[requires(true)]
#[ensures(true)]
fn struct_from_valid_destructure(
    item: &ItemStruct,
    bindings: &[Ident],
) -> proc_macro2::TokenStream {
    let ident = &item.ident;
    let valid_ty = quote!(super::#ident);
    let needs_data = item_needs_new_macro(&Item::Struct(item.clone()));
    match &item.fields {
        Fields::Named(fields) => {
            let names = fields
                .named
                .iter()
                .map(|field| field.ident.as_ref().unwrap());
            if needs_data {
                quote! {
                    let ::bityzba::data!(#valid_ty { #(#names: #bindings,)* }) = value.into_data();
                }
            } else {
                quote! {
                    let #valid_ty { #(#names: #bindings,)* } = value;
                }
            }
        }
        Fields::Unnamed(_) => {
            if needs_data {
                quote! {
                    let ::bityzba::data!(#valid_ty(#(#bindings,)*)) = value.into_data();
                }
            } else {
                quote! {
                    let #valid_ty(#(#bindings,)*) = value;
                }
            }
        }
        Fields::Unit => quote! {
            let _ = value;
        },
    }
}

#[requires(true)]
#[ensures(true)]
fn struct_from_valid_boxed_destructure(
    item: &ItemStruct,
    bindings: &[Ident],
) -> proc_macro2::TokenStream {
    let ident = &item.ident;
    let valid_ty = quote!(super::#ident);
    let needs_data = item_needs_new_macro(&Item::Struct(item.clone()));
    match &item.fields {
        Fields::Named(fields) => {
            let names = fields
                .named
                .iter()
                .map(|field| field.ident.as_ref().unwrap());
            if needs_data {
                quote! {
                    let ::bityzba::data!(#valid_ty { #(#names: #bindings,)* }) = (*value).into_data();
                }
            } else {
                quote! {
                    let #valid_ty { #(#names: #bindings,)* } = *value;
                }
            }
        }
        Fields::Unnamed(_) => {
            if needs_data {
                quote! {
                    let ::bityzba::data!(#valid_ty(#(#bindings,)*)) = (*value).into_data();
                }
            } else {
                quote! {
                    let #valid_ty(#(#bindings,)*) = *value;
                }
            }
        }
        Fields::Unit => quote! {
            let _ = value;
        },
    }
}

#[requires(true)]
#[ensures(true)]
fn recovered_enum_from_valid_impl(
    item: &ItemEnum,
    node_names: &BTreeSet<String>,
    aliases: &BTreeMap<String, Type>,
) -> syn::Result<proc_macro2::TokenStream> {
    let enum_ident = &item.ident;
    let valid_ty = quote!(super::#enum_ident);
    let uses_data_patterns = enum_uses_data_patterns(item);
    let arms = item
        .variants
        .iter()
        .map(|variant| {
            let variant_ident = &variant.ident;
            match &variant.fields {
                Fields::Named(fields) => {
                    let bindings = fields
                        .named
                        .iter()
                        .map(|field| field.ident.as_ref().unwrap().clone())
                        .collect::<Vec<_>>();
                    let pattern_bindings = bindings.clone();
                    let conversions = fields
                        .named
                        .iter()
                        .enumerate()
                        .map(|(index, field)| {
                            let binding = &bindings[index];
                            let converted = convert_valid_value_for_type(
                                &field.ty,
                                quote!(#binding),
                                node_names,
                                aliases,
                            )?;
                            Ok(quote!(let #binding = #converted;))
                        })
                        .collect::<syn::Result<Vec<_>>>()?;
                    let field_values = bindings.iter().map(|binding| quote!(#binding: #binding));
                    let pattern = if uses_data_patterns {
                        quote!(
                            ::bityzba::data!(#valid_ty::#variant_ident { #(#pattern_bindings,)* })
                        )
                    } else {
                        quote!(#valid_ty::#variant_ident { #(#pattern_bindings,)* })
                    };
                    Ok(quote! {
                        #pattern => {
                            #(#conversions)*
                            Self::#variant_ident { #(#field_values,)* }
                        }
                    })
                }
                Fields::Unnamed(fields) => {
                    let bindings = (0..fields.unnamed.len())
                        .map(|index| format_ident!("field_{index}"))
                        .collect::<Vec<_>>();
                    let pattern_bindings = bindings.clone();
                    let conversions = fields
                        .unnamed
                        .iter()
                        .enumerate()
                        .map(|(index, field)| {
                            let binding = &bindings[index];
                            let converted = convert_valid_value_for_type(
                                &field.ty,
                                quote!(#binding),
                                node_names,
                                aliases,
                            )?;
                            Ok(quote!(let #binding = #converted;))
                        })
                        .collect::<syn::Result<Vec<_>>>()?;
                    let pattern = if uses_data_patterns {
                        quote!(::bityzba::data!(#valid_ty::#variant_ident(#(#pattern_bindings,)*)))
                    } else {
                        quote!(#valid_ty::#variant_ident(#(#pattern_bindings,)*))
                    };
                    Ok(quote! {
                        #pattern => {
                            #(#conversions)*
                            Self::#variant_ident(#(#bindings,)*)
                        }
                    })
                }
                Fields::Unit => {
                    let pattern = if uses_data_patterns {
                        quote!(::bityzba::data!(#valid_ty::#variant_ident))
                    } else {
                        quote!(#valid_ty::#variant_ident)
                    };
                    Ok(quote!(#pattern => Self::#variant_ident))
                }
            }
        })
        .collect::<syn::Result<Vec<_>>>()?;
    let match_value = if uses_data_patterns {
        quote!(value.into_data())
    } else {
        quote!(value)
    };
    let boxed_match_value = if uses_data_patterns {
        quote!((*value).into_data())
    } else {
        quote!(*value)
    };
    Ok(quote! {
        impl #enum_ident {
            pub fn from_valid(value: #valid_ty) -> Self {
                *Self::from_valid_boxed(Box::new(value))
            }

            pub fn from_valid_boxed(value: Box<#valid_ty>) -> Box<Self> {
                Box::new(match #boxed_match_value {
                    #(#arms,)*
                })
            }

            pub(crate) fn from_valid_unboxed(value: #valid_ty) -> Self {
                match #match_value {
                    #(#arms,)*
                }
            }
        }
    })
}

#[requires(true)]
#[ensures(true)]
fn field_binding_ident(index: usize, field: &syn::Field) -> Ident {
    field
        .ident
        .as_ref()
        .map(|ident| format_ident!("converted_{ident}"))
        .unwrap_or_else(|| format_ident!("converted_{index}"))
}

#[requires(true)]
#[ensures(true)]
fn item_needs_new_macro(item: &Item) -> bool {
    match item {
        Item::Struct(item) => attrs_need_new_macro(&item.attrs),
        Item::Enum(item) => attrs_need_new_macro(&item.attrs),
        _ => false,
    }
}

#[requires(true)]
#[ensures(true)]
fn attrs_need_new_macro(attrs: &[Attribute]) -> bool {
    attrs
        .iter()
        .filter(|attr| {
            attr.path().is_ident("invariant") || attr.path().is_ident("expensive_invariant")
        })
        .any(|attr| !attr_is_true_contract_marker(attr))
}

#[requires(true)]
#[ensures(true)]
fn convert_value_for_type(
    ty: &Type,
    expr: proc_macro2::TokenStream,
    node_names: &BTreeSet<String>,
    aliases: &BTreeMap<String, Type>,
) -> syn::Result<proc_macro2::TokenStream> {
    match ty {
        Type::Path(path) if path.qself.is_none() => {
            let Some(last) = path.path.segments.last() else {
                return Ok(convert_recovered_atom(expr));
            };
            if path.path.segments.len() == 1
                && let Some(alias) = aliases.get(&last.ident.to_string())
            {
                return convert_value_for_type(alias, expr, node_names, aliases);
            }
            if is_wrapper_ident(&last.ident) {
                return convert_wrapper_value_for_type(
                    &last.ident,
                    &last.arguments,
                    expr,
                    node_names,
                    aliases,
                );
            }
            if path.path.segments.len() == 1 && node_names.contains(&last.ident.to_string()) {
                Ok(convert_recovered_node(expr))
            } else {
                Ok(convert_recovered_atom(expr))
            }
        }
        Type::Reference(reference) => Err(reference_tree_type_error(reference)),
        Type::Array(array) => {
            convert_array_value_for_type(&array.elem, &array.len, expr, node_names, aliases)
        }
        Type::Tuple(tuple) => convert_tuple_value_for_type(tuple, expr, node_names, aliases),
        _ => Ok(convert_recovered_atom(expr)),
    }
}

#[requires(true)]
#[ensures(true)]
fn convert_wrapper_value_for_type(
    wrapper: &Ident,
    arguments: &PathArguments,
    expr: proc_macro2::TokenStream,
    node_names: &BTreeSet<String>,
    aliases: &BTreeMap<String, Type>,
) -> syn::Result<proc_macro2::TokenStream> {
    let Some(inner) = first_type_argument(arguments) else {
        return Ok(quote!(Ok(#expr)));
    };
    match wrapper.to_string().as_str() {
        "Box" => {
            let inner = convert_value_for_type(inner, quote!(*value), node_names, aliases)?;
            Ok(quote!({
                let value = #expr;
                let value = #inner?;
                Ok(Box::new(value))
            }))
        }
        "Arc" => {
            let inner = convert_value_for_type(inner, quote!(value), node_names, aliases)?;
            Ok(quote!({
                let value = #expr;
                let value = match ::std::sync::Arc::try_unwrap(value) {
                    Ok(value) => value,
                    Err(value) => (*value).clone(),
                };
                let value = #inner?;
                Ok(::std::sync::Arc::new(value))
            }))
        }
        "Option" => {
            let inner = convert_value_for_type(inner, quote!(value), node_names, aliases)?;
            Ok(quote!({
                match #expr {
                    Some(value) => {
                        let value = #inner?;
                        Ok(Some(value))
                    }
                    None => Ok(None),
                }
            }))
        }
        "Vec" => convert_vec_value_for_type(inner, expr, quote!(Vec::new()), node_names, aliases),
        "Vec1" => {
            let converted = convert_vec_value_for_type(
                inner,
                quote!((#expr).into_vec()),
                quote!(Vec::new()),
                node_names,
                aliases,
            )?;
            Ok(quote!({
                let values = #converted?;
                Ok(::vec1::Vec1::try_from_vec(values).expect("recovered Vec1 converted from non-empty Vec1"))
            }))
        }
        "SmallVec" => {
            let inner = smallvec_item_type(inner);
            let converted = convert_vec_value_for_type(
                inner,
                quote!((#expr).into_vec()),
                quote!(Vec::new()),
                node_names,
                aliases,
            )?;
            Ok(quote!({
                let values = #converted?;
                Ok(::smallvec::SmallVec::from_vec(values))
            }))
        }
        "SmallVec1" => {
            let inner = smallvec_item_type(inner);
            let converted = convert_vec_value_for_type(
                inner,
                quote!((#expr).into_vec()),
                quote!(Vec::new()),
                node_names,
                aliases,
            )?;
            Ok(quote!({
                let values = #converted?;
                Ok(::vec1::smallvec_v1::SmallVec1::try_from_vec(values).expect("recovered SmallVec1 converted from non-empty SmallVec1"))
            }))
        }
        "WithFreeModifiers" => {
            let value = convert_value_for_type(inner, quote!(value), node_names, aliases)?;
            let free_modifiers = convert_vec_value_for_type(
                &parse_quote!(FreeModifierSyntax),
                quote!(free_modifiers),
                quote!(Vec::new()),
                node_names,
                aliases,
            )?;
            Ok(quote!({
                let WithFreeModifiers { value, free_modifiers } = #expr;
                let value = #value?;
                let free_modifiers = #free_modifiers?;
                Ok(super::WithFreeModifiers { value, free_modifiers })
            }))
        }
        "Chain" => {
            let links_ty = nth_type_argument(arguments, 1).ok_or_else(|| {
                syn::Error::new_spanned(wrapper, "`Chain` needs first and links type arguments")
            })?;
            let first = convert_value_for_type(inner, quote!(first), node_names, aliases)?;
            let links = convert_value_for_type(links_ty, quote!(links), node_names, aliases)?;
            Ok(quote!({
                let ::jbotci_tree::Chain { first, links } = #expr;
                let first = #first?;
                let links = #links?;
                Ok(::jbotci_tree::Chain { first, links })
            }))
        }
        "Recovered" => convert_value_for_type(inner, expr, node_names, aliases),
        _ => Ok(quote!(Ok(#expr))),
    }
}

#[requires(true)]
#[ensures(true)]
fn smallvec_item_type(inner: &Type) -> &Type {
    if let Type::Array(array) = inner {
        &array.elem
    } else {
        inner
    }
}

#[requires(true)]
#[ensures(true)]
fn convert_vec_value_for_type(
    inner: &Type,
    expr: proc_macro2::TokenStream,
    initial: proc_macro2::TokenStream,
    node_names: &BTreeSet<String>,
    aliases: &BTreeMap<String, Type>,
) -> syn::Result<proc_macro2::TokenStream> {
    let inner_conversion = convert_value_for_type(inner, quote!(value), node_names, aliases)?;
    Ok(quote!({
        let mut converted = #initial;
        for (index, value) in (#expr).into_iter().enumerate() {
            path.push(::jbotci_tree::TreePathStep::sequence_index(index));
            let value = #inner_conversion?;
            path.pop();
            converted.push(value);
        }
        Ok(converted)
    }))
}

#[requires(true)]
#[ensures(true)]
fn convert_array_value_for_type(
    inner: &Type,
    len: &syn::Expr,
    expr: proc_macro2::TokenStream,
    node_names: &BTreeSet<String>,
    aliases: &BTreeMap<String, Type>,
) -> syn::Result<proc_macro2::TokenStream> {
    let converted = convert_vec_value_for_type(
        inner,
        quote!((#expr).into_iter()),
        quote!(Vec::new()),
        node_names,
        aliases,
    )?;
    Ok(quote!({
        let values = #converted?;
        Ok(values
            .try_into()
            .unwrap_or_else(|_| panic!("recovered array conversion must preserve length {}", #len)))
    }))
}

#[requires(true)]
#[ensures(true)]
fn convert_tuple_value_for_type(
    tuple: &syn::TypeTuple,
    expr: proc_macro2::TokenStream,
    node_names: &BTreeSet<String>,
    aliases: &BTreeMap<String, Type>,
) -> syn::Result<proc_macro2::TokenStream> {
    if tuple.elems.is_empty() {
        return Ok(quote!({
            let () = #expr;
            Ok(())
        }));
    }
    let bindings: Vec<_> = (0..tuple.elems.len())
        .map(|index| format_ident!("tuple_{index}"))
        .collect();
    let conversions = tuple
        .elems
        .iter()
        .zip(&bindings)
        .map(|(elem, binding)| {
            let converted = convert_value_for_type(elem, quote!(#binding), node_names, aliases)?;
            Ok(quote! {
                let #binding = #converted?;
            })
        })
        .collect::<syn::Result<Vec<_>>>()?;
    Ok(quote!({
        let (#(#bindings,)*) = #expr;
        #(#conversions)*
        Ok((#(#bindings,)*))
    }))
}

#[requires(true)]
#[ensures(true)]
fn convert_recovered_node(expr: proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    quote! {
        (#expr).try_into_valid_boxed_with(path, |value, path| {
            value.try_into_valid_boxed_at_path(path)
        })
    }
}

#[requires(true)]
#[ensures(true)]
fn convert_recovered_atom(expr: proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    quote! {
        (#expr).try_into_valid_with(path, |value, _path| Ok(value))
    }
}

#[requires(true)]
#[ensures(true)]
fn convert_valid_value_for_type(
    ty: &Type,
    expr: proc_macro2::TokenStream,
    node_names: &BTreeSet<String>,
    aliases: &BTreeMap<String, Type>,
) -> syn::Result<proc_macro2::TokenStream> {
    match ty {
        Type::Path(path) if path.qself.is_none() => {
            let Some(last) = path.path.segments.last() else {
                return Ok(convert_valid_atom(expr));
            };
            if path.path.segments.len() == 1
                && let Some(alias) = aliases.get(&last.ident.to_string())
            {
                return convert_valid_value_for_type(alias, expr, node_names, aliases);
            }
            if is_wrapper_ident(&last.ident) {
                return convert_valid_wrapper_value_for_type(
                    &last.ident,
                    &last.arguments,
                    expr,
                    node_names,
                    aliases,
                );
            }
            if path.path.segments.len() == 1 && node_names.contains(&last.ident.to_string()) {
                Ok(convert_valid_node(&last.ident, expr))
            } else {
                Ok(convert_valid_atom(expr))
            }
        }
        Type::Reference(reference) => Err(reference_tree_type_error(reference)),
        Type::Array(array) => {
            convert_valid_array_value_for_type(&array.elem, &array.len, expr, node_names, aliases)
        }
        Type::Tuple(tuple) => convert_valid_tuple_value_for_type(tuple, expr, node_names, aliases),
        _ => Ok(convert_valid_atom(expr)),
    }
}

#[requires(true)]
#[ensures(true)]
fn convert_valid_wrapper_value_for_type(
    wrapper: &Ident,
    arguments: &PathArguments,
    expr: proc_macro2::TokenStream,
    node_names: &BTreeSet<String>,
    aliases: &BTreeMap<String, Type>,
) -> syn::Result<proc_macro2::TokenStream> {
    let Some(inner) = first_type_argument(arguments) else {
        return Ok(expr);
    };
    match wrapper.to_string().as_str() {
        "Box" => {
            let inner =
                convert_valid_boxed_value_for_type(inner, quote!(value), node_names, aliases)?;
            Ok(quote!({
                let value = #expr;
                Box::new(#inner)
            }))
        }
        "Arc" => {
            let inner = convert_valid_value_for_type(inner, quote!(value), node_names, aliases)?;
            Ok(quote!({
                let value = #expr;
                let value = match ::std::sync::Arc::try_unwrap(value) {
                    Ok(value) => value,
                    Err(value) => (*value).clone(),
                };
                ::std::sync::Arc::new(#inner)
            }))
        }
        "Option" => {
            let inner = convert_valid_value_for_type(inner, quote!(value), node_names, aliases)?;
            Ok(quote!({
                (#expr).map(|value| #inner)
            }))
        }
        "Vec" => {
            convert_valid_vec_value_for_type(inner, expr, quote!(Vec::new()), node_names, aliases)
        }
        "Vec1" => {
            let converted = convert_valid_vec_value_for_type(
                inner,
                quote!((#expr).into_vec()),
                quote!(Vec::new()),
                node_names,
                aliases,
            )?;
            Ok(quote!({
                let values = #converted;
                ::vec1::Vec1::try_from_vec(values).expect("valid Vec1 converted into non-empty recovered Vec1")
            }))
        }
        "SmallVec" => {
            let inner = smallvec_item_type(inner);
            let converted = convert_valid_vec_value_for_type(
                inner,
                quote!((#expr).into_vec()),
                quote!(Vec::new()),
                node_names,
                aliases,
            )?;
            Ok(quote!({
                let values = #converted;
                ::smallvec::SmallVec::from_vec(values)
            }))
        }
        "SmallVec1" => {
            let inner = smallvec_item_type(inner);
            let converted = convert_valid_vec_value_for_type(
                inner,
                quote!((#expr).into_vec()),
                quote!(Vec::new()),
                node_names,
                aliases,
            )?;
            Ok(quote!({
                let values = #converted;
                ::vec1::smallvec_v1::SmallVec1::try_from_vec(values)
                    .expect("valid SmallVec1 converted into non-empty recovered SmallVec1")
            }))
        }
        "WithFreeModifiers" => {
            let value = convert_valid_value_for_type(inner, quote!(value), node_names, aliases)?;
            Ok(quote!({
                let super::WithFreeModifiers { value, free_modifiers } = #expr;
                WithFreeModifiers {
                    value: #value,
                    free_modifiers: free_modifiers
                        .into_iter()
                        .map(FreeModifierSyntax::from_valid)
                        .map(Recovered::valid)
                        .collect(),
                    }
            }))
        }
        "Chain" => {
            let links_ty = nth_type_argument(arguments, 1).ok_or_else(|| {
                syn::Error::new_spanned(wrapper, "`Chain` needs first and links type arguments")
            })?;
            let first = convert_valid_value_for_type(inner, quote!(first), node_names, aliases)?;
            let links = convert_valid_value_for_type(links_ty, quote!(links), node_names, aliases)?;
            Ok(quote!({
                let ::jbotci_tree::Chain { first, links } = #expr;
                let first = #first;
                let links = #links;
                ::jbotci_tree::Chain { first, links }
            }))
        }
        "Recovered" => convert_valid_value_for_type(inner, expr, node_names, aliases),
        _ => Ok(expr),
    }
}

#[requires(true)]
#[ensures(true)]
fn convert_valid_boxed_value_for_type(
    ty: &Type,
    expr: proc_macro2::TokenStream,
    node_names: &BTreeSet<String>,
    aliases: &BTreeMap<String, Type>,
) -> syn::Result<proc_macro2::TokenStream> {
    match ty {
        Type::Path(path) if path.qself.is_none() => {
            let Some(last) = path.path.segments.last() else {
                let inner = convert_valid_value_for_type(ty, quote!(*value), node_names, aliases)?;
                return Ok(quote!({
                    let value = #expr;
                    #inner
                }));
            };
            if path.path.segments.len() == 1
                && let Some(alias) = aliases.get(&last.ident.to_string())
            {
                return convert_valid_boxed_value_for_type(alias, expr, node_names, aliases);
            }
            if path.path.segments.len() == 1 && node_names.contains(&last.ident.to_string()) {
                return Ok(convert_valid_boxed_node(&last.ident, expr));
            }
        }
        Type::Reference(reference) => {
            return convert_valid_boxed_value_for_type(&reference.elem, expr, node_names, aliases);
        }
        _ => {}
    }
    let inner = convert_valid_value_for_type(ty, quote!(*value), node_names, aliases)?;
    Ok(quote!({
        let value = #expr;
        #inner
    }))
}

#[requires(true)]
#[ensures(true)]
fn convert_valid_vec_value_for_type(
    inner: &Type,
    expr: proc_macro2::TokenStream,
    initial: proc_macro2::TokenStream,
    node_names: &BTreeSet<String>,
    aliases: &BTreeMap<String, Type>,
) -> syn::Result<proc_macro2::TokenStream> {
    let inner_conversion = convert_valid_value_for_type(inner, quote!(value), node_names, aliases)?;
    Ok(quote!({
        let mut converted = #initial;
        for value in (#expr).into_iter() {
            converted.push(#inner_conversion);
        }
        converted
    }))
}

#[requires(true)]
#[ensures(true)]
fn convert_valid_array_value_for_type(
    inner: &Type,
    len: &syn::Expr,
    expr: proc_macro2::TokenStream,
    node_names: &BTreeSet<String>,
    aliases: &BTreeMap<String, Type>,
) -> syn::Result<proc_macro2::TokenStream> {
    let converted = convert_valid_vec_value_for_type(
        inner,
        quote!((#expr).into_iter()),
        quote!(Vec::new()),
        node_names,
        aliases,
    )?;
    Ok(quote!({
        let values = #converted;
        values
            .try_into()
            .unwrap_or_else(|_| panic!("valid array conversion must preserve length {}", #len))
    }))
}

#[requires(true)]
#[ensures(true)]
fn convert_valid_tuple_value_for_type(
    tuple: &syn::TypeTuple,
    expr: proc_macro2::TokenStream,
    node_names: &BTreeSet<String>,
    aliases: &BTreeMap<String, Type>,
) -> syn::Result<proc_macro2::TokenStream> {
    if tuple.elems.is_empty() {
        return Ok(quote!({
            let () = #expr;
            ()
        }));
    }
    let bindings: Vec<_> = (0..tuple.elems.len())
        .map(|index| format_ident!("tuple_{index}"))
        .collect();
    let conversions = tuple
        .elems
        .iter()
        .zip(&bindings)
        .map(|(elem, binding)| {
            let converted =
                convert_valid_value_for_type(elem, quote!(#binding), node_names, aliases)?;
            Ok(quote!(#converted))
        })
        .collect::<syn::Result<Vec<_>>>()?;
    Ok(quote!({
        let (#(#bindings,)*) = #expr;
        (#(#conversions,)*)
    }))
}

#[requires(true)]
#[ensures(true)]
fn convert_valid_node(ident: &Ident, expr: proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    quote!(Recovered::valid(#ident::from_valid(#expr)))
}

#[requires(true)]
#[ensures(true)]
fn convert_valid_boxed_node(
    ident: &Ident,
    expr: proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    quote!(Recovered::valid_boxed(#ident::from_valid_boxed(#expr)))
}

#[requires(true)]
#[ensures(true)]
fn convert_valid_atom(expr: proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    quote!(Recovered::valid(#expr))
}

#[requires(true)]
#[ensures(true)]
fn items_use_wrapper(items: &[Item], wrapper: &str) -> bool {
    items.iter().any(|item| match item {
        Item::Struct(item) => fields_use_wrapper(&item.fields, wrapper),
        Item::Enum(item) => item
            .variants
            .iter()
            .any(|variant| fields_use_wrapper(&variant.fields, wrapper)),
        Item::Type(item) => type_uses_wrapper(&item.ty, wrapper),
        _ => false,
    })
}

#[requires(true)]
#[ensures(true)]
fn fields_use_wrapper(fields: &Fields, wrapper: &str) -> bool {
    fields
        .iter()
        .any(|field| type_uses_wrapper(&field.ty, wrapper))
}

#[requires(true)]
#[ensures(true)]
fn type_uses_wrapper(ty: &Type, wrapper: &str) -> bool {
    match ty {
        Type::Path(path) => {
            if path
                .path
                .segments
                .iter()
                .any(|segment| segment.ident == wrapper)
            {
                return true;
            }
            path.path.segments.iter().any(|segment| {
                let PathArguments::AngleBracketed(arguments) = &segment.arguments else {
                    return false;
                };
                arguments.args.iter().any(|argument| match argument {
                    GenericArgument::Type(ty) => type_uses_wrapper(ty, wrapper),
                    _ => false,
                })
            })
        }
        Type::Reference(reference) => type_uses_wrapper(&reference.elem, wrapper),
        Type::Array(array) => type_uses_wrapper(&array.elem, wrapper),
        _ => false,
    }
}

#[requires(true)]
#[ensures(true)]
fn strip_tree_attrs_from_fields(fields: &mut Fields) -> syn::Result<()> {
    for field in fields {
        let mut checked = Vec::new();
        for attr in &field.attrs {
            if attr.path().is_ident("tree_child") {
                tree_child_flags(std::slice::from_ref(attr))?;
            } else {
                checked.push(attr.clone());
            }
        }
        field.attrs = checked;
    }
    Ok(())
}

#[requires(true)]
#[ensures(true)]
fn node_ref_enum(items: &[Item]) -> syn::Result<proc_macro2::TokenStream> {
    let variants = items.iter().flat_map(|item| match item {
        Item::Struct(item) => vec![node_ref_struct_variant(item)],
        Item::Enum(item) => node_ref_enum_variants(item),
        _ => Vec::new(),
    });
    let constructor_arms = items.iter().flat_map(|item| match item {
        Item::Struct(item) => vec![node_ref_struct_constructor_arm(item)],
        Item::Enum(item) => node_ref_enum_constructor_arms(item),
        _ => Vec::new(),
    });
    let is_variant_arms = items.iter().flat_map(|item| match item {
        Item::Struct(item) => vec![node_ref_struct_is_variant_arm(item)],
        Item::Enum(item) => node_ref_enum_is_variant_arms(item),
        _ => Vec::new(),
    });
    let equality_arms = node_ref_equality_arms(items);
    let hash_arms = node_ref_hash_arms(items);
    let from_impls = node_ref_from_impls(items)?;
    Ok(quote! {
        #[derive(Clone, Copy, Debug)]
        pub enum NodeRef<'tree> {
            #(#variants,)*
        }

        impl NodeRef<'_> {
            pub fn constructor_name(self) -> &'static str {
                match self {
                    #(#constructor_arms,)*
                }
            }

            pub fn is_variant(self) -> bool {
                match self {
                    #(#is_variant_arms,)*
                }
            }
        }

        impl ::core::cmp::PartialEq for NodeRef<'_> {
            fn eq(&self, other: &Self) -> bool {
                match (*self, *other) {
                    #(#equality_arms,)*
                    _ => false,
                }
            }
        }

        impl ::core::cmp::Eq for NodeRef<'_> {}

        impl ::core::hash::Hash for NodeRef<'_> {
            fn hash<H>(&self, state: &mut H)
            where
                H: ::core::hash::Hasher,
            {
                match *self {
                    #(#hash_arms,)*
                }
            }
        }

        #(#from_impls)*
    })
}

#[requires(true)]
#[ensures(true)]
fn node_ref_struct_variant(item: &ItemStruct) -> proc_macro2::TokenStream {
    let ident = &item.ident;
    quote!(#ident(&'tree #ident))
}

#[requires(true)]
#[ensures(ret.len() == item.variants.len())]
fn node_ref_enum_variants(item: &ItemEnum) -> Vec<proc_macro2::TokenStream> {
    let enum_ident = &item.ident;
    item.variants
        .iter()
        .map(|variant| {
            let variant_ident = node_ref_variant_ident(enum_ident, &variant.ident);
            quote!(#variant_ident(&'tree #enum_ident))
        })
        .collect()
}

#[requires(true)]
#[ensures(true)]
fn node_ref_struct_constructor_arm(item: &ItemStruct) -> proc_macro2::TokenStream {
    let ident = &item.ident;
    let constructor = ident.to_string();
    quote!(NodeRef::#ident(..) => #constructor)
}

#[requires(true)]
#[ensures(ret.len() == item.variants.len())]
fn node_ref_enum_constructor_arms(item: &ItemEnum) -> Vec<proc_macro2::TokenStream> {
    let enum_ident = &item.ident;
    item.variants
        .iter()
        .map(|variant| {
            let variant_ident = node_ref_variant_ident(enum_ident, &variant.ident);
            let constructor = variant.ident.to_string();
            quote!(NodeRef::#variant_ident(..) => #constructor)
        })
        .collect()
}

#[requires(true)]
#[ensures(true)]
fn node_ref_struct_is_variant_arm(item: &ItemStruct) -> proc_macro2::TokenStream {
    let ident = &item.ident;
    quote!(NodeRef::#ident(..) => false)
}

#[requires(true)]
#[ensures(ret.len() == item.variants.len())]
fn node_ref_enum_is_variant_arms(item: &ItemEnum) -> Vec<proc_macro2::TokenStream> {
    let enum_ident = &item.ident;
    item.variants
        .iter()
        .map(|variant| {
            let variant_ident = node_ref_variant_ident(enum_ident, &variant.ident);
            quote!(NodeRef::#variant_ident(..) => true)
        })
        .collect()
}

#[requires(true)]
#[ensures(true)]
fn node_ref_equality_arms(items: &[Item]) -> Vec<proc_macro2::TokenStream> {
    node_ref_variant_idents(items)
        .into_iter()
        .map(|ident| {
            quote! {
                (NodeRef::#ident(left), NodeRef::#ident(right)) => ::core::ptr::eq(left, right)
            }
        })
        .collect()
}

#[requires(true)]
#[ensures(true)]
fn node_ref_hash_arms(items: &[Item]) -> Vec<proc_macro2::TokenStream> {
    node_ref_variant_idents(items)
        .into_iter()
        .enumerate()
        .map(|(tag, ident)| {
            quote! {
                NodeRef::#ident(node) => {
                    ::core::hash::Hash::hash(&#tag, state);
                    ::core::hash::Hash::hash(&(node as *const _ as usize), state);
                }
            }
        })
        .collect()
}

#[requires(true)]
#[ensures(true)]
fn node_ref_from_impls(items: &[Item]) -> syn::Result<Vec<proc_macro2::TokenStream>> {
    items
        .iter()
        .filter_map(|item| match item {
            Item::Struct(item) => Some(Ok(node_ref_struct_from_impl(item))),
            Item::Enum(item) => Some(node_ref_enum_from_impl(item)),
            _ => None,
        })
        .collect()
}

#[requires(true)]
#[ensures(true)]
fn node_ref_struct_from_impl(item: &ItemStruct) -> proc_macro2::TokenStream {
    let ident = &item.ident;
    quote! {
        impl<'tree> ::core::convert::From<&'tree #ident> for NodeRef<'tree> {
            fn from(node: &'tree #ident) -> Self {
                NodeRef::#ident(node)
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn node_ref_enum_from_impl(item: &ItemEnum) -> syn::Result<proc_macro2::TokenStream> {
    let enum_ident = &item.ident;
    let uses_data_patterns = enum_uses_data_patterns(item);
    let arms = item
        .variants
        .iter()
        .map(|variant| {
            let variant_ident = &variant.ident;
            let node_ref_variant = node_ref_variant_ident(enum_ident, variant_ident);
            let pattern = enum_variant_wildcard_pattern(
                enum_ident,
                variant_ident,
                &variant.fields,
                uses_data_patterns,
            )?;
            Ok(quote! {
                #pattern => NodeRef::#node_ref_variant(node),
            })
        })
        .collect::<syn::Result<Vec<_>>>()?;
    let match_value = if uses_data_patterns {
        quote!(node.as_data())
    } else {
        quote!(node)
    };
    Ok(quote! {
        impl<'tree> ::core::convert::From<&'tree #enum_ident> for NodeRef<'tree> {
            fn from(node: &'tree #enum_ident) -> Self {
                match #match_value {
                    #(#arms)*
                }
            }
        }
    })
}

#[requires(true)]
#[ensures(true)]
fn enum_variant_wildcard_pattern(
    enum_ident: &Ident,
    variant_ident: &Ident,
    fields: &Fields,
    uses_data_patterns: bool,
) -> syn::Result<proc_macro2::TokenStream> {
    let pattern = match fields {
        Fields::Named(fields) => {
            let fields = fields
                .named
                .iter()
                .map(|field| {
                    let ident = field.ident.as_ref().ok_or_else(|| {
                        syn::Error::new_spanned(field, "named field is missing an identifier")
                    })?;
                    Ok(quote!(#ident: _))
                })
                .collect::<syn::Result<Vec<_>>>()?;
            quote!(#enum_ident::#variant_ident { #(#fields,)* })
        }
        Fields::Unnamed(fields) => {
            let fields = fields.unnamed.iter().map(|_| quote!(_));
            quote!(#enum_ident::#variant_ident(#(#fields,)*))
        }
        Fields::Unit => quote!(#enum_ident::#variant_ident),
    };
    if uses_data_patterns {
        Ok(quote!(::bityzba::data!(#pattern)))
    } else {
        Ok(pattern)
    }
}

#[requires(true)]
#[ensures(true)]
fn node_ref_variant_idents(items: &[Item]) -> Vec<Ident> {
    items
        .iter()
        .flat_map(|item| match item {
            Item::Struct(item) => vec![item.ident.clone()],
            Item::Enum(item) => {
                let enum_ident = &item.ident;
                item.variants
                    .iter()
                    .map(|variant| node_ref_variant_ident(enum_ident, &variant.ident))
                    .collect()
            }
            _ => Vec::new(),
        })
        .collect()
}

#[requires(true)]
#[ensures(!ret.to_string().is_empty())]
fn node_ref_variant_ident(enum_ident: &Ident, variant_ident: &Ident) -> Ident {
    format_ident!("{enum_ident}{variant_ident}")
}

#[requires(true)]
#[ensures(true)]
fn atom_ref_enum(atom_types: &BTreeMap<String, Type>) -> proc_macro2::TokenStream {
    let variants = atom_types.values().map(|ty| {
        let ident = atom_variant_ident(ty);
        quote!(#ident(&'tree #ty))
    });
    quote! {
        #[derive(Clone, Copy, Debug)]
        pub enum AtomRef<'tree> {
            #(#variants,)*
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn atom_trait_impls(atom_types: &BTreeMap<String, Type>) -> proc_macro2::TokenStream {
    let impls = atom_types.values().map(|ty| {
        let variant = atom_variant_ident(ty);
        quote! {
            impl TreeNode for #ty {
                fn visit_in_order<'tree, V>(&'tree self, visitor: &mut V)
                where
                    V: ::jbotci_tree::TreeVisitor<'tree, Node = NodeRef<'tree>, Atom = AtomRef<'tree>>,
                {
                    visitor.visit_atom(AtomRef::#variant(self));
                }

                fn path_to_node_from<'tree>(
                    &'tree self,
                    _target: NodeRef<'tree>,
                    _path: &mut ::jbotci_tree::TreePath,
                ) -> bool {
                    false
                }

                fn node_at_path_steps<'tree>(
                    &'tree self,
                    _steps: &[::jbotci_tree::TreePathStep],
                ) -> Option<NodeRef<'tree>> {
                    None
                }
            }
        }
    });
    quote!(#(#impls)*)
}

#[requires(true)]
#[ensures(!ret.to_string().is_empty())]
fn atom_variant_ident(ty: &Type) -> Ident {
    let mut text = String::new();
    for ch in quote!(#ty).to_string().chars() {
        if ch.is_ascii_alphanumeric() {
            text.push(ch);
        } else {
            write!(&mut text, "_x{:X}_", ch as u32).expect("writing to String should not fail");
        }
    }
    if text.is_empty() {
        text = "Atom".to_owned();
    }
    if text.chars().next().is_some_and(|ch| ch.is_ascii_digit()) {
        text.insert_str(0, "Atom");
    }
    format_ident!("{text}")
}

#[requires(true)]
#[ensures(true)]
fn wrapper_trait_impls(
    include_recovered: bool,
    include_with_free_modifiers: bool,
) -> proc_macro2::TokenStream {
    let recovered_impl = include_recovered.then(|| {
        quote! {
            impl<T: TreeNode> TreeNode for Recovered<T> {
                fn as_node_ref<'tree>(&'tree self) -> Option<NodeRef<'tree>> {
                    match self {
                        ::jbotci_tree::Recovered::Valid(value) => value.as_node_ref(),
                        ::jbotci_tree::Recovered::Error(_) => None,
                        ::jbotci_tree::Recovered::Prefix(prefix) => prefix.value.as_node_ref(),
                    }
                }

                fn visit_in_order<'tree, V>(&'tree self, visitor: &mut V)
                where
                    V: ::jbotci_tree::TreeVisitor<'tree, Node = NodeRef<'tree>, Atom = AtomRef<'tree>>,
                {
                    match self {
                        ::jbotci_tree::Recovered::Valid(value) => value.visit_in_order(visitor),
                        ::jbotci_tree::Recovered::Error(item) => visitor.visit_recovered_error(item),
                        ::jbotci_tree::Recovered::Prefix(prefix) => {
                            for item in &prefix.errors {
                                visitor.visit_recovered_error(item);
                            }
                            prefix.value.visit_in_order(visitor);
                        }
                    }
                }

                fn path_to_node_from<'tree>(
                    &'tree self,
                    target: NodeRef<'tree>,
                    path: &mut ::jbotci_tree::TreePath,
                ) -> bool {
                    match self {
                        ::jbotci_tree::Recovered::Valid(value) => {
                            value.path_to_node_from(target, path)
                        }
                        ::jbotci_tree::Recovered::Error(_) => false,
                        ::jbotci_tree::Recovered::Prefix(prefix) => {
                            prefix.value.path_to_node_from(target, path)
                        }
                    }
                }

                fn node_at_path_steps<'tree>(
                    &'tree self,
                    steps: &[::jbotci_tree::TreePathStep],
                ) -> Option<NodeRef<'tree>> {
                    match self {
                        ::jbotci_tree::Recovered::Valid(value) => value.node_at_path_steps(steps),
                        ::jbotci_tree::Recovered::Error(_) => None,
                        ::jbotci_tree::Recovered::Prefix(prefix) => {
                            prefix.value.node_at_path_steps(steps)
                        }
                    }
                }
            }
        }
    });
    let with_free_modifiers_impl = include_with_free_modifiers.then(|| {
        let impl_header = if include_recovered {
            quote!(impl<T: TreeNode> TreeNode for WithFreeModifiers<T>)
        } else {
            quote!(impl<T: TreeNode, F: TreeNode> TreeNode for WithFreeModifiers<T, F>)
        };
        quote! {
            #impl_header {
                fn as_node_ref<'tree>(&'tree self) -> Option<NodeRef<'tree>> {
                    self.value.as_node_ref()
                }

                fn visit_in_order<'tree, V>(&'tree self, visitor: &mut V)
                where
                    V: ::jbotci_tree::TreeVisitor<'tree, Node = NodeRef<'tree>, Atom = AtomRef<'tree>>,
                {
                    self.value.visit_in_order(visitor);
                    if !self.free_modifiers.is_empty() {
                        let field_ref = ::jbotci_tree::FieldRef::new(Some("free_modifiers"), 1, false);
                        visitor.enter_field(field_ref);
                        self.free_modifiers.visit_in_order(visitor);
                        visitor.exit_field(field_ref);
                    }
                }

                fn path_to_node_from<'tree>(
                    &'tree self,
                    target: NodeRef<'tree>,
                    path: &mut ::jbotci_tree::TreePath,
                ) -> bool {
                    if self.value.path_to_node_from(target, path) {
                        return true;
                    }
                    if !self.free_modifiers.is_empty() {
                        path.push(::jbotci_tree::TreePathStep::field(Some("free_modifiers"), 1));
                        if self.free_modifiers.path_to_node_from(target, path) {
                            return true;
                        }
                        path.pop();
                    }
                    false
                }

                fn node_at_path_steps<'tree>(
                    &'tree self,
                    steps: &[::jbotci_tree::TreePathStep],
                ) -> Option<NodeRef<'tree>> {
                    if let Some(node) = self.value.node_at_path_steps(steps) {
                        return Some(node);
                    }
                    if let Some((step, rest)) = steps.split_first()
                        && step.is_field(Some("free_modifiers"), 1)
                    {
                        return self.free_modifiers.node_at_path_steps(rest);
                    }
                    None
                }
            }
        }
    });
    quote! {
        #recovered_impl
        #with_free_modifiers_impl

        impl<T: TreeNode + ?Sized> TreeNode for Box<T> {
            fn as_node_ref<'tree>(&'tree self) -> Option<NodeRef<'tree>> {
                (**self).as_node_ref()
            }

            fn visit_in_order<'tree, V>(&'tree self, visitor: &mut V)
            where
                V: ::jbotci_tree::TreeVisitor<'tree, Node = NodeRef<'tree>, Atom = AtomRef<'tree>>,
            {
                (**self).visit_in_order(visitor);
            }

            fn path_to_node_from<'tree>(
                &'tree self,
                target: NodeRef<'tree>,
                path: &mut ::jbotci_tree::TreePath,
            ) -> bool {
                (**self).path_to_node_from(target, path)
            }

            fn node_at_path_steps<'tree>(
                &'tree self,
                steps: &[::jbotci_tree::TreePathStep],
            ) -> Option<NodeRef<'tree>> {
                (**self).node_at_path_steps(steps)
            }
        }

        impl<T: TreeNode + ?Sized> TreeNode for ::std::sync::Arc<T> {
            fn as_node_ref<'tree>(&'tree self) -> Option<NodeRef<'tree>> {
                (**self).as_node_ref()
            }

            fn visit_in_order<'tree, V>(&'tree self, visitor: &mut V)
            where
                V: ::jbotci_tree::TreeVisitor<'tree, Node = NodeRef<'tree>, Atom = AtomRef<'tree>>,
            {
                (**self).visit_in_order(visitor);
            }

            fn path_to_node_from<'tree>(
                &'tree self,
                target: NodeRef<'tree>,
                path: &mut ::jbotci_tree::TreePath,
            ) -> bool {
                (**self).path_to_node_from(target, path)
            }

            fn node_at_path_steps<'tree>(
                &'tree self,
                steps: &[::jbotci_tree::TreePathStep],
            ) -> Option<NodeRef<'tree>> {
                (**self).node_at_path_steps(steps)
            }
        }

        impl<T: TreeNode> TreeNode for Option<T> {
            fn as_node_ref<'tree>(&'tree self) -> Option<NodeRef<'tree>> {
                self.as_ref().and_then(TreeNode::as_node_ref)
            }

            fn visit_in_order<'tree, V>(&'tree self, visitor: &mut V)
            where
                V: ::jbotci_tree::TreeVisitor<'tree, Node = NodeRef<'tree>, Atom = AtomRef<'tree>>,
            {
                if let Some(value) = self {
                    value.visit_in_order(visitor);
                }
            }

            fn path_to_node_from<'tree>(
                &'tree self,
                target: NodeRef<'tree>,
                path: &mut ::jbotci_tree::TreePath,
            ) -> bool {
                self.as_ref()
                    .is_some_and(|value| value.path_to_node_from(target, path))
            }

            fn node_at_path_steps<'tree>(
                &'tree self,
                steps: &[::jbotci_tree::TreePathStep],
            ) -> Option<NodeRef<'tree>> {
                self.as_ref()
                    .and_then(|value| value.node_at_path_steps(steps))
            }
        }

        impl<A: TreeNode, B: TreeNode> TreeNode for (A, B) {
            fn visit_in_order<'tree, V>(&'tree self, visitor: &mut V)
            where
                V: ::jbotci_tree::TreeVisitor<'tree, Node = NodeRef<'tree>, Atom = AtomRef<'tree>>,
            {
                visitor.enter_sequence();
                self.0.visit_in_order(visitor);
                self.1.visit_in_order(visitor);
                visitor.exit_sequence();
            }

            fn path_to_node_from<'tree>(
                &'tree self,
                target: NodeRef<'tree>,
                path: &mut ::jbotci_tree::TreePath,
            ) -> bool {
                path.push(::jbotci_tree::TreePathStep::sequence_index(0));
                if self.0.path_to_node_from(target, path) {
                    return true;
                }
                path.pop();
                path.push(::jbotci_tree::TreePathStep::sequence_index(1));
                if self.1.path_to_node_from(target, path) {
                    return true;
                }
                path.pop();
                false
            }

            fn node_at_path_steps<'tree>(
                &'tree self,
                steps: &[::jbotci_tree::TreePathStep],
            ) -> Option<NodeRef<'tree>> {
                let (step, rest) = steps.split_first()?;
                match step.as_sequence_index()? {
                    0 => self.0.node_at_path_steps(rest),
                    1 => self.1.node_at_path_steps(rest),
                    _ => None,
                }
            }
        }

        impl<E: TreeNode, L: TreeNode> TreeNode for ::jbotci_tree::Chain<E, Vec<L>> {
            fn visit_in_order<'tree, V>(&'tree self, visitor: &mut V)
            where
                V: ::jbotci_tree::TreeVisitor<'tree, Node = NodeRef<'tree>, Atom = AtomRef<'tree>>,
            {
                visitor.enter_chain();
                self.first.visit_in_order(visitor);
                for link in &self.links {
                    link.visit_in_order(visitor);
                }
                visitor.exit_chain();
            }

            fn path_to_node_from<'tree>(
                &'tree self,
                target: NodeRef<'tree>,
                path: &mut ::jbotci_tree::TreePath,
            ) -> bool {
                path.push(::jbotci_tree::TreePathStep::sequence_index(0));
                if self.first.path_to_node_from(target, path) {
                    return true;
                }
                path.pop();
                for (index, link) in self.links.iter().enumerate() {
                    path.push(::jbotci_tree::TreePathStep::sequence_index(index + 1));
                    if link.path_to_node_from(target, path) {
                        return true;
                    }
                    path.pop();
                }
                false
            }

            fn node_at_path_steps<'tree>(
                &'tree self,
                steps: &[::jbotci_tree::TreePathStep],
            ) -> Option<NodeRef<'tree>> {
                let (step, rest) = steps.split_first()?;
                let index = step.as_sequence_index()?;
                if index == 0 {
                    self.first.node_at_path_steps(rest)
                } else {
                    self.links.iter().nth(index - 1)?.node_at_path_steps(rest)
                }
            }
        }

        impl<E: TreeNode, L: TreeNode> TreeNode for ::jbotci_tree::Chain<E, ::vec1::Vec1<L>> {
            fn visit_in_order<'tree, V>(&'tree self, visitor: &mut V)
            where
                V: ::jbotci_tree::TreeVisitor<'tree, Node = NodeRef<'tree>, Atom = AtomRef<'tree>>,
            {
                visitor.enter_chain();
                self.first.visit_in_order(visitor);
                for link in &self.links {
                    link.visit_in_order(visitor);
                }
                visitor.exit_chain();
            }

            fn path_to_node_from<'tree>(
                &'tree self,
                target: NodeRef<'tree>,
                path: &mut ::jbotci_tree::TreePath,
            ) -> bool {
                path.push(::jbotci_tree::TreePathStep::sequence_index(0));
                if self.first.path_to_node_from(target, path) {
                    return true;
                }
                path.pop();
                for (index, link) in self.links.iter().enumerate() {
                    path.push(::jbotci_tree::TreePathStep::sequence_index(index + 1));
                    if link.path_to_node_from(target, path) {
                        return true;
                    }
                    path.pop();
                }
                false
            }

            fn node_at_path_steps<'tree>(
                &'tree self,
                steps: &[::jbotci_tree::TreePathStep],
            ) -> Option<NodeRef<'tree>> {
                let (step, rest) = steps.split_first()?;
                let index = step.as_sequence_index()?;
                if index == 0 {
                    self.first.node_at_path_steps(rest)
                } else {
                    self.links.iter().nth(index - 1)?.node_at_path_steps(rest)
                }
            }
        }

        impl<T: TreeNode> TreeNode for Vec<T> {
            fn visit_in_order<'tree, V>(&'tree self, visitor: &mut V)
            where
                V: ::jbotci_tree::TreeVisitor<'tree, Node = NodeRef<'tree>, Atom = AtomRef<'tree>>,
            {
                visitor.enter_sequence();
                for value in self {
                    value.visit_in_order(visitor);
                }
                visitor.exit_sequence();
            }

            fn path_to_node_from<'tree>(
                &'tree self,
                target: NodeRef<'tree>,
                path: &mut ::jbotci_tree::TreePath,
            ) -> bool {
                for (index, value) in self.iter().enumerate() {
                    path.push(::jbotci_tree::TreePathStep::sequence_index(index));
                    if value.path_to_node_from(target, path) {
                        return true;
                    }
                    path.pop();
                }
                false
            }

            fn node_at_path_steps<'tree>(
                &'tree self,
                steps: &[::jbotci_tree::TreePathStep],
            ) -> Option<NodeRef<'tree>> {
                let (step, rest) = steps.split_first()?;
                let index = step.as_sequence_index()?;
                self.get(index)?.node_at_path_steps(rest)
            }
        }

        impl<T: TreeNode> TreeNode for ::vec1::Vec1<T> {
            fn visit_in_order<'tree, V>(&'tree self, visitor: &mut V)
            where
                V: ::jbotci_tree::TreeVisitor<'tree, Node = NodeRef<'tree>, Atom = AtomRef<'tree>>,
            {
                visitor.enter_sequence();
                for value in self {
                    value.visit_in_order(visitor);
                }
                visitor.exit_sequence();
            }

            fn path_to_node_from<'tree>(
                &'tree self,
                target: NodeRef<'tree>,
                path: &mut ::jbotci_tree::TreePath,
            ) -> bool {
                for (index, value) in self.iter().enumerate() {
                    path.push(::jbotci_tree::TreePathStep::sequence_index(index));
                    if value.path_to_node_from(target, path) {
                        return true;
                    }
                    path.pop();
                }
                false
            }

            fn node_at_path_steps<'tree>(
                &'tree self,
                steps: &[::jbotci_tree::TreePathStep],
            ) -> Option<NodeRef<'tree>> {
                let (step, rest) = steps.split_first()?;
                let index = step.as_sequence_index()?;
                self.get(index)?.node_at_path_steps(rest)
            }
        }

        impl<A> TreeNode for ::smallvec::SmallVec<A>
        where
            A: ::smallvec::Array,
            A::Item: TreeNode,
        {
            fn visit_in_order<'tree, V>(&'tree self, visitor: &mut V)
            where
                V: ::jbotci_tree::TreeVisitor<'tree, Node = NodeRef<'tree>, Atom = AtomRef<'tree>>,
            {
                visitor.enter_sequence();
                for value in self {
                    value.visit_in_order(visitor);
                }
                visitor.exit_sequence();
            }

            fn path_to_node_from<'tree>(
                &'tree self,
                target: NodeRef<'tree>,
                path: &mut ::jbotci_tree::TreePath,
            ) -> bool {
                for (index, value) in self.iter().enumerate() {
                    path.push(::jbotci_tree::TreePathStep::sequence_index(index));
                    if value.path_to_node_from(target, path) {
                        return true;
                    }
                    path.pop();
                }
                false
            }

            fn node_at_path_steps<'tree>(
                &'tree self,
                steps: &[::jbotci_tree::TreePathStep],
            ) -> Option<NodeRef<'tree>> {
                let (step, rest) = steps.split_first()?;
                let index = step.as_sequence_index()?;
                self.get(index)?.node_at_path_steps(rest)
            }
        }

        impl<A> TreeNode for ::vec1::smallvec_v1::SmallVec1<A>
        where
            A: ::smallvec::Array,
            A::Item: TreeNode,
        {
            fn visit_in_order<'tree, V>(&'tree self, visitor: &mut V)
            where
                V: ::jbotci_tree::TreeVisitor<'tree, Node = NodeRef<'tree>, Atom = AtomRef<'tree>>,
            {
                visitor.enter_sequence();
                for value in self {
                    value.visit_in_order(visitor);
                }
                visitor.exit_sequence();
            }

            fn path_to_node_from<'tree>(
                &'tree self,
                target: NodeRef<'tree>,
                path: &mut ::jbotci_tree::TreePath,
            ) -> bool {
                for (index, value) in self.iter().enumerate() {
                    path.push(::jbotci_tree::TreePathStep::sequence_index(index));
                    if value.path_to_node_from(target, path) {
                        return true;
                    }
                    path.pop();
                }
                false
            }

            fn node_at_path_steps<'tree>(
                &'tree self,
                steps: &[::jbotci_tree::TreePathStep],
            ) -> Option<NodeRef<'tree>> {
                let (step, rest) = steps.split_first()?;
                let index = step.as_sequence_index()?;
                self.get(index)?.node_at_path_steps(rest)
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn walk_api(
    items: &[Item],
    atom_types: &BTreeMap<String, Type>,
    include_recovered: bool,
    include_with_free_modifiers: bool,
) -> syn::Result<proc_macro2::TokenStream> {
    let trait_methods = tree_walker_trait_methods(items)?;
    let recovered_error_method = include_recovered.then(|| {
        quote! {
            fn walk_recovered_error(&mut self, _item: &'tree super::RecoveryTreeItem) {}
        }
    });
    let walk_module = walk_module(items, include_recovered, include_with_free_modifiers)?;
    let walkable_impls = tree_walkable_impls(
        items,
        atom_types,
        include_recovered,
        include_with_free_modifiers,
    )?;
    Ok(quote! {
        /// Recursive, grammar-directed visitor generated from the tree model.
        ///
        /// Default methods descend through children in the same field order as
        /// `TreeNode::visit_in_order`. Override a node or enum-variant method
        /// to run pass-specific logic before, after, or around the generated
        /// descent, and call the matching `walk::*` free function when default
        /// descent should still run.
        pub trait TreeWalker<'tree> {
            fn walk_atom(&mut self, _atom: AtomRef<'tree>) {}

            #recovered_error_method
            #(#trait_methods)*
        }

        /// Types that can dispatch themselves into a generated `TreeWalker`.
        pub trait TreeWalkable<'tree> {
            fn walk_with<W>(&'tree self, walker: &mut W)
            where
                W: TreeWalker<'tree> + ?Sized;
        }

        #walk_module
        #walkable_impls
    })
}

#[requires(true)]
#[ensures(true)]
fn tree_walker_trait_methods(items: &[Item]) -> syn::Result<Vec<proc_macro2::TokenStream>> {
    items
        .iter()
        .map(|item| match item {
            Item::Struct(item) => {
                let ident = &item.ident;
                let method = walk_method_ident_for_type(ident);
                let function = walk_function_ident_for_type(ident);
                Ok(vec![quote! {
                    fn #method(&mut self, node: &'tree #ident) {
                        walk::#function(self, node);
                    }
                }])
            }
            Item::Enum(item) => {
                let enum_ident = &item.ident;
                let enum_method = walk_method_ident_for_type(enum_ident);
                let enum_function = walk_function_ident_for_type(enum_ident);
                let mut methods = vec![quote! {
                    fn #enum_method(&mut self, node: &'tree #enum_ident) {
                        walk::#enum_function(self, node);
                    }
                }];
                methods.extend(item.variants.iter().map(|variant| {
                    let method = walk_method_ident_for_variant(enum_ident, &variant.ident);
                    let function = walk_function_ident_for_variant(enum_ident, &variant.ident);
                    let params = enum_variant_payload_params(&variant.fields);
                    let args = enum_variant_payload_bindings(&variant.fields);
                    quote! {
                        fn #method(&mut self #(, #params)*) {
                            walk::#function(self #(, #args)*);
                        }
                    }
                }));
                Ok(methods)
            }
            Item::Type(_) => Ok(Vec::new()),
            other => Err(syn::Error::new_spanned(
                other,
                "tree_model! currently accepts only struct, enum, and type alias items",
            )),
        })
        .collect::<syn::Result<Vec<_>>>()
        .map(|groups| groups.into_iter().flatten().collect())
}

#[requires(true)]
#[ensures(true)]
fn walk_module(
    items: &[Item],
    include_recovered: bool,
    include_with_free_modifiers: bool,
) -> syn::Result<proc_macro2::TokenStream> {
    let wrapper_functions = walk_wrapper_functions(include_recovered, include_with_free_modifiers);
    let node_functions = items
        .iter()
        .map(|item| match item {
            Item::Struct(item) => walk_struct_function(item),
            Item::Enum(item) => walk_enum_functions(item),
            Item::Type(_) => Ok(Vec::new()),
            other => Err(syn::Error::new_spanned(
                other,
                "tree_model! currently accepts only struct, enum, and type alias items",
            )),
        })
        .collect::<syn::Result<Vec<_>>>()?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    Ok(quote! {
        /// Free descent functions backing `TreeWalker` default methods.
        pub mod walk {
            use super::*;

            #wrapper_functions
            #(#node_functions)*
        }
    })
}

#[requires(true)]
#[ensures(true)]
fn walk_wrapper_functions(
    include_recovered: bool,
    include_with_free_modifiers: bool,
) -> proc_macro2::TokenStream {
    let recovered_function = include_recovered.then(|| {
        quote! {
            pub fn recovered<'tree, W, T>(walker: &mut W, value: &'tree Recovered<T>)
            where
                W: TreeWalker<'tree> + ?Sized,
                T: TreeWalkable<'tree>,
            {
                match value {
                    ::jbotci_tree::Recovered::Valid(value) => {
                        TreeWalkable::walk_with(value, walker);
                    }
                    ::jbotci_tree::Recovered::Error(item) => {
                        walker.walk_recovered_error(item);
                    }
                    ::jbotci_tree::Recovered::Prefix(prefix) => {
                        for item in &prefix.errors {
                            walker.walk_recovered_error(item);
                        }
                        TreeWalkable::walk_with(&prefix.value, walker);
                    }
                }
            }
        }
    });
    let with_free_modifiers_function = include_with_free_modifiers.then(|| {
        if include_recovered {
            quote! {
                pub fn with_free_modifiers<'tree, W, T>(
                    walker: &mut W,
                    value: &'tree WithFreeModifiers<T>,
                )
                where
                    W: TreeWalker<'tree> + ?Sized,
                    T: TreeWalkable<'tree>,
                {
                    TreeWalkable::walk_with(&value.value, walker);
                    TreeWalkable::walk_with(&value.free_modifiers, walker);
                }
            }
        } else {
            quote! {
                pub fn with_free_modifiers<'tree, W, T, F>(
                    walker: &mut W,
                    value: &'tree WithFreeModifiers<T, F>,
                )
                where
                    W: TreeWalker<'tree> + ?Sized,
                    T: TreeWalkable<'tree>,
                    F: TreeWalkable<'tree>,
                {
                    TreeWalkable::walk_with(&value.value, walker);
                    TreeWalkable::walk_with(&value.free_modifiers, walker);
                }
            }
        }
    });
    quote! {
        #recovered_function
        #with_free_modifiers_function

        pub fn boxed<'tree, W, T>(walker: &mut W, value: &'tree Box<T>)
        where
            W: TreeWalker<'tree> + ?Sized,
            T: TreeWalkable<'tree> + ?Sized,
        {
            TreeWalkable::walk_with(&**value, walker);
        }

        pub fn arc<'tree, W, T>(walker: &mut W, value: &'tree ::std::sync::Arc<T>)
        where
            W: TreeWalker<'tree> + ?Sized,
            T: TreeWalkable<'tree> + ?Sized,
        {
            TreeWalkable::walk_with(&**value, walker);
        }

        pub fn option<'tree, W, T>(walker: &mut W, value: &'tree Option<T>)
        where
            W: TreeWalker<'tree> + ?Sized,
            T: TreeWalkable<'tree>,
        {
            if let Some(value) = value {
                TreeWalkable::walk_with(value, walker);
            }
        }

        pub fn tuple2<'tree, W, A, B>(walker: &mut W, value: &'tree (A, B))
        where
            W: TreeWalker<'tree> + ?Sized,
            A: TreeWalkable<'tree>,
            B: TreeWalkable<'tree>,
        {
            TreeWalkable::walk_with(&value.0, walker);
            TreeWalkable::walk_with(&value.1, walker);
        }

        pub fn chain_vec<'tree, W, E, L>(
            walker: &mut W,
            value: &'tree ::jbotci_tree::Chain<E, Vec<L>>,
        )
        where
            W: TreeWalker<'tree> + ?Sized,
            E: TreeWalkable<'tree>,
            L: TreeWalkable<'tree>,
        {
            TreeWalkable::walk_with(&value.first, walker);
            for link in &value.links {
                TreeWalkable::walk_with(link, walker);
            }
        }

        pub fn chain_vec1<'tree, W, E, L>(
            walker: &mut W,
            value: &'tree ::jbotci_tree::Chain<E, ::vec1::Vec1<L>>,
        )
        where
            W: TreeWalker<'tree> + ?Sized,
            E: TreeWalkable<'tree>,
            L: TreeWalkable<'tree>,
        {
            TreeWalkable::walk_with(&value.first, walker);
            for link in &value.links {
                TreeWalkable::walk_with(link, walker);
            }
        }

        pub fn vec<'tree, W, T>(walker: &mut W, value: &'tree Vec<T>)
        where
            W: TreeWalker<'tree> + ?Sized,
            T: TreeWalkable<'tree>,
        {
            for value in value {
                TreeWalkable::walk_with(value, walker);
            }
        }

        pub fn vec1<'tree, W, T>(walker: &mut W, value: &'tree ::vec1::Vec1<T>)
        where
            W: TreeWalker<'tree> + ?Sized,
            T: TreeWalkable<'tree>,
        {
            for value in value {
                TreeWalkable::walk_with(value, walker);
            }
        }

        pub fn small_vec<'tree, W, A>(walker: &mut W, value: &'tree ::smallvec::SmallVec<A>)
        where
            W: TreeWalker<'tree> + ?Sized,
            A: ::smallvec::Array,
            A::Item: TreeWalkable<'tree>,
        {
            for value in value {
                TreeWalkable::walk_with(value, walker);
            }
        }

        pub fn small_vec1<'tree, W, A>(
            walker: &mut W,
            value: &'tree ::vec1::smallvec_v1::SmallVec1<A>,
        )
        where
            W: TreeWalker<'tree> + ?Sized,
            A: ::smallvec::Array,
            A::Item: TreeWalkable<'tree>,
        {
            for value in value {
                TreeWalkable::walk_with(value, walker);
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn walk_struct_function(item: &ItemStruct) -> syn::Result<Vec<proc_macro2::TokenStream>> {
    let ident = &item.ident;
    let function = walk_function_ident_for_type(ident);
    let walks = field_walks(&item.fields, |index, field| {
        field
            .ident
            .as_ref()
            .map(|ident| quote!(&node.#ident))
            .unwrap_or_else(|| {
                let index = syn::Index::from(index);
                quote!(&node.#index)
            })
    })?;
    Ok(vec![quote! {
        pub fn #function<'tree, W>(walker: &mut W, node: &'tree #ident)
        where
            W: TreeWalker<'tree> + ?Sized,
        {
            #(#walks)*
        }
    }])
}

#[requires(true)]
#[ensures(true)]
fn walk_enum_functions(item: &ItemEnum) -> syn::Result<Vec<proc_macro2::TokenStream>> {
    let enum_ident = &item.ident;
    let enum_function = walk_function_ident_for_type(enum_ident);
    let uses_data_patterns = enum_uses_data_patterns(item);
    let enum_arms = item
        .variants
        .iter()
        .map(|variant| {
            let variant_method = walk_method_ident_for_variant(enum_ident, &variant.ident);
            let bindings = enum_variant_payload_bindings(&variant.fields);
            let pattern = enum_variant_payload_pattern(
                enum_ident,
                &variant.ident,
                &variant.fields,
                uses_data_patterns,
                &bindings,
            )?;
            Ok(quote! {
                #pattern => walker.#variant_method(#(#bindings),*),
            })
        })
        .collect::<syn::Result<Vec<_>>>()?;
    let match_value = if uses_data_patterns {
        quote!(node.as_data())
    } else {
        quote!(node)
    };
    let mut functions = vec![quote! {
        pub fn #enum_function<'tree, W>(walker: &mut W, node: &'tree #enum_ident)
        where
            W: TreeWalker<'tree> + ?Sized,
        {
            match #match_value {
                #(#enum_arms)*
            }
        }
    }];
    for variant in &item.variants {
        functions.push(walk_enum_variant_function(
            enum_ident,
            variant,
            uses_data_patterns,
        )?);
    }
    Ok(functions)
}

#[requires(true)]
#[ensures(true)]
fn walk_enum_variant_function(
    enum_ident: &Ident,
    variant: &syn::Variant,
    _uses_data_patterns: bool,
) -> syn::Result<proc_macro2::TokenStream> {
    let variant_ident = &variant.ident;
    let function = walk_function_ident_for_variant(enum_ident, variant_ident);
    let params = enum_variant_payload_params(&variant.fields);
    match &variant.fields {
        Fields::Named(_) => {
            let bindings = enum_variant_payload_bindings(&variant.fields);
            let walks = field_walks(&variant.fields, |_index, field| {
                let ident = field.ident.as_ref().unwrap();
                quote!(#ident)
            })?;
            Ok(quote! {
                pub fn #function<'tree, W>(walker: &mut W #(, #params)*)
                where
                    W: TreeWalker<'tree> + ?Sized,
                {
                    let _ = (#(#bindings,)*);
                    #(#walks)*
                }
            })
        }
        Fields::Unnamed(_) => {
            let bindings = enum_variant_payload_bindings(&variant.fields);
            let walks = field_walks(&variant.fields, |index, _field| {
                let ident = &bindings[index];
                quote!(#ident)
            })?;
            Ok(quote! {
                pub fn #function<'tree, W>(walker: &mut W #(, #params)*)
                where
                    W: TreeWalker<'tree> + ?Sized,
                {
                    let _ = (#(#bindings,)*);
                    #(#walks)*
                }
            })
        }
        Fields::Unit => Ok(quote! {
            pub fn #function<'tree, W>(_walker: &mut W)
            where
                W: TreeWalker<'tree> + ?Sized,
            {}
        }),
    }
}

#[requires(true)]
#[ensures(ret.len() == fields.len())]
fn enum_variant_payload_bindings(fields: &Fields) -> Vec<Ident> {
    match fields {
        Fields::Named(fields) => fields
            .named
            .iter()
            .map(|field| field.ident.clone().expect("named fields have identifiers"))
            .collect(),
        Fields::Unnamed(fields) => (0..fields.unnamed.len())
            .map(|index| format_ident!("field_{index}"))
            .collect(),
        Fields::Unit => Vec::new(),
    }
}

#[requires(true)]
#[ensures(ret.len() == fields.len())]
fn enum_variant_payload_params(fields: &Fields) -> Vec<proc_macro2::TokenStream> {
    enum_variant_payload_bindings(fields)
        .into_iter()
        .zip(fields.iter())
        .map(|(binding, field)| {
            let ty = &field.ty;
            quote!(#binding: &'tree #ty)
        })
        .collect()
}

#[requires(bindings.len() == fields.len())]
#[ensures(true)]
fn enum_variant_payload_pattern(
    enum_ident: &Ident,
    variant_ident: &Ident,
    fields: &Fields,
    uses_data_patterns: bool,
    bindings: &[Ident],
) -> syn::Result<proc_macro2::TokenStream> {
    Ok(match fields {
        Fields::Named(_) => {
            if uses_data_patterns {
                quote!(::bityzba::data!(#enum_ident::#variant_ident { #(#bindings,)* }))
            } else {
                quote!(#enum_ident::#variant_ident { #(#bindings,)* })
            }
        }
        Fields::Unnamed(_) => {
            if uses_data_patterns {
                quote!(::bityzba::data!(#enum_ident::#variant_ident(#(#bindings,)*)))
            } else {
                quote!(#enum_ident::#variant_ident(#(#bindings,)*))
            }
        }
        Fields::Unit => {
            if uses_data_patterns {
                quote!(::bityzba::data!(#enum_ident::#variant_ident))
            } else {
                quote!(#enum_ident::#variant_ident)
            }
        }
    })
}

#[requires(true)]
#[ensures(true)]
fn tree_walkable_impls(
    items: &[Item],
    atom_types: &BTreeMap<String, Type>,
    include_recovered: bool,
    include_with_free_modifiers: bool,
) -> syn::Result<proc_macro2::TokenStream> {
    let node_impls = items
        .iter()
        .map(|item| match item {
            Item::Struct(item) => Ok(tree_walkable_node_impl(&item.ident)),
            Item::Enum(item) => Ok(tree_walkable_node_impl(&item.ident)),
            Item::Type(_) => Ok(quote!()),
            other => Err(syn::Error::new_spanned(
                other,
                "tree_model! currently accepts only struct, enum, and type alias items",
            )),
        })
        .collect::<syn::Result<Vec<_>>>()?;
    let atom_impls = atom_types.values().map(|ty| {
        let variant = atom_variant_ident(ty);
        quote! {
            impl<'tree> TreeWalkable<'tree> for #ty {
                fn walk_with<W>(&'tree self, walker: &mut W)
                where
                    W: TreeWalker<'tree> + ?Sized,
                {
                    walker.walk_atom(AtomRef::#variant(self));
                }
            }
        }
    });
    let wrapper_impls = tree_walkable_wrapper_impls(include_recovered, include_with_free_modifiers);
    Ok(quote! {
        #(#node_impls)*
        #(#atom_impls)*
        #wrapper_impls
    })
}

#[requires(true)]
#[ensures(true)]
fn tree_walkable_node_impl(ident: &Ident) -> proc_macro2::TokenStream {
    let method = walk_method_ident_for_type(ident);
    quote! {
        impl<'tree> TreeWalkable<'tree> for #ident {
            fn walk_with<W>(&'tree self, walker: &mut W)
            where
                W: TreeWalker<'tree> + ?Sized,
            {
                walker.#method(self);
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn tree_walkable_wrapper_impls(
    include_recovered: bool,
    include_with_free_modifiers: bool,
) -> proc_macro2::TokenStream {
    let recovered_impl = include_recovered.then(|| {
        quote! {
            impl<'tree, T> TreeWalkable<'tree> for Recovered<T>
            where
                T: TreeWalkable<'tree>,
            {
                fn walk_with<W>(&'tree self, walker: &mut W)
                where
                    W: TreeWalker<'tree> + ?Sized,
                {
                    walk::recovered(walker, self);
                }
            }
        }
    });
    let with_free_modifiers_impl = include_with_free_modifiers.then(|| {
        if include_recovered {
            quote! {
                impl<'tree, T> TreeWalkable<'tree> for WithFreeModifiers<T>
                where
                    T: TreeWalkable<'tree>,
                {
                    fn walk_with<W>(&'tree self, walker: &mut W)
                    where
                        W: TreeWalker<'tree> + ?Sized,
                    {
                        walk::with_free_modifiers(walker, self);
                    }
                }
            }
        } else {
            quote! {
                impl<'tree, T, F> TreeWalkable<'tree> for WithFreeModifiers<T, F>
                where
                    T: TreeWalkable<'tree>,
                    F: TreeWalkable<'tree>,
                {
                    fn walk_with<W>(&'tree self, walker: &mut W)
                    where
                        W: TreeWalker<'tree> + ?Sized,
                    {
                        walk::with_free_modifiers(walker, self);
                    }
                }
            }
        }
    });
    quote! {
        #recovered_impl
        #with_free_modifiers_impl

        impl<'tree, T> TreeWalkable<'tree> for Box<T>
        where
            T: TreeWalkable<'tree> + ?Sized,
        {
            fn walk_with<W>(&'tree self, walker: &mut W)
            where
                W: TreeWalker<'tree> + ?Sized,
            {
                walk::boxed(walker, self);
            }
        }

        impl<'tree, T> TreeWalkable<'tree> for ::std::sync::Arc<T>
        where
            T: TreeWalkable<'tree> + ?Sized,
        {
            fn walk_with<W>(&'tree self, walker: &mut W)
            where
                W: TreeWalker<'tree> + ?Sized,
            {
                walk::arc(walker, self);
            }
        }

        impl<'tree, T> TreeWalkable<'tree> for Option<T>
        where
            T: TreeWalkable<'tree>,
        {
            fn walk_with<W>(&'tree self, walker: &mut W)
            where
                W: TreeWalker<'tree> + ?Sized,
            {
                walk::option(walker, self);
            }
        }

        impl<'tree, A, B> TreeWalkable<'tree> for (A, B)
        where
            A: TreeWalkable<'tree>,
            B: TreeWalkable<'tree>,
        {
            fn walk_with<W>(&'tree self, walker: &mut W)
            where
                W: TreeWalker<'tree> + ?Sized,
            {
                walk::tuple2(walker, self);
            }
        }

        impl<'tree, E, L> TreeWalkable<'tree> for ::jbotci_tree::Chain<E, Vec<L>>
        where
            E: TreeWalkable<'tree>,
            L: TreeWalkable<'tree>,
        {
            fn walk_with<W>(&'tree self, walker: &mut W)
            where
                W: TreeWalker<'tree> + ?Sized,
            {
                walk::chain_vec(walker, self);
            }
        }

        impl<'tree, E, L> TreeWalkable<'tree> for ::jbotci_tree::Chain<E, ::vec1::Vec1<L>>
        where
            E: TreeWalkable<'tree>,
            L: TreeWalkable<'tree>,
        {
            fn walk_with<W>(&'tree self, walker: &mut W)
            where
                W: TreeWalker<'tree> + ?Sized,
            {
                walk::chain_vec1(walker, self);
            }
        }

        impl<'tree, T> TreeWalkable<'tree> for Vec<T>
        where
            T: TreeWalkable<'tree>,
        {
            fn walk_with<W>(&'tree self, walker: &mut W)
            where
                W: TreeWalker<'tree> + ?Sized,
            {
                walk::vec(walker, self);
            }
        }

        impl<'tree, T> TreeWalkable<'tree> for ::vec1::Vec1<T>
        where
            T: TreeWalkable<'tree>,
        {
            fn walk_with<W>(&'tree self, walker: &mut W)
            where
                W: TreeWalker<'tree> + ?Sized,
            {
                walk::vec1(walker, self);
            }
        }

        impl<'tree, A> TreeWalkable<'tree> for ::smallvec::SmallVec<A>
        where
            A: ::smallvec::Array,
            A::Item: TreeWalkable<'tree>,
        {
            fn walk_with<W>(&'tree self, walker: &mut W)
            where
                W: TreeWalker<'tree> + ?Sized,
            {
                walk::small_vec(walker, self);
            }
        }

        impl<'tree, A> TreeWalkable<'tree> for ::vec1::smallvec_v1::SmallVec1<A>
        where
            A: ::smallvec::Array,
            A::Item: TreeWalkable<'tree>,
        {
            fn walk_with<W>(&'tree self, walker: &mut W)
            where
                W: TreeWalker<'tree> + ?Sized,
            {
                walk::small_vec1(walker, self);
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn field_walks<F>(fields: &Fields, access: F) -> syn::Result<Vec<proc_macro2::TokenStream>>
where
    F: Fn(usize, &syn::Field) -> proc_macro2::TokenStream,
{
    fields
        .iter()
        .enumerate()
        .filter_map(|(index, field)| match tree_child_flags(&field.attrs) {
            Ok(flags) if flags.skip => None,
            Ok(_) => {
                let access = access(index, field);
                Some(Ok(quote! {
                    TreeWalkable::walk_with(#access, walker);
                }))
            }
            Err(error) => Some(Err(error)),
        })
        .collect()
}

#[requires(true)]
#[ensures(true)]
fn walk_method_ident_for_type(ident: &Ident) -> Ident {
    let function = walk_function_ident_for_type(ident);
    format_ident!("walk_{function}")
}

#[requires(true)]
#[ensures(true)]
fn walk_function_ident_for_type(ident: &Ident) -> Ident {
    format_ident!("{}", walk_base_name(ident))
}

#[requires(true)]
#[ensures(true)]
fn walk_method_ident_for_variant(enum_ident: &Ident, variant_ident: &Ident) -> Ident {
    let function = walk_function_ident_for_variant(enum_ident, variant_ident);
    format_ident!("walk_{function}")
}

#[requires(true)]
#[ensures(true)]
fn walk_function_ident_for_variant(enum_ident: &Ident, variant_ident: &Ident) -> Ident {
    format_ident!(
        "{}_{}",
        walk_base_name(enum_ident),
        camel_case_to_snake_case(&variant_ident.to_string())
    )
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn walk_base_name(ident: &Ident) -> String {
    let text = ident.to_string();
    let text = text.strip_suffix("Syntax").unwrap_or(&text);
    camel_case_to_snake_case(text)
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn camel_case_to_snake_case(text: &str) -> String {
    let mut output = String::new();
    let mut previous_is_lower_or_digit = false;
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch.is_ascii_uppercase() {
            let next_is_lower = chars.peek().is_some_and(|next| next.is_ascii_lowercase());
            if !output.is_empty() && (previous_is_lower_or_digit || next_is_lower) {
                output.push('_');
            }
            output.push(ch.to_ascii_lowercase());
            previous_is_lower_or_digit = false;
        } else {
            output.push(ch);
            previous_is_lower_or_digit = ch.is_ascii_lowercase() || ch.is_ascii_digit();
        }
    }
    output
}

#[requires(true)]
#[ensures(true)]
fn tree_node_trait_impls(
    items: &[Item],
    node_names: &BTreeSet<String>,
) -> syn::Result<proc_macro2::TokenStream> {
    let _ = node_names;
    let impls = items
        .iter()
        .map(|item| match item {
            Item::Struct(item) => tree_node_struct_impl(item),
            Item::Enum(item) => tree_node_enum_impl(item),
            Item::Type(_) => Ok(quote!()),
            other => Err(syn::Error::new_spanned(
                other,
                "tree_model! currently accepts only struct, enum, and type alias items",
            )),
        })
        .collect::<syn::Result<Vec<_>>>()?;
    Ok(quote!(#(#impls)*))
}

#[requires(true)]
#[ensures(true)]
fn tree_node_struct_impl(item: &ItemStruct) -> syn::Result<proc_macro2::TokenStream> {
    let ident = &item.ident;
    let visits = field_visits(&item.fields, |index, field| {
        field
            .ident
            .as_ref()
            .map(|ident| quote!(&self.#ident))
            .unwrap_or_else(|| {
                let index = syn::Index::from(index);
                quote!(&self.#index)
            })
    })?;
    let paths = field_paths(&item.fields, |index, field| {
        field
            .ident
            .as_ref()
            .map(|ident| quote!(&self.#ident))
            .unwrap_or_else(|| {
                let index = syn::Index::from(index);
                quote!(&self.#index)
            })
    })?;
    let child_lookups = field_child_lookups(&item.fields, |index, field| {
        field
            .ident
            .as_ref()
            .map(|ident| quote!(&self.#ident))
            .unwrap_or_else(|| {
                let index = syn::Index::from(index);
                quote!(&self.#index)
            })
    })?;
    Ok(quote! {
        impl TreeNode for #ident {
            fn as_node_ref<'tree>(&'tree self) -> Option<NodeRef<'tree>> {
                Some(NodeRef::#ident(self))
            }

            fn visit_in_order<'tree, V>(&'tree self, visitor: &mut V)
            where
                V: ::jbotci_tree::TreeVisitor<'tree, Node = NodeRef<'tree>, Atom = AtomRef<'tree>>,
            {
                let node = NodeRef::#ident(self);
                visitor.enter_node(node);
                #(#visits)*
                visitor.exit_node(node);
            }

            fn path_to_node_from<'tree>(
                &'tree self,
                target: NodeRef<'tree>,
                path: &mut ::jbotci_tree::TreePath,
            ) -> bool {
                let node = NodeRef::#ident(self);
                if node == target {
                    return true;
                }
                #(#paths)*
                false
            }

            fn node_at_path_steps<'tree>(
                &'tree self,
                steps: &[::jbotci_tree::TreePathStep],
            ) -> Option<NodeRef<'tree>> {
                if steps.is_empty() {
                    return Some(NodeRef::#ident(self));
                }
                #(#child_lookups)*
                None
            }
        }
    })
}

#[requires(true)]
#[ensures(true)]
fn tree_node_enum_impl(item: &ItemEnum) -> syn::Result<proc_macro2::TokenStream> {
    let enum_ident = &item.ident;
    let uses_data_patterns = enum_uses_data_patterns(item);
    let arms = item
        .variants
        .iter()
        .map(|variant| {
            let variant_ident = &variant.ident;
            let node_ref_variant = node_ref_variant_ident(enum_ident, variant_ident);
            match &variant.fields {
                Fields::Named(fields) => {
                    let bindings = fields
                        .named
                        .iter()
                        .map(|field| field.ident.as_ref().unwrap());
                    let pattern_bindings = bindings.clone();
                    let visits = field_visits(&variant.fields, |_index, field| {
                        let ident = field.ident.as_ref().unwrap();
                        quote!(#ident)
                    })?;
                    let paths = field_paths(&variant.fields, |_index, field| {
                        let ident = field.ident.as_ref().unwrap();
                        quote!(#ident)
                    })?;
                    let child_lookups = field_child_lookups(&variant.fields, |_index, field| {
                        let ident = field.ident.as_ref().unwrap();
                        quote!(#ident)
                    })?;
                    let pattern = if uses_data_patterns {
                        quote!(
                            ::bityzba::data!(#enum_ident::#variant_ident { #(#pattern_bindings,)* })
                        )
                    } else {
                        quote!(#enum_ident::#variant_ident { #(#pattern_bindings,)* })
                    };
                    let visit_arm = quote! {
                        #pattern => {
                            let node = NodeRef::#node_ref_variant(self);
                            visitor.enter_node(node);
                            #(#visits)*
                            visitor.exit_node(node);
                        }
                    };
                    let path_arm = quote! {
                        #pattern => {
                            let node = NodeRef::#node_ref_variant(self);
                            if node == target {
                                return true;
                            }
                            #(#paths)*
                            false
                        }
                    };
                    let child_lookup_arm = quote! {
                        #pattern => {
                            if steps.is_empty() {
                                return Some(NodeRef::#node_ref_variant(self));
                            }
                            #(#child_lookups)*
                            None
                        }
                    };
                    Ok((visit_arm, path_arm, child_lookup_arm))
                }
                Fields::Unnamed(fields) => {
                    let bindings = (0..fields.unnamed.len())
                        .map(|index| format_ident!("field_{index}"))
                        .collect::<Vec<_>>();
                    let pattern_bindings = bindings.clone();
                    let visits = field_visits(&variant.fields, |index, _field| {
                        let ident = &bindings[index];
                        quote!(#ident)
                    })?;
                    let paths = field_paths(&variant.fields, |index, _field| {
                        let ident = &bindings[index];
                        quote!(#ident)
                    })?;
                    let child_lookups = field_child_lookups(&variant.fields, |index, _field| {
                        let ident = &bindings[index];
                        quote!(#ident)
                    })?;
                    let pattern = if uses_data_patterns {
                        quote!(
                            ::bityzba::data!(#enum_ident::#variant_ident(#(#pattern_bindings,)*))
                        )
                    } else {
                        quote!(#enum_ident::#variant_ident(#(#pattern_bindings,)*))
                    };
                    let visit_arm = quote! {
                        #pattern => {
                            let node = NodeRef::#node_ref_variant(self);
                            visitor.enter_node(node);
                            #(#visits)*
                            visitor.exit_node(node);
                        }
                    };
                    let path_arm = quote! {
                        #pattern => {
                            let node = NodeRef::#node_ref_variant(self);
                            if node == target {
                                return true;
                            }
                            #(#paths)*
                            false
                        }
                    };
                    let child_lookup_arm = quote! {
                        #pattern => {
                            if steps.is_empty() {
                                return Some(NodeRef::#node_ref_variant(self));
                            }
                            #(#child_lookups)*
                            None
                        }
                    };
                    Ok((visit_arm, path_arm, child_lookup_arm))
                }
                Fields::Unit => {
                    let pattern = if uses_data_patterns {
                        quote!(::bityzba::data!(#enum_ident::#variant_ident))
                    } else {
                        quote!(#enum_ident::#variant_ident)
                    };
                    let visit_arm = quote! {
                        #pattern => {
                            let node = NodeRef::#node_ref_variant(self);
                            visitor.enter_node(node);
                            visitor.exit_node(node);
                        }
                    };
                    let path_arm = quote! {
                        #pattern => {
                            NodeRef::#node_ref_variant(self) == target
                        }
                    };
                    let child_lookup_arm = quote! {
                        #pattern => {
                            steps.is_empty().then_some(NodeRef::#node_ref_variant(self))
                        }
                    };
                    Ok((visit_arm, path_arm, child_lookup_arm))
                }
            }
        })
        .collect::<syn::Result<Vec<_>>>()?;
    let visit_arms = arms.iter().map(|(visit_arm, _, _)| visit_arm);
    let path_arms = arms.iter().map(|(_, path_arm, _)| path_arm);
    let child_lookup_arms = arms.iter().map(|(_, _, child_lookup_arm)| child_lookup_arm);
    let match_value = if uses_data_patterns {
        quote!(self.as_data())
    } else {
        quote!(self)
    };
    Ok(quote! {
        impl TreeNode for #enum_ident {
            fn as_node_ref<'tree>(&'tree self) -> Option<NodeRef<'tree>> {
                Some(NodeRef::from(self))
            }

            fn visit_in_order<'tree, V>(&'tree self, visitor: &mut V)
            where
                V: ::jbotci_tree::TreeVisitor<'tree, Node = NodeRef<'tree>, Atom = AtomRef<'tree>>,
            {
                match #match_value {
                    #(#visit_arms)*
                }
            }

            fn path_to_node_from<'tree>(
                &'tree self,
                target: NodeRef<'tree>,
                path: &mut ::jbotci_tree::TreePath,
            ) -> bool {
                match #match_value {
                    #(#path_arms)*
                }
            }

            fn node_at_path_steps<'tree>(
                &'tree self,
                steps: &[::jbotci_tree::TreePathStep],
            ) -> Option<NodeRef<'tree>> {
                match #match_value {
                    #(#child_lookup_arms)*
                }
            }
        }
    })
}

#[requires(true)]
#[ensures(true)]
fn enum_uses_data_patterns(item: &ItemEnum) -> bool {
    item.attrs
        .iter()
        .filter(|attr| {
            attr.path().is_ident("invariant") || attr.path().is_ident("expensive_invariant")
        })
        .any(|attr| !attr_is_true_contract_marker(attr))
}

#[requires(true)]
#[ensures(true)]
fn attr_is_true_contract_marker(attr: &Attribute) -> bool {
    let syn::Meta::List(list) = &attr.meta else {
        return false;
    };
    bityzba_contract_syntax::contract_attribute_is_true_marker(list.tokens.clone())
}

#[requires(true)]
#[ensures(true)]
fn field_visits<F>(fields: &Fields, access: F) -> syn::Result<Vec<proc_macro2::TokenStream>>
where
    F: Fn(usize, &syn::Field) -> proc_macro2::TokenStream,
{
    fields
        .iter()
        .enumerate()
        .filter_map(|(index, field)| match tree_child_flags(&field.attrs) {
            Ok(flags) if flags.skip => None,
            Ok(flags) => {
                let name = field.ident.as_ref().map(Ident::to_string);
                let name = match name {
                    Some(name) => quote!(Some(#name)),
                    None => quote!(None),
                };
                let primary = flags.primary;
                let access = access(index, field);
                let absent_visit = if field_is_option(&field.ty) {
                    quote! {
                        if (#access).is_none() {
                            visitor.visit_absent_optional_field(field_ref);
                        }
                    }
                } else {
                    quote!()
                };
                Some(Ok(quote! {
                    let field_ref = ::jbotci_tree::FieldRef::new(#name, #index, #primary);
                    visitor.enter_field(field_ref);
                    #absent_visit
                    TreeNode::visit_in_order(#access, visitor);
                    visitor.exit_field(field_ref);
                }))
            }
            Err(error) => Some(Err(error)),
        })
        .collect()
}

#[requires(true)]
#[ensures(true)]
fn field_is_option(ty: &Type) -> bool {
    let Type::Path(path) = ty else {
        return false;
    };
    path.qself.is_none()
        && path
            .path
            .segments
            .last()
            .is_some_and(|segment| segment.ident == "Option")
}

#[requires(true)]
#[ensures(true)]
fn field_paths<F>(fields: &Fields, access: F) -> syn::Result<Vec<proc_macro2::TokenStream>>
where
    F: Fn(usize, &syn::Field) -> proc_macro2::TokenStream,
{
    fields
        .iter()
        .enumerate()
        .filter_map(|(index, field)| match tree_child_flags(&field.attrs) {
            Ok(flags) if flags.skip => None,
            Ok(_) => {
                let name = field_name_tokens(field);
                let access = access(index, field);
                Some(Ok(quote! {
                    path.push(::jbotci_tree::TreePathStep::field(#name, #index));
                    if TreeNode::path_to_node_from(#access, target, path) {
                        return true;
                    }
                    path.pop();
                }))
            }
            Err(error) => Some(Err(error)),
        })
        .collect()
}

#[requires(true)]
#[ensures(true)]
fn field_child_lookups<F>(fields: &Fields, access: F) -> syn::Result<Vec<proc_macro2::TokenStream>>
where
    F: Fn(usize, &syn::Field) -> proc_macro2::TokenStream,
{
    fields
        .iter()
        .enumerate()
        .filter_map(|(index, field)| match tree_child_flags(&field.attrs) {
            Ok(flags) if flags.skip => None,
            Ok(_) => {
                let name = field_name_tokens(field);
                let access = access(index, field);
                Some(Ok(quote! {
                    if let Some((step, rest)) = steps.split_first()
                        && step.is_field(#name, #index)
                    {
                        return TreeNode::node_at_path_steps(#access, rest);
                    }
                }))
            }
            Err(error) => Some(Err(error)),
        })
        .collect()
}

#[requires(true)]
#[ensures(true)]
fn field_name_tokens(field: &syn::Field) -> proc_macro2::TokenStream {
    let name = field.ident.as_ref().map(Ident::to_string);
    match name {
        Some(name) => quote!(Some(#name)),
        None => quote!(None),
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy)]
struct TreeChildFlags {
    primary: bool,
    skip: bool,
}

#[requires(true)]
#[ensures(true)]
fn tree_child_flags(attrs: &[Attribute]) -> syn::Result<TreeChildFlags> {
    let mut flags = TreeChildFlags {
        primary: false,
        skip: false,
    };
    let mut primary_attr = None::<&Attribute>;
    let mut skip_attr = None::<&Attribute>;
    for attr in attrs
        .iter()
        .filter(|attr| attr.path().is_ident("tree_child"))
    {
        if attr
            .parse_args::<syn::LitBool>()
            .is_ok_and(|lit| !lit.value)
        {
            flags.skip = true;
            skip_attr = Some(attr);
            continue;
        }
        let ident = attr.parse_args::<Ident>()?;
        if ident == "primary" {
            flags.primary = true;
            primary_attr = Some(attr);
        } else {
            return Err(syn::Error::new_spanned(
                attr,
                "supported tree_child flags are `primary` and `false`",
            ));
        }
    }
    if flags.primary && flags.skip {
        let attr = skip_attr
            .or(primary_attr)
            .expect("conflicting flags came from attributes");
        return Err(syn::Error::new_spanned(
            attr,
            "`tree_child(primary)` cannot be combined with `tree_child(false)`",
        ));
    }
    Ok(flags)
}
