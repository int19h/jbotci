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
    syn::custom_keyword!(model_path);
    syn::custom_keyword!(model_variant);
    syn::custom_keyword!(no_partial_valid);
    syn::custom_keyword!(node);
    syn::custom_keyword!(policy);
    syn::custom_keyword!(product);
    syn::custom_keyword!(recovered);
    syn::custom_keyword!(recursive);
    syn::custom_keyword!(require);
    syn::custom_keyword!(rule);
    syn::custom_keyword!(parsers);
    syn::custom_keyword!(scratch);
    syn::custom_keyword!(strict_parsers);
    syn::custom_keyword!(tree_model);
    syn::custom_keyword!(tuple_variant);
    syn::custom_keyword!(variant);
    syn::custom_keyword!(when);
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
        let auto_model_variants = match self.auto_model_variants(&type_env) {
            Ok(auto_model_variants) => auto_model_variants,
            Err(error) => return error.into_compile_error(),
        };
        let tree_model = if self.generate_model {
            match self.expand_generated_tree_model(&type_env, &auto_model_variants) {
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
        let mut helper_outputs = self.product_helper_outputs();
        if self.generate_model {
            helper_outputs.retain(|output| !self.generates_model_output_name(output));
        }
        let product_helpers = self.expand_product_helpers(&helper_outputs, &type_env);
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
                        &helper_outputs,
                        &type_env,
                        self.generate_model,
                        &model_outputs,
                        model_all_rules_local,
                        self.model_path.as_ref(),
                        rule.output(&type_env)
                            .is_some_and(|output| self.generates_model_output(output)),
                        &auto_model_variants,
                    )
                })
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        let partial_valid_parser_functions = if self.generate_partial_valid_parsers {
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
            } else if input.peek(kw::node) {
                rules.push(Rule::Node(input.parse()?));
            } else if input.peek(kw::product) {
                rules.push(Rule::Product(input.parse()?));
            } else {
                return Err(
                    input.error("expected `recursive`, `alias`, `rule`, `node`, or `product`")
                );
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
                Rule::Node(rule) => simple_type_ident(&rule.output),
                Rule::Product(rule) => simple_type_ident(&rule.0.output),
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

    fn auto_model_variants(&self, type_env: &GrammarTypeEnv) -> Result<BTreeMap<String, Ident>> {
        if !self.generate_model {
            return Ok(BTreeMap::new());
        }

        let mut plain_rules = BTreeMap::<String, Vec<(Ident, GeneratedStructModel)>>::new();
        let mut explicit_variants = BTreeMap::<String, BTreeSet<String>>::new();
        for rule in &self.rules {
            let (rule_kind, rule) = match rule {
                Rule::Alias(_) => continue,
                Rule::Struct(rule) => (GeneratedModelRuleKind::Node, rule),
                Rule::Node(rule) => (GeneratedModelRuleKind::Node, rule),
                Rule::Product(rule) => (GeneratedModelRuleKind::Product, &rule.0),
                Rule::Enum(_) => continue,
            };
            let Some(output) = simple_type_ident(&rule.output) else {
                continue;
            };
            let output_name = output.to_string();
            if !self.generates_model_output_name(&output_name) {
                continue;
            }
            match &rule.construction {
                ConstructionMode::NamedVariant(variant)
                | ConstructionMode::TupleVariant(variant) => {
                    let variant = rule.model_variant.as_ref().unwrap_or(variant);
                    explicit_variants
                        .entry(output_name)
                        .or_default()
                        .insert(variant.to_string());
                }
                ConstructionMode::Validated | ConstructionMode::Direct => {
                    let fields = rule.generated_model_fields(type_env)?;
                    plain_rules.entry(output_name).or_default().push((
                        rule.name.clone(),
                        GeneratedStructModel {
                            visibility: rule_kind.visibility_tokens(),
                            ident: output.clone(),
                            rule_name: rule.name.clone(),
                            fields,
                        },
                    ));
                }
            }
        }

        let mut auto_variants = BTreeMap::new();
        for (output, rules) in plain_rules {
            let has_explicit_variant = explicit_variants.contains_key(&output);
            let has_multiple_shapes = rules
                .first()
                .is_some_and(|(_, first)| rules.iter().any(|(_, rule)| !first.same_shape_as(rule)));
            if !has_explicit_variant && !has_multiple_shapes {
                continue;
            }

            let mut used_variants = explicit_variants.remove(&output).unwrap_or_default();
            for (rule_name, _) in rules {
                let mut variant = pascal_case_ident(&rule_name.to_string());
                if used_variants.contains(&variant.to_string()) {
                    let base = variant.to_string();
                    let mut suffix = 2usize;
                    loop {
                        variant = format_ident!("{base}{suffix}");
                        if !used_variants.contains(&variant.to_string()) {
                            break;
                        }
                        suffix += 1;
                    }
                }
                used_variants.insert(variant.to_string());
                auto_variants.insert(rule_name.to_string(), variant);
            }
        }

        Ok(auto_variants)
    }

    fn expand_generated_tree_model(
        &self,
        type_env: &GrammarTypeEnv,
        auto_model_variants: &BTreeMap<String, Ident>,
    ) -> Result<TokenStream2> {
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
        let generated_items = self.generated_tree_model_items(type_env, auto_model_variants)?;
        Ok(quote! {
            jbotci_tree::tree_model! {
                #(#attrs)*
                #(#manual_items)*
                #(#generated_items)*
            }
        })
    }

    fn generated_tree_model_items(
        &self,
        type_env: &GrammarTypeEnv,
        auto_model_variants: &BTreeMap<String, Ident>,
    ) -> Result<Vec<TokenStream2>> {
        let mut structs = BTreeMap::<String, GeneratedStructModel>::new();
        let mut enums = BTreeMap::<String, Vec<GeneratedVariantModel>>::new();
        for rule in &self.rules {
            let (rule_kind, rule) = match rule {
                Rule::Alias(_) => continue,
                Rule::Struct(rule) => (GeneratedModelRuleKind::Node, rule),
                Rule::Node(rule) => (GeneratedModelRuleKind::Node, rule),
                Rule::Product(rule) => (GeneratedModelRuleKind::Product, &rule.0),
                Rule::Enum(rule) => {
                    let Some(output) = simple_type_ident(&rule.output) else {
                        continue;
                    };
                    if !self.generates_model_output_name(&output.to_string()) {
                        continue;
                    }
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
                                tuple: false,
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
            match &rule.construction {
                ConstructionMode::NamedVariant(variant) => {
                    let fields = rule.generated_model_fields(type_env)?;
                    let variant = rule.model_variant.as_ref().unwrap_or(variant);
                    push_generated_variant(
                        &mut enums,
                        output.to_string(),
                        GeneratedVariantModel {
                            variant: variant.clone(),
                            rule_name: rule.name.clone(),
                            fields,
                            tuple: false,
                        },
                    )?;
                }
                ConstructionMode::TupleVariant(variant) => {
                    let fields = rule.generated_model_fields(type_env)?;
                    let variant = rule.model_variant.as_ref().unwrap_or(variant);
                    push_generated_variant(
                        &mut enums,
                        output.to_string(),
                        GeneratedVariantModel {
                            variant: variant.clone(),
                            rule_name: rule.name.clone(),
                            fields,
                            tuple: true,
                        },
                    )?;
                }
                ConstructionMode::Validated | ConstructionMode::Direct => {
                    if let Some(variant) = auto_model_variants.get(&rule.name.to_string()) {
                        let fields = rule.generated_model_fields(type_env)?;
                        push_generated_variant(
                            &mut enums,
                            output.to_string(),
                            GeneratedVariantModel {
                                variant: variant.clone(),
                                rule_name: rule.name.clone(),
                                fields,
                                tuple: false,
                            },
                        )?;
                        continue;
                    }
                    let key = output.to_string();
                    let fields = rule.generated_model_fields(type_env)?;
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
        Ok(items)
    }
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

fn syntax_type_ident_for_rule(name: &Ident) -> Ident {
    let base = pascal_case_ident(&name.to_string());
    format_ident!("{base}Syntax")
}

fn syntax_type_for_rule(name: &Ident) -> Type {
    let ident = syntax_type_ident_for_rule(name);
    parse_quote!(#ident)
}

fn enum_variant_ident_for_output(output: &Type, fallback: &Ident) -> Ident {
    let Some(output) = simple_type_ident(output) else {
        return pascal_case_ident(&fallback.to_string());
    };
    let output = output.to_string();
    let variant = output.strip_suffix("Syntax").unwrap_or(&output);
    format_ident!("{variant}")
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
    fn same_shape_as(&self, other: &Self) -> bool {
        token_streams_match(&self.visibility, &other.visibility)
            && self.ident == other.ident
            && self.fields.len() == other.fields.len()
            && self
                .fields
                .iter()
                .zip(&other.fields)
                .all(|(left, right)| left.same_shape_as(right))
    }

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
    fn same_shape_as(&self, other: &Self) -> bool {
        self.name == other.name
            && token_streams_match(&self.ty, &other.ty)
            && self.attrs.len() == other.attrs.len()
            && self
                .attrs
                .iter()
                .zip(&other.attrs)
                .all(|(left, right)| token_streams_match(&quote!(#left), &quote!(#right)))
    }

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
                && matches!(&rule.0.construction, ConstructionMode::Validated)
                && rule.0.fields.iter().all(|field| {
                    !matches!(
                        field.kind,
                        FieldKind::Default
                            | FieldKind::Computed
                            | FieldKind::Let
                            | FieldKind::TempLet
                            | FieldKind::Scratch
                    )
                })
                && let Some(output) = simple_type_ident(&rule.0.output)
                && !(self.generate_model && self.generates_model_output_name(&output.to_string()))
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
                (matches!(&rule.0.construction, ConstructionMode::Validated)
                    && !(self.generate_model
                        && self.generates_model_output_name(&output.to_string()))
                    && output_counts.get(&output.to_string()).copied() == Some(1)
                    && rule.0.fields.iter().all(|field| {
                        !matches!(
                            field.kind,
                            FieldKind::Default
                                | FieldKind::Computed
                                | FieldKind::Let
                                | FieldKind::TempLet
                                | FieldKind::Scratch
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
                Rule::Alias(_)
                | Rule::Product(_)
                | Rule::Node(_)
                | Rule::Struct(_)
                | Rule::Enum(_) => None,
            })
            .collect()
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
}

impl ParserExpr {
    fn compact_tokens(&self) -> String {
        match self {
            Self::Rust(expr) => compact_tokens(expr),
            Self::Vector(expr) => compact_tokens(&expr.to_token_stream()),
        }
    }

    fn to_token_stream(&self) -> TokenStream2 {
        match self {
            Self::Rust(expr) => quote!(#expr),
            Self::Vector(expr) => expr.to_token_stream(),
        }
    }

    fn rust_tokens(&self) -> TokenStream2 {
        self.to_token_stream()
    }
}

impl From<Expr> for ParserExpr {
    fn from(expr: Expr) -> Self {
        Self::Rust(expr)
    }
}

impl Parse for ParserExpr {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        if input.peek(syn::token::Bracket) {
            input.parse().map(Self::Vector)
        } else {
            input.parse().map(Self::Rust)
        }
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
    Node(NodeRule),
    Product(ProductRule),
    Enum(EnumRule),
}

impl Rule {
    fn name(&self) -> &Ident {
        match self {
            Rule::Alias(rule) => &rule.name,
            Rule::Struct(rule) => &rule.name,
            Rule::Node(rule) => &rule.name,
            Rule::Product(rule) => &rule.0.name,
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
            Rule::Node(rule) => Some(&rule.output),
            Rule::Product(rule) => Some(&rule.0.output),
            Rule::Enum(rule) => Some(&rule.output),
        }
    }

    fn expand_metadata(&self, type_env: &GrammarTypeEnv) -> Result<TokenStream2> {
        match self {
            Rule::Alias(rule) => rule.expand_metadata(type_env),
            Rule::Struct(rule) => Ok(rule.expand_metadata("struct")),
            Rule::Node(rule) => Ok(rule.expand_metadata("node")),
            Rule::Product(rule) => Ok(rule.0.expand_metadata("product")),
            Rule::Enum(rule) => rule.expand_metadata(type_env),
        }
    }

    fn arguments(&self) -> &[Ident] {
        match self {
            Rule::Alias(rule) => &rule.arguments,
            Rule::Struct(rule) => &rule.arguments,
            Rule::Node(rule) => &rule.arguments,
            Rule::Product(rule) => &rule.0.arguments,
            Rule::Enum(rule) => &rule.arguments,
        }
    }

    fn expand_strict_parser(
        &self,
        helper_outputs: &BTreeSet<String>,
        type_env: &GrammarTypeEnv,
        generate_model: bool,
        model_outputs: &Option<BTreeSet<String>>,
        model_all_rules_local: bool,
        model_path: Option<&Path>,
        use_model_construction: bool,
        auto_model_variants: &BTreeMap<String, Ident>,
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
                helper_outputs,
                type_env,
                generate_model,
                model_outputs,
                model_all_rules_local,
                model_path,
                use_model_construction,
                auto_model_variants,
            ),
            Rule::Node(rule) => rule.expand_strict_parser(
                helper_outputs,
                type_env,
                generate_model,
                model_outputs,
                model_all_rules_local,
                model_path,
                use_model_construction,
                auto_model_variants,
            ),
            Rule::Product(rule) => rule.0.expand_strict_parser(
                helper_outputs,
                type_env,
                generate_model,
                model_outputs,
                model_all_rules_local,
                model_path,
                use_model_construction,
                auto_model_variants,
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
        helper_outputs: &BTreeSet<String>,
        type_env: &GrammarTypeEnv,
        recovered_module: &TokenStream2,
    ) -> Option<TokenStream2> {
        match self {
            Rule::Alias(rule) => rule.expand_partial_valid_parser(type_env, recovered_module),
            Rule::Struct(rule) => {
                rule.expand_partial_valid_parser(helper_outputs, type_env, recovered_module)
            }
            Rule::Node(rule) => {
                rule.expand_partial_valid_parser(helper_outputs, type_env, recovered_module)
            }
            Rule::Product(rule) => {
                rule.0
                    .expand_partial_valid_parser(helper_outputs, type_env, recovered_module)
            }
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
            return Err(input.error("alias rules must use `=`; use `guard` or `guard_not` for parser-only assertions"));
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
                let branch_output = type_env
                    .rules
                    .get(&branch_name)
                    .or_else(|| type_env.recursive.get(&branch_name))?;
                let variant = enum_variant_ident_for_output(branch_output, &branch.name);
                let field = &branch.name;
                let branch_parser = if type_env.rules.contains_key(&branch_name) {
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
                    quote!(#output_tokens::#variant { #field })
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
    construction: ConstructionMode,
    model_variant: Option<Ident>,
    no_partial_valid: bool,
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
                FieldKind::Field | FieldKind::Computed | FieldKind::Let | FieldKind::Default => {
                    Some(field)
                }
                FieldKind::Scratch | FieldKind::TempLet | FieldKind::Require => None,
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
        generate_model: bool,
        model_outputs: &Option<BTreeSet<String>>,
        model_all_rules_local: bool,
        model_path: Option<&Path>,
        use_model_construction: bool,
        auto_model_variants: &BTreeMap<String, Ident>,
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
        let body = if !use_model_construction
            && simple_type_ident(output).is_some_and(|output| {
                helper_outputs.contains(&output.to_string())
                    && self.fields.iter().all(|field| {
                        !matches!(
                            field.kind,
                            FieldKind::Default
                                | FieldKind::Computed
                                | FieldKind::Let
                                | FieldKind::TempLet
                                | FieldKind::Scratch
                        )
                    })
            }) {
            let field_names = fields
                .iter()
                .map(|field| field.name.as_ref().expect("field items have names"));
            quote!(#output_tokens { #(#field_names,)* })
        } else if is_unit_type(output) {
            let let_bindings = self.fields.iter().filter_map(|field| {
                matches!(
                    field.kind,
                    FieldKind::Computed | FieldKind::Let | FieldKind::TempLet
                )
                .then(|| {
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
                matches!(
                    field.kind,
                    FieldKind::Computed | FieldKind::Let | FieldKind::TempLet
                )
                .then(|| {
                    let name = field.name.as_ref().expect("let field items have names");
                    let value = field.parser.rust_tokens();
                    quote!(let #name = #value;)
                })
            });
            let values = self.fields.iter().filter_map(|field| {
                let name = field.name.as_ref()?;
                match field.kind {
                    FieldKind::Field | FieldKind::Computed | FieldKind::Let => Some(quote!(#name)),
                    FieldKind::Default => {
                        let value = field.parser.rust_tokens();
                        Some(quote!(#value))
                    }
                    FieldKind::Scratch | FieldKind::TempLet | FieldKind::Require => None,
                }
            });
            quote!({
                #(#let_bindings)*
                (#(#values,)*)
            })
        } else if is_path_type(output) {
            let let_bindings = self.fields.iter().filter_map(|field| {
                matches!(
                    field.kind,
                    FieldKind::Computed | FieldKind::Let | FieldKind::TempLet
                )
                .then(|| {
                    let name = field.name.as_ref().expect("let field items have names");
                    let value = field.parser.rust_tokens();
                    quote!(let #name = #value;)
                })
            });
            let assignments = self.fields.iter().filter_map(|field| {
                let name = field.name.as_ref()?;
                match field.kind {
                    FieldKind::Field | FieldKind::Computed | FieldKind::Let => Some(quote!(#name,)),
                    FieldKind::Default => {
                        let value = field.parser.rust_tokens();
                        Some(quote!(#name: #value,))
                    }
                    FieldKind::Scratch | FieldKind::TempLet | FieldKind::Require => None,
                }
            });
            let auto_model_variant = use_model_construction
                .then(|| auto_model_variants.get(&self.name.to_string()))
                .flatten();
            if let Some(variant) = auto_model_variant {
                match &self.construction {
                    ConstructionMode::Validated | ConstructionMode::Direct => quote!({
                        #(#let_bindings)*
                        #output_tokens::#variant { #(#assignments)* }
                    }),
                    ConstructionMode::NamedVariant(_) | ConstructionMode::TupleVariant(_) => {
                        return None;
                    }
                }
            } else {
                match &self.construction {
                    ConstructionMode::Validated if use_model_construction => {
                        quote!({
                            #(#let_bindings)*
                            #output_tokens { #(#assignments)* }
                        })
                    }
                    ConstructionMode::Validated => {
                        quote!({
                            #(#let_bindings)*
                            bityzba::new!(#output_tokens { #(#assignments)* })
                        })
                    }
                    ConstructionMode::Direct => {
                        quote!({
                            #(#let_bindings)*
                            #output_tokens { #(#assignments)* }
                        })
                    }
                    ConstructionMode::NamedVariant(variant) if use_model_construction => {
                        let variant = self.model_variant.as_ref().unwrap_or(variant);
                        quote!({
                            #(#let_bindings)*
                            #output_tokens::#variant { #(#assignments)* }
                        })
                    }
                    ConstructionMode::NamedVariant(variant) => {
                        quote!({
                            #(#let_bindings)*
                            bityzba::new!(#output_tokens::#variant { #(#assignments)* })
                        })
                    }
                    ConstructionMode::TupleVariant(variant) => {
                        let values = self.fields.iter().filter_map(|field| {
                            let name = field.name.as_ref()?;
                            match field.kind {
                                FieldKind::Field | FieldKind::Computed | FieldKind::Let => {
                                    Some(quote!(#name))
                                }
                                FieldKind::Default => {
                                    let value = field.parser.rust_tokens();
                                    Some(quote!(#value))
                                }
                                FieldKind::Scratch | FieldKind::TempLet | FieldKind::Require => {
                                    None
                                }
                            }
                        });
                        if use_model_construction {
                            let variant = self.model_variant.as_ref().unwrap_or(variant);
                            quote!({
                                #(#let_bindings)*
                                #output_tokens::#variant(#(#values,)*)
                            })
                        } else {
                            quote!({
                                #(#let_bindings)*
                                bityzba::new!(#output_tokens::#variant(#(#values,)*))
                            })
                        }
                    }
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
        helper_outputs: &BTreeSet<String>,
        type_env: &GrammarTypeEnv,
        recovered_module: &TokenStream2,
    ) -> Option<TokenStream2> {
        if self.no_partial_valid {
            return None;
        }
        let has_default = self
            .fields
            .iter()
            .any(|field| matches!(field.kind, FieldKind::Default));
        let has_let = self.fields.iter().any(|field| {
            matches!(
                field.kind,
                FieldKind::Computed | FieldKind::Let | FieldKind::TempLet
            )
        });
        let has_scratch = self
            .fields
            .iter()
            .any(|field| matches!(field.kind, FieldKind::Scratch));
        let can_generate_strict = if is_unit_type(&self.output) {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StrictParserCallMode {
    Local,
    Legacy,
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

    fn legacy_recursive_parser(&self, name: &Ident) -> TokenStream2 {
        quote!(super::strict_generated_parser_family().#name)
    }

    fn legacy_free_modifier_parser(&self) -> TokenStream2 {
        let name = format_ident!("free_modifier");
        if self.recursive_has_local_parser("free_modifier") {
            self.legacy_recursive_parser(&name)
        } else {
            quote!(__generated_free_modifier.clone())
        }
    }
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

        type_env
    }
}

impl GrammarTypeEnv {
    fn rule_arguments_for_call(&self, rule: &str) -> Option<&[String]> {
        self.rule_arguments.get(rule).map(Vec::as_slice)
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
        FieldKind::Field | FieldKind::Scratch => field
            .name
            .as_ref()
            .expect("field items have names")
            .to_token_stream(),
        FieldKind::Require => quote!(_),
        FieldKind::Computed | FieldKind::Let | FieldKind::TempLet | FieldKind::Default => {
            unreachable!("computed/default items are not parser sequence items")
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
        let parser_name = if mode == StrictParserCallMode::Legacy
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
        ("some", 1) => {
            let inner = strict_rust_parser_expr_tokens(
                call.args.first().expect("length checked"),
                arguments,
                generation,
                free_modifier_parser,
                mode,
            )?;
            Some(quote!(#inner.map(Some)))
        }
        ("many", 1) => {
            let inner = strict_rust_parser_expr_tokens(
                call.args.first().expect("length checked"),
                arguments,
                generation,
                free_modifier_parser,
                mode,
            )?;
            Some(quote!(generated_runtime::strict_greedy_many_parser(#inner.boxed())))
        }
        ("many1", 1) => {
            let inner = strict_rust_parser_expr_tokens(
                call.args.first().expect("length checked"),
                arguments,
                generation,
                free_modifier_parser,
                mode,
            )?;
            Some(quote!(generated_runtime::strict_greedy_many1_parser(#inner.boxed())))
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
        ("guard", 2) => {
            let predicate = strict_rust_parser_expr_tokens(
                call.args.first().expect("length checked"),
                arguments,
                generation,
                free_modifier_parser,
                mode,
            )?;
            let parser = strict_rust_parser_expr_tokens(
                call.args.iter().nth(1).expect("length checked"),
                arguments,
                generation,
                free_modifier_parser,
                mode,
            )?;
            Some(quote!(generated_runtime::lookahead(#predicate).ignore_then(#parser)))
        }
        ("guard_not", 2) => {
            let predicate = strict_rust_parser_expr_tokens(
                call.args.first().expect("length checked"),
                arguments,
                generation,
                free_modifier_parser,
                mode,
            )?;
            let parser = strict_rust_parser_expr_tokens(
                call.args.iter().nth(1).expect("length checked"),
                arguments,
                generation,
                free_modifier_parser,
                mode,
            )?;
            Some(quote!(generated_runtime::not(#predicate).ignore_then(#parser)))
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
    let call_mode = if mode == StrictParserCallMode::Legacy
        || (generation.generate_model && !generation.rule_has_local_parser(function))
    {
        StrictParserCallMode::Legacy
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
    let call_mode = if mode == StrictParserCallMode::Legacy
        || (generation.generate_model && !generation.rule_has_local_parser(function))
    {
        StrictParserCallMode::Legacy
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
    let parser_name = if call_mode == StrictParserCallMode::Legacy {
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
    if mode == StrictParserCallMode::Legacy
        && generation.recursive_has_local_parser(&argument.to_string())
    {
        Some(generation.legacy_recursive_parser(&argument))
    } else {
        Some(quote!(#argument.clone()))
    }
}

fn strict_free_modifier_argument_tokens(
    generation: &StrictParserGeneration<'_>,
    free_modifier_parser: &Ident,
    mode: StrictParserCallMode,
) -> TokenStream2 {
    if mode == StrictParserCallMode::Legacy {
        generation.legacy_free_modifier_parser()
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
        ("raw_words_until", _) if !call.args.is_empty() => Some(quote!(Vec<Token>)),
        ("opt", 1) => {
            let inner = rust_parser_output_type(
                call.args.first().expect("length checked"),
                type_env,
                arguments,
            )?;
            Some(quote!(Option<#inner>))
        }
        ("some", 1) => {
            let inner = rust_parser_output_type(
                call.args.first().expect("length checked"),
                type_env,
                arguments,
            )?;
            Some(quote!(Option<#inner>))
        }
        ("many" | "many1", 1) => {
            let inner = rust_parser_output_type(
                call.args.first().expect("length checked"),
                type_env,
                arguments,
            )?;
            Some(quote!(Vec<#inner>))
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
        ("guard" | "guard_not", 2) => rust_parser_output_type(
            call.args.iter().nth(1).expect("length checked"),
            type_env,
            arguments,
        ),
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
            construction: ConstructionMode::Validated,
            model_variant: None,
            no_partial_valid: false,
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
        let parser: Expr = input.parse()?;
        input.parse::<Token![;]>()?;
        let parser: ParserExpr = if negated {
            syn::parse2::<Expr>(quote!(#parser.not()))
        } else {
            syn::parse2::<Expr>(quote!(#parser.lookahead().ignored()))
        }?
        .into();
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

fn parse_rule_after_kind(input: ParseStream<'_>) -> Result<NodeRule> {
    let name = input.parse()?;
    let arguments = parse_optional_arguments(input)?;
    input.parse::<Token![->]>()?;
    let output = input.parse()?;
    let content;
    braced!(content in input);

    let mut context = None;
    let mut fields = Vec::new();
    let mut construction = ConstructionMode::Validated;
    let mut model_variant = None;
    let mut no_partial_valid = false;
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
        } else if content.peek(kw::model_variant) {
            content.parse::<kw::model_variant>()?;
            model_variant = Some(content.parse()?);
            content.parse::<Token![;]>()?;
        } else if content.peek(kw::no_partial_valid) {
            content.parse::<kw::no_partial_valid>()?;
            no_partial_valid = true;
            content.parse::<Token![;]>()?;
        } else if content.peek(kw::fields) {
            fields = parse_fields_block(&content)?;
        } else if content.peek(kw::build) {
            return Err(content.error(
                "`build` blocks are no longer supported; use declarative fields, `default`, `let`, `scratch`, `require`, aliases, products, and construct variants",
            ));
        } else {
            return Err(content.error(
                "expected `context`, `construct`, `model_variant`, `no_partial_valid`, or `fields`",
            ));
        }
    }

    Ok(NodeRule {
        name,
        arguments,
        output,
        context,
        fields,
        construction,
        model_variant,
        no_partial_valid,
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
            (None, FieldKind::Computed | FieldKind::Let | FieldKind::Default) => {
                return Err(syn::Error::new_spanned(
                    self.parser.to_token_stream(),
                    "computed/default generated model fields require an explicit `: Type` annotation",
                ));
            }
            (None, FieldKind::Scratch | FieldKind::TempLet | FieldKind::Require) => {
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
        let parser = input.parse::<Expr>()?.into();
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
    Computed,
    Scratch,
    Let,
    TempLet,
    Default,
    Require,
}

impl FieldKind {
    fn as_str(&self) -> &'static str {
        match self {
            FieldKind::Field => "field",
            FieldKind::Computed => "field",
            FieldKind::Scratch => "scratch",
            FieldKind::Let => "let",
            FieldKind::TempLet => "let",
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
        ("many", 1) => {
            RecoveryExpr::Many(Box::new(classify_recovery_expr(&call.args[0], arguments)))
        }
        ("many1", 1) => {
            RecoveryExpr::Many1(Box::new(classify_recovery_expr(&call.args[0], arguments)))
        }
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
        ("guard", 2) => RecoveryExpr::Sequence(vec![
            RecoveryExpr::Lookahead(Box::new(classify_recovery_expr(&call.args[0], arguments))),
            classify_recovery_expr(&call.args[1], arguments),
        ]),
        ("guard_not", 2) => RecoveryExpr::Sequence(vec![
            RecoveryExpr::Not(Box::new(classify_recovery_expr(&call.args[0], arguments))),
            classify_recovery_expr(&call.args[1], arguments),
        ]),
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
            node item -> ItemSyntax {
                fields {
                    field token = cmavo(Be);
                }
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
                .contains("`build` blocks are no longer supported"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn grammar_does_not_support_recovered_build_blocks() {
        let result = syn::parse2::<SyntaxGrammar>(quote! {
            node item -> ItemSyntax {
                fields {
                    field token = cmavo(Be);
                }
                recovered_build |token| ItemSyntax { token };
            }
        });

        assert!(
            result.is_err(),
            "recovered_build blocks must be unsupported"
        );
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
            error
                .to_string()
                .contains("alias rules must use `=`"),
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
    fn grammar_rejects_duplicate_generated_struct_outputs() {
        let grammar = syn::parse2::<SyntaxGrammar>(quote! {
            tree_model {}
            model;

            node first_item -> ItemSyntax {
                fields {
                    field token = cmavo(Be);
                }
            }

            node second_item -> ItemSyntax {
                fields {
                    field token = cmavo(Be);
                }
            }
        })
        .expect("grammar parses before generated model expansion");

        let expanded = grammar.expand().to_string();
        assert!(
            expanded.contains("cannot generate one struct")
                && expanded.contains("generated model ownership must be one rule per struct"),
            "unexpected expansion: {expanded}"
        );
    }

    #[test]
    fn grammar_rejects_duplicate_generated_enum_variants() {
        let grammar = syn::parse2::<SyntaxGrammar>(quote! {
            tree_model {}
            model;

            node first_item -> ChoiceSyntax {
                construct variant Item;
                fields {
                    field token = cmavo(Be);
                }
            }

            node second_item -> ChoiceSyntax {
                construct variant Item;
                fields {
                    field token = cmavo(Be);
                }
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
