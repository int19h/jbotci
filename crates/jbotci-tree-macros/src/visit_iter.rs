//! Heap-stack drivers for `TreeNode` visit, path search, and path lookup.
//!
//! The generated loop expands `Box`, `Vec`, and the other wrappers itself.
//! A chain of nodes therefore stays at a constant call depth.

use std::collections::{BTreeMap, BTreeSet};

#[allow(unused_imports)]
use bityzba::{data, ensures, invariant, new, requires};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Fields, Ident, Item, ItemEnum, ItemStruct, Type};

#[invariant(true)]
#[invariant(::Node => true)]
#[invariant(::Atom(_) => true)]
#[invariant(::Transparent(_) => true)]
#[invariant(::Option(_) => true)]
#[invariant(::Sequence(_) => true)]
#[invariant(::Chain(_, _) => true)]
#[invariant(::Free(_, _) => true)]
#[invariant(::Recovered(_) => true)]
#[invariant(::Tuple(_) => true)]
enum Shape<'a> {
    Node,
    Atom(&'a Type),
    Transparent(&'a Type),
    Option(&'a Type),
    Sequence(&'a Type),
    Chain(&'a Type, &'a Type),
    Free(&'a Type, &'a Type),
    Recovered(&'a Type),
    Tuple(Vec<&'a Type>),
}

/// The counter stays below `u32::MAX`. The next call stores one larger value.
#[invariant(*next < u32::MAX)]
struct Gen {
    next: u32,
}

impl Gen {
    #[requires(true)]
    #[ensures(true)]
    fn new() -> Self {
        new!(Gen { next: 0 })
    }

    #[requires(self.next < u32::MAX - 1)]
    #[ensures(!ret.to_string().is_empty())]
    fn ident(&mut self, prefix: &str) -> Ident {
        let id = self.next;
        let next = id + 1;
        let updated =
            std::mem::replace(self, new!(Gen { next: 0 })).with_data(data! { next: next });
        *self = updated;
        format_ident!("{prefix}_{id}")
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn expand(
    items: &[Item],
    node_names: &BTreeSet<String>,
    aliases: &BTreeMap<String, Type>,
    atom_types: &BTreeMap<String, Type>,
    recovered_error: Option<TokenStream>,
) -> syn::Result<TokenStream> {
    let mut ids = Gen::new();
    let mut push_arms = Vec::new();
    let mut edge_arms = Vec::new();
    let mut take_arms = Vec::new();
    for item in items {
        match item {
            Item::Struct(item) => {
                let (push_arm, edge_arm, take_arm) = struct_arms(
                    item,
                    node_names,
                    aliases,
                    atom_types,
                    recovered_error.as_ref(),
                    &mut ids,
                )?;
                push_arms.push(push_arm);
                edge_arms.push(edge_arm);
                take_arms.push(take_arm);
            }
            Item::Enum(item) => {
                let (pushes, edges, takes) = enum_arms(
                    item,
                    node_names,
                    aliases,
                    atom_types,
                    recovered_error.as_ref(),
                    &mut ids,
                )?;
                push_arms.extend(pushes);
                edge_arms.extend(edges);
                take_arms.extend(takes);
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
    let recovered_variant = recovered_error.as_ref().map(|ty| {
        quote! {
            RecoveredError(&'tree #ty),
        }
    });
    let recovered_arm = recovered_error.as_ref().map(|_| {
        quote! {
            JbotciWalkFrame::RecoveredError(item) => visitor.visit_recovered_error(item),
        }
    });
    Ok(quote! {
        enum JbotciWalkFrame<'tree> {
            EnterNode(NodeRef<'tree>),
            ExitNode(NodeRef<'tree>),
            EnterField(::jbotci_tree::FieldRef),
            ExitField(::jbotci_tree::FieldRef),
            Absent(::jbotci_tree::FieldRef),
            EnterSequence,
            ExitSequence,
            EnterChain,
            ExitChain,
            Atom(AtomRef<'tree>),
            #recovered_variant
        }

        struct JbotciPathEdge<'tree> {
            steps: ::std::vec::Vec<::jbotci_tree::TreePathStep>,
            node: NodeRef<'tree>,
        }

        enum JbotciPathJob<'tree> {
            Expand(NodeRef<'tree>),
            Edge {
                steps: ::std::vec::Vec<::jbotci_tree::TreePathStep>,
                node: NodeRef<'tree>,
            },
            Pop(usize),
        }

        fn jbotci_drive_visit<'tree, V>(root: NodeRef<'tree>, visitor: &mut V)
        where
            V: ::jbotci_tree::TreeVisitor<'tree, Node = NodeRef<'tree>, Atom = AtomRef<'tree>>,
        {
            let mut stack = ::std::vec::Vec::new();
            stack.push(JbotciWalkFrame::EnterNode(root));
            while let Some(frame) = stack.pop() {
                match frame {
                    JbotciWalkFrame::EnterNode(node) => {
                        visitor.enter_node(node);
                        stack.push(JbotciWalkFrame::ExitNode(node));
                        node.push_child_frames(&mut stack);
                    }
                    JbotciWalkFrame::ExitNode(node) => visitor.exit_node(node),
                    JbotciWalkFrame::EnterField(field) => visitor.enter_field(field),
                    JbotciWalkFrame::ExitField(field) => visitor.exit_field(field),
                    JbotciWalkFrame::Absent(field) => visitor.visit_absent_optional_field(field),
                    JbotciWalkFrame::EnterSequence => visitor.enter_sequence(),
                    JbotciWalkFrame::ExitSequence => visitor.exit_sequence(),
                    JbotciWalkFrame::EnterChain => visitor.enter_chain(),
                    JbotciWalkFrame::ExitChain => visitor.exit_chain(),
                    JbotciWalkFrame::Atom(atom) => visitor.visit_atom(atom),
                    #recovered_arm
                }
            }
        }

        fn jbotci_drive_path<'tree>(
            root: NodeRef<'tree>,
            target: NodeRef<'tree>,
            path: &mut ::jbotci_tree::TreePath,
        ) -> bool {
            if root == target {
                return true;
            }
            let mut stack = ::std::vec::Vec::new();
            stack.push(JbotciPathJob::Expand(root));
            while let Some(job) = stack.pop() {
                match job {
                    JbotciPathJob::Pop(count) => {
                        for _ in 0..count {
                            path.pop();
                        }
                    }
                    JbotciPathJob::Edge { steps, node } => {
                        let count = steps.len();
                        for step in steps {
                            path.push(step);
                        }
                        if node == target {
                            return true;
                        }
                        stack.push(JbotciPathJob::Pop(count));
                        stack.push(JbotciPathJob::Expand(node));
                    }
                    JbotciPathJob::Expand(node) => {
                        let mut edges = node.path_edges();
                        edges.reverse();
                        for edge in edges {
                            stack.push(JbotciPathJob::Edge {
                                steps: edge.steps,
                                node: edge.node,
                            });
                        }
                    }
                }
            }
            false
        }

        fn jbotci_drive_node_at<'tree>(
            mut node: NodeRef<'tree>,
            mut steps: &[::jbotci_tree::TreePathStep],
        ) -> Option<NodeRef<'tree>> {
            loop {
                if steps.is_empty() {
                    return Some(node);
                }
                let (next, used) = node.take_steps(steps)?;
                if used == 0 || used > steps.len() {
                    return None;
                }
                node = next;
                steps = &steps[used..];
            }
        }

        impl<'tree> NodeRef<'tree> {
            fn push_child_frames(self, stack: &mut ::std::vec::Vec<JbotciWalkFrame<'tree>>) {
                match self {
                    #(#push_arms)*
                }
            }

            fn path_edges(self) -> ::std::vec::Vec<JbotciPathEdge<'tree>> {
                let mut edges = ::std::vec::Vec::new();
                match self {
                    #(#edge_arms)*
                }
                edges
            }

            fn take_steps(
                self,
                steps: &[::jbotci_tree::TreePathStep],
            ) -> Option<(NodeRef<'tree>, usize)> {
                let (step, rest) = steps.split_first()?;
                match self {
                    #(#take_arms)*
                }
            }
        }
    })
}

#[requires(true)]
#[ensures(true)]
fn struct_arms(
    item: &ItemStruct,
    node_names: &BTreeSet<String>,
    aliases: &BTreeMap<String, Type>,
    atom_types: &BTreeMap<String, Type>,
    recovered_error: Option<&TokenStream>,
    ids: &mut Gen,
) -> syn::Result<(TokenStream, TokenStream, TokenStream)> {
    let ident = &item.ident;
    let fields = field_rows(&item.fields, false)?;
    let push_body = push_fields(
        &fields,
        node_names,
        aliases,
        atom_types,
        recovered_error,
        ids,
    )?;
    let edge_body = edge_fields(
        &fields,
        node_names,
        aliases,
        atom_types,
        recovered_error,
        ids,
    )?;
    let take_body = take_fields(
        &fields,
        node_names,
        aliases,
        atom_types,
        recovered_error,
        ids,
    )?;
    Ok((
        quote! {
            NodeRef::#ident(value) => {
                #push_body
            }
        },
        quote! {
            NodeRef::#ident(value) => {
                #edge_body
            }
        },
        quote! {
            NodeRef::#ident(value) => {
                #take_body
                None
            }
        },
    ))
}

#[requires(true)]
#[ensures(true)]
fn enum_arms(
    item: &ItemEnum,
    node_names: &BTreeSet<String>,
    aliases: &BTreeMap<String, Type>,
    atom_types: &BTreeMap<String, Type>,
    recovered_error: Option<&TokenStream>,
    ids: &mut Gen,
) -> syn::Result<(Vec<TokenStream>, Vec<TokenStream>, Vec<TokenStream>)> {
    let enum_ident = &item.ident;
    let uses_data = super::enum_uses_data_patterns(item);
    let mut pushes = Vec::new();
    let mut edges = Vec::new();
    let mut takes = Vec::new();
    for variant in &item.variants {
        let variant_ident = &variant.ident;
        let node_variant = super::node_ref_variant_ident(enum_ident, variant_ident);
        let fields = field_rows(&variant.fields, true)?;
        let pattern = variant_pattern(enum_ident, variant_ident, &variant.fields, uses_data)?;
        let push_body = push_fields(
            &fields,
            node_names,
            aliases,
            atom_types,
            recovered_error,
            ids,
        )?;
        let edge_body = edge_fields(
            &fields,
            node_names,
            aliases,
            atom_types,
            recovered_error,
            ids,
        )?;
        let take_body = take_fields(
            &fields,
            node_names,
            aliases,
            atom_types,
            recovered_error,
            ids,
        )?;
        let match_value = if uses_data {
            quote!(value.as_data())
        } else {
            quote!(value)
        };
        pushes.push(quote! {
            NodeRef::#node_variant(value) => {
                #[allow(unreachable_patterns)]
                match #match_value {
                    #pattern => {
                        #push_body
                    }
                    _ => {}
                }
            }
        });
        edges.push(quote! {
            NodeRef::#node_variant(value) => {
                #[allow(unreachable_patterns)]
                match #match_value {
                    #pattern => {
                        #edge_body
                    }
                    _ => {}
                }
            }
        });
        takes.push(quote! {
            NodeRef::#node_variant(value) => {
                #[allow(unreachable_patterns)]
                match #match_value {
                    #pattern => {
                        #take_body
                        None
                    }
                    _ => None,
                }
            }
        });
    }
    Ok((pushes, edges, takes))
}

/// `option_field` matches an `Option` path. A row is not both primary and skipped.
/// `index` is the field position. Every index is valid for some field list.
#[invariant(*option_field == super::field_is_option(ty))]
#[invariant(!(*primary && *skip))]
struct FieldRow {
    index: usize,
    name: TokenStream,
    primary: bool,
    ty: Type,
    access: TokenStream,
    skip: bool,
    option_field: bool,
}

#[requires(true)]
#[ensures(true)]
fn field_rows(fields: &Fields, from_binding: bool) -> syn::Result<Vec<FieldRow>> {
    let mut rows = Vec::new();
    for (index, field) in fields.iter().enumerate() {
        let flags = super::tree_child_flags(&field.attrs)?;
        let name = super::field_name_tokens(field);
        let access = if from_binding {
            match &field.ident {
                Some(ident) => quote!(#ident),
                None => {
                    let ident = format_ident!("field_{index}");
                    quote!(#ident)
                }
            }
        } else if let Some(ident) = &field.ident {
            quote!(&value.#ident)
        } else {
            let index = syn::Index::from(index);
            quote!(&value.#index)
        };
        rows.push(new!(FieldRow {
            index,
            name,
            primary: flags.primary,
            ty: field.ty.clone(),
            access,
            skip: flags.skip,
            option_field: super::field_is_option(&field.ty),
        }));
    }
    Ok(rows)
}

#[requires(true)]
#[ensures(true)]
fn variant_pattern(
    enum_ident: &Ident,
    variant_ident: &Ident,
    fields: &Fields,
    uses_data: bool,
) -> syn::Result<TokenStream> {
    let pattern = match fields {
        Fields::Named(fields) => {
            let mut parts = Vec::new();
            for field in &fields.named {
                let ident = field.ident.as_ref().unwrap();
                if super::tree_child_flags(&field.attrs)?.skip {
                    parts.push(quote!(#ident: _));
                } else {
                    parts.push(quote!(#ident));
                }
            }
            quote!(#enum_ident::#variant_ident { #(#parts,)* })
        }
        Fields::Unnamed(fields) => {
            let mut parts = Vec::new();
            for (index, field) in fields.unnamed.iter().enumerate() {
                if super::tree_child_flags(&field.attrs)?.skip {
                    parts.push(quote!(_));
                } else {
                    let ident = format_ident!("field_{index}");
                    parts.push(quote!(#ident));
                }
            }
            quote!(#enum_ident::#variant_ident(#(#parts,)*))
        }
        Fields::Unit => quote!(#enum_ident::#variant_ident),
    };
    if uses_data {
        Ok(quote!(::bityzba::data!(#pattern)))
    } else {
        Ok(pattern)
    }
}

#[requires(true)]
#[ensures(true)]
fn push_fields(
    fields: &[FieldRow],
    node_names: &BTreeSet<String>,
    aliases: &BTreeMap<String, Type>,
    atom_types: &BTreeMap<String, Type>,
    recovered_error: Option<&TokenStream>,
    ids: &mut Gen,
) -> syn::Result<TokenStream> {
    let mut parts = Vec::new();
    for field in fields.iter().rev() {
        if field.skip {
            continue;
        }
        parts.push(push_field(
            field,
            node_names,
            aliases,
            atom_types,
            recovered_error,
            ids,
        )?);
    }
    Ok(quote!(#(#parts)*))
}

#[requires(true)]
#[ensures(true)]
fn push_field(
    field: &FieldRow,
    node_names: &BTreeSet<String>,
    aliases: &BTreeMap<String, Type>,
    atom_types: &BTreeMap<String, Type>,
    recovered_error: Option<&TokenStream>,
    ids: &mut Gen,
) -> syn::Result<TokenStream> {
    let access = &field.access;
    let name = &field.name;
    let index = field.index;
    let primary = field.primary;
    let field_ref = ids.ident("field_ref");
    let bound = ids.ident("access");
    let header = quote! {
        let #field_ref = ::jbotci_tree::FieldRef::new(#name, #index, #primary);
        let #bound = #access;
        stack.push(JbotciWalkFrame::ExitField(#field_ref));
    };
    let body = if field.option_field {
        let inner_ty = option_inner(&field.ty)?;
        let inner = ids.ident("inner");
        let inner_push = push_shape(
            &classify(inner_ty, node_names, aliases, &mut BTreeSet::new())?,
            &quote!(#inner),
            node_names,
            aliases,
            atom_types,
            recovered_error,
            ids,
        )?;
        quote! {
            if let Some(#inner) = #bound {
                #inner_push
            } else {
                stack.push(JbotciWalkFrame::Absent(#field_ref));
            }
        }
    } else {
        push_shape(
            &classify(&field.ty, node_names, aliases, &mut BTreeSet::new())?,
            &quote!(#bound),
            node_names,
            aliases,
            atom_types,
            recovered_error,
            ids,
        )?
    };
    Ok(quote! {
        {
            #header
            #body
            stack.push(JbotciWalkFrame::EnterField(#field_ref));
        }
    })
}

#[requires(true)]
#[ensures(true)]
fn edge_fields(
    fields: &[FieldRow],
    node_names: &BTreeSet<String>,
    aliases: &BTreeMap<String, Type>,
    atom_types: &BTreeMap<String, Type>,
    recovered_error: Option<&TokenStream>,
    ids: &mut Gen,
) -> syn::Result<TokenStream> {
    let mut parts = Vec::new();
    for field in fields {
        if field.skip {
            continue;
        }
        let prefix = ids.ident("prefix");
        let access = &field.access;
        let bound = ids.ident("access");
        let name = &field.name;
        let index = field.index;
        let body = if field.option_field {
            let inner_ty = option_inner(&field.ty)?;
            let inner = ids.ident("inner");
            let inner_edge = edge_shape(
                &classify(inner_ty, node_names, aliases, &mut BTreeSet::new())?,
                &quote!(#inner),
                &prefix,
                node_names,
                aliases,
                atom_types,
                recovered_error,
                ids,
            )?;
            quote! {
                if let Some(#inner) = #bound {
                    #inner_edge
                }
            }
        } else {
            edge_shape(
                &classify(&field.ty, node_names, aliases, &mut BTreeSet::new())?,
                &quote!(#bound),
                &prefix,
                node_names,
                aliases,
                atom_types,
                recovered_error,
                ids,
            )?
        };
        parts.push(quote! {
            {
                let mut #prefix = ::std::vec::Vec::new();
                #prefix.push(::jbotci_tree::TreePathStep::field(#name, #index));
                let #bound = #access;
                #body
            }
        });
    }
    Ok(quote!(#(#parts)*))
}

#[requires(true)]
#[ensures(true)]
fn take_fields(
    fields: &[FieldRow],
    node_names: &BTreeSet<String>,
    aliases: &BTreeMap<String, Type>,
    atom_types: &BTreeMap<String, Type>,
    recovered_error: Option<&TokenStream>,
    ids: &mut Gen,
) -> syn::Result<TokenStream> {
    let mut parts = Vec::new();
    for field in fields {
        if field.skip {
            continue;
        }
        let access = &field.access;
        let name = &field.name;
        let index = field.index;
        let bound = ids.ident("access");
        let resolved = ids.ident("resolved");
        let node = ids.ident("node");
        let extra = ids.ident("extra");
        let body = if field.option_field {
            let inner_ty = option_inner(&field.ty)?;
            let inner = ids.ident("inner");
            let inner_resolve = resolve_shape(
                &classify(inner_ty, node_names, aliases, &mut BTreeSet::new())?,
                &quote!(#inner),
                &quote!(rest),
                node_names,
                aliases,
                atom_types,
                recovered_error,
                ids,
            )?;
            quote! {
                match #bound {
                    Some(#inner) => #inner_resolve,
                    None => None,
                }
            }
        } else {
            resolve_shape(
                &classify(&field.ty, node_names, aliases, &mut BTreeSet::new())?,
                &quote!(#bound),
                &quote!(rest),
                node_names,
                aliases,
                atom_types,
                recovered_error,
                ids,
            )?
        };
        parts.push(quote! {
            if step.is_field(#name, #index) {
                let #bound = #access;
                let #resolved: ::core::option::Option<(NodeRef<'tree>, usize)> = #body;
                return match #resolved {
                    Some((#node, #extra)) => {
                        let #extra: usize = #extra;
                        Some((#node, #extra + 1))
                    }
                    None => None,
                };
            }
        });
    }
    Ok(quote!(#(#parts)*))
}

#[requires(true)]
#[ensures(true)]
fn push_shape(
    shape: &Shape<'_>,
    access: &TokenStream,
    node_names: &BTreeSet<String>,
    aliases: &BTreeMap<String, Type>,
    atom_types: &BTreeMap<String, Type>,
    recovered_error: Option<&TokenStream>,
    ids: &mut Gen,
) -> syn::Result<TokenStream> {
    match shape {
        Shape::Node => Ok(quote! {
            stack.push(JbotciWalkFrame::EnterNode(NodeRef::from(#access)));
        }),
        Shape::Atom(ty) => {
            let variant = atom_variant(ty, atom_types)?;
            Ok(quote! {
                stack.push(JbotciWalkFrame::Atom(AtomRef::#variant(#access)));
            })
        }
        Shape::Transparent(inner) => {
            let bound = ids.ident("inner");
            let body = push_shape(
                &classify(inner, node_names, aliases, &mut BTreeSet::new())?,
                &quote!(#bound),
                node_names,
                aliases,
                atom_types,
                recovered_error,
                ids,
            )?;
            Ok(quote! {
                {
                    let #bound = #access.as_ref();
                    #body
                }
            })
        }
        Shape::Option(inner) => {
            let bound = ids.ident("inner");
            let body = push_shape(
                &classify(inner, node_names, aliases, &mut BTreeSet::new())?,
                &quote!(#bound),
                node_names,
                aliases,
                atom_types,
                recovered_error,
                ids,
            )?;
            Ok(quote! {
                if let Some(#bound) = #access {
                    #body
                }
            })
        }
        Shape::Sequence(inner) => {
            let item = ids.ident("item");
            let body = push_shape(
                &classify(inner, node_names, aliases, &mut BTreeSet::new())?,
                &quote!(#item),
                node_names,
                aliases,
                atom_types,
                recovered_error,
                ids,
            )?;
            Ok(quote! {
                stack.push(JbotciWalkFrame::ExitSequence);
                for #item in #access.iter().rev() {
                    #body
                }
                stack.push(JbotciWalkFrame::EnterSequence);
            })
        }
        Shape::Chain(first, link) => {
            let link_item = ids.ident("link");
            let first_bound = ids.ident("first");
            let link_body = push_shape(
                &classify(link, node_names, aliases, &mut BTreeSet::new())?,
                &quote!(#link_item),
                node_names,
                aliases,
                atom_types,
                recovered_error,
                ids,
            )?;
            let first_body = push_shape(
                &classify(first, node_names, aliases, &mut BTreeSet::new())?,
                &quote!(#first_bound),
                node_names,
                aliases,
                atom_types,
                recovered_error,
                ids,
            )?;
            Ok(quote! {
                stack.push(JbotciWalkFrame::ExitChain);
                for #link_item in #access.links.iter().rev() {
                    #link_body
                }
                {
                    let #first_bound = &#access.first;
                    #first_body
                }
                stack.push(JbotciWalkFrame::EnterChain);
            })
        }
        Shape::Free(value, modifiers) => {
            let value_bound = ids.ident("value");
            let mod_item = ids.ident("modifier");
            let field_ref = ids.ident("field_ref");
            let value_body = push_shape(
                &classify(value, node_names, aliases, &mut BTreeSet::new())?,
                &quote!(#value_bound),
                node_names,
                aliases,
                atom_types,
                recovered_error,
                ids,
            )?;
            let mod_body = push_shape(
                &classify(modifiers, node_names, aliases, &mut BTreeSet::new())?,
                &quote!(#mod_item),
                node_names,
                aliases,
                atom_types,
                recovered_error,
                ids,
            )?;
            Ok(quote! {
                if !#access.free_modifiers.is_empty() {
                    let #field_ref = ::jbotci_tree::FieldRef::new(Some("free_modifiers"), 1, false);
                    stack.push(JbotciWalkFrame::ExitField(#field_ref));
                    stack.push(JbotciWalkFrame::ExitSequence);
                    for #mod_item in #access.free_modifiers.iter().rev() {
                        #mod_body
                    }
                    stack.push(JbotciWalkFrame::EnterSequence);
                    stack.push(JbotciWalkFrame::EnterField(#field_ref));
                }
                {
                    let #value_bound = &#access.value;
                    #value_body
                }
            })
        }
        Shape::Recovered(inner) => {
            let Some(_) = recovered_error else {
                return Err(syn::Error::new_spanned(
                    inner,
                    "Recovered tree fields require the recovered tree module",
                ));
            };
            let value = ids.ident("value");
            let item = ids.ident("item");
            let prefix = ids.ident("prefix");
            let body = push_shape(
                &classify(inner, node_names, aliases, &mut BTreeSet::new())?,
                &quote!(#value),
                node_names,
                aliases,
                atom_types,
                recovered_error,
                ids,
            )?;
            Ok(quote! {
                match #access {
                    ::jbotci_tree::Recovered::Valid(#value) => {
                        let #value = #value.as_ref();
                        #body
                    }
                    ::jbotci_tree::Recovered::Error(#item) => {
                        stack.push(JbotciWalkFrame::RecoveredError(#item));
                    }
                    ::jbotci_tree::Recovered::Prefix(#prefix) => {
                        {
                            let #value = #prefix.value.as_ref();
                            #body
                        }
                        for #item in #prefix.errors.iter().rev() {
                            stack.push(JbotciWalkFrame::RecoveredError(#item));
                        }
                    }
                }
            })
        }
        Shape::Tuple(elems) => {
            let mut bodies = Vec::new();
            for elem in elems.iter().rev() {
                let bound = ids.ident("elem");
                let body = push_shape(
                    &classify(elem, node_names, aliases, &mut BTreeSet::new())?,
                    &quote!(#bound),
                    node_names,
                    aliases,
                    atom_types,
                    recovered_error,
                    ids,
                )?;
                let index = syn::Index::from(elems.len() - 1 - bodies.len());
                bodies.push(quote! {
                    {
                        let #bound = &#access.#index;
                        #body
                    }
                });
            }
            Ok(quote! {
                stack.push(JbotciWalkFrame::ExitSequence);
                #(#bodies)*
                stack.push(JbotciWalkFrame::EnterSequence);
            })
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn edge_shape(
    shape: &Shape<'_>,
    access: &TokenStream,
    prefix: &Ident,
    node_names: &BTreeSet<String>,
    aliases: &BTreeMap<String, Type>,
    atom_types: &BTreeMap<String, Type>,
    recovered_error: Option<&TokenStream>,
    ids: &mut Gen,
) -> syn::Result<TokenStream> {
    let _ = (atom_types, recovered_error);
    match shape {
        Shape::Node => Ok(quote! {
            edges.push(JbotciPathEdge {
                steps: #prefix.clone(),
                node: NodeRef::from(#access),
            });
        }),
        Shape::Atom(_) => Ok(quote!()),
        Shape::Transparent(inner) => {
            let bound = ids.ident("inner");
            let body = edge_shape(
                &classify(inner, node_names, aliases, &mut BTreeSet::new())?,
                &quote!(#bound),
                prefix,
                node_names,
                aliases,
                atom_types,
                recovered_error,
                ids,
            )?;
            Ok(quote! {
                {
                    let #bound = #access.as_ref();
                    #body
                }
            })
        }
        Shape::Option(inner) => {
            let bound = ids.ident("inner");
            let body = edge_shape(
                &classify(inner, node_names, aliases, &mut BTreeSet::new())?,
                &quote!(#bound),
                prefix,
                node_names,
                aliases,
                atom_types,
                recovered_error,
                ids,
            )?;
            Ok(quote! {
                if let Some(#bound) = #access {
                    #body
                }
            })
        }
        Shape::Sequence(inner) => {
            let index = ids.ident("index");
            let item = ids.ident("item");
            let next = ids.ident("prefix");
            let body = edge_shape(
                &classify(inner, node_names, aliases, &mut BTreeSet::new())?,
                &quote!(#item),
                &next,
                node_names,
                aliases,
                atom_types,
                recovered_error,
                ids,
            )?;
            Ok(quote! {
                for (#index, #item) in #access.iter().enumerate() {
                    let mut #next = #prefix.clone();
                    #next.push(::jbotci_tree::TreePathStep::sequence_index(#index));
                    #body
                }
            })
        }
        Shape::Chain(first, link) => {
            let first_bound = ids.ident("first");
            let first_prefix = ids.ident("prefix");
            let link_index = ids.ident("index");
            let link_item = ids.ident("link");
            let link_prefix = ids.ident("prefix");
            let first_body = edge_shape(
                &classify(first, node_names, aliases, &mut BTreeSet::new())?,
                &quote!(#first_bound),
                &first_prefix,
                node_names,
                aliases,
                atom_types,
                recovered_error,
                ids,
            )?;
            let link_body = edge_shape(
                &classify(link, node_names, aliases, &mut BTreeSet::new())?,
                &quote!(#link_item),
                &link_prefix,
                node_names,
                aliases,
                atom_types,
                recovered_error,
                ids,
            )?;
            Ok(quote! {
                {
                    let #first_bound = &#access.first;
                    let mut #first_prefix = #prefix.clone();
                    #first_prefix.push(::jbotci_tree::TreePathStep::sequence_index(0));
                    #first_body
                }
                for (#link_index, #link_item) in #access.links.iter().enumerate() {
                    let mut #link_prefix = #prefix.clone();
                    #link_prefix.push(::jbotci_tree::TreePathStep::sequence_index(
                        #link_index + 1usize,
                    ));
                    #link_body
                }
            })
        }
        Shape::Free(value, modifiers) => {
            let value_bound = ids.ident("value");
            let value_body = edge_shape(
                &classify(value, node_names, aliases, &mut BTreeSet::new())?,
                &quote!(#value_bound),
                prefix,
                node_names,
                aliases,
                atom_types,
                recovered_error,
                ids,
            )?;
            let mod_prefix = ids.ident("prefix");
            let mod_index = ids.ident("index");
            let mod_item = ids.ident("modifier");
            let mod_next = ids.ident("prefix");
            let mod_body = edge_shape(
                &classify(modifiers, node_names, aliases, &mut BTreeSet::new())?,
                &quote!(#mod_item),
                &mod_next,
                node_names,
                aliases,
                atom_types,
                recovered_error,
                ids,
            )?;
            Ok(quote! {
                {
                    let #value_bound = &#access.value;
                    #value_body
                }
                if !#access.free_modifiers.is_empty() {
                    let mut #mod_prefix = #prefix.clone();
                    #mod_prefix.push(::jbotci_tree::TreePathStep::field(Some("free_modifiers"), 1));
                    for (#mod_index, #mod_item) in #access.free_modifiers.iter().enumerate() {
                        let mut #mod_next = #mod_prefix.clone();
                        #mod_next.push(::jbotci_tree::TreePathStep::sequence_index(#mod_index));
                        #mod_body
                    }
                }
            })
        }
        Shape::Recovered(inner) => {
            let value = ids.ident("value");
            let prefix_value = ids.ident("prefix");
            let body = edge_shape(
                &classify(inner, node_names, aliases, &mut BTreeSet::new())?,
                &quote!(#value),
                prefix,
                node_names,
                aliases,
                atom_types,
                recovered_error,
                ids,
            )?;
            Ok(quote! {
                match #access {
                    ::jbotci_tree::Recovered::Valid(#value) => {
                        let #value = #value.as_ref();
                        #body
                    }
                    ::jbotci_tree::Recovered::Error(_) => {}
                    ::jbotci_tree::Recovered::Prefix(#prefix_value) => {
                        let #value = #prefix_value.value.as_ref();
                        #body
                    }
                }
            })
        }
        Shape::Tuple(elems) => {
            let mut bodies = Vec::new();
            for (index, elem) in elems.iter().enumerate() {
                let bound = ids.ident("elem");
                let next = ids.ident("prefix");
                let body = edge_shape(
                    &classify(elem, node_names, aliases, &mut BTreeSet::new())?,
                    &quote!(#bound),
                    &next,
                    node_names,
                    aliases,
                    atom_types,
                    recovered_error,
                    ids,
                )?;
                let position = syn::Index::from(index);
                bodies.push(quote! {
                    {
                        let #bound = &#access.#position;
                        let mut #next = #prefix.clone();
                        #next.push(::jbotci_tree::TreePathStep::sequence_index(#index));
                        #body
                    }
                });
            }
            Ok(quote!(#(#bodies)*))
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn resolve_shape(
    shape: &Shape<'_>,
    access: &TokenStream,
    rest: &TokenStream,
    node_names: &BTreeSet<String>,
    aliases: &BTreeMap<String, Type>,
    atom_types: &BTreeMap<String, Type>,
    recovered_error: Option<&TokenStream>,
    ids: &mut Gen,
) -> syn::Result<TokenStream> {
    let _ = atom_types;
    match shape {
        Shape::Node => Ok(quote!(Some((NodeRef::from(#access), 0usize)))),
        Shape::Atom(_) => Ok(quote!(None)),
        Shape::Transparent(inner) => {
            let bound = ids.ident("inner");
            let body = resolve_shape(
                &classify(inner, node_names, aliases, &mut BTreeSet::new())?,
                &quote!(#bound),
                rest,
                node_names,
                aliases,
                atom_types,
                recovered_error,
                ids,
            )?;
            Ok(quote! {
                {
                    let #bound = #access.as_ref();
                    #body
                }
            })
        }
        Shape::Option(inner) => {
            let bound = ids.ident("inner");
            let body = resolve_shape(
                &classify(inner, node_names, aliases, &mut BTreeSet::new())?,
                &quote!(#bound),
                rest,
                node_names,
                aliases,
                atom_types,
                recovered_error,
                ids,
            )?;
            Ok(quote! {
                match #access {
                    Some(#bound) => #body,
                    None => None,
                }
            })
        }
        Shape::Sequence(inner) => {
            let step = ids.ident("step");
            let next = ids.ident("rest");
            let index = ids.ident("index");
            let item = ids.ident("item");
            let node = ids.ident("node");
            let extra = ids.ident("extra");
            let body = resolve_shape(
                &classify(inner, node_names, aliases, &mut BTreeSet::new())?,
                &quote!(#item),
                &quote!(#next),
                node_names,
                aliases,
                atom_types,
                recovered_error,
                ids,
            )?;
            Ok(quote! {
                match #rest.split_first() {
                    Some((#step, #next)) => match #step.as_sequence_index() {
                        Some(#index) => match #access.get(#index) {
                            Some(#item) => match #body {
                                Some((#node, #extra)) => {
                                    let #extra: usize = #extra;
                                    Some((#node, #extra + 1))
                                }
                                None => None,
                            },
                            None => None,
                        },
                        None => None,
                    },
                    None => None,
                }
            })
        }
        Shape::Chain(first, link) => {
            let step = ids.ident("step");
            let next = ids.ident("rest");
            let index = ids.ident("index");
            let item = ids.ident("item");
            let node = ids.ident("node");
            let extra = ids.ident("extra");
            let first_bound = ids.ident("first");
            let first_body = resolve_shape(
                &classify(first, node_names, aliases, &mut BTreeSet::new())?,
                &quote!(#first_bound),
                &quote!(#next),
                node_names,
                aliases,
                atom_types,
                recovered_error,
                ids,
            )?;
            let link_body = resolve_shape(
                &classify(link, node_names, aliases, &mut BTreeSet::new())?,
                &quote!(#item),
                &quote!(#next),
                node_names,
                aliases,
                atom_types,
                recovered_error,
                ids,
            )?;
            Ok(quote! {
                match #rest.split_first() {
                    Some((#step, #next)) => match #step.as_sequence_index() {
                        Some(0) => {
                            let #first_bound = &#access.first;
                            match #first_body {
                                Some((#node, #extra)) => {
                                    let #extra: usize = #extra;
                                    Some((#node, #extra + 1))
                                }
                                None => None,
                            }
                        }
                        Some(#index) => match #access.links.iter().nth(#index - 1usize) {
                            Some(#item) => match #link_body {
                                Some((#node, #extra)) => {
                                    let #extra: usize = #extra;
                                    Some((#node, #extra + 1))
                                }
                                None => None,
                            },
                            None => None,
                        },
                        None => None,
                    },
                    None => None,
                }
            })
        }
        Shape::Free(value, modifiers) => resolve_free(
            value,
            modifiers,
            access,
            rest,
            node_names,
            aliases,
            atom_types,
            recovered_error,
            ids,
        ),
        Shape::Recovered(inner) => {
            let value = ids.ident("value");
            let prefix = ids.ident("prefix");
            let body = resolve_shape(
                &classify(inner, node_names, aliases, &mut BTreeSet::new())?,
                &quote!(#value),
                rest,
                node_names,
                aliases,
                atom_types,
                recovered_error,
                ids,
            )?;
            Ok(quote! {
                match #access {
                    ::jbotci_tree::Recovered::Valid(#value) => {
                        let #value = #value.as_ref();
                        #body
                    }
                    ::jbotci_tree::Recovered::Error(_) => None,
                    ::jbotci_tree::Recovered::Prefix(#prefix) => {
                        let #value = #prefix.value.as_ref();
                        #body
                    }
                }
            })
        }
        Shape::Tuple(elems) => {
            let step = ids.ident("step");
            let next = ids.ident("rest");
            let mut arms = Vec::new();
            for (position, elem) in elems.iter().enumerate() {
                let bound = ids.ident("elem");
                let node = ids.ident("node");
                let extra = ids.ident("extra");
                let body = resolve_shape(
                    &classify(elem, node_names, aliases, &mut BTreeSet::new())?,
                    &quote!(#bound),
                    &quote!(#next),
                    node_names,
                    aliases,
                    atom_types,
                    recovered_error,
                    ids,
                )?;
                let field = syn::Index::from(position);
                arms.push(quote! {
                    Some(#position) => {
                        let #bound = &#access.#field;
                        match #body {
                            Some((#node, #extra)) => {
                                let #extra: usize = #extra;
                                Some((#node, #extra + 1))
                            }
                            None => None,
                        }
                    }
                });
            }
            Ok(quote! {
                match #rest.split_first() {
                    Some((#step, #next)) => match #step.as_sequence_index() {
                        #(#arms)*
                        Some(_) => None,
                        None => None,
                    },
                    None => None,
                }
            })
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn resolve_free(
    value: &Type,
    modifiers: &Type,
    access: &TokenStream,
    rest: &TokenStream,
    node_names: &BTreeSet<String>,
    aliases: &BTreeMap<String, Type>,
    atom_types: &BTreeMap<String, Type>,
    recovered_error: Option<&TokenStream>,
    ids: &mut Gen,
) -> syn::Result<TokenStream> {
    let value_shape = classify(value, node_names, aliases, &mut BTreeSet::new())?;
    let value_bound = ids.ident("value");
    let found = ids.ident("found");
    let step = ids.ident("step");
    let next = ids.ident("rest");
    let node = ids.ident("node");
    let extra = ids.ident("extra");
    let mods = ids.ident("modifiers");
    let mod_resolve = resolve_shape(
        &Shape::Sequence(modifiers),
        &quote!(#mods),
        &quote!(#next),
        node_names,
        aliases,
        atom_types,
        recovered_error,
        ids,
    )?;
    let fallback = quote! {
        match #rest.split_first() {
            Some((#step, #next)) if #step.is_field(Some("free_modifiers"), 1) => {
                let #mods = &#access.free_modifiers;
                match #mod_resolve {
                    Some((#node, #extra)) => {
                        let #extra: usize = #extra;
                        Some((#node, #extra + 1))
                    }
                    None => None,
                }
            }
            _ => None,
        }
    };
    if spine_is_node(&value_shape, node_names, aliases)? {
        let empty = ids.ident("empty");
        let start = ids.ident("node");
        let start_extra = ids.ident("extra");
        let open = resolve_shape(
            &value_shape,
            &quote!(#value_bound),
            &quote!(#empty),
            node_names,
            aliases,
            atom_types,
            recovered_error,
            ids,
        )?;
        Ok(quote! {
            {
                let #value_bound = &#access.value;
                let #empty: &[::jbotci_tree::TreePathStep] = &[];
                match #open {
                    Some((#start, #start_extra)) if #start_extra == 0 => {
                        match jbotci_drive_node_at(#start, #rest) {
                            Some(#found) => Some((#found, #rest.len())),
                            None => #fallback,
                        }
                    }
                    _ => #fallback,
                }
            }
        })
    } else {
        Ok(quote! {
            {
                let #value_bound = &#access.value;
                if let Some(#found) = TreeNode::node_at_path_steps(#value_bound, #rest) {
                    Some((#found, #rest.len()))
                } else {
                    #fallback
                }
            }
        })
    }
}

#[requires(true)]
#[ensures(true)]
fn spine_is_node(
    shape: &Shape<'_>,
    node_names: &BTreeSet<String>,
    aliases: &BTreeMap<String, Type>,
) -> syn::Result<bool> {
    match shape {
        Shape::Node => Ok(true),
        Shape::Transparent(inner) | Shape::Option(inner) | Shape::Recovered(inner) => {
            let inner = classify(inner, node_names, aliases, &mut BTreeSet::new())?;
            spine_is_node(&inner, node_names, aliases)
        }
        Shape::Atom(_)
        | Shape::Sequence(_)
        | Shape::Chain(_, _)
        | Shape::Free(_, _)
        | Shape::Tuple(_) => Ok(false),
    }
}

#[requires(true)]
#[ensures(true)]
fn classify<'a>(
    ty: &'a Type,
    node_names: &BTreeSet<String>,
    aliases: &'a BTreeMap<String, Type>,
    seen: &mut BTreeSet<String>,
) -> syn::Result<Shape<'a>> {
    match ty {
        Type::Paren(inner) => classify(&inner.elem, node_names, aliases, seen),
        Type::Group(inner) => classify(&inner.elem, node_names, aliases, seen),
        Type::Reference(reference) => Err(syn::Error::new_spanned(
            reference,
            "tree walk fields cannot use reference types",
        )),
        Type::Tuple(tuple) => Ok(Shape::Tuple(tuple.elems.iter().collect())),
        Type::Path(path) => classify_path(ty, path, node_names, aliases, seen),
        other => Ok(Shape::Atom(other)),
    }
}

#[requires(true)]
#[ensures(true)]
fn classify_path<'a>(
    ty: &'a Type,
    path: &'a syn::TypePath,
    node_names: &BTreeSet<String>,
    aliases: &'a BTreeMap<String, Type>,
    seen: &mut BTreeSet<String>,
) -> syn::Result<Shape<'a>> {
    let Some(segment) = path.path.segments.last() else {
        return Ok(Shape::Atom(ty));
    };
    if path.qself.is_none() && super::is_wrapper_ident(&segment.ident) {
        return classify_wrapper(ty, &segment.ident, &segment.arguments, node_names, aliases);
    }
    if path.qself.is_none()
        && path.path.segments.len() == 1
        && let Some(alias) = aliases.get(&segment.ident.to_string())
        && seen.insert(segment.ident.to_string())
    {
        return classify(alias, node_names, aliases, seen);
    }
    if path.path.segments.len() == 1 && node_names.contains(&segment.ident.to_string()) {
        Ok(Shape::Node)
    } else {
        Ok(Shape::Atom(ty))
    }
}

#[requires(true)]
#[ensures(true)]
fn classify_wrapper<'a>(
    ty: &'a Type,
    name: &Ident,
    arguments: &'a syn::PathArguments,
    node_names: &BTreeSet<String>,
    aliases: &'a BTreeMap<String, Type>,
) -> syn::Result<Shape<'a>> {
    let _ = (node_names, aliases);
    let args = super::type_arguments(arguments);
    let text = name.to_string();
    match text.as_str() {
        "Box" | "Arc" | "TreeSlot" => Ok(Shape::Transparent(first_arg(ty, &args)?)),
        "Option" => Ok(Shape::Option(first_arg(ty, &args)?)),
        "Vec" | "Vec1" => Ok(Shape::Sequence(first_arg(ty, &args)?)),
        "SmallVec" | "SmallVec1" => Ok(Shape::Sequence(super::smallvec_item_type(first_arg(
            ty, &args,
        )?))),
        "Chain" => {
            let first = args.first().copied().ok_or_else(|| {
                syn::Error::new_spanned(ty, "Chain tree fields need a first type argument")
            })?;
            let links = args.get(1).copied().ok_or_else(|| {
                syn::Error::new_spanned(ty, "Chain tree fields need a links type argument")
            })?;
            Ok(Shape::Chain(first, sequence_elem(links)?))
        }
        "WithFreeModifiers" => {
            let value = first_arg(ty, &args)?;
            let modifiers = args.get(1).copied().ok_or_else(|| {
                syn::Error::new_spanned(
                    ty,
                    "WithFreeModifiers tree fields need a modifier type argument",
                )
            })?;
            Ok(Shape::Free(value, modifiers))
        }
        "Recovered" => Ok(Shape::Recovered(first_arg(ty, &args)?)),
        _ => Err(syn::Error::new_spanned(
            ty,
            "tree walk does not know this wrapper type",
        )),
    }
}

#[requires(true)]
#[ensures(true)]
fn first_arg<'a>(ty: &'a Type, args: &[&'a Type]) -> syn::Result<&'a Type> {
    args.first()
        .copied()
        .ok_or_else(|| syn::Error::new_spanned(ty, "tree wrapper is missing its type argument"))
}

#[requires(true)]
#[ensures(true)]
fn sequence_elem(ty: &Type) -> syn::Result<&Type> {
    let Type::Path(path) = ty else {
        return Err(syn::Error::new_spanned(
            ty,
            "Chain links must be a Vec, Vec1, SmallVec, or SmallVec1",
        ));
    };
    let Some(segment) = path.path.segments.last() else {
        return Err(syn::Error::new_spanned(
            ty,
            "Chain links must be a Vec, Vec1, SmallVec, or SmallVec1",
        ));
    };
    let args = super::type_arguments(&segment.arguments);
    match segment.ident.to_string().as_str() {
        "Vec" | "Vec1" => first_arg(ty, &args),
        "SmallVec" | "SmallVec1" => Ok(super::smallvec_item_type(first_arg(ty, &args)?)),
        _ => Err(syn::Error::new_spanned(
            ty,
            "Chain links must be a Vec, Vec1, SmallVec, or SmallVec1",
        )),
    }
}

#[requires(true)]
#[ensures(true)]
fn option_inner(ty: &Type) -> syn::Result<&Type> {
    let Type::Path(path) = ty else {
        return Err(syn::Error::new_spanned(
            ty,
            "Option field is missing its type",
        ));
    };
    let Some(segment) = path.path.segments.last() else {
        return Err(syn::Error::new_spanned(
            ty,
            "Option field is missing its type",
        ));
    };
    first_arg(ty, &super::type_arguments(&segment.arguments))
}

#[requires(true)]
#[ensures(true)]
fn atom_variant(ty: &Type, atom_types: &BTreeMap<String, Type>) -> syn::Result<Ident> {
    let key = quote!(#ty).to_string();
    if !atom_types.contains_key(&key) {
        return Err(syn::Error::new_spanned(
            ty,
            "tree walk atom is missing from the atom reference enum",
        ));
    }
    Ok(super::atom_variant_ident(ty))
}

#[cfg(test)]
mod tests {
    use bityzba::{requires, try_new};

    use super::*;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_counter_at_the_top_of_u32_is_rejected() {
        assert!(try_new!(Gen { next: u32::MAX }).is_err());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn an_option_row_with_a_false_flag_is_rejected() {
        let ty: Type = syn::parse_quote!(Option<u8>);
        assert!(
            try_new!(FieldRow {
                index: 0,
                name: quote!(field),
                primary: false,
                ty,
                access: quote!(field),
                skip: false,
                option_field: false,
            })
            .is_err()
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_primary_skipped_row_is_rejected() {
        let ty: Type = syn::parse_quote!(u8);
        assert!(
            try_new!(FieldRow {
                index: 0,
                name: quote!(field),
                primary: true,
                ty,
                access: quote!(field),
                skip: true,
                option_field: false,
            })
            .is_err()
        );
    }
}
