//! Proc-macro implementation for generic jbotci tree models.

extern crate proc_macro;

use std::collections::{BTreeMap, BTreeSet};

use proc_macro::TokenStream;
use proc_macro2::{Spacing, TokenTree};
use quote::{format_ident, quote};
use syn::{
    Attribute, Fields, GenericArgument, Ident, Item, ItemEnum, ItemStruct, ItemType, PathArguments,
    Type, parse_macro_input, parse_quote,
};

#[proc_macro]
pub fn tree_model(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::File);
    expand_tree_model(input.items)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn expand_tree_model(mut items: Vec<Item>) -> syn::Result<proc_macro2::TokenStream> {
    let node_names = collect_node_names(&items)?;
    let aliases = collect_type_aliases(&items);
    let atom_types = collect_atom_types(&items, &node_names, &aliases)?;
    let node_ref = node_ref_enum(&items)?;
    let atom_ref = atom_ref_enum(&atom_types);
    let trait_impls = tree_node_trait_impls(&items, &node_names)?;
    let atom_impls = atom_trait_impls(&atom_types);
    let wrapper_impls = wrapper_trait_impls(false, false);
    let cleaned_items = items
        .iter_mut()
        .map(strip_tree_attrs_from_item)
        .collect::<syn::Result<Vec<_>>>()?;
    let valid_module = valid_module(&items);
    let recovered_module = recovered_module(&items, &node_names, &aliases)?;

    Ok(quote! {
        #(#cleaned_items)*

        #valid_module

        #recovered_module

        #node_ref
        #atom_ref

        pub trait TreeNode {
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

        #wrapper_impls
        #atom_impls
        #trait_impls
    })
}

fn valid_module(items: &[Item]) -> proc_macro2::TokenStream {
    let names = items.iter().filter_map(item_ident);
    quote! {
        pub mod valid {
            pub use super::{#(#names,)* AtomRef, NodeRef, TreeNode};
        }
    }
}

fn item_ident(item: &Item) -> Option<&Ident> {
    match item {
        Item::Struct(item) => Some(&item.ident),
        Item::Enum(item) => Some(&item.ident),
        Item::Type(item) => Some(&item.ident),
        _ => None,
    }
}

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
    let trait_impls = tree_node_trait_impls(&recovered_items, node_names)?;
    let atom_impls = atom_trait_impls(&recovered_atom_types);
    let has_with_free_modifiers = items_use_wrapper(items, "WithFreeModifiers");
    let wrapper_impls = wrapper_trait_impls(true, has_with_free_modifiers);
    let with_free_modifiers = recovered_with_free_modifiers(has_with_free_modifiers);
    let conversion_impls =
        recovered_conversion_impls(&recovered_items, items, node_names, aliases)?;
    let cleaned_items = recovered_items
        .iter_mut()
        .map(strip_tree_attrs_from_item)
        .collect::<syn::Result<Vec<_>>>()?;

    Ok(quote! {
        pub mod recovered {
            use super::*;

            pub type Recovered<T> = ::jbotci_tree::Recovered<T, super::RecoveryTreeItem>;
            pub type RecoveryError = ::jbotci_tree::RecoveryError<super::RecoveryTreeItem>;

            #with_free_modifiers

            #(#cleaned_items)*

            #node_ref
            #atom_ref

            pub trait TreeNode {
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

            #wrapper_impls
            #atom_impls
            #trait_impls
            #conversion_impls
        }
    })
}

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

fn collect_type_aliases(items: &[Item]) -> BTreeMap<String, Type> {
    items
        .iter()
        .filter_map(|item| match item {
            Item::Type(item) => Some((item.ident.to_string(), (*item.ty).clone())),
            _ => None,
        })
        .collect()
}

fn reject_generic_node(ident: &Ident, generics: &syn::Generics) -> syn::Result<()> {
    if generics.params.is_empty() {
        return Ok(());
    }
    Err(syn::Error::new_spanned(
        ident,
        "tree_model! node declarations must be concrete; use a transparent wrapper impl for generic helpers",
    ))
}

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
        collect_atom_type(&field.ty, node_names, aliases, atoms);
    }
    Ok(())
}

fn collect_atom_type(
    ty: &Type,
    node_names: &BTreeSet<String>,
    aliases: &BTreeMap<String, Type>,
    atoms: &mut BTreeMap<String, Type>,
) {
    match unwrap_tree_type(ty, node_names, aliases) {
        UnwrappedTreeType::Node => {}
        UnwrappedTreeType::Atom(atom) => {
            let key = quote!(#atom).to_string();
            atoms.entry(key).or_insert_with(|| atom.clone());
        }
        UnwrappedTreeType::Children(children) => {
            for child in children {
                collect_atom_type(child, node_names, aliases, atoms);
            }
        }
    }
}

enum UnwrappedTreeType<'a> {
    Node,
    Atom(&'a Type),
    Children(Vec<&'a Type>),
}

fn unwrap_tree_type<'a>(
    ty: &'a Type,
    node_names: &BTreeSet<String>,
    aliases: &'a BTreeMap<String, Type>,
) -> UnwrappedTreeType<'a> {
    unwrap_tree_type_with_seen(ty, node_names, aliases, &mut BTreeSet::new())
}

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
                let Some(inner) =
                    first_type_argument(&path.path.segments.last().unwrap().arguments)
                else {
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
];

fn first_type_argument(arguments: &PathArguments) -> Option<&Type> {
    let PathArguments::AngleBracketed(arguments) = arguments else {
        return None;
    };
    arguments.args.iter().find_map(|argument| match argument {
        GenericArgument::Type(ty) => Some(ty),
        _ => None,
    })
}

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

fn recovered_attrs(attrs: &[Attribute]) -> Vec<Attribute> {
    attrs
        .iter()
        .filter(|attr| {
            !attr.path().is_ident("invariant") && !attr.path().is_ident("expensive_invariant")
        })
        .cloned()
        .collect()
}

fn transform_fields_for_recovery(
    fields: &mut Fields,
    node_names: &BTreeSet<String>,
    aliases: &BTreeMap<String, Type>,
) -> syn::Result<()> {
    for field in fields {
        field.ty = transform_type_for_recovery(&field.ty, node_names, aliases)?;
    }
    Ok(())
}

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
        Type::Reference(reference) => {
            transform_type_for_recovery(&reference.elem, node_names, aliases)
        }
        Type::Array(array) => {
            let mut array = array.clone();
            array.elem = Box::new(transform_type_for_recovery(
                &array.elem,
                node_names,
                aliases,
            )?);
            Ok(Type::Array(array))
        }
        _ => {
            let _ = node_names;
            Ok(wrap_recovered(ty.clone()))
        }
    }
}

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
    let PathArguments::AngleBracketed(arguments) = &mut segment.arguments else {
        return Ok(ty);
    };
    for argument in &mut arguments.args {
        if let GenericArgument::Type(inner) = argument {
            *inner = transform_type_for_recovery(inner, node_names, aliases)?;
            break;
        }
    }
    Ok(ty)
}

fn wrap_recovered(ty: Type) -> Type {
    parse_quote!(Recovered<#ty>)
}

fn is_wrapper_ident(ident: &Ident) -> bool {
    WRAPPER_TYPES.contains(&ident.to_string().as_str())
}

fn recovered_with_free_modifiers(emit: bool) -> proc_macro2::TokenStream {
    if !emit {
        return quote!();
    }
    quote! {
        #[derive(Debug, Clone, PartialEq, Eq, ::serde::Serialize)]
        pub struct WithFreeModifiers<T> {
            pub value: T,
            pub free_modifiers: Vec<Recovered<FreeModifierSyntax>>,
        }
    }
}

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

fn recovered_struct_conversion_impl(
    item: &ItemStruct,
    node_names: &BTreeSet<String>,
    aliases: &BTreeMap<String, Type>,
) -> syn::Result<proc_macro2::TokenStream> {
    let ident = &item.ident;
    let valid_ty = quote!(super::#ident);
    let mut field_conversions = Vec::new();
    let mut field_values = Vec::new();
    for (index, field) in item.fields.iter().enumerate() {
        let binding = field_binding_ident(index, field);
        let access = field
            .ident
            .as_ref()
            .map(|ident| quote!(self.#ident))
            .unwrap_or_else(|| {
                let index = syn::Index::from(index);
                quote!(self.#index)
            });
        let path_name = field_name_tokens(field);
        let conversion = convert_value_for_type(&field.ty, access, node_names, aliases)?;
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
    Ok(quote! {
        impl #ident {
            pub fn try_into_valid(self) -> Result<#valid_ty, RecoveryError> {
                let mut path = ::jbotci_tree::TreePath::new();
                self.try_into_valid_at_path(&mut path)
            }

            pub(crate) fn try_into_valid_at_path(
                self,
                path: &mut ::jbotci_tree::TreePath,
            ) -> Result<#valid_ty, RecoveryError> {
                #(#field_conversions)*
                Ok(#construct)
            }
        }
    })
}

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
                self.try_into_valid_at_path(&mut path)
            }

            pub(crate) fn try_into_valid_at_path(
                self,
                path: &mut ::jbotci_tree::TreePath,
            ) -> Result<#valid_ty, RecoveryError> {
                match self {
                    #(#arms,)*
                }
            }
        }
    })
}

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
    let construct = match &item.fields {
        Fields::Named(_) => quote!(Self { #(#field_values,)* }),
        Fields::Unnamed(_) => quote!(Self(#(#field_values,)*)),
        Fields::Unit => quote!(Self),
    };
    Ok(quote! {
        impl #ident {
            pub fn from_valid(value: #valid_ty) -> Self {
                #destructure
                #(#conversions)*
                #construct
            }
        }
    })
}

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
    Ok(quote! {
        impl #enum_ident {
            pub fn from_valid(value: #valid_ty) -> Self {
                match #match_value {
                    #(#arms,)*
                }
            }
        }
    })
}

fn field_binding_ident(index: usize, field: &syn::Field) -> Ident {
    field
        .ident
        .as_ref()
        .map(|ident| format_ident!("converted_{ident}"))
        .unwrap_or_else(|| format_ident!("converted_{index}"))
}

fn item_needs_new_macro(item: &Item) -> bool {
    match item {
        Item::Struct(item) => attrs_need_new_macro(&item.attrs),
        Item::Enum(item) => attrs_need_new_macro(&item.attrs),
        _ => false,
    }
}

fn attrs_need_new_macro(attrs: &[Attribute]) -> bool {
    attrs
        .iter()
        .filter(|attr| {
            attr.path().is_ident("invariant") || attr.path().is_ident("expensive_invariant")
        })
        .any(|attr| !attr_is_true_contract_marker(attr))
}

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
        Type::Reference(reference) => {
            convert_value_for_type(&reference.elem, expr, node_names, aliases)
        }
        Type::Array(array) => {
            convert_array_value_for_type(&array.elem, &array.len, expr, node_names, aliases)
        }
        _ => Ok(convert_recovered_atom(expr)),
    }
}

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
        "Recovered" => convert_value_for_type(inner, expr, node_names, aliases),
        _ => Ok(quote!(Ok(#expr))),
    }
}

fn smallvec_item_type(inner: &Type) -> &Type {
    if let Type::Array(array) = inner {
        &array.elem
    } else {
        inner
    }
}

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

fn convert_recovered_node(expr: proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    quote! {
        (#expr).try_into_valid_with(path, |value, path| value.try_into_valid_at_path(path))
    }
}

fn convert_recovered_atom(expr: proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    quote! {
        (#expr).try_into_valid_with(path, |value, _path| Ok(value))
    }
}

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
        Type::Reference(reference) => {
            convert_valid_value_for_type(&reference.elem, expr, node_names, aliases)
        }
        Type::Array(array) => {
            convert_valid_array_value_for_type(&array.elem, &array.len, expr, node_names, aliases)
        }
        _ => Ok(convert_valid_atom(expr)),
    }
}

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
            let inner = convert_valid_value_for_type(inner, quote!(*value), node_names, aliases)?;
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
        "Recovered" => convert_valid_value_for_type(inner, expr, node_names, aliases),
        _ => Ok(expr),
    }
}

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

fn convert_valid_node(ident: &Ident, expr: proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    quote!(Recovered::valid(#ident::from_valid(#expr)))
}

fn convert_valid_atom(expr: proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    quote!(Recovered::valid(#expr))
}

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

fn fields_use_wrapper(fields: &Fields, wrapper: &str) -> bool {
    fields
        .iter()
        .any(|field| type_uses_wrapper(&field.ty, wrapper))
}

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
    })
}

fn node_ref_struct_variant(item: &ItemStruct) -> proc_macro2::TokenStream {
    let ident = &item.ident;
    quote!(#ident(&'tree #ident))
}

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

fn node_ref_struct_constructor_arm(item: &ItemStruct) -> proc_macro2::TokenStream {
    let ident = &item.ident;
    let constructor = ident.to_string();
    quote!(NodeRef::#ident(..) => #constructor)
}

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

fn node_ref_struct_is_variant_arm(item: &ItemStruct) -> proc_macro2::TokenStream {
    let ident = &item.ident;
    quote!(NodeRef::#ident(..) => false)
}

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

fn node_ref_variant_ident(enum_ident: &Ident, variant_ident: &Ident) -> Ident {
    format_ident!("{enum_ident}{variant_ident}")
}

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

fn atom_variant_ident(ty: &Type) -> Ident {
    let mut text = quote!(#ty)
        .to_string()
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .collect::<String>();
    if text.is_empty() {
        text = "Atom".to_owned();
    }
    if text.chars().next().is_some_and(|ch| ch.is_ascii_digit()) {
        text.insert_str(0, "Atom");
    }
    format_ident!("{text}")
}

fn wrapper_trait_impls(
    include_recovered: bool,
    include_with_free_modifiers: bool,
) -> proc_macro2::TokenStream {
    let recovered_impl = include_recovered.then(|| {
        quote! {
            impl<T: TreeNode> TreeNode for Recovered<T> {
                fn visit_in_order<'tree, V>(&'tree self, visitor: &mut V)
                where
                    V: ::jbotci_tree::TreeVisitor<'tree, Node = NodeRef<'tree>, Atom = AtomRef<'tree>>,
                {
                    match self {
                        ::jbotci_tree::Recovered::Valid(value) => value.visit_in_order(visitor),
                        ::jbotci_tree::Recovered::Error(item) => visitor.visit_recovered_error(item),
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
                    }
                }

                fn node_at_path_steps<'tree>(
                    &'tree self,
                    steps: &[::jbotci_tree::TreePathStep],
                ) -> Option<NodeRef<'tree>> {
                    match self {
                        ::jbotci_tree::Recovered::Valid(value) => value.node_at_path_steps(steps),
                        ::jbotci_tree::Recovered::Error(_) => None,
                    }
                }
            }
        }
    });
    let with_free_modifiers_impl = include_with_free_modifiers.then(|| {
        quote! {
            impl<T: TreeNode> TreeNode for WithFreeModifiers<T> {
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

fn enum_uses_data_patterns(item: &ItemEnum) -> bool {
    item.attrs
        .iter()
        .filter(|attr| {
            attr.path().is_ident("invariant") || attr.path().is_ident("expensive_invariant")
        })
        .any(|attr| !attr_is_true_contract_marker(attr))
}

fn attr_is_true_contract_marker(attr: &Attribute) -> bool {
    let syn::Meta::List(list) = &attr.meta else {
        return false;
    };
    attribute_segments(list.tokens.clone())
        .into_iter()
        .all(segment_is_true_contract_marker)
}

fn segment_is_true_contract_marker(segment: proc_macro2::TokenStream) -> bool {
    if let Some(expr) = variant_contract_expr(segment.clone()) {
        return expr_is_true_literal(&expr);
    }
    let Ok(expr) = syn::parse2::<syn::Expr>(segment) else {
        return false;
    };
    expr_is_true_literal(&expr)
}

fn variant_contract_expr(segment: proc_macro2::TokenStream) -> Option<syn::Expr> {
    let tokens = segment.into_iter().collect::<Vec<_>>();
    if !starts_with_double_colon(&tokens) {
        return None;
    }
    let arrow_index = top_level_fat_arrow_index(&tokens)?;
    let expr_tokens = tokens
        .into_iter()
        .skip(arrow_index + 2)
        .collect::<proc_macro2::TokenStream>();
    syn::parse2::<syn::Expr>(expr_tokens).ok()
}

fn expr_is_true_literal(expr: &syn::Expr) -> bool {
    matches!(
        expr,
        syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Bool(lit),
            ..
        }) if lit.value
    )
}

fn attribute_segments(tokens: proc_macro2::TokenStream) -> Vec<proc_macro2::TokenStream> {
    let mut segments = Vec::new();
    let mut segment = Vec::new();
    for token in tokens {
        match token {
            TokenTree::Punct(punct)
                if punct.as_char() == ',' && punct.spacing() == Spacing::Alone =>
            {
                if !segment.is_empty() {
                    segments.push(segment.into_iter().collect());
                    segment = Vec::new();
                }
            }
            token => segment.push(token),
        }
    }
    if !segment.is_empty() {
        segments.push(segment.into_iter().collect());
    }
    segments
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

fn field_name_tokens(field: &syn::Field) -> proc_macro2::TokenStream {
    let name = field.ident.as_ref().map(Ident::to_string);
    match name {
        Some(name) => quote!(Some(#name)),
        None => quote!(None),
    }
}

#[derive(Debug, Clone, Copy)]
struct TreeChildFlags {
    primary: bool,
    skip: bool,
}

fn tree_child_flags(attrs: &[Attribute]) -> syn::Result<TreeChildFlags> {
    let mut flags = TreeChildFlags {
        primary: false,
        skip: false,
    };
    for attr in attrs
        .iter()
        .filter(|attr| attr.path().is_ident("tree_child"))
    {
        if attr
            .parse_args::<syn::LitBool>()
            .is_ok_and(|lit| !lit.value)
        {
            flags.skip = true;
            continue;
        }
        let ident = attr.parse_args::<Ident>()?;
        if ident == "primary" {
            flags.primary = true;
        } else {
            return Err(syn::Error::new_spanned(
                attr,
                "supported tree_child flags are `primary` and `false`",
            ));
        }
    }
    if flags.primary && flags.skip {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            "`tree_child(primary)` cannot be combined with `tree_child(false)`",
        ));
    }
    Ok(flags)
}
