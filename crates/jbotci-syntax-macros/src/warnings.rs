//! Warning declarations and their grammar emission sites.

use std::collections::{BTreeMap, BTreeSet};

#[allow(unused_imports)]
use bityzba::{ensures, invariant, new, requires};
use proc_macro2::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{Attribute, Expr, Ident, LitStr, Result, Token, Visibility, braced, parenthesized};

use crate::{ParserExpr, Rule, VectorItem, kw, required_path_expr_last_segment};

#[invariant(true)]
pub(crate) struct WarningCatalog {
    attrs: Vec<Attribute>,
    visibility: Visibility,
    name: Ident,
    entries: Vec<WarningCategory>,
}

#[invariant(!code.value().is_empty() && !message.value().is_empty())]
struct WarningCategory {
    name: Ident,
    code: LitStr,
    message: LitStr,
    runtime: bool,
}

impl Parse for WarningCatalog {
    #[requires(true)]
    #[ensures(true)]
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        input.parse::<kw::warnings>()?;
        let content;
        braced!(content in input);
        let attrs = content.call(Attribute::parse_outer)?;
        let visibility = content.parse()?;
        content.parse::<Token![enum]>()?;
        let name: Ident = content.parse()?;
        if name != "ExperimentalConstruct" {
            return Err(syn::Error::new_spanned(
                &name,
                "expected `ExperimentalConstruct` as the warning enum name",
            ));
        }
        let categories;
        braced!(categories in content);
        let mut entries = Vec::new();
        let mut names = BTreeSet::new();
        let mut codes = BTreeSet::new();
        while !categories.is_empty() {
            let runtime = categories.peek(kw::runtime);
            if runtime {
                categories.parse::<kw::runtime>()?;
            }
            let name: Ident = categories.parse()?;
            categories.parse::<Token![=>]>()?;
            let strings;
            parenthesized!(strings in categories);
            let code: LitStr = strings.parse()?;
            strings.parse::<Token![,]>()?;
            let message: LitStr = strings.parse()?;
            if !strings.is_empty() {
                return Err(strings.error("expected only a warning code and message"));
            }
            if code.value().is_empty() || message.value().is_empty() {
                return Err(syn::Error::new_spanned(
                    &name,
                    "warning code and message must not be empty",
                ));
            }
            if !names.insert(name.to_string()) {
                return Err(syn::Error::new_spanned(&name, "duplicate warning category"));
            }
            if !codes.insert(code.value()) {
                return Err(syn::Error::new_spanned(&code, "duplicate warning code"));
            }
            entries.push(new!(WarningCategory {
                name,
                code,
                message,
                runtime
            }));
            if !categories.is_empty() {
                categories.parse::<Token![,]>()?;
            }
        }
        if !content.is_empty() {
            return Err(content.error("expected only a warning enum declaration"));
        }
        Ok(Self {
            attrs,
            visibility,
            name,
            entries,
        })
    }
}

impl WarningCatalog {
    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn validate_uses(&self, rules: &[Rule]) -> Result<()> {
        let mut uses = WarningUses::default();
        for rule in rules {
            match rule {
                Rule::Alias(rule) => uses.visit_parser(&rule.parser),
                Rule::Struct(rule) => {
                    for field in &rule.fields {
                        uses.visit_parser(&field.parser);
                    }
                }
                Rule::Enum(_) => {}
            }
        }
        if let Some(error) = uses.error {
            return Err(error);
        }
        for (name, site) in &uses.names {
            if !self.entries.iter().any(|entry| entry.name == name) {
                return Err(syn::Error::new(*site, "unknown warning category"));
            }
        }
        for entry in &self.entries {
            if !entry.runtime && !uses.names.contains_key(&entry.name.to_string()) {
                return Err(syn::Error::new_spanned(
                    &entry.name,
                    "warning category has no grammar emission site. Use `runtime` for a runtime-emitted category",
                ));
            }
        }
        Ok(())
    }

    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn expand(&self) -> TokenStream {
        let Self {
            attrs,
            visibility,
            name,
            entries,
        } = self;
        let variants = entries.iter().map(|entry| &entry.name).collect::<Vec<_>>();
        let codes = entries.iter().map(|entry| &entry.code);
        let messages = entries.iter().map(|entry| &entry.message);
        let runtime = entries
            .iter()
            .filter(|entry| entry.runtime)
            .map(|entry| &entry.name);
        quote! {
            #[::bityzba::invariant(true)]
            #(#attrs)*
            #visibility enum #name { #(#variants,)* }
            impl #name {
                pub const ALL: &'static [Self] = &[#(Self::#variants,)*];
                pub const RUNTIME_EMITTED: &'static [Self] = &[#(Self::#runtime,)*];

                #[::bityzba::requires(true)]
                #[::bityzba::ensures(!ret.is_empty())]
                pub const fn code(self) -> &'static str {
                    match self { #(Self::#variants => #codes,)* }
                }

                #[::bityzba::requires(true)]
                #[::bityzba::ensures(!ret.is_empty())]
                pub const fn message(self) -> &'static str {
                    match self { #(Self::#variants => #messages,)* }
                }
            }
        }
    }
}

#[invariant(true)]
#[derive(Default)]
struct WarningUses {
    names: BTreeMap<String, proc_macro2::Span>,
    error: Option<syn::Error>,
}

impl WarningUses {
    #[requires(true)]
    #[ensures(true)]
    fn record(&mut self, method: &Ident, category: Option<&Expr>) {
        if method != "warn" && method != "warn_if" {
            return;
        }
        let Some(category) = category else {
            return;
        };
        match required_path_expr_last_segment(category, "warning category") {
            Ok(name) => {
                self.names.insert(name, category.span());
            }
            Err(error) => {
                self.error = Some(error);
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_parser(&mut self, parser: &ParserExpr) {
        match parser {
            ParserExpr::Rust(expr) => self.visit_expr(expr),
            ParserExpr::Postfix {
                receiver,
                method,
                args,
            } => {
                self.visit_parser(receiver);
                self.record(method, args.first());
                for arg in args {
                    self.visit_expr(arg);
                }
            }
            ParserExpr::Chain(chain) => {
                self.visit_parser(&chain.first);
                self.visit_parser(&chain.links);
            }
            ParserExpr::Vector(vector) => {
                for item in &vector.items {
                    let parser = match item {
                        VectorItem::One(parser)
                        | VectorItem::Spread(parser)
                        | VectorItem::ZeroOrMore(parser)
                        | VectorItem::ZeroOrMoreSpread(parser)
                        | VectorItem::OneOrMore(parser)
                        | VectorItem::OneOrMoreSpread(parser)
                        | VectorItem::Assert { parser, .. } => parser,
                    };
                    self.visit_parser(parser);
                }
            }
        }
    }
}

impl<'ast> Visit<'ast> for WarningUses {
    #[requires(true)]
    #[ensures(true)]
    fn visit_expr_method_call(&mut self, call: &'ast syn::ExprMethodCall) {
        self.record(&call.method, call.args.first());
        syn::visit::visit_expr_method_call(self, call);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SyntaxGrammar;
    use quote::quote;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn grammar_warning_sites_include_nested_methods_and_probes() {
        let grammar = syn::parse2::<SyntaxGrammar>(quote! {
            warnings {
                #[derive(Debug, Clone, Copy)]
                pub enum ExperimentalConstruct {
                    Nested => ("syntax.warning.nested", "nested warning"),
                    Probe => ("syntax.warning.probe", "probe warning"),
                    runtime Runtime => ("syntax.warning.runtime", "runtime warning"),
                }
            }
            rule "item" item -> struct {
                warn_if cmavo(Bo) => Probe;
                field token <- opt(cmavo(Be).warn(Nested));
            }
        })
        .expect("valid warning grammar");
        grammar
            .warnings
            .as_ref()
            .unwrap()
            .validate_uses(&grammar.rules)
            .unwrap();
        let generated = grammar.expand().to_string();
        assert!(!generated.contains("compile_error"));
        assert!(generated.contains("RUNTIME_EMITTED"));
        assert!(generated.contains("syntax.warning.nested"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn warning_sites_in_aliases_vectors_and_chains_are_counted() {
        let grammar = syn::parse2::<SyntaxGrammar>(quote! {
            warnings {
                pub enum ExperimentalConstruct {
                    Alias => ("syntax.warning.alias", "alias warning"),
                    Vector => ("syntax.warning.vector", "vector warning"),
                    Chain => ("syntax.warning.chain", "chain warning"),
                }
            }
            alias "alias" leaf = cmavo(Be).warn(Alias);
            rule "vector" vector -> struct {
                field values <- [cmavo(Be).warn(Vector); ..cmavo(Bo).repeated()];
            }
            rule "chain" linked -> struct {
                field values <- chain(
                    first: cmavo(Be),
                    zero_or_more: cmavo(Bo).warn(Chain),
                    element: token,
                );
            }
        })
        .expect("valid declaration shape");
        grammar
            .warnings
            .as_ref()
            .unwrap()
            .validate_uses(&grammar.rules)
            .unwrap();
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn unknown_warning_probe_is_refused() {
        let grammar = syn::parse2::<SyntaxGrammar>(quote! {
            warnings { pub enum ExperimentalConstruct {} }
            rule "item" item -> struct {
                warn_if cmavo(Bo) => Unknown;
                field token <- cmavo(Be);
            }
        })
        .expect("valid declaration shape");
        let error = grammar
            .warnings
            .as_ref()
            .unwrap()
            .validate_uses(&grammar.rules)
            .unwrap_err();
        assert_eq!(error.to_string(), "unknown warning category");
    }
}
