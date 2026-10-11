//! Generate shallow borrowed views from the exact tree fields and variants.

use super::*;

#[invariant(true)]
struct FieldView {
    ty: proc_macro2::TokenStream,
    strict: proc_macro2::TokenStream,
    recovered: proc_macro2::TokenStream,
}

#[requires(true)]
#[ensures(true)]
fn source_type(
    ty: &Type,
    recovered: bool,
    nodes: &BTreeSet<String>,
    aliases: &BTreeMap<String, Type>,
) -> Type {
    let mut ty = ty.clone();
    match &mut ty {
        Type::Path(path) => {
            for segment in &mut path.path.segments {
                if let PathArguments::AngleBracketed(args) = &mut segment.arguments {
                    for arg in &mut args.args {
                        if let GenericArgument::Type(inner) = arg {
                            *inner = source_type(inner, recovered, nodes, aliases);
                        }
                    }
                }
            }
            if path.qself.is_none() && path.path.segments.len() == 1 {
                let name = path.path.segments[0].ident.to_string();
                if nodes.contains(&name)
                    || aliases.contains_key(&name)
                    || name == "WithFreeModifiers"
                    || name == "Recovered"
                {
                    if recovered {
                        let original = &path.path;
                        path.path = parse_quote!(super::recovered::#original);
                    } else {
                        let original = &path.path;
                        path.path = parse_quote!(super::#original);
                    }
                }
            }
        }
        Type::Array(array) => {
            array.elem = Box::new(source_type(&array.elem, recovered, nodes, aliases))
        }
        Type::Tuple(tuple) => {
            for elem in &mut tuple.elems {
                *elem = source_type(elem, recovered, nodes, aliases);
            }
        }
        _ => (),
    }
    ty
}

#[requires(true)]
#[ensures(true)]
fn field_view(
    ty: &Type,
    strict: proc_macro2::TokenStream,
    recovered: proc_macro2::TokenStream,
    nodes: &BTreeSet<String>,
    aliases: &BTreeMap<String, Type>,
    projectors: &mut Vec<(String, proc_macro2::TokenStream)>,
) -> syn::Result<FieldView> {
    if let Type::Path(path) = ty {
        let segment = path.path.segments.last().unwrap();
        let name = segment.ident.to_string();
        if path.path.segments.len() == 1 {
            if let Some(alias) = aliases.get(&name) {
                return field_view(alias, strict, recovered, nodes, aliases, projectors);
            }
            if nodes.contains(&name) {
                let reference = format_ident!("{}Ref", name);
                return Ok(FieldView {
                    ty: quote!(Slot<#reference<'tree>>),
                    strict: quote!(Slot::Valid(#reference::Strict(#strict))),
                    recovered: quote!(slot(#recovered, #reference::Recovered)),
                });
            }
        }
        let args = type_arguments(&segment.arguments);
        match name.as_str() {
            "Box" | "Arc" => {
                return field_view(
                    args[0],
                    quote!(&**#strict),
                    quote!(&**#recovered),
                    nodes,
                    aliases,
                    projectors,
                );
            }
            "Option" => {
                let inner = field_view(
                    args[0],
                    quote!(value),
                    quote!(value),
                    nodes,
                    aliases,
                    projectors,
                )?;
                let FieldView {
                    ty,
                    strict: s,
                    recovered: r,
                } = inner;
                return Ok(FieldView {
                    ty: quote!(Option<#ty>),
                    strict: quote!((#strict).as_ref().map(|value| #s)),
                    recovered: quote!((#recovered).as_ref().map(|value| #r)),
                });
            }
            "Vec" | "Vec1" | "SmallVec" | "SmallVec1" => {
                let inner = match args[0] {
                    Type::Array(array) => array.elem.as_ref(),
                    other => other,
                };
                let projection = field_view(
                    inner,
                    quote!(value),
                    quote!(value),
                    nodes,
                    aliases,
                    projectors,
                )?;
                let FieldView {
                    ty: v,
                    strict: s,
                    recovered: r,
                } = projection;
                let strict_type = source_type(inner, false, nodes, aliases);
                let recovered_type = source_type(
                    &transform_type_for_recovery(inner, nodes, aliases)?,
                    true,
                    nodes,
                    aliases,
                );
                let key = quote!(#inner).to_string();
                let index = projectors
                    .iter()
                    .position(|(existing, _)| existing == &key)
                    .unwrap_or(projectors.len());
                let projector = format_ident!("Projection{index}");
                if index == projectors.len() {
                    projectors.push((key, quote! {
                    #[bityzba::invariant(true)]
                    #[derive(Debug, Clone, Copy)]
                    pub struct #projector;
                    #[bityzba::contract_trait]
                    impl<'tree> ::jbotci_tree::view::Projection<'tree, #strict_type, #recovered_type> for #projector {
                        type View = #v;
                        #[inline]
                        #[bityzba::requires(true)] #[bityzba::ensures(true)]
                        fn strict(value: &'tree #strict_type) -> Self::View { #s }
                        #[inline]
                        #[bityzba::requires(true)] #[bityzba::ensures(true)]
                        fn recovered(value: &'tree #recovered_type) -> Self::View { #r }
                    }
                }));
                }
                return Ok(FieldView {
                    ty: quote!(Sequence<'tree, #strict_type, #recovered_type, #v, #projector>),
                    strict: quote!(Sequence::strict((#strict).as_slice())),
                    recovered: quote!(Sequence::recovered((#recovered).as_slice())),
                });
            }
            "Chain" => {
                let first = field_view(
                    args[0],
                    quote!(first),
                    quote!(first),
                    nodes,
                    aliases,
                    projectors,
                )?;
                let links = field_view(
                    args[1],
                    quote!(links),
                    quote!(links),
                    nodes,
                    aliases,
                    projectors,
                )?;
                let FieldView {
                    ty: f,
                    strict: fs,
                    recovered: fr,
                } = first;
                let FieldView {
                    ty: l,
                    strict: ls,
                    recovered: lr,
                } = links;
                return Ok(FieldView {
                    ty: quote!(ChainView<#f, #l>),
                    strict: quote!({ let ::jbotci_tree::Chain { first, links } = #strict; ChainView { first: #fs, links: #ls } }),
                    recovered: quote!({ let ::jbotci_tree::Chain { first, links } = #recovered; ChainView { first: #fr, links: #lr } }),
                });
            }
            "WithFreeModifiers" => {
                let value = field_view(
                    args[0],
                    quote!(value),
                    quote!(value),
                    nodes,
                    aliases,
                    projectors,
                )?;
                let modifier = args
                    .get(1)
                    .copied()
                    .cloned()
                    .unwrap_or_else(|| parse_quote!(FreeModifierSyntax));
                let modifiers: Type = parse_quote!(Vec<#modifier>);
                let modifiers = field_view(
                    &modifiers,
                    quote!(free_modifiers),
                    quote!(free_modifiers),
                    nodes,
                    aliases,
                    projectors,
                )?;
                let FieldView {
                    ty: v,
                    strict: vs,
                    recovered: vr,
                } = value;
                let FieldView {
                    ty: f,
                    strict: fs,
                    recovered: fr,
                } = modifiers;
                return Ok(FieldView {
                    ty: quote!(WithFreeModifiersView<#v, #f>),
                    strict: quote!({ let super::WithFreeModifiers { value, free_modifiers } = #strict; WithFreeModifiersView { value: #vs, free_modifiers: #fs } }),
                    recovered: quote!({ let super::recovered::WithFreeModifiers { value, free_modifiers } = #recovered; WithFreeModifiersView { value: #vr, free_modifiers: #fr } }),
                });
            }
            _ => (),
        }
    }
    match ty {
        Type::Tuple(tuple) => {
            let fields = tuple
                .elems
                .iter()
                .enumerate()
                .map(|(i, ty)| {
                    let i = syn::Index::from(i);
                    field_view(
                        ty,
                        quote!(&(#strict).#i),
                        quote!(&(#recovered).#i),
                        nodes,
                        aliases,
                        projectors,
                    )
                })
                .collect::<syn::Result<Vec<_>>>()?;
            let types = fields.iter().map(|f| &f.ty);
            let s = fields.iter().map(|f| &f.strict);
            let r = fields.iter().map(|f| &f.recovered);
            Ok(FieldView {
                ty: quote!((#(#types,)*)),
                strict: quote!((#(#s,)*)),
                recovered: quote!((#(#r,)*)),
            })
        }
        Type::Array(_) => Err(syn::Error::new_spanned(
            ty,
            "standalone arrays are not supported by tree views",
        )),
        _ => Ok(FieldView {
            ty: quote!(Slot<&'tree #ty>),
            strict: quote!(Slot::Valid(#strict)),
            recovered: quote!(slot(#recovered, |value| value)),
        }),
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn expand(
    items: &[Item],
    nodes: &BTreeSet<String>,
    aliases: &BTreeMap<String, Type>,
    atoms: &BTreeMap<String, Type>,
) -> syn::Result<proc_macro2::TokenStream> {
    let mut projectors = Vec::new();
    let mut definitions = Vec::new();
    let mut methods = Vec::new();
    let mut descents = Vec::new();
    for item in items {
        let (ident, cases) = match item {
            Item::Struct(item) => (&item.ident, vec![(None, &item.fields)]),
            Item::Enum(item) => (
                &item.ident,
                item.variants
                    .iter()
                    .map(|v| (Some(&v.ident), &v.fields))
                    .collect(),
            ),
            Item::Type(_) => continue,
            _ => unreachable!(),
        };
        let reference = format_ident!("{}Ref", ident);
        let view = format_ident!("{}View", ident);
        let method = walk_method_ident_for_type(ident);
        let function = walk_function_ident_for_type(ident);
        let mut view_cases = Vec::new();
        let mut view_invariants = Vec::new();
        let mut strict_cases = Vec::new();
        let mut recovered_cases = Vec::new();
        let mut walk_cases = Vec::new();
        let mut accessors = Vec::new();
        for (variant, fields) in cases {
            let bindings = fields
                .iter()
                .enumerate()
                .map(|(i, f)| {
                    f.ident
                        .clone()
                        .unwrap_or_else(|| format_ident!("field_{i}"))
                })
                .collect::<Vec<_>>();
            let projections = fields
                .iter()
                .zip(&bindings)
                .map(|(f, b)| {
                    field_view(
                        &f.ty,
                        quote!(#b),
                        quote!(#b),
                        nodes,
                        aliases,
                        &mut projectors,
                    )
                })
                .collect::<syn::Result<Vec<_>>>()?;
            if variant.is_none() {
                for (index, field) in fields.iter().enumerate() {
                    let getter = field
                        .ident
                        .as_ref()
                        .map(|name| format_ident!("field_{name}"))
                        .unwrap_or_else(|| format_ident!("field_{index}"));
                    let selected = fields
                        .iter()
                        .zip(&bindings)
                        .enumerate()
                        .map(|(selected, (source, binding))| {
                            if selected == index {
                                quote!(#binding)
                            } else if matches!(fields, Fields::Named(_)) {
                                let name = source.ident.as_ref().unwrap();
                                quote!(#name: _)
                            } else {
                                quote!(_)
                            }
                        })
                        .collect::<Vec<_>>();
                    let pattern = match fields {
                        Fields::Named(_) => quote!({#(#selected,)*}),
                        Fields::Unnamed(_) => quote!((#(#selected,)*)),
                        Fields::Unit => quote!(),
                    };
                    let FieldView {
                        ty,
                        strict,
                        recovered,
                    } = &projections[index];
                    accessors.push(quote! {
                        #[inline(always)]
                        #[bityzba::requires(true)] #[bityzba::ensures(true)]
                        pub fn #getter(self) -> #ty {
                            match self {
                                Self::Strict(super::#ident #pattern) => #strict,
                                Self::Recovered(super::recovered::#ident #pattern) => #recovered,
                            }
                        }
                    });
                }
            }
            let types = projections.iter().map(|f| &f.ty).collect::<Vec<_>>();
            let strict_values = projections.iter().map(|f| &f.strict).collect::<Vec<_>>();
            let recovered_values = projections.iter().map(|f| &f.recovered).collect::<Vec<_>>();
            let mut walked = Vec::new();
            let mut walk_bindings = Vec::new();
            let mut named_walk_bindings = Vec::new();
            for (field, binding) in fields.iter().zip(&bindings) {
                if tree_child_flags(&field.attrs)?.skip {
                    walk_bindings.push(quote!(_));
                    named_walk_bindings.push(quote!(#binding: _));
                } else {
                    walked.push(binding);
                    walk_bindings.push(quote!(#binding));
                    named_walk_bindings.push(quote!(#binding));
                }
            }
            let visibility = variant.is_none().then(|| quote!(pub));
            let (definition, pattern, view_pattern, s_value, r_value) = match fields {
                Fields::Named(_) => (
                    quote!({#(#visibility #bindings: #types,)*}),
                    quote!({#(#bindings,)*}),
                    quote!({#(#named_walk_bindings,)*}),
                    quote!({#(#bindings: #strict_values,)*}),
                    quote!({#(#bindings: #recovered_values,)*}),
                ),
                Fields::Unnamed(_) => (
                    quote!((#(#visibility #types,)*)),
                    quote!((#(#bindings,)*)),
                    quote!((#(#walk_bindings,)*)),
                    quote!((#(#strict_values,)*)),
                    quote!((#(#recovered_values,)*)),
                ),
                Fields::Unit => (quote!(), quote!(), quote!(), quote!(), quote!()),
            };
            if let Some(variant) = variant {
                let invariant = if matches!(fields, Fields::Unnamed(_)) {
                    let slots = fields.iter().map(|_| quote!(_));
                    quote!(#[bityzba::invariant(::#variant(#(#slots),*) => true)])
                } else {
                    quote!(#[bityzba::invariant(::#variant => true)])
                };
                view_invariants.push(invariant);
                view_cases.push(quote!(#variant #definition));
                strict_cases
                    .push(quote!(super::#ident::#variant #pattern => #view::#variant #s_value));
                recovered_cases.push(
                    quote!(super::recovered::#ident::#variant #pattern => #view::#variant #r_value),
                );
                walk_cases.push(quote!(#view::#variant #view_pattern => { #(Walkable::walk_with(#walked, walker);)* }));
            } else {
                let semicolon =
                    matches!(fields, Fields::Unnamed(_) | Fields::Unit).then(|| quote!(;));
                view_cases.push(quote!(pub struct #view<'tree> #definition #semicolon));
                strict_cases.push(quote!(super::#ident #pattern => #view #s_value));
                recovered_cases.push(quote!(super::recovered::#ident #pattern => #view #r_value));
                walk_cases.push(
                    quote!(#view #view_pattern => { #(Walkable::walk_with(#walked, walker);)* }),
                );
            }
        }
        let definition = if matches!(item, Item::Enum(_)) {
            quote!(pub enum #view<'tree> { #(#view_cases,)* })
        } else {
            quote!(#(#view_cases)*)
        };
        definitions.push(quote! {
            #[bityzba::invariant(true)]
            #(#view_invariants)*
            #[derive(Debug, Clone, Copy)]
            #definition
            #[bityzba::invariant(true)]
            #[bityzba::invariant(::Strict(_) => true)]
            #[bityzba::invariant(::Recovered(_) => true)]
            #[derive(Debug, Clone, Copy)]
            pub enum #reference<'tree> { Strict(&'tree super::#ident), Recovered(&'tree super::recovered::#ident) }
            impl<'tree> #reference<'tree> {
                #(#accessors)*
                #[bityzba::requires(true)]
                #[bityzba::ensures(true)]
                #[inline(always)]
                pub fn get(self) -> #view<'tree> {
                    match self {
                        Self::Strict(value) => match value { #(#strict_cases,)* },
                        Self::Recovered(value) => match value { #(#recovered_cases,)* },
                    }
                }
                #[bityzba::requires(true)]
                #[bityzba::ensures(matches!(self, Self::Strict(_)) -> !ret)]
                pub fn has_errors(self) -> bool {
                    match self {
                        Self::Strict(_) => false,
                        Self::Recovered(value) => ::jbotci_tree::RecoveredFieldState::recovery_error_slots(value) > 0,
                    }
                }
            }
            #[bityzba::contract_trait]
            impl<'tree> Inspect for #reference<'tree> {
                type View = #view<'tree>;
                #[inline(always)]
                fn inspect(self) -> Self::View { self.get() }
                #[inline]
                fn needs_recovery_evidence(self) -> bool { matches!(self, Self::Recovered(_)) }
            }
            #[bityzba::contract_trait]
            impl<'tree> AsView<'tree> for super::#ident {
                type Ref = #reference<'tree>;
                type Kind = ::jbotci_tree::view::StrictSource;
                #[bityzba::requires(true)]
                #[bityzba::ensures(ret.valid().is_some())]
                fn as_view(&'tree self) -> Slot<Self::Ref> { Slot::Valid(#reference::Strict(self)) }
            }
            #[bityzba::contract_trait]
            impl<'tree> AsView<'tree> for super::recovered::#ident {
                type Ref = #reference<'tree>;
                type Kind = ::jbotci_tree::view::RecoveredSource;
                #[bityzba::requires(true)]
                #[bityzba::ensures(ret.valid().is_some())]
                fn as_view(&'tree self) -> Slot<Self::Ref> { Slot::Valid(#reference::Recovered(self)) }
            }
            #[bityzba::contract_trait]
            impl<'tree> AsView<'tree> for super::recovered::Recovered<super::recovered::#ident> {
                type Ref = #reference<'tree>;
                type Kind = ::jbotci_tree::view::RecoveredSource;
                #[bityzba::requires(true)]
                #[bityzba::ensures(ret.valid().is_some() == matches!(self, super::recovered::Recovered::Valid(_)))]
                fn as_view(&'tree self) -> Slot<Self::Ref> { slot(self, #reference::Recovered) }
            }
            #[bityzba::contract_trait]
            impl<'tree> Walkable<'tree> for #reference<'tree> {
                #[bityzba::requires(true)]
                #[bityzba::ensures(true)]
                fn walk_with<W: Walker<'tree> + ?Sized>(self, walker: &mut W) { walker.#method(self); }
            }
        });
        methods.push(quote! {
            #[bityzba::requires(true)]
            #[bityzba::ensures(true)]
            fn #method(&mut self, node: #reference<'tree>) { walk::#function(self, node); }
        });
        descents.push(quote! {
            #[bityzba::requires(true)]
            #[bityzba::ensures(true)]
            pub fn #function<'tree, W: Walker<'tree> + ?Sized>(walker: &mut W, node: #reference<'tree>) {
                match node.get() { #(#walk_cases,)* }
            }
        });
    }
    let atom_impls = atoms.values().map(|ty| {
        let variant = atom_variant_ident(ty);
        quote! {
            #[bityzba::contract_trait]
            impl<'tree> Inspect for &'tree #ty {
                type View = Self;
                fn inspect(self) -> Self { self }
                fn needs_recovery_evidence(self) -> bool { false }
            }
            #[bityzba::contract_trait]
            impl<'tree> Walkable<'tree> for &'tree #ty {
                #[bityzba::requires(true)]
                #[bityzba::ensures(true)]
                fn walk_with<W: Walker<'tree> + ?Sized>(self, walker: &mut W) { walker.walk_atom(super::AtomRef::#variant(self)); }
            }
        }
    });
    let projectors = projectors.iter().map(|(_, definition)| definition);
    Ok(quote! {
        /// Shallow read-only projections shared by strict and recovered nodes.
        pub mod view {
            use super::*;
            pub use ::jbotci_tree::view::{Slot, Sequence, ChainView, WithFreeModifiersView};
            #[bityzba::requires(true)]
            #[bityzba::ensures(ret.valid().is_some() == matches!(value, super::recovered::Recovered::Valid(_)))]
            #[inline]
            pub fn slot<'tree, T, V: Copy>(value: &'tree super::recovered::Recovered<T>, project: impl FnOnce(&'tree T) -> V) -> Slot<V> {
                match value {
                    super::recovered::Recovered::Valid(value) => Slot::Valid(project(value)),
                    super::recovered::Recovered::Prefix(prefix) => Slot::Prefix(project(&prefix.value)),
                    super::recovered::Recovered::Error(_) => Slot::Error,
                }
            }
            #[bityzba::contract_trait]
            pub trait Inspect: Copy {
                type View;
                #[bityzba::requires(true)] #[bityzba::ensures(true)]
                fn inspect(self) -> Self::View;
                /// Strict nodes and parsed atomic values need no recovery proof.
                #[bityzba::requires(true)] #[bityzba::ensures(true)]
                fn needs_recovery_evidence(self) -> bool;
            }
            /// Project a valid child slot. Prefix and Error give no view.
            #[bityzba::requires(true)]
            #[bityzba::ensures(ret.is_some() == value.borrow().valid().is_some())]
            #[inline(always)]
            pub fn valid<V: Inspect>(value: impl ::std::borrow::Borrow<Slot<V>>) -> Option<V::View> {
                value.borrow().valid().map(Inspect::inspect)
            }
            #[bityzba::contract_trait]
            pub trait AsView<'tree> {
                type Ref: Walkable<'tree>;
                type Kind: ::jbotci_tree::view::SourceKind;
                #[bityzba::requires(true)] #[bityzba::ensures(true)]
                fn as_view(&'tree self) -> Slot<Self::Ref>;
            }
            #[bityzba::contract_trait]
            pub trait Walker<'tree> {
                #[bityzba::requires(true)] #[bityzba::ensures(true)]
                fn walk_atom(&mut self, _atom: super::AtomRef<'tree>) {}
                #[bityzba::requires(true)] #[bityzba::ensures(true)]
                fn walk_error(&mut self) {}
                #(#methods)*
            }
            #[bityzba::contract_trait]
            pub trait Walkable<'tree>: Copy {
                #[bityzba::requires(true)] #[bityzba::ensures(true)]
                fn walk_with<W: Walker<'tree> + ?Sized>(self, walker: &mut W);
            }
            #[bityzba::contract_trait]
            impl<'tree, V: Walkable<'tree>> Walkable<'tree> for Slot<V> {
                #[bityzba::requires(true)] #[bityzba::ensures(true)]
                fn walk_with<W: Walker<'tree> + ?Sized>(self, walker: &mut W) {
                    match self {
                        Slot::Valid(value) => value.walk_with(walker),
                        Slot::Prefix(value) => { walker.walk_error(); value.walk_with(walker); },
                        Slot::Error => walker.walk_error(),
                    }
                }
            }
            #[bityzba::contract_trait]
            impl<'tree, V: Walkable<'tree>> Walkable<'tree> for Option<V> {
                #[bityzba::requires(true)] #[bityzba::ensures(true)]
                fn walk_with<W: Walker<'tree> + ?Sized>(self, walker: &mut W) { if let Some(value) = self { value.walk_with(walker); } }
            }
            #[bityzba::contract_trait]
            impl<'tree, S, R, V: Walkable<'tree>, P: ::jbotci_tree::view::Projection<'tree, S, R, View = V>> Walkable<'tree> for Sequence<'tree, S, R, V, P> {
                #[bityzba::requires(true)] #[bityzba::ensures(true)]
                fn walk_with<W: Walker<'tree> + ?Sized>(self, walker: &mut W) { for value in self { value.walk_with(walker); } }
            }
            #[bityzba::contract_trait]
            impl<'tree, E: Walkable<'tree>, L: Walkable<'tree>> Walkable<'tree> for ChainView<E, L> {
                #[bityzba::requires(true)] #[bityzba::ensures(true)]
                fn walk_with<W: Walker<'tree> + ?Sized>(self, walker: &mut W) { self.first.walk_with(walker); self.links.walk_with(walker); }
            }
            #[bityzba::contract_trait]
            impl<'tree, V: Walkable<'tree>, F: Walkable<'tree>> Walkable<'tree> for WithFreeModifiersView<V, F> {
                #[bityzba::requires(true)] #[bityzba::ensures(true)]
                fn walk_with<W: Walker<'tree> + ?Sized>(self, walker: &mut W) { self.value.walk_with(walker); self.free_modifiers.walk_with(walker); }
            }
            #[bityzba::contract_trait]
            impl<'tree, A: Walkable<'tree>, B: Walkable<'tree>> Walkable<'tree> for (A, B) {
                #[bityzba::requires(true)] #[bityzba::ensures(true)]
                fn walk_with<W: Walker<'tree> + ?Sized>(self, walker: &mut W) { self.0.walk_with(walker); self.1.walk_with(walker); }
            }
            #(#projectors)*
            #(#atom_impls)*
            #(#definitions)*
            pub mod walk {
                use super::*;
                /// Visit a surviving value without counting its own prefix errors as child errors.
                #[bityzba::requires(true)]
                #[bityzba::ensures(ret == !matches!(value, Slot::Error))]
                pub fn value<'tree, V: Walkable<'tree>, W: Walker<'tree> + ?Sized>(walker: &mut W, value: Slot<V>) -> bool {
                    match value {
                        Slot::Valid(value) | Slot::Prefix(value) => { value.walk_with(walker); true },
                        Slot::Error => false,
                    }
                }
                #(#descents)*
            }
        }
    })
}
