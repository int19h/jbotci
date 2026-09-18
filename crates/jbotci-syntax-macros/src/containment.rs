//! Storage lowering for generated model fields. Parser results retain their
//! declared types; this plan changes both the stored type and its constructor
//! value together, without changing rule calls or recovery checkpoints.

use std::collections::BTreeSet;

use super::{ParserExpr, VectorItem};
#[allow(unused_imports)]
use bityzba::{ensures, invariant, requires};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Expr, GenericArgument, PathArguments, Type, parse_quote};

#[invariant(true)]
#[derive(Clone)]
pub(super) struct Containment {
    source: Type,
    action: Action,
}

#[invariant(true)]
#[invariant(::Tuple => true)]
#[invariant(::Array => true)]
#[invariant(::Wrapper => true)]
#[derive(Clone)]
enum Action {
    Identity,
    Share,
    Tuple {
        elements: Vec<Containment>,
    },
    Array {
        element: Box<Containment>,
    },
    Wrapper {
        kind: Wrapper,
        arguments: Vec<Containment>,
    },
}

#[invariant(true)]
#[derive(Clone, Copy)]
enum Wrapper {
    Arc,
    Box,
    Option,
    Vec,
    Vec1,
    SmallVec,
    SmallVec1,
    Chain,
    WithFreeModifiers,
}

impl Containment {
    #[requires(true)]
    #[ensures(true)]
    pub(super) fn new(source: &Type, nodes: &BTreeSet<String>) -> Self {
        let action = if super::simple_type_ident(source)
            .is_some_and(|name| nodes.contains(&name.to_string()))
        {
            Action::Share
        } else {
            match source {
                Type::Tuple(tuple) => Action::Tuple {
                    elements: tuple.elems.iter().map(|ty| Self::new(ty, nodes)).collect(),
                },
                Type::Array(array) => Action::Array {
                    element: Box::new(Self::new(&array.elem, nodes)),
                },
                Type::Path(path) if path.qself.is_none() => {
                    match path.path.segments.last().and_then(|segment| {
                        let kind = match segment.ident.to_string().as_str() {
                            "Arc" => Wrapper::Arc,
                            "Box" => Wrapper::Box,
                            "Option" => Wrapper::Option,
                            "Vec" => Wrapper::Vec,
                            "Vec1" => Wrapper::Vec1,
                            "SmallVec" => Wrapper::SmallVec,
                            "SmallVec1" => Wrapper::SmallVec1,
                            "Chain" => Wrapper::Chain,
                            "WithFreeModifiers" => Wrapper::WithFreeModifiers,
                            _ => return None,
                        };
                        let PathArguments::AngleBracketed(args) = &segment.arguments else {
                            return None;
                        };
                        let mut arguments: Vec<_> = args
                            .args
                            .iter()
                            .filter_map(|arg| {
                                let GenericArgument::Type(ty) = arg else {
                                    return None;
                                };
                                let plan = Self::new(ty, nodes);
                                // A direct Arc already shares its node. This is not
                                // an exemption for nodes deeper inside the pointer.
                                Some(
                                    if matches!(kind, Wrapper::Arc)
                                        && matches!(plan.action, Action::Share)
                                    {
                                        Self::identity(ty)
                                    } else {
                                        plan
                                    },
                                )
                            })
                            .collect();
                        if matches!(kind, Wrapper::WithFreeModifiers) && arguments.len() == 1 {
                            arguments.push(Self::new(&parse_quote!(FreeModifierSyntax), nodes));
                        }
                        Some(Action::Wrapper { kind, arguments })
                    }) {
                        Some(action) => action,
                        None => Action::Identity,
                    }
                }
                _ => Action::Identity,
            }
        };
        Self {
            source: source.clone(),
            action,
        }
    }

    #[requires(true)]
    #[ensures(!ret.changes())]
    pub(super) fn identity(source: &Type) -> Self {
        Self {
            source: source.clone(),
            action: Action::Identity,
        }
    }

    /// Apply an explicit storage opt-out to its output position, preserving the
    /// enclosing option/pointer/wrapper. Parser predicates and annotations do
    /// not create a new containment boundary.
    #[requires(true)]
    #[ensures(true)]
    pub(super) fn with_parser_policy(mut self, expr: &ParserExpr) -> syn::Result<Self> {
        self.apply_policy(expr)?;
        Ok(self)
    }

    #[requires(true)]
    #[ensures(true)]
    fn apply_policy(&mut self, expr: &ParserExpr) -> syn::Result<()> {
        match expr {
            ParserExpr::Rust(expr) => self.apply_parser_policy(expr)?,
            ParserExpr::Vector(vector) => {
                let mut selected: Option<Self> = None;
                for item in &vector.items {
                    let (parser, spread) = match item {
                        VectorItem::One(p)
                        | VectorItem::ZeroOrMore(p)
                        | VectorItem::OneOrMore(p) => (p, false),
                        VectorItem::Spread(p)
                        | VectorItem::ZeroOrMoreSpread(p)
                        | VectorItem::OneOrMoreSpread(p) => (p, true),
                        VectorItem::Assert { .. } => continue,
                    };
                    let mut candidate = self.clone();
                    if spread {
                        candidate.apply_policy(parser)?;
                    } else if let Some(element) = candidate.sequence_element_mut() {
                        element.apply_policy(parser)?;
                    }
                    Self::select_consistent(&mut selected, candidate, &parser.to_token_stream())?;
                }
                if let Some(selected) = selected {
                    *self = selected;
                }
            }
            ParserExpr::Postfix {
                receiver,
                method,
                args,
            } => {
                self.apply_method_policy(receiver, method, args)?;
            }
            ParserExpr::Chain(chain) => {
                if let Action::Wrapper {
                    kind: Wrapper::Chain,
                    arguments,
                } = &mut self.action
                {
                    arguments[0].apply_policy(&chain.first)?;
                    if let Some(element) = arguments[1].sequence_element_mut() {
                        element.apply_policy(&chain.links)?;
                    }
                }
            }
        }
        Ok(())
    }

    #[requires(true)]
    #[ensures(true)]
    fn sequence_element_mut(&mut self) -> Option<&mut Self> {
        match &mut self.action {
            Action::Wrapper {
                kind: Wrapper::Vec | Wrapper::Vec1,
                arguments,
            } => arguments.first_mut(),
            Action::Wrapper {
                kind: Wrapper::SmallVec | Wrapper::SmallVec1,
                arguments,
            } => match &mut arguments.first_mut()?.action {
                Action::Array { element } => Some(element),
                _ => None,
            },
            _ => None,
        }
    }

    // Every branch producing the same field/sequence element must agree on its
    // static storage type. Choosing a branch at runtime cannot change that type.
    #[requires(true)]
    #[ensures(true)]
    fn select_consistent(
        selected: &mut Option<Self>,
        candidate: Self,
        at: &TokenStream,
    ) -> syn::Result<()> {
        if let Some(previous) = selected {
            if previous.stored_type() != candidate.stored_type() {
                return Err(syn::Error::new_spanned(
                    at,
                    "conflicting inline storage policies for the same output position",
                ));
            }
        } else {
            *selected = Some(candidate);
        }
        Ok(())
    }

    #[requires(true)]
    #[ensures(true)]
    fn apply_method_policy(
        &mut self,
        receiver: &ParserExpr,
        method: &syn::Ident,
        args: &[Expr],
    ) -> syn::Result<()> {
        match method.to_string().as_str() {
            "wf" | "with_free_modifiers" | "prohibited_wf" | "wf_when" => {
                if let Action::Wrapper { arguments, .. } = &mut self.action
                    && let Some(inner) = arguments.first_mut()
                {
                    inner.apply_policy(receiver)?;
                }
            }
            "ignore_then" if args.len() == 1 => self.apply_parser_policy(&args[0])?,
            "elidable_terminator"
            | "lookahead"
            | "reject_output"
            | "warn"
            | "payload_start"
            | "then_ignore"
            | "not_next_selmaho"
            | "not_next_token"
            | "not_next_rule"
            | "followed_by" => self.apply_policy(receiver)?,
            _ => {}
        }
        Ok(())
    }

    #[requires(true)]
    #[ensures(true)]
    fn apply_parser_policy(&mut self, expr: &Expr) -> syn::Result<()> {
        match expr {
            Expr::Call(call) => {
                let name = super::call_name(call);
                if name.as_deref() == Some("inline") && call.args.len() == 1 {
                    self.action = Action::Identity;
                    return Ok(());
                }
                if matches!(name.as_deref(), Some("opt" | "arc" | "boxed")) && call.args.len() == 1
                {
                    if let Action::Wrapper { arguments, .. } = &mut self.action
                        && let Some(inner) = arguments.first_mut()
                    {
                        inner.apply_parser_policy(&call.args[0])?;
                    }
                } else if matches!(name.as_deref(), Some("feature" | "policy" | "memo_scope"))
                    && call.args.len() == 2
                {
                    self.apply_parser_policy(&call.args[1])?;
                } else if name.as_deref() == Some("choice") {
                    let alternatives = if call.args.len() == 1 {
                        super::choice_alternative_exprs(&call.args[0])
                    } else {
                        call.args.iter().collect()
                    };
                    let mut selected = None;
                    for alternative in alternatives {
                        let mut candidate = self.clone();
                        candidate.apply_parser_policy(alternative)?;
                        Self::select_consistent(&mut selected, candidate, &quote!(#alternative))?;
                    }
                    if let Some(selected) = selected {
                        *self = selected;
                    }
                }
            }
            Expr::MethodCall(method) => self.apply_method_policy(
                &ParserExpr::Rust(*method.receiver.clone()),
                &method.method,
                &method.args.iter().cloned().collect::<Vec<_>>(),
            )?,
            Expr::Paren(paren) => self.apply_parser_policy(&paren.expr)?,
            Expr::Group(group) => self.apply_parser_policy(&group.expr)?,
            Expr::Array(array) => {
                if let Some(vector) = super::array_vector_expr(array) {
                    self.apply_policy(&ParserExpr::Vector(vector))?;
                }
            }
            Expr::Tuple(tuple) => {
                if let Action::Tuple { elements } = &mut self.action {
                    for (plan, expr) in elements.iter_mut().zip(&tuple.elems) {
                        plan.apply_parser_policy(expr)?;
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    #[requires(true)]
    #[ensures(true)]
    fn changes(&self) -> bool {
        match &self.action {
            Action::Identity => false,
            Action::Share => true,
            Action::Tuple { elements } => elements.iter().any(Self::changes),
            Action::Array { element } => element.changes(),
            Action::Wrapper { arguments, .. } => arguments.iter().any(Self::changes),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn stored_type(&self) -> Type {
        let mut ty = self.source.clone();
        match (&self.action, &mut ty) {
            (Action::Share, _) => parse_quote!(::std::sync::Arc<#ty>),
            (Action::Tuple { elements }, Type::Tuple(tuple)) => {
                tuple.elems = elements.iter().map(Self::stored_type).collect();
                ty
            }
            (Action::Array { element }, Type::Array(array)) => {
                array.elem = Box::new(element.stored_type());
                ty
            }
            (Action::Wrapper { arguments, .. }, Type::Path(path)) => {
                let PathArguments::AngleBracketed(args) = &mut path
                    .path
                    .segments
                    .last_mut()
                    .expect("wrapper has a path segment")
                    .arguments
                else {
                    unreachable!()
                };
                let mut plans = arguments.iter();
                for arg in &mut args.args {
                    if let GenericArgument::Type(ty) = arg {
                        *ty = plans
                            .next()
                            .expect("one plan per type argument")
                            .stored_type();
                    }
                }
                for plan in plans {
                    args.args.push(GenericArgument::Type(plan.stored_type()));
                }
                ty
            }
            _ => ty,
        }
    }

    /// `free_modifier_wrapper` names the strict or recovered wrapper constructor.
    /// Recovery has already happened; sharing a recovered node therefore wraps
    /// the entire Recovered value, including its prefix/error information.
    #[requires(true)]
    #[ensures(true)]
    pub(super) fn lower(
        &self,
        value: TokenStream,
        free_modifier_wrapper: &TokenStream,
    ) -> TokenStream {
        if !self.changes() {
            return value;
        }
        match &self.action {
            Action::Identity => value,
            Action::Share => quote!(::std::sync::Arc::new(#value)),
            Action::Tuple { elements } => {
                let names = (0..elements.len())
                    .map(|i| format_ident!("__contained_{i}"))
                    .collect::<Vec<_>>();
                let values = elements
                    .iter()
                    .zip(&names)
                    .map(|(plan, name)| plan.lower(quote!(#name), free_modifier_wrapper));
                quote!({ let (#(#names,)*) = #value; (#(#values,)*) })
            }
            Action::Array { element } => {
                let mapped = element.lower(quote!(__element), free_modifier_wrapper);
                quote!((#value).map(|__element| #mapped))
            }
            Action::Wrapper { kind, arguments } => {
                let first = arguments
                    .first()
                    .expect("supported wrappers have type arguments");
                let mapped = first.lower(quote!(__element), free_modifier_wrapper);
                match kind {
                    Wrapper::Arc => quote!({
                        let __element = ::std::sync::Arc::unwrap_or_clone(#value);
                        ::std::sync::Arc::new(#mapped)
                    }),
                    Wrapper::Box => quote!({ let __element = *(#value); Box::new(#mapped) }),
                    Wrapper::Option => quote!((#value).map(|__element| #mapped)),
                    Wrapper::Vec => {
                        quote!((#value).into_iter().map(|__element| #mapped).collect::<Vec<_>>())
                    }
                    Wrapper::Vec1 => quote!((#value).mapped(|__element| #mapped)),
                    Wrapper::SmallVec | Wrapper::SmallVec1 => {
                        let Action::Array { element } = &first.action else {
                            unreachable!("small-vector storage is a fixed array")
                        };
                        let mapped = element.lower(quote!(__element), free_modifier_wrapper);
                        if matches!(kind, Wrapper::SmallVec) {
                            quote!((#value).into_iter().map(|__element| #mapped).collect::<::smallvec::SmallVec<_>>())
                        } else {
                            quote!(::vec1::smallvec_v1::SmallVec1::try_from_vec(
                                (#value).into_vec().into_iter().map(|__element| #mapped).collect()
                            ).expect("containment lowering preserves nonempty sequences"))
                        }
                    }
                    Wrapper::Chain => {
                        let first = first.lower(quote!(__first), free_modifier_wrapper);
                        let links = arguments[1].lower(quote!(__links), free_modifier_wrapper);
                        quote!({
                            let ::jbotci_tree::Chain { first: __first, links: __links } = #value;
                            ::jbotci_tree::Chain { first: #first, links: #links }
                        })
                    }
                    Wrapper::WithFreeModifiers => {
                        let inner = first.lower(quote!(__value), free_modifier_wrapper);
                        let modifier =
                            arguments[1].lower(quote!(__modifier), free_modifier_wrapper);
                        quote!({
                            let #free_modifier_wrapper { value: __value, free_modifiers: __modifiers } = #value;
                            #free_modifier_wrapper {
                                value: #inner,
                                free_modifiers: __modifiers.into_iter().map(|__modifier| #modifier).collect(),
                            }
                        })
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::ToTokens;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn nonempty_sequence_lowering_uses_cardinality_preserving_map() {
        let plan = Containment::new(
            &parse_quote!(vec1::Vec1<Node>),
            &BTreeSet::from(["Node".to_owned()]),
        );
        assert_eq!(
            plan.stored_type(),
            parse_quote!(vec1::Vec1<::std::sync::Arc<Node>>)
        );
        assert_eq!(
            plan.lower(quote!(input), &quote!(WithFreeModifiers))
                .to_string(),
            quote!((input).mapped(|__element| ::std::sync::Arc::new(__element))).to_string(),
        );
        // Check the collection API's ownership/cardinality guarantee with
        // non-Clone elements as well as the emitted call and stored type above.
        let input = vec1::vec1![
            Box::new(std::sync::Mutex::new(7)),
            Box::new(std::sync::Mutex::new(8))
        ];
        let addresses = input
            .iter()
            .map(|value| &**value as *const std::sync::Mutex<i32>)
            .collect::<Vec<_>>();
        let result = input.mapped(std::sync::Arc::new);
        assert_eq!(result.len(), 2);
        assert_eq!(
            result
                .iter()
                .map(|value| &***value as *const std::sync::Mutex<i32>)
                .collect::<Vec<_>>(),
            addresses
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn inline_policy_reaches_sequence_and_choice_elements() {
        let nodes = BTreeSet::from(["Node".to_owned()]);
        for (ty, parser, expected) in [
            ("Vec<Node>", "[zero_or_more inline(node)]", "Vec<Node>"),
            ("Vec1<Node>", "[one_or_more inline(node)]", "Vec1<Node>"),
            ("Vec<Node>", "[..inline(nodes)]", "Vec<Node>"),
            ("Node", "choice(inline(node), inline(node))", "Node"),
            (
                "(Node, Node)",
                "(inline(node), node)",
                "(Node, ::std::sync::Arc<Node>)",
            ),
            (
                "Vec<Node>",
                "[zero_or_more inline(node)].warn(warning)",
                "Vec<Node>",
            ),
        ] {
            let plan = Containment::new(&syn::parse_str(ty).unwrap(), &nodes)
                .with_parser_policy(&syn::parse_str(parser).unwrap())
                .unwrap();
            assert_eq!(
                plan.stored_type(),
                syn::parse_str::<Type>(expected).unwrap()
            );
        }
        for (ty, parser) in [
            ("Vec<Node>", "[inline(node); node]"),
            ("Node", "choice(inline(node), node)"),
        ] {
            assert!(
                Containment::new(&syn::parse_str(ty).unwrap(), &nodes)
                    .with_parser_policy(&syn::parse_str(parser).unwrap())
                    .is_err()
            );
        }
    }

    /// Compile and execute the actual emitted expressions, including recovered
    /// inputs. Token-string comparisons cannot catch constructor/type drift.
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn lowered_values_compile_and_preserve_contents() {
        let nodes = BTreeSet::from(["Node".to_owned()]);
        let mut checks = Vec::new();
        for (source, input, expected, assertion) in [
            ("Node", "Node(7)", "Arc<Node>", "assert_eq!(result.0, 7);"),
            (
                "Arc<Node>",
                "Arc::new(Node(7))",
                "Arc<Node>",
                "assert_eq!(result.0, 7);",
            ),
            (
                "Box<Node>",
                "Box::new(Node(7))",
                "Box<Arc<Node>>",
                "assert_eq!(result.0, 7);",
            ),
            (
                "Option<Node>",
                "Some(Node(7))",
                "Option<Arc<Node>>",
                "assert_eq!(result.unwrap().0, 7);",
            ),
            (
                "Vec<Node>",
                "vec![Node(7), Node(8)]",
                "Vec<Arc<Node>>",
                "assert_eq!(result.iter().map(|n| n.0).collect::<Vec<_>>(), [7, 8]);",
            ),
            (
                "(u8, Node)",
                "(1u8, Node(7))",
                "(u8, Arc<Node>)",
                "assert_eq!(result.0, 1); assert_eq!(result.1.0, 7);",
            ),
            (
                "[Node; 2]",
                "[Node(7), Node(8)]",
                "[Arc<Node>; 2]",
                "assert_eq!(result.map(|n| n.0), [7, 8]);",
            ),
            (
                "Arc<Option<Node>>",
                "Arc::new(Some(Node(7)))",
                "Arc<Option<Arc<Node>>>",
                "assert_eq!(result.as_ref().as_ref().unwrap().0, 7);",
            ),
            (
                "Arc<Arc<Node>>",
                "Arc::new(Arc::new(Node(7)))",
                "Arc<Arc<Node>>",
                "assert_eq!(result.0, 7);",
            ),
            (
                "WithFreeModifiers<u8, Node>",
                "WithFreeModifiers { value: 1u8, free_modifiers: vec![Node(7)] }",
                "WithFreeModifiers<u8, Arc<Node>>",
                "assert_eq!(result.value, 1); assert_eq!(result.free_modifiers[0].0, 7);",
            ),
            (
                "Node",
                "Recovered::Prefix(Node(7))",
                "Arc<Recovered<Node>>",
                "assert_eq!(*result, Recovered::Prefix(Node(7)));",
            ),
            (
                "Node",
                "Recovered::<Node>::Error",
                "Arc<Recovered<Node>>",
                "assert_eq!(*result, Recovered::Error);",
            ),
            (
                "WithFreeModifiers<u8, Node>",
                "WithFreeModifiers { value: Recovered::Valid(1u8), free_modifiers: vec![Recovered::Prefix(Node(7))] }",
                "WithFreeModifiers<Recovered<u8>, Arc<Recovered<Node>>>",
                "assert_eq!(result.value, Recovered::Valid(1)); assert_eq!(*result.free_modifiers[0], Recovered::Prefix(Node(7)));",
            ),
        ] {
            let source = syn::parse_str(source).unwrap();
            let input: syn::Expr = syn::parse_str(input).unwrap();
            let expected: Type = syn::parse_str(expected).unwrap();
            let assertion: TokenStream = assertion.parse().unwrap();
            let lowered =
                Containment::new(&source, &nodes).lower(quote!(input), &quote!(WithFreeModifiers));
            checks
                .push(quote!({ let input = #input; let result: #expected = #lowered; #assertion }));
        }
        let identity = Containment::new(&parse_quote!(Arc<Node>), &nodes)
            .lower(quote!(original.clone()), &quote!(WithFreeModifiers));
        let shared_container = Containment::new(&parse_quote!(Arc<Option<Node>>), &nodes)
            .lower(quote!(original.clone()), &quote!(WithFreeModifiers));
        let program = quote! {
            use std::sync::Arc;
            #[derive(Clone, Debug, PartialEq)] struct Node(u8);
            #[derive(Clone, Debug, PartialEq)] enum Recovered<T> { Valid(T), Prefix(T), Error }
            struct WithFreeModifiers<T, F> { value: T, free_modifiers: Vec<F> }
            fn main() {
                #(#checks)*
                let original = Arc::new(Node(7));
                let result = #identity;
                assert!(Arc::ptr_eq(&original, &result));
                let original = Arc::new(Some(Node(9)));
                let result = #shared_container;
                assert_eq!(original.as_ref().as_ref().unwrap().0, 9);
                assert_eq!(result.as_ref().as_ref().unwrap().0, 9);
            }
        };
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "jbotci-containment-{}-{unique}",
            std::process::id()
        ));
        std::fs::create_dir(&directory).unwrap();
        let source = directory.join("main.rs");
        let binary = directory.join(format!("lowered{}", std::env::consts::EXE_SUFFIX));
        std::fs::write(&source, program.to_string()).unwrap();
        let compilation = std::process::Command::new("rustc")
            .arg("--edition=2024")
            .arg("-Cstrip=symbols")
            .arg(&source)
            .arg("-o")
            .arg(&binary)
            .output()
            .unwrap();
        let execution = compilation
            .status
            .success()
            .then(|| std::process::Command::new(&binary).output().unwrap());
        std::fs::remove_dir_all(&directory).unwrap();
        assert!(
            compilation.status.success(),
            "{}",
            String::from_utf8_lossy(&compilation.stderr)
        );
        let execution = execution.unwrap();
        assert!(
            execution.status.success(),
            "{}",
            String::from_utf8_lossy(&execution.stderr)
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn stored_shapes_preserve_wrappers_and_share_each_node() {
        let nodes = BTreeSet::from(["Node".to_owned()]);
        for (source, expected) in [
            ("Node", "::std::sync::Arc<Node>"),
            ("Option<Node>", "Option<::std::sync::Arc<Node>>"),
            ("Vec<Node>", "Vec<::std::sync::Arc<Node>>"),
            ("Box<Node>", "Box<::std::sync::Arc<Node>>"),
            ("Arc<Node>", "Arc<Node>"),
            ("Arc<Option<Node>>", "Arc<Option<::std::sync::Arc<Node>>>"),
            ("Arc<Arc<Node>>", "Arc<Arc<Node>>"),
            ("(Token, Node)", "(Token, ::std::sync::Arc<Node>)"),
            ("[Node; 3]", "[::std::sync::Arc<Node>; 3]"),
            (
                "SmallVec<[Node; 2]>",
                "SmallVec<[::std::sync::Arc<Node>; 2]>",
            ),
            (
                "WithFreeModifiers<Token, Node>",
                "WithFreeModifiers<Token, ::std::sync::Arc<Node>>",
            ),
        ] {
            let source = syn::parse_str(source).unwrap();
            let expected: Type = syn::parse_str(expected).unwrap();
            assert_eq!(
                Containment::new(&source, &nodes)
                    .stored_type()
                    .to_token_stream()
                    .to_string(),
                expected.to_token_stream().to_string()
            );
        }
    }
}
