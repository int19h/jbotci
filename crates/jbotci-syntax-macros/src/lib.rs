//! Proc macros for syntax grammar declarations.

use std::collections::{BTreeMap, BTreeSet};

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{ToTokens, format_ident, quote};
use syn::{
    Attribute, Expr, ExprCall, ExprClosure, ExprMethodCall, ExprPath, ExprTuple, Ident, LitStr,
    Path, Result, Token, Type, braced, parenthesized,
    parse::{Parse, ParseStream},
    parse_macro_input,
};

#[proc_macro]
pub fn syntax_grammar(input: TokenStream) -> TokenStream {
    let grammar = parse_macro_input!(input as SyntaxGrammar);
    grammar.expand().into()
}

mod kw {
    syn::custom_keyword!(alias);
    syn::custom_keyword!(build);
    syn::custom_keyword!(construct);
    syn::custom_keyword!(context);
    syn::custom_keyword!(default);
    syn::custom_keyword!(direct);
    syn::custom_keyword!(env);
    syn::custom_keyword!(feature);
    syn::custom_keyword!(field);
    syn::custom_keyword!(fields);
    syn::custom_keyword!(model);
    syn::custom_keyword!(node);
    syn::custom_keyword!(policy);
    syn::custom_keyword!(product);
    syn::custom_keyword!(recovered);
    syn::custom_keyword!(recovered_build);
    syn::custom_keyword!(recursive);
    syn::custom_keyword!(require);
    syn::custom_keyword!(parsers);
    syn::custom_keyword!(scratch);
    syn::custom_keyword!(tree_model);
    syn::custom_keyword!(tuple_variant);
    syn::custom_keyword!(variant);
    syn::custom_keyword!(when);
}

struct SyntaxGrammar {
    tree_model: Option<syn::File>,
    generate_model: bool,
    env: Option<Type>,
    recovered_module: Option<Path>,
    generate_parsers: bool,
    recursive: Vec<RecursiveRule>,
    rules: Vec<Rule>,
}

impl SyntaxGrammar {
    fn expand(&self) -> TokenStream2 {
        let type_env = GrammarTypeEnv::new(&self.recursive, &self.rules);
        let tree_model = if self.generate_model {
            match self.expand_generated_tree_model(&type_env) {
                Ok(tree_model) => Some(tree_model),
                Err(error) => return error.into_compile_error(),
            }
        } else {
            self.tree_model.as_ref().map(expand_tree_model_block)
        };
        let Some(env) = &self.env else {
            return quote! {
                #tree_model
            };
        };
        let env = compact_tokens(env);
        let recovered_module = self.recovered_module_tokens();
        let helper_outputs = self.product_helper_outputs();
        let product_helpers = self.expand_product_helpers(&helper_outputs, &type_env);
        let recursive = self.recursive.iter().map(RecursiveRule::expand);
        let rules = self.rules.iter().map(Rule::expand_metadata);
        let rule_lookup_arms = self.rules.iter().enumerate().map(|(index, rule)| {
            let name = rule.name().to_string();
            quote!(#name => Some(&SYNTAX_GRAMMAR_RULES[#index]))
        });
        let parser_functions = if self.generate_parsers {
            self.rules
                .iter()
                .filter_map(|rule| rule.expand_strict_parser(&helper_outputs, &type_env))
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        let partial_valid_parser_functions = if self.generate_parsers {
            self.rules
                .iter()
                .filter_map(|rule| {
                    rule.expand_partial_valid_parser(&helper_outputs, &type_env, &recovered_module)
                })
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        let recursive_family = if self.generate_parsers {
            self.expand_strict_recursive_family()
        } else {
            None
        };
        let recursive_partial_valid = if self.generate_parsers {
            self.expand_partial_valid_recursive_roots(&recovered_module)
        } else {
            Vec::new()
        };

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
                Some(&'static SyntaxGrammarRecoveryExpr),
                Boxed(&'static SyntaxGrammarRecoveryExpr),
                Arc(&'static SyntaxGrammarRecoveryExpr),
                WithFreeModifiers(&'static SyntaxGrammarRecoveryExpr),
                PayloadStart(&'static SyntaxGrammarRecoveryExpr),
                Ignored(&'static SyntaxGrammarRecoveryExpr),
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
            #(#product_helpers)*
            #(#parser_functions)*
            #(#partial_valid_parser_functions)*
            #recursive_family
            #(#recursive_partial_valid)*

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
        let tree_model = if input.peek(kw::tree_model) {
            Some(parse_tree_model_block(input)?)
        } else {
            None
        };

        let generate_model = if input.peek(kw::model) {
            input.parse::<kw::model>()?;
            input.parse::<Token![;]>()?;
            true
        } else {
            false
        };

        let env = if input.peek(kw::env) {
            input.parse::<kw::env>()?;
            let env = input.parse()?;
            input.parse::<Token![;]>()?;
            Some(env)
        } else {
            None
        };

        let recovered_module = if input.peek(kw::recovered) {
            input.parse::<kw::recovered>()?;
            let path = input.parse()?;
            input.parse::<Token![;]>()?;
            Some(path)
        } else {
            None
        };

        let generate_parsers = if env.is_some() && input.peek(kw::parsers) {
            input.parse::<kw::parsers>()?;
            input.parse::<Token![;]>()?;
            true
        } else {
            false
        };

        let mut recursive = Vec::new();
        let mut rules = Vec::new();
        while !input.is_empty() {
            if input.peek(kw::recursive) {
                recursive = parse_recursive_block(input)?;
            } else if input.peek(kw::alias) {
                rules.push(Rule::Alias(input.parse()?));
            } else if input.peek(kw::node) {
                rules.push(Rule::Node(input.parse()?));
            } else if input.peek(kw::product) {
                rules.push(Rule::Product(input.parse()?));
            } else {
                return Err(input.error("expected `recursive`, `alias`, `node`, or `product`"));
            }
        }

        Ok(Self {
            tree_model,
            generate_model,
            env,
            recovered_module,
            generate_parsers,
            recursive,
            rules,
        })
    }
}

fn parse_tree_model_block(input: ParseStream<'_>) -> Result<syn::File> {
    input.parse::<kw::tree_model>()?;
    let content;
    braced!(content in input);
    let attrs = content.call(syn::Attribute::parse_inner)?;
    let mut items = Vec::new();
    while !content.is_empty() {
        items.push(content.parse()?);
    }
    Ok(syn::File {
        shebang: None,
        attrs,
        items,
    })
}

fn expand_tree_model_block(file: &syn::File) -> TokenStream2 {
    let attrs = &file.attrs;
    let items = &file.items;
    quote! {
        jbotci_tree::tree_model! {
            #(#attrs)*
            #(#items)*
        }
    }
}

impl SyntaxGrammar {
    fn expand_generated_tree_model(&self, type_env: &GrammarTypeEnv) -> Result<TokenStream2> {
        let attrs = self
            .tree_model
            .as_ref()
            .map(|file| file.attrs.as_slice())
            .unwrap_or(&[]);
        let manual_items = self
            .tree_model
            .as_ref()
            .map(|file| file.items.as_slice())
            .unwrap_or(&[]);
        let generated_items = self.generated_tree_model_items(type_env)?;
        Ok(quote! {
            jbotci_tree::tree_model! {
                #(#attrs)*
                #(#manual_items)*
                #(#generated_items)*
            }
        })
    }

    fn generated_tree_model_items(&self, type_env: &GrammarTypeEnv) -> Result<Vec<TokenStream2>> {
        let mut structs = BTreeMap::<String, GeneratedStructModel>::new();
        let mut enums = BTreeMap::<String, Vec<GeneratedVariantModel>>::new();
        for rule in &self.rules {
            let (rule_kind, rule) = match rule {
                Rule::Alias(_) => continue,
                Rule::Node(rule) => (GeneratedModelRuleKind::Node, rule),
                Rule::Product(rule) => (GeneratedModelRuleKind::Product, &rule.0),
            };
            let Some(output) = simple_type_ident(&rule.output) else {
                continue;
            };
            match &rule.construction {
                ConstructionMode::NamedVariant(variant) => {
                    let fields = rule.generated_model_fields(type_env)?;
                    enums
                        .entry(output.to_string())
                        .or_default()
                        .push(GeneratedVariantModel {
                            variant: variant.clone(),
                            fields,
                            tuple: false,
                        });
                }
                ConstructionMode::TupleVariant(variant) => {
                    let fields = rule.generated_model_fields(type_env)?;
                    enums
                        .entry(output.to_string())
                        .or_default()
                        .push(GeneratedVariantModel {
                            variant: variant.clone(),
                            fields,
                            tuple: true,
                        });
                }
                ConstructionMode::Validated | ConstructionMode::Direct => {
                    let key = output.to_string();
                    if let Some(existing) = structs.get(&key) {
                        return Err(syn::Error::new_spanned(
                            &rule.name,
                            format!(
                                "cannot generate one struct `{key}` from both `{}` and `{}`; use alias rules for delegation or construct variants for alternatives",
                                existing.rule_name, rule.name
                            ),
                        ));
                    }
                    let fields = rule.generated_model_fields(type_env)?;
                    structs.insert(
                        key,
                        GeneratedStructModel {
                            visibility: rule_kind.visibility_tokens(),
                            ident: output.clone(),
                            rule_name: rule.name.clone(),
                            fields,
                        },
                    );
                }
            }
        }
        for output in enums.keys() {
            if structs.contains_key(output) {
                return Err(syn::Error::new_spanned(
                    format_ident!("{output}"),
                    format!(
                        "cannot generate `{output}` as both a struct and an enum; use construct variants consistently"
                    ),
                ));
            }
        }

        let mut items = Vec::new();
        items.extend(structs.values().map(GeneratedStructModel::expand));
        items.extend(enums.iter().map(|(name, variants)| {
            let ident = format_ident!("{name}");
            let invariants = variants.iter().map(|variant| {
                let variant = &variant.variant;
                quote!(#[bityzba::invariant(::#variant => true)])
            });
            quote! {
                #[bityzba::invariant(true)]
                #(#invariants)*
                #[derive(Debug, Clone, PartialEq, Eq, ::serde::Serialize)]
                pub enum #ident {
                    #(#variants,)*
                }
            }
        }));
        Ok(items)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GeneratedModelRuleKind {
    Node,
    Product,
}

impl GeneratedModelRuleKind {
    fn visibility_tokens(self) -> TokenStream2 {
        match self {
            Self::Node => quote!(pub),
            Self::Product => quote!(pub),
        }
    }
}

struct GeneratedStructModel {
    visibility: TokenStream2,
    ident: Ident,
    rule_name: Ident,
    fields: Vec<GeneratedFieldModel>,
}

impl GeneratedStructModel {
    fn expand(&self) -> TokenStream2 {
        let visibility = &self.visibility;
        let ident = &self.ident;
        let fields = self.fields.iter().map(GeneratedFieldModel::expand_struct);
        quote! {
            #[bityzba::invariant(true)]
            #[derive(Debug, Clone, PartialEq, Eq, ::serde::Serialize)]
            #visibility struct #ident {
                #(#fields,)*
            }
        }
    }
}

struct GeneratedVariantModel {
    variant: Ident,
    fields: Vec<GeneratedFieldModel>,
    tuple: bool,
}

impl ToTokens for GeneratedVariantModel {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let variant = &self.variant;
        let expanded = if self.tuple {
            let types = self.fields.iter().map(GeneratedFieldModel::expand_tuple);
            quote!(#variant(#(#types),*))
        } else {
            let fields = self
                .fields
                .iter()
                .map(GeneratedFieldModel::expand_variant_named);
            quote!(#variant { #(#fields,)* })
        };
        tokens.extend(expanded);
    }
}

struct GeneratedFieldModel {
    attrs: Vec<Attribute>,
    name: Ident,
    ty: TokenStream2,
}

impl GeneratedFieldModel {
    fn expand_struct(&self) -> TokenStream2 {
        let attrs = &self.attrs;
        let name = &self.name;
        let ty = &self.ty;
        quote!(#(#attrs)* pub #name: #ty)
    }

    fn expand_variant_named(&self) -> TokenStream2 {
        let attrs = &self.attrs;
        let name = &self.name;
        let ty = &self.ty;
        quote!(#(#attrs)* #name: #ty)
    }

    fn expand_tuple(&self) -> TokenStream2 {
        self.ty.clone()
    }
}

impl SyntaxGrammar {
    fn recovered_module_tokens(&self) -> TokenStream2 {
        self.recovered_module.as_ref().map_or_else(
            || quote!(crate::tree::recovered),
            |recovered_module| quote!(#recovered_module),
        )
    }

    fn product_helper_outputs(&self) -> BTreeSet<String> {
        let mut output_counts = BTreeMap::new();
        for rule in &self.rules {
            if let Rule::Product(rule) = rule
                && rule.0.build.is_none()
                && matches!(&rule.0.construction, ConstructionMode::Validated)
                && rule.0.fields.iter().all(|field| {
                    !matches!(
                        field.kind,
                        FieldKind::Default | FieldKind::Let | FieldKind::Scratch
                    )
                })
                && let Some(output) = simple_type_ident(&rule.0.output)
            {
                *output_counts.entry(output.to_string()).or_insert(0usize) += 1;
            }
        }
        self.rules
            .iter()
            .filter_map(|rule| {
                let Rule::Product(rule) = rule else {
                    return None;
                };
                let output = simple_type_ident(&rule.0.output)?;
                (rule.0.build.is_none()
                    && matches!(&rule.0.construction, ConstructionMode::Validated)
                    && output_counts.get(&output.to_string()).copied() == Some(1)
                    && rule.0.fields.iter().all(|field| {
                        !matches!(
                            field.kind,
                            FieldKind::Default | FieldKind::Let | FieldKind::Scratch
                        )
                    }))
                .then(|| output.to_string())
            })
            .collect()
    }

    fn expand_product_helpers(
        &self,
        helper_outputs: &BTreeSet<String>,
        type_env: &GrammarTypeEnv,
    ) -> Vec<TokenStream2> {
        self.rules
            .iter()
            .filter_map(|rule| match rule {
                Rule::Product(rule)
                    if simple_type_ident(&rule.0.output)
                        .is_some_and(|output| helper_outputs.contains(&output.to_string())) =>
                {
                    rule.0.expand_product_helper(type_env)
                }
                Rule::Alias(_) | Rule::Product(_) | Rule::Node(_) => None,
            })
            .collect()
    }

    fn expand_strict_recursive_family(&self) -> Option<TokenStream2> {
        if self.recursive.is_empty() {
            return None;
        }
        let recursive_names = self
            .recursive
            .iter()
            .map(|rule| rule.name.to_string())
            .collect::<BTreeSet<_>>();
        let family_ident = format_ident!("StrictGeneratedParserFamily");
        let fields = self.recursive.iter().map(|rule| {
            let name = &rule.name;
            let output = &rule.output;
            quote!(#name: BoxedParser<'tokens, #output>)
        });
        let declarations = self.recursive.iter().map(|rule| {
            let name = &rule.name;
            quote!(let mut #name = Recursive::declare();)
        });
        let definitions = self.recursive.iter().filter_map(|recursive| {
            let rule = self
                .rules
                .iter()
                .find(|rule| rule.name().to_string() == recursive.name.to_string())?;
            let parser_name = format_ident!("strict_{}_parser", rule.name());
            let parser_arguments = rule
                .arguments()
                .iter()
                .map(|argument| {
                    let argument_name = argument.to_string();
                    recursive_names
                        .contains(&argument_name)
                        .then(|| quote!(#argument.clone().boxed()))
                })
                .collect::<Option<Vec<_>>>()?;
            let hidden_free_modifier = if recursive_names.contains("free_modifier") {
                let free_modifier = format_ident!("free_modifier");
                quote!(#free_modifier.clone().boxed())
            } else {
                quote!(generated_runtime::strict_empty_free_modifier_parser())
            };
            let name = &recursive.name;
            Some(quote! {
                #name.define(#parser_name(
                    #(#parser_arguments,)*
                    #hidden_free_modifier,
                ));
            })
        });
        let outputs = self.recursive.iter().map(|rule| {
            let name = &rule.name;
            quote!(#name: #name.boxed())
        });
        let root_functions = self.recursive.iter().map(|rule| {
            let root_name = &rule.name;
            let function = format_ident!("strict_generated_{}_parser", root_name);
            let output = &rule.output;
            quote! {
                #[allow(dead_code, unused_variables)]
                pub(crate) fn #function<'tokens>() -> BoxedParser<'tokens, #output> {
                    strict_generated_parser_family().#root_name
                }
            }
        });
        Some(quote! {
            #[allow(dead_code)]
            #[bityzba::invariant(true)]
            struct #family_ident<'tokens> {
                #(#fields,)*
            }

            #[allow(dead_code)]
            pub(crate) fn strict_generated_parser_family<'tokens>() -> #family_ident<'tokens> {
                #(#declarations)*
                #(#definitions)*
                #family_ident {
                    #(#outputs,)*
                }
            }

            #(#root_functions)*
        })
    }

    fn expand_partial_valid_recursive_roots(
        &self,
        recovered_module: &TokenStream2,
    ) -> Vec<TokenStream2> {
        self.recursive
            .iter()
            .filter_map(|rule| {
                let output = simple_type_ident(&rule.output)?;
                let function = format_ident!("partial_valid_generated_{}_parser", rule.name);
                let strict_function = format_ident!("strict_generated_{}_parser", rule.name);
                let recovered_output = quote!(#recovered_module::#output);
                Some(quote! {
                    #[allow(dead_code)]
                    pub(crate) fn #function<'tokens>() -> BoxedParser<'tokens, #recovered_output> {
                        #strict_function()
                            .map(#recovered_output::from_valid)
                            .boxed()
                    }
                })
            })
            .collect()
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
    Alias(AliasRule),
    Node(NodeRule),
    Product(ProductRule),
}

impl Rule {
    fn name(&self) -> &Ident {
        match self {
            Rule::Alias(rule) => &rule.name,
            Rule::Node(rule) => &rule.name,
            Rule::Product(rule) => &rule.0.name,
        }
    }

    fn expand_metadata(&self) -> TokenStream2 {
        match self {
            Rule::Alias(rule) => rule.expand_metadata(),
            Rule::Node(rule) => rule.expand_metadata("node"),
            Rule::Product(rule) => rule.0.expand_metadata("product"),
        }
    }

    fn output(&self) -> &Type {
        match self {
            Rule::Alias(rule) => &rule.output,
            Rule::Node(rule) => &rule.output,
            Rule::Product(rule) => &rule.0.output,
        }
    }

    fn arguments(&self) -> &[Ident] {
        match self {
            Rule::Alias(rule) => &rule.arguments,
            Rule::Node(rule) => &rule.arguments,
            Rule::Product(rule) => &rule.0.arguments,
        }
    }

    fn expand_strict_parser(
        &self,
        helper_outputs: &BTreeSet<String>,
        type_env: &GrammarTypeEnv,
    ) -> Option<TokenStream2> {
        match self {
            Rule::Alias(rule) => rule.expand_strict_parser(type_env),
            Rule::Node(rule) => rule.expand_strict_parser(helper_outputs, type_env),
            Rule::Product(rule) => rule.0.expand_strict_parser(helper_outputs, type_env),
        }
    }

    fn expand_partial_valid_parser(
        &self,
        helper_outputs: &BTreeSet<String>,
        type_env: &GrammarTypeEnv,
        recovered_module: &TokenStream2,
    ) -> Option<TokenStream2> {
        match self {
            Rule::Alias(rule) => rule.expand_partial_valid_parser(type_env, recovered_module),
            Rule::Node(rule) => {
                rule.expand_partial_valid_parser(helper_outputs, type_env, recovered_module)
            }
            Rule::Product(rule) => {
                rule.0
                    .expand_partial_valid_parser(helper_outputs, type_env, recovered_module)
            }
        }
    }
}

struct AliasRule {
    name: Ident,
    arguments: Vec<Ident>,
    output: Type,
    context: Option<LitStr>,
    requires: Vec<Expr>,
    parser: Expr,
}

impl AliasRule {
    fn expand_metadata(&self) -> TokenStream2 {
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
        let parser = compact_tokens(&self.parser);
        let recovery = classify_recovery_expr(&self.parser, &argument_names).expand();
        let requires = self.requires.iter().map(|require| {
            let parser = compact_tokens(require);
            let recovery = classify_recovery_expr(require, &argument_names).expand();
            quote! {
                SyntaxGrammarField {
                    kind: "require",
                    name: "",
                    parser: #parser,
                    recovery: #recovery,
                    conditions: &[],
                }
            }
        });
        quote! {
            SyntaxGrammarRule {
                kind: "alias",
                name: #name,
                arguments: &[#(#arguments),*],
                output: #output,
                context: #context,
                fields: &[
                    #(#requires,)*
                    SyntaxGrammarField {
                    kind: "alias",
                    name: "",
                    parser: #parser,
                    recovery: #recovery,
                    conditions: &[],
                    }
                ],
            }
        }
    }

    fn expand_strict_parser(&self, type_env: &GrammarTypeEnv) -> Option<TokenStream2> {
        let argument_types = self.argument_types(type_env)?;
        let argument_names = self.argument_name_set();
        let free_modifier_parser = format_ident!("__generated_free_modifier");
        let parser = strict_parser_expr_tokens(
            &self.parser,
            &argument_names,
            type_env,
            &free_modifier_parser,
        )?;
        let parser = self
            .requires
            .iter()
            .rev()
            .try_fold(parser, |parser, require| {
                let require = strict_parser_expr_tokens(
                    require,
                    &argument_names,
                    type_env,
                    &free_modifier_parser,
                )?;
                Some(quote!(#require.ignore_then(#parser)))
            })?;
        let name = format_ident!("strict_{}_parser", self.name);
        let output = &self.output;
        let argument_params = self.arguments.iter().map(|argument| {
            let ty = argument_types
                .get(&argument.to_string())
                .expect("argument types are populated from recursive declarations");
            quote!(#argument: BoxedParser<'tokens, #ty>)
        });
        let hidden_free_modifier = strict_free_modifier_param_tokens();
        let rule_name = self.name.to_string();
        let parser_body = self.context.as_ref().map_or(parser.clone(), |context| {
            let context = context.value();
            quote!(generated_runtime::syntax_context(#context, #parser))
        });
        Some(quote! {
            #[allow(dead_code, unused_variables)]
            pub(crate) fn #name<'tokens>(
                #(#argument_params,)*
                #hidden_free_modifier
            ) -> BoxedParser<'tokens, #output> {
                generated_runtime::memoized_rule(
                    #rule_name,
                    #parser_body
                )
            }
        })
    }

    fn expand_partial_valid_parser(
        &self,
        type_env: &GrammarTypeEnv,
        recovered_module: &TokenStream2,
    ) -> Option<TokenStream2> {
        let argument_types = self.argument_types(type_env)?;
        let output = simple_type_ident(&self.output)?;
        let name = format_ident!("partial_valid_{}_parser", self.name);
        let strict_name = format_ident!("strict_{}_parser", self.name);
        let recovered_output = quote!(#recovered_module::#output);
        let argument_params = self.arguments.iter().map(|argument| {
            let ty = argument_types
                .get(&argument.to_string())
                .expect("argument types are populated from recursive declarations");
            quote!(#argument: BoxedParser<'tokens, #ty>)
        });
        let parser_arguments = self.arguments.iter().map(|argument| quote!(#argument));
        let hidden_free_modifier = strict_free_modifier_param_tokens();
        Some(quote! {
            #[allow(dead_code, unused_variables)]
            pub(crate) fn #name<'tokens>(
                #(#argument_params,)*
                #hidden_free_modifier
            ) -> BoxedParser<'tokens, #recovered_output> {
                #strict_name(
                    #(#parser_arguments,)*
                    __generated_free_modifier,
                )
                .map(#recovered_output::from_valid)
                .boxed()
            }
        })
    }

    fn argument_types(&self, type_env: &GrammarTypeEnv) -> Option<BTreeMap<String, Type>> {
        let mut arguments = BTreeMap::new();
        for argument in &self.arguments {
            let ty = type_env.recursive.get(&argument.to_string())?.clone();
            arguments.insert(argument.to_string(), ty);
        }
        Some(arguments)
    }

    fn argument_name_set(&self) -> BTreeSet<String> {
        self.arguments
            .iter()
            .map(Ident::to_string)
            .collect::<BTreeSet<_>>()
    }
}

impl Parse for AliasRule {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        input.parse::<kw::alias>()?;
        let name = input.parse()?;
        let arguments = parse_optional_arguments(input)?;
        input.parse::<Token![->]>()?;
        let output = input.parse()?;
        let content;
        braced!(content in input);

        let mut context = None;
        let mut requires = Vec::new();
        let mut parser = None;
        while !content.is_empty() {
            if content.peek(kw::context) {
                content.parse::<kw::context>()?;
                context = Some(content.parse()?);
                content.parse::<Token![;]>()?;
            } else if content.peek(kw::require) {
                content.parse::<kw::require>()?;
                requires.push(content.parse()?);
                content.parse::<Token![;]>()?;
            } else {
                if parser.is_some() {
                    return Err(content.error("alias rules accept exactly one parser expression"));
                }
                parser = Some(content.parse()?);
                content.parse::<Token![;]>()?;
            }
        }
        let parser =
            parser.ok_or_else(|| input.error("alias rule requires a parser expression"))?;
        Ok(Self {
            name,
            arguments,
            output,
            context,
            requires,
            parser,
        })
    }
}

struct NodeRule {
    name: Ident,
    arguments: Vec<Ident>,
    output: Type,
    context: Option<LitStr>,
    fields: Vec<FieldItem>,
    build: Option<ExprClosure>,
    construction: ConstructionMode,
    _recovered_build: Option<ExprClosure>,
}

impl NodeRule {
    fn generated_model_fields(
        &self,
        type_env: &GrammarTypeEnv,
    ) -> Result<Vec<GeneratedFieldModel>> {
        let argument_types = self.argument_types(type_env).ok_or_else(|| {
            syn::Error::new_spanned(
                &self.name,
                "cannot infer generated model field types because a rule argument is not a declared recursive rule",
            )
        })?;
        self.fields
            .iter()
            .filter_map(|field| match field.kind {
                FieldKind::Field | FieldKind::Let | FieldKind::Default => Some(field),
                FieldKind::Scratch | FieldKind::Require => None,
            })
            .map(|field| field.generated_model_field(type_env, &argument_types))
            .collect()
    }

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

    fn expand_strict_parser(
        &self,
        helper_outputs: &BTreeSet<String>,
        type_env: &GrammarTypeEnv,
    ) -> Option<TokenStream2> {
        let argument_types = self.argument_types(type_env)?;
        let argument_names = self.argument_name_set();
        let sequence_items = self
            .fields
            .iter()
            .filter(|field| {
                matches!(
                    field.kind,
                    FieldKind::Field | FieldKind::Scratch | FieldKind::Require
                )
            })
            .collect::<Vec<_>>();
        let fields = self
            .fields
            .iter()
            .filter(|field| matches!(field.kind, FieldKind::Field))
            .collect::<Vec<_>>();
        let free_modifier_parser = format_ident!("__generated_free_modifier");
        let (parser, pattern) = strict_sequence_parser_tokens(
            &sequence_items,
            &argument_names,
            type_env,
            &free_modifier_parser,
        )?;
        let name = format_ident!("strict_{}_parser", self.name);
        let output = &self.output;
        let argument_params = self.arguments.iter().map(|argument| {
            let ty = argument_types
                .get(&argument.to_string())
                .expect("argument types are populated from recursive declarations");
            quote!(#argument: BoxedParser<'tokens, #ty>)
        });
        let hidden_free_modifier = strict_free_modifier_param_tokens();
        let body = if let Some(build) = &self.build {
            build.body.to_token_stream()
        } else if simple_type_ident(output).is_some_and(|output| {
            helper_outputs.contains(&output.to_string())
                && self.fields.iter().all(|field| {
                    !matches!(
                        field.kind,
                        FieldKind::Default | FieldKind::Let | FieldKind::Scratch
                    )
                })
        }) {
            let field_names = fields
                .iter()
                .map(|field| field.name.as_ref().expect("field items have names"));
            quote!(#output { #(#field_names,)* })
        } else if is_unit_type(output) {
            let let_bindings = self.fields.iter().filter_map(|field| {
                matches!(field.kind, FieldKind::Let).then(|| {
                    let name = field.name.as_ref().expect("let field items have names");
                    let value = &field.parser;
                    quote!(let #name = #value;)
                })
            });
            quote!({
                #(#let_bindings)*
                ()
            })
        } else if is_path_type(output) {
            let let_bindings = self.fields.iter().filter_map(|field| {
                matches!(field.kind, FieldKind::Let).then(|| {
                    let name = field.name.as_ref().expect("let field items have names");
                    let value = &field.parser;
                    quote!(let #name = #value;)
                })
            });
            let assignments = self.fields.iter().filter_map(|field| {
                let name = field.name.as_ref()?;
                match field.kind {
                    FieldKind::Field | FieldKind::Let => Some(quote!(#name,)),
                    FieldKind::Default => {
                        let value = &field.parser;
                        Some(quote!(#name: #value,))
                    }
                    FieldKind::Scratch | FieldKind::Require => None,
                }
            });
            match &self.construction {
                ConstructionMode::Validated => {
                    quote!({
                        #(#let_bindings)*
                        bityzba::new!(#output { #(#assignments)* })
                    })
                }
                ConstructionMode::Direct => {
                    quote!({
                        #(#let_bindings)*
                        #output { #(#assignments)* }
                    })
                }
                ConstructionMode::NamedVariant(variant) => {
                    quote!({
                        #(#let_bindings)*
                        bityzba::new!(#output::#variant { #(#assignments)* })
                    })
                }
                ConstructionMode::TupleVariant(variant) => {
                    let values = self.fields.iter().filter_map(|field| {
                        let name = field.name.as_ref()?;
                        match field.kind {
                            FieldKind::Field | FieldKind::Let => Some(quote!(#name)),
                            FieldKind::Default => {
                                let value = &field.parser;
                                Some(quote!(#value))
                            }
                            FieldKind::Scratch | FieldKind::Require => None,
                        }
                    });
                    quote!({
                        #(#let_bindings)*
                        bityzba::new!(#output::#variant(#(#values,)*))
                    })
                }
            }
        } else {
            return None;
        };
        let rule_name = self.name.to_string();
        let parser_body = quote!(#parser.map(|#pattern| #body));
        let parser_body = self
            .context
            .as_ref()
            .map_or(parser_body.clone(), |context| {
                let context = context.value();
                quote!(generated_runtime::syntax_context(#context, #parser_body))
            });
        Some(quote! {
            #[allow(dead_code, unused_variables)]
            pub(crate) fn #name<'tokens>(
                #(#argument_params,)*
                #hidden_free_modifier
            ) -> BoxedParser<'tokens, #output> {
                generated_runtime::memoized_rule(
                    #rule_name,
                    #parser_body
                )
            }
        })
    }

    fn expand_partial_valid_parser(
        &self,
        helper_outputs: &BTreeSet<String>,
        type_env: &GrammarTypeEnv,
        recovered_module: &TokenStream2,
    ) -> Option<TokenStream2> {
        let can_generate_strict = if self.build.is_none() {
            let has_default = self
                .fields
                .iter()
                .any(|field| matches!(field.kind, FieldKind::Default));
            let has_let = self
                .fields
                .iter()
                .any(|field| matches!(field.kind, FieldKind::Let));
            let has_scratch = self
                .fields
                .iter()
                .any(|field| matches!(field.kind, FieldKind::Scratch));
            if is_unit_type(&self.output) {
                true
            } else {
                is_path_type(&self.output)
                    && simple_type_ident(&self.output).is_none_or(|output| {
                        if helper_outputs.contains(&output.to_string()) {
                            !has_default && !has_let && !has_scratch
                        } else {
                            true
                        }
                    })
            }
        } else {
            true
        };
        if !can_generate_strict {
            return None;
        }
        let argument_types = self.argument_types(type_env)?;
        let output = simple_type_ident(&self.output)?;
        if helper_outputs.contains(&output.to_string()) {
            return None;
        }
        let name = format_ident!("partial_valid_{}_parser", self.name);
        let strict_name = format_ident!("strict_{}_parser", self.name);
        let recovered_output = quote!(#recovered_module::#output);
        let argument_params = self.arguments.iter().map(|argument| {
            let ty = argument_types
                .get(&argument.to_string())
                .expect("argument types are populated from recursive declarations");
            quote!(#argument: BoxedParser<'tokens, #ty>)
        });
        let parser_arguments = self.arguments.iter().map(|argument| quote!(#argument));
        let hidden_free_modifier = strict_free_modifier_param_tokens();
        Some(quote! {
            #[allow(dead_code, unused_variables)]
            pub(crate) fn #name<'tokens>(
                #(#argument_params,)*
                #hidden_free_modifier
            ) -> BoxedParser<'tokens, #recovered_output> {
                #strict_name(
                    #(#parser_arguments,)*
                    __generated_free_modifier,
                )
                .map(#recovered_output::from_valid)
                .boxed()
            }
        })
    }

    fn expand_product_helper(&self, type_env: &GrammarTypeEnv) -> Option<TokenStream2> {
        let output = simple_type_ident(&self.output)?;
        let argument_types = self.argument_types(type_env)?;
        let fields = self
            .fields
            .iter()
            .filter(|field| matches!(field.kind, FieldKind::Field))
            .map(|field| {
                let name = field.name.as_ref().expect("field items have names");
                let ty = parser_output_type(&field.parser, type_env, &argument_types)?;
                Some(quote!(pub #name: #ty))
            })
            .collect::<Option<Vec<_>>>()?;
        Some(quote! {
            #[derive(Debug, Clone, PartialEq, Eq)]
            #[bityzba::invariant(true)]
            pub(crate) struct #output {
                #(#fields,)*
            }
        })
    }

    fn argument_types(&self, type_env: &GrammarTypeEnv) -> Option<BTreeMap<String, Type>> {
        let mut arguments = BTreeMap::new();
        for argument in &self.arguments {
            let ty = type_env.recursive.get(&argument.to_string())?.clone();
            arguments.insert(argument.to_string(), ty);
        }
        Some(arguments)
    }

    fn argument_name_set(&self) -> BTreeSet<String> {
        self.arguments
            .iter()
            .map(Ident::to_string)
            .collect::<BTreeSet<_>>()
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

struct GrammarTypeEnv {
    recursive: BTreeMap<String, Type>,
    rules: BTreeMap<String, Type>,
    rule_arguments: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ConstructionMode {
    Validated,
    Direct,
    NamedVariant(Ident),
    TupleVariant(Ident),
}

impl GrammarTypeEnv {
    fn new(recursive: &[RecursiveRule], rules: &[Rule]) -> Self {
        Self {
            recursive: recursive
                .iter()
                .map(|rule| (rule.name.to_string(), rule.output.clone()))
                .collect(),
            rules: rules
                .iter()
                .map(|rule| (rule.name().to_string(), rule.output().clone()))
                .collect(),
            rule_arguments: rules
                .iter()
                .map(|rule| {
                    (
                        rule.name().to_string(),
                        rule.arguments()
                            .iter()
                            .map(Ident::to_string)
                            .collect::<Vec<_>>(),
                    )
                })
                .collect(),
        }
    }
}

fn strict_free_modifier_param_tokens() -> TokenStream2 {
    quote!(__generated_free_modifier: BoxedParser<'tokens, FreeModifierSyntax>,)
}

fn strict_sequence_parser_tokens(
    fields: &[&FieldItem],
    arguments: &BTreeSet<String>,
    type_env: &GrammarTypeEnv,
    free_modifier_parser: &Ident,
) -> Option<(TokenStream2, TokenStream2)> {
    let Some(first) = fields.first() else {
        return Some((quote!(generated_runtime::empty()), quote!(())));
    };
    let mut parser =
        strict_parser_expr_tokens(&first.parser, arguments, type_env, free_modifier_parser)?;
    let mut pattern = sequence_item_pattern(first);
    for field in fields.iter().skip(1) {
        let next =
            strict_parser_expr_tokens(&field.parser, arguments, type_env, free_modifier_parser)?;
        let name = sequence_item_pattern(field);
        parser = quote!(#parser.then(#next));
        pattern = quote!((#pattern, #name));
    }
    Some((parser, pattern))
}

fn sequence_item_pattern(field: &FieldItem) -> TokenStream2 {
    match field.kind {
        FieldKind::Field | FieldKind::Scratch => field
            .name
            .as_ref()
            .expect("field items have names")
            .to_token_stream(),
        FieldKind::Require => quote!(_),
        FieldKind::Let | FieldKind::Default => {
            unreachable!("let and default items are not parser sequence items")
        }
    }
}

fn strict_parser_expr_tokens(
    expr: &Expr,
    arguments: &BTreeSet<String>,
    type_env: &GrammarTypeEnv,
    free_modifier_parser: &Ident,
) -> Option<TokenStream2> {
    match expr {
        Expr::Call(call) => {
            strict_call_parser_expr_tokens(call, arguments, type_env, free_modifier_parser)
        }
        Expr::MethodCall(method) => {
            strict_method_parser_expr_tokens(method, arguments, type_env, free_modifier_parser)
        }
        Expr::Path(path) => strict_path_parser_expr_tokens(path, arguments, free_modifier_parser),
        Expr::Tuple(tuple) => {
            strict_tuple_parser_expr_tokens(tuple, arguments, type_env, free_modifier_parser)
        }
        _ => None,
    }
}

fn strict_method_parser_expr_tokens(
    method: &ExprMethodCall,
    arguments: &BTreeSet<String>,
    type_env: &GrammarTypeEnv,
    free_modifier_parser: &Ident,
) -> Option<TokenStream2> {
    if method.method == "warn" && method.args.len() == 1 {
        let inner =
            strict_parser_expr_tokens(&method.receiver, arguments, type_env, free_modifier_parser)?;
        let construct = method.args.first().and_then(path_expr_last_segment)?;
        let construct = format_ident!("{construct}");
        Some(quote! {
            #inner.map_with(
                |value, extra: &mut chumsky::input::MapExtra<'tokens, '_, ParserInput<'tokens>, ParseExtra<'tokens>>| {
                    extra.state().warn(ExperimentalConstruct::#construct, &value);
                    value
                },
            )
        })
    } else if method.method == "not_next_selmaho" && method.args.len() == 1 {
        let inner =
            strict_parser_expr_tokens(&method.receiver, arguments, type_env, free_modifier_parser)?;
        let selmaho = method.args.first().and_then(path_expr_last_segment)?;
        let selmaho = format_ident!("{selmaho}");
        Some(quote! {
            #inner
                .then(generated_runtime::not_next_selmaho(Selmaho::#selmaho))
                .map(|(value, _)| value)
        })
    } else if method.method == "not_next_token" && method.args.len() == 1 {
        let inner =
            strict_parser_expr_tokens(&method.receiver, arguments, type_env, free_modifier_parser)?;
        let predicate = method.args.first().and_then(path_expr_last_segment)?;
        let predicate = format_ident!("{predicate}");
        Some(quote! {
            #inner
                .then(generated_runtime::not_next_token(SyntaxGrammarTokenPredicate::#predicate))
                .map(|(value, _)| value)
        })
    } else if method.method == "not_next_rule" && method.args.len() == 1 {
        let inner =
            strict_parser_expr_tokens(&method.receiver, arguments, type_env, free_modifier_parser)?;
        let rule = method.args.first().and_then(path_expr_last_segment)?;
        if !type_env.rules.contains_key(&rule) {
            return None;
        }
        let parser_arguments = type_env
            .rule_arguments
            .get(&rule)
            .into_iter()
            .flatten()
            .map(|argument| {
                if arguments.contains(argument) {
                    let argument = format_ident!("{argument}");
                    Some(quote!(#argument.clone()))
                } else {
                    None
                }
            })
            .collect::<Option<Vec<_>>>()?;
        let parser_name = format_ident!("strict_{}_parser", rule);
        let expected = format!("not {rule}");
        Some(quote! {
            generated_runtime::not_next_rule_after(
                #inner,
                #parser_name(
                    #(#parser_arguments,)*
                    #free_modifier_parser.clone(),
                ),
                #expected,
            )
        })
    } else if method.method == "followed_by" && method.args.len() == 1 {
        let guard_expr = method.args.first()?;
        let inner =
            strict_parser_expr_tokens(&method.receiver, arguments, type_env, free_modifier_parser)?;
        let guard =
            strict_parser_expr_tokens(guard_expr, arguments, type_env, free_modifier_parser)?;
        Some(quote!(generated_runtime::followed_by(#inner, #guard)))
    } else if method.method == "lookahead" && method.args.is_empty() {
        let inner =
            strict_parser_expr_tokens(&method.receiver, arguments, type_env, free_modifier_parser)?;
        Some(quote!(generated_runtime::lookahead(#inner)))
    } else if method.method == "not" && method.args.is_empty() {
        let inner =
            strict_parser_expr_tokens(&method.receiver, arguments, type_env, free_modifier_parser)?;
        Some(quote!(generated_runtime::not(#inner)))
    } else if method.method == "ignored" && method.args.is_empty() {
        let inner =
            strict_parser_expr_tokens(&method.receiver, arguments, type_env, free_modifier_parser)?;
        Some(quote!(#inner.map(|_| ())))
    } else if (method.method == "wf"
        || method.method == "with_free_modifiers"
        || method.method == "prohibited_wf")
        && method.args.is_empty()
    {
        let inner =
            strict_parser_expr_tokens(&method.receiver, arguments, type_env, free_modifier_parser)?;
        let free_modifier_list = if method.method == "prohibited_wf" {
            quote!(generated_runtime::strict_cll_prohibited_free_modifier_list_parser(
                #free_modifier_parser.clone()
            ))
        } else {
            quote!(generated_runtime::strict_free_modifier_list_parser(#free_modifier_parser.clone()))
        };
        Some(quote! {
            #inner
                .then(#free_modifier_list)
                .map(|(value, free_modifiers)| WithFreeModifiers::new(value, free_modifiers))
        })
    } else {
        None
    }
}

fn strict_call_parser_expr_tokens(
    call: &ExprCall,
    arguments: &BTreeSet<String>,
    type_env: &GrammarTypeEnv,
    free_modifier_parser: &Ident,
) -> Option<TokenStream2> {
    let function = call_name(call)?;
    if type_env.rules.contains_key(&function) {
        let parser_name = format_ident!("strict_{}_parser", function);
        let parser_arguments = call
            .args
            .iter()
            .map(|argument| {
                strict_parser_expr_tokens(argument, arguments, type_env, free_modifier_parser)
            })
            .collect::<Option<Vec<_>>>()?;
        return Some(quote!(#parser_name(
            #(#parser_arguments,)*
            #free_modifier_parser.clone()
        )));
    }
    match (function.as_str(), call.args.len()) {
        ("cmavo", 1) => {
            let cmavo = call.args.first().and_then(path_expr_last_segment)?;
            let cmavo = format_ident!("{cmavo}");
            Some(quote!(cmavo(Cmavo::#cmavo)))
        }
        ("selmaho", 1) => {
            let selmaho = call.args.first().and_then(path_expr_last_segment)?;
            let selmaho = format_ident!("{selmaho}");
            Some(quote!(selmaho(Selmaho::#selmaho)))
        }
        ("word_category", 1) => {
            let category = call.args.first().and_then(path_expr_last_segment)?;
            let category = format_ident!("{category}");
            Some(quote!(generated_runtime::word_category(SyntaxWordCategory::#category)))
        }
        ("exact_word_category", 1) => {
            let category = call.args.first().and_then(path_expr_last_segment)?;
            let category = format_ident!("{category}");
            Some(quote!(generated_runtime::exact_word_category(SyntaxWordCategory::#category)))
        }
        ("quote_marker", 1) => {
            let cmavo = call.args.first().and_then(path_expr_last_segment)?;
            let cmavo = format_ident!("{cmavo}");
            Some(quote!(generated_runtime::quote_marker(Cmavo::#cmavo)))
        }
        ("delimited_quote_marker", 1) => {
            let cmavo = call.args.first().and_then(path_expr_last_segment)?;
            let cmavo = format_ident!("{cmavo}");
            Some(quote!(generated_runtime::delimited_quote_marker(Cmavo::#cmavo)))
        }
        ("raw_words_until", _) if !call.args.is_empty() => {
            let terminators = call
                .args
                .iter()
                .map(|argument| {
                    let cmavo = path_expr_last_segment(argument)?;
                    let cmavo = format_ident!("{cmavo}");
                    Some(quote!(Cmavo::#cmavo))
                })
                .collect::<Option<Vec<_>>>()?;
            Some(quote!(generated_runtime::raw_words_until(&[#(#terminators),*])))
        }
        ("feature", 2) => {
            let feature = call.args.first().and_then(path_expr_last_segment)?;
            let feature = format_ident!("{feature}");
            let inner = strict_parser_expr_tokens(
                call.args.iter().nth(1).expect("length checked"),
                arguments,
                type_env,
                free_modifier_parser,
            )?;
            Some(quote!(generated_runtime::feature_gate(
                generated_runtime::SyntaxGrammarFeature::#feature,
                #inner,
            )))
        }
        ("policy", 2) => {
            let policy = call.args.first().and_then(path_expr_last_segment)?;
            let policy = format_ident!("{policy}");
            let inner = strict_parser_expr_tokens(
                call.args.iter().nth(1).expect("length checked"),
                arguments,
                type_env,
                free_modifier_parser,
            )?;
            Some(quote!(generated_runtime::policy_gate(
                generated_runtime::SyntaxGrammarPolicyFlag::#policy,
                #inner,
            )))
        }
        ("relation_word", 0) => Some(quote!(relation_word())),
        ("tanru_unit_relation_word", 0) => {
            Some(quote!(generated_runtime::tanru_unit_relation_word()))
        }
        ("text_leading_cmevla_word", 0) => {
            Some(quote!(generated_runtime::text_leading_cmevla_word()))
        }
        ("cmevla_word" | "leading_indicator", 0) => {
            let parser = format_ident!("{function}");
            Some(quote!(#parser()))
        }
        ("pa_word", 0) => Some(quote!(pa_word())),
        ("opt", 1) => {
            let inner = strict_parser_expr_tokens(
                call.args.first().expect("length checked"),
                arguments,
                type_env,
                free_modifier_parser,
            )?;
            Some(quote!(generated_runtime::strict_optional(#inner)))
        }
        ("some", 1) => {
            let inner = strict_parser_expr_tokens(
                call.args.first().expect("length checked"),
                arguments,
                type_env,
                free_modifier_parser,
            )?;
            Some(quote!(#inner.map(Some)))
        }
        ("opt_or_default", 1) => {
            let inner = strict_parser_expr_tokens(
                call.args.first().expect("length checked"),
                arguments,
                type_env,
                free_modifier_parser,
            )?;
            Some(quote!(generated_runtime::strict_optional(#inner).map(Option::unwrap_or_default)))
        }
        ("many" | "many_local", 1) => {
            let inner = strict_parser_expr_tokens(
                call.args.first().expect("length checked"),
                arguments,
                type_env,
                free_modifier_parser,
            )?;
            Some(quote!(generated_runtime::strict_greedy_many_parser(#inner.boxed())))
        }
        ("many1" | "nonempty", 1) => {
            let inner = strict_parser_expr_tokens(
                call.args.first().expect("length checked"),
                arguments,
                type_env,
                free_modifier_parser,
            )?;
            Some(quote!(generated_runtime::strict_greedy_many1_parser(#inner.boxed())))
        }
        ("vec1", 1) => {
            let inner = strict_parser_expr_tokens(
                call.args.first().expect("length checked"),
                arguments,
                type_env,
                free_modifier_parser,
            )?;
            Some(quote! {
                generated_runtime::strict_greedy_many1_parser(#inner.boxed()).map(|items| {
                    vec1::Vec1::try_from_vec(items)
                        .expect("strict_greedy_many1_parser guarantees a non-empty vector")
                })
            })
        }
        ("singleton", 1) => {
            let inner = strict_parser_expr_tokens(
                call.args.first().expect("length checked"),
                arguments,
                type_env,
                free_modifier_parser,
            )?;
            Some(quote!(generated_runtime::singleton(#inner)))
        }
        ("prepend", 2) => {
            let head = strict_parser_expr_tokens(
                call.args.first().expect("length checked"),
                arguments,
                type_env,
                free_modifier_parser,
            )?;
            let tail = strict_parser_expr_tokens(
                call.args.iter().nth(1).expect("length checked"),
                arguments,
                type_env,
                free_modifier_parser,
            )?;
            Some(quote!(generated_runtime::prepend(#head, #tail)))
        }
        ("append", 2) => {
            let left = strict_parser_expr_tokens(
                call.args.first().expect("length checked"),
                arguments,
                type_env,
                free_modifier_parser,
            )?;
            let right = strict_parser_expr_tokens(
                call.args.iter().nth(1).expect("length checked"),
                arguments,
                type_env,
                free_modifier_parser,
            )?;
            Some(quote!(generated_runtime::append(#left, #right)))
        }
        ("concat", 2) => {
            let head = strict_parser_expr_tokens(
                call.args.first().expect("length checked"),
                arguments,
                type_env,
                free_modifier_parser,
            )?;
            let tail = strict_parser_expr_tokens(
                call.args.iter().nth(1).expect("length checked"),
                arguments,
                type_env,
                free_modifier_parser,
            )?;
            Some(quote!(generated_runtime::concat(#head, #tail)))
        }
        ("boxed", 1) => {
            let inner = strict_parser_expr_tokens(
                call.args.first().expect("length checked"),
                arguments,
                type_env,
                free_modifier_parser,
            )?;
            Some(quote!(#inner.map(Box::new)))
        }
        ("arc", 1) => {
            let inner = strict_parser_expr_tokens(
                call.args.first().expect("length checked"),
                arguments,
                type_env,
                free_modifier_parser,
            )?;
            Some(quote!(#inner.map(std::sync::Arc::new)))
        }
        ("recover_as", 2) => strict_parser_expr_tokens(
            call.args.iter().nth(1).expect("length checked"),
            arguments,
            type_env,
            free_modifier_parser,
        ),
        ("choice", 1) => {
            let alternatives = call.args.first().and_then(|expr| {
                choice_alternative_parser_tokens(expr, arguments, type_env, free_modifier_parser)
            })?;
            strict_choice_chain(alternatives)
        }
        ("choice", _) => {
            let alternatives = call
                .args
                .iter()
                .map(|expr| {
                    strict_parser_expr_tokens(expr, arguments, type_env, free_modifier_parser)
                })
                .collect::<Option<Vec<_>>>()?;
            strict_choice_chain(alternatives)
        }
        ("seq" | "sequence", _) => {
            let parts = call
                .args
                .iter()
                .map(|expr| {
                    strict_parser_expr_tokens(expr, arguments, type_env, free_modifier_parser)
                })
                .collect::<Option<Vec<_>>>()?;
            strict_sequence_expr_chain(parts)
        }
        ("empty", 0) => Some(quote!(generated_runtime::empty())),
        ("eof", 0) => Some(quote!(generated_runtime::eof())),
        _ => None,
    }
}

fn strict_path_parser_expr_tokens(
    path: &ExprPath,
    arguments: &BTreeSet<String>,
    free_modifier_parser: &Ident,
) -> Option<TokenStream2> {
    if path.qself.is_none()
        && path.path.segments.len() == 1
        && let Some(segment) = path.path.segments.first()
    {
        if arguments.contains(&segment.ident.to_string()) {
            let name = &segment.ident;
            return Some(quote!(#name.clone()));
        }
        let name = format_ident!("strict_{}_parser", segment.ident);
        Some(quote!(#name(#free_modifier_parser.clone())))
    } else {
        None
    }
}

fn strict_tuple_parser_expr_tokens(
    tuple: &ExprTuple,
    arguments: &BTreeSet<String>,
    type_env: &GrammarTypeEnv,
    free_modifier_parser: &Ident,
) -> Option<TokenStream2> {
    let parts = tuple
        .elems
        .iter()
        .map(|expr| strict_parser_expr_tokens(expr, arguments, type_env, free_modifier_parser))
        .collect::<Option<Vec<_>>>()?;
    strict_sequence_expr_chain(parts)
}

fn strict_sequence_expr_chain(mut parts: Vec<TokenStream2>) -> Option<TokenStream2> {
    if parts.is_empty() {
        return Some(quote!(generated_runtime::empty()));
    }
    let mut parser = parts.remove(0);
    for part in parts {
        parser = quote!(#parser.then(#part));
    }
    Some(parser)
}

fn choice_alternative_parser_tokens(
    expr: &Expr,
    arguments: &BTreeSet<String>,
    type_env: &GrammarTypeEnv,
    free_modifier_parser: &Ident,
) -> Option<Vec<TokenStream2>> {
    if let Expr::Tuple(ExprTuple { elems, .. }) = expr {
        elems
            .iter()
            .map(|expr| strict_parser_expr_tokens(expr, arguments, type_env, free_modifier_parser))
            .collect()
    } else {
        strict_parser_expr_tokens(expr, arguments, type_env, free_modifier_parser)
            .map(|expr| vec![expr])
    }
}

fn strict_choice_chain(mut alternatives: Vec<TokenStream2>) -> Option<TokenStream2> {
    if alternatives.is_empty() {
        return None;
    }
    if alternatives.len() == 1 {
        return alternatives.pop();
    }
    let alternatives = alternatives
        .into_iter()
        .map(|alternative| quote!(#alternative.boxed()));
    Some(quote!(generated_runtime::strict_ordered_choice_parsers(
        vec![
            #(#alternatives),*
        ]
    )))
}

fn parser_output_type(
    expr: &Expr,
    type_env: &GrammarTypeEnv,
    arguments: &BTreeMap<String, Type>,
) -> Option<TokenStream2> {
    match expr {
        Expr::Call(call) => call_parser_output_type(call, type_env, arguments),
        Expr::MethodCall(method) => method_parser_output_type(method, type_env, arguments),
        Expr::Path(path) => path_parser_output_type(path, type_env, arguments),
        Expr::Tuple(tuple) => tuple_parser_output_type(tuple, type_env, arguments),
        _ => None,
    }
}

fn method_parser_output_type(
    method: &ExprMethodCall,
    type_env: &GrammarTypeEnv,
    arguments: &BTreeMap<String, Type>,
) -> Option<TokenStream2> {
    if method.method == "warn"
        || method.method == "not_next_selmaho"
        || method.method == "not_next_token"
        || method.method == "not_next_rule"
        || method.method == "followed_by"
        || method.method == "lookahead"
    {
        parser_output_type(&method.receiver, type_env, arguments)
    } else if method.method == "not" || method.method == "ignored" {
        Some(quote!(()))
    } else if (method.method == "wf"
        || method.method == "with_free_modifiers"
        || method.method == "prohibited_wf")
        && method.args.is_empty()
    {
        let inner = parser_output_type(&method.receiver, type_env, arguments)?;
        Some(quote!(WithFreeModifiers<#inner>))
    } else {
        None
    }
}

fn call_parser_output_type(
    call: &ExprCall,
    type_env: &GrammarTypeEnv,
    arguments: &BTreeMap<String, Type>,
) -> Option<TokenStream2> {
    let function = call_name(call)?;
    if let Some(ty) = type_env.rules.get(&function) {
        return Some(quote!(#ty));
    }
    match (function.as_str(), call.args.len()) {
        (
            "cmavo"
            | "selmaho"
            | "word_category"
            | "exact_word_category"
            | "quote_marker"
            | "delimited_quote_marker",
            1,
        )
        | (
            "relation_word"
            | "tanru_unit_relation_word"
            | "cmevla_word"
            | "text_leading_cmevla_word"
            | "pa_word",
            0,
        ) => Some(quote!(Token)),
        ("raw_words_until", _) if !call.args.is_empty() => Some(quote!(Vec<Token>)),
        ("leading_indicator", 0) => Some(quote!(Indicator)),
        ("opt", 1) => {
            let inner = parser_output_type(
                call.args.first().expect("length checked"),
                type_env,
                arguments,
            )?;
            Some(quote!(Option<#inner>))
        }
        ("some", 1) => {
            let inner = parser_output_type(
                call.args.first().expect("length checked"),
                type_env,
                arguments,
            )?;
            Some(quote!(Option<#inner>))
        }
        ("opt_or_default", 1) => parser_output_type(
            call.args.first().expect("length checked"),
            type_env,
            arguments,
        ),
        ("many" | "many1" | "many_local", 1) => {
            let inner = parser_output_type(
                call.args.first().expect("length checked"),
                type_env,
                arguments,
            )?;
            Some(quote!(Vec<#inner>))
        }
        ("vec1", 1) => {
            let inner = parser_output_type(
                call.args.first().expect("length checked"),
                type_env,
                arguments,
            )?;
            Some(quote!(vec1::Vec1<#inner>))
        }
        ("singleton", 1) => {
            let inner = parser_output_type(
                call.args.first().expect("length checked"),
                type_env,
                arguments,
            )?;
            Some(quote!(Vec<#inner>))
        }
        ("prepend", 2) => {
            let head = parser_output_type(
                call.args.first().expect("length checked"),
                type_env,
                arguments,
            )?;
            Some(quote!(Vec<#head>))
        }
        ("append", 2) => parser_output_type(
            call.args.first().expect("length checked"),
            type_env,
            arguments,
        ),
        ("concat", 2) => parser_output_type(
            call.args.first().expect("length checked"),
            type_env,
            arguments,
        ),
        ("boxed", 1) => {
            let inner = parser_output_type(
                call.args.first().expect("length checked"),
                type_env,
                arguments,
            )?;
            Some(quote!(Box<#inner>))
        }
        ("arc", 1) => {
            let inner = parser_output_type(
                call.args.first().expect("length checked"),
                type_env,
                arguments,
            )?;
            Some(quote!(std::sync::Arc<#inner>))
        }
        ("recover_as", 2) => parser_output_type(
            call.args.iter().nth(1).expect("length checked"),
            type_env,
            arguments,
        ),
        ("feature" | "policy", 2) => parser_output_type(
            call.args.iter().nth(1).expect("length checked"),
            type_env,
            arguments,
        ),
        ("nonempty", 1) => parser_output_type(
            call.args.first().expect("length checked"),
            type_env,
            arguments,
        ),
        ("choice", 1) => choice_output_type(
            call.args.first().expect("length checked"),
            type_env,
            arguments,
        ),
        ("choice", _) => choice_outputs_same(call.args.iter(), type_env, arguments),
        ("seq" | "sequence", _) => sequence_output_type(call.args.iter(), type_env, arguments),
        ("empty" | "eof", 0) => Some(quote!(())),
        _ => None,
    }
}

fn choice_output_type(
    expr: &Expr,
    type_env: &GrammarTypeEnv,
    arguments: &BTreeMap<String, Type>,
) -> Option<TokenStream2> {
    if let Expr::Tuple(ExprTuple { elems, .. }) = expr {
        choice_outputs_same(elems.iter(), type_env, arguments)
    } else {
        parser_output_type(expr, type_env, arguments)
    }
}

fn choice_outputs_same<'a>(
    exprs: impl Iterator<Item = &'a Expr>,
    type_env: &GrammarTypeEnv,
    arguments: &BTreeMap<String, Type>,
) -> Option<TokenStream2> {
    let mut outputs = exprs.map(|expr| parser_output_type(expr, type_env, arguments));
    let first = outputs.next()??;
    if outputs.all(|output| {
        output
            .as_ref()
            .is_some_and(|output| output.to_string() == first.to_string())
    }) {
        Some(first)
    } else {
        None
    }
}

fn sequence_output_type<'a>(
    exprs: impl Iterator<Item = &'a Expr>,
    type_env: &GrammarTypeEnv,
    arguments: &BTreeMap<String, Type>,
) -> Option<TokenStream2> {
    let mut outputs = exprs
        .map(|expr| parser_output_type(expr, type_env, arguments))
        .collect::<Option<Vec<_>>>()?;
    if outputs.is_empty() {
        return Some(quote!(()));
    }
    let mut output = outputs.remove(0);
    for next in outputs {
        output = quote!((#output, #next));
    }
    Some(output)
}

fn tuple_parser_output_type(
    tuple: &ExprTuple,
    type_env: &GrammarTypeEnv,
    arguments: &BTreeMap<String, Type>,
) -> Option<TokenStream2> {
    sequence_output_type(tuple.elems.iter(), type_env, arguments)
}

fn path_parser_output_type(
    path: &ExprPath,
    type_env: &GrammarTypeEnv,
    arguments: &BTreeMap<String, Type>,
) -> Option<TokenStream2> {
    if path.qself.is_none()
        && path.path.segments.len() == 1
        && let Some(segment) = path.path.segments.first()
    {
        let name = segment.ident.to_string();
        if let Some(ty) = arguments.get(&name).or_else(|| type_env.rules.get(&name)) {
            return Some(quote!(#ty));
        }
    }
    None
}

fn simple_type_ident(output: &Type) -> Option<&Ident> {
    let Type::Path(path) = output else {
        return None;
    };
    if path.qself.is_some() || path.path.segments.len() != 1 {
        return None;
    }
    Some(&path.path.segments.first()?.ident)
}

fn is_path_type(output: &Type) -> bool {
    matches!(output, Type::Path(_))
}

fn is_unit_type(output: &Type) -> bool {
    matches!(output, Type::Tuple(tuple) if tuple.elems.is_empty())
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
    let mut build = None;
    let mut construction = ConstructionMode::Validated;
    let mut recovered_build = None;
    while !content.is_empty() {
        if content.peek(kw::context) {
            content.parse::<kw::context>()?;
            context = Some(content.parse()?);
            content.parse::<Token![;]>()?;
        } else if content.peek(kw::construct) {
            content.parse::<kw::construct>()?;
            if content.peek(kw::direct) {
                content.parse::<kw::direct>()?;
                construction = ConstructionMode::Direct;
            } else if content.peek(kw::variant) {
                content.parse::<kw::variant>()?;
                construction = ConstructionMode::NamedVariant(content.parse()?);
            } else if content.peek(kw::tuple_variant) {
                content.parse::<kw::tuple_variant>()?;
                construction = ConstructionMode::TupleVariant(content.parse()?);
            } else {
                return Err(content
                    .error("expected `direct`, `variant`, or `tuple_variant` construction mode"));
            }
            content.parse::<Token![;]>()?;
        } else if content.peek(kw::fields) {
            fields = parse_fields_block(&content)?;
        } else if content.peek(kw::build) {
            content.parse::<kw::build>()?;
            build = Some(content.parse()?);
            content.parse::<Token![;]>()?;
        } else if content.peek(kw::recovered_build) {
            content.parse::<kw::recovered_build>()?;
            recovered_build = Some(content.parse()?);
            content.parse::<Token![;]>()?;
        } else {
            return Err(content.error(
                "expected `context`, `construct`, `fields`, `build`, or `recovered_build`",
            ));
        }
    }

    Ok(NodeRule {
        name,
        arguments,
        output,
        context,
        fields,
        build,
        construction,
        _recovered_build: recovered_build,
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
    attrs: Vec<Attribute>,
    conditions: Vec<Condition>,
    kind: FieldKind,
    name: Option<Ident>,
    ty: Option<Type>,
    parser: Expr,
}

impl FieldItem {
    fn expand(&self, arguments: &BTreeSet<String>) -> TokenStream2 {
        let kind = self.kind.as_str();
        let name = self
            .name
            .as_ref()
            .map_or_else(String::new, Ident::to_string);
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

    fn generated_model_field(
        &self,
        type_env: &GrammarTypeEnv,
        argument_types: &BTreeMap<String, Type>,
    ) -> Result<GeneratedFieldModel> {
        if !self.conditions.is_empty() {
            return Err(syn::Error::new_spanned(
                self.name
                    .as_ref()
                    .map_or_else(|| self.parser.to_token_stream(), Ident::to_token_stream),
                "conditional generated model fields need explicit model support before they can be emitted",
            ));
        }
        let name = self
            .name
            .clone()
            .ok_or_else(|| syn::Error::new_spanned(&self.parser, "model fields need a name"))?;
        let ty = match (&self.ty, &self.kind) {
            (Some(ty), _) => quote!(#ty),
            (None, FieldKind::Field) => {
                parser_output_type(&self.parser, type_env, argument_types).ok_or_else(|| {
                    syn::Error::new_spanned(
                        &self.parser,
                        "cannot infer generated model field type from parser expression; add an explicit `: Type` annotation",
                    )
                })?
            }
            (None, FieldKind::Let | FieldKind::Default) => {
                return Err(syn::Error::new_spanned(
                    &self.parser,
                    "computed/default generated model fields require an explicit `: Type` annotation",
                ));
            }
            (None, FieldKind::Scratch | FieldKind::Require) => {
                unreachable!("parser-only fields are filtered before model field generation")
            }
        };
        Ok(GeneratedFieldModel {
            attrs: self.attrs.clone(),
            name,
            ty,
        })
    }
}

impl Parse for FieldItem {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let attrs = input.call(Attribute::parse_outer)?;
        let mut conditions = Vec::new();
        while input.peek(kw::when) {
            conditions.push(input.parse()?);
        }

        let (kind, name) = if input.peek(kw::field) {
            input.parse::<kw::field>()?;
            (FieldKind::Field, Some(input.parse()?))
        } else if input.peek(Token![let]) {
            input.parse::<Token![let]>()?;
            (FieldKind::Let, Some(input.parse()?))
        } else if input.peek(kw::default) {
            input.parse::<kw::default>()?;
            (FieldKind::Default, Some(input.parse()?))
        } else if input.peek(kw::scratch) {
            input.parse::<kw::scratch>()?;
            (FieldKind::Scratch, Some(input.parse()?))
        } else if input.peek(kw::require) {
            input.parse::<kw::require>()?;
            (FieldKind::Require, None)
        } else {
            return Err(input.error("expected `field`, `scratch`, `let`, `default`, or `require`"));
        };
        let ty = if !matches!(kind, FieldKind::Require) && input.peek(Token![:]) {
            input.parse::<Token![:]>()?;
            Some(input.parse()?)
        } else {
            None
        };
        if !matches!(kind, FieldKind::Require) {
            input.parse::<Token![=]>()?;
        }
        let parser = input.parse()?;
        input.parse::<Token![;]>()?;
        Ok(Self {
            attrs,
            conditions,
            kind,
            name,
            ty,
            parser,
        })
    }
}

enum FieldKind {
    Field,
    Scratch,
    Let,
    Default,
    Require,
}

impl FieldKind {
    fn as_str(&self) -> &'static str {
        match self {
            FieldKind::Field => "field",
            FieldKind::Scratch => "scratch",
            FieldKind::Let => "let",
            FieldKind::Default => "default",
            FieldKind::Require => "require",
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
    Some(Box<RecoveryExpr>),
    Boxed(Box<RecoveryExpr>),
    Arc(Box<RecoveryExpr>),
    WithFreeModifiers(Box<RecoveryExpr>),
    PayloadStart(Box<RecoveryExpr>),
    Ignored(Box<RecoveryExpr>),
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
            RecoveryExpr::Some(inner) => {
                let inner = inner.expand();
                quote!(SyntaxGrammarRecoveryExpr::Some(&#inner))
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
            RecoveryExpr::Ignored(inner) => {
                let inner = inner.expand();
                quote!(SyntaxGrammarRecoveryExpr::Ignored(&#inner))
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
        ("ignored", 0) => RecoveryExpr::Ignored(inner()),
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
        ("quote_marker" | "delimited_quote_marker", 1) => call
            .args
            .first()
            .and_then(path_expr_last_segment)
            .map(RecoveryExpr::Cmavo)
            .unwrap_or_else(|| RecoveryExpr::Opaque(compact_tokens(call))),
        ("opt", 1) => RecoveryExpr::Opt(Box::new(classify_recovery_expr(&call.args[0], arguments))),
        ("some", 1) => {
            RecoveryExpr::Some(Box::new(classify_recovery_expr(&call.args[0], arguments)))
        }
        ("opt_or_default", 1) => {
            RecoveryExpr::Opt(Box::new(classify_recovery_expr(&call.args[0], arguments)))
        }
        ("many" | "many_local", 1) => {
            RecoveryExpr::Many(Box::new(classify_recovery_expr(&call.args[0], arguments)))
        }
        ("many1", 1) | ("nonempty", 1) => {
            RecoveryExpr::Many1(Box::new(classify_recovery_expr(&call.args[0], arguments)))
        }
        ("vec1", 1) => {
            RecoveryExpr::Many1(Box::new(classify_recovery_expr(&call.args[0], arguments)))
        }
        ("singleton", 1) => {
            RecoveryExpr::Some(Box::new(classify_recovery_expr(&call.args[0], arguments)))
        }
        ("prepend" | "append" | "concat", 2) => RecoveryExpr::Sequence(
            call.args
                .iter()
                .map(|expr| classify_recovery_expr(expr, arguments))
                .collect(),
        ),
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
