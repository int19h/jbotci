//! Proc macros for syntax grammar declarations.

use std::collections::{BTreeMap, BTreeSet};

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{ToTokens, format_ident, quote};
use syn::{
    Attribute, Expr, ExprArray, ExprCall, ExprMethodCall, ExprPath, ExprTuple, GenericArgument,
    Ident, LitStr, Path, PathArguments, Result, Token, Type, braced, bracketed, parenthesized,
    parse::{Parse, ParseStream},
    parse_macro_input, parse_quote,
};

#[proc_macro]
pub fn syntax_grammar(input: TokenStream) -> TokenStream {
    let grammar = parse_macro_input!(input as SyntaxGrammar);
    grammar.expand().into()
}

mod kw {
    syn::custom_keyword!(alias);
    syn::custom_keyword!(assert);
    syn::custom_keyword!(env);
    syn::custom_keyword!(feature);
    syn::custom_keyword!(field);
    syn::custom_keyword!(model);
    syn::custom_keyword!(model_path);
    syn::custom_keyword!(policy);
    syn::custom_keyword!(recovered);
    syn::custom_keyword!(recursive);
    syn::custom_keyword!(rule);
    syn::custom_keyword!(parsers);
    syn::custom_keyword!(strict_parsers);
    syn::custom_keyword!(tree_model);
    syn::custom_keyword!(when);
    syn::custom_keyword!(chain);
    syn::custom_keyword!(one_or_more);
    syn::custom_keyword!(zero_or_more);
}

struct SyntaxGrammar {
    tree_model: Option<syn::File>,
    generate_model: bool,
    model_outputs: Option<BTreeSet<String>>,
    model_path: Option<Path>,
    env: Option<Type>,
    recovered_module: Option<Path>,
    generate_parsers: bool,
    generate_partial_valid_parsers: bool,
    recursive: Vec<RecursiveRule>,
    rules: Vec<Rule>,
}

impl SyntaxGrammar {
    fn expand(&self) -> TokenStream2 {
        let type_env = GrammarTypeEnv::new(&self.recursive, &self.rules);
        let model_outputs = self.resolved_model_outputs();
        let model_all_rules_local = self.generate_model && self.model_outputs.is_none();
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
        let recursive = self.recursive.iter().map(RecursiveRule::expand);
        let rules = match self
            .rules
            .iter()
            .map(|rule| rule.expand_metadata(&type_env))
            .collect::<Result<Vec<_>>>()
        {
            Ok(rules) => rules,
            Err(error) => return error.into_compile_error(),
        };
        let rule_lookup_arms = self.rules.iter().enumerate().map(|(index, rule)| {
            let name = rule.name().to_string();
            quote!(#name => Some(&SYNTAX_GRAMMAR_RULES[#index]))
        });
        let parser_functions = if self.generate_parsers {
            self.rules
                .iter()
                .filter(|rule| {
                    !self.generate_model
                        || rule
                            .output(&type_env)
                            .is_some_and(|output| self.rule_has_local_parser(output))
                })
                .filter_map(|rule| {
                    rule.expand_strict_parser(
                        &type_env,
                        self.generate_model,
                        &model_outputs,
                        model_all_rules_local,
                        self.model_path.as_ref(),
                        rule.output(&type_env)
                            .is_some_and(|output| self.generates_model_output(output)),
                    )
                })
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        let partial_valid_parser_functions = if self.generate_partial_valid_parsers {
            self.rules
                .iter()
                .filter_map(|rule| rule.expand_partial_valid_parser(&type_env, &recovered_module))
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        let recursive_family = if self.generate_parsers {
            self.expand_strict_recursive_family()
        } else {
            None
        };
        let recursive_partial_valid = if self.generate_partial_valid_parsers {
            self.expand_partial_valid_recursive_roots(&recovered_module)
        } else {
            Vec::new()
        };

        quote! {
            #tree_model

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

        let (generate_model, model_outputs) = if input.peek(kw::model) {
            input.parse::<kw::model>()?;
            let model_outputs = if input.peek(syn::token::Brace) {
                Some(parse_model_output_filter(input)?)
            } else {
                None
            };
            input.parse::<Token![;]>()?;
            (true, model_outputs)
        } else {
            (false, None)
        };

        let model_path = if input.peek(kw::model_path) {
            input.parse::<kw::model_path>()?;
            let path = input.parse()?;
            input.parse::<Token![;]>()?;
            Some(path)
        } else {
            None
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

        let (generate_parsers, generate_partial_valid_parsers) =
            if env.is_some() && input.peek(kw::parsers) {
                input.parse::<kw::parsers>()?;
                input.parse::<Token![;]>()?;
                (true, true)
            } else if env.is_some() && input.peek(kw::strict_parsers) {
                input.parse::<kw::strict_parsers>()?;
                input.parse::<Token![;]>()?;
                (true, false)
            } else {
                (false, false)
            };

        let mut recursive = Vec::new();
        let mut rules = Vec::new();
        while !input.is_empty() {
            if input.peek(kw::recursive) {
                if !recursive.is_empty() {
                    return Err(input.error("duplicate `recursive` block"));
                }
                recursive = parse_recursive_block(input)?;
            } else if input.peek(kw::alias) {
                rules.push(Rule::Alias(input.parse()?));
            } else if input.peek(kw::rule) {
                rules.push(parse_explicit_rule(input)?);
            } else {
                return Err(input.error("expected `recursive`, `alias`, or `rule`"));
            }
        }
        validate_unique_recursive_rules(&recursive)?;
        validate_unique_rules(&rules)?;

        Ok(Self {
            tree_model,
            generate_model,
            model_outputs,
            model_path,
            env,
            recovered_module,
            generate_parsers,
            generate_partial_valid_parsers,
            recursive,
            rules,
        })
    }
}

fn validate_unique_recursive_rules(rules: &[RecursiveRule]) -> Result<()> {
    let mut names = BTreeSet::new();
    for rule in rules {
        if !names.insert(rule.name.to_string()) {
            return Err(syn::Error::new_spanned(
                &rule.name,
                "duplicate recursive rule declaration",
            ));
        }
    }
    Ok(())
}

fn validate_unique_rules(rules: &[Rule]) -> Result<()> {
    let mut names = BTreeSet::new();
    for rule in rules {
        let name = rule.name();
        if !names.insert(name.to_string()) {
            return Err(syn::Error::new_spanned(
                name,
                "duplicate grammar rule declaration",
            ));
        }
    }
    Ok(())
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

fn parse_model_output_filter(input: ParseStream<'_>) -> Result<BTreeSet<String>> {
    let content;
    braced!(content in input);
    let mut outputs = BTreeSet::new();
    while !content.is_empty() {
        let ident: Ident = content.parse()?;
        outputs.insert(ident.to_string());
        if content.peek(Token![,]) {
            content.parse::<Token![,]>()?;
        } else if !content.is_empty() {
            return Err(content.error("expected `,` between generated model output names"));
        }
    }
    if outputs.is_empty() {
        return Err(content.error("generated model output filter cannot be empty"));
    }
    Ok(outputs)
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
    fn resolved_model_outputs(&self) -> Option<BTreeSet<String>> {
        if !self.generate_model {
            return None;
        }
        if let Some(outputs) = &self.model_outputs {
            return Some(outputs.clone());
        }
        let outputs = self
            .rules
            .iter()
            .filter_map(|rule| match rule {
                Rule::Alias(_) => None,
                Rule::Struct(rule) => simple_type_ident(&rule.output),
                Rule::Enum(rule) => simple_type_ident(&rule.output),
            })
            .map(Ident::to_string)
            .collect::<BTreeSet<_>>();
        Some(outputs)
    }

    fn generates_model_output(&self, output: &Type) -> bool {
        output_is_generated_model(self.generate_model, &self.resolved_model_outputs(), output)
    }

    fn generates_model_output_name(&self, output: &str) -> bool {
        self.generate_model
            && self
                .resolved_model_outputs()
                .as_ref()
                .is_none_or(|outputs| outputs.contains(output))
    }

    fn rule_has_local_parser(&self, output: &Type) -> bool {
        !self.generate_model || self.model_outputs.is_none() || self.generates_model_output(output)
    }

    fn parser_type_tokens(&self, output: &Type) -> TokenStream2 {
        parser_type_tokens(
            output,
            self.generate_model,
            &self.resolved_model_outputs(),
            self.model_path.as_ref(),
        )
    }

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
        let generated_model = self.generated_tree_model_items(type_env)?;
        let generated_items = generated_model.tree_items;
        let support_items = generated_model.support_items;
        Ok(quote! {
            jbotci_tree::tree_model! {
                #(#attrs)*
                #(#manual_items)*
                #(#generated_items)*
            }
            #(#support_items)*
        })
    }

    fn generated_tree_model_items(&self, type_env: &GrammarTypeEnv) -> Result<GeneratedTreeModel> {
        let mut structs = BTreeMap::<String, GeneratedStructModel>::new();
        let mut enums = BTreeMap::<String, Vec<GeneratedVariantModel>>::new();
        let mut transparent_constructors = BTreeSet::<String>::new();
        let mut transparent_field_pairs = BTreeSet::<(String, String)>::new();
        let mut chain_link_element_fields = BTreeSet::<(String, String)>::new();
        let mut variant_struct_outputs = BTreeSet::<(String, String)>::new();
        for rule in &self.rules {
            collect_chain_link_element_fields_for_rule(
                rule,
                type_env,
                &mut chain_link_element_fields,
            )?;
            let rule = match rule {
                Rule::Alias(_) => continue,
                Rule::Struct(rule) => rule,
                Rule::Enum(rule) => {
                    let Some(output) = simple_type_ident(&rule.output) else {
                        continue;
                    };
                    if !self.generates_model_output_name(&output.to_string()) {
                        continue;
                    }
                    let enum_constructor = generated_constructor_name(&output);
                    transparent_constructors.insert(enum_constructor.clone());
                    for branch in &rule.branches {
                        let branch_name = branch.name.to_string();
                        let Some(branch_output) = type_env
                            .rules
                            .get(&branch_name)
                            .or_else(|| type_env.recursive.get(&branch_name))
                        else {
                            return Err(syn::Error::new_spanned(
                                &branch.name,
                                "enum branches must reference a known type-producing rule or recursive parser argument",
                            ));
                        };
                        let variant = enum_variant_ident_for_output(branch_output, &branch.name);
                        let variant_constructor = variant.to_string();
                        transparent_constructors.insert(variant_constructor.clone());
                        if let Some(branch_output_ident) = simple_type_ident(branch_output) {
                            variant_struct_outputs.insert((
                                variant_constructor.clone(),
                                branch_output_ident.to_string(),
                            ));
                        }
                        transparent_field_pairs
                            .insert((enum_constructor.clone(), branch_name.clone()));
                        transparent_field_pairs
                            .insert((enum_constructor.clone(), snake_case(&enum_constructor)));
                        transparent_field_pairs
                            .insert((variant_constructor.clone(), branch_name.clone()));
                        transparent_field_pairs.insert((
                            variant_constructor.clone(),
                            snake_case(&variant_constructor),
                        ));
                        let field = GeneratedFieldModel {
                            attrs: branch.attrs.clone(),
                            name: branch.name.clone(),
                            ty: quote!(#branch_output),
                        };
                        push_generated_variant(
                            &mut enums,
                            output.to_string(),
                            GeneratedVariantModel {
                                variant,
                                rule_name: rule.name.clone(),
                                fields: vec![field],
                                tuple: true,
                            },
                        )?;
                    }
                    continue;
                }
            };
            let Some(output) = simple_type_ident(&rule.output) else {
                continue;
            };
            if !self.generates_model_output_name(&output.to_string()) {
                continue;
            }
            let key = output.to_string();
            let fields = rule.generated_model_fields(type_env)?;
            if fields.len() == 1 {
                let constructor = generated_constructor_name(&output);
                transparent_constructors.insert(constructor.clone());
                let field_name = fields[0].name.to_string();
                transparent_field_pairs.insert((constructor.clone(), field_name));
                transparent_field_pairs.insert((constructor.clone(), snake_case(&constructor)));
            }
            if let Some(existing) = structs.get(&key) {
                return Err(syn::Error::new_spanned(
                    &rule.name,
                    format!(
                        "cannot generate one struct `{key}` from both `{}` and `{}`; generated model ownership must be one rule per struct",
                        existing.rule_name, rule.name
                    ),
                ));
            }
            structs.insert(
                key,
                GeneratedStructModel {
                    visibility: quote!(pub),
                    ident: output.clone(),
                    rule_name: rule.name.clone(),
                    fields,
                },
            );
        }
        for output in enums.keys() {
            if structs.contains_key(output) {
                return Err(syn::Error::new_spanned(
                    format_ident!("{output}"),
                    format!(
                        "cannot generate `{output}` as both a struct and an enum; use an explicit enum rule for alternatives"
                    ),
                ));
            }
        }

        let mut items = Vec::new();
        items.extend(structs.values().map(GeneratedStructModel::expand));
        items.extend(enums.iter().map(|(name, variants)| {
            let ident = format_ident!("{name}");
            let invariants = variants.iter().map(GeneratedVariantModel::invariant_attr);
            quote! {
                #[bityzba::invariant(true)]
                #(#invariants)*
                #[derive(Debug, Clone, PartialEq, Eq, ::serde::Serialize)]
                pub enum #ident {
                    #(#variants,)*
                }
            }
        }));
        let transparent_constructors = transparent_constructors.iter();
        let transparent_field_pairs = transparent_field_pairs
            .iter()
            .map(|(constructor, field)| quote!((#constructor, #field)));
        let chain_link_element_fields = chain_link_element_fields
            .iter()
            .map(|(constructor, field)| quote!((#constructor, #field)));
        let struct_field_order_items =
            structs
                .values()
                .filter(|model| model.fields.len() > 1)
                .map(|model| {
                    let constructor = generated_constructor_name(&model.ident);
                    let fields = model.fields.iter().map(|field| field.name.to_string());
                    quote!(GeneratedModelFieldOrder {
                        constructor: #constructor,
                        fields: &[#(#fields,)*],
                    })
                });
        let variant_field_order_items =
            variant_struct_outputs
                .iter()
                .filter_map(|(constructor, output)| {
                    let model = structs.get(output)?;
                    if model.fields.len() <= 1 {
                        return None;
                    }
                    let fields = model.fields.iter().map(|field| field.name.to_string());
                    Some(quote!(GeneratedModelFieldOrder {
                        constructor: #constructor,
                        fields: &[#(#fields,)*],
                    }))
                });
        let field_order_items = struct_field_order_items.chain(variant_field_order_items);
        let support_items = vec![quote! {
            #[derive(Debug, Clone, Copy, PartialEq, Eq)]
            pub struct GeneratedModelFieldOrder {
                pub constructor: &'static str,
                pub fields: &'static [&'static str],
            }

            #[doc(hidden)]
            pub const GENERATED_MODEL_TRANSPARENT_TREE_CONSTRUCTORS: &[&str] = &[
                #(#transparent_constructors,)*
            ];

            #[doc(hidden)]
            pub const GENERATED_MODEL_TRANSPARENT_TREE_FIELD_PAIRS: &[(&str, &str)] = &[
                #(#transparent_field_pairs,)*
            ];

            #[doc(hidden)]
            pub const GENERATED_MODEL_CHAIN_LINK_TREE_ELEMENT_FIELDS: &[(&str, &str)] = &[
                #(#chain_link_element_fields,)*
            ];

            #[doc(hidden)]
            pub const GENERATED_MODEL_FIELD_ORDERS: &[GeneratedModelFieldOrder] = &[
                #(#field_order_items,)*
            ];
        }];
        Ok(GeneratedTreeModel {
            tree_items: items,
            support_items,
        })
    }
}

struct GeneratedTreeModel {
    tree_items: Vec<TokenStream2>,
    support_items: Vec<TokenStream2>,
}

fn collect_chain_link_element_fields_for_rule(
    rule: &Rule,
    type_env: &GrammarTypeEnv,
    fields: &mut BTreeSet<(String, String)>,
) -> Result<()> {
    let Some(argument_types) = rule.argument_types(type_env) else {
        return Ok(());
    };
    match rule {
        Rule::Alias(rule) => collect_chain_link_element_fields_for_parser_expr(
            &rule.parser,
            type_env,
            &argument_types,
            fields,
        ),
        Rule::Struct(rule) => {
            for field in &rule.fields {
                collect_chain_link_element_fields_for_parser_expr(
                    &field.parser,
                    type_env,
                    &argument_types,
                    fields,
                )?;
            }
            Ok(())
        }
        Rule::Enum(_) => Ok(()),
    }
}

fn collect_chain_link_element_fields_for_parser_expr(
    expr: &ParserExpr,
    type_env: &GrammarTypeEnv,
    argument_types: &BTreeMap<String, Type>,
    fields: &mut BTreeSet<(String, String)>,
) -> Result<()> {
    match expr {
        ParserExpr::Rust(expr) => {
            if let Expr::Array(array) = expr
                && let Some(vector) = array_vector_expr(array)
            {
                collect_chain_link_element_fields_for_parser_expr(
                    &ParserExpr::Vector(vector),
                    type_env,
                    argument_types,
                    fields,
                )?;
            }
        }
        ParserExpr::Vector(expr) => {
            for item in &expr.items {
                collect_chain_link_element_fields_for_vector_item(
                    item,
                    type_env,
                    argument_types,
                    fields,
                )?;
            }
        }
        ParserExpr::Chain(expr) => {
            let link =
                parser_output_type(&expr.links, type_env, argument_types).ok_or_else(|| {
                    syn::Error::new_spanned(
                        expr.to_token_stream(),
                        "cannot infer chain link output type for generated Tree metadata",
                    )
                })?;
            let link_ty = syn::parse2::<Type>(link).map_err(|error| {
                syn::Error::new_spanned(
                    expr.to_token_stream(),
                    format!("cannot parse chain link output type: {error}"),
                )
            })?;
            let link_ident = simple_type_ident(&link_ty).ok_or_else(|| {
                syn::Error::new_spanned(
                    expr.to_token_stream(),
                    "chain link parser must produce a generated struct type",
                )
            })?;
            fields.insert((
                generated_constructor_name(link_ident),
                expr.element.to_string(),
            ));
            collect_chain_link_element_fields_for_parser_expr(
                &expr.first,
                type_env,
                argument_types,
                fields,
            )?;
            collect_chain_link_element_fields_for_parser_expr(
                &expr.links,
                type_env,
                argument_types,
                fields,
            )?;
        }
        ParserExpr::Postfix { receiver, .. } => {
            collect_chain_link_element_fields_for_parser_expr(
                receiver,
                type_env,
                argument_types,
                fields,
            )?;
        }
    }
    Ok(())
}

fn collect_chain_link_element_fields_for_vector_item(
    item: &VectorItem,
    type_env: &GrammarTypeEnv,
    argument_types: &BTreeMap<String, Type>,
    fields: &mut BTreeSet<(String, String)>,
) -> Result<()> {
    let parser = match item {
        VectorItem::One(parser)
        | VectorItem::Spread(parser)
        | VectorItem::ZeroOrMore(parser)
        | VectorItem::ZeroOrMoreSpread(parser)
        | VectorItem::OneOrMore(parser)
        | VectorItem::OneOrMoreSpread(parser)
        | VectorItem::Assert { parser, .. } => parser,
    };
    collect_chain_link_element_fields_for_parser_expr(parser, type_env, argument_types, fields)
}

fn push_generated_variant(
    enums: &mut BTreeMap<String, Vec<GeneratedVariantModel>>,
    output: String,
    variant: GeneratedVariantModel,
) -> Result<()> {
    let variants = enums.entry(output).or_default();
    if let Some(existing) = variants
        .iter()
        .find(|existing| existing.variant == variant.variant)
    {
        return Err(syn::Error::new_spanned(
            &variant.rule_name,
            format!(
                "cannot generate enum variant `{}` from both `{}` and `{}`; generated model ownership must be one rule per enum variant",
                variant.variant, existing.rule_name, variant.rule_name
            ),
        ));
    }
    variants.push(variant);
    Ok(())
}

fn token_streams_match(left: &TokenStream2, right: &TokenStream2) -> bool {
    left.to_string() == right.to_string()
}

fn type_token_streams_match(left: &TokenStream2, right: &TokenStream2) -> bool {
    if token_streams_match(left, right) {
        return true;
    }
    let Ok(left) = syn::parse2::<Type>(left.clone()) else {
        return false;
    };
    let Ok(right) = syn::parse2::<Type>(right.clone()) else {
        return false;
    };
    canonical_type_key(&left)
        .is_some_and(|left| canonical_type_key(&right).is_some_and(|right| left == right))
}

fn canonical_type_key(ty: &Type) -> Option<String> {
    match ty {
        Type::Path(path) if path.qself.is_none() => {
            let segment = path.path.segments.last()?;
            let type_name = if segment.ident == "Vec" {
                "Vec".to_owned()
            } else if segment.ident == "Vec1" {
                "Vec1".to_owned()
            } else {
                path.path.to_token_stream().to_string()
            };
            let args = match &segment.arguments {
                PathArguments::None => String::new(),
                PathArguments::AngleBracketed(args) => {
                    let args = args
                        .args
                        .iter()
                        .map(|arg| match arg {
                            GenericArgument::Type(ty) => canonical_type_key(ty),
                            _ => Some(arg.to_token_stream().to_string()),
                        })
                        .collect::<Option<Vec<_>>>()?
                        .join(",");
                    format!("<{args}>")
                }
                PathArguments::Parenthesized(args) => args.to_token_stream().to_string(),
            };
            Some(format!("{type_name}{args}"))
        }
        Type::Tuple(tuple) => {
            let elems = tuple
                .elems
                .iter()
                .map(canonical_type_key)
                .collect::<Option<Vec<_>>>()?
                .join(",");
            Some(format!("({elems})"))
        }
        Type::Paren(paren) => canonical_type_key(&paren.elem),
        Type::Group(group) => canonical_type_key(&group.elem),
        Type::Reference(reference) => {
            canonical_type_key(&reference.elem).map(|inner| format!("&{inner}"))
        }
        _ => Some(ty.to_token_stream().to_string()),
    }
}

fn pascal_case_ident(name: &str) -> Ident {
    let mut out = String::new();
    let mut uppercase_next = true;
    for ch in name.chars() {
        if ch == '_' || ch == '-' || ch == ' ' {
            uppercase_next = true;
            continue;
        }
        if uppercase_next {
            out.extend(ch.to_uppercase());
            uppercase_next = false;
        } else {
            out.push(ch);
        }
    }
    if out.is_empty() {
        out.push_str("Rule");
    }
    format_ident!("{out}")
}

fn snake_case(name: &str) -> String {
    let mut out = String::new();
    let mut previous_was_separator = false;
    for (index, ch) in name.chars().enumerate() {
        if ch == '-' || ch == ' ' {
            if !out.is_empty() && !previous_was_separator {
                out.push('_');
            }
            previous_was_separator = true;
            continue;
        }
        if ch == '_' {
            if !out.is_empty() && !previous_was_separator {
                out.push('_');
            }
            previous_was_separator = true;
            continue;
        }
        if ch.is_uppercase() {
            if index > 0 && !previous_was_separator {
                out.push('_');
            }
            out.extend(ch.to_lowercase());
        } else {
            out.push(ch);
        }
        previous_was_separator = false;
    }
    out
}

fn syntax_type_ident_for_rule(name: &Ident) -> Ident {
    let base = pascal_case_ident(&name.to_string());
    format_ident!("{base}Syntax")
}

fn syntax_type_for_rule(name: &Ident) -> Type {
    let ident = syntax_type_ident_for_rule(name);
    parse_quote!(#ident)
}

fn generated_constructor_name(output: &Ident) -> String {
    let output = output.to_string();
    output
        .strip_suffix("Syntax")
        .unwrap_or(output.as_str())
        .to_owned()
}

fn enum_variant_ident_for_output(output: &Type, fallback: &Ident) -> Ident {
    let Some(output) = simple_type_ident(output) else {
        return pascal_case_ident(&fallback.to_string());
    };
    let output = output.to_string();
    let variant = output.strip_suffix("Syntax").unwrap_or(&output);
    format_ident!("{variant}")
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
        if self.fields.len() == 1 {
            let fields = self
                .fields
                .iter()
                .map(GeneratedFieldModel::expand_tuple_struct);
            return quote! {
                #[bityzba::invariant(true)]
                #[derive(Debug, Clone, PartialEq, Eq, ::serde::Serialize)]
                #visibility struct #ident(#(#fields),*);
            };
        }
        let fields = self
            .fields
            .iter()
            .map(GeneratedFieldModel::expand_named_struct);
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
    rule_name: Ident,
    fields: Vec<GeneratedFieldModel>,
    tuple: bool,
}

impl GeneratedVariantModel {
    fn invariant_attr(&self) -> TokenStream2 {
        let variant = &self.variant;
        if self.tuple {
            quote!(#[bityzba::invariant(::#variant(..) => true)])
        } else {
            quote!(#[bityzba::invariant(::#variant => true)])
        }
    }
}

impl ToTokens for GeneratedVariantModel {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let variant = &self.variant;
        let expanded = if self.tuple {
            let types = self
                .fields
                .iter()
                .map(GeneratedFieldModel::expand_tuple_variant);
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
    fn expand_named_struct(&self) -> TokenStream2 {
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

    fn expand_tuple_struct(&self) -> TokenStream2 {
        let attrs = &self.attrs;
        let ty = &self.ty;
        quote!(#(#attrs)* pub #ty)
    }

    fn expand_tuple_variant(&self) -> TokenStream2 {
        let attrs = &self.attrs;
        let ty = &self.ty;
        quote!(#(#attrs)* #ty)
    }
}

impl SyntaxGrammar {
    fn recovered_module_tokens(&self) -> TokenStream2 {
        self.recovered_module.as_ref().map_or_else(
            || quote!(crate::tree::recovered),
            |recovered_module| quote!(#recovered_module),
        )
    }

    fn expand_strict_recursive_family(&self) -> Option<TokenStream2> {
        if self.recursive.is_empty() {
            return None;
        }
        let all_recursive_names = self
            .recursive
            .iter()
            .map(|rule| rule.name.to_string())
            .collect::<BTreeSet<_>>();
        let recursive_rules = self
            .recursive
            .iter()
            .filter(|rule| !self.generate_model || self.rule_has_local_parser(&rule.output))
            .collect::<Vec<_>>();
        if recursive_rules.is_empty() {
            return None;
        }
        let local_recursive_names = recursive_rules
            .iter()
            .map(|rule| rule.name.to_string())
            .collect::<BTreeSet<_>>();
        let family_ident = format_ident!("StrictGeneratedParserFamily");
        let fields = recursive_rules.iter().map(|rule| {
            let name = &rule.name;
            let output = self.parser_type_tokens(&rule.output);
            quote!(#name: BoxedParser<'tokens, #output>)
        });
        let declarations = recursive_rules.iter().map(|rule| {
            let name = &rule.name;
            quote!(let mut #name = Recursive::declare();)
        });
        let definitions = recursive_rules.iter().filter_map(|recursive| {
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
                    if local_recursive_names.contains(&argument_name) {
                        Some(quote!(#argument.clone().boxed()))
                    } else if all_recursive_names.contains(&argument_name) {
                        Some(quote!(super::strict_generated_parser_family().#argument))
                    } else {
                        None
                    }
                })
                .collect::<Option<Vec<_>>>()?;
            let hidden_free_modifier = if local_recursive_names.contains("free_modifier") {
                let free_modifier = format_ident!("free_modifier");
                quote!(#free_modifier.clone().boxed())
            } else if all_recursive_names.contains("free_modifier") {
                quote!(super::strict_generated_parser_family().free_modifier)
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
        let outputs = recursive_rules.iter().map(|rule| {
            let name = &rule.name;
            quote!(#name: #name.boxed())
        });
        let root_functions = recursive_rules.iter().map(|rule| {
            let root_name = &rule.name;
            let function = format_ident!("strict_generated_{}_parser", root_name);
            let output = self.parser_type_tokens(&rule.output);
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

enum ParserExpr {
    Rust(Expr),
    Vector(VectorExpr),
    Chain(ChainExpr),
    Postfix {
        receiver: Box<ParserExpr>,
        method: Ident,
        args: Vec<Expr>,
    },
}

impl ParserExpr {
    fn compact_tokens(&self) -> String {
        match self {
            Self::Rust(expr) => compact_tokens(expr),
            Self::Vector(expr) => compact_tokens(&expr.to_token_stream()),
            Self::Chain(expr) => compact_tokens(&expr.to_token_stream()),
            Self::Postfix { .. } => compact_tokens(&self.to_token_stream()),
        }
    }

    fn to_token_stream(&self) -> TokenStream2 {
        match self {
            Self::Rust(expr) => quote!(#expr),
            Self::Vector(expr) => expr.to_token_stream(),
            Self::Chain(expr) => expr.to_token_stream(),
            Self::Postfix {
                receiver,
                method,
                args,
            } => {
                let receiver = receiver.to_token_stream();
                quote!(#receiver.#method(#(#args),*))
            }
        }
    }

    fn rust_tokens(&self) -> TokenStream2 {
        self.to_token_stream()
    }

    fn postfix(self, method: &str, args: Vec<Expr>) -> Self {
        Self::Postfix {
            receiver: Box::new(self),
            method: format_ident!("{method}"),
            args,
        }
    }
}

impl From<Expr> for ParserExpr {
    fn from(expr: Expr) -> Self {
        Self::Rust(expr)
    }
}

impl Parse for ParserExpr {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let mut expr = if input.peek(kw::chain) {
            Self::Chain(input.parse()?)
        } else if input.peek(syn::token::Bracket) {
            Self::Vector(input.parse()?)
        } else {
            return input.parse().map(Self::Rust);
        };
        while input.peek(Token![.]) {
            input.parse::<Token![.]>()?;
            let method = input.parse()?;
            let content;
            parenthesized!(content in input);
            let mut args = Vec::new();
            while !content.is_empty() {
                args.push(content.parse()?);
                if content.peek(Token![,]) {
                    content.parse::<Token![,]>()?;
                } else if !content.is_empty() {
                    return Err(content.error("expected `,` between parser method arguments"));
                }
            }
            expr = Self::Postfix {
                receiver: Box::new(expr),
                method,
                args,
            };
        }
        Ok(expr)
    }
}

struct ChainExpr {
    first: Box<ParserExpr>,
    links: Box<ParserExpr>,
    links_kind: ChainLinksKind,
    element: Ident,
}

#[derive(Clone, Copy)]
enum ChainLinksKind {
    ZeroOrMore,
    OneOrMore,
}

impl Parse for ChainExpr {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        input.parse::<kw::chain>()?;
        let content;
        parenthesized!(content in input);
        let mut first = None;
        let mut links = None;
        let mut links_kind = None;
        let mut element = None;
        while !content.is_empty() {
            if content.peek(kw::zero_or_more) {
                content.parse::<kw::zero_or_more>()?;
                content.parse::<Token![:]>()?;
                links = Some(Box::new(content.parse()?));
                links_kind = Some(ChainLinksKind::ZeroOrMore);
            } else if content.peek(kw::one_or_more) {
                content.parse::<kw::one_or_more>()?;
                content.parse::<Token![:]>()?;
                links = Some(Box::new(content.parse()?));
                links_kind = Some(ChainLinksKind::OneOrMore);
            } else {
                let label: Ident = content.parse()?;
                content.parse::<Token![:]>()?;
                match label.to_string().as_str() {
                    "first" => first = Some(Box::new(content.parse()?)),
                    "element" => element = Some(content.parse()?),
                    _ => {
                        return Err(syn::Error::new_spanned(
                            label,
                            "expected `first`, `zero_or_more`, `one_or_more`, or `element` in chain expression",
                        ));
                    }
                }
            }
            if content.peek(Token![,]) {
                content.parse::<Token![,]>()?;
            } else if !content.is_empty() {
                return Err(content.error("expected `,` between chain expression entries"));
            }
        }
        let first = first.ok_or_else(|| content.error("chain expression needs `first: ...`"))?;
        let links = links.ok_or_else(|| {
            content.error("chain expression needs `zero_or_more: ...` or `one_or_more: ...`")
        })?;
        let links_kind =
            links_kind.ok_or_else(|| content.error("chain expression needs link cardinality"))?;
        let element =
            element.ok_or_else(|| content.error("chain expression needs `element: field_name`"))?;
        Ok(Self {
            first,
            links,
            links_kind,
            element,
        })
    }
}

impl ToTokens for ChainExpr {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let first = self.first.to_token_stream();
        let links = self.links.to_token_stream();
        let element = &self.element;
        let links_kind = match self.links_kind {
            ChainLinksKind::ZeroOrMore => quote!(zero_or_more),
            ChainLinksKind::OneOrMore => quote!(one_or_more),
        };
        tokens.extend(quote!(chain(first: #first, #links_kind: #links, element: #element)));
    }
}

struct VectorExpr {
    items: Vec<VectorItem>,
}

enum VectorItem {
    One(ParserExpr),
    Spread(ParserExpr),
    ZeroOrMore(ParserExpr),
    ZeroOrMoreSpread(ParserExpr),
    OneOrMore(ParserExpr),
    OneOrMoreSpread(ParserExpr),
    Assert { negated: bool, parser: ParserExpr },
}

impl Parse for VectorExpr {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let content;
        bracketed!(content in input);
        let mut items = Vec::new();
        while !content.is_empty() {
            let item = if content.peek(kw::assert) {
                content.parse::<kw::assert>()?;
                let negated = content.peek(Token![!]);
                if negated {
                    content.parse::<Token![!]>()?;
                }
                VectorItem::Assert {
                    negated,
                    parser: content.parse()?,
                }
            } else if content.peek(Token![..]) {
                content.parse::<Token![..]>()?;
                VectorItem::Spread(content.parse()?)
            } else if content.peek(kw::zero_or_more) {
                content.parse::<kw::zero_or_more>()?;
                if content.peek(Token![..]) {
                    content.parse::<Token![..]>()?;
                    VectorItem::ZeroOrMoreSpread(content.parse()?)
                } else {
                    VectorItem::ZeroOrMore(content.parse()?)
                }
            } else if content.peek(kw::one_or_more) {
                content.parse::<kw::one_or_more>()?;
                if content.peek(Token![..]) {
                    content.parse::<Token![..]>()?;
                    VectorItem::OneOrMoreSpread(content.parse()?)
                } else {
                    VectorItem::OneOrMore(content.parse()?)
                }
            } else {
                VectorItem::One(content.parse()?)
            };
            items.push(item);
            if content.peek(Token![;]) {
                content.parse::<Token![;]>()?;
            } else if !content.is_empty() {
                return Err(content.error("expected `;` between vector parser items"));
            }
        }
        if items.is_empty() {
            return Err(content.error("vector parser expressions need at least one item"));
        }
        Ok(Self { items })
    }
}

impl ToTokens for VectorExpr {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let items = self.items.iter().map(VectorItem::to_token_stream);
        tokens.extend(quote!([#(#items;)*]));
    }
}

impl VectorItem {
    fn to_token_stream(&self) -> TokenStream2 {
        match self {
            Self::One(expr) => {
                let expr = expr.to_token_stream();
                quote!(#expr)
            }
            Self::Spread(expr) => {
                let expr = expr.to_token_stream();
                quote!(..#expr)
            }
            Self::ZeroOrMore(expr) => {
                let expr = expr.to_token_stream();
                quote!(zero_or_more #expr)
            }
            Self::ZeroOrMoreSpread(expr) => {
                let expr = expr.to_token_stream();
                quote!(zero_or_more ..#expr)
            }
            Self::OneOrMore(expr) => {
                let expr = expr.to_token_stream();
                quote!(one_or_more #expr)
            }
            Self::OneOrMoreSpread(expr) => {
                let expr = expr.to_token_stream();
                quote!(one_or_more ..#expr)
            }
            Self::Assert { negated, parser } => {
                let parser = parser.to_token_stream();
                if *negated {
                    quote!(assert !#parser)
                } else {
                    quote!(assert #parser)
                }
            }
        }
    }
}

enum Rule {
    Alias(AliasRule),
    Struct(NodeRule),
    Enum(EnumRule),
}

impl Rule {
    fn name(&self) -> &Ident {
        match self {
            Rule::Alias(rule) => &rule.name,
            Rule::Struct(rule) => &rule.name,
            Rule::Enum(rule) => &rule.name,
        }
    }

    fn output<'a>(&'a self, type_env: &'a GrammarTypeEnv) -> Option<&'a Type> {
        type_env.rules.get(&self.name().to_string())
    }

    fn declared_output(&self) -> Option<&Type> {
        match self {
            Rule::Alias(_) => None,
            Rule::Struct(rule) => Some(&rule.output),
            Rule::Enum(rule) => Some(&rule.output),
        }
    }

    fn expand_metadata(&self, type_env: &GrammarTypeEnv) -> Result<TokenStream2> {
        match self {
            Rule::Alias(rule) => rule.expand_metadata(type_env),
            Rule::Struct(rule) => Ok(rule.expand_metadata("struct")),
            Rule::Enum(rule) => rule.expand_metadata(type_env),
        }
    }

    fn arguments(&self) -> &[Ident] {
        match self {
            Rule::Alias(rule) => &rule.arguments,
            Rule::Struct(rule) => &rule.arguments,
            Rule::Enum(rule) => &rule.arguments,
        }
    }

    fn argument_types(&self, type_env: &GrammarTypeEnv) -> Option<BTreeMap<String, Type>> {
        match self {
            Rule::Alias(rule) => rule.argument_types(type_env),
            Rule::Struct(rule) => rule.argument_types(type_env),
            Rule::Enum(rule) => rule.argument_types(type_env),
        }
    }

    fn expand_strict_parser(
        &self,
        type_env: &GrammarTypeEnv,
        generate_model: bool,
        model_outputs: &Option<BTreeSet<String>>,
        model_all_rules_local: bool,
        model_path: Option<&Path>,
        use_model_construction: bool,
    ) -> Option<TokenStream2> {
        match self {
            Rule::Alias(rule) => rule.expand_strict_parser(
                type_env,
                generate_model,
                model_outputs,
                model_all_rules_local,
                model_path,
            ),
            Rule::Struct(rule) => rule.expand_strict_parser(
                type_env,
                generate_model,
                model_outputs,
                model_all_rules_local,
                model_path,
                use_model_construction,
            ),
            Rule::Enum(rule) => rule.expand_strict_parser(
                type_env,
                generate_model,
                model_outputs,
                model_all_rules_local,
                model_path,
                use_model_construction,
            ),
        }
    }

    fn expand_partial_valid_parser(
        &self,
        type_env: &GrammarTypeEnv,
        recovered_module: &TokenStream2,
    ) -> Option<TokenStream2> {
        match self {
            Rule::Alias(rule) => rule.expand_partial_valid_parser(type_env, recovered_module),
            Rule::Struct(rule) => rule.expand_partial_valid_parser(type_env, recovered_module),
            Rule::Enum(rule) => rule.expand_partial_valid_parser(type_env, recovered_module),
        }
    }
}

struct AliasRule {
    name: Ident,
    arguments: Vec<Ident>,
    context: Option<LitStr>,
    parser: ParserExpr,
}

impl AliasRule {
    fn expand_metadata(&self, type_env: &GrammarTypeEnv) -> Result<TokenStream2> {
        let name = self.name.to_string();
        let arguments = self.arguments.iter().map(Ident::to_string);
        let output = type_env
            .rules
            .get(&name)
            .ok_or_else(|| {
                syn::Error::new_spanned(
                    &self.name,
                    "cannot infer alias output type; add or fix a parser expression whose output is inferable",
                )
            })
            .map(compact_tokens)?;
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
        let parser = self.parser.compact_tokens();
        let recovery = classify_parser_expr(&self.parser, &argument_names).expand();
        Ok(quote! {
            SyntaxGrammarRule {
                kind: "alias",
                name: #name,
                arguments: &[#(#arguments),*],
                output: #output,
                context: #context,
                fields: &[
                    SyntaxGrammarField {
                    kind: "alias",
                    name: "",
                    parser: #parser,
                    recovery: #recovery,
                    conditions: &[],
                    }
                ],
            }
        })
    }

    fn expand_strict_parser(
        &self,
        type_env: &GrammarTypeEnv,
        generate_model: bool,
        model_outputs: &Option<BTreeSet<String>>,
        model_all_rules_local: bool,
        model_path: Option<&Path>,
    ) -> Option<TokenStream2> {
        let argument_types = self.argument_types(type_env)?;
        let output = type_env.rules.get(&self.name.to_string())?;
        let argument_names = self.argument_name_set();
        let free_modifier_parser = format_ident!("__generated_free_modifier");
        let generation = StrictParserGeneration {
            type_env,
            generate_model,
            model_outputs,
            model_all_rules_local,
        };
        let parser = strict_parser_expr_tokens(
            &self.parser,
            &argument_names,
            &generation,
            &free_modifier_parser,
            StrictParserCallMode::Local,
        )?;
        let name = format_ident!("strict_{}_parser", self.name);
        let output = parser_type_tokens(output, generate_model, model_outputs, model_path);
        let argument_params = self.arguments.iter().map(|argument| {
            let ty = argument_types
                .get(&argument.to_string())
                .expect("argument types are populated from recursive declarations");
            let ty = parser_type_tokens(ty, generate_model, model_outputs, model_path);
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
        let output = type_env.rules.get(&self.name.to_string())?;
        let output = simple_type_ident(output)?;
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
        if !input.peek(LitStr) {
            return Err(input.error("alias rules must use `alias \"context\" name = parser;`"));
        }
        let context = Some(input.parse()?);
        let name = input.parse()?;
        let arguments = parse_optional_arguments(input)?;
        if !input.peek(Token![=]) {
            return Err(input.error(
                "alias rules must use `=`; use parser method chains for parser-only assertions",
            ));
        }
        input.parse::<Token![=]>()?;
        let parser = input.parse()?;
        input.parse::<Token![;]>()?;
        Ok(Self {
            name,
            arguments,
            context,
            parser,
        })
    }
}

struct EnumRule {
    name: Ident,
    arguments: Vec<Ident>,
    output: Type,
    context: LitStr,
    branches: Vec<EnumBranch>,
}

struct EnumBranch {
    attrs: Vec<Attribute>,
    conditions: Vec<Condition>,
    name: Ident,
}

impl EnumRule {
    fn expand_metadata(&self, type_env: &GrammarTypeEnv) -> Result<TokenStream2> {
        let name = self.name.to_string();
        let arguments = self.arguments.iter().map(Ident::to_string);
        let output = compact_tokens(
            type_env
                .rules
                .get(&name)
                .ok_or_else(|| syn::Error::new_spanned(&self.name, "missing enum output type"))?,
        );
        let context = self.context.value();
        let fields = self.branches.iter().map(|branch| {
            let branch_name = branch.name.to_string();
            let conditions = branch.conditions.iter().map(Condition::expand);
            quote! {
                SyntaxGrammarField {
                    kind: "variant",
                    name: #branch_name,
                    parser: #branch_name,
                    recovery: SyntaxGrammarRecoveryExpr::Rule(#branch_name),
                    conditions: &[#(#conditions),*],
                }
            }
        });
        Ok(quote! {
            SyntaxGrammarRule {
                kind: "enum",
                name: #name,
                arguments: &[#(#arguments),*],
                output: #output,
                context: Some(#context),
                fields: &[#(#fields),*],
            }
        })
    }

    fn expand_strict_parser(
        &self,
        type_env: &GrammarTypeEnv,
        generate_model: bool,
        model_outputs: &Option<BTreeSet<String>>,
        model_all_rules_local: bool,
        model_path: Option<&Path>,
        use_model_construction: bool,
    ) -> Option<TokenStream2> {
        let argument_types = self.argument_types(type_env)?;
        let argument_names = self.argument_name_set();
        let free_modifier_parser = format_ident!("__generated_free_modifier");
        let generation = StrictParserGeneration {
            type_env,
            generate_model,
            model_outputs,
            model_all_rules_local,
        };
        let output_tokens =
            parser_type_tokens(&self.output, generate_model, model_outputs, model_path);
        let alternatives = self
            .branches
            .iter()
            .map(|branch| {
                let branch_name = branch.name.to_string();
                let branch_is_argument = argument_names.contains(&branch_name);
                let branch_output = if branch_is_argument {
                    argument_types.get(&branch_name)?
                } else {
                    type_env
                        .rules
                        .get(&branch_name)
                        .or_else(|| type_env.recursive.get(&branch_name))?
                };
                let variant = enum_variant_ident_for_output(branch_output, &branch.name);
                let field = &branch.name;
                let branch_parser = if branch_is_argument {
                    strict_argument_parser_tokens(
                        &branch_name,
                        &argument_names,
                        &generation,
                        StrictParserCallMode::Local,
                    )?
                } else if type_env.rules.contains_key(&branch_name) {
                    strict_rule_call_by_argument_names(
                        &branch_name,
                        type_env.rule_arguments_for_call(&branch_name)?,
                        &argument_names,
                        &generation,
                        &free_modifier_parser,
                        StrictParserCallMode::Local,
                    )?
                } else {
                    strict_argument_parser_tokens(
                        &branch_name,
                        &argument_names,
                        &generation,
                        StrictParserCallMode::Local,
                    )?
                };
                let branch_parser = branch
                    .conditions
                    .iter()
                    .rev()
                    .fold(branch_parser, |parser, condition| {
                        condition.expand_strict_gate(parser)
                    });
                let body = if use_model_construction {
                    quote!(#output_tokens::#variant(#field))
                } else {
                    quote!(bityzba::new!(#output_tokens::#variant { #field }))
                };
                Some(quote!(#branch_parser.map(|#field| #body)))
            })
            .collect::<Option<Vec<_>>>()?;
        let parser = strict_choice_chain(alternatives)?;
        let name = format_ident!("strict_{}_parser", self.name);
        let argument_params = self.arguments.iter().map(|argument| {
            let ty = argument_types
                .get(&argument.to_string())
                .expect("argument types are populated from recursive declarations");
            let ty = parser_type_tokens(ty, generate_model, model_outputs, model_path);
            quote!(#argument: BoxedParser<'tokens, #ty>)
        });
        let hidden_free_modifier = strict_free_modifier_param_tokens();
        let rule_name = self.name.to_string();
        let context = self.context.value();
        let parser_body = quote!(generated_runtime::syntax_context(#context, #parser));
        Some(quote! {
            #[allow(dead_code, unused_variables)]
            pub(crate) fn #name<'tokens>(
                #(#argument_params,)*
                #hidden_free_modifier
            ) -> BoxedParser<'tokens, #output_tokens> {
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

struct NodeRule {
    name: Ident,
    arguments: Vec<Ident>,
    output: Type,
    context: Option<LitStr>,
    fields: Vec<FieldItem>,
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
                FieldKind::Field | FieldKind::Computed => Some(field),
                FieldKind::TempLet | FieldKind::Require => None,
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
        type_env: &GrammarTypeEnv,
        generate_model: bool,
        model_outputs: &Option<BTreeSet<String>>,
        model_all_rules_local: bool,
        model_path: Option<&Path>,
        use_model_construction: bool,
    ) -> Option<TokenStream2> {
        let argument_types = self.argument_types(type_env)?;
        let argument_names = self.argument_name_set();
        let sequence_items = self
            .fields
            .iter()
            .filter(|field| matches!(field.kind, FieldKind::Field | FieldKind::Require))
            .collect::<Vec<_>>();
        let free_modifier_parser = format_ident!("__generated_free_modifier");
        let generation = StrictParserGeneration {
            type_env,
            generate_model,
            model_outputs,
            model_all_rules_local,
        };
        let (parser, pattern) = strict_sequence_parser_tokens(
            &sequence_items,
            &argument_names,
            &generation,
            &free_modifier_parser,
            StrictParserCallMode::Local,
        )?;
        let name = format_ident!("strict_{}_parser", self.name);
        let output = &self.output;
        let output_tokens = parser_type_tokens(output, generate_model, model_outputs, model_path);
        let argument_params = self.arguments.iter().map(|argument| {
            let ty = argument_types
                .get(&argument.to_string())
                .expect("argument types are populated from recursive declarations");
            let ty = parser_type_tokens(ty, generate_model, model_outputs, model_path);
            quote!(#argument: BoxedParser<'tokens, #ty>)
        });
        let hidden_free_modifier = strict_free_modifier_param_tokens();
        let body = if is_unit_type(output) {
            let let_bindings = self.fields.iter().filter_map(|field| {
                matches!(field.kind, FieldKind::Computed | FieldKind::TempLet).then(|| {
                    let name = field.name.as_ref().expect("let field items have names");
                    let value = field.parser.rust_tokens();
                    quote!(let #name = #value;)
                })
            });
            quote!({
                #(#let_bindings)*
                ()
            })
        } else if use_model_construction && matches!(output, Type::Tuple(_)) {
            let let_bindings = self.fields.iter().filter_map(|field| {
                matches!(field.kind, FieldKind::Computed | FieldKind::TempLet).then(|| {
                    let name = field.name.as_ref().expect("let field items have names");
                    let value = field.parser.rust_tokens();
                    quote!(let #name = #value;)
                })
            });
            let values = self.fields.iter().filter_map(|field| {
                let name = field.name.as_ref()?;
                match field.kind {
                    FieldKind::Field | FieldKind::Computed => Some(quote!(#name)),
                    FieldKind::TempLet | FieldKind::Require => None,
                }
            });
            quote!({
                #(#let_bindings)*
                (#(#values,)*)
            })
        } else if is_path_type(output) {
            let let_bindings = self.fields.iter().filter_map(|field| {
                matches!(field.kind, FieldKind::Computed | FieldKind::TempLet).then(|| {
                    let name = field.name.as_ref().expect("let field items have names");
                    let value = field.parser.rust_tokens();
                    quote!(let #name = #value;)
                })
            });
            let constructed_fields = self
                .fields
                .iter()
                .filter_map(|field| {
                    let name = field.name.as_ref()?;
                    match field.kind {
                        FieldKind::Field | FieldKind::Computed => Some(name),
                        FieldKind::TempLet | FieldKind::Require => None,
                    }
                })
                .collect::<Vec<_>>();
            if use_model_construction {
                if constructed_fields.len() == 1 {
                    let field = constructed_fields[0];
                    quote!({
                        #(#let_bindings)*
                        #output_tokens(#field)
                    })
                } else {
                    let assignments = constructed_fields.iter().map(|name| quote!(#name,));
                    quote!({
                        #(#let_bindings)*
                        #output_tokens { #(#assignments)* }
                    })
                }
            } else {
                let assignments = constructed_fields.iter().map(|name| quote!(#name,));
                quote!({
                    #(#let_bindings)*
                    bityzba::new!(#output_tokens { #(#assignments)* })
                })
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
            ) -> BoxedParser<'tokens, #output_tokens> {
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
        if !is_unit_type(&self.output) && !is_path_type(&self.output) {
            return None;
        }
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

struct GrammarTypeEnv {
    recursive: BTreeMap<String, Type>,
    rules: BTreeMap<String, Type>,
    rule_arguments: BTreeMap<String, Vec<String>>,
    generated_struct_fields: BTreeMap<String, BTreeMap<String, Type>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StrictParserCallMode {
    Local,
    External,
}

struct StrictParserGeneration<'a> {
    type_env: &'a GrammarTypeEnv,
    generate_model: bool,
    model_outputs: &'a Option<BTreeSet<String>>,
    model_all_rules_local: bool,
}

impl StrictParserGeneration<'_> {
    fn rule_has_local_parser(&self, name: &str) -> bool {
        self.model_all_rules_local || self.rule_is_generated_model(name)
    }

    fn rule_is_generated_model(&self, name: &str) -> bool {
        self.type_env.rules.get(name).is_some_and(|output| {
            output_is_generated_model(self.generate_model, self.model_outputs, output)
        })
    }

    fn recursive_has_local_parser(&self, name: &str) -> bool {
        self.model_all_rules_local || self.recursive_is_generated_model(name)
    }

    fn recursive_is_generated_model(&self, name: &str) -> bool {
        self.type_env.recursive.get(name).is_some_and(|output| {
            output_is_generated_model(self.generate_model, self.model_outputs, output)
        })
    }

    fn external_recursive_parser(&self, name: &Ident) -> TokenStream2 {
        quote!(super::strict_generated_parser_family().#name)
    }

    fn external_free_modifier_parser(&self) -> TokenStream2 {
        let name = format_ident!("free_modifier");
        if self.recursive_has_local_parser("free_modifier") {
            self.external_recursive_parser(&name)
        } else {
            quote!(__generated_free_modifier.clone())
        }
    }
}

impl GrammarTypeEnv {
    fn new(recursive: &[RecursiveRule], rules: &[Rule]) -> Self {
        let mut type_env = Self {
            recursive: recursive
                .iter()
                .map(|rule| (rule.name.to_string(), rule.output.clone()))
                .collect(),
            rules: BTreeMap::new(),
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
            generated_struct_fields: BTreeMap::new(),
        };

        for rule in rules {
            if let Some(output) = rule.declared_output() {
                type_env
                    .rules
                    .insert(rule.name().to_string(), output.clone());
            }
        }

        loop {
            let mut inserted = false;
            for rule in rules {
                let Rule::Alias(alias) = rule else {
                    continue;
                };
                let name = alias.name.to_string();
                if type_env.rules.contains_key(&name) {
                    continue;
                }
                let Some(argument_types) = alias.argument_types(&type_env) else {
                    continue;
                };
                let Some(output) = parser_output_type(&alias.parser, &type_env, &argument_types)
                    .and_then(|tokens| syn::parse2::<Type>(tokens).ok())
                else {
                    continue;
                };
                type_env.rules.insert(name, output);
                inserted = true;
            }
            if !inserted {
                break;
            }
        }

        type_env.generated_struct_fields = rules
            .iter()
            .filter_map(|rule| {
                let Rule::Struct(rule) = rule else {
                    return None;
                };
                let output = simple_type_ident(&rule.output)?.to_string();
                let argument_types = rule.argument_types(&type_env)?;
                let fields = rule
                    .fields
                    .iter()
                    .filter_map(|field| {
                        let name = field.name.as_ref()?.to_string();
                        let ty = field_type_for_chain_metadata(field, &type_env, &argument_types)?;
                        Some((name, ty))
                    })
                    .collect::<BTreeMap<_, _>>();
                Some((output, fields))
            })
            .collect();

        type_env
    }
}

impl GrammarTypeEnv {
    fn rule_arguments_for_call(&self, rule: &str) -> Option<&[String]> {
        self.rule_arguments.get(rule).map(Vec::as_slice)
    }

    fn generated_struct_field_type(
        &self,
        struct_ty: &TokenStream2,
        field: &Ident,
    ) -> Option<TokenStream2> {
        let ty = syn::parse2::<Type>(struct_ty.clone()).ok()?;
        let output = simple_type_ident(&ty)?;
        self.generated_struct_fields
            .get(&output.to_string())?
            .get(&field.to_string())
            .map(|ty| quote!(#ty))
    }
}

fn field_type_for_chain_metadata(
    field: &FieldItem,
    type_env: &GrammarTypeEnv,
    argument_types: &BTreeMap<String, Type>,
) -> Option<Type> {
    match (&field.ty, &field.kind) {
        (Some(ty), _) => Some(ty.clone()),
        (None, FieldKind::Field) => parser_output_type(&field.parser, type_env, argument_types)
            .and_then(|ty| syn::parse2::<Type>(ty).ok()),
        (None, FieldKind::Computed | FieldKind::TempLet | FieldKind::Require) => None,
    }
}

fn strict_free_modifier_param_tokens() -> TokenStream2 {
    quote!(__generated_free_modifier: BoxedParser<'tokens, FreeModifierSyntax>,)
}

fn strict_sequence_parser_tokens(
    fields: &[&FieldItem],
    arguments: &BTreeSet<String>,
    generation: &StrictParserGeneration<'_>,
    free_modifier_parser: &Ident,
    mode: StrictParserCallMode,
) -> Option<(TokenStream2, TokenStream2)> {
    let Some(first) = fields.first() else {
        return Some((quote!(generated_runtime::empty()), quote!(())));
    };
    let mut parser = strict_parser_expr_tokens(
        &first.parser,
        arguments,
        generation,
        free_modifier_parser,
        mode,
    )?;
    let mut pattern = sequence_item_pattern(first);
    for field in fields.iter().skip(1) {
        let next = strict_parser_expr_tokens(
            &field.parser,
            arguments,
            generation,
            free_modifier_parser,
            mode,
        )?;
        let name = sequence_item_pattern(field);
        parser = quote!(#parser.then(#next));
        pattern = quote!((#pattern, #name));
    }
    Some((parser, pattern))
}

fn sequence_item_pattern(field: &FieldItem) -> TokenStream2 {
    match field.kind {
        FieldKind::Field => field
            .name
            .as_ref()
            .expect("field items have names")
            .to_token_stream(),
        FieldKind::Require => quote!(_),
        FieldKind::Computed | FieldKind::TempLet => {
            unreachable!("computed items are not parser sequence items")
        }
    }
}

fn strict_parser_expr_tokens(
    expr: &ParserExpr,
    arguments: &BTreeSet<String>,
    generation: &StrictParserGeneration<'_>,
    free_modifier_parser: &Ident,
    mode: StrictParserCallMode,
) -> Option<TokenStream2> {
    match expr {
        ParserExpr::Rust(expr) => {
            strict_rust_parser_expr_tokens(expr, arguments, generation, free_modifier_parser, mode)
        }
        ParserExpr::Vector(expr) => strict_vector_parser_expr_tokens(
            expr,
            arguments,
            generation,
            free_modifier_parser,
            mode,
        ),
        ParserExpr::Chain(expr) => {
            strict_chain_parser_expr_tokens(expr, arguments, generation, free_modifier_parser, mode)
        }
        ParserExpr::Postfix {
            receiver,
            method,
            args,
        } => strict_postfix_parser_expr_tokens(
            receiver,
            method,
            args,
            arguments,
            generation,
            free_modifier_parser,
            mode,
        ),
    }
}

fn strict_chain_parser_expr_tokens(
    expr: &ChainExpr,
    arguments: &BTreeSet<String>,
    generation: &StrictParserGeneration<'_>,
    free_modifier_parser: &Ident,
    mode: StrictParserCallMode,
) -> Option<TokenStream2> {
    let first = strict_parser_expr_tokens(
        &expr.first,
        arguments,
        generation,
        free_modifier_parser,
        mode,
    )?;
    let link = strict_parser_expr_tokens(
        &expr.links,
        arguments,
        generation,
        free_modifier_parser,
        mode,
    )?;
    let links = match expr.links_kind {
        ChainLinksKind::ZeroOrMore => quote!(generated_runtime::strict_greedy_many_parser(
            #link.boxed()
        )),
        ChainLinksKind::OneOrMore => quote! {
            generated_runtime::strict_greedy_many1_parser(#link.boxed()).map(|__links| {
                vec1::Vec1::try_from_vec(__links)
                    .expect("chain parser expression has statically non-zero link cardinality")
            })
        },
    };
    Some(quote! {
        #first
            .then(#links)
            .map(|(first, links)| ::jbotci_tree::Chain::new(first, links))
    })
}

fn strict_postfix_parser_expr_tokens(
    receiver: &ParserExpr,
    method: &Ident,
    args: &[Expr],
    arguments: &BTreeSet<String>,
    generation: &StrictParserGeneration<'_>,
    free_modifier_parser: &Ident,
    mode: StrictParserCallMode,
) -> Option<TokenStream2> {
    let inner =
        strict_parser_expr_tokens(receiver, arguments, generation, free_modifier_parser, mode)?;
    match (method.to_string().as_str(), args.len()) {
        ("lookahead", 0) => Some(quote!(generated_runtime::lookahead(#inner))),
        ("not", 0) => Some(quote!(generated_runtime::not(#inner))),
        ("ignored", 0) => Some(quote!(#inner.map(|_| ()))),
        ("ignore_then", 1) => {
            let parser = strict_rust_parser_expr_tokens(
                args.first().expect("length checked"),
                arguments,
                generation,
                free_modifier_parser,
                mode,
            )?;
            Some(quote!(#inner.ignore_then(#parser)))
        }
        ("wf" | "with_free_modifiers" | "prohibited_wf", 0) => {
            let free_modifier =
                strict_free_modifier_argument_tokens(generation, free_modifier_parser, mode);
            let free_modifier_list = if method == "prohibited_wf" {
                quote!(generated_runtime::strict_cll_prohibited_free_modifier_list_parser(
                    #free_modifier
                ))
            } else {
                quote!(generated_runtime::strict_free_modifier_list_parser(#free_modifier))
            };
            Some(quote! {
                #inner
                    .then(#free_modifier_list)
                    .map(|(value, free_modifiers)| WithFreeModifiers::new(value, free_modifiers))
            })
        }
        _ => None,
    }
}

fn strict_vector_parser_expr_tokens(
    expr: &VectorExpr,
    arguments: &BTreeSet<String>,
    generation: &StrictParserGeneration<'_>,
    free_modifier_parser: &Ident,
    mode: StrictParserCallMode,
) -> Option<TokenStream2> {
    let mut parsers = Vec::new();
    let mut bindings = Vec::new();
    let mut statements = Vec::new();
    for (index, item) in expr.items.iter().enumerate() {
        let binding = format_ident!("__vector_item_{index}");
        match item {
            VectorItem::One(expr) => {
                parsers.push(strict_parser_expr_tokens(
                    expr,
                    arguments,
                    generation,
                    free_modifier_parser,
                    mode,
                )?);
                bindings.push(quote!(#binding));
                statements.push(quote!(__items.push(#binding);));
            }
            VectorItem::Spread(expr) => {
                parsers.push(strict_parser_expr_tokens(
                    expr,
                    arguments,
                    generation,
                    free_modifier_parser,
                    mode,
                )?);
                bindings.push(quote!(#binding));
                statements.push(quote!(__items.extend(#binding);));
            }
            VectorItem::ZeroOrMore(expr) => {
                let inner = strict_parser_expr_tokens(
                    expr,
                    arguments,
                    generation,
                    free_modifier_parser,
                    mode,
                )?;
                parsers.push(quote!(generated_runtime::strict_greedy_many_parser(
                    #inner.boxed()
                )));
                bindings.push(quote!(#binding));
                statements.push(quote!(__items.extend(#binding);));
            }
            VectorItem::ZeroOrMoreSpread(expr) => {
                let inner = strict_parser_expr_tokens(
                    expr,
                    arguments,
                    generation,
                    free_modifier_parser,
                    mode,
                )?;
                parsers.push(quote!(generated_runtime::strict_greedy_many_parser(
                    #inner.boxed()
                )));
                bindings.push(quote!(#binding));
                statements.push(quote! {
                    for __chunk in #binding {
                        __items.extend(__chunk);
                    }
                });
            }
            VectorItem::OneOrMore(expr) => {
                let inner = strict_parser_expr_tokens(
                    expr,
                    arguments,
                    generation,
                    free_modifier_parser,
                    mode,
                )?;
                parsers.push(quote!(generated_runtime::strict_greedy_many1_parser(
                    #inner.boxed()
                )));
                bindings.push(quote!(#binding));
                statements.push(quote!(__items.extend(#binding);));
            }
            VectorItem::OneOrMoreSpread(expr) => {
                let inner = strict_parser_expr_tokens(
                    expr,
                    arguments,
                    generation,
                    free_modifier_parser,
                    mode,
                )?;
                parsers.push(quote!(generated_runtime::strict_greedy_many1_parser(
                    #inner.boxed()
                )));
                bindings.push(quote!(#binding));
                statements.push(quote! {
                    for __chunk in #binding {
                        __items.extend(__chunk);
                    }
                });
            }
            VectorItem::Assert { negated, parser } => {
                let parser = strict_parser_expr_tokens(
                    parser,
                    arguments,
                    generation,
                    free_modifier_parser,
                    mode,
                )?;
                let parser = if *negated {
                    quote!(generated_runtime::not(#parser))
                } else {
                    quote!(generated_runtime::lookahead(#parser).map(|_| ()))
                };
                parsers.push(parser);
                bindings.push(quote!(_));
            }
        }
    }
    let parser = strict_sequence_expr_chain(parsers)?;
    let pattern = nested_sequence_pattern(bindings)?;
    let returns_vec1 = vector_output_is_vec1(expr, generation.type_env, arguments)?;
    let finish = if returns_vec1 {
        quote! {
            vec1::Vec1::try_from_vec(__items)
                .expect("vector parser expression has statically non-zero cardinality")
        }
    } else {
        quote!(__items)
    };
    Some(quote! {
        #parser.map(|#pattern| {
            let mut __items = Vec::new();
            #(#statements)*
            #finish
        })
    })
}

fn nested_sequence_pattern(mut bindings: Vec<TokenStream2>) -> Option<TokenStream2> {
    if bindings.is_empty() {
        return Some(quote!(()));
    }
    let mut pattern = bindings.remove(0);
    for binding in bindings {
        pattern = quote!((#pattern, #binding));
    }
    Some(pattern)
}

fn strict_rust_parser_expr_tokens(
    expr: &Expr,
    arguments: &BTreeSet<String>,
    generation: &StrictParserGeneration<'_>,
    free_modifier_parser: &Ident,
    mode: StrictParserCallMode,
) -> Option<TokenStream2> {
    match expr {
        Expr::Call(call) => {
            strict_call_parser_expr_tokens(call, arguments, generation, free_modifier_parser, mode)
        }
        Expr::MethodCall(method) => strict_method_parser_expr_tokens(
            method,
            arguments,
            generation,
            free_modifier_parser,
            mode,
        ),
        Expr::Path(path) => {
            strict_path_parser_expr_tokens(path, arguments, generation, free_modifier_parser, mode)
        }
        Expr::Tuple(tuple) => strict_tuple_parser_expr_tokens(
            tuple,
            arguments,
            generation,
            free_modifier_parser,
            mode,
        ),
        Expr::Array(array) => strict_vector_parser_expr_tokens(
            &array_vector_expr(array)?,
            arguments,
            generation,
            free_modifier_parser,
            mode,
        ),
        _ => None,
    }
}

fn strict_method_parser_expr_tokens(
    method: &ExprMethodCall,
    arguments: &BTreeSet<String>,
    generation: &StrictParserGeneration<'_>,
    free_modifier_parser: &Ident,
    mode: StrictParserCallMode,
) -> Option<TokenStream2> {
    if method.method == "warn" && method.args.len() == 1 {
        let inner = strict_rust_parser_expr_tokens(
            &method.receiver,
            arguments,
            generation,
            free_modifier_parser,
            mode,
        )?;
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
        let inner = strict_rust_parser_expr_tokens(
            &method.receiver,
            arguments,
            generation,
            free_modifier_parser,
            mode,
        )?;
        let selmaho = method.args.first().and_then(path_expr_last_segment)?;
        let selmaho = format_ident!("{selmaho}");
        Some(quote! {
            #inner
                .then(generated_runtime::not_next_selmaho(Selmaho::#selmaho))
                .map(|(value, _)| value)
        })
    } else if method.method == "not_next_token" && method.args.len() == 1 {
        let inner = strict_rust_parser_expr_tokens(
            &method.receiver,
            arguments,
            generation,
            free_modifier_parser,
            mode,
        )?;
        let predicate = method.args.first().and_then(path_expr_last_segment)?;
        let predicate = format_ident!("{predicate}");
        Some(quote! {
            #inner
                .then(generated_runtime::not_next_token(SyntaxGrammarTokenPredicate::#predicate))
                .map(|(value, _)| value)
        })
    } else if method.method == "not_next_rule" && method.args.len() == 1 {
        let inner = strict_rust_parser_expr_tokens(
            &method.receiver,
            arguments,
            generation,
            free_modifier_parser,
            mode,
        )?;
        let rule = method.args.first().and_then(path_expr_last_segment)?;
        if !generation.type_env.rules.contains_key(&rule) {
            return None;
        }
        let parser_arguments = generation
            .type_env
            .rule_arguments
            .get(&rule)
            .into_iter()
            .flatten()
            .map(|argument| strict_argument_parser_tokens(argument, arguments, generation, mode))
            .collect::<Option<Vec<_>>>()?;
        let parser_name = format_ident!("strict_{}_parser", rule);
        let parser_name = if mode == StrictParserCallMode::External
            || (generation.generate_model && !generation.rule_has_local_parser(&rule))
        {
            quote!(super::#parser_name)
        } else {
            quote!(#parser_name)
        };
        let free_modifier =
            strict_free_modifier_argument_tokens(generation, free_modifier_parser, mode);
        let expected = format!("not {rule}");
        Some(quote! {
            generated_runtime::not_next_rule_after(
                #inner,
                #parser_name(
                    #(#parser_arguments,)*
                    #free_modifier,
                ),
                #expected,
            )
        })
    } else if method.method == "followed_by" && method.args.len() == 1 {
        let guard_expr = method.args.first()?;
        let inner = strict_rust_parser_expr_tokens(
            &method.receiver,
            arguments,
            generation,
            free_modifier_parser,
            mode,
        )?;
        let guard = strict_rust_parser_expr_tokens(
            guard_expr,
            arguments,
            generation,
            free_modifier_parser,
            mode,
        )?;
        Some(quote!(generated_runtime::followed_by(#inner, #guard)))
    } else if method.method == "lookahead" && method.args.is_empty() {
        let inner = strict_rust_parser_expr_tokens(
            &method.receiver,
            arguments,
            generation,
            free_modifier_parser,
            mode,
        )?;
        Some(quote!(generated_runtime::lookahead(#inner)))
    } else if method.method == "not" && method.args.is_empty() {
        let inner = strict_rust_parser_expr_tokens(
            &method.receiver,
            arguments,
            generation,
            free_modifier_parser,
            mode,
        )?;
        Some(quote!(generated_runtime::not(#inner)))
    } else if method.method == "ignored" && method.args.is_empty() {
        let inner = strict_rust_parser_expr_tokens(
            &method.receiver,
            arguments,
            generation,
            free_modifier_parser,
            mode,
        )?;
        Some(quote!(#inner.map(|_| ())))
    } else if method.method == "ignore_then" && method.args.len() == 1 {
        let inner = strict_rust_parser_expr_tokens(
            &method.receiver,
            arguments,
            generation,
            free_modifier_parser,
            mode,
        )?;
        let parser = strict_rust_parser_expr_tokens(
            method.args.first().expect("length checked"),
            arguments,
            generation,
            free_modifier_parser,
            mode,
        )?;
        Some(quote!(#inner.ignore_then(#parser)))
    } else if (method.method == "wf"
        || method.method == "with_free_modifiers"
        || method.method == "prohibited_wf")
        && method.args.is_empty()
    {
        let inner = strict_rust_parser_expr_tokens(
            &method.receiver,
            arguments,
            generation,
            free_modifier_parser,
            mode,
        )?;
        let free_modifier =
            strict_free_modifier_argument_tokens(generation, free_modifier_parser, mode);
        let free_modifier_list = if method.method == "prohibited_wf" {
            quote!(generated_runtime::strict_cll_prohibited_free_modifier_list_parser(
                #free_modifier
            ))
        } else {
            quote!(generated_runtime::strict_free_modifier_list_parser(#free_modifier))
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
    generation: &StrictParserGeneration<'_>,
    free_modifier_parser: &Ident,
    mode: StrictParserCallMode,
) -> Option<TokenStream2> {
    let function = call_name(call)?;
    if generation.type_env.rules.contains_key(&function) {
        return strict_rule_call_parser_tokens(
            &function,
            call.args.iter(),
            arguments,
            generation,
            free_modifier_parser,
            mode,
        );
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
        ("word_not_cmavo", _) if !call.args.is_empty() => {
            let terminators = call
                .args
                .iter()
                .map(|argument| {
                    let cmavo = path_expr_last_segment(argument)?;
                    let cmavo = format_ident!("{cmavo}");
                    Some(quote!(Cmavo::#cmavo))
                })
                .collect::<Option<Vec<_>>>()?;
            Some(quote!(generated_runtime::word_not_cmavo(&[#(#terminators),*])))
        }
        ("feature", 2) => {
            let feature = call.args.first().and_then(path_expr_last_segment)?;
            let feature = format_ident!("{feature}");
            let inner = strict_rust_parser_expr_tokens(
                call.args.iter().nth(1).expect("length checked"),
                arguments,
                generation,
                free_modifier_parser,
                mode,
            )?;
            Some(quote!(generated_runtime::feature_gate(
                generated_runtime::SyntaxGrammarFeature::#feature,
                #inner,
            )))
        }
        ("feature", 1) => {
            let feature = call.args.first().and_then(path_expr_last_segment)?;
            let feature = format_ident!("{feature}");
            Some(quote!(generated_runtime::feature_gate(
                generated_runtime::SyntaxGrammarFeature::#feature,
                generated_runtime::empty(),
            )))
        }
        ("policy", 2) => {
            let policy = call.args.first().and_then(path_expr_last_segment)?;
            let policy = format_ident!("{policy}");
            let inner = strict_rust_parser_expr_tokens(
                call.args.iter().nth(1).expect("length checked"),
                arguments,
                generation,
                free_modifier_parser,
                mode,
            )?;
            Some(quote!(generated_runtime::policy_gate(
                generated_runtime::SyntaxGrammarPolicyFlag::#policy,
                #inner,
            )))
        }
        ("policy", 1) => {
            let policy = call.args.first().and_then(path_expr_last_segment)?;
            let policy = format_ident!("{policy}");
            Some(quote!(generated_runtime::policy_gate(
                generated_runtime::SyntaxGrammarPolicyFlag::#policy,
                generated_runtime::empty(),
            )))
        }
        ("relation_word", 0) => Some(quote!(relation_word())),
        ("tanru_unit_relation_word", 0) => {
            Some(quote!(generated_runtime::tanru_unit_relation_word()))
        }
        ("text_leading_cmevla_word", 0) => {
            Some(quote!(generated_runtime::text_leading_cmevla_word()))
        }
        ("cmevla_word", 0) => {
            let parser = format_ident!("{function}");
            Some(quote!(#parser()))
        }
        ("pa_word", 0) => Some(quote!(pa_word())),
        ("opt", 1) => {
            let inner = strict_rust_parser_expr_tokens(
                call.args.first().expect("length checked"),
                arguments,
                generation,
                free_modifier_parser,
                mode,
            )?;
            Some(quote!(generated_runtime::strict_optional(#inner)))
        }
        ("boxed", 1) => {
            let inner = strict_rust_parser_expr_tokens(
                call.args.first().expect("length checked"),
                arguments,
                generation,
                free_modifier_parser,
                mode,
            )?;
            Some(quote!(#inner.map(Box::new)))
        }
        ("arc", 1) => {
            let inner = strict_rust_parser_expr_tokens(
                call.args.first().expect("length checked"),
                arguments,
                generation,
                free_modifier_parser,
                mode,
            )?;
            Some(quote!(#inner.map(std::sync::Arc::new)))
        }
        ("choice", 1) => {
            let alternatives =
                call.args
                    .first()
                    .map(choice_alternative_exprs)
                    .and_then(|exprs| {
                        strict_choice_alternative_parser_tokens(
                            exprs,
                            arguments,
                            generation,
                            free_modifier_parser,
                            mode,
                        )
                    })?;
            strict_choice_chain(alternatives)
        }
        ("choice", _) => {
            let alternatives = strict_choice_alternative_parser_tokens(
                call.args.iter().collect(),
                arguments,
                generation,
                free_modifier_parser,
                mode,
            )?;
            strict_choice_chain(alternatives)
        }
        ("empty", 0) => Some(quote!(generated_runtime::empty())),
        ("eof", 0) => Some(quote!(generated_runtime::eof())),
        _ => None,
    }
}

fn strict_path_parser_expr_tokens(
    path: &ExprPath,
    arguments: &BTreeSet<String>,
    generation: &StrictParserGeneration<'_>,
    free_modifier_parser: &Ident,
    mode: StrictParserCallMode,
) -> Option<TokenStream2> {
    if path.qself.is_none()
        && path.path.segments.len() == 1
        && let Some(segment) = path.path.segments.first()
    {
        if arguments.contains(&segment.ident.to_string()) {
            return strict_argument_parser_tokens(
                &segment.ident.to_string(),
                arguments,
                generation,
                mode,
            );
        }
        strict_rule_call_parser_tokens(
            &segment.ident.to_string(),
            std::iter::empty(),
            arguments,
            generation,
            free_modifier_parser,
            mode,
        )
    } else {
        None
    }
}

fn strict_tuple_parser_expr_tokens(
    tuple: &ExprTuple,
    arguments: &BTreeSet<String>,
    generation: &StrictParserGeneration<'_>,
    free_modifier_parser: &Ident,
    mode: StrictParserCallMode,
) -> Option<TokenStream2> {
    let parts = tuple
        .elems
        .iter()
        .map(|expr| {
            strict_rust_parser_expr_tokens(expr, arguments, generation, free_modifier_parser, mode)
        })
        .collect::<Option<Vec<_>>>()?;
    strict_sequence_expr_chain(parts)
}

fn strict_rule_call_parser_tokens<'a>(
    function: &str,
    argument_exprs: impl Iterator<Item = &'a Expr>,
    arguments: &BTreeSet<String>,
    generation: &StrictParserGeneration<'_>,
    free_modifier_parser: &Ident,
    mode: StrictParserCallMode,
) -> Option<TokenStream2> {
    if !generation.type_env.rules.contains_key(function) {
        return None;
    }
    let call_mode = if mode == StrictParserCallMode::External
        || (generation.generate_model && !generation.rule_has_local_parser(function))
    {
        StrictParserCallMode::External
    } else {
        StrictParserCallMode::Local
    };
    let parser_arguments = argument_exprs
        .map(|argument| {
            strict_rust_parser_expr_tokens(
                argument,
                arguments,
                generation,
                free_modifier_parser,
                call_mode,
            )
        })
        .collect::<Option<Vec<_>>>()?;
    strict_rule_call_tokens(
        function,
        parser_arguments,
        generation,
        free_modifier_parser,
        call_mode,
    )
}

fn strict_rule_call_by_argument_names(
    function: &str,
    argument_names: &[String],
    available_arguments: &BTreeSet<String>,
    generation: &StrictParserGeneration<'_>,
    free_modifier_parser: &Ident,
    mode: StrictParserCallMode,
) -> Option<TokenStream2> {
    if !generation.type_env.rules.contains_key(function) {
        return None;
    }
    let call_mode = if mode == StrictParserCallMode::External
        || (generation.generate_model && !generation.rule_has_local_parser(function))
    {
        StrictParserCallMode::External
    } else {
        StrictParserCallMode::Local
    };
    let parser_arguments = argument_names
        .iter()
        .map(|argument| {
            strict_argument_parser_tokens(argument, available_arguments, generation, call_mode)
        })
        .collect::<Option<Vec<_>>>()?;
    strict_rule_call_tokens(
        function,
        parser_arguments,
        generation,
        free_modifier_parser,
        call_mode,
    )
}

fn strict_rule_call_tokens(
    function: &str,
    parser_arguments: Vec<TokenStream2>,
    generation: &StrictParserGeneration<'_>,
    free_modifier_parser: &Ident,
    call_mode: StrictParserCallMode,
) -> Option<TokenStream2> {
    let parser_name = format_ident!("strict_{}_parser", function);
    let parser_name = if call_mode == StrictParserCallMode::External {
        quote!(super::#parser_name)
    } else {
        quote!(#parser_name)
    };
    let free_modifier =
        strict_free_modifier_argument_tokens(generation, free_modifier_parser, call_mode);
    Some(quote!(#parser_name(
        #(#parser_arguments,)*
        #free_modifier
    )))
}

fn strict_argument_parser_tokens(
    argument: &str,
    arguments: &BTreeSet<String>,
    generation: &StrictParserGeneration<'_>,
    mode: StrictParserCallMode,
) -> Option<TokenStream2> {
    if !arguments.contains(argument) {
        return None;
    }
    let argument = format_ident!("{argument}");
    if mode == StrictParserCallMode::External
        && generation.recursive_has_local_parser(&argument.to_string())
    {
        Some(generation.external_recursive_parser(&argument))
    } else {
        Some(quote!(#argument.clone()))
    }
}

fn strict_free_modifier_argument_tokens(
    generation: &StrictParserGeneration<'_>,
    free_modifier_parser: &Ident,
    mode: StrictParserCallMode,
) -> TokenStream2 {
    if mode == StrictParserCallMode::External {
        generation.external_free_modifier_parser()
    } else {
        quote!(#free_modifier_parser.clone())
    }
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

fn choice_alternative_exprs(expr: &Expr) -> Vec<&Expr> {
    if let Expr::Tuple(ExprTuple { elems, .. }) = expr {
        elems.iter().collect()
    } else {
        vec![expr]
    }
}

fn strict_choice_alternative_parser_tokens(
    exprs: Vec<&Expr>,
    arguments: &BTreeSet<String>,
    generation: &StrictParserGeneration<'_>,
    free_modifier_parser: &Ident,
    mode: StrictParserCallMode,
) -> Option<Vec<TokenStream2>> {
    let argument_types = argument_type_map(arguments, generation.type_env)?;
    let outputs = exprs
        .iter()
        .map(|expr| rust_parser_output_type(expr, generation.type_env, &argument_types))
        .collect::<Option<Vec<_>>>()?;
    let target_output = common_choice_output_type(&outputs)?;
    exprs
        .iter()
        .zip(outputs.iter())
        .map(|(expr, output)| {
            let parser = strict_rust_parser_expr_tokens(
                expr,
                arguments,
                generation,
                free_modifier_parser,
                mode,
            )?;
            coerce_choice_parser_output(parser, output, &target_output)
        })
        .collect()
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
    expr: &ParserExpr,
    type_env: &GrammarTypeEnv,
    arguments: &BTreeMap<String, Type>,
) -> Option<TokenStream2> {
    match expr {
        ParserExpr::Rust(expr) => rust_parser_output_type(expr, type_env, arguments),
        ParserExpr::Vector(expr) => vector_parser_output_type(expr, type_env, arguments),
        ParserExpr::Chain(expr) => chain_parser_output_type(expr, type_env, arguments),
        ParserExpr::Postfix {
            receiver,
            method,
            args,
        } => postfix_parser_output_type(receiver, method, args, type_env, arguments),
    }
}

fn chain_parser_output_type(
    expr: &ChainExpr,
    type_env: &GrammarTypeEnv,
    arguments: &BTreeMap<String, Type>,
) -> Option<TokenStream2> {
    let first = parser_output_type(&expr.first, type_env, arguments)?;
    let link = parser_output_type(&expr.links, type_env, arguments)?;
    let element = type_env.generated_struct_field_type(&link, &expr.element)?;
    if !type_token_streams_match(&first, &element) {
        return None;
    }
    let links = match expr.links_kind {
        ChainLinksKind::ZeroOrMore => quote!(Vec<#link>),
        ChainLinksKind::OneOrMore => quote!(vec1::Vec1<#link>),
    };
    Some(quote!(::jbotci_tree::Chain<#first, #links>))
}

fn postfix_parser_output_type(
    receiver: &ParserExpr,
    method: &Ident,
    args: &[Expr],
    type_env: &GrammarTypeEnv,
    arguments: &BTreeMap<String, Type>,
) -> Option<TokenStream2> {
    match (method.to_string().as_str(), args.len()) {
        ("lookahead", 0) => parser_output_type(receiver, type_env, arguments),
        ("not" | "ignored", 0) => Some(quote!(())),
        ("ignore_then", 1) => {
            rust_parser_output_type(args.first().expect("length checked"), type_env, arguments)
        }
        ("wf" | "with_free_modifiers" | "prohibited_wf", 0) => {
            let inner = parser_output_type(receiver, type_env, arguments)?;
            Some(quote!(WithFreeModifiers<#inner, FreeModifierSyntax>))
        }
        _ => None,
    }
}

fn vector_parser_output_type(
    expr: &VectorExpr,
    type_env: &GrammarTypeEnv,
    arguments: &BTreeMap<String, Type>,
) -> Option<TokenStream2> {
    let element = vector_element_type(expr, type_env, arguments)?;
    if vector_min_cardinality(expr, type_env, arguments)? > 0 {
        Some(quote!(vec1::Vec1<#element>))
    } else {
        Some(quote!(Vec<#element>))
    }
}

fn vector_output_is_vec1(
    expr: &VectorExpr,
    type_env: &GrammarTypeEnv,
    argument_names: &BTreeSet<String>,
) -> Option<bool> {
    let arguments = argument_type_map(argument_names, type_env)?;
    Some(vector_min_cardinality(expr, type_env, &arguments)? > 0)
}

fn argument_type_map(
    argument_names: &BTreeSet<String>,
    type_env: &GrammarTypeEnv,
) -> Option<BTreeMap<String, Type>> {
    argument_names
        .iter()
        .map(|name| Some((name.clone(), type_env.recursive.get(name)?.clone())))
        .collect()
}

fn vector_element_type(
    expr: &VectorExpr,
    type_env: &GrammarTypeEnv,
    arguments: &BTreeMap<String, Type>,
) -> Option<TokenStream2> {
    let mut element: Option<TokenStream2> = None;
    for item in &expr.items {
        let item_element = match item {
            VectorItem::One(expr) => Some(parser_output_type(expr, type_env, arguments)?),
            VectorItem::Spread(expr) => {
                let output = parser_output_type(expr, type_env, arguments)?;
                vector_collection_element_type(&output)
            }
            VectorItem::ZeroOrMoreSpread(expr) | VectorItem::OneOrMoreSpread(expr) => {
                let output = parser_output_type(expr, type_env, arguments)?;
                vector_collection_element_type(&output)
            }
            VectorItem::ZeroOrMore(expr) | VectorItem::OneOrMore(expr) => {
                Some(parser_output_type(expr, type_env, arguments)?)
            }
            VectorItem::Assert { .. } => None,
        };
        let Some(item_element) = item_element else {
            continue;
        };
        if let Some(element) = &element {
            if !type_token_streams_match(element, &item_element) {
                return None;
            }
        } else {
            element = Some(item_element);
        }
    }
    element
}

fn vector_min_cardinality(
    expr: &VectorExpr,
    type_env: &GrammarTypeEnv,
    arguments: &BTreeMap<String, Type>,
) -> Option<usize> {
    let mut min = 0usize;
    for item in &expr.items {
        match item {
            VectorItem::One(_) | VectorItem::OneOrMore(_) => min += 1,
            VectorItem::OneOrMoreSpread(expr) => {
                let output = parser_output_type(expr, type_env, arguments)?;
                if vector_collection_is_vec1(&output)? {
                    min += 1;
                }
            }
            VectorItem::Spread(expr) => {
                let output = parser_output_type(expr, type_env, arguments)?;
                if vector_collection_is_vec1(&output)? {
                    min += 1;
                }
            }
            VectorItem::ZeroOrMore(_)
            | VectorItem::ZeroOrMoreSpread(_)
            | VectorItem::Assert { .. } => {}
        }
    }
    Some(min)
}

fn vector_collection_element_type(output: &TokenStream2) -> Option<TokenStream2> {
    let ty = syn::parse2::<Type>(output.clone()).ok()?;
    match ty {
        Type::Path(path) => {
            let segment = path.path.segments.last()?;
            if segment.ident != "Vec" && segment.ident != "Vec1" {
                return None;
            }
            let PathArguments::AngleBracketed(args) = &segment.arguments else {
                return None;
            };
            args.args.iter().find_map(|arg| match arg {
                GenericArgument::Type(ty) => Some(quote!(#ty)),
                _ => None,
            })
        }
        _ => None,
    }
}

fn vector_collection_is_vec1(output: &TokenStream2) -> Option<bool> {
    let ty = syn::parse2::<Type>(output.clone()).ok()?;
    match ty {
        Type::Path(path) => {
            let segment = path.path.segments.last()?;
            if segment.ident == "Vec1" {
                Some(true)
            } else if segment.ident == "Vec" {
                Some(false)
            } else {
                None
            }
        }
        _ => None,
    }
}

fn rust_parser_output_type(
    expr: &Expr,
    type_env: &GrammarTypeEnv,
    arguments: &BTreeMap<String, Type>,
) -> Option<TokenStream2> {
    match expr {
        Expr::Call(call) => call_rust_parser_output_type(call, type_env, arguments),
        Expr::MethodCall(method) => method_rust_parser_output_type(method, type_env, arguments),
        Expr::Path(path) => path_rust_parser_output_type(path, type_env, arguments),
        Expr::Tuple(tuple) => tuple_rust_parser_output_type(tuple, type_env, arguments),
        Expr::Array(array) => {
            vector_parser_output_type(&array_vector_expr(array)?, type_env, arguments)
        }
        _ => None,
    }
}

fn method_rust_parser_output_type(
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
        rust_parser_output_type(&method.receiver, type_env, arguments)
    } else if method.method == "not" || method.method == "ignored" {
        Some(quote!(()))
    } else if method.method == "ignore_then" && method.args.len() == 1 {
        rust_parser_output_type(
            method.args.first().expect("length checked"),
            type_env,
            arguments,
        )
    } else if (method.method == "wf"
        || method.method == "with_free_modifiers"
        || method.method == "prohibited_wf")
        && method.args.is_empty()
    {
        let inner = rust_parser_output_type(&method.receiver, type_env, arguments)?;
        Some(quote!(WithFreeModifiers<#inner, FreeModifierSyntax>))
    } else {
        None
    }
}

fn call_rust_parser_output_type(
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
        ("word_not_cmavo", _) if !call.args.is_empty() => Some(quote!(Token)),
        ("feature" | "policy", 1) => Some(quote!(())),
        ("opt", 1) => {
            let inner = rust_parser_output_type(
                call.args.first().expect("length checked"),
                type_env,
                arguments,
            )?;
            Some(quote!(Option<#inner>))
        }
        ("boxed", 1) => {
            let inner = rust_parser_output_type(
                call.args.first().expect("length checked"),
                type_env,
                arguments,
            )?;
            Some(quote!(Box<#inner>))
        }
        ("arc", 1) => {
            let inner = rust_parser_output_type(
                call.args.first().expect("length checked"),
                type_env,
                arguments,
            )?;
            Some(quote!(std::sync::Arc<#inner>))
        }
        ("feature" | "policy", 2) => rust_parser_output_type(
            call.args.iter().nth(1).expect("length checked"),
            type_env,
            arguments,
        ),
        ("choice", 1) => choice_output_type(
            call.args.first().expect("length checked"),
            type_env,
            arguments,
        ),
        ("choice", _) => choice_outputs_same(call.args.iter(), type_env, arguments),
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
        rust_parser_output_type(expr, type_env, arguments)
    }
}

fn choice_outputs_same<'a>(
    exprs: impl Iterator<Item = &'a Expr>,
    type_env: &GrammarTypeEnv,
    arguments: &BTreeMap<String, Type>,
) -> Option<TokenStream2> {
    let outputs = exprs
        .map(|expr| rust_parser_output_type(expr, type_env, arguments))
        .collect::<Option<Vec<_>>>()?;
    common_choice_output_type(&outputs)
}

fn common_choice_output_type(outputs: &[TokenStream2]) -> Option<TokenStream2> {
    let first = outputs.first()?;
    if outputs
        .iter()
        .all(|output| type_token_streams_match(output, first))
    {
        return Some(first.clone());
    }

    let mut element: Option<TokenStream2> = None;
    let mut saw_vec = false;
    for output in outputs {
        let item_element = vector_collection_element_type(output)?;
        if let Some(element) = &element {
            if !type_token_streams_match(element, &item_element) {
                return None;
            }
        } else {
            element = Some(item_element);
        }
        if !vector_collection_is_vec1(output)? {
            saw_vec = true;
        }
    }

    let element = element?;
    if saw_vec {
        Some(quote!(Vec<#element>))
    } else {
        Some(quote!(vec1::Vec1<#element>))
    }
}

fn coerce_choice_parser_output(
    parser: TokenStream2,
    source: &TokenStream2,
    target: &TokenStream2,
) -> Option<TokenStream2> {
    if type_token_streams_match(source, target) {
        return Some(parser);
    }

    let source_element = vector_collection_element_type(source)?;
    let target_element = vector_collection_element_type(target)?;
    if type_token_streams_match(&source_element, &target_element)
        && vector_collection_is_vec1(source)?
        && !vector_collection_is_vec1(target)?
    {
        return Some(quote! {
            #parser.map(|__items| __items.into_iter().collect::<Vec<_>>())
        });
    }

    None
}

fn sequence_output_type<'a>(
    exprs: impl Iterator<Item = &'a Expr>,
    type_env: &GrammarTypeEnv,
    arguments: &BTreeMap<String, Type>,
) -> Option<TokenStream2> {
    let mut outputs = exprs
        .map(|expr| rust_parser_output_type(expr, type_env, arguments))
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

fn tuple_rust_parser_output_type(
    tuple: &ExprTuple,
    type_env: &GrammarTypeEnv,
    arguments: &BTreeMap<String, Type>,
) -> Option<TokenStream2> {
    sequence_output_type(tuple.elems.iter(), type_env, arguments)
}

fn path_rust_parser_output_type(
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
    let segment = path.path.segments.first()?;
    if !matches!(segment.arguments, PathArguments::None) {
        return None;
    }
    Some(&segment.ident)
}

fn parser_type_tokens(
    output: &Type,
    generate_model: bool,
    model_outputs: &Option<BTreeSet<String>>,
    model_path: Option<&Path>,
) -> TokenStream2 {
    if output_is_generated_model(generate_model, model_outputs, output)
        && let Some(output_ident) = simple_type_ident(output)
    {
        model_path.map_or_else(
            || quote!(self::#output_ident),
            |model_path| quote!(#model_path::#output_ident),
        )
    } else {
        quote!(#output)
    }
}

fn output_is_generated_model(
    generate_model: bool,
    model_outputs: &Option<BTreeSet<String>>,
    output: &Type,
) -> bool {
    if !generate_model {
        return false;
    }
    type_mentions_generated_model(output, model_outputs)
}

fn type_mentions_generated_model(ty: &Type, model_outputs: &Option<BTreeSet<String>>) -> bool {
    match ty {
        Type::Path(path) => {
            if path.qself.is_none()
                && path.path.segments.len() == 1
                && model_outputs.as_ref().is_none_or(|outputs| {
                    outputs.contains(&path.path.segments[0].ident.to_string())
                })
            {
                return true;
            }
            path.path.segments.iter().any(|segment| {
                let PathArguments::AngleBracketed(args) = &segment.arguments else {
                    return false;
                };
                args.args.iter().any(|argument| match argument {
                    GenericArgument::Type(ty) => type_mentions_generated_model(ty, model_outputs),
                    GenericArgument::AssocType(assoc) => {
                        type_mentions_generated_model(&assoc.ty, model_outputs)
                    }
                    _ => false,
                })
            })
        }
        Type::Tuple(tuple) => tuple
            .elems
            .iter()
            .any(|ty| type_mentions_generated_model(ty, model_outputs)),
        Type::Paren(paren) => type_mentions_generated_model(&paren.elem, model_outputs),
        Type::Group(group) => type_mentions_generated_model(&group.elem, model_outputs),
        Type::Reference(reference) => type_mentions_generated_model(&reference.elem, model_outputs),
        _ => false,
    }
}

fn is_path_type(output: &Type) -> bool {
    matches!(output, Type::Path(_))
}

fn is_unit_type(output: &Type) -> bool {
    matches!(output, Type::Tuple(tuple) if tuple.elems.is_empty())
}

fn parse_explicit_rule(input: ParseStream<'_>) -> Result<Rule> {
    input.parse::<kw::rule>()?;
    let context: LitStr = input.parse()?;
    let name: Ident = input.parse()?;
    let arguments = parse_optional_arguments(input)?;
    input.parse::<Token![->]>()?;
    if input.peek(Token![struct]) {
        input.parse::<Token![struct]>()?;
        let content;
        braced!(content in input);
        let fields = parse_explicit_struct_fields(&content)?;
        Ok(Rule::Struct(NodeRule {
            output: syntax_type_for_rule(&name),
            name,
            arguments,
            context: Some(context),
            fields,
        }))
    } else if input.peek(Token![enum]) {
        input.parse::<Token![enum]>()?;
        let content;
        braced!(content in input);
        let mut branches = Vec::new();
        while !content.is_empty() {
            let attrs = content.call(Attribute::parse_outer)?;
            let mut conditions = Vec::new();
            while content.peek(kw::when) {
                conditions.push(content.parse()?);
            }
            let name = content.parse()?;
            branches.push(EnumBranch {
                attrs,
                conditions,
                name,
            });
            if content.peek(Token![,]) {
                content.parse::<Token![,]>()?;
            } else if content.peek(Token![;]) {
                content.parse::<Token![;]>()?;
            } else if !content.is_empty() {
                return Err(content.error("expected `,` or `;` between enum branches"));
            }
        }
        if branches.is_empty() {
            return Err(content.error("enum rules need at least one branch"));
        }
        Ok(Rule::Enum(EnumRule {
            output: syntax_type_for_rule(&name),
            name,
            arguments,
            context,
            branches,
        }))
    } else {
        Err(input.error("expected `struct` or `enum` after `->`"))
    }
}

fn parse_explicit_struct_fields(input: ParseStream<'_>) -> Result<Vec<FieldItem>> {
    let mut fields = Vec::new();
    while !input.is_empty() {
        fields.push(parse_explicit_struct_field(input)?);
    }
    Ok(fields)
}

fn parse_explicit_struct_field(input: ParseStream<'_>) -> Result<FieldItem> {
    let attrs = input.call(Attribute::parse_outer)?;
    let mut conditions = Vec::new();
    while input.peek(kw::when) {
        conditions.push(input.parse()?);
    }

    if input.peek(kw::field) {
        input.parse::<kw::field>()?;
        let name = input.parse()?;
        let ty = if input.peek(Token![:]) {
            input.parse::<Token![:]>()?;
            Some(input.parse()?)
        } else {
            None
        };
        let kind = if input.peek(Token![<]) {
            input.parse::<Token![<]>()?;
            input.parse::<Token![-]>()?;
            FieldKind::Field
        } else if input.peek(Token![=]) {
            input.parse::<Token![=]>()?;
            FieldKind::Computed
        } else {
            return Err(input.error("expected `<-` for a parser field or `=` for a computed field"));
        };
        let parser = if matches!(kind, FieldKind::Field) {
            input.parse()?
        } else {
            input.parse::<Expr>()?.into()
        };
        input.parse::<Token![;]>()?;
        Ok(FieldItem {
            attrs,
            conditions,
            kind,
            name: Some(name),
            ty,
            parser,
        })
    } else if input.peek(Token![let]) {
        input.parse::<Token![let]>()?;
        let name = input.parse()?;
        let ty = if input.peek(Token![:]) {
            input.parse::<Token![:]>()?;
            Some(input.parse()?)
        } else {
            None
        };
        input.parse::<Token![=]>()?;
        let parser = input.parse::<Expr>()?.into();
        input.parse::<Token![;]>()?;
        Ok(FieldItem {
            attrs,
            conditions,
            kind: FieldKind::TempLet,
            name: Some(name),
            ty,
            parser,
        })
    } else if input.peek(kw::assert) {
        input.parse::<kw::assert>()?;
        let negated = input.peek(Token![!]);
        if negated {
            input.parse::<Token![!]>()?;
        }
        let parser: ParserExpr = input.parse()?;
        input.parse::<Token![;]>()?;
        let parser = if negated {
            parser.postfix("not", Vec::new())
        } else {
            parser
                .postfix("lookahead", Vec::new())
                .postfix("ignored", Vec::new())
        };
        Ok(FieldItem {
            attrs,
            conditions,
            kind: FieldKind::Require,
            name: None,
            ty: None,
            parser,
        })
    } else {
        Err(input.error("expected `field`, `let`, or `assert`"))
    }
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

struct FieldItem {
    attrs: Vec<Attribute>,
    conditions: Vec<Condition>,
    kind: FieldKind,
    name: Option<Ident>,
    ty: Option<Type>,
    parser: ParserExpr,
}

impl FieldItem {
    fn expand(&self, arguments: &BTreeSet<String>) -> TokenStream2 {
        let kind = self.kind.as_str();
        let name = self
            .name
            .as_ref()
            .map_or_else(String::new, Ident::to_string);
        let parser = self.parser.compact_tokens();
        let recovery = classify_parser_expr(&self.parser, arguments).expand();
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
        let name = self.name.clone().ok_or_else(|| {
            syn::Error::new_spanned(self.parser.to_token_stream(), "model fields need a name")
        })?;
        let ty = match (&self.ty, &self.kind) {
            (Some(ty), _) => quote!(#ty),
            (None, FieldKind::Field) => {
                parser_output_type(&self.parser, type_env, argument_types).ok_or_else(|| {
                    syn::Error::new_spanned(
                        self.parser.to_token_stream(),
                        "cannot infer generated model field type from parser expression; add an explicit `: Type` annotation",
                    )
                })?
            }
            (None, FieldKind::Computed) => {
                return Err(syn::Error::new_spanned(
                    self.parser.to_token_stream(),
                    "computed generated model fields require an explicit `: Type` annotation",
                ));
            }
            (None, FieldKind::TempLet | FieldKind::Require) => {
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

enum FieldKind {
    Field,
    Computed,
    TempLet,
    Require,
}

impl FieldKind {
    fn as_str(&self) -> &'static str {
        match self {
            FieldKind::Field => "field",
            FieldKind::Computed => "field",
            FieldKind::TempLet => "let",
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

    fn expand_strict_gate(&self, parser: TokenStream2) -> TokenStream2 {
        let name = &self.name;
        match self.kind {
            ConditionKind::Feature => quote! {
                generated_runtime::feature_gate(
                    generated_runtime::SyntaxGrammarFeature::#name,
                    #parser,
                )
            },
            ConditionKind::Policy => quote! {
                generated_runtime::policy_gate(
                    generated_runtime::SyntaxGrammarPolicyFlag::#name,
                    #parser,
                )
            },
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

fn classify_parser_expr(expr: &ParserExpr, arguments: &BTreeSet<String>) -> RecoveryExpr {
    match expr {
        ParserExpr::Rust(expr) => classify_recovery_expr(expr, arguments),
        ParserExpr::Vector(expr) => RecoveryExpr::Sequence(
            expr.items
                .iter()
                .map(|item| match item {
                    VectorItem::One(expr) | VectorItem::Spread(expr) => {
                        classify_parser_expr(expr, arguments)
                    }
                    VectorItem::ZeroOrMore(expr) | VectorItem::ZeroOrMoreSpread(expr) => {
                        RecoveryExpr::Many(Box::new(classify_parser_expr(expr, arguments)))
                    }
                    VectorItem::OneOrMore(expr) | VectorItem::OneOrMoreSpread(expr) => {
                        RecoveryExpr::Many1(Box::new(classify_parser_expr(expr, arguments)))
                    }
                    VectorItem::Assert { negated, parser } => {
                        let inner = classify_parser_expr(parser, arguments);
                        if *negated {
                            RecoveryExpr::Not(Box::new(inner))
                        } else {
                            RecoveryExpr::Lookahead(Box::new(inner))
                        }
                    }
                })
                .collect(),
        ),
        ParserExpr::Chain(expr) => {
            let links = match expr.links_kind {
                ChainLinksKind::ZeroOrMore => {
                    RecoveryExpr::Many(Box::new(classify_parser_expr(&expr.links, arguments)))
                }
                ChainLinksKind::OneOrMore => {
                    RecoveryExpr::Many1(Box::new(classify_parser_expr(&expr.links, arguments)))
                }
            };
            RecoveryExpr::Sequence(vec![classify_parser_expr(&expr.first, arguments), links])
        }
        ParserExpr::Postfix {
            receiver,
            method,
            args,
        } => classify_postfix_recovery_expr(receiver, method, args, arguments),
    }
}

fn classify_postfix_recovery_expr(
    receiver: &ParserExpr,
    method: &Ident,
    args: &[Expr],
    arguments: &BTreeSet<String>,
) -> RecoveryExpr {
    let inner = || Box::new(classify_parser_expr(receiver, arguments));
    match (method.to_string().as_str(), args.len()) {
        ("wf", 0) | ("with_free_modifiers", 0) => RecoveryExpr::WithFreeModifiers(inner()),
        ("ignored", 0) => RecoveryExpr::Ignored(inner()),
        ("not", 0) => RecoveryExpr::Not(inner()),
        ("lookahead", 0) => RecoveryExpr::Lookahead(inner()),
        ("ignore_then", 1) => RecoveryExpr::Sequence(vec![
            classify_parser_expr(receiver, arguments),
            classify_recovery_expr(args.first().expect("length checked"), arguments),
        ]),
        _ => {
            let receiver = receiver.to_token_stream();
            RecoveryExpr::Opaque(compact_tokens(quote!(#receiver.#method(#(#args),*))))
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
        Expr::Array(array) => array_vector_expr(array)
            .map(|expr| classify_parser_expr(&ParserExpr::Vector(expr), arguments))
            .unwrap_or_else(|| RecoveryExpr::Opaque(compact_tokens(expr))),
        _ => RecoveryExpr::Opaque(compact_tokens(expr)),
    }
}

fn array_vector_expr(array: &ExprArray) -> Option<VectorExpr> {
    if array.elems.is_empty() {
        return None;
    }
    Some(VectorExpr {
        items: array
            .elems
            .iter()
            .cloned()
            .map(ParserExpr::Rust)
            .map(VectorItem::One)
            .collect(),
    })
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
        ("ignore_then", 1) => RecoveryExpr::Sequence(vec![
            classify_recovery_expr(&method.receiver, arguments),
            classify_recovery_expr(method.args.first().expect("length checked"), arguments),
        ]),
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
        ("feature" | "policy", 1) => RecoveryExpr::Opaque(compact_tokens(call)),
        ("boxed", 1) => {
            RecoveryExpr::Boxed(Box::new(classify_recovery_expr(&call.args[0], arguments)))
        }
        ("arc", 1) => RecoveryExpr::Arc(Box::new(classify_recovery_expr(&call.args[0], arguments))),
        ("choice", 1) => RecoveryExpr::Choice(
            call.args
                .first()
                .map(choice_alternative_exprs)
                .unwrap_or_default()
                .into_iter()
                .map(|expr| classify_recovery_expr(expr, arguments))
                .collect(),
        ),
        ("choice", _) => RecoveryExpr::Choice(
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
    if arguments.contains(&text) || (path.qself.is_none() && path.path.segments.len() == 1) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;

    #[test]
    fn grammar_rejects_build_blocks() {
        let result = syn::parse2::<SyntaxGrammar>(quote! {
            rule "item" item -> struct {
                field token <- cmavo(Be);
                build |token| ItemSyntax { token };
            }
        });

        let error = match result {
            Ok(_) => panic!("build blocks must be rejected"),
            Err(error) => error,
        };
        assert!(
            error
                .to_string()
                .contains("expected `field`, `let`, or `assert`"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn grammar_does_not_support_recovered_build_blocks() {
        let result = syn::parse2::<SyntaxGrammar>(quote! {
            rule "item" item -> struct {
                field token <- cmavo(Be);
                recovered_build |token| ItemSyntax { token };
            }
        });

        assert!(
            result.is_err(),
            "recovered_build blocks must be unsupported"
        );
    }

    #[test]
    fn grammar_rejects_old_node_and_product_rules() {
        let node_result = syn::parse2::<SyntaxGrammar>(quote! {
            node item -> ItemSyntax {
                fields {
                    field token = cmavo(Be);
                }
            }
        });
        let product_result = syn::parse2::<SyntaxGrammar>(quote! {
            product item -> ItemSyntax {
                fields {
                    field token = cmavo(Be);
                }
            }
        });

        for result in [node_result, product_result] {
            let error = match result {
                Ok(_) => panic!("old type-bearing rules must be rejected"),
                Err(error) => error,
            };
            assert!(
                error
                    .to_string()
                    .contains("expected `recursive`, `alias`, or `rule`"),
                "unexpected error: {error}"
            );
        }
    }

    #[test]
    fn grammar_rejects_old_struct_body_forms() {
        let old_fields = syn::parse2::<SyntaxGrammar>(quote! {
            rule "item" item -> struct {
                fields {
                    field token = cmavo(Be);
                }
            }
        });
        let old_default = syn::parse2::<SyntaxGrammar>(quote! {
            rule "item" item -> struct {
                default token = cmavo(Be);
            }
        });
        let old_scratch = syn::parse2::<SyntaxGrammar>(quote! {
            rule "item" item -> struct {
                scratch token = cmavo(Be);
            }
        });
        let old_construct = syn::parse2::<SyntaxGrammar>(quote! {
            rule "item" item -> struct {
                construct variant Item;
                field token <- cmavo(Be);
            }
        });

        for result in [old_fields, old_default, old_scratch, old_construct] {
            let error = match result {
                Ok(_) => panic!("old struct-body form must be rejected"),
                Err(error) => error,
            };
            assert!(
                error
                    .to_string()
                    .contains("expected `field`, `let`, or `assert`"),
                "unexpected error: {error}"
            );
        }
    }

    #[test]
    fn grammar_rejects_old_typed_alias_rules() {
        let result = syn::parse2::<SyntaxGrammar>(quote! {
            alias item_alias(item) -> ItemSyntax {
                item;
            }
        });

        let error = match result {
            Ok(_) => panic!("old typed alias rules must be rejected"),
            Err(error) => error,
        };
        assert!(
            error
                .to_string()
                .contains("alias rules must use `alias \"context\" name = parser;`"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn grammar_rejects_alias_bodies() {
        let result = syn::parse2::<SyntaxGrammar>(quote! {
            alias "item" item_alias {
                assert !cmavo(Bo);
                item;
            }
        });

        let error = match result {
            Ok(_) => panic!("alias body rules must be rejected"),
            Err(error) => error,
        };
        assert!(
            error.to_string().contains("alias rules must use `=`"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn grammar_rejects_duplicate_recursive_blocks() {
        let result = syn::parse2::<SyntaxGrammar>(quote! {
            recursive {
                item: ItemSyntax;
            }

            recursive {
                other_item: OtherItemSyntax;
            }
        });

        let error = match result {
            Ok(_) => panic!("duplicate recursive blocks must be rejected"),
            Err(error) => error,
        };
        assert!(
            error.to_string().contains("duplicate `recursive` block"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn grammar_rejects_duplicate_recursive_names() {
        let result = syn::parse2::<SyntaxGrammar>(quote! {
            recursive {
                item: ItemSyntax;
                item: OtherItemSyntax;
            }
        });

        let error = match result {
            Ok(_) => panic!("duplicate recursive declarations must be rejected"),
            Err(error) => error,
        };
        assert!(
            error
                .to_string()
                .contains("duplicate recursive rule declaration"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn grammar_rejects_duplicate_rule_names() {
        let result = syn::parse2::<SyntaxGrammar>(quote! {
            rule "item" item -> struct {
                field token <- cmavo(Be);
            }

            alias "item alias" item = cmavo(Bo);
        });

        let error = match result {
            Ok(_) => panic!("duplicate rule declarations must be rejected"),
            Err(error) => error,
        };
        assert!(
            error
                .to_string()
                .contains("duplicate grammar rule declaration"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn grammar_accepts_vector_parser_method_suffixes() {
        let grammar = syn::parse2::<SyntaxGrammar>(quote! {
            env SyntaxGrammarEnv;

            rule "item" item -> struct {
                field tokens <- [one_or_more cmavo(Be)].wf();
            }
        })
        .expect("vector parser method suffix parses");

        let expanded = grammar.expand().to_string();
        assert!(
            expanded.contains("SyntaxGrammarRecoveryExpr :: WithFreeModifiers"),
            "vector `.wf()` should be represented as a normal parser suffix: {expanded}"
        );
    }

    #[test]
    fn generated_chain_requires_link_element_field() {
        let grammar = syn::parse2::<SyntaxGrammar>(quote! {
            tree_model {}
            model;

            rule "item" item -> struct {
                field token <- cmavo(Be);
            }

            rule "link" link -> struct {
                field connector <- cmavo(Bo);
                field item <- item;
            }

            rule "chain" chain -> struct {
                field run <- chain(first: item, zero_or_more: link, element: missing);
            }
        })
        .expect("grammar tokens parse before model expansion");

        let expanded = grammar.expand().to_string();
        assert!(
            expanded.contains("compile_error")
                && expanded.contains("cannot infer generated model field type"),
            "bad chain element fields should fail during generated model expansion: {expanded}"
        );
    }

    #[test]
    fn enum_branch_prefers_same_named_parser_argument_over_rule() {
        let grammar = syn::parse2::<SyntaxGrammar>(quote! {
            tree_model {}
            model;
            env generated_runtime::SyntaxGrammarEnv;
            strict_parsers;

            recursive {
                item: ItemSyntax;
            }

            rule "item" item(item) -> struct {
                field inner <- item;
            }

            rule "wrapper" wrapper(item) -> enum {
                item,
            }
        })
        .expect("grammar parses before expansion");

        let expanded = grammar.expand().to_string();
        assert!(
            expanded.contains("item . clone () . map"),
            "enum branch should wrap the parser argument: {expanded}"
        );
        assert!(
            expanded.contains("WrapperSyntax :: Item (item)"),
            "enum branch should construct the wrapper from the parser argument: {expanded}"
        );
    }

    #[test]
    fn generated_single_field_structs_are_newtypes() {
        let grammar = syn::parse2::<SyntaxGrammar>(quote! {
            tree_model {}
            model;
            env generated_runtime::SyntaxGrammarEnv;
            strict_parsers;

            rule "item" item -> struct {
                field token <- cmavo(Be);
            }
        })
        .expect("grammar parses before expansion");

        let expanded = grammar.expand().to_string();
        assert!(
            expanded.contains("pub struct ItemSyntax (pub Token)"),
            "single-field generated structs should be model newtypes: {expanded}"
        );
        assert!(
            expanded.contains("ItemSyntax (token)"),
            "single-field generated struct parser should construct a newtype: {expanded}"
        );
    }

    #[test]
    fn grammar_rejects_duplicate_generated_enum_variants() {
        let grammar = syn::parse2::<SyntaxGrammar>(quote! {
            tree_model {}
            model;

            rule "item" item -> struct {
                field token <- cmavo(Be);
            }

            rule "choice" choice -> enum {
                item,
                item,
            }
        })
        .expect("grammar parses before generated model expansion");

        let expanded = grammar.expand().to_string();
        assert!(
            expanded.contains("cannot generate enum variant")
                && expanded.contains("generated model ownership must be one rule per enum variant"),
            "unexpected expansion: {expanded}"
        );
    }
}
