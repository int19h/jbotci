//! Proc macros for syntax grammar declarations.

use std::collections::BTreeSet;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{ToTokens, quote};
use syn::{
    Expr, ExprCall, ExprMethodCall, ExprPath, Ident, LitStr, Result, Token, Type, braced,
    parenthesized,
    parse::{Parse, ParseStream},
    parse_macro_input,
};

#[proc_macro]
pub fn syntax_grammar(input: TokenStream) -> TokenStream {
    let grammar = parse_macro_input!(input as SyntaxGrammar);
    grammar.expand().into()
}

mod kw {
    syn::custom_keyword!(build);
    syn::custom_keyword!(context);
    syn::custom_keyword!(env);
    syn::custom_keyword!(feature);
    syn::custom_keyword!(field);
    syn::custom_keyword!(fields);
    syn::custom_keyword!(node);
    syn::custom_keyword!(policy);
    syn::custom_keyword!(product);
    syn::custom_keyword!(recovered_build);
    syn::custom_keyword!(recursive);
    syn::custom_keyword!(when);
}

struct SyntaxGrammar {
    env: Type,
    recursive: Vec<RecursiveRule>,
    rules: Vec<Rule>,
}

impl SyntaxGrammar {
    fn expand(&self) -> TokenStream2 {
        let env = compact_tokens(&self.env);
        let recursive = self.recursive.iter().map(RecursiveRule::expand);
        let rules = self.rules.iter().map(Rule::expand_metadata);
        let rule_lookup_arms = self.rules.iter().enumerate().map(|(index, rule)| {
            let name = rule.name().to_string();
            quote!(#name => Some(&SYNTAX_GRAMMAR_RULES[#index]))
        });

        quote! {
            #[derive(Debug, Clone, Copy, PartialEq, Eq)]
            pub(crate) struct SyntaxGrammarRecursiveRule {
                pub name: &'static str,
                pub output: &'static str,
            }

            #[derive(Debug, Clone, Copy)]
            pub(crate) struct SyntaxGrammarField {
                pub kind: &'static str,
                pub name: &'static str,
                pub parser: &'static str,
                pub recovery: SyntaxGrammarRecoveryExpr,
                pub conditions: &'static [SyntaxGrammarCondition],
            }

            #[derive(Debug, Clone, Copy, PartialEq, Eq)]
            pub(crate) struct SyntaxGrammarCondition {
                pub kind: SyntaxGrammarConditionKind,
                pub name: &'static str,
            }

            #[derive(Debug, Clone, Copy, PartialEq, Eq)]
            pub(crate) enum SyntaxGrammarConditionKind {
                Feature,
                Policy,
            }

            #[derive(Debug, Clone, Copy, PartialEq, Eq)]
            pub(crate) enum SyntaxGrammarTokenPredicate {
                TaggedSelbriParentOwnedStart,
                TaggedSumtiTermParentOwnedStart,
            }

            #[derive(Debug, Clone, Copy, PartialEq, Eq)]
            pub(crate) enum SyntaxGrammarRecoveryExpr {
                Cmavo(Cmavo),
                Selmaho(Selmaho),
                WordCategory(crate::SyntaxWordCategory),
                ExactWordCategory(crate::SyntaxWordCategory),
                Opt(&'static SyntaxGrammarRecoveryExpr),
                Many(&'static SyntaxGrammarRecoveryExpr),
                Many1(&'static SyntaxGrammarRecoveryExpr),
                Boxed(&'static SyntaxGrammarRecoveryExpr),
                Arc(&'static SyntaxGrammarRecoveryExpr),
                WithFreeModifiers(&'static SyntaxGrammarRecoveryExpr),
                PayloadStart(&'static SyntaxGrammarRecoveryExpr),
                NotNextSelmaho(Selmaho),
                NotNextToken(SyntaxGrammarTokenPredicate),
                NotNextRule(&'static str),
                Lookahead(&'static SyntaxGrammarRecoveryExpr),
                Not(&'static SyntaxGrammarRecoveryExpr),
                Choice(&'static [SyntaxGrammarRecoveryExpr]),
                Sequence(&'static [SyntaxGrammarRecoveryExpr]),
                BareNegationTerm,
                RelationWord,
                Rule(&'static str),
                Opaque(&'static str),
                Eof,
            }

            #[derive(Debug, Clone, Copy)]
            pub(crate) struct SyntaxGrammarRule {
                pub kind: &'static str,
                pub name: &'static str,
                pub arguments: &'static [&'static str],
                pub output: &'static str,
                pub context: Option<&'static str>,
                pub fields: &'static [SyntaxGrammarField],
            }

            pub(crate) const SYNTAX_GRAMMAR_ENV: &str = #env;
            pub(crate) const SYNTAX_GRAMMAR_RECURSIVE_RULES: &[SyntaxGrammarRecursiveRule] = &[
                #(#recursive),*
            ];
            pub(crate) const SYNTAX_GRAMMAR_RULES: &[SyntaxGrammarRule] = &[
                #(#rules),*
            ];

            #[bityzba::requires(!name.is_empty())]
            #[bityzba::ensures(ret.as_ref().is_none_or(|rule| rule.name == name))]
            pub(crate) fn syntax_grammar_rule_by_name(
                name: &str,
            ) -> Option<&'static SyntaxGrammarRule> {
                match name {
                    #(#rule_lookup_arms,)*
                    _ => None,
                }
            }
        }
    }
}

impl Parse for SyntaxGrammar {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        input.parse::<kw::env>()?;
        let env = input.parse()?;
        input.parse::<Token![;]>()?;

        let mut recursive = Vec::new();
        let mut rules = Vec::new();
        while !input.is_empty() {
            if input.peek(kw::recursive) {
                recursive = parse_recursive_block(input)?;
            } else if input.peek(kw::node) {
                rules.push(Rule::Node(input.parse()?));
            } else if input.peek(kw::product) {
                rules.push(Rule::Product(input.parse()?));
            } else {
                return Err(input.error("expected `recursive`, `node`, or `product`"));
            }
        }

        Ok(Self {
            env,
            recursive,
            rules,
        })
    }
}

fn parse_recursive_block(input: ParseStream<'_>) -> Result<Vec<RecursiveRule>> {
    input.parse::<kw::recursive>()?;
    let content;
    braced!(content in input);
    let mut rules = Vec::new();
    while !content.is_empty() {
        rules.push(content.parse()?);
    }
    Ok(rules)
}

struct RecursiveRule {
    name: Ident,
    output: Type,
}

impl RecursiveRule {
    fn expand(&self) -> TokenStream2 {
        let name = self.name.to_string();
        let output = compact_tokens(&self.output);
        quote! {
            SyntaxGrammarRecursiveRule {
                name: #name,
                output: #output,
            }
        }
    }
}

impl Parse for RecursiveRule {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let name = input.parse()?;
        input.parse::<Token![:]>()?;
        let output = input.parse()?;
        input.parse::<Token![;]>()?;
        Ok(Self { name, output })
    }
}

enum Rule {
    Node(NodeRule),
    Product(ProductRule),
}

impl Rule {
    fn name(&self) -> &Ident {
        match self {
            Rule::Node(rule) => &rule.name,
            Rule::Product(rule) => &rule.0.name,
        }
    }

    fn expand_metadata(&self) -> TokenStream2 {
        match self {
            Rule::Node(rule) => rule.expand_metadata("node"),
            Rule::Product(rule) => rule.0.expand_metadata("product"),
        }
    }
}

struct NodeRule {
    name: Ident,
    arguments: Vec<Ident>,
    output: Type,
    context: Option<LitStr>,
    fields: Vec<FieldItem>,
}

impl NodeRule {
    fn expand_metadata(&self, kind: &'static str) -> TokenStream2 {
        let name = self.name.to_string();
        let arguments = self.arguments.iter().map(Ident::to_string);
        let output = compact_tokens(&self.output);
        let context = self.context.as_ref().map_or_else(
            || quote!(None),
            |context| {
                let value = context.value();
                quote!(Some(#value))
            },
        );
        let argument_names = self
            .arguments
            .iter()
            .map(Ident::to_string)
            .collect::<BTreeSet<_>>();
        let fields = self
            .fields
            .iter()
            .map(|field| field.expand(&argument_names));
        quote! {
            SyntaxGrammarRule {
                kind: #kind,
                name: #name,
                arguments: &[#(#arguments),*],
                output: #output,
                context: #context,
                fields: &[#(#fields),*],
            }
        }
    }
}

impl Parse for NodeRule {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        input.parse::<kw::node>()?;
        parse_rule_after_kind(input)
    }
}

struct ProductRule(NodeRule);

impl Parse for ProductRule {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        input.parse::<kw::product>()?;
        parse_rule_after_kind(input).map(Self)
    }
}

fn parse_rule_after_kind(input: ParseStream<'_>) -> Result<NodeRule> {
    let name = input.parse()?;
    let arguments = parse_optional_arguments(input)?;
    input.parse::<Token![->]>()?;
    let output = input.parse()?;
    let content;
    braced!(content in input);

    let mut context = None;
    let mut fields = Vec::new();
    while !content.is_empty() {
        if content.peek(kw::context) {
            content.parse::<kw::context>()?;
            context = Some(content.parse()?);
            content.parse::<Token![;]>()?;
        } else if content.peek(kw::fields) {
            fields = parse_fields_block(&content)?;
        } else if content.peek(kw::build) {
            parse_ignored_closure_item(&content, true)?;
        } else if content.peek(kw::recovered_build) {
            parse_ignored_closure_item(&content, false)?;
        } else {
            return Err(
                content.error("expected `context`, `fields`, `build`, or `recovered_build`")
            );
        }
    }

    Ok(NodeRule {
        name,
        arguments,
        output,
        context,
        fields,
    })
}

fn parse_optional_arguments(input: ParseStream<'_>) -> Result<Vec<Ident>> {
    if !input.peek(syn::token::Paren) {
        return Ok(Vec::new());
    }
    let content;
    parenthesized!(content in input);
    let mut arguments = Vec::new();
    while !content.is_empty() {
        arguments.push(content.parse()?);
        if content.peek(Token![,]) {
            content.parse::<Token![,]>()?;
        }
    }
    Ok(arguments)
}

fn parse_ignored_closure_item(input: ParseStream<'_>, strict: bool) -> Result<()> {
    if strict {
        input.parse::<kw::build>()?;
    } else {
        input.parse::<kw::recovered_build>()?;
    }
    let _closure: syn::ExprClosure = input.parse()?;
    input.parse::<Token![;]>()?;
    Ok(())
}

fn parse_fields_block(input: ParseStream<'_>) -> Result<Vec<FieldItem>> {
    input.parse::<kw::fields>()?;
    let content;
    braced!(content in input);
    let mut fields = Vec::new();
    while !content.is_empty() {
        fields.push(content.parse()?);
    }
    Ok(fields)
}

struct FieldItem {
    conditions: Vec<Condition>,
    kind: FieldKind,
    name: Ident,
    parser: Expr,
}

impl FieldItem {
    fn expand(&self, arguments: &BTreeSet<String>) -> TokenStream2 {
        let kind = self.kind.as_str();
        let name = self.name.to_string();
        let parser = compact_tokens(&self.parser);
        let recovery = classify_recovery_expr(&self.parser, arguments).expand();
        let conditions = self.conditions.iter().map(Condition::expand);
        quote! {
            SyntaxGrammarField {
                kind: #kind,
                name: #name,
                parser: #parser,
                recovery: #recovery,
                conditions: &[#(#conditions),*],
            }
        }
    }
}

impl Parse for FieldItem {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let mut conditions = Vec::new();
        while input.peek(kw::when) {
            conditions.push(input.parse()?);
        }

        let kind = if input.peek(kw::field) {
            input.parse::<kw::field>()?;
            FieldKind::Field
        } else if input.peek(Token![let]) {
            input.parse::<Token![let]>()?;
            FieldKind::Let
        } else {
            return Err(input.error("expected `field` or `let`"));
        };
        let name = input.parse()?;
        input.parse::<Token![=]>()?;
        let parser = input.parse()?;
        input.parse::<Token![;]>()?;
        Ok(Self {
            conditions,
            kind,
            name,
            parser,
        })
    }
}

enum FieldKind {
    Field,
    Let,
}

impl FieldKind {
    fn as_str(&self) -> &'static str {
        match self {
            FieldKind::Field => "field",
            FieldKind::Let => "let",
        }
    }
}

struct Condition {
    kind: ConditionKind,
    name: Ident,
}

impl Condition {
    fn expand(&self) -> TokenStream2 {
        let kind = match self.kind {
            ConditionKind::Feature => quote!(SyntaxGrammarConditionKind::Feature),
            ConditionKind::Policy => quote!(SyntaxGrammarConditionKind::Policy),
        };
        let name = self.name.to_string();
        quote! {
            SyntaxGrammarCondition {
                kind: #kind,
                name: #name,
            }
        }
    }
}

impl Parse for Condition {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        input.parse::<kw::when>()?;
        if input.peek(kw::feature) {
            input.parse::<kw::feature>()?;
            let content;
            parenthesized!(content in input);
            Ok(Self {
                kind: ConditionKind::Feature,
                name: content.parse()?,
            })
        } else if input.peek(kw::policy) {
            input.parse::<kw::policy>()?;
            let content;
            parenthesized!(content in input);
            Ok(Self {
                kind: ConditionKind::Policy,
                name: content.parse()?,
            })
        } else {
            Err(input.error("expected `feature` or `policy` condition"))
        }
    }
}

enum ConditionKind {
    Feature,
    Policy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RecoveryExpr {
    Cmavo(String),
    Selmaho(String),
    WordCategory(String),
    ExactWordCategory(String),
    Opt(Box<RecoveryExpr>),
    Many(Box<RecoveryExpr>),
    Many1(Box<RecoveryExpr>),
    Boxed(Box<RecoveryExpr>),
    Arc(Box<RecoveryExpr>),
    WithFreeModifiers(Box<RecoveryExpr>),
    PayloadStart(Box<RecoveryExpr>),
    NotNextSelmaho(String),
    NotNextToken(String),
    NotNextRule(String),
    Lookahead(Box<RecoveryExpr>),
    Not(Box<RecoveryExpr>),
    Choice(Vec<RecoveryExpr>),
    Sequence(Vec<RecoveryExpr>),
    BareNegationTerm,
    RelationWord,
    Rule(String),
    Opaque(String),
    Eof,
}

impl RecoveryExpr {
    fn expand(self) -> TokenStream2 {
        match self {
            RecoveryExpr::Cmavo(cmavo) => {
                let cmavo = syn::Ident::new(&cmavo, proc_macro2::Span::call_site());
                quote!(SyntaxGrammarRecoveryExpr::Cmavo(Cmavo::#cmavo))
            }
            RecoveryExpr::Selmaho(selmaho) => {
                let selmaho = syn::Ident::new(&selmaho, proc_macro2::Span::call_site());
                quote!(SyntaxGrammarRecoveryExpr::Selmaho(Selmaho::#selmaho))
            }
            RecoveryExpr::WordCategory(category) => {
                let category = syn::Ident::new(&category, proc_macro2::Span::call_site());
                quote!(SyntaxGrammarRecoveryExpr::WordCategory(crate::SyntaxWordCategory::#category))
            }
            RecoveryExpr::ExactWordCategory(category) => {
                let category = syn::Ident::new(&category, proc_macro2::Span::call_site());
                quote!(SyntaxGrammarRecoveryExpr::ExactWordCategory(crate::SyntaxWordCategory::#category))
            }
            RecoveryExpr::Opt(inner) => {
                let inner = inner.expand();
                quote!(SyntaxGrammarRecoveryExpr::Opt(&#inner))
            }
            RecoveryExpr::Many(inner) => {
                let inner = inner.expand();
                quote!(SyntaxGrammarRecoveryExpr::Many(&#inner))
            }
            RecoveryExpr::Many1(inner) => {
                let inner = inner.expand();
                quote!(SyntaxGrammarRecoveryExpr::Many1(&#inner))
            }
            RecoveryExpr::Boxed(inner) => {
                let inner = inner.expand();
                quote!(SyntaxGrammarRecoveryExpr::Boxed(&#inner))
            }
            RecoveryExpr::Arc(inner) => {
                let inner = inner.expand();
                quote!(SyntaxGrammarRecoveryExpr::Arc(&#inner))
            }
            RecoveryExpr::WithFreeModifiers(inner) => {
                let inner = inner.expand();
                quote!(SyntaxGrammarRecoveryExpr::WithFreeModifiers(&#inner))
            }
            RecoveryExpr::PayloadStart(inner) => {
                let inner = inner.expand();
                quote!(SyntaxGrammarRecoveryExpr::PayloadStart(&#inner))
            }
            RecoveryExpr::NotNextSelmaho(selmaho) => {
                let selmaho = syn::Ident::new(&selmaho, proc_macro2::Span::call_site());
                quote!(SyntaxGrammarRecoveryExpr::NotNextSelmaho(Selmaho::#selmaho))
            }
            RecoveryExpr::NotNextToken(predicate) => {
                let predicate = syn::Ident::new(&predicate, proc_macro2::Span::call_site());
                quote!(SyntaxGrammarRecoveryExpr::NotNextToken(SyntaxGrammarTokenPredicate::#predicate))
            }
            RecoveryExpr::NotNextRule(rule) => {
                quote!(SyntaxGrammarRecoveryExpr::NotNextRule(#rule))
            }
            RecoveryExpr::Lookahead(inner) => {
                let inner = inner.expand();
                quote!(SyntaxGrammarRecoveryExpr::Lookahead(&#inner))
            }
            RecoveryExpr::Not(inner) => {
                let inner = inner.expand();
                quote!(SyntaxGrammarRecoveryExpr::Not(&#inner))
            }
            RecoveryExpr::Choice(alternatives) => {
                let alternatives = alternatives.into_iter().map(RecoveryExpr::expand);
                quote!(SyntaxGrammarRecoveryExpr::Choice(&[#(#alternatives),*]))
            }
            RecoveryExpr::Sequence(parts) => {
                let parts = parts.into_iter().map(RecoveryExpr::expand);
                quote!(SyntaxGrammarRecoveryExpr::Sequence(&[#(#parts),*]))
            }
            RecoveryExpr::BareNegationTerm => quote!(SyntaxGrammarRecoveryExpr::BareNegationTerm),
            RecoveryExpr::RelationWord => quote!(SyntaxGrammarRecoveryExpr::RelationWord),
            RecoveryExpr::Rule(rule) => quote!(SyntaxGrammarRecoveryExpr::Rule(#rule)),
            RecoveryExpr::Opaque(text) => quote!(SyntaxGrammarRecoveryExpr::Opaque(#text)),
            RecoveryExpr::Eof => quote!(SyntaxGrammarRecoveryExpr::Eof),
        }
    }
}

fn classify_recovery_expr(expr: &Expr, arguments: &BTreeSet<String>) -> RecoveryExpr {
    match expr {
        Expr::Call(call) => classify_call_recovery_expr(call, arguments),
        Expr::MethodCall(method) => classify_method_recovery_expr(method, arguments),
        Expr::Path(path) => classify_path_recovery_expr(path, arguments),
        Expr::Tuple(tuple) => RecoveryExpr::Sequence(
            tuple
                .elems
                .iter()
                .map(|expr| classify_recovery_expr(expr, arguments))
                .collect(),
        ),
        _ => RecoveryExpr::Opaque(compact_tokens(expr)),
    }
}

fn classify_method_recovery_expr(
    method: &ExprMethodCall,
    arguments: &BTreeSet<String>,
) -> RecoveryExpr {
    let inner = || Box::new(classify_recovery_expr(&method.receiver, arguments));
    match (method.method.to_string().as_str(), method.args.len()) {
        ("wf", 0) | ("with_free_modifiers", 0) => RecoveryExpr::WithFreeModifiers(inner()),
        ("payload_start", 0) => RecoveryExpr::PayloadStart(inner()),
        ("not_next_selmaho", 1) => method
            .args
            .first()
            .and_then(path_expr_last_segment)
            .map(RecoveryExpr::NotNextSelmaho)
            .unwrap_or_else(|| RecoveryExpr::Opaque(compact_tokens(method))),
        ("not_next_token", 1) => method
            .args
            .first()
            .and_then(path_expr_last_segment)
            .map(RecoveryExpr::NotNextToken)
            .unwrap_or_else(|| RecoveryExpr::Opaque(compact_tokens(method))),
        ("not_next_rule", 1) => method
            .args
            .first()
            .and_then(path_expr_last_segment)
            .map(RecoveryExpr::NotNextRule)
            .unwrap_or_else(|| RecoveryExpr::Opaque(compact_tokens(method))),
        ("lookahead", 0) => RecoveryExpr::Lookahead(inner()),
        ("not", 0) => RecoveryExpr::Not(inner()),
        _ => RecoveryExpr::Opaque(compact_tokens(method)),
    }
}

fn classify_call_recovery_expr(call: &ExprCall, arguments: &BTreeSet<String>) -> RecoveryExpr {
    let Some(name) = call_name(call) else {
        return RecoveryExpr::Opaque(compact_tokens(call));
    };
    match (name.as_str(), call.args.len()) {
        ("cmavo", 1) => call
            .args
            .first()
            .and_then(path_expr_last_segment)
            .map(RecoveryExpr::Cmavo)
            .unwrap_or_else(|| RecoveryExpr::Opaque(compact_tokens(call))),
        ("selmaho", 1) => call
            .args
            .first()
            .and_then(path_expr_last_segment)
            .map(RecoveryExpr::Selmaho)
            .unwrap_or_else(|| RecoveryExpr::Opaque(compact_tokens(call))),
        ("word_category", 1) => call
            .args
            .first()
            .and_then(path_expr_last_segment)
            .map(RecoveryExpr::WordCategory)
            .unwrap_or_else(|| RecoveryExpr::Opaque(compact_tokens(call))),
        ("exact_word_category", 1) => call
            .args
            .first()
            .and_then(path_expr_last_segment)
            .map(RecoveryExpr::ExactWordCategory)
            .unwrap_or_else(|| RecoveryExpr::Opaque(compact_tokens(call))),
        ("opt", 1) => RecoveryExpr::Opt(Box::new(classify_recovery_expr(&call.args[0], arguments))),
        ("many" | "many_local", 1) => {
            RecoveryExpr::Many(Box::new(classify_recovery_expr(&call.args[0], arguments)))
        }
        ("many1", 1) | ("nonempty", 1) => {
            RecoveryExpr::Many1(Box::new(classify_recovery_expr(&call.args[0], arguments)))
        }
        ("boxed", 1) => {
            RecoveryExpr::Boxed(Box::new(classify_recovery_expr(&call.args[0], arguments)))
        }
        ("arc", 1) => RecoveryExpr::Arc(Box::new(classify_recovery_expr(&call.args[0], arguments))),
        ("recover_as", 2) => call
            .args
            .first()
            .and_then(path_expr_last_segment)
            .map(RecoveryExpr::Rule)
            .unwrap_or_else(|| RecoveryExpr::Opaque(compact_tokens(call))),
        ("choice", _) => RecoveryExpr::Choice(
            call.args
                .iter()
                .map(|expr| classify_recovery_expr(expr, arguments))
                .collect(),
        ),
        ("seq" | "sequence", _) => RecoveryExpr::Sequence(
            call.args
                .iter()
                .map(|expr| classify_recovery_expr(expr, arguments))
                .collect(),
        ),
        ("relation_word" | "tanru_unit_relation_word", 0) => RecoveryExpr::RelationWord,
        ("bare_negation_term", 0) => RecoveryExpr::BareNegationTerm,
        ("eof", 0) => RecoveryExpr::Eof,
        _ if call.args.is_empty() && arguments.contains(&name) => RecoveryExpr::Rule(name),
        _ if call.args.is_empty() => RecoveryExpr::Rule(name),
        _ => RecoveryExpr::Opaque(compact_tokens(call)),
    }
}

fn classify_path_recovery_expr(path: &ExprPath, arguments: &BTreeSet<String>) -> RecoveryExpr {
    let text = compact_tokens(path);
    if arguments.contains(&text) {
        RecoveryExpr::Rule(text)
    } else {
        RecoveryExpr::Opaque(text)
    }
}

fn call_name(call: &ExprCall) -> Option<String> {
    let Expr::Path(path) = call.func.as_ref() else {
        return None;
    };
    path.path
        .segments
        .last()
        .map(|segment| segment.ident.to_string())
}

fn path_expr_last_segment(expr: &Expr) -> Option<String> {
    let Expr::Path(path) = expr else {
        return None;
    };
    path.path
        .segments
        .last()
        .map(|segment| segment.ident.to_string())
}

fn compact_tokens(tokens: impl ToTokens) -> String {
    tokens
        .to_token_stream()
        .to_string()
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect()
}
