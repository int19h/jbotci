//! Proc macros for syntax grammar declarations.

use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
};

#[allow(unused_imports)]
use bityzba::{data, ensures, invariant, new, requires};
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{ToTokens, format_ident, quote};
use syn::{
    Attribute, Expr, ExprArray, ExprCall, ExprMethodCall, ExprPath, ExprTuple, GenericArgument,
    Ident, LitStr, Path, PathArguments, Result, Token, Type, braced, bracketed, parenthesized,
    parse::{Parse, ParseStream},
    parse_macro_input, parse_quote,
    punctuated::Punctuated,
};

#[requires(true)]
#[ensures(true)]
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
    syn::custom_keyword!(recursive);
    syn::custom_keyword!(rule);
    syn::custom_keyword!(strict_parsers);
    syn::custom_keyword!(tree_model);
    syn::custom_keyword!(when);
    syn::custom_keyword!(chain);
    syn::custom_keyword!(one_or_more);
    syn::custom_keyword!(zero_or_more);
}

#[invariant(true)]
struct SyntaxGrammar {
    tree_model: Option<syn::File>,
    generate_model: bool,
    model_outputs: Option<BTreeSet<String>>,
    model_path: Option<Path>,
    env: Option<Type>,
    generate_parsers: bool,
    recursive: Vec<RecursiveRule>,
    rules: Vec<Rule>,
}

impl SyntaxGrammar {
    #[requires(true)]
    #[ensures(true)]
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
        let recovered_module = quote!(self::recovered);
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
        let anchor_metadata = match expand_recovery_anchor_metadata(&self.rules, &type_env) {
            Ok(metadata) => metadata,
            Err(error) => return error.into_compile_error(),
        };
        let rule_lookup_arms = self.rules.iter().enumerate().map(|(index, rule)| {
            let name = rule.name().to_string();
            quote!(#name => Some(&SYNTAX_GRAMMAR_RULES[#index]))
        });
        let parser_functions = if self.generate_parsers {
            match self
                .rules
                .iter()
                .filter(|rule| {
                    !self.generate_model
                        || rule
                            .output(&type_env)
                            .is_some_and(|output| self.rule_has_local_parser(output))
                })
                .map(|rule| {
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
                .collect::<Result<Vec<_>>>()
            {
                Ok(functions) => functions,
                Err(error) => return error.into_compile_error(),
            }
        } else {
            Vec::new()
        };
        let recovered_parser_functions = if self.generate_parsers && self.generate_model {
            match self
                .rules
                .iter()
                .filter(|rule| {
                    !self.generate_model
                        || rule
                            .output(&type_env)
                            .is_some_and(|output| self.rule_has_local_parser(output))
                })
                .map(|rule| {
                    rule.expand_recovered_parser(
                        &type_env,
                        &model_outputs,
                        model_all_rules_local,
                        self.model_path.as_ref(),
                        &recovered_module,
                        rule.output(&type_env)
                            .is_some_and(|output| self.generates_model_output(output)),
                    )
                })
                .collect::<Result<Vec<_>>>()
            {
                Ok(functions) => functions,
                Err(error) => return error.into_compile_error(),
            }
        } else {
            Vec::new()
        };
        let recursive_family = if self.generate_parsers {
            match self.expand_strict_recursive_family() {
                Ok(family) => family,
                Err(error) => return error.into_compile_error(),
            }
        } else {
            None
        };
        let recovered_recursive_family = if self.generate_parsers && self.generate_model {
            match self.expand_recovered_recursive_family(&recovered_module) {
                Ok(family) => family,
                Err(error) => return error.into_compile_error(),
            }
        } else {
            None
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

            #[derive(Debug, Clone, Copy, PartialEq, Eq)]
            pub(crate) enum SyntaxGrammarAnchorToken {
                Cmavo(Cmavo),
                Selmaho(Selmaho),
            }

            #[derive(Debug, Clone, Copy, PartialEq, Eq)]
            pub(crate) enum SyntaxGrammarAnchorOrigin {
                LiteralRun,
                RepetitionElementFirst,
                FieldFirst,
            }

            #[derive(Debug, Clone, Copy, PartialEq, Eq)]
            pub(crate) struct SyntaxGrammarAnchorTokenSet {
                pub tokens: &'static [SyntaxGrammarAnchorToken],
                pub conditions: &'static [SyntaxGrammarCondition],
            }

            #[derive(Debug, Clone, Copy, PartialEq, Eq)]
            pub(crate) struct SyntaxGrammarAnchorRun {
                pub start_tokens: &'static [SyntaxGrammarAnchorToken],
                pub resume_field: usize,
                pub origin: SyntaxGrammarAnchorOrigin,
                pub conditions: &'static [SyntaxGrammarCondition],
            }

            #[derive(Debug, Clone, Copy, PartialEq, Eq)]
            pub(crate) struct SyntaxGrammarFieldAnchors {
                pub field_index: usize,
                pub field_name: &'static str,
                pub anchors: &'static [SyntaxGrammarAnchorRun],
            }

            #[derive(Debug, Clone, Copy, PartialEq, Eq)]
            pub(crate) struct SyntaxGrammarRuleAnchorMetadata {
                pub rule: &'static str,
                pub first: &'static [SyntaxGrammarAnchorTokenSet],
                pub fields: &'static [SyntaxGrammarFieldAnchors],
            }

            #[derive(Debug, Clone, Copy, PartialEq, Eq)]
            pub(crate) struct SyntaxGrammarSubtextContainer {
                pub rule: &'static str,
                pub opener_field: usize,
                pub opener_tokens: &'static [SyntaxGrammarAnchorToken],
                pub text_field: usize,
                pub closer_field: usize,
                pub closer_tokens: &'static [SyntaxGrammarAnchorToken],
            }

            pub(crate) const SYNTAX_GRAMMAR_ENV: &str = #env;
            #(#parser_functions)*
            #(#recovered_parser_functions)*
            #recursive_family
            #recovered_recursive_family

            pub(crate) const SYNTAX_GRAMMAR_RECURSIVE_RULES: &[SyntaxGrammarRecursiveRule] = &[
                #(#recursive),*
            ];
            pub(crate) const SYNTAX_GRAMMAR_RULES: &[SyntaxGrammarRule] = &[
                #(#rules),*
            ];
            #anchor_metadata

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

        let generate_parsers = if env.is_some() && input.peek(kw::strict_parsers) {
            input.parse::<kw::strict_parsers>()?;
            input.parse::<Token![;]>()?;
            true
        } else {
            false
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
            generate_parsers,
            recursive,
            rules,
        })
    }
}

#[requires(true)]
#[ensures(true)]
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

#[requires(true)]
#[ensures(true)]
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

#[requires(true)]
#[ensures(true)]
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

#[requires(true)]
#[ensures(ret.is_err() || ret.as_ref().is_ok_and(|outputs| !outputs.is_empty()))]
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

#[requires(true)]
#[ensures(true)]
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
    #[requires(true)]
    #[ensures(true)]
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

    #[requires(true)]
    #[ensures(true)]
    fn generates_model_output(&self, output: &Type) -> bool {
        output_is_generated_model(self.generate_model, &self.resolved_model_outputs(), output)
    }

    #[requires(true)]
    #[ensures(true)]
    fn generates_model_output_name(&self, output: &str) -> bool {
        self.generate_model
            && self
                .resolved_model_outputs()
                .as_ref()
                .is_none_or(|outputs| outputs.contains(output))
    }

    #[requires(true)]
    #[ensures(true)]
    fn rule_has_local_parser(&self, output: &Type) -> bool {
        !self.generate_model || self.model_outputs.is_none() || self.generates_model_output(output)
    }

    #[requires(true)]
    #[ensures(true)]
    fn parser_type_tokens(&self, output: &Type) -> TokenStream2 {
        parser_type_tokens(
            output,
            self.generate_model,
            &self.resolved_model_outputs(),
            self.model_path.as_ref(),
        )
    }

    #[requires(true)]
    #[ensures(true)]
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

    #[requires(true)]
    #[ensures(true)]
    fn generated_tree_model_items(&self, type_env: &GrammarTypeEnv) -> Result<GeneratedTreeModel> {
        let mut structs = BTreeMap::<String, GeneratedStructModel>::new();
        let mut enums = BTreeMap::<String, Vec<GeneratedVariantModel>>::new();
        let mut transparent_constructors = BTreeSet::<String>::new();
        let mut transparent_field_pairs = BTreeSet::<(String, String)>::new();
        let mut chain_link_element_fields = BTreeSet::<(String, String)>::new();
        let mut variant_struct_outputs = BTreeSet::<(String, String)>::new();
        let mut constructor_labels = BTreeMap::<String, String>::new();
        let mut elidable_terminator_fields = BTreeMap::<String, String>::new();
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
                    constructor_labels.insert(enum_constructor.clone(), rule.context.value());
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
                        constructor_labels.insert(
                            variant_constructor.clone(),
                            self.rule_context_label(&branch.name)
                                .unwrap_or_else(|| rule.context.value()),
                        );
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
                        let field = GeneratedFieldModel::from_data(data!(GeneratedFieldModel {
                            attrs: branch.attrs.clone(),
                            name: branch.name.clone(),
                            ty: quote!(#branch_output),
                        }));
                        push_generated_variant(
                            &mut enums,
                            output.to_string(),
                            GeneratedVariantModel::from_data(data!(GeneratedVariantModel {
                                variant,
                                rule_name: rule.name.clone(),
                                fields: vec![field],
                                tuple: true,
                            })),
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
            for (field, cmavo) in rule.generated_elidable_terminator_fields()? {
                if let Some(existing) =
                    elidable_terminator_fields.insert(field.clone(), cmavo.clone())
                    && existing != cmavo
                {
                    return Err(syn::Error::new_spanned(
                        &rule.name,
                        format!(
                            "field `{field}` is annotated with both `{existing}` and `{cmavo}` elidable terminators",
                        ),
                    ));
                }
            }
            let fields = rule.generated_model_fields(type_env)?;
            if fields.len() == 1 {
                let constructor = generated_constructor_name(&output);
                transparent_constructors.insert(constructor.clone());
                let field_name = fields[0].name.to_string();
                transparent_field_pairs.insert((constructor.clone(), field_name));
                transparent_field_pairs.insert((constructor.clone(), snake_case(&constructor)));
            }
            if let Some(context) = &rule.context {
                constructor_labels.insert(generated_constructor_name(&output), context.value());
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
        let constructor_label_items = constructor_labels
            .iter()
            .map(|(constructor, label)| quote!((#constructor, #label)));
        let elidable_terminator_items = elidable_terminator_fields.iter().map(|(field, cmavo)| {
            let cmavo = format_ident!("{cmavo}");
            quote!(GeneratedModelElidableTerminator {
                field: #field,
                cmavo: Cmavo::#cmavo,
            })
        });
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

            #[derive(Debug, Clone, Copy, PartialEq, Eq)]
            pub struct GeneratedModelElidableTerminator {
                pub field: &'static str,
                pub cmavo: Cmavo,
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
            pub const GENERATED_MODEL_CONSTRUCTOR_LABELS: &[(&str, &str)] = &[
                #(#constructor_label_items,)*
            ];

            #[doc(hidden)]
            pub const GENERATED_MODEL_ELIDABLE_TERMINATORS: &[GeneratedModelElidableTerminator] = &[
                #(#elidable_terminator_items,)*
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

    #[requires(true)]
    #[ensures(true)]
    fn rule_context_label(&self, name: &Ident) -> Option<String> {
        let name = name.to_string();
        self.rules
            .iter()
            .find(|rule| rule.name().to_string() == name)
            .and_then(Rule::context_label)
    }
}

#[invariant(true)]
struct GeneratedTreeModel {
    tree_items: Vec<TokenStream2>,
    support_items: Vec<TokenStream2>,
}

#[requires(true)]
#[ensures(true)]
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

#[requires(true)]
#[ensures(true)]
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

#[requires(true)]
#[ensures(true)]
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

#[requires(true)]
#[ensures(true)]
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

#[requires(true)]
#[ensures(true)]
fn token_streams_match(left: &TokenStream2, right: &TokenStream2) -> bool {
    left.to_string() == right.to_string()
}

#[requires(true)]
#[ensures(true)]
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

#[requires(true)]
#[ensures(true)]
fn canonical_type_key(ty: &Type) -> Option<String> {
    match ty {
        Type::Path(path) if path.qself.is_none() => canonical_path_key(&path.path),
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
            let inner = canonical_type_key(&reference.elem)?;
            let mut prefix = String::from("&");
            if let Some(lifetime) = &reference.lifetime {
                prefix.push_str(&lifetime.to_token_stream().to_string());
                prefix.push(' ');
            }
            if reference.mutability.is_some() {
                prefix.push_str("mut ");
            }
            Some(format!("{prefix}{inner}"))
        }
        _ => Some(ty.to_token_stream().to_string()),
    }
}

#[requires(true)]
#[ensures(true)]
fn canonical_path_key(path: &Path) -> Option<String> {
    let last_segment = path.segments.last()?;
    let is_vector_collection = last_segment.ident == "Vec" || last_segment.ident == "Vec1";
    if is_vector_collection {
        return canonical_path_arguments(&last_segment.arguments)
            .map(|args| format!("{}{}", last_segment.ident, args));
    }

    let segments = path
        .segments
        .iter()
        .map(|segment| {
            canonical_path_arguments(&segment.arguments)
                .map(|args| format!("{}{}", segment.ident, args))
        })
        .collect::<Option<Vec<_>>>()?
        .join("::");
    if path.leading_colon.is_some() {
        Some(format!("::{segments}"))
    } else {
        Some(segments)
    }
}

#[requires(true)]
#[ensures(true)]
fn canonical_path_arguments(arguments: &PathArguments) -> Option<String> {
    match arguments {
        PathArguments::None => Some(String::new()),
        PathArguments::AngleBracketed(arguments) => {
            let arguments = arguments
                .args
                .iter()
                .map(|argument| match argument {
                    GenericArgument::Type(ty) => canonical_type_key(ty),
                    _ => Some(argument.to_token_stream().to_string()),
                })
                .collect::<Option<Vec<_>>>()?
                .join(",");
            Some(format!("<{arguments}>"))
        }
        PathArguments::Parenthesized(arguments) => Some(arguments.to_token_stream().to_string()),
    }
}

#[requires(name.chars().any(|ch| ch != '_' && ch != '-' && ch != ' '))]
#[ensures(!ret.to_string().is_empty())]
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
    format_ident!("{out}")
}

#[requires(name.chars().any(|ch| ch != '_' && ch != '-' && ch != ' '))]
#[ensures(!ret.is_empty())]
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

#[requires(true)]
#[ensures(true)]
fn syntax_type_ident_for_rule(name: &Ident) -> Ident {
    let base = pascal_case_ident(&name.to_string());
    format_ident!("{base}Syntax")
}

#[requires(true)]
#[ensures(true)]
fn syntax_type_for_rule(name: &Ident) -> Type {
    let ident = syntax_type_ident_for_rule(name);
    parse_quote!(#ident)
}

#[requires(true)]
#[ensures(true)]
fn generated_constructor_name(output: &Ident) -> String {
    let output = output.to_string();
    output
        .strip_suffix("Syntax")
        .unwrap_or(output.as_str())
        .to_owned()
}

#[requires(true)]
#[ensures(true)]
fn enum_variant_ident_for_output(output: &Type, fallback: &Ident) -> Ident {
    let Some(output) = simple_type_ident(output) else {
        return pascal_case_ident(&fallback.to_string());
    };
    let output = output.to_string();
    let variant = output.strip_suffix("Syntax").unwrap_or(&output);
    format_ident!("{variant}")
}

#[invariant(true)]
struct GeneratedStructModel {
    visibility: TokenStream2,
    ident: Ident,
    rule_name: Ident,
    fields: Vec<GeneratedFieldModel>,
}

impl GeneratedStructModel {
    #[requires(true)]
    #[ensures(true)]
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

#[invariant(!fields.is_empty(), "generated enum variants carry at least one field")]
struct GeneratedVariantModel {
    variant: Ident,
    rule_name: Ident,
    fields: Vec<GeneratedFieldModel>,
    tuple: bool,
}

impl GeneratedVariantModel {
    #[requires(true)]
    #[ensures(true)]
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

#[invariant(!ty.is_empty(), "generated model field type tokens must not be empty")]
struct GeneratedFieldModel {
    attrs: Vec<Attribute>,
    name: Ident,
    ty: TokenStream2,
}

impl GeneratedFieldModel {
    #[requires(true)]
    #[ensures(true)]
    fn expand_named_struct(&self) -> TokenStream2 {
        let attrs = &self.attrs;
        let name = &self.name;
        let ty = &self.ty;
        quote!(#(#attrs)* pub #name: #ty)
    }

    #[requires(true)]
    #[ensures(true)]
    fn expand_variant_named(&self) -> TokenStream2 {
        let attrs = &self.attrs;
        let name = &self.name;
        let ty = &self.ty;
        quote!(#(#attrs)* #name: #ty)
    }

    #[requires(true)]
    #[ensures(true)]
    fn expand_tuple_struct(&self) -> TokenStream2 {
        let attrs = &self.attrs;
        let ty = &self.ty;
        quote!(#(#attrs)* pub #ty)
    }

    #[requires(true)]
    #[ensures(true)]
    fn expand_tuple_variant(&self) -> TokenStream2 {
        let attrs = &self.attrs;
        let ty = &self.ty;
        quote!(#(#attrs)* #ty)
    }
}

impl SyntaxGrammar {
    #[requires(true)]
    #[ensures(true)]
    fn expand_strict_recursive_family(&self) -> Result<Option<TokenStream2>> {
        if self.recursive.is_empty() {
            return Ok(None);
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
            return Ok(None);
        }
        let local_recursive_names = recursive_rules
            .iter()
            .map(|rule| rule.name.to_string())
            .collect::<BTreeSet<_>>();
        let family_ident = format_ident!("StrictGeneratedParserFamily");
        let fields = recursive_rules.iter().map(|rule| {
            let name = &rule.name;
            let output = self.parser_type_tokens(&rule.output);
            quote!(#name: BoxedParser<'tokens, generated_runtime::SharedSyntaxOutput<#output>>)
        });
        let declarations = recursive_rules.iter().map(|rule| {
            let name = &rule.name;
            quote!(let mut #name = __generated_recursive_family.declare();)
        });
        let definitions = recursive_rules
            .iter()
            .map(|recursive| {
                let rule = self
                    .rules
                    .iter()
                    .find(|rule| rule.name().to_string() == recursive.name.to_string())
                    .ok_or_else(|| {
                        syn::Error::new_spanned(
                            &recursive.name,
                            "recursive parser declaration has no matching rule",
                        )
                    })?;
                let parser_name = format_ident!("strict_{}_parser", rule.name());
                let parser_arguments = rule
                    .arguments()
                    .iter()
                    .map(|argument| {
                        let argument_name = argument.to_string();
                        if local_recursive_names.contains(&argument_name) {
                            Ok(quote!(#argument.clone().map(
                                generated_runtime::SharedSyntaxOutput::into_owned
                            )))
                        } else if all_recursive_names.contains(&argument_name) {
                            Ok(quote!(super::strict_generated_parser_family()
                                .#argument
                                .map(generated_runtime::SharedSyntaxOutput::into_owned)))
                        } else {
                            Err(syn::Error::new_spanned(
                                argument,
                                "recursive parser argument is not declared in the recursive block",
                            ))
                        }
                    })
                    .collect::<Result<Vec<_>>>()?;
                let hidden_free_modifier = if local_recursive_names.contains("free_modifier") {
                    let free_modifier = format_ident!("free_modifier");
                    quote!(#free_modifier.clone().map(
                        generated_runtime::SharedSyntaxOutput::into_owned
                    ).boxed())
                } else if all_recursive_names.contains("free_modifier") {
                    quote!(
                        super::strict_generated_parser_family()
                            .free_modifier
                            .map(generated_runtime::SharedSyntaxOutput::into_owned)
                            .boxed()
                    )
                } else {
                    quote!(generated_runtime::strict_empty_free_modifier_parser())
                };
                let name = &recursive.name;
                Ok(quote! {
                    #name.define(#parser_name(
                        #(#parser_arguments,)*
                        #hidden_free_modifier,
                    ));
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let outputs = recursive_rules.iter().map(|rule| {
            let name = &rule.name;
            quote!(#name: __generated_recursive_family.own(#name).boxed())
        });
        let root_functions = recursive_rules.iter().map(|rule| {
            let root_name = &rule.name;
            let function = format_ident!("strict_generated_{}_parser", root_name);
            let shared_function = format_ident!("strict_generated_{}_shared_parser", root_name);
            let output = self.parser_type_tokens(&rule.output);
            quote! {
                #[allow(dead_code, unused_variables)]
                pub(crate) fn #shared_function<'tokens>() -> BoxedParser<
                    'tokens,
                    generated_runtime::SharedSyntaxOutput<#output>,
                > {
                    strict_generated_parser_family().#root_name
                }

                #[allow(dead_code, unused_variables)]
                pub(crate) fn #function<'tokens>() -> BoxedParser<'tokens, #output> {
                    #shared_function()
                        .map(generated_runtime::SharedSyntaxOutput::into_owned)
                        .boxed()
                }
            }
        });
        Ok(Some(quote! {
            #[allow(dead_code)]
            #[bityzba::invariant(true)]
            struct #family_ident<'tokens> {
                #(#fields,)*
            }

            #[allow(dead_code)]
            pub(crate) fn strict_generated_parser_family<'tokens>() -> #family_ident<'tokens> {
                let __generated_recursive_family = RecursiveFamily::new();
                #(#declarations)*
                #(#definitions)*
                #family_ident {
                    #(#outputs,)*
                }
            }

            #(#root_functions)*
        }))
    }

    #[requires(true)]
    #[ensures(true)]
    fn expand_recovered_recursive_family(
        &self,
        recovered_module: &TokenStream2,
    ) -> Result<Option<TokenStream2>> {
        if self.recursive.is_empty() {
            return Ok(None);
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
            return Ok(None);
        }
        let local_recursive_names = recursive_rules
            .iter()
            .map(|rule| rule.name.to_string())
            .collect::<BTreeSet<_>>();
        let family_ident = format_ident!("RecoveredGeneratedParserFamily");
        let fields = recursive_rules.iter().map(|rule| {
            let name = &rule.name;
            let output = recovered_rule_function_output_tokens(&rule.output, recovered_module);
            quote!(#name: BoxedParser<'tokens, generated_runtime::SharedSyntaxOutput<#output>>)
        });
        let declarations = recursive_rules.iter().map(|rule| {
            let name = &rule.name;
            quote!(let mut #name = __generated_recursive_family.declare();)
        });
        let definitions = recursive_rules
            .iter()
            .map(|recursive| {
                let rule = self
                    .rules
                    .iter()
                    .find(|rule| rule.name().to_string() == recursive.name.to_string())
                    .ok_or_else(|| {
                        syn::Error::new_spanned(
                            &recursive.name,
                            "recursive parser declaration has no matching rule",
                        )
                    })?;
                let parser_name = format_ident!("recovered_{}_parser", rule.name());
                let parser_arguments = rule
                    .arguments()
                    .iter()
                    .map(|argument| {
                        let argument_name = argument.to_string();
                        if local_recursive_names.contains(&argument_name) {
                            Ok(quote!(#argument.clone().map(
                                generated_runtime::SharedSyntaxOutput::into_owned
                            )))
                        } else if all_recursive_names.contains(&argument_name) {
                            Ok(quote!(super::recovered_generated_parser_family(
                                __generated_recovery_rules.clone()
                            ).#argument.map(
                                generated_runtime::SharedSyntaxOutput::into_owned
                            )))
                        } else {
                            Err(syn::Error::new_spanned(
                                argument,
                                "recursive parser argument is not declared in the recursive block",
                            ))
                        }
                    })
                    .collect::<Result<Vec<_>>>()?;
                let hidden_free_modifier = if local_recursive_names.contains("free_modifier") {
                    let free_modifier = format_ident!("free_modifier");
                    quote!(#free_modifier.clone().map(
                        generated_runtime::SharedSyntaxOutput::into_owned
                    ).boxed())
                } else if all_recursive_names.contains("free_modifier") {
                    quote!(
                        super::recovered_generated_parser_family(
                            __generated_recovery_rules.clone()
                        )
                        .free_modifier
                        .map(generated_runtime::SharedSyntaxOutput::into_owned)
                        .boxed()
                    )
                } else {
                    quote!(generated_runtime::recovered_empty_free_modifier_parser())
                };
                let name = &recursive.name;
                Ok(quote! {
                    #name.define(#parser_name(
                        #(#parser_arguments,)*
                        #hidden_free_modifier,
                        __generated_recovery_rules.clone(),
                    ));
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let outputs = recursive_rules.iter().map(|rule| {
            let name = &rule.name;
            quote!(#name: __generated_recursive_family.own(#name).boxed())
        });
        let root_functions = recursive_rules.iter().map(|rule| {
            let root_name = &rule.name;
            let function = format_ident!("recovered_generated_{}_parser", root_name);
            let shared_function = format_ident!("recovered_generated_{}_shared_parser", root_name);
            let output = recovered_rule_function_output_tokens(&rule.output, recovered_module);
            quote! {
                #[allow(dead_code, unused_variables)]
                pub(crate) fn #shared_function<'tokens>(
                    __generated_recovery_rules: std::sync::Arc<[&'static str]>,
                ) -> BoxedParser<
                    'tokens,
                    generated_runtime::SharedSyntaxOutput<#output>,
                > {
                    recovered_generated_parser_family(__generated_recovery_rules).#root_name
                }

                #[allow(dead_code, unused_variables)]
                pub(crate) fn #function<'tokens>(
                    __generated_recovery_rules: std::sync::Arc<[&'static str]>,
                ) -> BoxedParser<'tokens, #output> {
                    #shared_function(__generated_recovery_rules)
                        .map(generated_runtime::SharedSyntaxOutput::into_owned)
                        .boxed()
                }
            }
        });
        Ok(Some(quote! {
            #[allow(dead_code)]
            #[bityzba::invariant(true)]
            struct #family_ident<'tokens> {
                #(#fields,)*
            }

            #[allow(dead_code)]
            pub(crate) fn recovered_generated_parser_family<'tokens>(
                __generated_recovery_rules: std::sync::Arc<[&'static str]>,
            ) -> #family_ident<'tokens> {
                let __generated_recursive_family = RecursiveFamily::new();
                #(#declarations)*
                #(#definitions)*
                #family_ident {
                    #(#outputs,)*
                }
            }

            #(#root_functions)*
        }))
    }
}

#[requires(true)]
#[ensures(true)]
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

#[invariant(true)]
struct RecursiveRule {
    name: Ident,
    output: Type,
}

impl RecursiveRule {
    #[requires(true)]
    #[ensures(true)]
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

#[invariant(true)]
#[invariant(::Chain(_) => true)]
#[invariant(::Postfix => true)]
#[invariant(::Rust(_) => true)]
#[invariant(::Vector(_) => true)]
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
    #[requires(true)]
    #[ensures(true)]
    fn compact_tokens(&self) -> String {
        match self {
            Self::Rust(expr) => compact_tokens(expr),
            Self::Vector(expr) => compact_tokens(&expr.to_token_stream()),
            Self::Chain(expr) => compact_tokens(&expr.to_token_stream()),
            Self::Postfix { .. } => compact_tokens(&self.to_token_stream()),
        }
    }

    #[requires(true)]
    #[ensures(true)]
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

    #[requires(true)]
    #[ensures(true)]
    fn rust_tokens(&self) -> TokenStream2 {
        self.to_token_stream()
    }

    #[requires(true)]
    #[ensures(true)]
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

#[invariant(true)]
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

#[invariant(!items.is_empty(), "vector parser expressions need at least one item")]
struct VectorExpr {
    items: Vec<VectorItem>,
}

#[invariant(true)]
#[invariant(::Assert => true)]
#[invariant(::One(_) => true)]
#[invariant(::OneOrMore(_) => true)]
#[invariant(::OneOrMoreSpread(_) => true)]
#[invariant(::Spread(_) => true)]
#[invariant(::ZeroOrMore(_) => true)]
#[invariant(::ZeroOrMoreSpread(_) => true)]
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
        Ok(Self::from_data(data!(VectorExpr { items })))
    }
}

impl ToTokens for VectorExpr {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let items = self.items.iter().map(VectorItem::to_token_stream);
        tokens.extend(quote!([#(#items;)*]));
    }
}

impl VectorItem {
    #[requires(true)]
    #[ensures(true)]
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

#[invariant(true)]
#[invariant(::Alias(_) => true)]
#[invariant(::Enum(_) => true)]
#[invariant(::Struct(_) => true)]
enum Rule {
    Alias(AliasRule),
    Struct(NodeRule),
    Enum(EnumRule),
}

impl Rule {
    #[requires(true)]
    #[ensures(true)]
    fn name(&self) -> &Ident {
        match self {
            Rule::Alias(rule) => &rule.name,
            Rule::Struct(rule) => &rule.name,
            Rule::Enum(rule) => &rule.name,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn output<'a>(&'a self, type_env: &'a GrammarTypeEnv) -> Option<&'a Type> {
        type_env.rules.get(&self.name().to_string())
    }

    #[requires(true)]
    #[ensures(true)]
    fn declared_output(&self) -> Option<&Type> {
        match self {
            Rule::Alias(_) => None,
            Rule::Struct(rule) => Some(&rule.output),
            Rule::Enum(rule) => Some(&rule.output),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn context_label(&self) -> Option<String> {
        match self {
            Rule::Alias(rule) => rule.context.as_ref().map(LitStr::value),
            Rule::Struct(rule) => rule.context.as_ref().map(LitStr::value),
            Rule::Enum(rule) => Some(rule.context.value()),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn expand_metadata(&self, type_env: &GrammarTypeEnv) -> Result<TokenStream2> {
        match self {
            Rule::Alias(rule) => rule.expand_metadata(type_env),
            Rule::Struct(rule) => rule.expand_metadata("struct", type_env),
            Rule::Enum(rule) => rule.expand_metadata(type_env),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn arguments(&self) -> &[Ident] {
        match self {
            Rule::Alias(rule) => &rule.arguments,
            Rule::Struct(rule) => &rule.arguments,
            Rule::Enum(rule) => &rule.arguments,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn argument_types(&self, type_env: &GrammarTypeEnv) -> Option<BTreeMap<String, Type>> {
        match self {
            Rule::Alias(rule) => rule.argument_types(type_env),
            Rule::Struct(rule) => rule.argument_types(type_env),
            Rule::Enum(rule) => rule.argument_types(type_env),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn expand_strict_parser(
        &self,
        type_env: &GrammarTypeEnv,
        generate_model: bool,
        model_outputs: &Option<BTreeSet<String>>,
        model_all_rules_local: bool,
        model_path: Option<&Path>,
        use_model_construction: bool,
    ) -> Result<TokenStream2> {
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

    #[requires(true)]
    #[ensures(true)]
    fn expand_recovered_parser(
        &self,
        type_env: &GrammarTypeEnv,
        model_outputs: &Option<BTreeSet<String>>,
        model_all_rules_local: bool,
        model_path: Option<&Path>,
        recovered_module: &TokenStream2,
        use_model_construction: bool,
    ) -> Result<TokenStream2> {
        match self {
            Rule::Alias(rule) => rule.expand_recovered_parser(
                type_env,
                model_outputs,
                model_all_rules_local,
                model_path,
                recovered_module,
            ),
            Rule::Struct(rule) => rule.expand_recovered_parser(
                type_env,
                model_outputs,
                model_all_rules_local,
                model_path,
                recovered_module,
                use_model_construction,
            ),
            Rule::Enum(rule) => rule.expand_recovered_parser(
                type_env,
                model_outputs,
                model_all_rules_local,
                model_path,
                recovered_module,
                use_model_construction,
            ),
        }
    }
}

#[invariant(true)]
struct AliasRule {
    name: Ident,
    arguments: Vec<Ident>,
    context: Option<LitStr>,
    parser: ParserExpr,
}

impl AliasRule {
    #[requires(true)]
    #[ensures(true)]
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
        let recovery = classify_parser_expr(&self.parser, &argument_names, type_env)?.expand();
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

    #[requires(true)]
    #[ensures(true)]
    fn expand_strict_parser(
        &self,
        type_env: &GrammarTypeEnv,
        generate_model: bool,
        model_outputs: &Option<BTreeSet<String>>,
        model_all_rules_local: bool,
        model_path: Option<&Path>,
    ) -> Result<TokenStream2> {
        let argument_types = self.argument_types(type_env).ok_or_else(|| {
            syn::Error::new_spanned(
                &self.name,
                "cannot generate strict alias parser because an argument is not a declared recursive rule",
            )
        })?;
        let output = type_env.rules.get(&self.name.to_string()).ok_or_else(|| {
            syn::Error::new_spanned(
                &self.name,
                "cannot generate strict alias parser because its output type cannot be inferred",
            )
        })?;
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
        let argument_tokens = strict_parser_argument_tokens(
            &self.arguments,
            &argument_types,
            generate_model,
            model_outputs,
            model_path,
        );
        let argument_generic_params = &argument_tokens.generic_params;
        let argument_params = &argument_tokens.params;
        let argument_where_clause = &argument_tokens.where_clause;
        let hidden_free_modifier = strict_free_modifier_param_tokens();
        let rule_name = self.name.to_string();
        let context = self.context.as_ref().map_or_else(
            || quote!(None),
            |context| {
                let context = context.value();
                quote!(Some(#context))
            },
        );
        Ok(quote! {
            #[allow(dead_code, unused_variables)]
            pub(crate) fn #name<'tokens #(, #argument_generic_params)*>(
                #(#argument_params,)*
                #hidden_free_modifier
            ) -> BoxedParser<'tokens, generated_runtime::SharedSyntaxOutput<#output>>
            #argument_where_clause
            {
                generated_runtime::rule_wrapper(
                    #rule_name,
                    #context,
                    #parser
                )
            }
        })
    }

    #[requires(true)]
    #[ensures(true)]
    fn expand_recovered_parser(
        &self,
        type_env: &GrammarTypeEnv,
        model_outputs: &Option<BTreeSet<String>>,
        model_all_rules_local: bool,
        model_path: Option<&Path>,
        recovered_module: &TokenStream2,
    ) -> Result<TokenStream2> {
        let argument_types = self.argument_types(type_env).ok_or_else(|| {
            syn::Error::new_spanned(
                &self.name,
                "cannot generate recovered alias parser because an argument is not a declared recursive rule",
            )
        })?;
        let output = type_env.rules.get(&self.name.to_string()).ok_or_else(|| {
            syn::Error::new_spanned(
                &self.name,
                "cannot generate recovered alias parser because its output type cannot be inferred",
            )
        })?;
        let argument_names = self.argument_name_set();
        let free_modifier_parser = format_ident!("__generated_free_modifier");
        let generation = RecoveredParserGeneration {
            type_env,
            model_outputs,
            model_all_rules_local,
            recovered_module,
        };
        let parser = recovered_parser_expr_tokens(
            &self.parser,
            &argument_names,
            &generation,
            &free_modifier_parser,
            RecoveredParserCallMode::Local,
        )?;
        let name = format_ident!("recovered_{}_parser", self.name);
        let output =
            recovered_parser_value_type_tokens(output, model_outputs, model_path, recovered_module);
        let argument_tokens = recovered_parser_argument_tokens(
            &self.arguments,
            &argument_types,
            model_outputs,
            model_path,
            recovered_module,
        );
        let argument_generic_params = &argument_tokens.generic_params;
        let argument_params = &argument_tokens.params;
        let argument_where_clause = &argument_tokens.where_clause;
        let hidden_free_modifier = recovered_free_modifier_param_tokens(recovered_module);
        let hidden_recovery_rules = recovered_rules_param_tokens();
        let rule_name = self.name.to_string();
        let context = self.context.as_ref().map_or_else(
            || quote!(None),
            |context| {
                let context = context.value();
                quote!(Some(#context))
            },
        );
        Ok(quote! {
            #[allow(dead_code, unused_variables)]
            pub(crate) fn #name<'tokens #(, #argument_generic_params)*>(
                #(#argument_params,)*
                #hidden_free_modifier
                #hidden_recovery_rules
            ) -> BoxedParser<'tokens, generated_runtime::SharedSyntaxOutput<#output>>
            #argument_where_clause
            {
                generated_runtime::rule_wrapper(
                    #rule_name,
                    #context,
                    #parser
                )
            }
        })
    }

    #[requires(true)]
    #[ensures(true)]
    fn argument_types(&self, type_env: &GrammarTypeEnv) -> Option<BTreeMap<String, Type>> {
        let mut arguments = BTreeMap::new();
        for argument in &self.arguments {
            let ty = type_env.recursive.get(&argument.to_string())?.clone();
            arguments.insert(argument.to_string(), ty);
        }
        Some(arguments)
    }

    #[requires(true)]
    #[ensures(true)]
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

#[invariant(!branches.is_empty(), "enum rules need at least one branch")]
struct EnumRule {
    name: Ident,
    arguments: Vec<Ident>,
    output: Type,
    context: LitStr,
    branches: Vec<EnumBranch>,
}

#[invariant(true)]
struct EnumBranch {
    attrs: Vec<Attribute>,
    conditions: Vec<Condition>,
    name: Ident,
}

impl EnumRule {
    #[requires(true)]
    #[ensures(true)]
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
        let argument_names = self.argument_name_set();
        let mut fields = Vec::new();
        for branch in &self.branches {
            let branch_name = branch.name.to_string();
            if !type_env.rule_known_for_recovery(&branch_name, &argument_names) {
                return Err(syn::Error::new_spanned(
                    &branch.name,
                    "enum branches must reference a known grammar rule or recursive parser argument",
                ));
            }
            let conditions = branch.conditions.iter().map(Condition::expand);
            fields.push(quote! {
                SyntaxGrammarField {
                    kind: "variant",
                    name: #branch_name,
                    parser: #branch_name,
                    recovery: SyntaxGrammarRecoveryExpr::Rule(#branch_name),
                    conditions: &[#(#conditions),*],
                }
            });
        }
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

    #[requires(true)]
    #[ensures(true)]
    fn expand_strict_parser(
        &self,
        type_env: &GrammarTypeEnv,
        generate_model: bool,
        model_outputs: &Option<BTreeSet<String>>,
        model_all_rules_local: bool,
        model_path: Option<&Path>,
        use_model_construction: bool,
    ) -> Result<TokenStream2> {
        let argument_types = self.argument_types(type_env).ok_or_else(|| {
            syn::Error::new_spanned(
                &self.name,
                "cannot generate strict enum parser because an argument is not a declared recursive rule",
            )
        })?;
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
                    argument_types.get(&branch_name).ok_or_else(|| {
                        syn::Error::new_spanned(
                            &branch.name,
                            "enum branch argument is not declared for this rule",
                        )
                    })?
                } else {
                    type_env
                        .rules
                        .get(&branch_name)
                        .or_else(|| type_env.recursive.get(&branch_name))
                        .ok_or_else(|| {
                            syn::Error::new_spanned(
                                &branch.name,
                                "enum branch does not name a rule, recursive parser, or rule argument",
                            )
                        })?
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
                        &branch.name,
                        type_env.rule_arguments_for_call(&branch_name).ok_or_else(|| {
                            syn::Error::new_spanned(
                                &branch.name,
                                "cannot find argument list for enum branch rule",
                            )
                        })?,
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
                Ok(quote!(#branch_parser.map(|#field| #body)))
            })
            .collect::<Result<Vec<_>>>()?;
        let parser = strict_choice_chain(alternatives, &self.name)?;
        let name = format_ident!("strict_{}_parser", self.name);
        let argument_tokens = strict_parser_argument_tokens(
            &self.arguments,
            &argument_types,
            generate_model,
            model_outputs,
            model_path,
        );
        let argument_generic_params = &argument_tokens.generic_params;
        let argument_params = &argument_tokens.params;
        let argument_where_clause = &argument_tokens.where_clause;
        let hidden_free_modifier = strict_free_modifier_param_tokens();
        let rule_name = self.name.to_string();
        let context = self.context.value();
        Ok(quote! {
            #[allow(dead_code, unused_variables)]
            pub(crate) fn #name<'tokens #(, #argument_generic_params)*>(
                #(#argument_params,)*
                #hidden_free_modifier
            ) -> BoxedParser<'tokens, generated_runtime::SharedSyntaxOutput<#output_tokens>>
            #argument_where_clause
            {
                generated_runtime::rule_wrapper(
                    #rule_name,
                    Some(#context),
                    #parser
                )
            }
        })
    }

    #[requires(true)]
    #[ensures(true)]
    fn expand_recovered_parser(
        &self,
        type_env: &GrammarTypeEnv,
        model_outputs: &Option<BTreeSet<String>>,
        model_all_rules_local: bool,
        model_path: Option<&Path>,
        recovered_module: &TokenStream2,
        use_model_construction: bool,
    ) -> Result<TokenStream2> {
        let argument_types = self.argument_types(type_env).ok_or_else(|| {
            syn::Error::new_spanned(
                &self.name,
                "cannot generate recovered enum parser because an argument is not a declared recursive rule",
            )
        })?;
        let argument_names = self.argument_name_set();
        let free_modifier_parser = format_ident!("__generated_free_modifier");
        let generation = RecoveredParserGeneration {
            type_env,
            model_outputs,
            model_all_rules_local,
            recovered_module,
        };
        let output_tokens = recovered_rule_function_output_tokens(&self.output, recovered_module);
        let alternatives = self
            .branches
            .iter()
            .map(|branch| {
                let branch_name = branch.name.to_string();
                let branch_is_argument = argument_names.contains(&branch_name);
                let branch_output = if branch_is_argument {
                    argument_types.get(&branch_name).ok_or_else(|| {
                        syn::Error::new_spanned(
                            &branch.name,
                            "enum branch argument is not declared for this rule",
                        )
                    })?
                } else {
                    type_env
                        .rules
                        .get(&branch_name)
                        .or_else(|| type_env.recursive.get(&branch_name))
                        .ok_or_else(|| {
                            syn::Error::new_spanned(
                                &branch.name,
                                "enum branch does not name a rule, recursive parser, or rule argument",
                            )
                        })?
                };
                let variant = enum_variant_ident_for_output(branch_output, &branch.name);
                let field = &branch.name;
                let branch_parser = if branch_is_argument {
                    recovered_argument_parser_tokens(
                        &branch_name,
                        &argument_names,
                        &generation,
                        RecoveredParserCallMode::Local,
                        true,
                    )?
                } else if type_env.rules.contains_key(&branch_name) {
                    recovered_rule_call_by_argument_names(
                        &branch.name,
                        type_env.rule_arguments_for_call(&branch_name).ok_or_else(|| {
                            syn::Error::new_spanned(
                                &branch.name,
                                "cannot find argument list for enum branch rule",
                            )
                        })?,
                        &argument_names,
                        &generation,
                        &free_modifier_parser,
                        RecoveredParserCallMode::Local,
                    )?
                } else {
                    recovered_argument_parser_tokens(
                        &branch_name,
                        &argument_names,
                        &generation,
                        RecoveredParserCallMode::Local,
                        true,
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
                Ok(quote!(#branch_parser.map(|#field| #body)))
            })
            .collect::<Result<Vec<_>>>()?;
        let parser = strict_choice_chain(alternatives, &self.name)?;
        let name = format_ident!("recovered_{}_parser", self.name);
        let argument_tokens = recovered_parser_argument_tokens(
            &self.arguments,
            &argument_types,
            model_outputs,
            model_path,
            recovered_module,
        );
        let argument_generic_params = &argument_tokens.generic_params;
        let argument_params = &argument_tokens.params;
        let argument_where_clause = &argument_tokens.where_clause;
        let hidden_free_modifier = recovered_free_modifier_param_tokens(recovered_module);
        let hidden_recovery_rules = recovered_rules_param_tokens();
        let rule_name = self.name.to_string();
        let context = self.context.value();
        Ok(quote! {
            #[allow(dead_code, unused_variables)]
            pub(crate) fn #name<'tokens #(, #argument_generic_params)*>(
                #(#argument_params,)*
                #hidden_free_modifier
                #hidden_recovery_rules
            ) -> BoxedParser<'tokens, generated_runtime::SharedSyntaxOutput<#output_tokens>>
            #argument_where_clause
            {
                generated_runtime::rule_wrapper(
                    #rule_name,
                    Some(#context),
                    #parser
                )
            }
        })
    }

    #[requires(true)]
    #[ensures(true)]
    fn argument_types(&self, type_env: &GrammarTypeEnv) -> Option<BTreeMap<String, Type>> {
        let mut arguments = BTreeMap::new();
        for argument in &self.arguments {
            let ty = type_env.recursive.get(&argument.to_string())?.clone();
            arguments.insert(argument.to_string(), ty);
        }
        Some(arguments)
    }

    #[requires(true)]
    #[ensures(true)]
    fn argument_name_set(&self) -> BTreeSet<String> {
        self.arguments
            .iter()
            .map(Ident::to_string)
            .collect::<BTreeSet<_>>()
    }
}

#[invariant(true)]
struct NodeRule {
    name: Ident,
    arguments: Vec<Ident>,
    output: Type,
    context: Option<LitStr>,
    fields: Vec<FieldItem>,
}

impl NodeRule {
    #[requires(true)]
    #[ensures(true)]
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

    #[requires(true)]
    #[ensures(true)]
    fn generated_elidable_terminator_fields(&self) -> Result<Vec<(String, String)>> {
        self.fields
            .iter()
            .filter_map(|field| match field.elidable_terminator_cmavo() {
                Ok(Some(cmavo)) => Some(
                    field
                        .name
                        .as_ref()
                        .map(|name| Ok((name.to_string(), cmavo)))
                        .unwrap_or_else(|| {
                            Err(syn::Error::new_spanned(
                                field.parser.to_token_stream(),
                                "elidable terminator annotations require a named parser field",
                            ))
                        }),
                ),
                Ok(None) => None,
                Err(error) => Some(Err(error)),
            })
            .collect()
    }

    #[requires(true)]
    #[ensures(true)]
    fn expand_metadata(
        &self,
        kind: &'static str,
        type_env: &GrammarTypeEnv,
    ) -> Result<TokenStream2> {
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
            .map(|field| field.expand(&argument_names, type_env))
            .collect::<Result<Vec<_>>>()?;
        Ok(quote! {
            SyntaxGrammarRule {
                kind: #kind,
                name: #name,
                arguments: &[#(#arguments),*],
                output: #output,
                context: #context,
                fields: &[#(#fields),*],
            }
        })
    }

    #[requires(true)]
    #[ensures(true)]
    fn expand_strict_parser(
        &self,
        type_env: &GrammarTypeEnv,
        generate_model: bool,
        model_outputs: &Option<BTreeSet<String>>,
        model_all_rules_local: bool,
        model_path: Option<&Path>,
        use_model_construction: bool,
    ) -> Result<TokenStream2> {
        let argument_types = self.argument_types(type_env).ok_or_else(|| {
            syn::Error::new_spanned(
                &self.name,
                "cannot generate strict struct parser because an argument is not a declared recursive rule",
            )
        })?;
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
        let argument_tokens = strict_parser_argument_tokens(
            &self.arguments,
            &argument_types,
            generate_model,
            model_outputs,
            model_path,
        );
        let argument_generic_params = &argument_tokens.generic_params;
        let argument_params = &argument_tokens.params;
        let argument_where_clause = &argument_tokens.where_clause;
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
            return Err(syn::Error::new_spanned(
                &self.output,
                "strict parser generation supports unit, tuple, and path output types",
            ));
        };
        let rule_name = self.name.to_string();
        let parser_body = quote!(#parser.map(|#pattern| #body));
        let context = self.context.as_ref().map_or_else(
            || quote!(None),
            |context| {
                let context = context.value();
                quote!(Some(#context))
            },
        );
        Ok(quote! {
            #[allow(dead_code, unused_variables)]
            pub(crate) fn #name<'tokens #(, #argument_generic_params)*>(
                #(#argument_params,)*
                #hidden_free_modifier
            ) -> BoxedParser<'tokens, generated_runtime::SharedSyntaxOutput<#output_tokens>>
            #argument_where_clause
            {
                generated_runtime::rule_wrapper(
                    #rule_name,
                    #context,
                    #parser_body
                )
            }
        })
    }

    #[requires(true)]
    #[ensures(true)]
    fn expand_recovered_parser(
        &self,
        type_env: &GrammarTypeEnv,
        model_outputs: &Option<BTreeSet<String>>,
        model_all_rules_local: bool,
        model_path: Option<&Path>,
        recovered_module: &TokenStream2,
        use_model_construction: bool,
    ) -> Result<TokenStream2> {
        let argument_types = self.argument_types(type_env).ok_or_else(|| {
            syn::Error::new_spanned(
                &self.name,
                "cannot generate recovered struct parser because an argument is not a declared recursive rule",
            )
        })?;
        let argument_names = self.argument_name_set();
        let sequence_items = self
            .fields
            .iter()
            .filter(|field| matches!(field.kind, FieldKind::Field | FieldKind::Require))
            .collect::<Vec<_>>();
        let free_modifier_parser = format_ident!("__generated_free_modifier");
        let generation = RecoveredParserGeneration {
            type_env,
            model_outputs,
            model_all_rules_local,
            recovered_module,
        };
        let (parser, pattern) = recovered_sequence_parser_tokens(
            &self.name.to_string(),
            &sequence_items,
            &argument_names,
            &generation,
            &free_modifier_parser,
            RecoveredParserCallMode::Local,
            true,
        )?;
        let (plain_parser, plain_pattern) = recovered_sequence_parser_tokens(
            &self.name.to_string(),
            &sequence_items,
            &argument_names,
            &generation,
            &free_modifier_parser,
            RecoveredParserCallMode::Local,
            false,
        )?;
        let name = format_ident!("recovered_{}_parser", self.name);
        let output = &self.output;
        let output_tokens = recovered_rule_function_output_tokens(output, recovered_module);
        let argument_tokens = recovered_parser_argument_tokens(
            &self.arguments,
            &argument_types,
            model_outputs,
            model_path,
            recovered_module,
        );
        let argument_generic_params = &argument_tokens.generic_params;
        let argument_params = &argument_tokens.params;
        let argument_where_clause = &argument_tokens.where_clause;
        let hidden_free_modifier = recovered_free_modifier_param_tokens(recovered_module);
        let hidden_recovery_rules = recovered_rules_param_tokens();
        let let_bindings = self.fields.iter().filter_map(|field| {
            matches!(field.kind, FieldKind::Computed | FieldKind::TempLet).then(|| {
                let name = field.name.as_ref().expect("let field items have names");
                let value = field.parser.rust_tokens();
                quote!(let #name = #value;)
            })
        });
        let body = if is_unit_type(output) {
            quote!({
                #(#let_bindings)*
                ()
            })
        } else if use_model_construction && matches!(output, Type::Tuple(_)) {
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
            return Err(syn::Error::new_spanned(
                &self.output,
                "recovered parser generation supports unit, tuple, and path output types",
            ));
        };
        let rule_name = self.name.to_string();
        // A recovery directive can only act at fields of its owning rule. Keeping
        // ordinary recovered rules on the uninstrumented typed parser avoids one
        // dynamic parser frame per field, which is material for deeply nested WASM
        // parses, without changing the recovered model they produce.
        let parser_body = quote! {
            if __generated_recovery_rules.iter().any(|rule| *rule == #rule_name) {
                #parser.map(|#pattern| #body).boxed()
            } else {
                #plain_parser.map(|#plain_pattern| #body).boxed()
            }
        };
        let context = self.context.as_ref().map_or_else(
            || quote!(None),
            |context| {
                let context = context.value();
                quote!(Some(#context))
            },
        );
        Ok(quote! {
            #[allow(dead_code, unused_variables)]
            pub(crate) fn #name<'tokens #(, #argument_generic_params)*>(
                #(#argument_params,)*
                #hidden_free_modifier
                #hidden_recovery_rules
            ) -> BoxedParser<'tokens, generated_runtime::SharedSyntaxOutput<#output_tokens>>
            #argument_where_clause
            {
                generated_runtime::rule_wrapper(
                    #rule_name,
                    #context,
                    #parser_body
                )
            }
        })
    }

    #[requires(true)]
    #[ensures(true)]
    fn argument_types(&self, type_env: &GrammarTypeEnv) -> Option<BTreeMap<String, Type>> {
        let mut arguments = BTreeMap::new();
        for argument in &self.arguments {
            let ty = type_env.recursive.get(&argument.to_string())?.clone();
            arguments.insert(argument.to_string(), ty);
        }
        Some(arguments)
    }

    #[requires(true)]
    #[ensures(true)]
    fn argument_name_set(&self) -> BTreeSet<String> {
        self.arguments
            .iter()
            .map(Ident::to_string)
            .collect::<BTreeSet<_>>()
    }
}

#[invariant(true)]
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

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RecoveredParserCallMode {
    Local,
    External,
}

#[invariant(true)]
struct StrictParserGeneration<'a> {
    type_env: &'a GrammarTypeEnv,
    generate_model: bool,
    model_outputs: &'a Option<BTreeSet<String>>,
    model_all_rules_local: bool,
}

impl StrictParserGeneration<'_> {
    #[requires(true)]
    #[ensures(true)]
    fn rule_has_local_parser(&self, name: &str) -> bool {
        self.model_all_rules_local || self.rule_is_generated_model(name)
    }

    #[requires(true)]
    #[ensures(true)]
    fn rule_is_generated_model(&self, name: &str) -> bool {
        self.type_env.rules.get(name).is_some_and(|output| {
            output_is_generated_model(self.generate_model, self.model_outputs, output)
        })
    }

    #[requires(true)]
    #[ensures(true)]
    fn recursive_has_local_parser(&self, name: &str) -> bool {
        self.model_all_rules_local || self.recursive_is_generated_model(name)
    }

    #[requires(true)]
    #[ensures(true)]
    fn recursive_is_generated_model(&self, name: &str) -> bool {
        self.type_env.recursive.get(name).is_some_and(|output| {
            output_is_generated_model(self.generate_model, self.model_outputs, output)
        })
    }

    #[requires(true)]
    #[ensures(true)]
    fn external_recursive_parser(&self, name: &Ident) -> TokenStream2 {
        quote!(super::strict_generated_parser_family()
            .#name
            .map(generated_runtime::SharedSyntaxOutput::into_owned))
    }

    #[requires(true)]
    #[ensures(true)]
    fn external_free_modifier_parser(&self) -> TokenStream2 {
        let name = format_ident!("free_modifier");
        if self.recursive_has_local_parser("free_modifier") {
            self.external_recursive_parser(&name)
        } else {
            quote!(__generated_free_modifier.clone())
        }
    }
}

#[invariant(true)]
struct RecoveredParserGeneration<'a> {
    type_env: &'a GrammarTypeEnv,
    model_outputs: &'a Option<BTreeSet<String>>,
    model_all_rules_local: bool,
    recovered_module: &'a TokenStream2,
}

impl RecoveredParserGeneration<'_> {
    #[requires(true)]
    #[ensures(true)]
    fn rule_has_local_parser(&self, name: &str) -> bool {
        self.model_all_rules_local || self.rule_is_generated_model(name)
    }

    #[requires(true)]
    #[ensures(true)]
    fn rule_is_generated_model(&self, name: &str) -> bool {
        self.type_env
            .rules
            .get(name)
            .is_some_and(|output| output_is_generated_model(true, self.model_outputs, output))
    }

    #[requires(true)]
    #[ensures(true)]
    fn recursive_has_local_parser(&self, name: &str) -> bool {
        self.model_all_rules_local || self.recursive_is_generated_model(name)
    }

    #[requires(true)]
    #[ensures(true)]
    fn recursive_is_generated_model(&self, name: &str) -> bool {
        self.type_env
            .recursive
            .get(name)
            .is_some_and(|output| output_is_generated_model(true, self.model_outputs, output))
    }

    #[requires(true)]
    #[ensures(true)]
    fn external_recursive_parser(&self, name: &Ident) -> TokenStream2 {
        quote!(super::recovered_generated_parser_family(
            __generated_recovery_rules.clone()
        ).#name.map(generated_runtime::SharedSyntaxOutput::into_owned))
    }

    #[requires(true)]
    #[ensures(true)]
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
    #[requires(true)]
    #[ensures(true)]
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
    #[requires(true)]
    #[ensures(true)]
    fn rule_known_for_recovery(&self, name: &str, arguments: &BTreeSet<String>) -> bool {
        arguments.contains(name) || self.rules.contains_key(name)
    }

    #[requires(true)]
    #[ensures(true)]
    fn rule_arguments_for_call(&self, rule: &str) -> Option<&[String]> {
        self.rule_arguments.get(rule).map(Vec::as_slice)
    }

    #[requires(true)]
    #[ensures(true)]
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

#[requires(true)]
#[ensures(true)]
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

#[requires(true)]
#[ensures(true)]
fn strict_free_modifier_param_tokens() -> TokenStream2 {
    quote!(__generated_free_modifier: BoxedParser<'tokens, FreeModifierSyntax>,)
}

#[invariant(generic_params.len() == params.len())]
struct StrictParserArgumentTokens {
    generic_params: Vec<Ident>,
    params: Vec<TokenStream2>,
    where_clause: TokenStream2,
}

#[requires(true)]
#[ensures(true)]
fn strict_parser_argument_tokens(
    arguments: &[Ident],
    argument_types: &BTreeMap<String, Type>,
    generate_model: bool,
    model_outputs: &Option<BTreeSet<String>>,
    model_path: Option<&Path>,
) -> StrictParserArgumentTokens {
    let mut generic_params = Vec::new();
    let mut params = Vec::new();
    let mut where_predicates = Vec::new();
    for (index, argument) in arguments.iter().enumerate() {
        let generic = format_ident!("__Argument{index}Parser");
        let ty = argument_types
            .get(&argument.to_string())
            .expect("argument types are populated from recursive declarations");
        let ty = parser_type_tokens(ty, generate_model, model_outputs, model_path);
        generic_params.push(generic.clone());
        params.push(quote!(#argument: #generic));
        where_predicates.push(quote!(
            #generic: Parser<'tokens, #ty> + Clone + 'tokens
        ));
    }
    let where_clause = if where_predicates.is_empty() {
        quote!()
    } else {
        quote!(where #(#where_predicates,)*)
    };
    new!(StrictParserArgumentTokens {
        generic_params,
        params,
        where_clause,
    })
}

#[requires(true)]
#[ensures(true)]
fn recovered_free_modifier_param_tokens(recovered_module: &TokenStream2) -> TokenStream2 {
    quote!(__generated_free_modifier: BoxedParser<'tokens, #recovered_module::FreeModifierSyntax>,)
}

#[requires(true)]
#[ensures(true)]
fn recovered_rules_param_tokens() -> TokenStream2 {
    quote!(__generated_recovery_rules: std::sync::Arc<[&'static str]>,)
}

#[requires(true)]
#[ensures(true)]
fn recovered_parser_argument_tokens(
    arguments: &[Ident],
    argument_types: &BTreeMap<String, Type>,
    model_outputs: &Option<BTreeSet<String>>,
    model_path: Option<&Path>,
    recovered_module: &TokenStream2,
) -> StrictParserArgumentTokens {
    let mut generic_params = Vec::new();
    let mut params = Vec::new();
    let mut where_predicates = Vec::new();
    for (index, argument) in arguments.iter().enumerate() {
        let generic = format_ident!("__Argument{index}RecoveredParser");
        let ty = argument_types
            .get(&argument.to_string())
            .expect("argument types are populated from recursive declarations");
        let ty = if output_is_generated_model(true, model_outputs, ty) {
            recovered_rule_function_output_tokens(ty, recovered_module)
        } else {
            recovered_parser_value_type_tokens(ty, model_outputs, model_path, recovered_module)
        };
        generic_params.push(generic.clone());
        params.push(quote!(#argument: #generic));
        where_predicates.push(quote!(
            #generic: Parser<'tokens, #ty> + Clone + 'tokens
        ));
    }
    let where_clause = if where_predicates.is_empty() {
        quote!()
    } else {
        quote!(where #(#where_predicates,)*)
    };
    new!(StrictParserArgumentTokens {
        generic_params,
        params,
        where_clause,
    })
}

#[requires(true)]
#[ensures(true)]
fn strict_sequence_parser_tokens(
    fields: &[&FieldItem],
    arguments: &BTreeSet<String>,
    generation: &StrictParserGeneration<'_>,
    free_modifier_parser: &Ident,
    mode: StrictParserCallMode,
) -> Result<(TokenStream2, TokenStream2)> {
    let Some(first) = fields.first() else {
        return Ok((quote!(generated_runtime::empty()), quote!(())));
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
    Ok((parser, pattern))
}

#[requires(true)]
#[ensures(true)]
fn recovered_sequence_parser_tokens(
    rule_name: &str,
    fields: &[&FieldItem],
    arguments: &BTreeSet<String>,
    generation: &RecoveredParserGeneration<'_>,
    free_modifier_parser: &Ident,
    mode: RecoveredParserCallMode,
    instrument_fields: bool,
) -> Result<(TokenStream2, TokenStream2)> {
    let Some(first) = fields.first() else {
        return Ok((quote!(generated_runtime::empty()), quote!(())));
    };
    let mut parser = if instrument_fields && matches!(first.kind, FieldKind::Field) {
        recovered_field_parser_expr_tokens(
            rule_name,
            0usize,
            &first.parser,
            arguments,
            generation,
            free_modifier_parser,
            mode,
        )?
    } else {
        recovered_parser_expr_tokens(
            &first.parser,
            arguments,
            generation,
            free_modifier_parser,
            mode,
        )?
    };
    let mut pattern = sequence_item_pattern(first);
    for (field_index, field) in fields.iter().enumerate().skip(1) {
        let next = if instrument_fields && matches!(field.kind, FieldKind::Field) {
            recovered_field_parser_expr_tokens(
                rule_name,
                field_index,
                &field.parser,
                arguments,
                generation,
                free_modifier_parser,
                mode,
            )?
        } else {
            recovered_parser_expr_tokens(
                &field.parser,
                arguments,
                generation,
                free_modifier_parser,
                mode,
            )?
        };
        let name = sequence_item_pattern(field);
        parser = quote!(#parser.then(#next));
        pattern = quote!((#pattern, #name));
    }
    Ok((parser, pattern))
}

#[requires(!rule_name.is_empty())]
#[ensures(true)]
fn recovered_field_parser_expr_tokens(
    rule_name: &str,
    field_index: usize,
    parser: &ParserExpr,
    arguments: &BTreeSet<String>,
    generation: &RecoveredParserGeneration<'_>,
    free_modifier_parser: &Ident,
    mode: RecoveredParserCallMode,
) -> Result<TokenStream2> {
    if let ParserExpr::Vector(vector) = parser {
        if let [item] = vector.items.as_slice() {
            let repeated = match item {
                VectorItem::ZeroOrMore(expr) => {
                    let inner = recovered_parser_expr_tokens(
                        expr,
                        arguments,
                        generation,
                        free_modifier_parser,
                        mode,
                    )?;
                    Some(quote! {
                        generated_runtime::recovered_greedy_many_field_parser(
                            #rule_name,
                            #field_index,
                            0usize,
                            #inner.boxed()
                        )
                    })
                }
                VectorItem::ZeroOrMoreSpread(expr) => {
                    let inner = recovered_parser_expr_tokens(
                        expr,
                        arguments,
                        generation,
                        free_modifier_parser,
                        mode,
                    )?;
                    Some(quote! {
                        generated_runtime::recovered_greedy_many_field_parser(
                            #rule_name,
                            #field_index,
                            0usize,
                            #inner.boxed()
                        )
                        .map(|__chunks| {
                            let mut __items = Vec::new();
                            for __chunk in __chunks {
                                __items.extend(__chunk);
                            }
                            __items
                        })
                    })
                }
                VectorItem::OneOrMore(expr) => {
                    let inner = recovered_parser_expr_tokens(
                        expr,
                        arguments,
                        generation,
                        free_modifier_parser,
                        mode,
                    )?;
                    Some(quote! {
                        generated_runtime::recovered_greedy_many_field_parser(
                            #rule_name,
                            #field_index,
                            1usize,
                            #inner.boxed()
                        )
                        .map(|__items| {
                            vec1::Vec1::try_from_vec(__items)
                                .expect("recovered non-empty vector parser preserves cardinality")
                        })
                    })
                }
                VectorItem::OneOrMoreSpread(expr) => {
                    let inner = recovered_parser_expr_tokens(
                        expr,
                        arguments,
                        generation,
                        free_modifier_parser,
                        mode,
                    )?;
                    Some(quote! {
                        generated_runtime::recovered_greedy_many_field_parser(
                            #rule_name,
                            #field_index,
                            1usize,
                            #inner.boxed()
                        )
                        .map(|__chunks| {
                            let mut __items = Vec::new();
                            for __chunk in __chunks {
                                __items.extend(__chunk);
                            }
                            vec1::Vec1::try_from_vec(__items)
                                .expect("recovered non-empty spread vector parser preserves cardinality")
                        })
                    })
                }
                VectorItem::One(_) | VectorItem::Spread(_) | VectorItem::Assert { .. } => None,
            };
            if let Some(repeated) = repeated {
                return Ok(repeated);
            }
        }
    }

    let inner =
        recovered_parser_expr_tokens(parser, arguments, generation, free_modifier_parser, mode)?;
    Ok(quote!(generated_runtime::recovered_field_parser(
        #rule_name,
        #field_index,
        #inner
    )))
}

#[requires(true)]
#[ensures(true)]
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

#[requires(true)]
#[ensures(true)]
fn strict_parser_expr_tokens(
    expr: &ParserExpr,
    arguments: &BTreeSet<String>,
    generation: &StrictParserGeneration<'_>,
    free_modifier_parser: &Ident,
    mode: StrictParserCallMode,
) -> Result<TokenStream2> {
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

#[requires(true)]
#[ensures(true)]
fn strict_chain_parser_expr_tokens(
    expr: &ChainExpr,
    arguments: &BTreeSet<String>,
    generation: &StrictParserGeneration<'_>,
    free_modifier_parser: &Ident,
    mode: StrictParserCallMode,
) -> Result<TokenStream2> {
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
    Ok(quote! {
        #first
            .then(#links)
            .map(|(first, links)| ::jbotci_tree::Chain::new(first, links))
    })
}

#[requires(true)]
#[ensures(true)]
fn strict_postfix_parser_expr_tokens(
    receiver: &ParserExpr,
    method: &Ident,
    args: &[Expr],
    arguments: &BTreeSet<String>,
    generation: &StrictParserGeneration<'_>,
    free_modifier_parser: &Ident,
    mode: StrictParserCallMode,
) -> Result<TokenStream2> {
    let inner =
        strict_parser_expr_tokens(receiver, arguments, generation, free_modifier_parser, mode)?;
    match (method.to_string().as_str(), args.len()) {
        ("elidable_terminator", 1) => Ok(inner),
        ("lookahead", 0) => Ok(quote!(generated_runtime::lookahead(#inner))),
        ("not", 0) => Ok(quote!(generated_runtime::not(#inner))),
        ("ignored", 0) => Ok(quote!(#inner.map(|_| ()))),
        ("ignore_then", 1) => {
            let parser = strict_rust_parser_expr_tokens(
                args.first().expect("length checked"),
                arguments,
                generation,
                free_modifier_parser,
                mode,
            )?;
            Ok(quote!(#inner.ignore_then(#parser)))
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
            Ok(quote! {
                generated_runtime::with_free_modifier_list(#inner, #free_modifier_list)
            })
        }
        _ => Err(syn::Error::new_spanned(
            method,
            "unsupported parser postfix method in strict parser generation",
        )),
    }
}

#[requires(true)]
#[ensures(true)]
fn strict_vector_parser_expr_tokens(
    expr: &VectorExpr,
    arguments: &BTreeSet<String>,
    generation: &StrictParserGeneration<'_>,
    free_modifier_parser: &Ident,
    mode: StrictParserCallMode,
) -> Result<TokenStream2> {
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
    let pattern = nested_sequence_pattern(bindings);
    let returns_vec1 =
        vector_output_is_vec1(expr, generation.type_env, arguments).ok_or_else(|| {
            syn::Error::new_spanned(
                expr.to_token_stream(),
                "cannot infer vector parser output type during strict parser generation",
            )
        })?;
    let finish = if returns_vec1 {
        quote! {
            vec1::Vec1::try_from_vec(__items)
                .expect("vector parser expression has statically non-zero cardinality")
        }
    } else {
        quote!(__items)
    };
    Ok(quote! {
        #parser.map(|#pattern| {
            let mut __items = Vec::new();
            #(#statements)*
            #finish
        })
    })
}

#[requires(true)]
#[ensures(true)]
fn recovered_parser_expr_tokens(
    expr: &ParserExpr,
    arguments: &BTreeSet<String>,
    generation: &RecoveredParserGeneration<'_>,
    free_modifier_parser: &Ident,
    mode: RecoveredParserCallMode,
) -> Result<TokenStream2> {
    match expr {
        ParserExpr::Rust(expr) => recovered_rust_parser_expr_tokens(
            expr,
            arguments,
            generation,
            free_modifier_parser,
            mode,
        ),
        ParserExpr::Vector(expr) => recovered_vector_parser_expr_tokens(
            expr,
            arguments,
            generation,
            free_modifier_parser,
            mode,
        ),
        ParserExpr::Chain(expr) => recovered_chain_parser_expr_tokens(
            expr,
            arguments,
            generation,
            free_modifier_parser,
            mode,
        ),
        ParserExpr::Postfix {
            receiver,
            method,
            args,
        } => recovered_postfix_parser_expr_tokens(
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

#[requires(true)]
#[ensures(true)]
fn recovered_chain_parser_expr_tokens(
    expr: &ChainExpr,
    arguments: &BTreeSet<String>,
    generation: &RecoveredParserGeneration<'_>,
    free_modifier_parser: &Ident,
    mode: RecoveredParserCallMode,
) -> Result<TokenStream2> {
    let first = recovered_parser_expr_tokens(
        &expr.first,
        arguments,
        generation,
        free_modifier_parser,
        mode,
    )?;
    let link = recovered_parser_expr_tokens(
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
    Ok(quote! {
        #first
            .then(#links)
            .map(|(first, links)| ::jbotci_tree::Chain::new(first, links))
    })
}

#[requires(true)]
#[ensures(true)]
fn recovered_postfix_parser_expr_tokens(
    receiver: &ParserExpr,
    method: &Ident,
    args: &[Expr],
    arguments: &BTreeSet<String>,
    generation: &RecoveredParserGeneration<'_>,
    free_modifier_parser: &Ident,
    mode: RecoveredParserCallMode,
) -> Result<TokenStream2> {
    let inner =
        recovered_parser_expr_tokens(receiver, arguments, generation, free_modifier_parser, mode)?;
    match (method.to_string().as_str(), args.len()) {
        ("elidable_terminator", 1) => Ok(inner),
        ("lookahead", 0) => Ok(quote!(generated_runtime::lookahead(#inner))),
        ("not", 0) => Ok(quote!(generated_runtime::not(#inner))),
        ("ignored", 0) => Ok(quote!(#inner.map(|_| ()))),
        ("ignore_then", 1) => {
            let parser = recovered_rust_parser_expr_tokens(
                args.first().expect("length checked"),
                arguments,
                generation,
                free_modifier_parser,
                mode,
            )?;
            Ok(quote!(#inner.ignore_then(#parser)))
        }
        ("wf" | "with_free_modifiers" | "prohibited_wf", 0) => {
            let free_modifier =
                recovered_free_modifier_argument_tokens(generation, free_modifier_parser, mode);
            let free_modifier_list = if method == "prohibited_wf" {
                quote!(generated_runtime::recovered_cll_prohibited_free_modifier_list_parser(
                    #free_modifier
                ))
            } else {
                quote!(generated_runtime::recovered_free_modifier_list_parser(#free_modifier))
            };
            let recovered_module = generation.recovered_module;
            Ok(quote! {
                #inner
                    .then(#free_modifier_list)
                    .map(|(value, free_modifiers)| #recovered_module::WithFreeModifiers {
                        value,
                        free_modifiers,
                    })
            })
        }
        _ => Err(syn::Error::new_spanned(
            method,
            "unsupported parser postfix method in recovered parser generation",
        )),
    }
}

#[requires(true)]
#[ensures(true)]
fn recovered_vector_parser_expr_tokens(
    expr: &VectorExpr,
    arguments: &BTreeSet<String>,
    generation: &RecoveredParserGeneration<'_>,
    free_modifier_parser: &Ident,
    mode: RecoveredParserCallMode,
) -> Result<TokenStream2> {
    let mut parsers = Vec::new();
    let mut bindings = Vec::new();
    let mut statements = Vec::new();
    for (index, item) in expr.items.iter().enumerate() {
        let binding = format_ident!("__vector_item_{index}");
        match item {
            VectorItem::One(expr) => {
                parsers.push(recovered_parser_expr_tokens(
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
                parsers.push(recovered_parser_expr_tokens(
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
                let inner = recovered_parser_expr_tokens(
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
                let inner = recovered_parser_expr_tokens(
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
                let inner = recovered_parser_expr_tokens(
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
                let inner = recovered_parser_expr_tokens(
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
                let parser = recovered_parser_expr_tokens(
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
    let pattern = nested_sequence_pattern(bindings);
    let returns_vec1 =
        vector_output_is_vec1(expr, generation.type_env, arguments).ok_or_else(|| {
            syn::Error::new_spanned(
                expr.to_token_stream(),
                "cannot infer vector parser output type during recovered parser generation",
            )
        })?;
    let finish = if returns_vec1 {
        quote! {
            vec1::Vec1::try_from_vec(__items)
                .expect("vector parser expression has statically non-zero cardinality")
        }
    } else {
        quote!(__items)
    };
    Ok(quote! {
        #parser.map(|#pattern| {
            let mut __items = Vec::new();
            #(#statements)*
            #finish
        })
    })
}

#[requires(true)]
#[ensures(true)]
fn nested_sequence_pattern(mut bindings: Vec<TokenStream2>) -> TokenStream2 {
    if bindings.is_empty() {
        return quote!(());
    }
    let mut pattern = bindings.remove(0);
    for binding in bindings {
        pattern = quote!((#pattern, #binding));
    }
    pattern
}

#[requires(true)]
#[ensures(true)]
fn strict_rust_parser_expr_tokens(
    expr: &Expr,
    arguments: &BTreeSet<String>,
    generation: &StrictParserGeneration<'_>,
    free_modifier_parser: &Ident,
    mode: StrictParserCallMode,
) -> Result<TokenStream2> {
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
            &array_vector_expr(array).ok_or_else(|| {
                syn::Error::new_spanned(
                    array,
                    "strict parser generation cannot infer an empty vector parser expression",
                )
            })?,
            arguments,
            generation,
            free_modifier_parser,
            mode,
        ),
        _ => Err(syn::Error::new_spanned(
            expr,
            "unsupported parser expression in strict parser generation",
        )),
    }
}

#[requires(true)]
#[ensures(true)]
fn strict_method_parser_expr_tokens(
    method: &ExprMethodCall,
    arguments: &BTreeSet<String>,
    generation: &StrictParserGeneration<'_>,
    free_modifier_parser: &Ident,
    mode: StrictParserCallMode,
) -> Result<TokenStream2> {
    if method.method == "elidable_terminator" && method.args.len() == 1 {
        strict_rust_parser_expr_tokens(
            &method.receiver,
            arguments,
            generation,
            free_modifier_parser,
            mode,
        )
    } else if method.method == "warn" && method.args.len() == 1 {
        let inner = strict_rust_parser_expr_tokens(
            &method.receiver,
            arguments,
            generation,
            free_modifier_parser,
            mode,
        )?;
        let construct = required_path_expr_last_segment(
            method.args.first().expect("length checked"),
            "warn() requires a generated construct path",
        )?;
        let construct = format_ident!("{construct}");
        Ok(quote! {
            #inner.map_with(
                |value, extra: &mut MapExtra<'tokens, '_>| {
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
        let selmaho = required_path_expr_last_segment(
            method.args.first().expect("length checked"),
            "not_next_selmaho() requires a selma'o path",
        )?;
        let selmaho = format_ident!("{selmaho}");
        Ok(quote! {
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
        let predicate = required_path_expr_last_segment(
            method.args.first().expect("length checked"),
            "not_next_token() requires a token predicate path",
        )?;
        let predicate = format_ident!("{predicate}");
        Ok(quote! {
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
        let rule_arg = method.args.first().expect("length checked");
        let rule = required_path_expr_last_segment(
            rule_arg,
            "not_next_rule() requires a grammar rule path",
        )?;
        if !generation.type_env.rules.contains_key(&rule) {
            return Err(syn::Error::new_spanned(
                rule_arg,
                "not_next_rule() names an unknown grammar rule",
            ));
        }
        let parser_arguments = generation
            .type_env
            .rule_arguments
            .get(&rule)
            .into_iter()
            .flatten()
            .map(|argument| strict_argument_parser_tokens(argument, arguments, generation, mode))
            .collect::<Result<Vec<_>>>()?;
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
        Ok(quote! {
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
        let guard_expr = method.args.first().expect("length checked");
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
        Ok(quote!(generated_runtime::followed_by(#inner, #guard)))
    } else if method.method == "complete_statement_item" && method.args.is_empty() {
        let inner = strict_rust_parser_expr_tokens(
            &method.receiver,
            arguments,
            generation,
            free_modifier_parser,
            mode,
        )?;
        Ok(quote!(generated_runtime::complete_statement_item(
            #inner,
            "complete statement item",
        )))
    } else if method.method == "complete_before_selmaho" && method.args.len() == 1 {
        let inner = strict_rust_parser_expr_tokens(
            &method.receiver,
            arguments,
            generation,
            free_modifier_parser,
            mode,
        )?;
        let selmaho = required_path_expr_last_segment(
            method.args.first().expect("length checked"),
            "complete_before_selmaho() requires a selma'o path",
        )?;
        let selmaho = format_ident!("{selmaho}");
        Ok(quote!(generated_runtime::complete_before_selmaho(
            #inner,
            Selmaho::#selmaho,
            "complete form before selma'o",
        )))
    } else if method.method == "lookahead" && method.args.is_empty() {
        let inner = strict_rust_parser_expr_tokens(
            &method.receiver,
            arguments,
            generation,
            free_modifier_parser,
            mode,
        )?;
        Ok(quote!(generated_runtime::lookahead(#inner)))
    } else if method.method == "not" && method.args.is_empty() {
        let inner = strict_rust_parser_expr_tokens(
            &method.receiver,
            arguments,
            generation,
            free_modifier_parser,
            mode,
        )?;
        Ok(quote!(generated_runtime::not(#inner)))
    } else if method.method == "ignored" && method.args.is_empty() {
        let inner = strict_rust_parser_expr_tokens(
            &method.receiver,
            arguments,
            generation,
            free_modifier_parser,
            mode,
        )?;
        Ok(quote!(#inner.map(|_| ())))
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
        Ok(quote!(#inner.ignore_then(#parser)))
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
        Ok(quote! {
            #inner
                .then(#free_modifier_list)
                .map(|(value, free_modifiers)| WithFreeModifiers::new(value, free_modifiers))
        })
    } else {
        Err(syn::Error::new_spanned(
            method,
            "unsupported parser method in strict parser generation",
        ))
    }
}

#[requires(true)]
#[ensures(true)]
fn strict_call_parser_expr_tokens(
    call: &ExprCall,
    arguments: &BTreeSet<String>,
    generation: &StrictParserGeneration<'_>,
    free_modifier_parser: &Ident,
    mode: StrictParserCallMode,
) -> Result<TokenStream2> {
    let function = call_name(call).ok_or_else(|| {
        syn::Error::new_spanned(call, "strict parser calls must use a named function")
    })?;
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
            let cmavo = required_path_expr_last_segment(
                call.args.first().expect("length checked"),
                "cmavo() requires a cmavo path",
            )?;
            let cmavo = format_ident!("{cmavo}");
            Ok(quote!(cmavo(Cmavo::#cmavo)))
        }
        ("selmaho", 1) => {
            let selmaho = required_path_expr_last_segment(
                call.args.first().expect("length checked"),
                "selmaho() requires a selma'o path",
            )?;
            let selmaho = format_ident!("{selmaho}");
            Ok(quote!(selmaho(Selmaho::#selmaho)))
        }
        ("word_category", 1) => {
            let category = required_path_expr_last_segment(
                call.args.first().expect("length checked"),
                "word_category() requires a word category path",
            )?;
            let category = format_ident!("{category}");
            Ok(quote!(generated_runtime::word_category(SyntaxWordCategory::#category)))
        }
        ("quote_marker", 1) => {
            let cmavo = required_path_expr_last_segment(
                call.args.first().expect("length checked"),
                "quote_marker() requires a cmavo path",
            )?;
            let cmavo = format_ident!("{cmavo}");
            Ok(quote!(generated_runtime::quote_marker(Cmavo::#cmavo)))
        }
        ("delimited_quote_marker", 1) => {
            let cmavo = required_path_expr_last_segment(
                call.args.first().expect("length checked"),
                "delimited_quote_marker() requires a cmavo path",
            )?;
            let cmavo = format_ident!("{cmavo}");
            Ok(quote!(generated_runtime::delimited_quote_marker(Cmavo::#cmavo)))
        }
        ("word_not_cmavo", _) if !call.args.is_empty() => {
            let terminators = call
                .args
                .iter()
                .map(|argument| {
                    let cmavo = required_path_expr_last_segment(
                        argument,
                        "word_not_cmavo() requires cmavo paths",
                    )?;
                    let cmavo = format_ident!("{cmavo}");
                    Ok(quote!(Cmavo::#cmavo))
                })
                .collect::<Result<Vec<_>>>()?;
            Ok(quote!(generated_runtime::word_not_cmavo(&[#(#terminators),*])))
        }
        ("feature", 2) => {
            let feature = required_path_expr_last_segment(
                call.args.first().expect("length checked"),
                "feature() requires a feature path",
            )?;
            let feature = format_ident!("{feature}");
            let inner = strict_rust_parser_expr_tokens(
                call.args.iter().nth(1).expect("length checked"),
                arguments,
                generation,
                free_modifier_parser,
                mode,
            )?;
            Ok(quote!(generated_runtime::feature_gate(
                generated_runtime::SyntaxGrammarFeature::#feature,
                #inner,
            )))
        }
        ("feature", 1) => {
            let feature = required_path_expr_last_segment(
                call.args.first().expect("length checked"),
                "feature() requires a feature path",
            )?;
            let feature = format_ident!("{feature}");
            Ok(quote!(generated_runtime::feature_gate(
                generated_runtime::SyntaxGrammarFeature::#feature,
                generated_runtime::empty(),
            )))
        }
        ("policy", 2) => {
            let policy = required_path_expr_last_segment(
                call.args.first().expect("length checked"),
                "policy() requires a policy path",
            )?;
            let policy = format_ident!("{policy}");
            let inner = strict_rust_parser_expr_tokens(
                call.args.iter().nth(1).expect("length checked"),
                arguments,
                generation,
                free_modifier_parser,
                mode,
            )?;
            Ok(quote!(generated_runtime::policy_gate(
                generated_runtime::SyntaxGrammarPolicyFlag::#policy,
                #inner,
            )))
        }
        ("policy", 1) => {
            let policy = required_path_expr_last_segment(
                call.args.first().expect("length checked"),
                "policy() requires a policy path",
            )?;
            let policy = format_ident!("{policy}");
            Ok(quote!(generated_runtime::policy_gate(
                generated_runtime::SyntaxGrammarPolicyFlag::#policy,
                generated_runtime::empty(),
            )))
        }
        ("relation_word", 0) => Ok(quote!(relation_word())),
        ("tanru_unit_relation_word", 0) => {
            Ok(quote!(generated_runtime::tanru_unit_relation_word()))
        }
        ("text_leading_cmevla_word", 0) => {
            Ok(quote!(generated_runtime::text_leading_cmevla_word()))
        }
        ("cmevla_word", 0) => {
            let parser = format_ident!("{function}");
            Ok(quote!(#parser()))
        }
        ("pa_word", 0) => Ok(quote!(pa_word())),
        ("opt", 1) => {
            let inner = strict_rust_parser_expr_tokens(
                call.args.first().expect("length checked"),
                arguments,
                generation,
                free_modifier_parser,
                mode,
            )?;
            Ok(quote!(generated_runtime::strict_optional(#inner)))
        }
        ("boxed", 1) => {
            let inner = strict_rust_parser_expr_tokens(
                call.args.first().expect("length checked"),
                arguments,
                generation,
                free_modifier_parser,
                mode,
            )?;
            Ok(quote!(#inner.map(Box::new)))
        }
        ("arc", 1) => {
            let inner = strict_rust_parser_expr_tokens(
                call.args.first().expect("length checked"),
                arguments,
                generation,
                free_modifier_parser,
                mode,
            )?;
            Ok(quote!(#inner.map(std::sync::Arc::new)))
        }
        ("choice", 1) => {
            let alternatives = call
                .args
                .first()
                .map(choice_alternative_exprs)
                .map(|exprs| {
                    strict_choice_alternative_parser_tokens(
                        exprs,
                        arguments,
                        generation,
                        free_modifier_parser,
                        mode,
                    )
                })
                .expect("length checked")?;
            strict_choice_chain(alternatives, call)
        }
        ("choice", _) => {
            let alternatives = strict_choice_alternative_parser_tokens(
                call.args.iter().collect(),
                arguments,
                generation,
                free_modifier_parser,
                mode,
            )?;
            strict_choice_chain(alternatives, call)
        }
        ("empty", 0) => Ok(quote!(generated_runtime::empty())),
        ("eof", 0) => Ok(quote!(generated_runtime::eof())),
        _ => Err(syn::Error::new_spanned(
            call,
            "unsupported parser call in strict parser generation",
        )),
    }
}

#[requires(true)]
#[ensures(true)]
fn strict_path_parser_expr_tokens(
    path: &ExprPath,
    arguments: &BTreeSet<String>,
    generation: &StrictParserGeneration<'_>,
    free_modifier_parser: &Ident,
    mode: StrictParserCallMode,
) -> Result<TokenStream2> {
    if path.qself.is_none()
        && path.path.segments.len() == 1
        && let Some(segment) = path.path.segments.first()
    {
        let name = segment.ident.to_string();
        if arguments.contains(&name) {
            return strict_argument_parser_tokens(&name, arguments, generation, mode);
        }
        if generation.type_env.rules.contains_key(&name) {
            return strict_rule_call_parser_tokens(
                &name,
                std::iter::empty(),
                arguments,
                generation,
                free_modifier_parser,
                mode,
            );
        }
    }
    Err(syn::Error::new_spanned(
        path,
        "unknown parser rule or argument in strict parser generation",
    ))
}

#[requires(true)]
#[ensures(true)]
fn strict_tuple_parser_expr_tokens(
    tuple: &ExprTuple,
    arguments: &BTreeSet<String>,
    generation: &StrictParserGeneration<'_>,
    free_modifier_parser: &Ident,
    mode: StrictParserCallMode,
) -> Result<TokenStream2> {
    let parts = tuple
        .elems
        .iter()
        .map(|expr| {
            strict_rust_parser_expr_tokens(expr, arguments, generation, free_modifier_parser, mode)
        })
        .collect::<Result<Vec<_>>>()?;
    strict_sequence_expr_chain(parts)
}

#[requires(!function.is_empty())]
#[ensures(true)]
fn strict_rule_call_parser_tokens<'a>(
    function: &str,
    argument_exprs: impl Iterator<Item = &'a Expr>,
    arguments: &BTreeSet<String>,
    generation: &StrictParserGeneration<'_>,
    free_modifier_parser: &Ident,
    mode: StrictParserCallMode,
) -> Result<TokenStream2> {
    if !generation.type_env.rules.contains_key(function) {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            format!("unknown grammar rule `{function}` in strict parser generation"),
        ));
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
        .collect::<Result<Vec<_>>>()?;
    Ok(strict_rule_call_tokens(
        function,
        parser_arguments,
        generation,
        free_modifier_parser,
        call_mode,
    ))
}

#[requires(true)]
#[ensures(true)]
fn strict_rule_call_by_argument_names(
    function: &Ident,
    argument_names: &[String],
    available_arguments: &BTreeSet<String>,
    generation: &StrictParserGeneration<'_>,
    free_modifier_parser: &Ident,
    mode: StrictParserCallMode,
) -> Result<TokenStream2> {
    let function_name = function.to_string();
    if !generation.type_env.rules.contains_key(&function_name) {
        return Err(syn::Error::new_spanned(
            function,
            "unknown grammar rule in strict parser generation",
        ));
    }
    let call_mode = if mode == StrictParserCallMode::External
        || (generation.generate_model && !generation.rule_has_local_parser(&function_name))
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
        .collect::<Result<Vec<_>>>()?;
    Ok(strict_rule_call_tokens(
        &function_name,
        parser_arguments,
        generation,
        free_modifier_parser,
        call_mode,
    ))
}

#[requires(!function.is_empty())]
#[ensures(true)]
fn strict_rule_call_tokens(
    function: &str,
    parser_arguments: Vec<TokenStream2>,
    generation: &StrictParserGeneration<'_>,
    free_modifier_parser: &Ident,
    call_mode: StrictParserCallMode,
) -> TokenStream2 {
    let parser_name = format_ident!("strict_{}_parser", function);
    let parser_name = if call_mode == StrictParserCallMode::External {
        quote!(super::#parser_name)
    } else {
        quote!(#parser_name)
    };
    let free_modifier =
        strict_free_modifier_argument_tokens(generation, free_modifier_parser, call_mode);
    quote!(#parser_name(
        #(#parser_arguments,)*
        #free_modifier
    ).map(generated_runtime::SharedSyntaxOutput::into_owned))
}

#[requires(!argument.is_empty())]
#[ensures(true)]
fn strict_argument_parser_tokens(
    argument: &str,
    arguments: &BTreeSet<String>,
    generation: &StrictParserGeneration<'_>,
    mode: StrictParserCallMode,
) -> Result<TokenStream2> {
    if !arguments.contains(argument) {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            format!("parser argument `{argument}` is not available in this strict parser"),
        ));
    }
    let argument = format_ident!("{argument}");
    if mode == StrictParserCallMode::External
        && generation.recursive_has_local_parser(&argument.to_string())
    {
        Ok(generation.external_recursive_parser(&argument))
    } else {
        Ok(quote!(#argument.clone()))
    }
}

#[requires(true)]
#[ensures(true)]
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

#[requires(true)]
#[ensures(true)]
fn strict_sequence_expr_chain(mut parts: Vec<TokenStream2>) -> Result<TokenStream2> {
    if parts.is_empty() {
        return Ok(quote!(generated_runtime::empty()));
    }
    let mut parser = parts.remove(0);
    for part in parts {
        parser = quote!(#parser.then(#part));
    }
    Ok(parser)
}

#[requires(true)]
#[ensures(true)]
fn choice_alternative_exprs(expr: &Expr) -> Vec<&Expr> {
    if let Expr::Tuple(ExprTuple { elems, .. }) = expr {
        elems.iter().collect()
    } else {
        vec![expr]
    }
}

#[requires(true)]
#[ensures(true)]
fn strict_choice_alternative_parser_tokens(
    exprs: Vec<&Expr>,
    arguments: &BTreeSet<String>,
    generation: &StrictParserGeneration<'_>,
    free_modifier_parser: &Ident,
    mode: StrictParserCallMode,
) -> Result<Vec<TokenStream2>> {
    let Some(first_expr) = exprs.first() else {
        return Ok(Vec::new());
    };
    let argument_types = argument_type_map(arguments, generation.type_env).ok_or_else(|| {
        syn::Error::new_spanned(
            *first_expr,
            "cannot infer parser argument types during strict choice generation",
        )
    })?;
    let outputs = exprs
        .iter()
        .map(|expr| {
            rust_parser_output_type(expr, generation.type_env, &argument_types).ok_or_else(|| {
                syn::Error::new_spanned(
                    *expr,
                    "cannot infer choice alternative output type during strict parser generation",
                )
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let target_output = common_choice_output_type(&outputs).ok_or_else(|| {
        syn::Error::new_spanned(
            *first_expr,
            "choice alternatives have incompatible output types during strict parser generation",
        )
    })?;
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
            coerce_choice_parser_output(parser, output, &target_output).ok_or_else(|| {
                syn::Error::new_spanned(
                    *expr,
                    "choice alternative output cannot be coerced to the common output type",
                )
            })
        })
        .collect()
}

#[requires(true)]
#[ensures(true)]
fn strict_choice_chain(
    mut alternatives: Vec<TokenStream2>,
    span: impl ToTokens,
) -> Result<TokenStream2> {
    if alternatives.is_empty() {
        return Err(syn::Error::new_spanned(
            span,
            "strict parser choice must have at least one alternative",
        ));
    }
    if alternatives.len() == 1 {
        return Ok(alternatives.pop().expect("length checked"));
    }
    let alternatives = alternatives
        .into_iter()
        .map(|alternative| quote!(#alternative.boxed()));
    Ok(quote!(generated_runtime::strict_ordered_choice_parsers(
        vec![
            #(#alternatives),*
        ]
    )))
}

#[requires(true)]
#[ensures(true)]
fn recovered_rust_parser_expr_tokens(
    expr: &Expr,
    arguments: &BTreeSet<String>,
    generation: &RecoveredParserGeneration<'_>,
    free_modifier_parser: &Ident,
    mode: RecoveredParserCallMode,
) -> Result<TokenStream2> {
    match expr {
        Expr::Call(call) => recovered_call_parser_expr_tokens(
            call,
            arguments,
            generation,
            free_modifier_parser,
            mode,
        ),
        Expr::MethodCall(method) => recovered_method_parser_expr_tokens(
            method,
            arguments,
            generation,
            free_modifier_parser,
            mode,
        ),
        Expr::Path(path) => recovered_path_parser_expr_tokens(
            path,
            arguments,
            generation,
            free_modifier_parser,
            mode,
        ),
        Expr::Tuple(tuple) => recovered_tuple_parser_expr_tokens(
            tuple,
            arguments,
            generation,
            free_modifier_parser,
            mode,
        ),
        Expr::Array(array) => recovered_vector_parser_expr_tokens(
            &array_vector_expr(array).ok_or_else(|| {
                syn::Error::new_spanned(
                    array,
                    "recovered parser generation cannot infer an empty vector parser expression",
                )
            })?,
            arguments,
            generation,
            free_modifier_parser,
            mode,
        ),
        _ => Err(syn::Error::new_spanned(
            expr,
            "unsupported parser expression in recovered parser generation",
        )),
    }
}

#[requires(true)]
#[ensures(true)]
fn recovered_method_parser_expr_tokens(
    method: &ExprMethodCall,
    arguments: &BTreeSet<String>,
    generation: &RecoveredParserGeneration<'_>,
    free_modifier_parser: &Ident,
    mode: RecoveredParserCallMode,
) -> Result<TokenStream2> {
    if method.method == "elidable_terminator" && method.args.len() == 1 {
        recovered_rust_parser_expr_tokens(
            &method.receiver,
            arguments,
            generation,
            free_modifier_parser,
            mode,
        )
    } else if method.method == "warn" && method.args.len() == 1 {
        let inner = recovered_rust_parser_expr_tokens(
            &method.receiver,
            arguments,
            generation,
            free_modifier_parser,
            mode,
        )?;
        let construct = required_path_expr_last_segment(
            method.args.first().expect("length checked"),
            "warn() requires a generated construct path",
        )?;
        let construct = format_ident!("{construct}");
        let recovered_module = generation.recovered_module;
        Ok(quote! {
            #inner.map_with(
                |value, extra: &mut MapExtra<'tokens, '_>| {
                    if let #recovered_module::Recovered::Valid(token) = &value {
                        extra.state().warn(ExperimentalConstruct::#construct, token.as_ref());
                    }
                    value
                },
            )
        })
    } else if method.method == "not_next_selmaho" && method.args.len() == 1 {
        let inner = recovered_rust_parser_expr_tokens(
            &method.receiver,
            arguments,
            generation,
            free_modifier_parser,
            mode,
        )?;
        let selmaho = required_path_expr_last_segment(
            method.args.first().expect("length checked"),
            "not_next_selmaho() requires a selma'o path",
        )?;
        let selmaho = format_ident!("{selmaho}");
        Ok(quote! {
            #inner
                .then(generated_runtime::not_next_selmaho(Selmaho::#selmaho))
                .map(|(value, _)| value)
        })
    } else if method.method == "not_next_token" && method.args.len() == 1 {
        let inner = recovered_rust_parser_expr_tokens(
            &method.receiver,
            arguments,
            generation,
            free_modifier_parser,
            mode,
        )?;
        let predicate = required_path_expr_last_segment(
            method.args.first().expect("length checked"),
            "not_next_token() requires a token predicate path",
        )?;
        let predicate = format_ident!("{predicate}");
        Ok(quote! {
            #inner
                .then(generated_runtime::not_next_token(SyntaxGrammarTokenPredicate::#predicate))
                .map(|(value, _)| value)
        })
    } else if method.method == "followed_by" && method.args.len() == 1 {
        let guard_expr = method.args.first().expect("length checked");
        let inner = recovered_rust_parser_expr_tokens(
            &method.receiver,
            arguments,
            generation,
            free_modifier_parser,
            mode,
        )?;
        let guard = recovered_rust_parser_expr_tokens(
            guard_expr,
            arguments,
            generation,
            free_modifier_parser,
            mode,
        )?;
        Ok(quote!(generated_runtime::followed_by(#inner, #guard)))
    } else if method.method == "complete_statement_item" && method.args.is_empty() {
        let inner = recovered_rust_parser_expr_tokens(
            &method.receiver,
            arguments,
            generation,
            free_modifier_parser,
            mode,
        )?;
        Ok(quote!(generated_runtime::complete_statement_item(
            #inner,
            "complete statement item",
        )))
    } else if method.method == "complete_before_selmaho" && method.args.len() == 1 {
        let inner = recovered_rust_parser_expr_tokens(
            &method.receiver,
            arguments,
            generation,
            free_modifier_parser,
            mode,
        )?;
        let selmaho = required_path_expr_last_segment(
            method.args.first().expect("length checked"),
            "complete_before_selmaho() requires a selma'o path",
        )?;
        let selmaho = format_ident!("{selmaho}");
        Ok(quote!(generated_runtime::complete_before_selmaho(
            #inner,
            Selmaho::#selmaho,
            "complete form before selma'o",
        )))
    } else if method.method == "lookahead" && method.args.is_empty() {
        let inner = recovered_rust_parser_expr_tokens(
            &method.receiver,
            arguments,
            generation,
            free_modifier_parser,
            mode,
        )?;
        Ok(quote!(generated_runtime::lookahead(#inner)))
    } else if method.method == "not" && method.args.is_empty() {
        let inner = recovered_rust_parser_expr_tokens(
            &method.receiver,
            arguments,
            generation,
            free_modifier_parser,
            mode,
        )?;
        Ok(quote!(generated_runtime::not(#inner)))
    } else if method.method == "ignored" && method.args.is_empty() {
        let inner = recovered_rust_parser_expr_tokens(
            &method.receiver,
            arguments,
            generation,
            free_modifier_parser,
            mode,
        )?;
        Ok(quote!(#inner.map(|_| ())))
    } else if method.method == "ignore_then" && method.args.len() == 1 {
        let inner = recovered_rust_parser_expr_tokens(
            &method.receiver,
            arguments,
            generation,
            free_modifier_parser,
            mode,
        )?;
        let parser = recovered_rust_parser_expr_tokens(
            method.args.first().expect("length checked"),
            arguments,
            generation,
            free_modifier_parser,
            mode,
        )?;
        Ok(quote!(#inner.ignore_then(#parser)))
    } else if (method.method == "wf"
        || method.method == "with_free_modifiers"
        || method.method == "prohibited_wf")
        && method.args.is_empty()
    {
        let inner = recovered_rust_parser_expr_tokens(
            &method.receiver,
            arguments,
            generation,
            free_modifier_parser,
            mode,
        )?;
        let free_modifier =
            recovered_free_modifier_argument_tokens(generation, free_modifier_parser, mode);
        let free_modifier_list = if method.method == "prohibited_wf" {
            quote!(generated_runtime::recovered_cll_prohibited_free_modifier_list_parser(
                #free_modifier
            ))
        } else {
            quote!(generated_runtime::recovered_free_modifier_list_parser(#free_modifier))
        };
        let recovered_module = generation.recovered_module;
        Ok(quote! {
            #inner
                .then(#free_modifier_list)
                .map(|(value, free_modifiers)| #recovered_module::WithFreeModifiers {
                    value,
                    free_modifiers,
                })
        })
    } else {
        Err(syn::Error::new_spanned(
            method,
            "unsupported parser method in recovered parser generation",
        ))
    }
}

#[requires(true)]
#[ensures(true)]
fn recovered_call_parser_expr_tokens(
    call: &ExprCall,
    arguments: &BTreeSet<String>,
    generation: &RecoveredParserGeneration<'_>,
    free_modifier_parser: &Ident,
    mode: RecoveredParserCallMode,
) -> Result<TokenStream2> {
    let function = call_name(call).ok_or_else(|| {
        syn::Error::new_spanned(call, "recovered parser calls must use a named function")
    })?;
    if generation.type_env.rules.contains_key(&function) {
        return recovered_rule_call_parser_tokens(
            &function,
            call.args.iter(),
            arguments,
            generation,
            free_modifier_parser,
            mode,
            true,
        );
    }
    let recovered_module = generation.recovered_module;
    match (function.as_str(), call.args.len()) {
        ("cmavo", 1) => {
            let cmavo = required_path_expr_last_segment(
                call.args.first().expect("length checked"),
                "cmavo() requires a cmavo path",
            )?;
            let cmavo = format_ident!("{cmavo}");
            Ok(quote!(cmavo(Cmavo::#cmavo).map(#recovered_module::Recovered::valid)))
        }
        ("selmaho", 1) => {
            let selmaho = required_path_expr_last_segment(
                call.args.first().expect("length checked"),
                "selmaho() requires a selma'o path",
            )?;
            let selmaho = format_ident!("{selmaho}");
            Ok(quote!(selmaho(Selmaho::#selmaho).map(#recovered_module::Recovered::valid)))
        }
        ("word_category", 1) => {
            let category = required_path_expr_last_segment(
                call.args.first().expect("length checked"),
                "word_category() requires a word category path",
            )?;
            let category = format_ident!("{category}");
            Ok(
                quote!(generated_runtime::word_category(SyntaxWordCategory::#category).map(#recovered_module::Recovered::valid)),
            )
        }
        ("quote_marker", 1) => {
            let cmavo = required_path_expr_last_segment(
                call.args.first().expect("length checked"),
                "quote_marker() requires a cmavo path",
            )?;
            let cmavo = format_ident!("{cmavo}");
            Ok(
                quote!(generated_runtime::quote_marker(Cmavo::#cmavo).map(#recovered_module::Recovered::valid)),
            )
        }
        ("delimited_quote_marker", 1) => {
            let cmavo = required_path_expr_last_segment(
                call.args.first().expect("length checked"),
                "delimited_quote_marker() requires a cmavo path",
            )?;
            let cmavo = format_ident!("{cmavo}");
            Ok(
                quote!(generated_runtime::delimited_quote_marker(Cmavo::#cmavo).map(#recovered_module::Recovered::valid)),
            )
        }
        ("word_not_cmavo", _) if !call.args.is_empty() => {
            let terminators = call
                .args
                .iter()
                .map(|argument| {
                    let cmavo = required_path_expr_last_segment(
                        argument,
                        "word_not_cmavo() requires cmavo paths",
                    )?;
                    let cmavo = format_ident!("{cmavo}");
                    Ok(quote!(Cmavo::#cmavo))
                })
                .collect::<Result<Vec<_>>>()?;
            Ok(
                quote!(generated_runtime::word_not_cmavo(&[#(#terminators),*]).map(#recovered_module::Recovered::valid)),
            )
        }
        ("feature", 2) => {
            let feature = required_path_expr_last_segment(
                call.args.first().expect("length checked"),
                "feature() requires a feature path",
            )?;
            let feature = format_ident!("{feature}");
            let inner = recovered_rust_parser_expr_tokens(
                call.args.iter().nth(1).expect("length checked"),
                arguments,
                generation,
                free_modifier_parser,
                mode,
            )?;
            Ok(quote!(generated_runtime::feature_gate(
                generated_runtime::SyntaxGrammarFeature::#feature,
                #inner,
            )))
        }
        ("feature", 1) => {
            let feature = required_path_expr_last_segment(
                call.args.first().expect("length checked"),
                "feature() requires a feature path",
            )?;
            let feature = format_ident!("{feature}");
            Ok(quote!(generated_runtime::feature_gate(
                generated_runtime::SyntaxGrammarFeature::#feature,
                generated_runtime::empty(),
            )))
        }
        ("policy", 2) => {
            let policy = required_path_expr_last_segment(
                call.args.first().expect("length checked"),
                "policy() requires a policy path",
            )?;
            let policy = format_ident!("{policy}");
            let inner = recovered_rust_parser_expr_tokens(
                call.args.iter().nth(1).expect("length checked"),
                arguments,
                generation,
                free_modifier_parser,
                mode,
            )?;
            Ok(quote!(generated_runtime::policy_gate(
                generated_runtime::SyntaxGrammarPolicyFlag::#policy,
                #inner,
            )))
        }
        ("policy", 1) => {
            let policy = required_path_expr_last_segment(
                call.args.first().expect("length checked"),
                "policy() requires a policy path",
            )?;
            let policy = format_ident!("{policy}");
            Ok(quote!(generated_runtime::policy_gate(
                generated_runtime::SyntaxGrammarPolicyFlag::#policy,
                generated_runtime::empty(),
            )))
        }
        ("relation_word", 0) => {
            Ok(quote!(relation_word().map(#recovered_module::Recovered::valid)))
        }
        ("tanru_unit_relation_word", 0) => Ok(
            quote!(generated_runtime::tanru_unit_relation_word().map(#recovered_module::Recovered::valid)),
        ),
        ("text_leading_cmevla_word", 0) => Ok(
            quote!(generated_runtime::text_leading_cmevla_word().map(#recovered_module::Recovered::valid)),
        ),
        ("cmevla_word", 0) => Ok(quote!(cmevla_word().map(#recovered_module::Recovered::valid))),
        ("pa_word", 0) => Ok(quote!(pa_word().map(#recovered_module::Recovered::valid))),
        ("opt", 1) => {
            let inner = recovered_rust_parser_expr_tokens(
                call.args.first().expect("length checked"),
                arguments,
                generation,
                free_modifier_parser,
                mode,
            )?;
            Ok(quote!(generated_runtime::strict_optional(#inner)))
        }
        ("boxed", 1) => {
            let inner = recovered_rust_parser_expr_tokens(
                call.args.first().expect("length checked"),
                arguments,
                generation,
                free_modifier_parser,
                mode,
            )?;
            Ok(quote!(#inner.map(Box::new)))
        }
        ("arc", 1) => {
            let inner = recovered_rust_parser_expr_tokens(
                call.args.first().expect("length checked"),
                arguments,
                generation,
                free_modifier_parser,
                mode,
            )?;
            Ok(quote!(#inner.map(std::sync::Arc::new)))
        }
        ("choice", 1) => {
            let alternatives = call
                .args
                .first()
                .map(choice_alternative_exprs)
                .map(|exprs| {
                    recovered_choice_alternative_parser_tokens(
                        exprs,
                        arguments,
                        generation,
                        free_modifier_parser,
                        mode,
                    )
                })
                .expect("length checked")?;
            strict_choice_chain(alternatives, call)
        }
        ("choice", _) => {
            let alternatives = recovered_choice_alternative_parser_tokens(
                call.args.iter().collect(),
                arguments,
                generation,
                free_modifier_parser,
                mode,
            )?;
            strict_choice_chain(alternatives, call)
        }
        ("empty", 0) => Ok(quote!(generated_runtime::empty())),
        ("eof", 0) => Ok(quote!(generated_runtime::eof())),
        _ => Err(syn::Error::new_spanned(
            call,
            "unsupported parser call in recovered parser generation",
        )),
    }
}

#[requires(true)]
#[ensures(true)]
fn recovered_path_parser_expr_tokens(
    path: &ExprPath,
    arguments: &BTreeSet<String>,
    generation: &RecoveredParserGeneration<'_>,
    free_modifier_parser: &Ident,
    mode: RecoveredParserCallMode,
) -> Result<TokenStream2> {
    if path.qself.is_none()
        && path.path.segments.len() == 1
        && let Some(segment) = path.path.segments.first()
    {
        let name = segment.ident.to_string();
        if arguments.contains(&name) {
            return recovered_argument_parser_tokens(&name, arguments, generation, mode, true);
        }
        if generation.type_env.rules.contains_key(&name) {
            return recovered_rule_call_parser_tokens(
                &name,
                std::iter::empty(),
                arguments,
                generation,
                free_modifier_parser,
                mode,
                true,
            );
        }
    }
    Err(syn::Error::new_spanned(
        path,
        "unknown parser rule or argument in recovered parser generation",
    ))
}

#[requires(true)]
#[ensures(true)]
fn recovered_tuple_parser_expr_tokens(
    tuple: &ExprTuple,
    arguments: &BTreeSet<String>,
    generation: &RecoveredParserGeneration<'_>,
    free_modifier_parser: &Ident,
    mode: RecoveredParserCallMode,
) -> Result<TokenStream2> {
    let parts = tuple
        .elems
        .iter()
        .map(|expr| {
            recovered_rust_parser_expr_tokens(
                expr,
                arguments,
                generation,
                free_modifier_parser,
                mode,
            )
        })
        .collect::<Result<Vec<_>>>()?;
    strict_sequence_expr_chain(parts)
}

#[requires(true)]
#[ensures(true)]
fn recovered_rule_argument_expr_tokens(
    expr: &Expr,
    arguments: &BTreeSet<String>,
    generation: &RecoveredParserGeneration<'_>,
    free_modifier_parser: &Ident,
    mode: RecoveredParserCallMode,
) -> Result<TokenStream2> {
    if let Expr::Path(path) = expr
        && path.qself.is_none()
        && path.path.segments.len() == 1
        && let Some(segment) = path.path.segments.first()
    {
        let name = segment.ident.to_string();
        if arguments.contains(&name) {
            return recovered_argument_parser_tokens(&name, arguments, generation, mode, false);
        }
        if generation.type_env.rules.contains_key(&name) {
            return recovered_rule_call_parser_tokens(
                &name,
                std::iter::empty(),
                arguments,
                generation,
                free_modifier_parser,
                mode,
                false,
            );
        }
    }
    if let Expr::Call(call) = expr {
        let Some(function) = call_name(call) else {
            return recovered_rust_parser_expr_tokens(
                expr,
                arguments,
                generation,
                free_modifier_parser,
                mode,
            );
        };
        if generation.type_env.rules.contains_key(&function) {
            return recovered_rule_call_parser_tokens(
                &function,
                call.args.iter(),
                arguments,
                generation,
                free_modifier_parser,
                mode,
                false,
            );
        }
    }
    recovered_rust_parser_expr_tokens(expr, arguments, generation, free_modifier_parser, mode)
}

#[requires(!function.is_empty())]
#[ensures(true)]
fn recovered_rule_call_parser_tokens<'a>(
    function: &str,
    argument_exprs: impl Iterator<Item = &'a Expr>,
    arguments: &BTreeSet<String>,
    generation: &RecoveredParserGeneration<'_>,
    free_modifier_parser: &Ident,
    mode: RecoveredParserCallMode,
    wrap_generated_model_output: bool,
) -> Result<TokenStream2> {
    if !generation.type_env.rules.contains_key(function) {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            format!("unknown grammar rule `{function}` in recovered parser generation"),
        ));
    }
    let call_mode = if mode == RecoveredParserCallMode::External
        || !generation.rule_has_local_parser(function)
    {
        RecoveredParserCallMode::External
    } else {
        RecoveredParserCallMode::Local
    };
    let parser_arguments = argument_exprs
        .map(|argument| {
            recovered_rule_argument_expr_tokens(
                argument,
                arguments,
                generation,
                free_modifier_parser,
                call_mode,
            )
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(recovered_rule_call_tokens(
        function,
        parser_arguments,
        generation,
        free_modifier_parser,
        call_mode,
        wrap_generated_model_output,
    ))
}

#[requires(true)]
#[ensures(true)]
fn recovered_rule_call_by_argument_names(
    function: &Ident,
    argument_names: &[String],
    available_arguments: &BTreeSet<String>,
    generation: &RecoveredParserGeneration<'_>,
    free_modifier_parser: &Ident,
    mode: RecoveredParserCallMode,
) -> Result<TokenStream2> {
    let function_name = function.to_string();
    if !generation.type_env.rules.contains_key(&function_name) {
        return Err(syn::Error::new_spanned(
            function,
            "unknown grammar rule in recovered parser generation",
        ));
    }
    let call_mode = if mode == RecoveredParserCallMode::External
        || !generation.rule_has_local_parser(&function_name)
    {
        RecoveredParserCallMode::External
    } else {
        RecoveredParserCallMode::Local
    };
    let parser_arguments = argument_names
        .iter()
        .map(|argument| {
            recovered_argument_parser_tokens(
                argument,
                available_arguments,
                generation,
                call_mode,
                false,
            )
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(recovered_rule_call_tokens(
        &function_name,
        parser_arguments,
        generation,
        free_modifier_parser,
        call_mode,
        true,
    ))
}

#[requires(!function.is_empty())]
#[ensures(true)]
fn recovered_rule_call_tokens(
    function: &str,
    parser_arguments: Vec<TokenStream2>,
    generation: &RecoveredParserGeneration<'_>,
    free_modifier_parser: &Ident,
    call_mode: RecoveredParserCallMode,
    wrap_generated_model_output: bool,
) -> TokenStream2 {
    let parser_name = format_ident!("recovered_{}_parser", function);
    let parser_name = if call_mode == RecoveredParserCallMode::External {
        quote!(super::#parser_name)
    } else {
        quote!(#parser_name)
    };
    let free_modifier =
        recovered_free_modifier_argument_tokens(generation, free_modifier_parser, call_mode);
    let parser = quote!(#parser_name(
        #(#parser_arguments,)*
        #free_modifier,
        __generated_recovery_rules.clone()
    ).map(generated_runtime::SharedSyntaxOutput::into_owned));
    if wrap_generated_model_output && generation.rule_is_generated_model(function) {
        let recovered_module = generation.recovered_module;
        quote!(#parser.map(#recovered_module::Recovered::valid))
    } else {
        parser
    }
}

#[requires(!argument.is_empty())]
#[ensures(true)]
fn recovered_argument_parser_tokens(
    argument: &str,
    arguments: &BTreeSet<String>,
    generation: &RecoveredParserGeneration<'_>,
    mode: RecoveredParserCallMode,
    wrap_generated_model_output: bool,
) -> Result<TokenStream2> {
    if !arguments.contains(argument) {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            format!("parser argument `{argument}` is not available in this recovered parser"),
        ));
    }
    let argument = format_ident!("{argument}");
    let parser = if mode == RecoveredParserCallMode::External
        && generation.recursive_has_local_parser(&argument.to_string())
    {
        generation.external_recursive_parser(&argument)
    } else {
        quote!(#argument.clone())
    };
    if wrap_generated_model_output && generation.recursive_is_generated_model(&argument.to_string())
    {
        let recovered_module = generation.recovered_module;
        Ok(quote!(#parser.map(#recovered_module::Recovered::valid)))
    } else {
        Ok(parser)
    }
}

#[requires(true)]
#[ensures(true)]
fn recovered_free_modifier_argument_tokens(
    generation: &RecoveredParserGeneration<'_>,
    free_modifier_parser: &Ident,
    mode: RecoveredParserCallMode,
) -> TokenStream2 {
    if mode == RecoveredParserCallMode::External {
        generation.external_free_modifier_parser()
    } else {
        quote!(#free_modifier_parser.clone())
    }
}

#[requires(true)]
#[ensures(true)]
fn recovered_choice_alternative_parser_tokens(
    exprs: Vec<&Expr>,
    arguments: &BTreeSet<String>,
    generation: &RecoveredParserGeneration<'_>,
    free_modifier_parser: &Ident,
    mode: RecoveredParserCallMode,
) -> Result<Vec<TokenStream2>> {
    let Some(first_expr) = exprs.first() else {
        return Ok(Vec::new());
    };
    let argument_types = argument_type_map(arguments, generation.type_env).ok_or_else(|| {
        syn::Error::new_spanned(
            *first_expr,
            "cannot infer parser argument types during recovered choice generation",
        )
    })?;
    let outputs = exprs
        .iter()
        .map(|expr| {
            rust_parser_output_type(expr, generation.type_env, &argument_types)
                .and_then(|ty| syn::parse2::<Type>(ty).ok())
                .map(|ty| {
                    recovered_field_type_tokens(
                        &ty,
                        generation.model_outputs,
                        None,
                        generation.recovered_module,
                    )
                })
                .ok_or_else(|| {
                    syn::Error::new_spanned(
                        *expr,
                        "cannot infer choice alternative output type during recovered parser generation",
                    )
                })
        })
        .collect::<Result<Vec<_>>>()?;
    let target_output = common_choice_output_type(&outputs).ok_or_else(|| {
        syn::Error::new_spanned(
            *first_expr,
            "choice alternatives have incompatible output types during recovered parser generation",
        )
    })?;
    exprs
        .iter()
        .zip(outputs.iter())
        .map(|(expr, output)| {
            let parser = recovered_rust_parser_expr_tokens(
                expr,
                arguments,
                generation,
                free_modifier_parser,
                mode,
            )?;
            coerce_choice_parser_output(parser, output, &target_output).ok_or_else(|| {
                syn::Error::new_spanned(
                    *expr,
                    "choice alternative output cannot be coerced to the common recovered output type",
                )
            })
        })
        .collect()
}

#[requires(true)]
#[ensures(true)]
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

#[requires(true)]
#[ensures(true)]
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

#[requires(true)]
#[ensures(true)]
fn postfix_parser_output_type(
    receiver: &ParserExpr,
    method: &Ident,
    args: &[Expr],
    type_env: &GrammarTypeEnv,
    arguments: &BTreeMap<String, Type>,
) -> Option<TokenStream2> {
    match (method.to_string().as_str(), args.len()) {
        ("elidable_terminator", 1) => parser_output_type(receiver, type_env, arguments),
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

#[requires(true)]
#[ensures(true)]
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

#[requires(true)]
#[ensures(true)]
fn vector_output_is_vec1(
    expr: &VectorExpr,
    type_env: &GrammarTypeEnv,
    argument_names: &BTreeSet<String>,
) -> Option<bool> {
    let arguments = argument_type_map(argument_names, type_env)?;
    Some(vector_min_cardinality(expr, type_env, &arguments)? > 0)
}

#[requires(true)]
#[ensures(true)]
fn argument_type_map(
    argument_names: &BTreeSet<String>,
    type_env: &GrammarTypeEnv,
) -> Option<BTreeMap<String, Type>> {
    argument_names
        .iter()
        .map(|name| Some((name.clone(), type_env.recursive.get(name)?.clone())))
        .collect()
}

#[requires(true)]
#[ensures(true)]
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

#[requires(true)]
#[ensures(true)]
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

#[requires(true)]
#[ensures(true)]
fn vector_collection_element_type(output: &TokenStream2) -> Option<TokenStream2> {
    let ty = syn::parse2::<Type>(output.clone()).ok()?;
    vector_collection_type_parts(&ty).map(|(_, element)| quote!(#element))
}

#[requires(true)]
#[ensures(true)]
fn vector_collection_is_vec1(output: &TokenStream2) -> Option<bool> {
    let ty = syn::parse2::<Type>(output.clone()).ok()?;
    vector_collection_type_parts(&ty).map(|(is_vec1, _)| is_vec1)
}

#[requires(true)]
#[ensures(true)]
fn vector_collection_type_parts(ty: &Type) -> Option<(bool, &Type)> {
    let Type::Path(path) = ty else {
        return None;
    };
    let segment = path.path.segments.last()?;
    let is_vec1 = if segment.ident == "Vec1" {
        true
    } else if segment.ident == "Vec" {
        false
    } else {
        return None;
    };
    let PathArguments::AngleBracketed(args) = &segment.arguments else {
        return None;
    };
    let element = args.args.iter().find_map(|arg| match arg {
        GenericArgument::Type(ty) => Some(ty),
        _ => None,
    })?;
    Some((is_vec1, element))
}

#[requires(true)]
#[ensures(true)]
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

#[requires(true)]
#[ensures(true)]
fn method_rust_parser_output_type(
    method: &ExprMethodCall,
    type_env: &GrammarTypeEnv,
    arguments: &BTreeMap<String, Type>,
) -> Option<TokenStream2> {
    if method.method == "warn"
        || method.method == "elidable_terminator"
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

#[requires(true)]
#[ensures(true)]
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
        ("cmavo" | "selmaho" | "word_category" | "quote_marker" | "delimited_quote_marker", 1)
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

#[requires(true)]
#[ensures(true)]
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

#[requires(true)]
#[ensures(true)]
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

#[requires(true)]
#[ensures(true)]
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

#[requires(true)]
#[ensures(true)]
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

#[requires(true)]
#[ensures(true)]
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

#[requires(true)]
#[ensures(true)]
fn tuple_rust_parser_output_type(
    tuple: &ExprTuple,
    type_env: &GrammarTypeEnv,
    arguments: &BTreeMap<String, Type>,
) -> Option<TokenStream2> {
    sequence_output_type(tuple.elems.iter(), type_env, arguments)
}

#[requires(true)]
#[ensures(true)]
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

#[requires(true)]
#[ensures(true)]
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

#[requires(true)]
#[ensures(true)]
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

#[requires(true)]
#[ensures(true)]
fn recovered_rule_function_output_tokens(
    output: &Type,
    recovered_module: &TokenStream2,
) -> TokenStream2 {
    if let Some(output_ident) = simple_type_ident(output) {
        quote!(#recovered_module::#output_ident)
    } else {
        recovered_parser_value_type_tokens(output, &None, None, recovered_module)
    }
}

#[requires(true)]
#[ensures(true)]
fn recovered_parser_value_type_tokens(
    output: &Type,
    model_outputs: &Option<BTreeSet<String>>,
    model_path: Option<&Path>,
    recovered_module: &TokenStream2,
) -> TokenStream2 {
    if output_is_generated_model(true, model_outputs, output)
        && let Some(output_ident) = simple_type_ident(output)
    {
        let _ = model_path;
        return quote!(#recovered_module::#output_ident);
    }
    recovered_field_type_tokens(output, model_outputs, model_path, recovered_module)
}

#[requires(true)]
#[ensures(true)]
fn recovered_field_type_tokens(
    ty: &Type,
    model_outputs: &Option<BTreeSet<String>>,
    model_path: Option<&Path>,
    recovered_module: &TokenStream2,
) -> TokenStream2 {
    match ty {
        Type::Path(path) if path.qself.is_none() => {
            let Some(segment) = path.path.segments.last() else {
                return quote!(#recovered_module::Recovered<#ty>);
            };
            if is_wrapper_ident(&segment.ident) {
                return recovered_wrapper_type_tokens(
                    &segment.ident,
                    &segment.arguments,
                    model_outputs,
                    model_path,
                    recovered_module,
                );
            }
            if output_is_generated_model(true, model_outputs, ty)
                && let Some(output_ident) = simple_type_ident(ty)
            {
                return quote!(#recovered_module::Recovered<#recovered_module::#output_ident>);
            }
            quote!(#recovered_module::Recovered<#ty>)
        }
        Type::Tuple(tuple) => {
            let elems = tuple.elems.iter().map(|elem| {
                recovered_field_type_tokens(elem, model_outputs, model_path, recovered_module)
            });
            quote!((#(#elems,)*))
        }
        Type::Array(array) => {
            let elem = recovered_field_type_tokens(
                &array.elem,
                model_outputs,
                model_path,
                recovered_module,
            );
            let len = &array.len;
            quote!([#elem; #len])
        }
        _ => quote!(#recovered_module::Recovered<#ty>),
    }
}

#[requires(true)]
#[ensures(true)]
fn is_wrapper_ident(ident: &Ident) -> bool {
    matches!(
        ident.to_string().as_str(),
        "Arc"
            | "Box"
            | "Chain"
            | "Option"
            | "SmallVec"
            | "SmallVec1"
            | "Vec"
            | "Vec1"
            | "WithFreeModifiers"
    )
}

#[requires(true)]
#[ensures(true)]
fn first_type_argument(arguments: &PathArguments) -> Option<&Type> {
    nth_type_argument(arguments, 0)
}

#[requires(true)]
#[ensures(true)]
fn nth_type_argument(arguments: &PathArguments, index: usize) -> Option<&Type> {
    let PathArguments::AngleBracketed(arguments) = arguments else {
        return None;
    };
    arguments
        .args
        .iter()
        .filter_map(|argument| match argument {
            GenericArgument::Type(ty) => Some(ty),
            _ => None,
        })
        .nth(index)
}

#[requires(true)]
#[ensures(true)]
fn recovered_wrapper_type_tokens(
    wrapper: &Ident,
    arguments: &PathArguments,
    model_outputs: &Option<BTreeSet<String>>,
    model_path: Option<&Path>,
    recovered_module: &TokenStream2,
) -> TokenStream2 {
    let Some(inner) = first_type_argument(arguments) else {
        return quote!(#wrapper);
    };
    let wrapper_name = wrapper.to_string();
    match wrapper_name.as_str() {
        "Box" => {
            let inner =
                recovered_field_type_tokens(inner, model_outputs, model_path, recovered_module);
            quote!(Box<#inner>)
        }
        "Arc" => {
            let inner =
                recovered_field_type_tokens(inner, model_outputs, model_path, recovered_module);
            quote!(std::sync::Arc<#inner>)
        }
        "Option" => {
            let inner =
                recovered_field_type_tokens(inner, model_outputs, model_path, recovered_module);
            quote!(Option<#inner>)
        }
        "Vec" => {
            let inner =
                recovered_field_type_tokens(inner, model_outputs, model_path, recovered_module);
            quote!(Vec<#inner>)
        }
        "Vec1" => {
            let inner =
                recovered_field_type_tokens(inner, model_outputs, model_path, recovered_module);
            quote!(vec1::Vec1<#inner>)
        }
        "SmallVec" => {
            let inner =
                recovered_field_type_tokens(inner, model_outputs, model_path, recovered_module);
            quote!(smallvec::SmallVec<#inner>)
        }
        "SmallVec1" => {
            let inner =
                recovered_field_type_tokens(inner, model_outputs, model_path, recovered_module);
            quote!(vec1::smallvec_v1::SmallVec1<#inner>)
        }
        "WithFreeModifiers" => {
            let inner =
                recovered_field_type_tokens(inner, model_outputs, model_path, recovered_module);
            quote!(#recovered_module::WithFreeModifiers<#inner>)
        }
        "Chain" => {
            let links = nth_type_argument(arguments, 1).unwrap_or(inner);
            let first =
                recovered_field_type_tokens(inner, model_outputs, model_path, recovered_module);
            let links =
                recovered_field_type_tokens(links, model_outputs, model_path, recovered_module);
            quote!(::jbotci_tree::Chain<#first, #links>)
        }
        _ => quote!(#recovered_module::Recovered<#wrapper>),
    }
}

#[requires(true)]
#[ensures(true)]
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

#[requires(true)]
#[ensures(true)]
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

#[requires(true)]
#[ensures(true)]
fn is_path_type(output: &Type) -> bool {
    matches!(output, Type::Path(_))
}

#[requires(true)]
#[ensures(true)]
fn is_unit_type(output: &Type) -> bool {
    matches!(output, Type::Tuple(tuple) if tuple.elems.is_empty())
}

#[requires(true)]
#[ensures(true)]
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
        Ok(Rule::Enum(EnumRule::from_data(data!(EnumRule {
            output: syntax_type_for_rule(&name),
            name,
            arguments,
            context,
            branches,
        }))))
    } else {
        Err(input.error("expected `struct` or `enum` after `->`"))
    }
}

#[requires(true)]
#[ensures(true)]
fn parse_explicit_struct_fields(input: ParseStream<'_>) -> Result<Vec<FieldItem>> {
    let mut fields = Vec::new();
    while !input.is_empty() {
        fields.push(parse_explicit_struct_field(input)?);
    }
    Ok(fields)
}

#[requires(true)]
#[ensures(true)]
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
        Ok(FieldItem::from_data(data!(FieldItem {
            attrs,
            conditions,
            kind,
            name: Some(name),
            ty,
            parser,
        })))
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
        Ok(FieldItem::from_data(data!(FieldItem {
            attrs,
            conditions,
            kind: FieldKind::TempLet,
            name: Some(name),
            ty,
            parser,
        })))
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
        Ok(FieldItem::from_data(data!(FieldItem {
            attrs,
            conditions,
            kind: FieldKind::Require,
            name: None,
            ty: None,
            parser,
        })))
    } else {
        Err(input.error("expected `field`, `let`, or `assert`"))
    }
}

#[requires(true)]
#[ensures(true)]
fn parse_optional_arguments(input: ParseStream<'_>) -> Result<Vec<Ident>> {
    if !input.peek(syn::token::Paren) {
        return Ok(Vec::new());
    }
    let content;
    parenthesized!(content in input);
    Ok(Punctuated::<Ident, Token![,]>::parse_terminated(&content)?
        .into_iter()
        .collect())
}

#[invariant(matches!(kind, FieldKind::Require) == name.is_none(), "only assert field items are nameless")]
#[invariant(!matches!(kind, FieldKind::Require) || ty.is_none(), "assert field items do not have output types")]
struct FieldItem {
    attrs: Vec<Attribute>,
    conditions: Vec<Condition>,
    kind: FieldKind,
    name: Option<Ident>,
    ty: Option<Type>,
    parser: ParserExpr,
}

impl FieldItem {
    #[requires(true)]
    #[ensures(true)]
    fn expand(
        &self,
        arguments: &BTreeSet<String>,
        type_env: &GrammarTypeEnv,
    ) -> Result<TokenStream2> {
        let kind = self.kind.as_str();
        let name = self
            .name
            .as_ref()
            .map_or_else(String::new, Ident::to_string);
        let parser = self.parser.compact_tokens();
        let recovery = classify_parser_expr(&self.parser, arguments, type_env)?.expand();
        let conditions = self.conditions.iter().map(Condition::expand);
        Ok(quote! {
            SyntaxGrammarField {
                kind: #kind,
                name: #name,
                parser: #parser,
                recovery: #recovery,
                conditions: &[#(#conditions),*],
            }
        })
    }

    #[requires(true)]
    #[ensures(true)]
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
        Ok(GeneratedFieldModel::from_data(data!(GeneratedFieldModel {
            attrs: self.attrs.clone(),
            name,
            ty,
        })))
    }

    #[requires(true)]
    #[ensures(ret.is_err() || ret.as_ref().is_ok_and(|cmavo| cmavo.as_ref().is_none_or(|cmavo| !cmavo.is_empty())))]
    fn elidable_terminator_cmavo(&self) -> Result<Option<String>> {
        match &self.parser {
            ParserExpr::Rust(Expr::MethodCall(method))
                if method.method == "elidable_terminator" =>
            {
                self.elidable_terminator_cmavo_from_rust_method(method)
            }
            ParserExpr::Postfix {
                receiver,
                method,
                args,
            } if method == "elidable_terminator" => {
                self.elidable_terminator_cmavo_from_parser_postfix(receiver, args)
            }
            ParserExpr::Rust(_)
            | ParserExpr::Vector(_)
            | ParserExpr::Chain(_)
            | ParserExpr::Postfix { .. } => Ok(None),
        }
    }

    #[requires(method.method == "elidable_terminator")]
    #[ensures(ret.is_err() || ret.as_ref().is_ok_and(|cmavo| cmavo.as_ref().is_some_and(|cmavo| !cmavo.is_empty())))]
    fn elidable_terminator_cmavo_from_rust_method(
        &self,
        method: &ExprMethodCall,
    ) -> Result<Option<String>> {
        self.validate_elidable_terminator_field()?;
        let cmavo = elidable_terminator_marker_arg(
            method.args.len(),
            method.args.first(),
            self.parser.to_token_stream(),
        )?;
        let parsed = optional_elidable_terminator_cmavo_expr(&method.receiver).ok_or_else(|| {
            syn::Error::new_spanned(
                method.receiver.to_token_stream(),
                "elidable_terminator() must annotate an optional cmavo or selma'o terminator parser",
            )
        })?;
        validate_elidable_terminator_match(&method.receiver, &cmavo, &parsed)?;
        Ok(Some(cmavo))
    }

    #[requires(true)]
    #[ensures(ret.is_err() || ret.as_ref().is_ok_and(|cmavo| cmavo.as_ref().is_some_and(|cmavo| !cmavo.is_empty())))]
    fn elidable_terminator_cmavo_from_parser_postfix(
        &self,
        receiver: &ParserExpr,
        args: &[Expr],
    ) -> Result<Option<String>> {
        self.validate_elidable_terminator_field()?;
        let cmavo = elidable_terminator_marker_arg(
            args.len(),
            args.first(),
            self.parser.to_token_stream(),
        )?;
        let parsed = optional_elidable_terminator_cmavo(receiver).ok_or_else(|| {
            syn::Error::new_spanned(
                receiver.to_token_stream(),
                "elidable_terminator() must annotate an optional cmavo or selma'o terminator parser",
            )
        })?;
        validate_elidable_terminator_match(receiver.to_token_stream(), &cmavo, &parsed)?;
        Ok(Some(cmavo))
    }

    #[requires(true)]
    #[ensures(ret.is_ok() -> matches!(self.kind, FieldKind::Field))]
    fn validate_elidable_terminator_field(&self) -> Result<()> {
        if matches!(self.kind, FieldKind::Field) {
            Ok(())
        } else {
            Err(syn::Error::new_spanned(
                self.parser.to_token_stream(),
                "elidable terminator annotations are only valid on parser fields",
            ))
        }
    }
}

#[requires(true)]
#[ensures(ret.is_err() || ret.as_ref().is_ok_and(|cmavo| !cmavo.is_empty()))]
fn elidable_terminator_marker_arg(
    args_len: usize,
    first_arg: Option<&Expr>,
    span: impl ToTokens,
) -> Result<String> {
    if args_len != 1 {
        return Err(syn::Error::new_spanned(
            span,
            "elidable_terminator() requires one cmavo path",
        ));
    }
    required_path_expr_last_segment(
        first_arg.expect("length checked"),
        "elidable_terminator() requires a cmavo path",
    )
}

#[requires(!cmavo.is_empty())]
#[requires(!parsed.is_empty())]
#[ensures(ret.is_ok() -> cmavo == parsed)]
fn validate_elidable_terminator_match(
    span: impl ToTokens,
    cmavo: &str,
    parsed: &str,
) -> Result<()> {
    if parsed == cmavo {
        Ok(())
    } else {
        Err(syn::Error::new_spanned(
            span,
            format!(
                "elidable_terminator({cmavo}) does not match optional terminator parser `{parsed}`",
            ),
        ))
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|cmavo| !cmavo.is_empty()))]
fn optional_elidable_terminator_cmavo(expr: &ParserExpr) -> Option<String> {
    match expr {
        ParserExpr::Rust(expr) => optional_elidable_terminator_cmavo_expr(expr),
        ParserExpr::Vector(_) | ParserExpr::Chain(_) | ParserExpr::Postfix { .. } => None,
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|cmavo| !cmavo.is_empty()))]
fn optional_elidable_terminator_cmavo_expr(expr: &Expr) -> Option<String> {
    let Expr::Call(call) = expr else {
        return None;
    };
    if call_name(call)? != "opt" || call.args.len() != 1 {
        return None;
    }
    elidable_terminator_terminal_cmavo(call.args.first().expect("length checked"))
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|cmavo| !cmavo.is_empty()))]
fn elidable_terminator_terminal_cmavo(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Call(call) => {
            let name = call_name(call)?;
            match (name.as_str(), call.args.len()) {
                ("cmavo" | "selmaho", 1) => call.args.first().and_then(path_expr_last_segment),
                ("arc" | "boxed", 1) => {
                    elidable_terminator_terminal_cmavo(call.args.first().expect("length checked"))
                }
                ("feature" | "policy", 2) => elidable_terminator_terminal_cmavo(
                    call.args.iter().nth(1).expect("length checked"),
                ),
                _ => None,
            }
        }
        Expr::MethodCall(method) => match (method.method.to_string().as_str(), method.args.len()) {
            ("wf" | "with_free_modifiers" | "prohibited_wf" | "payload_start" | "lookahead", 0)
            | ("warn", 1) => elidable_terminator_terminal_cmavo(&method.receiver),
            _ => None,
        },
        Expr::Group(group) => elidable_terminator_terminal_cmavo(&group.expr),
        Expr::Paren(paren) => elidable_terminator_terminal_cmavo(&paren.expr),
        _ => None,
    }
}

enum FieldKind {
    Field,
    Computed,
    TempLet,
    Require,
}

impl FieldKind {
    #[requires(true)]
    #[ensures(true)]
    fn as_str(&self) -> &'static str {
        match self {
            FieldKind::Field => "field",
            FieldKind::Computed => "field",
            FieldKind::TempLet => "let",
            FieldKind::Require => "require",
        }
    }
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Condition {
    kind: ConditionKind,
    name: Ident,
}

impl Condition {
    #[requires(true)]
    #[ensures(true)]
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

    #[requires(true)]
    #[ensures(true)]
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

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum ConditionKind {
    Feature,
    Policy,
}

#[invariant(true)]
#[invariant(::Arc(_) => true)]
#[invariant(::Boxed(_) => true)]
#[invariant(::Choice(_) => true)]
#[invariant(::Cmavo(_) => true)]
#[invariant(::Ignored(_) => true)]
#[invariant(::Lookahead(_) => true)]
#[invariant(::Many(_) => true)]
#[invariant(::Many1(_) => true)]
#[invariant(::Not(_) => true)]
#[invariant(::NotNextRule(_) => true)]
#[invariant(::NotNextSelmaho(_) => true)]
#[invariant(::NotNextToken(_) => true)]
#[invariant(::Opaque(_) => true)]
#[invariant(::Opt(_) => true)]
#[invariant(::PayloadStart(_) => true)]
#[invariant(::Rule(_) => true)]
#[invariant(::Selmaho(_) => true)]
#[invariant(::Sequence(_) => true)]
#[invariant(::WithFreeModifiers(_) => true)]
#[invariant(::WordCategory(_) => true)]
#[derive(Debug, Clone, PartialEq, Eq)]
enum RecoveryExpr {
    Cmavo(String),
    Selmaho(String),
    WordCategory(String),
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
    #[requires(true)]
    #[ensures(true)]
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

#[requires(true)]
#[ensures(true)]
fn classify_parser_expr(
    expr: &ParserExpr,
    arguments: &BTreeSet<String>,
    type_env: &GrammarTypeEnv,
) -> Result<RecoveryExpr> {
    match expr {
        ParserExpr::Rust(expr) => classify_recovery_expr(expr, arguments, type_env),
        ParserExpr::Vector(expr) => Ok(RecoveryExpr::Sequence(
            expr.items
                .iter()
                .map(|item| match item {
                    VectorItem::One(expr) | VectorItem::Spread(expr) => {
                        classify_parser_expr(expr, arguments, type_env)
                    }
                    VectorItem::ZeroOrMore(expr) | VectorItem::ZeroOrMoreSpread(expr) => {
                        classify_parser_expr(expr, arguments, type_env)
                            .map(Box::new)
                            .map(RecoveryExpr::Many)
                    }
                    VectorItem::OneOrMore(expr) | VectorItem::OneOrMoreSpread(expr) => {
                        classify_parser_expr(expr, arguments, type_env)
                            .map(Box::new)
                            .map(RecoveryExpr::Many1)
                    }
                    VectorItem::Assert { negated, parser } => {
                        let inner = classify_parser_expr(parser, arguments, type_env)?;
                        if *negated {
                            Ok(RecoveryExpr::Not(Box::new(inner)))
                        } else {
                            Ok(RecoveryExpr::Lookahead(Box::new(inner)))
                        }
                    }
                })
                .collect::<Result<Vec<_>>>()?,
        )),
        ParserExpr::Chain(expr) => {
            let links =
                match expr.links_kind {
                    ChainLinksKind::ZeroOrMore => RecoveryExpr::Many(Box::new(
                        classify_parser_expr(&expr.links, arguments, type_env)?,
                    )),
                    ChainLinksKind::OneOrMore => RecoveryExpr::Many1(Box::new(
                        classify_parser_expr(&expr.links, arguments, type_env)?,
                    )),
                };
            Ok(RecoveryExpr::Sequence(vec![
                classify_parser_expr(&expr.first, arguments, type_env)?,
                links,
            ]))
        }
        ParserExpr::Postfix {
            receiver,
            method,
            args,
        } => classify_postfix_recovery_expr(receiver, method, args, arguments, type_env),
    }
}

#[requires(true)]
#[ensures(true)]
fn classify_postfix_recovery_expr(
    receiver: &ParserExpr,
    method: &Ident,
    args: &[Expr],
    arguments: &BTreeSet<String>,
    type_env: &GrammarTypeEnv,
) -> Result<RecoveryExpr> {
    let inner = || classify_parser_expr(receiver, arguments, type_env).map(Box::new);
    match (method.to_string().as_str(), args.len()) {
        ("wf", 0) | ("with_free_modifiers", 0) | ("prohibited_wf", 0) => {
            Ok(RecoveryExpr::WithFreeModifiers(inner()?))
        }
        ("warn", 1) | ("elidable_terminator", 1) => {
            classify_parser_expr(receiver, arguments, type_env)
        }
        ("ignored", 0) => Ok(RecoveryExpr::Ignored(inner()?)),
        ("not", 0) => Ok(RecoveryExpr::Not(inner()?)),
        ("lookahead", 0) => Ok(RecoveryExpr::Lookahead(inner()?)),
        ("ignore_then", 1) => Ok(RecoveryExpr::Sequence(vec![
            classify_parser_expr(receiver, arguments, type_env)?,
            classify_recovery_expr(args.first().expect("length checked"), arguments, type_env)?,
        ])),
        _ => {
            let receiver = receiver.to_token_stream();
            Ok(RecoveryExpr::Opaque(compact_tokens(
                quote!(#receiver.#method(#(#args),*)),
            )))
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn classify_recovery_expr(
    expr: &Expr,
    arguments: &BTreeSet<String>,
    type_env: &GrammarTypeEnv,
) -> Result<RecoveryExpr> {
    match expr {
        Expr::Call(call) => classify_call_recovery_expr(call, arguments, type_env),
        Expr::MethodCall(method) => classify_method_recovery_expr(method, arguments, type_env),
        Expr::Path(path) => classify_path_recovery_expr(path, arguments, type_env),
        Expr::Tuple(tuple) => Ok(RecoveryExpr::Sequence(
            tuple
                .elems
                .iter()
                .map(|expr| classify_recovery_expr(expr, arguments, type_env))
                .collect::<Result<Vec<_>>>()?,
        )),
        Expr::Array(array) => {
            if let Some(expr) = array_vector_expr(array) {
                classify_parser_expr(&ParserExpr::Vector(expr), arguments, type_env)
            } else {
                Ok(RecoveryExpr::Opaque(compact_tokens(expr)))
            }
        }
        _ => Ok(RecoveryExpr::Opaque(compact_tokens(expr))),
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|expr| !expr.items.is_empty()))]
fn array_vector_expr(array: &ExprArray) -> Option<VectorExpr> {
    if array.elems.is_empty() {
        return None;
    }
    Some(VectorExpr::from_data(data!(VectorExpr {
        items: array
            .elems
            .iter()
            .cloned()
            .map(ParserExpr::Rust)
            .map(VectorItem::One)
            .collect(),
    })))
}

#[requires(true)]
#[ensures(true)]
fn classify_method_recovery_expr(
    method: &ExprMethodCall,
    arguments: &BTreeSet<String>,
    type_env: &GrammarTypeEnv,
) -> Result<RecoveryExpr> {
    let inner = || classify_recovery_expr(&method.receiver, arguments, type_env).map(Box::new);
    match (method.method.to_string().as_str(), method.args.len()) {
        ("elidable_terminator", 1) => classify_recovery_expr(&method.receiver, arguments, type_env),
        ("wf", 0) | ("with_free_modifiers", 0) | ("prohibited_wf", 0) => {
            Ok(RecoveryExpr::WithFreeModifiers(inner()?))
        }
        ("warn", 1) => classify_recovery_expr(&method.receiver, arguments, type_env),
        ("payload_start", 0) => Ok(RecoveryExpr::PayloadStart(inner()?)),
        ("ignored", 0) => Ok(RecoveryExpr::Ignored(inner()?)),
        ("ignore_then", 1) => Ok(RecoveryExpr::Sequence(vec![
            classify_recovery_expr(&method.receiver, arguments, type_env)?,
            classify_recovery_expr(
                method.args.first().expect("length checked"),
                arguments,
                type_env,
            )?,
        ])),
        ("not_next_selmaho", 1) => Ok(method
            .args
            .first()
            .and_then(path_expr_last_segment)
            .map(RecoveryExpr::NotNextSelmaho)
            .unwrap_or_else(|| RecoveryExpr::Opaque(compact_tokens(method)))),
        ("not_next_token", 1) => Ok(method
            .args
            .first()
            .and_then(path_expr_last_segment)
            .map(RecoveryExpr::NotNextToken)
            .unwrap_or_else(|| RecoveryExpr::Opaque(compact_tokens(method)))),
        ("not_next_rule", 1) => classify_not_next_rule_recovery_expr(method, arguments, type_env),
        ("lookahead", 0) => Ok(RecoveryExpr::Lookahead(inner()?)),
        ("not", 0) => Ok(RecoveryExpr::Not(inner()?)),
        _ => Ok(RecoveryExpr::Opaque(compact_tokens(method))),
    }
}

#[requires(true)]
#[ensures(true)]
fn classify_not_next_rule_recovery_expr(
    method: &ExprMethodCall,
    arguments: &BTreeSet<String>,
    type_env: &GrammarTypeEnv,
) -> Result<RecoveryExpr> {
    let Some(argument) = method.args.first() else {
        return Ok(RecoveryExpr::Opaque(compact_tokens(method)));
    };
    let Some(rule) = path_expr_last_segment(argument) else {
        return Ok(RecoveryExpr::Opaque(compact_tokens(method)));
    };
    if type_env.rule_known_for_recovery(&rule, arguments) {
        Ok(RecoveryExpr::NotNextRule(rule))
    } else {
        Err(syn::Error::new_spanned(
            argument,
            format!("unknown grammar rule `{rule}` in recovery metadata"),
        ))
    }
}

#[requires(true)]
#[ensures(true)]
fn classify_call_recovery_expr(
    call: &ExprCall,
    arguments: &BTreeSet<String>,
    type_env: &GrammarTypeEnv,
) -> Result<RecoveryExpr> {
    let Some(name) = call_name(call) else {
        return Ok(RecoveryExpr::Opaque(compact_tokens(call)));
    };
    Ok(match (name.as_str(), call.args.len()) {
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
        ("quote_marker" | "delimited_quote_marker", 1) => call
            .args
            .first()
            .and_then(path_expr_last_segment)
            .map(RecoveryExpr::Cmavo)
            .unwrap_or_else(|| RecoveryExpr::Opaque(compact_tokens(call))),
        ("opt", 1) => RecoveryExpr::Opt(Box::new(classify_recovery_expr(
            &call.args[0],
            arguments,
            type_env,
        )?)),
        ("feature" | "policy", 1) => RecoveryExpr::Opaque(compact_tokens(call)),
        ("boxed", 1) => RecoveryExpr::Boxed(Box::new(classify_recovery_expr(
            &call.args[0],
            arguments,
            type_env,
        )?)),
        ("arc", 1) => RecoveryExpr::Arc(Box::new(classify_recovery_expr(
            &call.args[0],
            arguments,
            type_env,
        )?)),
        ("choice", 1) => RecoveryExpr::Choice(
            call.args
                .first()
                .map(choice_alternative_exprs)
                .unwrap_or_default()
                .into_iter()
                .map(|expr| classify_recovery_expr(expr, arguments, type_env))
                .collect::<Result<Vec<_>>>()?,
        ),
        ("choice", _) => RecoveryExpr::Choice(
            call.args
                .iter()
                .map(|expr| classify_recovery_expr(expr, arguments, type_env))
                .collect::<Result<Vec<_>>>()?,
        ),
        ("pa_word", 0) => RecoveryExpr::Selmaho("Pa".to_owned()),
        ("cmevla_word" | "text_leading_cmevla_word", 0) => {
            RecoveryExpr::WordCategory("Cmevla".to_owned())
        }
        ("relation_word" | "tanru_unit_relation_word", 0) => RecoveryExpr::RelationWord,
        ("bare_negation_term", 0) => RecoveryExpr::BareNegationTerm,
        ("eof", 0) => RecoveryExpr::Eof,
        _ if type_env.rule_known_for_recovery(&name, arguments) => RecoveryExpr::Rule(name),
        _ if call.args.is_empty() => RecoveryExpr::Opaque(compact_tokens(call)),
        _ => RecoveryExpr::Opaque(compact_tokens(call)),
    })
}

#[requires(true)]
#[ensures(true)]
fn classify_path_recovery_expr(
    path: &ExprPath,
    arguments: &BTreeSet<String>,
    type_env: &GrammarTypeEnv,
) -> Result<RecoveryExpr> {
    let text = compact_tokens(path);
    if type_env.rule_known_for_recovery(&text, arguments) {
        Ok(RecoveryExpr::Rule(text))
    } else if path.qself.is_none() && path.path.segments.len() == 1 {
        Err(syn::Error::new_spanned(
            path,
            format!("unknown grammar rule `{text}` in recovery metadata"),
        ))
    } else {
        Ok(RecoveryExpr::Opaque(text))
    }
}

#[requires(true)]
#[ensures(true)]
fn call_name(call: &ExprCall) -> Option<String> {
    let Expr::Path(path) = call.func.as_ref() else {
        return None;
    };
    path.path
        .segments
        .last()
        .map(|segment| segment.ident.to_string())
}

#[requires(true)]
#[ensures(true)]
fn path_expr_last_segment(expr: &Expr) -> Option<String> {
    let Expr::Path(path) = expr else {
        return None;
    };
    path.path
        .segments
        .last()
        .map(|segment| segment.ident.to_string())
}

#[requires(!message.is_empty())]
#[ensures(ret.is_err() || ret.as_ref().is_ok_and(|segment| !segment.is_empty()))]
fn required_path_expr_last_segment(expr: &Expr, message: &'static str) -> Result<String> {
    path_expr_last_segment(expr).ok_or_else(|| syn::Error::new_spanned(expr, message))
}

#[requires(true)]
#[ensures(true)]
fn compact_tokens(tokens: impl ToTokens) -> String {
    tokens
        .to_token_stream()
        .to_string()
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect()
}

#[invariant(true)]
#[invariant(::Cmavo(cmavo) => !cmavo.is_empty())]
#[invariant(::Selmaho(selmaho) => !selmaho.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum AnchorToken {
    Cmavo(String),
    Selmaho(String),
}

impl AnchorToken {
    #[requires(!cmavo.is_empty())]
    #[ensures(true)]
    fn cmavo(cmavo: String) -> Self {
        Self::from_data(data!(AnchorToken::Cmavo(cmavo)))
    }

    #[requires(!selmaho.is_empty())]
    #[ensures(true)]
    fn selmaho(selmaho: String) -> Self {
        Self::from_data(data!(AnchorToken::Selmaho(selmaho)))
    }

    #[requires(true)]
    #[ensures(true)]
    fn expand(&self) -> TokenStream2 {
        match self.as_data() {
            data!(AnchorToken::Cmavo(cmavo)) => {
                let cmavo = format_ident!("{cmavo}");
                quote!(SyntaxGrammarAnchorToken::Cmavo(Cmavo::#cmavo))
            }
            data!(AnchorToken::Selmaho(selmaho)) => {
                let selmaho = format_ident!("{selmaho}");
                quote!(SyntaxGrammarAnchorToken::Selmaho(Selmaho::#selmaho))
            }
        }
    }
}

#[invariant(!name.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
struct AnchorCondition {
    kind: ConditionKind,
    name: String,
}

impl AnchorCondition {
    #[requires(true)]
    #[ensures(!ret.name.is_empty())]
    fn from_condition(condition: &Condition) -> Self {
        Self::from_data(data!(AnchorCondition {
            kind: condition.kind,
            name: condition.name.to_string(),
        }))
    }

    #[requires(!self.name.is_empty())]
    #[ensures(true)]
    fn expand(&self) -> TokenStream2 {
        let kind = match self.kind {
            ConditionKind::Feature => quote!(SyntaxGrammarConditionKind::Feature),
            ConditionKind::Policy => quote!(SyntaxGrammarConditionKind::Policy),
        };
        let name = &self.name;
        quote! {
            SyntaxGrammarCondition {
                kind: #kind,
                name: #name,
            }
        }
    }
}

#[invariant(!tokens.is_empty())]
#[invariant(conditions.iter().all(|condition| !condition.name.is_empty()))]
#[derive(Debug, Clone, PartialEq, Eq)]
struct FirstEntry {
    tokens: BTreeSet<AnchorToken>,
    conditions: Vec<AnchorCondition>,
}

impl FirstEntry {
    #[requires(!tokens.is_empty())]
    #[ensures(!ret.tokens.is_empty())]
    fn new(tokens: BTreeSet<AnchorToken>, conditions: Vec<AnchorCondition>) -> Self {
        Self::from_data(data!(FirstEntry { tokens, conditions }))
    }

    #[requires(true)]
    #[ensures(!ret.tokens.is_empty())]
    fn with_conditions(&self, conditions: &[AnchorCondition]) -> Self {
        Self::from_data(data!(FirstEntry {
            tokens: self.tokens.clone(),
            conditions: combine_anchor_conditions(conditions, &self.conditions),
        }))
    }

    #[requires(!self.tokens.is_empty())]
    #[ensures(true)]
    fn expand(&self) -> TokenStream2 {
        let tokens = expand_anchor_token_slice(&self.tokens);
        let conditions = expand_anchor_condition_slice(&self.conditions);
        quote! {
            SyntaxGrammarAnchorTokenSet {
                tokens: #tokens,
                conditions: #conditions,
            }
        }
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum AnchorRunOrigin {
    LiteralRun,
    RepetitionElementFirst,
    FieldFirst,
}

impl AnchorRunOrigin {
    #[requires(true)]
    #[ensures(true)]
    fn expand(self) -> TokenStream2 {
        match self {
            Self::LiteralRun => quote!(SyntaxGrammarAnchorOrigin::LiteralRun),
            Self::RepetitionElementFirst => {
                quote!(SyntaxGrammarAnchorOrigin::RepetitionElementFirst)
            }
            Self::FieldFirst => quote!(SyntaxGrammarAnchorOrigin::FieldFirst),
        }
    }
}

#[invariant(!start_tokens.is_empty())]
#[invariant(conditions.iter().all(|condition| !condition.name.is_empty()))]
#[derive(Debug, Clone, PartialEq, Eq)]
struct AnchorRunSpec {
    start_tokens: BTreeSet<AnchorToken>,
    resume_field: usize,
    origin: AnchorRunOrigin,
    conditions: Vec<AnchorCondition>,
}

impl AnchorRunSpec {
    #[requires(!start_tokens.is_empty())]
    #[ensures(!ret.start_tokens.is_empty())]
    fn new(
        start_tokens: BTreeSet<AnchorToken>,
        resume_field: usize,
        origin: AnchorRunOrigin,
        conditions: Vec<AnchorCondition>,
    ) -> Self {
        Self::from_data(data!(AnchorRunSpec {
            start_tokens,
            resume_field,
            origin,
            conditions,
        }))
    }

    #[requires(!self.start_tokens.is_empty())]
    #[ensures(true)]
    fn expand(&self) -> TokenStream2 {
        let start_tokens = expand_anchor_token_slice(&self.start_tokens);
        let resume_field = self.resume_field;
        let origin = self.origin.expand();
        let conditions = expand_anchor_condition_slice(&self.conditions);
        quote! {
            SyntaxGrammarAnchorRun {
                start_tokens: #start_tokens,
                resume_field: #resume_field,
                origin: #origin,
                conditions: #conditions,
            }
        }
    }
}

#[invariant(!field_name.is_empty())]
#[invariant(anchors.iter().all(|anchor| !anchor.start_tokens.is_empty()))]
#[derive(Debug, Clone, PartialEq, Eq)]
struct FieldAnchorSpec {
    field_index: usize,
    field_name: String,
    anchors: Vec<AnchorRunSpec>,
}

impl FieldAnchorSpec {
    #[requires(true)]
    #[ensures(ret.field_index == field_index)]
    fn new(field_index: usize, field_name: String, anchors: Vec<AnchorRunSpec>) -> Self {
        Self::from_data(data!(FieldAnchorSpec {
            field_index,
            field_name,
            anchors,
        }))
    }

    #[requires(true)]
    #[ensures(true)]
    fn expand(&self) -> TokenStream2 {
        let field_index = self.field_index;
        let field_name = &self.field_name;
        let anchors = self.anchors.iter().map(AnchorRunSpec::expand);
        quote! {
            SyntaxGrammarFieldAnchors {
                field_index: #field_index,
                field_name: #field_name,
                anchors: &[#(#anchors),*],
            }
        }
    }
}

#[invariant(!rule.is_empty())]
#[invariant(opener_field < text_field && text_field < closer_field)]
#[invariant(!opener_tokens.is_empty())]
#[invariant(!closer_tokens.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
struct SubtextContainerSpec {
    rule: String,
    opener_field: usize,
    opener_tokens: BTreeSet<AnchorToken>,
    text_field: usize,
    closer_field: usize,
    closer_tokens: BTreeSet<AnchorToken>,
}

impl SubtextContainerSpec {
    #[requires(!opener_tokens.is_empty())]
    #[requires(!closer_tokens.is_empty())]
    #[ensures(!ret.opener_tokens.is_empty())]
    #[ensures(!ret.closer_tokens.is_empty())]
    fn new(
        rule: String,
        opener_field: usize,
        opener_tokens: BTreeSet<AnchorToken>,
        text_field: usize,
        closer_field: usize,
        closer_tokens: BTreeSet<AnchorToken>,
    ) -> Self {
        Self::from_data(data!(SubtextContainerSpec {
            rule,
            opener_field,
            opener_tokens,
            text_field,
            closer_field,
            closer_tokens,
        }))
    }

    #[requires(!self.opener_tokens.is_empty())]
    #[requires(!self.closer_tokens.is_empty())]
    #[ensures(true)]
    fn expand(&self) -> TokenStream2 {
        let rule = &self.rule;
        let opener_field = self.opener_field;
        let opener_tokens = expand_anchor_token_slice(&self.opener_tokens);
        let text_field = self.text_field;
        let closer_field = self.closer_field;
        let closer_tokens = expand_anchor_token_slice(&self.closer_tokens);
        quote! {
            SyntaxGrammarSubtextContainer {
                rule: #rule,
                opener_field: #opener_field,
                opener_tokens: #opener_tokens,
                text_field: #text_field,
                closer_field: #closer_field,
                closer_tokens: #closer_tokens,
            }
        }
    }
}

#[invariant(rule_indices.values().all(|index| *index < rules.len()))]
#[invariant(rule_indices.keys().all(|name| !name.is_empty()))]
struct RecoveryAnchorAnalyzer<'a> {
    rules: &'a [Rule],
    type_env: &'a GrammarTypeEnv,
    rule_indices: BTreeMap<String, usize>,
    first_cache: RefCell<BTreeMap<String, Vec<FirstEntry>>>,
    first_visiting: RefCell<BTreeSet<String>>,
    nullable_cache: RefCell<BTreeMap<String, bool>>,
    nullable_visiting: RefCell<BTreeSet<String>>,
}

impl<'a> RecoveryAnchorAnalyzer<'a> {
    #[requires(true)]
    #[ensures(ret.rules.len() == rules.len())]
    fn new(rules: &'a [Rule], type_env: &'a GrammarTypeEnv) -> Self {
        Self::from_data(data!(RecoveryAnchorAnalyzer {
            rules,
            type_env,
            rule_indices: rules
                .iter()
                .enumerate()
                .map(|(index, rule)| (rule.name().to_string(), index))
                .collect(),
            first_cache: RefCell::new(BTreeMap::new()),
            first_visiting: RefCell::new(BTreeSet::new()),
            nullable_cache: RefCell::new(BTreeMap::new()),
            nullable_visiting: RefCell::new(BTreeSet::new()),
        }))
    }

    #[requires(true)]
    #[ensures(true)]
    fn expand_metadata(&self) -> Result<TokenStream2> {
        let mut rule_items = Vec::new();
        for rule in self.rules {
            rule_items.push(self.expand_rule_metadata(rule)?);
        }
        let rule_lookup_arms = self.rules.iter().enumerate().map(|(index, rule)| {
            let name = rule.name().to_string();
            quote!(#name => Some(&SYNTAX_GRAMMAR_RECOVERY_ANCHORS[#index]))
        });
        let container_items = self
            .subtext_containers()?
            .iter()
            .map(SubtextContainerSpec::expand)
            .collect::<Vec<_>>();
        Ok(quote! {
            pub(crate) const SYNTAX_GRAMMAR_RECOVERY_ANCHORS: &[SyntaxGrammarRuleAnchorMetadata] = &[
                #(#rule_items),*
            ];

            pub(crate) const SYNTAX_GRAMMAR_SUBTEXT_CONTAINERS: &[SyntaxGrammarSubtextContainer] = &[
                #(#container_items),*
            ];

            #[bityzba::requires(!name.is_empty())]
            #[bityzba::ensures(ret.as_ref().is_none_or(|metadata| metadata.rule == name))]
            pub(crate) fn syntax_grammar_anchor_metadata_by_rule_name(
                name: &str,
            ) -> Option<&'static SyntaxGrammarRuleAnchorMetadata> {
                match name {
                    #(#rule_lookup_arms,)*
                    _ => None,
                }
            }
        })
    }

    #[requires(true)]
    #[ensures(true)]
    fn expand_rule_metadata(&self, rule: &Rule) -> Result<TokenStream2> {
        let name = rule.name().to_string();
        let first = self.rule_first_entries(&name)?;
        let first_items = first.iter().map(FirstEntry::expand);
        let field_items = self
            .field_anchor_specs(rule)?
            .iter()
            .map(FieldAnchorSpec::expand)
            .collect::<Vec<_>>();
        Ok(quote! {
            SyntaxGrammarRuleAnchorMetadata {
                rule: #name,
                first: &[#(#first_items),*],
                fields: &[#(#field_items),*],
            }
        })
    }

    #[requires(true)]
    #[ensures(true)]
    fn field_anchor_specs(&self, rule: &Rule) -> Result<Vec<FieldAnchorSpec>> {
        let Rule::Struct(rule) = rule else {
            return Ok(Vec::new());
        };
        let argument_names = rule.argument_name_set();
        let mut specs = Vec::new();
        for (field_index, field) in rule.fields.iter().enumerate() {
            if !matches!(field.kind, FieldKind::Field) {
                continue;
            }
            specs.push(FieldAnchorSpec::new(
                field_index,
                field
                    .name
                    .as_ref()
                    .map_or_else(String::new, Ident::to_string),
                self.anchor_runs_from_field(rule, &argument_names, field_index)?,
            ));
        }
        Ok(specs)
    }

    #[requires(start_field <= rule.fields.len())]
    #[ensures(ret.is_err() || ret.as_ref().is_ok_and(|runs| runs.iter().all(|run| !run.start_tokens.is_empty())))]
    fn anchor_runs_from_field(
        &self,
        rule: &NodeRule,
        argument_names: &BTreeSet<String>,
        start_field: usize,
    ) -> Result<Vec<AnchorRunSpec>> {
        let mut runs = Vec::new();
        let mut field_index = start_field;
        while field_index < rule.fields.len() {
            let field = &rule.fields[field_index];
            if !matches!(field.kind, FieldKind::Field) {
                field_index += 1;
                continue;
            }
            let expr = classify_parser_expr(&field.parser, argument_names, self.type_env)?;
            let conditions = anchor_conditions_from(&field.conditions);
            if literal_start_tokens(&expr).is_some() {
                let (literal_entries, after_run) =
                    self.literal_run_first_entries(rule, argument_names, field_index)?;
                for entry in literal_entries {
                    let entry = entry.into_data();
                    push_anchor_run(
                        &mut runs,
                        entry.tokens,
                        field_index,
                        AnchorRunOrigin::LiteralRun,
                        entry.conditions,
                    );
                }
                field_index = after_run;
                continue;
            }
            let origin = anchor_origin_for_non_literal_expr(&expr);
            for entry in self.expr_first_entries(&expr)? {
                let entry = entry.into_data();
                push_anchor_run(
                    &mut runs,
                    entry.tokens,
                    field_index,
                    origin,
                    combine_anchor_conditions(&conditions, &entry.conditions),
                );
            }
            field_index += 1;
        }
        Ok(runs)
    }

    #[requires(start_field < rule.fields.len())]
    #[ensures(ret.is_err() || ret.as_ref().is_ok_and(|(entries, field)| *field > start_field && entries.iter().all(|entry| !entry.tokens.is_empty())))]
    fn literal_run_first_entries(
        &self,
        rule: &NodeRule,
        argument_names: &BTreeSet<String>,
        start_field: usize,
    ) -> Result<(Vec<FirstEntry>, usize)> {
        let mut entries = Vec::new();
        let mut field_index = start_field;
        while field_index < rule.fields.len() {
            let field = &rule.fields[field_index];
            if !matches!(field.kind, FieldKind::Field) {
                break;
            }
            let expr = classify_parser_expr(&field.parser, argument_names, self.type_env)?;
            let Some(tokens) = literal_start_tokens(&expr) else {
                break;
            };
            push_first_entry(
                &mut entries,
                FirstEntry::new(tokens, anchor_conditions_from(&field.conditions)),
            );
            field_index += 1;
            if !self.expr_nullable(&expr)? {
                break;
            }
        }
        Ok((entries, field_index))
    }

    #[requires(!rule_name.is_empty())]
    #[ensures(ret.is_err() || ret.as_ref().is_ok_and(|entries| entries.iter().all(|entry| !entry.tokens.is_empty())))]
    fn rule_first_entries(&self, rule_name: &str) -> Result<Vec<FirstEntry>> {
        if let Some(cached) = self.first_cache.borrow().get(rule_name) {
            return Ok(cached.clone());
        }
        if !self
            .first_visiting
            .borrow_mut()
            .insert(rule_name.to_owned())
        {
            // The DSL generates PEG parsers, so true left recursion (including through
            // nullable prefixes) is invalid. A cycle here must pass through a consumed
            // token before returning to this rule, so breaking it contributes no FIRST
            // token and keeps the memoized result exact rather than heuristic.
            return Ok(Vec::new());
        }
        let entries = if let Some(index) = self.rule_indices.get(rule_name).copied() {
            match &self.rules[index] {
                Rule::Alias(rule) => {
                    let argument_names = rule.argument_name_set();
                    let expr = classify_parser_expr(&rule.parser, &argument_names, self.type_env)?;
                    self.expr_first_entries(&expr)?
                }
                Rule::Struct(rule) => {
                    let argument_names = rule.argument_name_set();
                    self.struct_first_entries(rule, &argument_names)?
                }
                Rule::Enum(rule) => self.enum_first_entries(rule)?,
            }
        } else {
            Vec::new()
        };
        self.first_visiting.borrow_mut().remove(rule_name);
        self.first_cache
            .borrow_mut()
            .insert(rule_name.to_owned(), entries.clone());
        Ok(entries)
    }

    #[requires(true)]
    #[ensures(ret.is_err() || ret.as_ref().is_ok_and(|entries| entries.iter().all(|entry| !entry.tokens.is_empty())))]
    fn struct_first_entries(
        &self,
        rule: &NodeRule,
        argument_names: &BTreeSet<String>,
    ) -> Result<Vec<FirstEntry>> {
        let mut entries = Vec::new();
        for field in &rule.fields {
            match field.kind {
                FieldKind::Field => {
                    let expr = classify_parser_expr(&field.parser, argument_names, self.type_env)?;
                    let conditions = anchor_conditions_from(&field.conditions);
                    for entry in self.expr_first_entries(&expr)? {
                        push_first_entry(&mut entries, entry.with_conditions(&conditions));
                    }
                    if !self.expr_nullable(&expr)? {
                        break;
                    }
                }
                FieldKind::Require | FieldKind::Computed | FieldKind::TempLet => {}
            }
        }
        Ok(entries)
    }

    #[requires(true)]
    #[ensures(ret.is_err() || ret.as_ref().is_ok_and(|entries| entries.iter().all(|entry| !entry.tokens.is_empty())))]
    fn enum_first_entries(&self, rule: &EnumRule) -> Result<Vec<FirstEntry>> {
        let mut entries = Vec::new();
        for branch in &rule.branches {
            let conditions = anchor_conditions_from(&branch.conditions);
            for entry in self.rule_first_entries(&branch.name.to_string())? {
                push_first_entry(&mut entries, entry.with_conditions(&conditions));
            }
        }
        Ok(entries)
    }

    #[requires(true)]
    #[ensures(ret.is_err() || ret.as_ref().is_ok_and(|entries| entries.iter().all(|entry| !entry.tokens.is_empty())))]
    fn expr_first_entries(&self, expr: &RecoveryExpr) -> Result<Vec<FirstEntry>> {
        let mut entries = Vec::new();
        match expr {
            RecoveryExpr::Cmavo(cmavo) => {
                push_first_entry(&mut entries, first_entry(AnchorToken::cmavo(cmavo.clone())));
            }
            RecoveryExpr::Selmaho(selmaho) => {
                push_first_entry(
                    &mut entries,
                    first_entry(AnchorToken::selmaho(selmaho.clone())),
                );
            }
            RecoveryExpr::Opt(inner)
            | RecoveryExpr::Many(inner)
            | RecoveryExpr::Many1(inner)
            | RecoveryExpr::Boxed(inner)
            | RecoveryExpr::Arc(inner)
            | RecoveryExpr::WithFreeModifiers(inner)
            | RecoveryExpr::PayloadStart(inner)
            | RecoveryExpr::Ignored(inner) => {
                entries.extend(self.expr_first_entries(inner)?);
            }
            RecoveryExpr::Choice(alternatives) => {
                for alternative in alternatives {
                    for entry in self.expr_first_entries(alternative)? {
                        push_first_entry(&mut entries, entry);
                    }
                }
            }
            RecoveryExpr::Sequence(parts) => {
                for part in parts {
                    for entry in self.expr_first_entries(part)? {
                        push_first_entry(&mut entries, entry);
                    }
                    if !self.expr_nullable(part)? {
                        break;
                    }
                }
            }
            RecoveryExpr::Rule(rule) => {
                entries.extend(self.rule_first_entries(rule)?);
            }
            RecoveryExpr::WordCategory(_)
            | RecoveryExpr::Lookahead(_)
            | RecoveryExpr::Not(_)
            | RecoveryExpr::NotNextSelmaho(_)
            | RecoveryExpr::NotNextToken(_)
            | RecoveryExpr::NotNextRule(_)
            | RecoveryExpr::BareNegationTerm
            | RecoveryExpr::RelationWord
            | RecoveryExpr::Opaque(_)
            | RecoveryExpr::Eof => {}
        }
        Ok(entries)
    }

    #[requires(true)]
    #[ensures(true)]
    fn expr_nullable(&self, expr: &RecoveryExpr) -> Result<bool> {
        Ok(match expr {
            RecoveryExpr::Opt(_) | RecoveryExpr::Many(_) => true,
            RecoveryExpr::Boxed(inner)
            | RecoveryExpr::Arc(inner)
            | RecoveryExpr::WithFreeModifiers(inner)
            | RecoveryExpr::PayloadStart(inner)
            | RecoveryExpr::Ignored(inner)
            | RecoveryExpr::Many1(inner) => self.expr_nullable(inner)?,
            RecoveryExpr::Choice(alternatives) => {
                let mut nullable = false;
                for alternative in alternatives {
                    nullable |= self.expr_nullable(alternative)?;
                }
                nullable
            }
            RecoveryExpr::Sequence(parts) => {
                let mut nullable = true;
                for part in parts {
                    nullable &= self.expr_nullable(part)?;
                    if !nullable {
                        break;
                    }
                }
                nullable
            }
            RecoveryExpr::Lookahead(_)
            | RecoveryExpr::Not(_)
            | RecoveryExpr::NotNextSelmaho(_)
            | RecoveryExpr::NotNextToken(_)
            | RecoveryExpr::NotNextRule(_)
            | RecoveryExpr::Eof => true,
            RecoveryExpr::Rule(rule) => self.rule_nullable(rule)?,
            RecoveryExpr::Cmavo(_)
            | RecoveryExpr::Selmaho(_)
            | RecoveryExpr::WordCategory(_)
            | RecoveryExpr::BareNegationTerm
            | RecoveryExpr::RelationWord
            | RecoveryExpr::Opaque(_) => false,
        })
    }

    #[requires(!rule_name.is_empty())]
    #[ensures(true)]
    fn rule_nullable(&self, rule_name: &str) -> Result<bool> {
        if let Some(cached) = self.nullable_cache.borrow().get(rule_name) {
            return Ok(*cached);
        }
        if !self
            .nullable_visiting
            .borrow_mut()
            .insert(rule_name.to_owned())
        {
            // As with FIRST sets, a nullable cycle would imply PEG left recursion.
            // Generated grammar rules are non-left-recursive, so a recursive visit is
            // only a defensive guard and cannot make the current rule nullable.
            return Ok(false);
        }
        let nullable = if let Some(index) = self.rule_indices.get(rule_name).copied() {
            match &self.rules[index] {
                Rule::Alias(rule) => {
                    let argument_names = rule.argument_name_set();
                    let expr = classify_parser_expr(&rule.parser, &argument_names, self.type_env)?;
                    self.expr_nullable(&expr)?
                }
                Rule::Struct(rule) => {
                    let argument_names = rule.argument_name_set();
                    let mut nullable = true;
                    for field in &rule.fields {
                        if matches!(field.kind, FieldKind::Field) {
                            let expr = classify_parser_expr(
                                &field.parser,
                                &argument_names,
                                self.type_env,
                            )?;
                            nullable &= self.expr_nullable(&expr)?;
                            if !nullable {
                                break;
                            }
                        }
                    }
                    nullable
                }
                Rule::Enum(rule) => {
                    let mut nullable = false;
                    for branch in &rule.branches {
                        nullable |= self.rule_nullable(&branch.name.to_string())?;
                    }
                    nullable
                }
            }
        } else {
            false
        };
        self.nullable_visiting.borrow_mut().remove(rule_name);
        self.nullable_cache
            .borrow_mut()
            .insert(rule_name.to_owned(), nullable);
        Ok(nullable)
    }

    #[requires(true)]
    #[ensures(ret.is_err() || ret.as_ref().is_ok_and(|containers| containers.iter().all(|container| !container.opener_tokens.is_empty() && !container.closer_tokens.is_empty())))]
    fn subtext_containers(&self) -> Result<Vec<SubtextContainerSpec>> {
        let mut containers = Vec::new();
        for rule in self.rules {
            let Rule::Struct(rule) = rule else {
                continue;
            };
            let argument_names = rule.argument_name_set();
            for (field_index, field) in rule.fields.iter().enumerate() {
                if !matches!(field.kind, FieldKind::Field) {
                    continue;
                }
                let expr = classify_parser_expr(&field.parser, &argument_names, self.type_env)?;
                if !expr_is_rule_reference(&expr, "text") {
                    continue;
                }
                let Some((opener_field, opener_tokens)) =
                    previous_literal_field(rule, &argument_names, field_index, self.type_env)?
                else {
                    continue;
                };
                let Some((closer_field, closer_tokens)) =
                    next_literal_field(rule, &argument_names, field_index, self.type_env)?
                else {
                    continue;
                };
                containers.push(SubtextContainerSpec::new(
                    rule.name.to_string(),
                    opener_field,
                    opener_tokens,
                    field_index,
                    closer_field,
                    closer_tokens,
                ));
            }
        }
        Ok(containers)
    }
}

#[requires(true)]
#[ensures(true)]
fn expand_recovery_anchor_metadata(
    rules: &[Rule],
    type_env: &GrammarTypeEnv,
) -> Result<TokenStream2> {
    RecoveryAnchorAnalyzer::new(rules, type_env).expand_metadata()
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|tokens| !tokens.is_empty()))]
fn literal_start_tokens(expr: &RecoveryExpr) -> Option<BTreeSet<AnchorToken>> {
    match expr {
        RecoveryExpr::Cmavo(cmavo) => Some(BTreeSet::from([AnchorToken::cmavo(cmavo.clone())])),
        RecoveryExpr::Selmaho(selmaho) => {
            Some(BTreeSet::from([AnchorToken::selmaho(selmaho.clone())]))
        }
        RecoveryExpr::Opt(inner)
        | RecoveryExpr::Boxed(inner)
        | RecoveryExpr::Arc(inner)
        | RecoveryExpr::WithFreeModifiers(inner)
        | RecoveryExpr::PayloadStart(inner)
        | RecoveryExpr::Ignored(inner) => literal_start_tokens(inner),
        RecoveryExpr::Choice(alternatives) => {
            let mut tokens = BTreeSet::new();
            for alternative in alternatives {
                if let Some(alternative_tokens) = literal_start_tokens(alternative) {
                    tokens.extend(alternative_tokens);
                }
            }
            (!tokens.is_empty()).then_some(tokens)
        }
        RecoveryExpr::Many(_)
        | RecoveryExpr::Many1(_)
        | RecoveryExpr::Lookahead(_)
        | RecoveryExpr::Not(_)
        | RecoveryExpr::NotNextSelmaho(_)
        | RecoveryExpr::NotNextToken(_)
        | RecoveryExpr::NotNextRule(_)
        | RecoveryExpr::Sequence(_)
        | RecoveryExpr::WordCategory(_)
        | RecoveryExpr::BareNegationTerm
        | RecoveryExpr::RelationWord
        | RecoveryExpr::Rule(_)
        | RecoveryExpr::Opaque(_)
        | RecoveryExpr::Eof => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn anchor_origin_for_non_literal_expr(expr: &RecoveryExpr) -> AnchorRunOrigin {
    match expr {
        RecoveryExpr::Boxed(inner)
        | RecoveryExpr::Arc(inner)
        | RecoveryExpr::WithFreeModifiers(inner)
        | RecoveryExpr::PayloadStart(inner)
        | RecoveryExpr::Ignored(inner) => anchor_origin_for_non_literal_expr(inner),
        RecoveryExpr::Sequence(parts) if parts.len() == 1 => {
            anchor_origin_for_non_literal_expr(&parts[0])
        }
        RecoveryExpr::Many(_) | RecoveryExpr::Many1(_) => AnchorRunOrigin::RepetitionElementFirst,
        _ => AnchorRunOrigin::FieldFirst,
    }
}

#[requires(true)]
#[ensures(true)]
fn expr_is_rule_reference(expr: &RecoveryExpr, rule_name: &str) -> bool {
    match expr {
        RecoveryExpr::Rule(rule) => rule == rule_name,
        RecoveryExpr::Boxed(inner)
        | RecoveryExpr::Arc(inner)
        | RecoveryExpr::WithFreeModifiers(inner)
        | RecoveryExpr::PayloadStart(inner)
        | RecoveryExpr::Ignored(inner)
        | RecoveryExpr::Opt(inner) => expr_is_rule_reference(inner, rule_name),
        _ => false,
    }
}

#[requires(true)]
#[ensures(ret.iter().all(|condition| !condition.name.is_empty()))]
fn anchor_conditions_from(conditions: &[Condition]) -> Vec<AnchorCondition> {
    conditions
        .iter()
        .map(AnchorCondition::from_condition)
        .collect()
}

#[requires(true)]
#[ensures(ret.len() >= base.len())]
fn combine_anchor_conditions(
    base: &[AnchorCondition],
    additional: &[AnchorCondition],
) -> Vec<AnchorCondition> {
    let mut conditions = base.to_vec();
    for condition in additional {
        if !conditions.contains(condition) {
            conditions.push(condition.clone());
        }
    }
    conditions
}

#[requires(true)]
#[ensures(!ret.tokens.is_empty())]
fn first_entry(token: AnchorToken) -> FirstEntry {
    FirstEntry::new(BTreeSet::from([token]), Vec::new())
}

#[requires(!entry.tokens.is_empty())]
#[ensures(entries.iter().all(|entry| !entry.tokens.is_empty()))]
fn push_first_entry(entries: &mut Vec<FirstEntry>, entry: FirstEntry) {
    let existing_index = entries
        .iter()
        .position(|existing| existing.conditions == entry.conditions);
    let entry = entry.into_data();
    if let Some(existing_index) = existing_index {
        let existing = entries.remove(existing_index).into_data();
        let mut tokens = existing.tokens;
        tokens.extend(entry.tokens);
        entries.insert(
            existing_index,
            FirstEntry::from_data(data!(FirstEntry {
                tokens,
                conditions: existing.conditions,
            })),
        );
    } else {
        entries.push(FirstEntry::from_data(entry));
    }
}

#[requires(!tokens.is_empty())]
#[ensures(runs.iter().all(|run| !run.start_tokens.is_empty()))]
fn push_anchor_run(
    runs: &mut Vec<AnchorRunSpec>,
    tokens: BTreeSet<AnchorToken>,
    resume_field: usize,
    origin: AnchorRunOrigin,
    conditions: Vec<AnchorCondition>,
) {
    let existing_index = runs.iter().position(|existing| {
        existing.resume_field == resume_field
            && existing.origin == origin
            && existing.conditions == conditions
    });
    if let Some(existing_index) = existing_index {
        let existing = runs.remove(existing_index).into_data();
        let mut start_tokens = existing.start_tokens;
        start_tokens.extend(tokens);
        runs.insert(
            existing_index,
            AnchorRunSpec::from_data(data!(AnchorRunSpec {
                start_tokens,
                resume_field: existing.resume_field,
                origin: existing.origin,
                conditions: existing.conditions,
            })),
        );
    } else {
        runs.push(AnchorRunSpec::new(tokens, resume_field, origin, conditions));
    }
}

#[requires(true)]
#[ensures(ret.is_err() || ret.as_ref().is_ok_and(|value| value.as_ref().is_none_or(|(_, tokens)| !tokens.is_empty())))]
fn previous_literal_field(
    rule: &NodeRule,
    argument_names: &BTreeSet<String>,
    before_field: usize,
    type_env: &GrammarTypeEnv,
) -> Result<Option<(usize, BTreeSet<AnchorToken>)>> {
    for field_index in (0..before_field).rev() {
        let field = &rule.fields[field_index];
        if !matches!(field.kind, FieldKind::Field) {
            continue;
        }
        let expr = classify_parser_expr(&field.parser, argument_names, type_env)?;
        if let Some(tokens) = literal_start_tokens(&expr) {
            return Ok(Some((field_index, tokens)));
        }
    }
    Ok(None)
}

#[requires(after_field < rule.fields.len())]
#[ensures(ret.is_err() || ret.as_ref().is_ok_and(|value| value.as_ref().is_none_or(|(_, tokens)| !tokens.is_empty())))]
fn next_literal_field(
    rule: &NodeRule,
    argument_names: &BTreeSet<String>,
    after_field: usize,
    type_env: &GrammarTypeEnv,
) -> Result<Option<(usize, BTreeSet<AnchorToken>)>> {
    for field_index in after_field + 1..rule.fields.len() {
        let field = &rule.fields[field_index];
        if !matches!(field.kind, FieldKind::Field) {
            continue;
        }
        let expr = classify_parser_expr(&field.parser, argument_names, type_env)?;
        if let Some(tokens) = literal_start_tokens(&expr) {
            return Ok(Some((field_index, tokens)));
        }
    }
    Ok(None)
}

#[requires(true)]
#[ensures(true)]
fn expand_anchor_token_slice(tokens: &BTreeSet<AnchorToken>) -> TokenStream2 {
    let tokens = tokens.iter().map(AnchorToken::expand);
    quote!(&[#(#tokens),*])
}

#[requires(true)]
#[ensures(true)]
fn expand_anchor_condition_slice(conditions: &[AnchorCondition]) -> TokenStream2 {
    let conditions = conditions.iter().map(AnchorCondition::expand);
    quote!(&[#(#conditions),*])
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;

    #[requires(true)]
    #[ensures(true)]
    #[test]
    fn type_token_comparison_normalizes_paths_without_erasing_reference_mutability() {
        assert!(type_token_streams_match(
            &quote!(std::vec::Vec<Option<Token>>),
            &quote!(Vec<Option<Token>>),
        ));
        assert!(type_token_streams_match(
            &quote!(Option<std::vec::Vec<Token>>),
            &quote!(Option<Vec<Token>>),
        ));
        assert!(!type_token_streams_match(
            &quote!(&Token),
            &quote!(&mut Token),
        ));
    }

    #[requires(true)]
    #[ensures(true)]
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

    #[requires(true)]
    #[ensures(true)]
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

    #[requires(true)]
    #[ensures(true)]
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

    #[requires(true)]
    #[ensures(true)]
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

    #[requires(true)]
    #[ensures(true)]
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

    #[requires(true)]
    #[ensures(true)]
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

    #[requires(true)]
    #[ensures(true)]
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

    #[requires(true)]
    #[ensures(true)]
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

    #[requires(true)]
    #[ensures(true)]
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

    #[requires(true)]
    #[ensures(true)]
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

    #[requires(true)]
    #[ensures(true)]
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

    #[requires(true)]
    #[ensures(true)]
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

    #[requires(true)]
    #[ensures(true)]
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

    #[requires(true)]
    #[ensures(true)]
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

    #[requires(true)]
    #[ensures(true)]
    #[test]
    fn strict_parser_unknown_call_reports_compile_error() {
        let grammar = syn::parse2::<SyntaxGrammar>(quote! {
            env generated_runtime::SyntaxGrammarEnv;
            strict_parsers;

            rule "item" item -> struct {
                field token: std::sync::Arc<Token> <- arc(unknown_parser());
            }
        })
        .expect("grammar parses before strict parser generation");

        let expanded = grammar.expand().to_string();
        assert!(
            expanded.contains("compile_error")
                && expanded.contains("unsupported parser call in strict parser generation"),
            "unknown strict parser calls should be reported: {expanded}"
        );
    }

    #[requires(true)]
    #[ensures(true)]
    #[test]
    fn strict_parser_unknown_method_reports_compile_error() {
        let grammar = syn::parse2::<SyntaxGrammar>(quote! {
            env generated_runtime::SyntaxGrammarEnv;
            strict_parsers;

            rule "item" item -> struct {
                field token: std::sync::Arc<Token> <- arc(cmavo(Be).payload_start());
            }
        })
        .expect("grammar parses before strict parser generation");

        let expanded = grammar.expand().to_string();
        assert!(
            expanded.contains("compile_error")
                && expanded.contains("unsupported parser method in strict parser generation"),
            "unknown strict parser methods should be reported: {expanded}"
        );
    }

    #[requires(true)]
    #[ensures(true)]
    #[test]
    fn strict_recursive_root_without_rule_reports_compile_error() {
        let grammar = syn::parse2::<SyntaxGrammar>(quote! {
            env generated_runtime::SyntaxGrammarEnv;
            strict_parsers;

            recursive {
                item: ItemSyntax;
            }

            rule "other" other -> struct {
                field token <- cmavo(Be);
            }
        })
        .expect("grammar parses before strict recursive parser generation");

        let expanded = grammar.expand().to_string();
        assert!(
            expanded.contains("compile_error")
                && expanded.contains("recursive parser declaration has no matching rule"),
            "missing recursive root rules should be reported: {expanded}"
        );
    }
}
