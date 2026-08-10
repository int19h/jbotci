//! Compile-time Python projection of the canonical generated syntax schema.

use std::collections::{BTreeSet, VecDeque};
use std::fmt::Write as _;

#[allow(unused_imports)]
use bityzba::{data, ensures, invariant, new, requires};
use proc_macro::TokenStream;
use proc_macro2::{Delimiter, Group, Ident, Literal, TokenStream as TokenStream2, TokenTree};
use quote::{format_ident, quote};
use syn::{Error, LitInt, LitStr, Result};

/// Consume schema version 1 and generate the complete native syntax binding.
#[requires(true)]
#[ensures(true)]
#[proc_macro]
pub fn generate_syntax_bindings(input: TokenStream) -> TokenStream {
    match parse_schema(input.into()).and_then(expand_schema) {
        Ok(output) => output.into(),
        Err(error) => error.into_compile_error().into(),
    }
}

/// Consume schema version 1 and generate the Rust/Python parity inventory.
///
/// This shares the parser and normalized model representation with
/// `generate_syntax_bindings`, so grammar additions cannot be hidden by a
/// separately maintained generated-model list.
#[requires(true)]
#[ensures(true)]
#[proc_macro]
pub fn generate_syntax_parity_inventory(input: TokenStream) -> TokenStream {
    match parse_schema(input.into()).and_then(expand_parity_inventory) {
        Ok(output) => output.into(),
        Err(error) => error.into_compile_error().into(),
    }
}

#[invariant(true, "every token stream has a valid cursor position")]
#[derive(Debug)]
struct Cursor {
    tokens: VecDeque<TokenTree>,
}

impl Cursor {
    #[requires(true)]
    #[ensures(true)]
    fn new(stream: TokenStream2) -> Self {
        Self {
            tokens: stream.into_iter().collect(),
        }
    }

    #[requires(true)]
    #[ensures(ret == self.tokens.is_empty())]
    fn is_done(&self) -> bool {
        self.tokens.is_empty()
    }

    #[requires(true)]
    #[ensures(ret.is_some() == !old(self.tokens.is_empty()))]
    fn take(&mut self) -> Option<TokenTree> {
        self.tokens.pop_front()
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|value| !value.is_empty()) || ret.is_err())]
    fn take_ident(&mut self) -> Result<String> {
        match self.take() {
            Some(TokenTree::Ident(ident)) => Ok(ident.to_string()),
            Some(token) => Err(Error::new_spanned(token, "expected schema identifier")),
            None => Err(Error::new(
                proc_macro2::Span::call_site(),
                "expected schema identifier, found end of input",
            )),
        }
    }

    #[requires(!expected.is_empty())]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn expect_ident(&mut self, expected: &str) -> Result<()> {
        let actual = self.take_ident()?;
        if actual == expected {
            Ok(())
        } else {
            Err(Error::new(
                proc_macro2::Span::call_site(),
                format!("expected schema identifier `{expected}`, found `{actual}`"),
            ))
        }
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn take_group(&mut self, delimiter: Delimiter) -> Result<TokenStream2> {
        match self.take() {
            Some(TokenTree::Group(group)) if group.delimiter() == delimiter => Ok(group.stream()),
            Some(token) => Err(Error::new_spanned(
                token,
                format!("expected {delimiter:?} schema group"),
            )),
            None => Err(Error::new(
                proc_macro2::Span::call_site(),
                format!("expected {delimiter:?} schema group, found end of input"),
            )),
        }
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn take_literal(&mut self) -> Result<Literal> {
        match self.take() {
            Some(TokenTree::Literal(literal)) => Ok(literal),
            Some(token) => Err(Error::new_spanned(token, "expected schema literal")),
            None => Err(Error::new(
                proc_macro2::Span::call_site(),
                "expected schema literal, found end of input",
            )),
        }
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn take_lit_string(&mut self) -> Result<LitStr> {
        let literal = self.take_literal()?;
        syn::parse_str::<LitStr>(&literal.to_string())
            .map_err(|_| Error::new_spanned(&literal, "expected schema string literal"))
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|value| !value.is_empty()) || ret.is_err())]
    fn take_string(&mut self) -> Result<String> {
        let literal = self.take_lit_string()?;
        let value = literal.value();
        if value.is_empty() {
            Err(Error::new_spanned(
                literal,
                "schema string literals must not be empty",
            ))
        } else {
            Ok(value)
        }
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn take_usize(&mut self) -> Result<usize> {
        let literal = self.take_literal()?;
        syn::parse_str::<LitInt>(&literal.to_string())
            .and_then(|literal| literal.base10_parse::<usize>())
            .map_err(|_| Error::new_spanned(literal, "expected schema usize literal"))
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn take_bool(&mut self) -> Result<bool> {
        match self.take_ident()?.as_str() {
            "true" => Ok(true),
            "false" => Ok(false),
            other => Err(Error::new(
                proc_macro2::Span::call_site(),
                format!("expected schema boolean, found `{other}`"),
            )),
        }
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn expect_punct(&mut self, expected: char) -> Result<()> {
        match self.take() {
            Some(TokenTree::Punct(punct)) if punct.as_char() == expected => Ok(()),
            Some(token) => Err(Error::new_spanned(
                token,
                format!("expected schema punctuation `{expected}`"),
            )),
            None => Err(Error::new(
                proc_macro2::Span::call_site(),
                format!("expected schema punctuation `{expected}`, found end of input"),
            )),
        }
    }

    #[requires(true)]
    #[ensures(ret.is_ok() == self.is_done())]
    fn finish(&self) -> Result<()> {
        if self.is_done() {
            Ok(())
        } else {
            Err(Error::new_spanned(
                self.tokens.iter().cloned().collect::<TokenStream2>(),
                "unsupported trailing schema field or token",
            ))
        }
    }
}

#[invariant(*version == 1, "only canonical schema version 1 is supported")]
#[invariant(!models.is_empty(), "the generated syntax schema is non-empty")]
#[derive(Debug, Clone, PartialEq, Eq)]
struct Schema {
    version: usize,
    models: Vec<Model>,
    metadata: Metadata,
}

#[invariant(::Product { common, shape, fields } =>
    !common.strict_name.is_empty()
        && !common.recovered_name.is_empty()
        && ((*shape == Shape::Tuple) == (fields.len() == 1)))]
#[invariant(::Sum { common, variants } =>
    !common.strict_name.is_empty()
        && !common.recovered_name.is_empty()
        && !variants.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
enum Model {
    Product {
        common: ModelCommon,
        shape: Shape,
        fields: Vec<Field>,
    },
    Sum {
        common: ModelCommon,
        variants: Vec<Variant>,
    },
}

impl Model {
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn strict_name(&self) -> &str {
        match self.as_data() {
            data!(Model::Product { common, .. }) | data!(Model::Sum { common, .. }) => {
                &common.strict_name
            }
        }
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn recovered_name(&self) -> &str {
        match self.as_data() {
            data!(Model::Product { common, .. }) | data!(Model::Sum { common, .. }) => {
                &common.recovered_name
            }
        }
    }
}

#[invariant(!strict_name.is_empty())]
#[invariant(!recovered_name.is_empty())]
#[invariant(!rule.is_empty())]
#[invariant(docs.iter().any(|line| !line.trim().is_empty()))]
#[invariant(!constructor.name.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
struct ModelCommon {
    strict_name: String,
    recovered_name: String,
    rule: String,
    docs: Vec<String>,
    constructor: Constructor,
}

#[invariant(!name.is_empty())]
#[invariant(!owner_rule.is_empty())]
#[invariant(!source_rule.is_empty())]
#[invariant(docs.iter().any(|line| !line.trim().is_empty()))]
#[invariant(!constructor.name.is_empty())]
#[invariant((*shape == Shape::Tuple) == (fields.len() == 1))]
#[derive(Debug, Clone, PartialEq, Eq)]
struct Variant {
    name: String,
    owner_rule: String,
    source_rule: String,
    docs: Vec<String>,
    constructor: Constructor,
    shape: Shape,
    fields: Vec<Field>,
}

#[invariant(!source_name.is_empty())]
#[invariant(docs.iter().any(|line| !line.trim().is_empty()))]
#[derive(Debug, Clone, PartialEq, Eq)]
struct Field {
    source_name: String,
    rust_name: RustName,
    index: usize,
    docs: Vec<String>,
    strict: BindingType,
    recovered: BindingType,
}

#[invariant(::Named { name } => !name.is_empty())]
#[invariant(::Tuple { .. } => true)]
#[derive(Debug, Clone, PartialEq, Eq)]
enum RustName {
    Named { name: String },
    Tuple { index: usize },
}

#[invariant(!name.is_empty())]
#[invariant(label.as_ref().is_none_or(|value| !value.is_empty()))]
#[derive(Debug, Clone, PartialEq, Eq)]
struct Constructor {
    name: String,
    label: Option<String>,
}

#[invariant(true, "both schema-v1 shapes are valid")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Shape {
    Named,
    Tuple,
}

#[invariant(::ModelReference { name } => !name.is_empty())]
#[invariant(::LeafReference { path, .. } => !path.is_empty() && path.iter().all(|part| !part.is_empty()))]
#[invariant(::Optional { .. } => true)]
#[invariant(::Repeated { .. } => true)]
#[invariant(::NonEmptyRepeated { .. } => true)]
#[invariant(::Boxed { .. } => true)]
#[invariant(::Shared { .. } => true)]
#[invariant(::RecoveredField { .. } => true)]
#[invariant(::WithIndicators { .. } => true)]
#[invariant(::WithFreeModifiers { .. } => true)]
#[invariant(::Chain { .. } => true)]
#[invariant(::Tuple { .. } => true)]
#[invariant(::Fixed { length, .. } => *length > 0)]
#[derive(Debug, Clone, PartialEq, Eq)]
enum BindingType {
    ModelReference {
        name: String,
    },
    LeafReference {
        kind: LeafKind,
        absolute: bool,
        path: Vec<String>,
    },
    Optional {
        value: Box<BindingType>,
    },
    Repeated {
        value: Box<BindingType>,
    },
    NonEmptyRepeated {
        value: Box<BindingType>,
    },
    Boxed {
        value: Box<BindingType>,
    },
    Shared {
        value: Box<BindingType>,
    },
    RecoveredField {
        value: Box<BindingType>,
    },
    WithIndicators {
        value: Box<BindingType>,
    },
    WithFreeModifiers {
        value: Box<BindingType>,
        free_modifiers: Box<BindingType>,
    },
    Chain {
        first: Box<BindingType>,
        links: Box<BindingType>,
    },
    Tuple {
        elements: Vec<BindingType>,
    },
    Fixed {
        length: usize,
        value: Box<BindingType>,
    },
}

#[invariant(true, "the schema parser enumerates every version-1 leaf kind")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LeafKind {
    SyntaxToken,
    MorphologyCmavo,
    MorphologySelmaho,
    MorphologyWord,
    MorphologyWordLike,
    SourceId,
    SourceSpan,
    Boolean,
    Integer,
    String,
    External,
}

#[invariant(true, "both generated syntax namespaces use the same schema")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProjectionMode {
    Strict,
    Recovered,
}

const LENS_OPTION_VALUE: usize = 10;
const LENS_SEQUENCE_ITEM: usize = 11;
const LENS_RECOVERED_VALID: usize = 12;
const LENS_RECOVERED_ERROR: usize = 13;
const LENS_RECOVERED_PREFIX: usize = 14;
const LENS_WITH_INDICATORS: usize = 15;
const LENS_WITH_FREE_VALUE: usize = 16;
const LENS_WITH_FREE_MODIFIERS: usize = 17;
const LENS_CHAIN_FIRST: usize = 18;
const LENS_CHAIN_LINK: usize = 19;
const LENS_TUPLE_ITEM: usize = 20;

impl ProjectionMode {
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn module_name(self) -> &'static str {
        match self {
            Self::Strict => "jbotci.syntax.strict",
            Self::Recovered => "jbotci.syntax.recovered",
        }
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn mode_name(self) -> &'static str {
        match self {
            Self::Strict => "strict",
            Self::Recovered => "recovered",
        }
    }
}

// One dispatcher match arm together with the outlined function holding its
// body.  Arm bodies must live in dedicated `#[inline(never)]` functions: an
// unoptimized build gives every inline arm's locals distinct stack slots, so a
// schema-wide dispatcher with all bodies inline accumulates a multi-megabyte
// frame and overflows the default thread stack on entry.
#[invariant(true)]
struct OutlinedMatchArm {
    function: TokenStream2,
    arm: TokenStream2,
}

#[invariant(
    true,
    "metadata is validated structurally even when not emitted publicly"
)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct Metadata {
    transparent_constructors: Vec<String>,
    transparent_fields: Vec<(String, String)>,
    chain_link_element_fields: Vec<(String, String)>,
    constructor_labels: Vec<(String, String)>,
    elidable_terminators: Vec<(String, String)>,
    field_orders: Vec<(String, Vec<String>)>,
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn parse_schema(input: TokenStream2) -> Result<Schema> {
    let mut root = Cursor::new(input);
    root.expect_ident("syntax_binding_schema")?;
    let body = root.take_group(Delimiter::Brace)?;
    root.finish()?;

    let mut body = Cursor::new(body);
    body.expect_ident("version")?;
    let mut version_group = Cursor::new(body.take_group(Delimiter::Parenthesis)?);
    let version = version_group.take_usize()?;
    version_group.finish()?;
    if version != 1 {
        return Err(Error::new(
            proc_macro2::Span::call_site(),
            format!("unsupported syntax binding schema version {version}; expected version 1"),
        ));
    }
    body.expect_punct(',')?;
    body.expect_ident("models")?;
    let models = parse_models(body.take_group(Delimiter::Bracket)?)?;
    body.expect_punct(',')?;
    body.expect_ident("metadata")?;
    let metadata = parse_metadata(body.take_group(Delimiter::Brace)?)?;
    body.finish()?;
    validate_model_references(&models)?;
    validate_metadata(&metadata)?;
    Ok(new!(Schema {
        version,
        models,
        metadata,
    }))
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn validate_metadata(metadata: &Metadata) -> Result<()> {
    let has_empty_value = metadata
        .transparent_constructors
        .iter()
        .any(String::is_empty)
        || metadata
            .transparent_fields
            .iter()
            .chain(&metadata.chain_link_element_fields)
            .chain(&metadata.constructor_labels)
            .chain(&metadata.elidable_terminators)
            .any(|(first, second)| first.is_empty() || second.is_empty())
        || metadata.field_orders.iter().any(|(constructor, fields)| {
            constructor.is_empty() || fields.iter().any(String::is_empty)
        });
    if has_empty_value {
        Err(Error::new(
            proc_macro2::Span::call_site(),
            "syntax binding schema metadata contains an empty identifier",
        ))
    } else {
        Ok(())
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|models| !models.is_empty()) || ret.is_err())]
fn parse_models(input: TokenStream2) -> Result<Vec<Model>> {
    let mut cursor = Cursor::new(input);
    let mut models = Vec::new();
    while !cursor.is_done() {
        models.push(parse_model(&mut cursor)?);
        if !cursor.is_done() {
            cursor.expect_punct(',')?;
        }
    }
    if models.is_empty() {
        Err(Error::new(
            proc_macro2::Span::call_site(),
            "syntax binding schema must contain at least one model",
        ))
    } else {
        Ok(models)
    }
}

#[requires(!cursor.is_done())]
#[ensures(ret.is_ok() || ret.is_err())]
fn parse_model(cursor: &mut Cursor) -> Result<Model> {
    match cursor.take_ident()?.as_str() {
        "product" => parse_product(cursor.take_group(Delimiter::Brace)?),
        "sum" => parse_sum(cursor.take_group(Delimiter::Brace)?),
        other => Err(Error::new(
            proc_macro2::Span::call_site(),
            format!("unknown syntax schema model kind `{other}`"),
        )),
    }
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn parse_product(input: TokenStream2) -> Result<Model> {
    let mut cursor = Cursor::new(input);
    let (strict_name, recovered_name) = parse_names(&mut cursor)?;
    cursor.expect_punct(',')?;
    let rule = parse_string_property(&mut cursor, "rule")?;
    cursor.expect_punct(',')?;
    let docs = parse_docs(&mut cursor)?;
    cursor.expect_punct(',')?;
    let constructor = parse_constructor(&mut cursor)?;
    cursor.expect_punct(',')?;
    let shape = parse_shape(&mut cursor)?;
    cursor.expect_punct(',')?;
    let fields = parse_fields(&mut cursor)?;
    cursor.finish()?;
    validate_fields(&fields, shape)?;
    Ok(new!(Model::Product {
        common: new!(ModelCommon {
            strict_name,
            recovered_name,
            rule,
            docs,
            constructor,
        }),
        shape,
        fields,
    }))
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn parse_sum(input: TokenStream2) -> Result<Model> {
    let mut cursor = Cursor::new(input);
    let (strict_name, recovered_name) = parse_names(&mut cursor)?;
    cursor.expect_punct(',')?;
    let rule = parse_string_property(&mut cursor, "rule")?;
    cursor.expect_punct(',')?;
    let docs = parse_docs(&mut cursor)?;
    cursor.expect_punct(',')?;
    let constructor = parse_constructor(&mut cursor)?;
    cursor.expect_punct(',')?;
    cursor.expect_ident("variants")?;
    let variants = parse_variants(cursor.take_group(Delimiter::Bracket)?)?;
    cursor.finish()?;
    if variants.is_empty() {
        return Err(Error::new(
            proc_macro2::Span::call_site(),
            "schema sum must contain at least one variant",
        ));
    }
    Ok(new!(Model::Sum {
        common: new!(ModelCommon {
            strict_name,
            recovered_name,
            rule,
            docs,
            constructor,
        }),
        variants,
    }))
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn parse_names(cursor: &mut Cursor) -> Result<(String, String)> {
    cursor.expect_ident("names")?;
    let mut names = Cursor::new(cursor.take_group(Delimiter::Parenthesis)?);
    let strict = parse_string_property(&mut names, "strict")?;
    names.expect_punct(',')?;
    let recovered = parse_string_property(&mut names, "recovered")?;
    names.finish()?;
    Ok((strict, recovered))
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn parse_constructor(cursor: &mut Cursor) -> Result<Constructor> {
    cursor.expect_ident("constructor")?;
    let mut constructor = Cursor::new(cursor.take_group(Delimiter::Parenthesis)?);
    let name = parse_string_property(&mut constructor, "name")?;
    constructor.expect_punct(',')?;
    constructor.expect_ident("label")?;
    let mut label_group = Cursor::new(constructor.take_group(Delimiter::Parenthesis)?);
    let label = match label_group.take_ident()?.as_str() {
        "none" => None,
        "some" => Some(parse_string_group(&mut label_group)?),
        other => {
            return Err(Error::new(
                proc_macro2::Span::call_site(),
                format!("unknown constructor label form `{other}`"),
            ));
        }
    };
    label_group.finish()?;
    constructor.finish()?;
    Ok(new!(Constructor { name, label }))
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn parse_shape(cursor: &mut Cursor) -> Result<Shape> {
    cursor.expect_ident("shape")?;
    let mut shape_group = Cursor::new(cursor.take_group(Delimiter::Parenthesis)?);
    let shape = match shape_group.take_ident()?.as_str() {
        "named" => Shape::Named,
        "tuple" => Shape::Tuple,
        other => {
            return Err(Error::new(
                proc_macro2::Span::call_site(),
                format!("unknown schema shape `{other}`"),
            ));
        }
    };
    shape_group.finish()?;
    Ok(shape)
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn parse_variants(input: TokenStream2) -> Result<Vec<Variant>> {
    let mut cursor = Cursor::new(input);
    let mut variants = Vec::new();
    while !cursor.is_done() {
        cursor.expect_ident("variant")?;
        variants.push(parse_variant(cursor.take_group(Delimiter::Brace)?)?);
        if !cursor.is_done() {
            cursor.expect_punct(',')?;
        }
    }
    Ok(variants)
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn parse_variant(input: TokenStream2) -> Result<Variant> {
    let mut cursor = Cursor::new(input);
    let name = parse_string_property(&mut cursor, "name")?;
    cursor.expect_punct(',')?;
    let owner_rule = parse_string_property(&mut cursor, "owner_rule")?;
    cursor.expect_punct(',')?;
    let source_rule = parse_string_property(&mut cursor, "source_rule")?;
    cursor.expect_punct(',')?;
    let docs = parse_docs(&mut cursor)?;
    cursor.expect_punct(',')?;
    let constructor = parse_constructor(&mut cursor)?;
    cursor.expect_punct(',')?;
    let shape = parse_shape(&mut cursor)?;
    cursor.expect_punct(',')?;
    let fields = parse_fields(&mut cursor)?;
    cursor.finish()?;
    validate_fields(&fields, shape)?;
    Ok(new!(Variant {
        name,
        owner_rule,
        source_rule,
        docs,
        constructor,
        shape,
        fields,
    }))
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn parse_fields(cursor: &mut Cursor) -> Result<Vec<Field>> {
    cursor.expect_ident("fields")?;
    let mut fields = Cursor::new(cursor.take_group(Delimiter::Bracket)?);
    let mut parsed = Vec::new();
    while !fields.is_done() {
        fields.expect_ident("field")?;
        parsed.push(parse_field(fields.take_group(Delimiter::Brace)?)?);
        if !fields.is_done() {
            fields.expect_punct(',')?;
        }
    }
    Ok(parsed)
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn parse_field(input: TokenStream2) -> Result<Field> {
    let mut cursor = Cursor::new(input);
    let source_name = parse_string_property(&mut cursor, "source_name")?;
    cursor.expect_punct(',')?;
    cursor.expect_ident("rust_name")?;
    let mut rust_name_group = Cursor::new(cursor.take_group(Delimiter::Parenthesis)?);
    let rust_name = match rust_name_group.take_ident()?.as_str() {
        "named" => new!(RustName::Named {
            name: parse_string_group(&mut rust_name_group)?,
        }),
        "tuple" => new!(RustName::Tuple {
            index: parse_usize_group(&mut rust_name_group)?,
        }),
        other => {
            return Err(Error::new(
                proc_macro2::Span::call_site(),
                format!("unknown Rust field-name shape `{other}`"),
            ));
        }
    };
    rust_name_group.finish()?;
    cursor.expect_punct(',')?;
    let index = parse_usize_property(&mut cursor, "index")?;
    cursor.expect_punct(',')?;
    let docs = parse_docs(&mut cursor)?;
    cursor.expect_punct(',')?;
    let strict = parse_type_property(&mut cursor, "strict")?;
    cursor.expect_punct(',')?;
    let recovered = parse_type_property(&mut cursor, "recovered")?;
    cursor.finish()?;
    Ok(new!(Field {
        source_name,
        rust_name,
        index,
        docs,
        strict,
        recovered,
    }))
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn validate_fields(fields: &[Field], shape: Shape) -> Result<()> {
    if (shape == Shape::Tuple) != (fields.len() == 1) {
        return Err(Error::new(
            proc_macro2::Span::call_site(),
            "schema tuple shapes must contain exactly one field; named shapes must be unit or contain multiple fields",
        ));
    }
    for (index, field) in fields.iter().enumerate() {
        if field.index != index {
            return Err(Error::new(
                proc_macro2::Span::call_site(),
                format!(
                    "schema field `{}` has index {}, expected {index}",
                    field.source_name, field.index
                ),
            ));
        }
        match field.rust_name.as_data() {
            data!(RustName::Named { name })
                if shape == Shape::Named && name == &field.source_name => {}
            data!(RustName::Tuple { index: rust_index })
                if shape == Shape::Tuple && *rust_index == index => {}
            _ => {
                return Err(Error::new(
                    proc_macro2::Span::call_site(),
                    format!(
                        "schema field `{}` has inconsistent Rust naming metadata",
                        field.source_name
                    ),
                ));
            }
        }
    }
    Ok(())
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn parse_docs(cursor: &mut Cursor) -> Result<Vec<String>> {
    cursor.expect_ident("docs")?;
    let mut docs = Cursor::new(cursor.take_group(Delimiter::Bracket)?);
    let mut values = Vec::new();
    while !docs.is_done() {
        values.push(docs.take_lit_string()?.value());
        if !docs.is_done() {
            docs.expect_punct(',')?;
        }
    }
    if values.iter().any(|line| !line.trim().is_empty()) {
        Ok(values)
    } else {
        Err(Error::new(
            proc_macro2::Span::call_site(),
            "schema documentation must contain non-blank text",
        ))
    }
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn parse_type_property(cursor: &mut Cursor, name: &str) -> Result<BindingType> {
    cursor.expect_ident(name)?;
    parse_binding_type(cursor.take_group(Delimiter::Parenthesis)?)
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn parse_binding_type(input: TokenStream2) -> Result<BindingType> {
    let mut cursor = Cursor::new(input);
    let kind = cursor.take_ident()?;
    let args = cursor.take_group(Delimiter::Parenthesis)?;
    cursor.finish()?;
    match kind.as_str() {
        "reference" => parse_reference(args),
        "optional" => parse_unary_type(args, |value| new!(BindingType::Optional { value })),
        "repeated" => parse_unary_type(args, |value| new!(BindingType::Repeated { value })),
        "non_empty_repeated" => {
            parse_unary_type(args, |value| new!(BindingType::NonEmptyRepeated { value }))
        }
        "boxed" => parse_unary_type(args, |value| new!(BindingType::Boxed { value })),
        "shared" => parse_unary_type(args, |value| new!(BindingType::Shared { value })),
        "recovered_field" => {
            parse_unary_type(args, |value| new!(BindingType::RecoveredField { value }))
        }
        "with_indicators" => {
            parse_unary_type(args, |value| new!(BindingType::WithIndicators { value }))
        }
        "with_free_modifiers" => parse_with_free_modifiers(args),
        "chain" => parse_chain(args),
        "tuple" => Ok(new!(BindingType::Tuple {
            elements: parse_type_list(args)?,
        })),
        "fixed" => parse_fixed(args),
        other => Err(Error::new(
            proc_macro2::Span::call_site(),
            format!("unknown normalized binding wrapper `{other}`"),
        )),
    }
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn parse_unary_type(
    input: TokenStream2,
    constructor: fn(Box<BindingType>) -> BindingType,
) -> Result<BindingType> {
    Ok(constructor(Box::new(parse_binding_type(input)?)))
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn parse_reference(input: TokenStream2) -> Result<BindingType> {
    let mut cursor = Cursor::new(input);
    let kind = cursor.take_ident()?;
    let args = cursor.take_group(Delimiter::Parenthesis)?;
    cursor.finish()?;
    match kind.as_str() {
        "model" => {
            let mut args = Cursor::new(args);
            let name = args.take_string()?;
            args.finish()?;
            Ok(new!(BindingType::ModelReference { name }))
        }
        "leaf" => parse_leaf(args),
        other => Err(Error::new(
            proc_macro2::Span::call_site(),
            format!("unknown binding reference kind `{other}`"),
        )),
    }
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn parse_leaf(input: TokenStream2) -> Result<BindingType> {
    let mut cursor = Cursor::new(input);
    cursor.expect_ident("kind")?;
    let mut kind = Cursor::new(cursor.take_group(Delimiter::Parenthesis)?);
    let kind_name = kind.take_ident()?;
    kind.finish()?;
    let kind = match kind_name.as_str() {
        "syntax_token" => LeafKind::SyntaxToken,
        "morphology_cmavo" => LeafKind::MorphologyCmavo,
        "morphology_selmaho" => LeafKind::MorphologySelmaho,
        "morphology_word" => LeafKind::MorphologyWord,
        "morphology_word_like" => LeafKind::MorphologyWordLike,
        "source_id" => LeafKind::SourceId,
        "source_span" => LeafKind::SourceSpan,
        "boolean" => LeafKind::Boolean,
        "integer" => LeafKind::Integer,
        "string" => LeafKind::String,
        "external" => LeafKind::External,
        other => {
            return Err(Error::new(
                proc_macro2::Span::call_site(),
                format!("unknown schema leaf kind `{other}`"),
            ));
        }
    };
    cursor.expect_punct(',')?;
    let absolute = parse_bool_property(&mut cursor, "absolute")?;
    cursor.expect_punct(',')?;
    cursor.expect_ident("path")?;
    let path = parse_string_list(cursor.take_group(Delimiter::Parenthesis)?)?;
    cursor.finish()?;
    validate_leaf(kind, absolute, &path)?;
    Ok(new!(BindingType::LeafReference {
        kind,
        absolute,
        path,
    }))
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn validate_leaf(kind: LeafKind, absolute: bool, path: &[String]) -> Result<()> {
    if path.is_empty() || path.iter().any(String::is_empty) {
        return Err(Error::new(
            proc_macro2::Span::call_site(),
            "schema leaf path must contain non-empty components",
        ));
    }
    if kind == LeafKind::External {
        return Err(Error::new(
            proc_macro2::Span::call_site(),
            format!("unsupported external schema leaf path {path:?} (absolute={absolute})"),
        ));
    }
    let valid_path = match kind {
        LeafKind::SyntaxToken => path_matches_any(
            path,
            &[&["Token"], &["crate", "Token"], &["jbotci_syntax", "Token"]],
        ),
        LeafKind::MorphologyCmavo => {
            path_matches_any(path, &[&["Cmavo"], &["jbotci_morphology", "Cmavo"]])
        }
        LeafKind::MorphologySelmaho => {
            path_matches_any(path, &[&["Selmaho"], &["jbotci_morphology", "Selmaho"]])
        }
        LeafKind::MorphologyWord => {
            path_matches_any(path, &[&["Word"], &["jbotci_morphology", "Word"]])
        }
        LeafKind::MorphologyWordLike => {
            path_matches_any(path, &[&["WordLike"], &["jbotci_morphology", "WordLike"]])
        }
        LeafKind::SourceId => {
            path_matches_any(path, &[&["SourceId"], &["jbotci_source", "SourceId"]])
        }
        LeafKind::SourceSpan => {
            path_matches_any(path, &[&["SourceSpan"], &["jbotci_source", "SourceSpan"]])
        }
        LeafKind::Boolean => primitive_path_matches(path, &["bool"]),
        LeafKind::Integer => is_integer_path(path),
        LeafKind::String => path_matches_any(
            path,
            &[
                &["String"],
                &["alloc", "string", "String"],
                &["std", "string", "String"],
            ],
        ),
        LeafKind::External => unreachable!("external leaves return before path validation"),
    };
    let absolute_is_valid = if path.len() == 1 || path.first().is_some_and(|part| part == "crate") {
        !absolute
    } else {
        true
    };
    if !valid_path || !absolute_is_valid {
        return Err(Error::new(
            proc_macro2::Span::call_site(),
            format!(
                "schema leaf kind {kind:?} has unsupported path {path:?} (absolute={absolute})"
            ),
        ));
    }
    Ok(())
}

#[requires(candidates.iter().all(|candidate| !candidate.is_empty()))]
#[ensures(true)]
fn path_matches_any(path: &[String], candidates: &[&[&str]]) -> bool {
    candidates.iter().any(|candidate| {
        path.iter()
            .map(String::as_str)
            .eq(candidate.iter().copied())
    })
}

#[requires(!names.is_empty())]
#[ensures(true)]
fn primitive_path_matches(path: &[String], names: &[&str]) -> bool {
    match path {
        [name] => names.contains(&name.as_str()),
        [root, primitive, name]
            if (root == "core" || root == "std") && primitive == "primitive" =>
        {
            names.contains(&name.as_str())
        }
        _ => false,
    }
}

#[requires(!path.is_empty())]
#[ensures(true)]
fn is_integer_path(path: &[String]) -> bool {
    primitive_path_matches(
        path,
        &[
            "i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16", "u32", "u64", "u128", "usize",
        ],
    )
}

#[requires(is_integer_path(path))]
#[ensures(true)]
fn integer_rust_type(path: &[String]) -> TokenStream2 {
    let name = path
        .last()
        .expect("the integer path precondition requires one component");
    let ident = format_ident!("{name}");
    quote!(#ident)
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn parse_with_free_modifiers(input: TokenStream2) -> Result<BindingType> {
    let mut cursor = Cursor::new(input);
    let value = Box::new(parse_type_property(&mut cursor, "value")?);
    cursor.expect_punct(',')?;
    let name = cursor.take_ident()?;
    if name != "free_modifier" && name != "free_modifiers" {
        return Err(Error::new(
            proc_macro2::Span::call_site(),
            format!("unknown with-free-modifiers property `{name}`"),
        ));
    }
    let free_modifiers = Box::new(parse_binding_type(
        cursor.take_group(Delimiter::Parenthesis)?,
    )?);
    cursor.finish()?;
    Ok(new!(BindingType::WithFreeModifiers {
        value,
        free_modifiers,
    }))
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn parse_chain(input: TokenStream2) -> Result<BindingType> {
    let mut cursor = Cursor::new(input);
    let first = Box::new(parse_type_property(&mut cursor, "first")?);
    cursor.expect_punct(',')?;
    let links = Box::new(parse_type_property(&mut cursor, "links")?);
    cursor.finish()?;
    Ok(new!(BindingType::Chain { first, links }))
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn parse_fixed(input: TokenStream2) -> Result<BindingType> {
    let mut cursor = Cursor::new(input);
    let length = parse_usize_property(&mut cursor, "length")?;
    if length == 0 {
        return Err(Error::new(
            proc_macro2::Span::call_site(),
            "fixed schema arrays must have positive length",
        ));
    }
    cursor.expect_punct(',')?;
    let value = Box::new(parse_type_property(&mut cursor, "value")?);
    cursor.finish()?;
    Ok(new!(BindingType::Fixed { length, value }))
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn parse_type_list(input: TokenStream2) -> Result<Vec<BindingType>> {
    let mut cursor = Cursor::new(input);
    let mut values = Vec::new();
    while !cursor.is_done() {
        let kind = cursor.take_ident()?;
        let args = cursor.take_group(Delimiter::Parenthesis)?;
        let mut expression = TokenStream2::new();
        expression.extend([
            TokenTree::Ident(Ident::new(&kind, proc_macro2::Span::call_site())),
            TokenTree::Group(Group::new(Delimiter::Parenthesis, args)),
        ]);
        values.push(parse_binding_type(expression)?);
        if !cursor.is_done() {
            cursor.expect_punct(',')?;
        }
    }
    Ok(values)
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn parse_metadata(input: TokenStream2) -> Result<Metadata> {
    let mut cursor = Cursor::new(input);
    let transparent_constructors =
        parse_string_list_property(&mut cursor, "transparent_constructors")?;
    cursor.expect_punct(',')?;
    let transparent_fields =
        parse_pair_list_property(&mut cursor, "transparent_fields", "transparent_field")?;
    cursor.expect_punct(',')?;
    let chain_link_element_fields = parse_pair_list_property(
        &mut cursor,
        "chain_link_element_fields",
        "chain_link_element_field",
    )?;
    cursor.expect_punct(',')?;
    let constructor_labels =
        parse_pair_list_property(&mut cursor, "constructor_labels", "constructor_label")?;
    cursor.expect_punct(',')?;
    let elidable_terminators =
        parse_pair_list_property(&mut cursor, "elidable_terminators", "elidable_terminator")?;
    cursor.expect_punct(',')?;
    cursor.expect_ident("field_orders")?;
    let field_orders = parse_field_orders(cursor.take_group(Delimiter::Bracket)?)?;
    cursor.finish()?;
    Ok(Metadata {
        transparent_constructors,
        transparent_fields,
        chain_link_element_fields,
        constructor_labels,
        elidable_terminators,
        field_orders,
    })
}

#[requires(!name.is_empty())]
#[ensures(ret.is_ok() || ret.is_err())]
fn parse_string_list_property(cursor: &mut Cursor, name: &str) -> Result<Vec<String>> {
    cursor.expect_ident(name)?;
    parse_string_list(cursor.take_group(Delimiter::Bracket)?)
}

#[requires(!name.is_empty() && !call.is_empty())]
#[ensures(ret.is_ok() || ret.is_err())]
fn parse_pair_list_property(
    cursor: &mut Cursor,
    name: &str,
    call: &str,
) -> Result<Vec<(String, String)>> {
    cursor.expect_ident(name)?;
    let mut values = Cursor::new(cursor.take_group(Delimiter::Bracket)?);
    let mut pairs = Vec::new();
    while !values.is_done() {
        values.expect_ident(call)?;
        let mut pair = Cursor::new(values.take_group(Delimiter::Parenthesis)?);
        let first = pair.take_string()?;
        pair.expect_punct(',')?;
        let second = pair.take_string()?;
        pair.finish()?;
        pairs.push((first, second));
        if !values.is_done() {
            values.expect_punct(',')?;
        }
    }
    Ok(pairs)
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn parse_field_orders(input: TokenStream2) -> Result<Vec<(String, Vec<String>)>> {
    let mut cursor = Cursor::new(input);
    let mut values = Vec::new();
    while !cursor.is_done() {
        cursor.expect_ident("field_order")?;
        let mut value = Cursor::new(cursor.take_group(Delimiter::Parenthesis)?);
        let constructor = value.take_string()?;
        value.expect_punct(',')?;
        let fields = parse_string_list(value.take_group(Delimiter::Bracket)?)?;
        value.finish()?;
        values.push((constructor, fields));
        if !cursor.is_done() {
            cursor.expect_punct(',')?;
        }
    }
    Ok(values)
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn parse_string_list(input: TokenStream2) -> Result<Vec<String>> {
    let mut cursor = Cursor::new(input);
    let mut values = Vec::new();
    while !cursor.is_done() {
        values.push(cursor.take_string()?);
        if !cursor.is_done() {
            cursor.expect_punct(',')?;
        }
    }
    Ok(values)
}

#[requires(!name.is_empty())]
#[ensures(ret.as_ref().is_ok_and(|value| !value.is_empty()) || ret.is_err())]
fn parse_string_property(cursor: &mut Cursor, name: &str) -> Result<String> {
    cursor.expect_ident(name)?;
    parse_string_group(cursor)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|value| !value.is_empty()) || ret.is_err())]
fn parse_string_group(cursor: &mut Cursor) -> Result<String> {
    let mut value = Cursor::new(cursor.take_group(Delimiter::Parenthesis)?);
    let parsed = value.take_string()?;
    value.finish()?;
    Ok(parsed)
}

#[requires(!name.is_empty())]
#[ensures(ret.is_ok() || ret.is_err())]
fn parse_usize_property(cursor: &mut Cursor, name: &str) -> Result<usize> {
    cursor.expect_ident(name)?;
    parse_usize_group(cursor)
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn parse_usize_group(cursor: &mut Cursor) -> Result<usize> {
    let mut value = Cursor::new(cursor.take_group(Delimiter::Parenthesis)?);
    let parsed = value.take_usize()?;
    value.finish()?;
    Ok(parsed)
}

#[requires(!name.is_empty())]
#[ensures(ret.is_ok() || ret.is_err())]
fn parse_bool_property(cursor: &mut Cursor, name: &str) -> Result<bool> {
    cursor.expect_ident(name)?;
    let mut value = Cursor::new(cursor.take_group(Delimiter::Parenthesis)?);
    let parsed = value.take_bool()?;
    value.finish()?;
    Ok(parsed)
}

#[requires(!models.is_empty())]
#[ensures(ret.is_ok() || ret.is_err())]
fn validate_model_references(models: &[Model]) -> Result<()> {
    let strict = models
        .iter()
        .map(|model| model.strict_name().to_owned())
        .collect::<BTreeSet<_>>();
    let recovered = models
        .iter()
        .map(|model| model.recovered_name().to_owned())
        .collect::<BTreeSet<_>>();
    if strict.len() != models.len() || recovered.len() != models.len() {
        return Err(Error::new(
            proc_macro2::Span::call_site(),
            "syntax schema contains duplicate model names",
        ));
    }
    for name in strict.iter().chain(&recovered) {
        validate_python_identifier(name, "model name")?;
    }
    for model in models {
        match model.as_data() {
            data!(Model::Product { fields, .. }) => {
                validate_python_identifier(model.strict_name(), "strict model name")?;
                validate_python_identifier(model.recovered_name(), "recovered model name")?;
                validate_field_names(fields)?;
                validate_field_references(fields, &strict, &recovered)?;
            }
            data!(Model::Sum { common, variants }) => {
                let mut variant_names = BTreeSet::new();
                for variant in variants {
                    if variant.owner_rule != common.rule {
                        return Err(Error::new(
                            proc_macro2::Span::call_site(),
                            format!(
                                "schema variant `{}` names owner rule `{}` instead of `{}`",
                                variant.name, variant.owner_rule, common.rule,
                            ),
                        ));
                    }
                    validate_python_identifier(&variant.name, "variant name")?;
                    if !variant_names.insert(variant.name.as_str()) {
                        return Err(Error::new(
                            proc_macro2::Span::call_site(),
                            format!(
                                "syntax schema model `{}` contains duplicate variant `{}`",
                                model.strict_name(),
                                variant.name,
                            ),
                        ));
                    }
                    validate_field_names(&variant.fields)?;
                    validate_field_references(&variant.fields, &strict, &recovered)?;
                }
            }
        }
    }
    validate_namespace_inventory(models, ProjectionMode::Strict)?;
    validate_namespace_inventory(models, ProjectionMode::Recovered)?;
    Ok(())
}

#[requires(!models.is_empty())]
#[ensures(ret.is_ok() || ret.is_err())]
fn validate_namespace_inventory(models: &[Model], mode: ProjectionMode) -> Result<()> {
    let mut names = BTreeSet::new();
    for model in models {
        let owner = mode_model_name(model, mode);
        validate_generated_class_name(owner)?;
        if !names.insert(owner.to_owned()) {
            return Err(Error::new(
                proc_macro2::Span::call_site(),
                format!("duplicate generated Python syntax name `{owner}`"),
            ));
        }
        if let data!(Model::Sum { variants, .. }) = model.as_data() {
            for variant in variants {
                let concrete = variant_class_name(owner, &variant.name);
                validate_generated_class_name(&concrete)?;
                if !names.insert(concrete.clone()) {
                    return Err(Error::new(
                        proc_macro2::Span::call_site(),
                        format!("duplicate generated Python syntax name `{concrete}`"),
                    ));
                }
            }
        }
    }
    Ok(())
}

#[requires(!value.is_empty())]
#[ensures(ret.is_ok() || ret.is_err())]
fn validate_generated_class_name(value: &str) -> Result<()> {
    const RESERVED: &[&str] = &[
        "Chain",
        "ClassVar",
        "Cmavo",
        "Literal",
        "RecoveredField",
        "Sequence",
        "Selmaho",
        "SourceId",
        "SourceSpan",
        "Token",
        "TypeAlias",
        "WithFreeModifiers",
        "WithIndicators",
        "Word",
        "WordLike",
        "_SyntaxNode",
        "cast",
        "final",
    ];
    if RESERVED.contains(&value) {
        Err(Error::new(
            proc_macro2::Span::call_site(),
            format!("generated Python syntax class name `{value}` is reserved"),
        ))
    } else {
        Ok(())
    }
}

#[requires(!description.is_empty())]
#[ensures(ret.is_ok() || ret.is_err())]
fn validate_python_identifier(value: &str, description: &str) -> Result<()> {
    let is_identifier = !value.is_empty()
        && !matches!(
            value,
            "False"
                | "None"
                | "True"
                | "and"
                | "as"
                | "assert"
                | "async"
                | "await"
                | "break"
                | "class"
                | "continue"
                | "def"
                | "del"
                | "elif"
                | "else"
                | "except"
                | "finally"
                | "for"
                | "from"
                | "global"
                | "if"
                | "import"
                | "in"
                | "is"
                | "lambda"
                | "nonlocal"
                | "not"
                | "or"
                | "pass"
                | "raise"
                | "return"
                | "try"
                | "while"
                | "with"
                | "yield"
        )
        && value.chars().enumerate().all(|(index, character)| {
            character == '_'
                || character.is_ascii_alphanumeric() && (index > 0 || !character.is_ascii_digit())
        });
    if is_identifier {
        Ok(())
    } else {
        Err(Error::new(
            proc_macro2::Span::call_site(),
            format!("schema {description} `{value}` is not a supported Python identifier"),
        ))
    }
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn validate_field_names(fields: &[Field]) -> Result<()> {
    const RESERVED: &[&str] = &[
        "__hash__",
        "__init_subclass__",
        "__match_args__",
        "__repr__",
        "_debug_projection_count",
        "_field",
        "_from_fields",
        "_from_native",
        "_native",
        "_schema_id",
        "same_identity",
    ];
    let mut names = BTreeSet::new();
    for field in fields {
        validate_python_identifier(&field.source_name, "field name")?;
        if RESERVED.contains(&field.source_name.as_str()) {
            return Err(Error::new(
                proc_macro2::Span::call_site(),
                format!(
                    "schema field name `{}` conflicts with generated Python behavior",
                    field.source_name
                ),
            ));
        }
        if !names.insert(field.source_name.as_str()) {
            return Err(Error::new(
                proc_macro2::Span::call_site(),
                format!("duplicate schema field name `{}`", field.source_name),
            ));
        }
    }
    Ok(())
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn validate_field_references(
    fields: &[Field],
    strict: &BTreeSet<String>,
    recovered: &BTreeSet<String>,
) -> Result<()> {
    for field in fields {
        validate_type_references(&field.strict, strict)?;
        validate_type_references(&field.recovered, recovered)?;
    }
    Ok(())
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn validate_type_references(value: &BindingType, models: &BTreeSet<String>) -> Result<()> {
    match value.as_data() {
        data!(BindingType::ModelReference { name }) => {
            if !models.contains(name) {
                return Err(Error::new(
                    proc_macro2::Span::call_site(),
                    format!("syntax schema references unknown model `{name}`"),
                ));
            }
        }
        data!(BindingType::LeafReference { .. }) => {}
        data!(BindingType::Optional { value })
        | data!(BindingType::Repeated { value })
        | data!(BindingType::NonEmptyRepeated { value })
        | data!(BindingType::Boxed { value })
        | data!(BindingType::Shared { value })
        | data!(BindingType::RecoveredField { value }) => {
            validate_type_references(value, models)?;
        }
        data!(BindingType::WithIndicators { value }) => {
            if !matches!(
                value.as_data(),
                data!(BindingType::LeafReference {
                    kind: LeafKind::MorphologyWordLike,
                    ..
                })
            ) {
                return Err(Error::new(
                    proc_macro2::Span::call_site(),
                    "syntax bindings support WithIndicators only for the canonical WordLike leaf",
                ));
            }
            validate_type_references(value, models)?;
        }
        data!(BindingType::WithFreeModifiers {
            value,
            free_modifiers,
        }) => {
            validate_type_references(value, models)?;
            validate_type_references(free_modifiers, models)?;
        }
        data!(BindingType::Chain { first, links }) => {
            if !matches!(
                links.as_data(),
                data!(BindingType::Repeated { .. }) | data!(BindingType::NonEmptyRepeated { .. })
            ) {
                return Err(Error::new(
                    proc_macro2::Span::call_site(),
                    "syntax binding Chain links must use a supported repeated wrapper",
                ));
            }
            validate_type_references(first, models)?;
            validate_type_references(links, models)?;
        }
        data!(BindingType::Tuple { elements }) => {
            for element in elements {
                validate_type_references(element, models)?;
            }
        }
        data!(BindingType::Fixed { value, .. }) => validate_type_references(value, models)?,
    }
    Ok(())
}

impl BindingType {
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn annotation(&self, input: bool) -> String {
        match self.as_data() {
            data!(BindingType::ModelReference { name }) => name.clone(),
            data!(BindingType::LeafReference { kind, .. }) => match kind {
                LeafKind::SyntaxToken => "Token".to_owned(),
                LeafKind::MorphologyCmavo => "Cmavo".to_owned(),
                LeafKind::MorphologySelmaho => "Selmaho".to_owned(),
                LeafKind::MorphologyWord => "Word".to_owned(),
                LeafKind::MorphologyWordLike => "WordLike".to_owned(),
                LeafKind::SourceId => "SourceId".to_owned(),
                LeafKind::SourceSpan => "SourceSpan".to_owned(),
                LeafKind::Boolean => "bool".to_owned(),
                LeafKind::Integer => "int".to_owned(),
                LeafKind::String => "str".to_owned(),
                LeafKind::External => unreachable!("external leaves are rejected while parsing"),
            },
            data!(BindingType::Optional { value }) => {
                format!("{} | None", value.annotation(input))
            }
            data!(BindingType::Repeated { value })
            | data!(BindingType::NonEmptyRepeated { value }) => {
                let value = value.annotation(input);
                if input {
                    format!("Sequence[{value}]")
                } else {
                    format!("tuple[{value}, ...]")
                }
            }
            data!(BindingType::Boxed { value }) | data!(BindingType::Shared { value }) => {
                value.annotation(input)
            }
            data!(BindingType::RecoveredField { value }) => {
                format!("RecoveredField[{}]", value.annotation(input))
            }
            data!(BindingType::WithIndicators { .. }) => "WithIndicators".to_owned(),
            data!(BindingType::WithFreeModifiers {
                value,
                free_modifiers,
            }) => format!(
                "WithFreeModifiers[{}, {}]",
                value.annotation(input),
                sequence_element_annotation(free_modifiers, input)
            ),
            data!(BindingType::Chain { first, links }) => format!(
                "Chain[{}, {}]",
                first.annotation(input),
                sequence_element_annotation(links, input)
            ),
            data!(BindingType::Tuple { elements }) => {
                tuple_annotation(elements.iter().map(|element| element.annotation(input)))
            }
            data!(BindingType::Fixed { length, value }) => {
                tuple_annotation((0..*length).map(|_| value.annotation(input)))
            }
        }
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn sequence_element_annotation(value: &BindingType, input: bool) -> String {
    match value.as_data() {
        data!(BindingType::Repeated { value }) | data!(BindingType::NonEmptyRepeated { value }) => {
            value.annotation(input)
        }
        _ => value.annotation(input),
    }
}

#[requires(true)]
#[ensures(ret.starts_with("tuple[") && ret.ends_with(']'))]
fn tuple_annotation(values: impl IntoIterator<Item = String>) -> String {
    let values = values.into_iter().collect::<Vec<_>>();
    if values.is_empty() {
        "tuple[()]".to_owned()
    } else {
        format!("tuple[{}]", values.join(", "))
    }
}

#[requires(true)]
#[ensures(ret.starts_with('\'') && ret.ends_with('\''))]
fn python_string(value: &str) -> String {
    let mut output = String::with_capacity(value.len() + 2);
    output.push('\'');
    for character in value.chars() {
        match character {
            '\\' => output.push_str("\\\\"),
            '\'' => output.push_str("\\'"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            character if character.is_control() => {
                write!(output, "\\u{:04x}", character as u32)
                    .expect("writing to String cannot fail");
            }
            character => output.push(character),
        }
    }
    output.push('\'');
    output
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn documentation(lines: &[String]) -> String {
    lines
        .iter()
        .map(|line| line.trim())
        .collect::<Vec<_>>()
        .join("\n")
}

#[requires(lines.iter().any(|line| !line.trim().is_empty()))]
#[ensures(ret.split('\n').all(|line| line == "#" || line.starts_with("# ")))]
fn commented_documentation(lines: &[String]) -> String {
    documentation(lines)
        .split('\n')
        .map(|line| {
            if line.is_empty() {
                "#".to_owned()
            } else {
                format!("# {line}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[requires(!owner.is_empty() && !variant.is_empty())]
#[ensures(ret.starts_with(owner) && ret.ends_with(variant))]
fn variant_class_name(owner: &str, variant: &str) -> String {
    format!("{owner}{variant}")
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn mode_model_name(model: &Model, mode: ProjectionMode) -> &str {
    match mode {
        ProjectionMode::Strict => model.strict_name(),
        ProjectionMode::Recovered => model.recovered_name(),
    }
}

#[requires(true)]
#[ensures(true)]
fn mode_fields<'a>(field: &'a Field, mode: ProjectionMode) -> &'a BindingType {
    match mode {
        ProjectionMode::Strict => &field.strict,
        ProjectionMode::Recovered => &field.recovered,
    }
}

#[requires(!class_name.is_empty())]
#[ensures(true)]
fn render_runtime_class(
    output: &mut String,
    class_name: &str,
    class_id: usize,
    docs: &[String],
    fields: &[Field],
    mode: ProjectionMode,
) {
    writeln!(output, "@final\nclass {class_name}(_SyntaxNode):")
        .expect("writing to String cannot fail");
    writeln!(output, "    {}", python_string(&documentation(docs)))
        .expect("writing to String cannot fail");
    writeln!(output, "    __slots__ = ()\n    _schema_id = {class_id}")
        .expect("writing to String cannot fail");
    let match_args = fields
        .iter()
        .map(|field| python_string(&field.source_name))
        .collect::<Vec<_>>()
        .join(", ");
    let tuple_comma = if fields.len() == 1 { "," } else { "" };
    writeln!(output, "    __match_args__ = ({match_args}{tuple_comma})")
        .expect("writing to String cannot fail");
    let parameters = fields
        .iter()
        .map(|field| {
            format!(
                "{}: {}",
                field.source_name,
                mode_fields(field, mode).annotation(true)
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    let values = fields
        .iter()
        .map(|field| field.source_name.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    let values_comma = if fields.len() == 1 { "," } else { "" };
    let separator = if fields.is_empty() { "" } else { ", " };
    writeln!(
        output,
        "    def __new__(cls{separator}{parameters}) -> {class_name}:\n        return cls._from_fields(({values}{values_comma}))"
    )
    .expect("writing to String cannot fail");
    writeln!(
        output,
        "    def __init__(self{separator}{parameters}) -> None:\n        pass"
    )
    .expect("writing to String cannot fail");
    for field in fields {
        let annotation = mode_fields(field, mode).annotation(false);
        let docs = python_string(&documentation(&field.docs));
        writeln!(
            output,
            "    @property\n    def {}(self) -> {}:\n        {}\n        return cast({}, self._field({}))",
            field.source_name, annotation, docs, annotation, field.index
        )
        .expect("writing to String cannot fail");
    }
    writeln!(
        output,
        "    def __init_subclass__(cls) -> None:\n        raise TypeError('{} is final')\n",
        class_name
    )
    .expect("writing to String cannot fail");
}

#[requires(!class_name.is_empty())]
#[ensures(true)]
fn render_stub_class(
    output: &mut String,
    class_name: &str,
    docs: &[String],
    fields: &[Field],
    mode: ProjectionMode,
) {
    writeln!(output, "@final\nclass {class_name}:").expect("writing to String cannot fail");
    writeln!(output, "    {}", python_string(&documentation(docs)))
        .expect("writing to String cannot fail");
    let match_args = fields
        .iter()
        .map(|field| format!("Literal[{}]", python_string(&field.source_name)))
        .collect::<Vec<_>>()
        .join(", ");
    let match_args = if match_args.is_empty() {
        "tuple[()]".to_owned()
    } else {
        format!("tuple[{match_args}]")
    };
    writeln!(output, "    __match_args__: ClassVar[{match_args}]")
        .expect("writing to String cannot fail");
    if fields.len() <= 2 {
        let parameters = fields
            .iter()
            .map(|field| {
                format!(
                    "{}: {}",
                    field.source_name,
                    mode_fields(field, mode).annotation(true)
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        let separator = if fields.is_empty() { "" } else { ", " };
        writeln!(
            output,
            "    def __new__(cls{separator}{parameters}) -> {class_name}: ..."
        )
        .expect("writing to String cannot fail");
    } else {
        writeln!(output, "    def __new__(\n        cls,").expect("writing to String cannot fail");
        for field in fields {
            writeln!(
                output,
                "        {}: {},",
                field.source_name,
                mode_fields(field, mode).annotation(true)
            )
            .expect("writing to String cannot fail");
        }
        writeln!(output, "    ) -> {class_name}: ...").expect("writing to String cannot fail");
    }
    for field in fields {
        writeln!(
            output,
            "    @property\n    def {}(self) -> {}:\n        {}\n        ...",
            field.source_name,
            mode_fields(field, mode).annotation(false),
            python_string(&documentation(&field.docs))
        )
        .expect("writing to String cannot fail");
    }
    // `__hash__: ClassVar[None]` is the typeshed idiom for unhashable classes;
    // the ignore is required because it overrides `object.__hash__`.
    output.push_str(
        "    __hash__: ClassVar[None]  # type: ignore[assignment]\n    def same_identity(self, other: object, /) -> bool: ...\n    def __repr__(self, /) -> str: ...\n    def __eq__(self, other: object, /) -> bool: ...\n\n",
    );
}

#[invariant(!runtime.is_empty() && !stub.is_empty() && !inventory.is_empty())]
#[invariant(!concrete_inventory.is_empty())]
struct RenderedNamespace {
    runtime: String,
    stub: String,
    inventory: Vec<String>,
    concrete_inventory: Vec<String>,
}

#[requires(!schema.models.is_empty())]
#[ensures(!ret.runtime.is_empty() && !ret.stub.is_empty() && !ret.inventory.is_empty())]
fn render_namespace(schema: &Schema, mode: ProjectionMode) -> RenderedNamespace {
    let module_doc = format!(
        "Generated {} syntax model. Exhaustive variant checking relies on the packaged type hints and a type checker.",
        mode.mode_name()
    );
    let mut runtime = format!(
        "# Generated from the canonical syntax binding schema; do not edit.\n{}\nfrom __future__ import annotations\n\nfrom collections.abc import Sequence\nfrom typing import TypeAlias, cast, final\n\nfrom jbotci.morphology import Cmavo, Selmaho, Word, WordLike\nfrom jbotci.source import SourceId, SourceSpan\nfrom jbotci.syntax import Chain, RecoveredField, Token, WithFreeModifiers, WithIndicators\nfrom jbotci.syntax._runtime import _SyntaxNode\n\n",
        python_string(&module_doc),
    );
    let mut stub = format!(
        "# Generated from the canonical syntax binding schema; do not edit.\n{}\nfrom collections.abc import Sequence\nfrom typing import ClassVar, Literal, TypeAlias, final\n\nfrom jbotci.morphology import Cmavo, Selmaho, Word, WordLike\nfrom jbotci.source import SourceId, SourceSpan\nfrom jbotci.syntax import Chain, RecoveredField, Token, WithFreeModifiers, WithIndicators\n\n",
        python_string(&module_doc)
    );
    let mut inventory = Vec::new();
    let mut concrete_inventory = Vec::new();
    let mut class_id = 0usize;
    for model in &schema.models {
        let owner = mode_model_name(model, mode);
        match model.as_data() {
            data!(Model::Product { common, fields, .. }) => {
                render_runtime_class(&mut runtime, owner, class_id, &common.docs, fields, mode);
                render_stub_class(&mut stub, owner, &common.docs, fields, mode);
                inventory.push(owner.to_owned());
                concrete_inventory.push(owner.to_owned());
                class_id += 1;
            }
            data!(Model::Sum { common, variants }) => {
                let mut variant_names = Vec::with_capacity(variants.len());
                for variant in variants {
                    let name = variant_class_name(owner, &variant.name);
                    render_runtime_class(
                        &mut runtime,
                        &name,
                        class_id,
                        &variant.docs,
                        &variant.fields,
                        mode,
                    );
                    render_stub_class(&mut stub, &name, &variant.docs, &variant.fields, mode);
                    inventory.push(name.clone());
                    concrete_inventory.push(name.clone());
                    variant_names.push(name);
                    class_id += 1;
                }
                let union = variant_names.join(" | ");
                writeln!(runtime, "{owner}: TypeAlias = {union}\n")
                    .expect("writing to String cannot fail");
                writeln!(
                    stub,
                    "{}\n{owner}: TypeAlias = {union}\n",
                    commented_documentation(&common.docs)
                )
                .expect("writing to String cannot fail");
                inventory.push(owner.to_owned());
            }
        }
    }
    let all = inventory
        .iter()
        .map(|name| python_string(name))
        .collect::<Vec<_>>()
        .join(",\n    ");
    let comma = if inventory.len() == 1 { "," } else { "" };
    write!(runtime, "__all__ = (\n    {all}{comma}\n)\n").expect("writing to String cannot fail");
    write!(stub, "__all__: tuple[str, ...]\n").expect("writing to String cannot fail");
    new!(RenderedNamespace {
        runtime,
        stub,
        inventory,
        concrete_inventory,
    })
}

#[requires(!schema.models.is_empty())]
#[ensures(true)]
fn expand_native_roots(schema: &Schema) -> TokenStream2 {
    let strict_root_invariants = schema.models.iter().map(|model| {
        let ident = format_ident!("{}", model.strict_name());
        quote!(#[bityzba::invariant(::#ident(..) => true)])
    });
    let recovered_root_invariants = schema.models.iter().map(|model| {
        let ident = format_ident!("{}", model.recovered_name());
        quote!(#[bityzba::invariant(::#ident(..) => true)])
    });
    let strict_root_variants = schema.models.iter().map(|model| {
        let ident = format_ident!("{}", model.strict_name());
        quote!(#ident(::std::sync::Arc<::jbotci_syntax::generated_model::#ident>))
    });
    let strict_model_invariants = schema.models.iter().map(|model| {
        let ident = format_ident!("{}", model.strict_name());
        quote!(#[bityzba::invariant(::#ident => true)])
    });
    let strict_model_variants = schema.models.iter().map(|model| {
        let ident = format_ident!("{}", model.strict_name());
        quote!(#ident)
    });
    let recovered_root_variants = schema.models.iter().map(|model| {
        let ident = format_ident!("{}", model.recovered_name());
        quote!(#ident(::std::sync::Arc<::jbotci_syntax::generated_model::recovered::#ident>))
    });
    let strict_node_arms = schema.models.iter().map(|model| {
        let ident = format_ident!("{}", model.strict_name());
        quote! {
            StrictSyntaxRoot::#ident(value) => {
                ::jbotci_syntax::generated_model::TreeNode::node_at_path(value.as_ref(), path)
            }
        }
    });
    let recovered_node_arms = schema.models.iter().map(|model| {
        let ident = format_ident!("{}", model.recovered_name());
        quote! {
            RecoveredSyntaxRoot::#ident(value) => {
                ::jbotci_syntax::generated_model::recovered::TreeNode::node_at_path(value.as_ref(), path)
            }
        }
    });
    let mut strict_class_arms = Vec::new();
    let mut strict_model_arms = Vec::new();
    let mut recovered_class_arms = Vec::new();
    let mut strict_equality_arms = Vec::new();
    let mut recovered_equality_arms = Vec::new();
    let mut class_id = 0usize;
    for model in &schema.models {
        let strict_ident = format_ident!("{}", model.strict_name());
        let recovered_ident = format_ident!("{}", model.recovered_name());
        match model.as_data() {
            data!(Model::Product { .. }) => {
                strict_class_arms.push(quote!(
                    ::jbotci_syntax::generated_model::NodeRef::#strict_ident(..) => #class_id
                ));
                recovered_class_arms.push(quote!(
                    ::jbotci_syntax::generated_model::recovered::NodeRef::#recovered_ident(..) => #class_id
                ));
                strict_equality_arms.push(quote!(
                    (
                        ::jbotci_syntax::generated_model::NodeRef::#strict_ident(left),
                        ::jbotci_syntax::generated_model::NodeRef::#strict_ident(right),
                    ) => left == right
                ));
                strict_model_arms.push(quote!(
                    ::jbotci_syntax::generated_model::NodeRef::#strict_ident(..) =>
                        StrictSyntaxModel::#strict_ident
                ));
                recovered_equality_arms.push(quote!(
                    (
                        ::jbotci_syntax::generated_model::recovered::NodeRef::#recovered_ident(left),
                        ::jbotci_syntax::generated_model::recovered::NodeRef::#recovered_ident(right),
                    ) => left == right
                ));
                class_id += 1;
            }
            data!(Model::Sum { variants, .. }) => {
                for variant in variants {
                    let strict_node = format_ident!("{}{}", model.strict_name(), variant.name);
                    let recovered_node =
                        format_ident!("{}{}", model.recovered_name(), variant.name);
                    strict_class_arms.push(quote!(
                        ::jbotci_syntax::generated_model::NodeRef::#strict_node(..) => #class_id
                    ));
                    recovered_class_arms.push(quote!(
                        ::jbotci_syntax::generated_model::recovered::NodeRef::#recovered_node(..) => #class_id
                    ));
                    strict_equality_arms.push(quote!(
                        (
                            ::jbotci_syntax::generated_model::NodeRef::#strict_node(left),
                            ::jbotci_syntax::generated_model::NodeRef::#strict_node(right),
                        ) => left == right
                    ));
                    strict_model_arms.push(quote!(
                        ::jbotci_syntax::generated_model::NodeRef::#strict_node(..) =>
                            StrictSyntaxModel::#strict_ident
                    ));
                    recovered_equality_arms.push(quote!(
                        (
                            ::jbotci_syntax::generated_model::recovered::NodeRef::#recovered_node(left),
                            ::jbotci_syntax::generated_model::recovered::NodeRef::#recovered_node(right),
                        ) => left == right
                    ));
                    class_id += 1;
                }
            }
        }
    }
    quote! {
        #(#strict_model_invariants)*
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub(crate) enum StrictSyntaxModel {
            #(#strict_model_variants,)*
        }

        #(#strict_root_invariants)*
        #[derive(Debug)]
        enum StrictSyntaxRoot {
            #(#strict_root_variants,)*
        }

        impl StrictSyntaxRoot {
            #[bityzba::requires(true)]
            #[bityzba::ensures(true)]
            fn node_at_path(
                &self,
                path: &::jbotci_tree::TreePath,
            ) -> Option<::jbotci_syntax::generated_model::NodeRef<'_>> {
                match self {
                    #(#strict_node_arms,)*
                }
            }
        }

        #(#recovered_root_invariants)*
        #[derive(Debug)]
        enum RecoveredSyntaxRoot {
            #(#recovered_root_variants,)*
        }

        impl RecoveredSyntaxRoot {
            #[bityzba::requires(true)]
            #[bityzba::ensures(true)]
            fn node_at_path(
                &self,
                path: &::jbotci_tree::TreePath,
            ) -> Option<::jbotci_syntax::generated_model::recovered::NodeRef<'_>> {
                match self {
                    #(#recovered_node_arms,)*
                }
            }
        }

        #[bityzba::invariant(::Strict { .. } => true)]
        #[bityzba::invariant(::Recovered { .. } => true)]
        #[derive(Debug)]
        enum SyntaxRoot {
            Strict { value: StrictSyntaxRoot },
            Recovered { value: RecoveredSyntaxRoot },
        }

        #[bityzba::invariant(true, "root models enforce their generated invariants")]
        #[derive(Debug)]
        pub(crate) struct SyntaxOwner {
            root: SyntaxRoot,
            projections: ::std::sync::atomic::AtomicUsize,
        }

        impl SyntaxOwner {
            #[bityzba::requires(true)]
            #[bityzba::ensures(true)]
            fn projection_count(&self) -> usize {
                self.projections.load(::std::sync::atomic::Ordering::Relaxed)
            }

            #[bityzba::requires(true)]
            #[bityzba::ensures(self.projection_count() >= old(self.projection_count()))]
            fn record_projection(&self) {
                self.projections
                    .fetch_update(
                        ::std::sync::atomic::Ordering::Relaxed,
                        ::std::sync::atomic::Ordering::Relaxed,
                        |value| Some(value.saturating_add(1)),
                    )
                    .expect("projection counter update cannot be rejected");
            }
        }

        #[bityzba::requires(true)]
        #[bityzba::ensures(true)]
        fn strict_class_id(
            node: ::jbotci_syntax::generated_model::NodeRef<'_>,
        ) -> usize {
            match node {
                #(#strict_class_arms,)*
            }
        }

        #[bityzba::requires(true)]
        #[bityzba::ensures(true)]
        fn strict_syntax_model(
            node: ::jbotci_syntax::generated_model::NodeRef<'_>,
        ) -> StrictSyntaxModel {
            match node {
                #(#strict_model_arms,)*
            }
        }

        #[bityzba::requires(true)]
        #[bityzba::ensures(true)]
        fn recovered_class_id(
            node: ::jbotci_syntax::generated_model::recovered::NodeRef<'_>,
        ) -> usize {
            match node {
                #(#recovered_class_arms,)*
            }
        }

        #[bityzba::requires(true)]
        #[bityzba::ensures(true)]
        fn strict_nodes_equal(
            left: ::jbotci_syntax::generated_model::NodeRef<'_>,
            right: ::jbotci_syntax::generated_model::NodeRef<'_>,
        ) -> bool {
            match (left, right) {
                #(#strict_equality_arms,)*
                _ => false,
            }
        }

        #[bityzba::requires(true)]
        #[bityzba::ensures(true)]
        fn recovered_nodes_equal(
            left: ::jbotci_syntax::generated_model::recovered::NodeRef<'_>,
            right: ::jbotci_syntax::generated_model::recovered::NodeRef<'_>,
        ) -> bool {
            match (left, right) {
                #(#recovered_equality_arms,)*
                _ => false,
            }
        }
    }
}

#[requires(!name.is_empty())]
#[ensures(true)]
fn model_type(name: &str, mode: ProjectionMode) -> TokenStream2 {
    let ident = format_ident!("{name}");
    match mode {
        ProjectionMode::Strict => quote!(::jbotci_syntax::generated_model::#ident),
        ProjectionMode::Recovered => {
            quote!(::jbotci_syntax::generated_model::recovered::#ident)
        }
    }
}

#[requires(!name.is_empty())]
#[ensures(!ret.is_empty())]
fn model_node_idents(schema: &Schema, name: &str) -> Vec<Ident> {
    let model = schema
        .models
        .iter()
        .find(|model| model.strict_name() == name || model.recovered_name() == name)
        .expect("validated model references resolve");
    match model.as_data() {
        data!(Model::Product { .. }) => vec![format_ident!("{name}")],
        data!(Model::Sum { variants, .. }) => variants
            .iter()
            .map(|variant| format_ident!("{name}{}", variant.name))
            .collect(),
    }
}

#[requires(!name.is_empty())]
#[ensures(true)]
fn extract_model_expression(
    schema: &Schema,
    name: &str,
    mode: ProjectionMode,
    value: &TokenStream2,
) -> TokenStream2 {
    let nodes = model_node_idents(schema, name);
    let expected = LitStr::new(name, proc_macro2::Span::call_site());
    match mode {
        ProjectionMode::Strict => quote!({
            let handle = extract_syntax_value(#value)?;
            match &handle.owner.root {
                SyntaxRoot::Strict { value: root } => {
                    match root.node_at_path(&handle.path) {
                        #(
                            Some(::jbotci_syntax::generated_model::NodeRef::#nodes(node)) =>
                                (*node).clone(),
                        )*
                        _ => return Err(::pyo3::exceptions::PyTypeError::new_err(
                            format!("expected generated strict {} value", #expected),
                        )),
                    }
                }
                SyntaxRoot::Recovered { .. } => {
                    return Err(::pyo3::exceptions::PyTypeError::new_err(
                        format!("expected generated strict {} value", #expected),
                    ));
                }
            }
        }),
        ProjectionMode::Recovered => quote!({
            let handle = extract_syntax_value(#value)?;
            match &handle.owner.root {
                SyntaxRoot::Recovered { value: root } => {
                    match root.node_at_path(&handle.path) {
                        #(
                            Some(::jbotci_syntax::generated_model::recovered::NodeRef::#nodes(node)) =>
                                (*node).clone(),
                        )*
                        _ => return Err(::pyo3::exceptions::PyTypeError::new_err(
                            format!("expected generated recovered {} value", #expected),
                        )),
                    }
                }
                SyntaxRoot::Strict { .. } => {
                    return Err(::pyo3::exceptions::PyTypeError::new_err(
                        format!("expected generated recovered {} value", #expected),
                    ));
                }
            }
        }),
    }
}

#[requires(!parameter.is_empty())]
#[ensures(true)]
fn extract_binding_expression(
    schema: &Schema,
    binding: &BindingType,
    mode: ProjectionMode,
    value: &TokenStream2,
    parameter: &str,
) -> TokenStream2 {
    let parameter = LitStr::new(parameter, proc_macro2::Span::call_site());
    match binding.as_data() {
        data!(BindingType::ModelReference { name }) => {
            extract_model_expression(schema, name, mode, value)
        }
        data!(BindingType::LeafReference { kind, path, .. }) => match kind {
            LeafKind::SyntaxToken => quote!({
                (#value)
                    .extract::<::pyo3::PyRef<'_, PySyntaxToken>>()
                    .map_err(|_| ::pyo3::exceptions::PyTypeError::new_err(
                        format!("{} must be a jbotci.syntax.Token", #parameter),
                    ))?
                    .handle
                    .clone_rust()
            }),
            LeafKind::Boolean => quote!((#value).extract::<bool>()?),
            LeafKind::Integer => {
                let integer = integer_rust_type(path);
                quote!((#value).extract::<#integer>()?)
            }
            LeafKind::String => quote!((#value).extract::<String>()?),
            LeafKind::SourceId => quote!({
                (#value)
                    .extract::<::pyo3::PyRef<'_, crate::source::PySourceId>>()?
                    .clone_rust()
            }),
            LeafKind::SourceSpan => quote!({
                (#value)
                    .extract::<::pyo3::PyRef<'_, crate::source::PySourceSpan>>()?
                    .clone_rust()
            }),
            LeafKind::MorphologyWord => quote!({
                crate::morphology::word_handle_from_python(#value)?.clone_rust()
            }),
            LeafKind::MorphologyWordLike => quote!({
                crate::morphology::extract_word_like(#value)?.into_owned()
            }),
            LeafKind::MorphologyCmavo => {
                quote!({
                    crate::morphology::enum_from_python::<::jbotci_morphology::Cmavo>(
                        (#value).py(),
                        #value,
                    )?
                })
            }
            LeafKind::MorphologySelmaho => {
                quote!({
                    crate::morphology::enum_from_python::<::jbotci_morphology::Selmaho>(
                        (#value).py(),
                        #value,
                    )?
                })
            }
            LeafKind::External => unreachable!("external leaves are rejected while parsing"),
        },
        data!(BindingType::Optional { value: inner }) => {
            let inner = extract_binding_expression(schema, inner, mode, value, &parameter.value());
            quote!({
                if (#value).is_none() {
                    None
                } else {
                    Some(#inner)
                }
            })
        }
        data!(BindingType::Repeated { value: inner }) => {
            let item = quote!(item);
            let inner = extract_binding_expression(schema, inner, mode, &item, &parameter.value());
            quote!({
                crate::support::extract_sequence(#value, #parameter, |item| Ok(#inner))?
            })
        }
        data!(BindingType::NonEmptyRepeated { value: inner }) => {
            let item = quote!(item);
            let inner = extract_binding_expression(schema, inner, mode, &item, &parameter.value());
            quote!({
                let values = crate::support::extract_sequence(#value, #parameter, |item| Ok(#inner))?;
                ::vec1::Vec1::try_from_vec(values).map_err(|_| {
                    ::pyo3::exceptions::PyValueError::new_err(
                        format!("{} must contain at least one value", #parameter),
                    )
                })?
            })
        }
        data!(BindingType::Boxed { value: inner }) => {
            let inner = extract_binding_expression(schema, inner, mode, value, &parameter.value());
            quote!(Box::new(#inner))
        }
        data!(BindingType::Shared { value: inner }) => {
            let inner = extract_binding_expression(schema, inner, mode, value, &parameter.value());
            quote!(::std::sync::Arc::new(#inner))
        }
        data!(BindingType::RecoveredField { value: inner }) => {
            let inner_value = quote!(&inner_value);
            let inner =
                extract_binding_expression(schema, inner, mode, &inner_value, &parameter.value());
            quote!({
                let module = (#value).py().import("jbotci.syntax")?;
                let value_type = (#value).get_type();
                if value_type.is(module.getattr("RecoveredValid")?.cast::<::pyo3::types::PyType>()?) {
                    let inner_value = (#value).getattr("value")?;
                    ::jbotci_tree::Recovered::valid(#inner)
                } else if value_type.is(module.getattr("RecoveredError")?.cast::<::pyo3::types::PyType>()?) {
                    let error = (#value).getattr("error")?;
                    ::jbotci_tree::Recovered::error(extract_recovery_item(&error)?)
                } else if value_type.is(module.getattr("RecoveredPrefix")?.cast::<::pyo3::types::PyType>()?) {
                    let errors = (#value).getattr("errors")?;
                    let errors = crate::support::extract_sequence(&errors, "errors", extract_recovery_item)?;
                    if errors.is_empty() {
                        return Err(::pyo3::exceptions::PyValueError::new_err(
                            "errors must contain at least one recovery item",
                        ));
                    }
                    let inner_value = (#value).getattr("value")?;
                    ::jbotci_tree::Recovered::prefix(errors, #inner)
                } else {
                    return Err(::pyo3::exceptions::PyTypeError::new_err(
                        format!("{} must be a RecoveredField variant", #parameter),
                    ));
                }
            })
        }
        data!(BindingType::WithIndicators { .. }) => quote!({
            extract_with_indicators(#value)?.into_owned()
        }),
        data!(BindingType::WithFreeModifiers {
            value: inner,
            free_modifiers,
        }) => {
            let inner_value = quote!(&inner_value);
            let free_value = quote!(&free_values);
            let inner =
                extract_binding_expression(schema, inner, mode, &inner_value, &parameter.value());
            let free = match free_modifiers.as_data() {
                data!(BindingType::Repeated { .. })
                | data!(BindingType::NonEmptyRepeated { .. }) => extract_binding_expression(
                    schema,
                    free_modifiers,
                    mode,
                    &free_value,
                    "free_modifiers",
                ),
                _ => {
                    let item = quote!(item);
                    let item = extract_binding_expression(
                        schema,
                        free_modifiers,
                        mode,
                        &item,
                        "free_modifiers",
                    );
                    quote!({
                        crate::support::extract_sequence(
                            &free_values,
                            "free_modifiers",
                            |item| Ok(#item),
                        )?
                    })
                }
            };
            let construct = match mode {
                ProjectionMode::Strict => quote! {
                    ::jbotci_syntax::tree::WithFreeModifiers::new(inner, free_modifiers)
                },
                ProjectionMode::Recovered => quote! {
                    ::jbotci_syntax::generated_model::recovered::WithFreeModifiers {
                        value: inner,
                        free_modifiers,
                    }
                },
            };
            quote!({
                let module = (#value).py().import("jbotci.syntax")?;
                if !(#value).get_type().is(module.getattr("WithFreeModifiers")?.cast::<::pyo3::types::PyType>()?) {
                    return Err(::pyo3::exceptions::PyTypeError::new_err(
                        format!("{} must be a WithFreeModifiers value", #parameter),
                    ));
                }
                let inner_value = (#value).getattr("value")?;
                let inner = #inner;
                let free_values = (#value).getattr("free_modifiers")?;
                let free_modifiers = #free;
                #construct
            })
        }
        data!(BindingType::Chain { first, links }) => {
            let first_value = quote!(&first_value);
            let links_value = quote!(&link_values);
            let first =
                extract_binding_expression(schema, first, mode, &first_value, &parameter.value());
            let links = extract_binding_expression(schema, links, mode, &links_value, "links");
            quote!({
                let module = (#value).py().import("jbotci.syntax")?;
                if !(#value).get_type().is(module.getattr("Chain")?.cast::<::pyo3::types::PyType>()?) {
                    return Err(::pyo3::exceptions::PyTypeError::new_err(
                        format!("{} must be a Chain value", #parameter),
                    ));
                }
                let first_value = (#value).getattr("first")?;
                let first = #first;
                let link_values = (#value).getattr("links")?;
                let links = #links;
                ::jbotci_tree::Chain::new(first, links)
            })
        }
        data!(BindingType::Tuple { elements }) => {
            let values = elements.iter().enumerate().map(|(index, element)| {
                let value = quote!(&tuple.get_item(#index)?);
                extract_binding_expression(schema, element, mode, &value, &parameter.value())
            });
            let length = elements.len();
            quote!({
                let tuple = (#value).cast::<::pyo3::types::PyTuple>().map_err(|_| {
                    ::pyo3::exceptions::PyTypeError::new_err(
                        format!("{} must be a tuple", #parameter),
                    )
                })?;
                if tuple.len() != #length {
                    return Err(::pyo3::exceptions::PyValueError::new_err(
                        format!("{} must contain exactly {} values", #parameter, #length),
                    ));
                }
                (#(#values,)*)
            })
        }
        data!(BindingType::Fixed {
            length,
            value: inner
        }) => {
            let item = quote!(item);
            let inner = extract_binding_expression(schema, inner, mode, &item, &parameter.value());
            quote!({
                let values = crate::support::extract_sequence(#value, #parameter, |item| Ok(#inner))?;
                if values.len() != #length {
                    return Err(::pyo3::exceptions::PyValueError::new_err(
                        format!("{} must contain exactly {} values", #parameter, #length),
                    ));
                }
                values.try_into().map_err(|_| {
                    ::pyo3::exceptions::PyValueError::new_err(
                        format!("{} has invalid cardinality", #parameter),
                    )
                })?
            })
        }
    }
}

#[requires(!model_name.is_empty())]
#[ensures(true)]
fn constructed_value_expression(
    model_name: &str,
    variant_name: Option<&str>,
    shape: Shape,
    fields: &[Field],
    field_bindings: &[Ident],
    mode: ProjectionMode,
) -> TokenStream2 {
    let ty = model_type(model_name, mode);
    let named_fields = fields.iter().map(|field| match field.rust_name.as_data() {
        data!(RustName::Named { name }) => format_ident!("{name}"),
        data!(RustName::Tuple { .. }) => {
            unreachable!("validated named shapes contain only named Rust fields")
        }
    });
    match variant_name {
        None if shape == Shape::Tuple => {
            quote!(#ty(#(#field_bindings),*))
        }
        None => {
            quote!(#ty { #(#named_fields: #field_bindings),* })
        }
        Some(variant) if shape == Shape::Tuple => {
            let variant = format_ident!("{variant}");
            quote!(#ty::#variant(#(#field_bindings),*))
        }
        Some(variant) => {
            let variant = format_ident!("{variant}");
            quote!(#ty::#variant { #(#named_fields: #field_bindings),* })
        }
    }
}

#[requires(!model_name.is_empty())]
#[ensures(true)]
fn constructor_arm(
    schema: &Schema,
    class_id: usize,
    model_name: &str,
    variant_name: Option<&str>,
    shape: Shape,
    fields: &[Field],
    mode: ProjectionMode,
) -> OutlinedMatchArm {
    let module = LitStr::new(mode.module_name(), proc_macro2::Span::call_site());
    let field_count = fields.len();
    let field_bindings = fields
        .iter()
        .map(|field| format_ident!("field_{}", field.index))
        .collect::<Vec<_>>();
    let python_bindings = fields
        .iter()
        .map(|field| format_ident!("python_field_{}", field.index))
        .collect::<Vec<_>>();
    let extractions = fields
        .iter()
        .zip(&field_bindings)
        .zip(&python_bindings)
        .map(|((field, binding), python)| {
            let index = field.index;
            let parameter = &field.source_name;
            let value = quote!(&#python);
            let expression = extract_binding_expression(
                schema,
                mode_fields(field, mode),
                mode,
                &value,
                parameter,
            );
            quote! {
                let #python = fields.get_item(#index)?;
                let #binding = #expression;
            }
        });
    let value = constructed_value_expression(
        model_name,
        variant_name,
        shape,
        fields,
        &field_bindings,
        mode,
    );
    let root_ident = format_ident!("{model_name}");
    let root = match mode {
        ProjectionMode::Strict => quote! {
            SyntaxRoot::Strict {
                value: StrictSyntaxRoot::#root_ident(::std::sync::Arc::new(value)),
            }
        },
        ProjectionMode::Recovered => quote! {
            SyntaxRoot::Recovered {
                value: RecoveredSyntaxRoot::#root_ident(::std::sync::Arc::new(value)),
            }
        },
    };
    let fn_name = format_ident!("construct_{}_{}", mode.mode_name(), class_id);
    let function = quote! {
        #[bityzba::requires(true)]
        #[bityzba::ensures(ret.is_ok() || ret.is_err())]
        #[inline(never)]
        fn #fn_name(
            fields: &::pyo3::Bound<'_, ::pyo3::types::PyTuple>,
        ) -> ::pyo3::PyResult<PySyntaxValue> {
            if fields.len() != #field_count {
                return Err(::pyo3::exceptions::PyTypeError::new_err(format!(
                    "{} constructor requires exactly {} fields, received {}",
                    #module,
                    #field_count,
                    fields.len(),
                )));
            }
            #(#extractions)*
            // Generated grammar nodes carry audited no-op bityzba invariants:
            // every combination of their already-validated field values is a
            // valid node.  They deliberately remain ordinary public Rust
            // structs/enums, so direct construction is their canonical API.
            let value = #value;
            let owner = ::std::sync::Arc::new(SyntaxOwner {
                root: #root,
                projections: ::std::sync::atomic::AtomicUsize::new(0),
            });
            let handle = ::bityzba::new!(SyntaxHandle {
                owner,
                path: ::jbotci_tree::TreePath::new(),
                class_id: #class_id,
            });
            Ok(PySyntaxValue { handle })
        }
    };
    let arm = quote! {
        (#module, #class_id) => #fn_name(fields)
    };
    OutlinedMatchArm { function, arm }
}

#[requires(!schema.models.is_empty())]
#[ensures(true)]
fn expand_constructors(schema: &Schema) -> TokenStream2 {
    let mut strict_arms = Vec::new();
    let mut recovered_arms = Vec::new();
    let mut class_id = 0usize;
    for model in &schema.models {
        match model.as_data() {
            data!(Model::Product {
                common,
                shape,
                fields,
            }) => {
                strict_arms.push(constructor_arm(
                    schema,
                    class_id,
                    &common.strict_name,
                    None,
                    *shape,
                    fields,
                    ProjectionMode::Strict,
                ));
                recovered_arms.push(constructor_arm(
                    schema,
                    class_id,
                    &common.recovered_name,
                    None,
                    *shape,
                    fields,
                    ProjectionMode::Recovered,
                ));
                class_id += 1;
            }
            data!(Model::Sum { common, variants }) => {
                for variant in variants {
                    strict_arms.push(constructor_arm(
                        schema,
                        class_id,
                        &common.strict_name,
                        Some(&variant.name),
                        variant.shape,
                        &variant.fields,
                        ProjectionMode::Strict,
                    ));
                    recovered_arms.push(constructor_arm(
                        schema,
                        class_id,
                        &common.recovered_name,
                        Some(&variant.name),
                        variant.shape,
                        &variant.fields,
                        ProjectionMode::Recovered,
                    ));
                    class_id += 1;
                }
            }
        }
    }
    let functions = strict_arms
        .iter()
        .chain(&recovered_arms)
        .map(|outlined| &outlined.function);
    let strict_arms = strict_arms.iter().map(|outlined| &outlined.arm);
    let recovered_arms = recovered_arms.iter().map(|outlined| &outlined.arm);
    quote! {
        #(#functions)*

        #[bityzba::requires(true)]
        #[bityzba::ensures(ret.is_ok() || ret.is_err())]
        #[::pyo3::pyfunction]
        #[pyo3(name = "_syntax_construct")]
        fn syntax_construct(
            module_name: &str,
            class_id: usize,
            fields: &::pyo3::Bound<'_, ::pyo3::types::PyTuple>,
        ) -> ::pyo3::PyResult<PySyntaxValue> {
            match (module_name, class_id) {
                #(#strict_arms,)*
                #(#recovered_arms,)*
                _ => Err(::pyo3::exceptions::PyValueError::new_err(format!(
                    "unknown generated syntax class id {} for {}",
                    class_id,
                    module_name,
                ))),
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn project_model_expression(
    mode: ProjectionMode,
    owner: &TokenStream2,
    path: &TokenStream2,
) -> TokenStream2 {
    let (root_pattern, class_id) = match mode {
        ProjectionMode::Strict => (
            quote!(SyntaxRoot::Strict { value: root }),
            quote!(strict_class_id(node)),
        ),
        ProjectionMode::Recovered => (
            quote!(SyntaxRoot::Recovered { value: root }),
            quote!(recovered_class_id(node)),
        ),
    };
    quote!({
        (#owner).record_projection();
        let class_id = match &(#owner).root {
            #root_pattern => {
                let node = root.node_at_path(#path).ok_or_else(|| {
                    ::pyo3::exceptions::PyValueError::new_err(
                        "generated syntax child path did not resolve",
                    )
                })?;
                #class_id
            }
            _ => unreachable!("projection mode matches the typed owner root"),
        };
        wrap_syntax_value(
            py,
            ::bityzba::new!(SyntaxHandle {
                owner: ::std::sync::Arc::clone(#owner),
                path: (#path).clone(),
                class_id,
            }),
        )
    })
}

#[requires(true)]
#[ensures(true)]
fn project_binding_expression(
    binding: &BindingType,
    mode: ProjectionMode,
    value: &TokenStream2,
    owner: &TokenStream2,
    path: &TokenStream2,
    anchor_path: &TokenStream2,
    lens: &[TokenStream2],
) -> TokenStream2 {
    match binding.as_data() {
        data!(BindingType::ModelReference { .. }) => project_model_expression(mode, owner, path),
        data!(BindingType::LeafReference { kind, path, .. }) => match kind {
            LeafKind::SyntaxToken => quote!({
                Ok::<_, ::pyo3::PyErr>(::pyo3::Py::new(
                    py,
                    PySyntaxToken {
                        handle: TokenHandle::from_rust((#value).clone()),
                    },
                )?.into_any())
            }),
            LeafKind::Boolean => quote!({
                Ok::<_, ::pyo3::PyErr>((*(#value)).into_pyobject(py)?.into_any().unbind())
            }),
            LeafKind::Integer => {
                let integer = integer_rust_type(path);
                quote!({
                    let value: #integer = *(#value);
                    Ok::<_, ::pyo3::PyErr>(value.into_pyobject(py)?.into_any().unbind())
                })
            }
            LeafKind::String => quote!({
                Ok::<_, ::pyo3::PyErr>((#value).clone().into_pyobject(py)?.into_any().unbind())
            }),
            LeafKind::SourceId => quote!({
                Ok::<_, ::pyo3::PyErr>(::pyo3::Py::new(py, crate::source::PySourceId::from_rust((#value).clone()))?.into_any())
            }),
            LeafKind::SourceSpan => quote!({
                Ok::<_, ::pyo3::PyErr>(::pyo3::Py::new(py, crate::source::PySourceSpan::from_rust((#value).clone()))?.into_any())
            }),
            LeafKind::MorphologyWord => quote!({
                crate::morphology::word_to_python(
                    py,
                    crate::morphology::WordHandle::from_owned((#value).clone()),
                )
            }),
            LeafKind::MorphologyWordLike => quote!({
                crate::morphology::word_like_to_python(
                    py,
                    crate::morphology::WordLikeHandle::root((#value).clone()),
                )
            }),
            LeafKind::MorphologyCmavo => quote!({
                crate::morphology::enum_to_python(py, *(#value))
            }),
            LeafKind::MorphologySelmaho => quote!({
                crate::morphology::enum_to_python(py, *(#value))
            }),
            LeafKind::External => unreachable!("external leaves are rejected while parsing"),
        },
        data!(BindingType::Optional { value: inner }) => {
            let mut inner_lens = lens.to_vec();
            let tag = LENS_OPTION_VALUE;
            inner_lens.push(quote!(#tag));
            let inner = project_binding_expression(
                inner,
                mode,
                &quote!(value),
                owner,
                path,
                anchor_path,
                &inner_lens,
            );
            quote!({
                match #value {
                    Some(value) => {
                        let value = value;
                        #inner
                    }
                    None => Ok::<_, ::pyo3::PyErr>(py.None()),
                }
            })
        }
        data!(BindingType::Repeated { value: inner })
        | data!(BindingType::NonEmptyRepeated { value: inner }) => {
            let mut item_lens = lens.to_vec();
            let tag = LENS_SEQUENCE_ITEM;
            item_lens.extend([quote!(#tag), quote!(index)]);
            let inner = project_binding_expression(
                inner,
                mode,
                &quote!(value),
                owner,
                &quote!(&item_path),
                anchor_path,
                &item_lens,
            );
            quote!({
                let values = (#value)
                    .iter()
                    .enumerate()
                    .map(|(index, value)| {
                        let mut item_path = (#path).clone();
                        item_path.push(::jbotci_tree::TreePathStep::sequence_index(index));
                        #inner
                    })
                    .collect::<::pyo3::PyResult<Vec<_>>>()?;
                Ok::<_, ::pyo3::PyErr>(crate::support::sequence_to_tuple(py, values)?.unbind().into_any())
            })
        }
        data!(BindingType::Boxed { value: inner })
        | data!(BindingType::Shared { value: inner }) => project_binding_expression(
            inner,
            mode,
            &quote!((#value).as_ref()),
            owner,
            path,
            anchor_path,
            lens,
        ),
        data!(BindingType::RecoveredField { value: inner }) => {
            let mut valid_lens = lens.to_vec();
            let valid_tag = LENS_RECOVERED_VALID;
            valid_lens.push(quote!(#valid_tag));
            let valid = project_binding_expression(
                inner,
                mode,
                &quote!(value.as_ref()),
                owner,
                path,
                anchor_path,
                &valid_lens,
            );
            let mut prefix_lens = lens.to_vec();
            let prefix_tag = LENS_RECOVERED_PREFIX;
            prefix_lens.push(quote!(#prefix_tag));
            let prefix = project_binding_expression(
                inner,
                mode,
                &quote!(prefix.value.as_ref()),
                owner,
                path,
                anchor_path,
                &prefix_lens,
            );
            let valid_identity = valid_lens.iter();
            let mut error_lens = lens.to_vec();
            let error_tag = LENS_RECOVERED_ERROR;
            error_lens.push(quote!(#error_tag));
            let error_identity = error_lens.iter();
            let prefix_identity = prefix_lens.iter();
            quote!({
                match #value {
                    ::jbotci_tree::Recovered::Valid(value) => {
                        let value = #valid?;
                        call_projected_syntax_wrapper(
                            py,
                            "RecoveredValid",
                            vec![value],
                            #owner,
                            #path,
                            vec![#(#valid_identity),*],
                        )
                    }
                    ::jbotci_tree::Recovered::Error(error) => {
                        let error = recovery_item_to_python(py, error.clone())?;
                        call_projected_syntax_wrapper(
                            py,
                            "RecoveredError",
                            vec![error],
                            #owner,
                            #path,
                            vec![#(#error_identity),*],
                        )
                    }
                    ::jbotci_tree::Recovered::Prefix(prefix) => {
                        let errors = prefix
                            .errors
                            .iter()
                            .cloned()
                            .map(|error| recovery_item_to_python(py, error))
                            .collect::<::pyo3::PyResult<Vec<_>>>()?;
                        let errors = crate::support::sequence_to_tuple(py, errors)?.unbind().into_any();
                        let value = #prefix?;
                        call_projected_syntax_wrapper(
                            py,
                            "RecoveredPrefix",
                            vec![errors, value],
                            #owner,
                            #path,
                            vec![#(#prefix_identity),*],
                        )
                    }
                }
            })
        }
        data!(BindingType::WithIndicators { .. }) => {
            let mut projected_lens = lens.to_vec();
            let tag = LENS_WITH_INDICATORS;
            projected_lens.push(quote!(#tag));
            let projected_lens = projected_lens.iter();
            quote!({
                let handle = WithIndicatorsHandle::from_projection(
                    ::std::sync::Arc::clone(#owner),
                    (#anchor_path).clone(),
                    vec![#(#projected_lens),*],
                ).ok_or_else(|| {
                    ::pyo3::exceptions::PyValueError::new_err(
                        "generated WithIndicators field lens did not resolve",
                    )
                })?;
                with_indicators_to_python(py, handle)
            })
        }
        data!(BindingType::WithFreeModifiers {
            value: inner,
            free_modifiers,
        }) => {
            let mut wrapper_lens = lens.to_vec();
            let value_tag = LENS_WITH_FREE_VALUE;
            wrapper_lens.push(quote!(#value_tag));
            let inner = project_binding_expression(
                inner,
                mode,
                &quote!(&(#value).value),
                owner,
                path,
                anchor_path,
                &wrapper_lens,
            );
            let free_binding = match free_modifiers.as_data() {
                data!(BindingType::Repeated { .. })
                | data!(BindingType::NonEmptyRepeated { .. }) => free_modifiers.clone(),
                _ => Box::new(new!(BindingType::Repeated {
                    value: Box::new(free_modifiers.as_ref().clone()),
                })),
            };
            let mut free_lens = lens.to_vec();
            let free_tag = LENS_WITH_FREE_MODIFIERS;
            free_lens.push(quote!(#free_tag));
            let free = project_binding_expression(
                &free_binding,
                mode,
                &quote!(&(#value).free_modifiers),
                owner,
                &quote!(&free_path),
                anchor_path,
                &free_lens,
            );
            let identity = wrapper_lens.iter();
            quote!({
                // Keep the borrowed Rust wrapper named by `#value` in scope
                // until both children have been projected.  The projected
                // Python child must not shadow it before free-modifier access.
                let projected_value = #inner?;
                let mut free_path = (#path).clone();
                free_path.push(::jbotci_tree::TreePathStep::field(Some("free_modifiers"), 1));
                let free_modifiers = #free?;
                call_projected_syntax_wrapper(
                    py,
                    "WithFreeModifiers",
                    vec![projected_value, free_modifiers],
                    #owner,
                    #path,
                    vec![#(#identity),*],
                )
            })
        }
        data!(BindingType::Chain { first, links }) => {
            let mut wrapper_lens = lens.to_vec();
            let first_tag = LENS_CHAIN_FIRST;
            wrapper_lens.push(quote!(#first_tag));
            let first = project_binding_expression(
                first,
                mode,
                &quote!(&(#value).first),
                owner,
                &quote!(&first_path),
                anchor_path,
                &wrapper_lens,
            );
            let link_type = match links.as_data() {
                data!(BindingType::Repeated { value })
                | data!(BindingType::NonEmptyRepeated { value }) => value.as_ref(),
                _ => unreachable!("validated chain links are a sequence wrapper"),
            };
            let mut link_lens = lens.to_vec();
            let link_tag = LENS_CHAIN_LINK;
            link_lens.extend([quote!(#link_tag), quote!(index)]);
            let link = project_binding_expression(
                link_type,
                mode,
                &quote!(link),
                owner,
                &quote!(&link_path),
                anchor_path,
                &link_lens,
            );
            let identity = wrapper_lens.iter();
            quote!({
                let mut first_path = (#path).clone();
                first_path.push(::jbotci_tree::TreePathStep::sequence_index(0));
                let first = #first?;
                let links = (#value)
                    .links
                    .iter()
                    .enumerate()
                    .map(|(index, link)| {
                        let mut link_path = (#path).clone();
                        link_path.push(::jbotci_tree::TreePathStep::sequence_index(index + 1));
                        #link
                    })
                    .collect::<::pyo3::PyResult<Vec<_>>>()?;
                let links = crate::support::sequence_to_tuple(py, links)?.unbind().into_any();
                call_projected_syntax_wrapper(
                    py,
                    "Chain",
                    vec![first, links],
                    #owner,
                    #path,
                    vec![#(#identity),*],
                )
            })
        }
        data!(BindingType::Tuple { elements }) => {
            let projected = elements.iter().enumerate().map(|(index, element)| {
                let tuple_index = syn::Index::from(index);
                let mut item_lens = lens.to_vec();
                let tag = LENS_TUPLE_ITEM;
                item_lens.extend([quote!(#tag), quote!(#index)]);
                let inner = project_binding_expression(
                    element,
                    mode,
                    &quote!(&(#value).#tuple_index),
                    owner,
                    &quote!(&item_path),
                    anchor_path,
                    &item_lens,
                );
                quote!({
                    let mut item_path = (#path).clone();
                    item_path.push(::jbotci_tree::TreePathStep::sequence_index(#index));
                    #inner
                })
            });
            quote!({
                let values = vec![#(#projected?),*];
                Ok::<_, ::pyo3::PyErr>(
                    crate::support::sequence_to_tuple(py, values)?
                        .unbind()
                        .into_any(),
                )
            })
        }
        data!(BindingType::Fixed { value: inner, .. }) => {
            let mut item_lens = lens.to_vec();
            let tag = LENS_SEQUENCE_ITEM;
            item_lens.extend([quote!(#tag), quote!(index)]);
            let inner = project_binding_expression(
                inner,
                mode,
                &quote!(value),
                owner,
                &quote!(&item_path),
                anchor_path,
                &item_lens,
            );
            quote!({
                let values = (#value)
                    .iter()
                    .enumerate()
                    .map(|(index, value)| {
                        let mut item_path = (#path).clone();
                        item_path.push(::jbotci_tree::TreePathStep::sequence_index(index));
                        #inner
                    })
                    .collect::<::pyo3::PyResult<Vec<_>>>()?;
                Ok::<_, ::pyo3::PyErr>(crate::support::sequence_to_tuple(py, values)?.unbind().into_any())
            })
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn product_field_access(field: &Field) -> TokenStream2 {
    match field.rust_name.as_data() {
        data!(RustName::Named { name }) => {
            let field = format_ident!("{name}");
            quote!(&node.#field)
        }
        data!(RustName::Tuple { index }) => {
            let index = syn::Index::from(*index);
            quote!(&node.#index)
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn resolve_with_indicators_expression(
    binding: &BindingType,
    value: &TokenStream2,
    lens: &TokenStream2,
) -> TokenStream2 {
    match binding.as_data() {
        data!(BindingType::ModelReference { .. }) => quote!(None),
        data!(BindingType::LeafReference {
            kind: LeafKind::SyntaxToken,
            ..
        }) => {
            let tag = LENS_WITH_INDICATORS;
            quote!({
                if #lens == [#tag] {
                    Some((#value).as_indicators())
                } else {
                    None
                }
            })
        }
        data!(BindingType::LeafReference { .. }) => quote!(None),
        data!(BindingType::Optional { value: inner }) => {
            let tag = LENS_OPTION_VALUE;
            let inner = resolve_with_indicators_expression(inner, &quote!(value), &quote!(rest));
            quote!({
                let [actual_tag, rest @ ..] = #lens else {
                    return None;
                };
                if *actual_tag != #tag {
                    return None;
                }
                let value = (#value).as_ref()?;
                #inner
            })
        }
        data!(BindingType::Repeated { value: inner })
        | data!(BindingType::NonEmptyRepeated { value: inner }) => {
            let tag = LENS_SEQUENCE_ITEM;
            let inner = resolve_with_indicators_expression(inner, &quote!(value), &quote!(rest));
            quote!({
                let [actual_tag, index, rest @ ..] = #lens else {
                    return None;
                };
                if *actual_tag != #tag {
                    return None;
                }
                let value = (#value).get(*index)?;
                #inner
            })
        }
        data!(BindingType::Boxed { value: inner })
        | data!(BindingType::Shared { value: inner }) => {
            resolve_with_indicators_expression(inner, &quote!((#value).as_ref()), lens)
        }
        data!(BindingType::RecoveredField { value: inner }) => {
            let valid_tag = LENS_RECOVERED_VALID;
            let prefix_tag = LENS_RECOVERED_PREFIX;
            let valid =
                resolve_with_indicators_expression(inner, &quote!(value.as_ref()), &quote!(rest));
            let prefix = resolve_with_indicators_expression(
                inner,
                &quote!(prefix.value.as_ref()),
                &quote!(rest),
            );
            quote!({
                let [actual_tag, rest @ ..] = #lens else {
                    return None;
                };
                match (#value, *actual_tag) {
                    (::jbotci_tree::Recovered::Valid(value), #valid_tag) => #valid,
                    (::jbotci_tree::Recovered::Prefix(prefix), #prefix_tag) => #prefix,
                    _ => None,
                }
            })
        }
        data!(BindingType::WithIndicators { .. }) => {
            let tag = LENS_WITH_INDICATORS;
            quote!({
                if #lens == [#tag] {
                    Some(#value)
                } else {
                    None
                }
            })
        }
        data!(BindingType::WithFreeModifiers {
            value: inner,
            free_modifiers,
        }) => {
            let value_tag = LENS_WITH_FREE_VALUE;
            let free_tag = LENS_WITH_FREE_MODIFIERS;
            let inner =
                resolve_with_indicators_expression(inner, &quote!(&(#value).value), &quote!(rest));
            let free = match free_modifiers.as_data() {
                data!(BindingType::Repeated { .. })
                | data!(BindingType::NonEmptyRepeated { .. }) => {
                    resolve_with_indicators_expression(
                        free_modifiers,
                        &quote!(&(#value).free_modifiers),
                        &quote!(rest),
                    )
                }
                _ => {
                    let sequence_tag = LENS_SEQUENCE_ITEM;
                    let free_modifier = resolve_with_indicators_expression(
                        free_modifiers,
                        &quote!(free_modifier),
                        &quote!(rest),
                    );
                    quote!({
                        let [actual_tag, index, rest @ ..] = rest else {
                            return None;
                        };
                        if *actual_tag != #sequence_tag {
                            return None;
                        }
                        let free_modifier = (#value).free_modifiers.get(*index)?;
                        #free_modifier
                    })
                }
            };
            quote!({
                let [actual_tag, rest @ ..] = #lens else {
                    return None;
                };
                match *actual_tag {
                    #value_tag => #inner,
                    #free_tag => #free,
                    _ => None,
                }
            })
        }
        data!(BindingType::Chain { first, links }) => {
            let first_tag = LENS_CHAIN_FIRST;
            let link_tag = LENS_CHAIN_LINK;
            let first =
                resolve_with_indicators_expression(first, &quote!(&(#value).first), &quote!(rest));
            let link_type = match links.as_data() {
                data!(BindingType::Repeated { value })
                | data!(BindingType::NonEmptyRepeated { value }) => value.as_ref(),
                _ => unreachable!("validated chain links are a sequence wrapper"),
            };
            let link = resolve_with_indicators_expression(link_type, &quote!(link), &quote!(rest));
            quote!({
                let [actual_tag, rest @ ..] = #lens else {
                    return None;
                };
                if *actual_tag == #first_tag {
                    #first
                } else if *actual_tag == #link_tag {
                    let [index, rest @ ..] = rest else {
                        return None;
                    };
                    let link = (#value).links.get(*index)?;
                    #link
                } else {
                    None
                }
            })
        }
        data!(BindingType::Tuple { elements }) => {
            let tag = LENS_TUPLE_ITEM;
            let arms = elements.iter().enumerate().map(|(index, element)| {
                let index_value = syn::Index::from(index);
                let inner = resolve_with_indicators_expression(
                    element,
                    &quote!(&(#value).#index_value),
                    &quote!(rest),
                );
                quote!(#index => #inner)
            });
            quote!({
                let [actual_tag, index, rest @ ..] = #lens else {
                    return None;
                };
                if *actual_tag != #tag {
                    return None;
                }
                match *index {
                    #(#arms,)*
                    _ => None,
                }
            })
        }
        data!(BindingType::Fixed { value: inner, .. }) => {
            let tag = LENS_SEQUENCE_ITEM;
            let inner = resolve_with_indicators_expression(inner, &quote!(value), &quote!(rest));
            quote!({
                let [actual_tag, index, rest @ ..] = #lens else {
                    return None;
                };
                if *actual_tag != #tag {
                    return None;
                }
                let value = (#value).get(*index)?;
                #inner
            })
        }
    }
}

#[requires(!model_name.is_empty() && !variant_name.is_empty())]
#[ensures(true)]
fn variant_field_access(
    model_name: &str,
    variant_name: &str,
    fields: &[Field],
    selected: usize,
    mode: ProjectionMode,
) -> TokenStream2 {
    let ty = model_type(model_name, mode);
    let variant = format_ident!("{variant_name}");
    let bindings = (0..fields.len())
        .map(|index| format_ident!("field_{index}"))
        .collect::<Vec<_>>();
    let selected = &bindings[selected];
    match fields.first().map(|field| field.rust_name.as_data()) {
        Some(data!(RustName::Tuple { .. })) => quote!({
            let #ty::#variant(#(#bindings),*) = node else {
                unreachable!("node-ref variant and enum data variant agree")
            };
            #selected
        }),
        Some(data!(RustName::Named { .. })) => {
            let names = fields.iter().map(|field| match field.rust_name.as_data() {
                data!(RustName::Named { name }) => format_ident!("{name}"),
                data!(RustName::Tuple { .. }) => {
                    unreachable!("validated fields have one consistent shape")
                }
            });
            quote!({
                let #ty::#variant { #(#names: #bindings),* } = node else {
                    unreachable!("node-ref variant and enum data variant agree")
                };
                #selected
            })
        }
        None => unreachable!("generated enum variants have at least one field"),
    }
}

#[requires(!model_name.is_empty())]
#[ensures(ret.len() <= fields.len())]
fn with_indicators_resolution_arms_for_fields(
    model_name: &str,
    variant_name: Option<&str>,
    fields: &[Field],
    mode: ProjectionMode,
    class_id: usize,
) -> Vec<OutlinedMatchArm> {
    let node_ident = match variant_name {
        Some(variant) => format_ident!("{model_name}{variant}"),
        None => format_ident!("{model_name}"),
    };
    fields
        .iter()
        .filter(|field| binding_contains_with_indicators(mode_fields(field, mode)))
        .map(|field| {
            let index = field.index;
            let access = match variant_name {
                Some(variant) => variant_field_access(model_name, variant, fields, index, mode),
                None => product_field_access(field),
            };
            let resolution = resolve_with_indicators_expression(
                mode_fields(field, mode),
                &quote!(value),
                &quote!(remaining),
            );
            let node_pattern = match mode {
                ProjectionMode::Strict => quote!(
                    ::jbotci_syntax::generated_model::NodeRef::#node_ident(node)
                ),
                ProjectionMode::Recovered => quote!(
                    ::jbotci_syntax::generated_model::recovered::NodeRef::#node_ident(node)
                ),
            };
            let root_pattern = match mode {
                ProjectionMode::Strict => quote!(SyntaxRoot::Strict { value: root }),
                ProjectionMode::Recovered => quote!(SyntaxRoot::Recovered { value: root }),
            };
            let root_type = match mode {
                ProjectionMode::Strict => quote!(StrictSyntaxRoot),
                ProjectionMode::Recovered => quote!(RecoveredSyntaxRoot),
            };
            let fn_name = format_ident!(
                "with_indicators_{}_{}_{}",
                mode.mode_name(),
                class_id,
                index,
            );
            let function = quote! {
                #[bityzba::requires(true)]
                #[bityzba::ensures(true)]
                #[inline(never)]
                fn #fn_name<'a>(
                    root: &'a #root_type,
                    path: &::jbotci_tree::TreePath,
                    remaining: &[usize],
                ) -> Option<&'a ::jbotci_syntax::WithIndicators<::jbotci_morphology::WordLike>> {
                    let node = root.node_at_path(path)?;
                    let #node_pattern = node else {
                        return None;
                    };
                    let value = #access;
                    #resolution
                }
            };
            let arm = quote! {
                (#root_pattern, #class_id, #index) => #fn_name(root, path, remaining)
            };
            OutlinedMatchArm { function, arm }
        })
        .collect()
}

#[requires(true)]
#[ensures(true)]
fn binding_contains_with_indicators(binding: &BindingType) -> bool {
    match binding.as_data() {
        data!(BindingType::WithIndicators { .. }) => true,
        data!(BindingType::LeafReference {
            kind: LeafKind::SyntaxToken,
            ..
        }) => true,
        data!(BindingType::ModelReference { .. }) | data!(BindingType::LeafReference { .. }) => {
            false
        }
        data!(BindingType::Optional { value })
        | data!(BindingType::Repeated { value })
        | data!(BindingType::NonEmptyRepeated { value })
        | data!(BindingType::Boxed { value })
        | data!(BindingType::Shared { value })
        | data!(BindingType::RecoveredField { value }) => binding_contains_with_indicators(value),
        data!(BindingType::WithFreeModifiers {
            value,
            free_modifiers,
        }) => {
            binding_contains_with_indicators(value)
                || binding_contains_with_indicators(free_modifiers)
        }
        data!(BindingType::Chain { first, links }) => {
            binding_contains_with_indicators(first) || binding_contains_with_indicators(links)
        }
        data!(BindingType::Tuple { elements }) => {
            elements.iter().any(binding_contains_with_indicators)
        }
        data!(BindingType::Fixed { value, .. }) => binding_contains_with_indicators(value),
    }
}

#[requires(!model_name.is_empty())]
#[ensures(ret.len() == fields.len())]
fn projection_arms_for_fields(
    model_name: &str,
    variant_name: Option<&str>,
    fields: &[Field],
    mode: ProjectionMode,
    class_id: usize,
) -> Vec<OutlinedMatchArm> {
    let node_ident = match variant_name {
        Some(variant) => format_ident!("{model_name}{variant}"),
        None => format_ident!("{model_name}"),
    };
    fields
        .iter()
        .map(|field| {
            let index = field.index;
            let field_name = match field.rust_name.as_data() {
                data!(RustName::Named { name }) => {
                    let name = LitStr::new(name, proc_macro2::Span::call_site());
                    quote!(Some(#name))
                }
                data!(RustName::Tuple { .. }) => quote!(None),
            };
            let access = match variant_name {
                Some(variant) => variant_field_access(model_name, variant, fields, index, mode),
                None => product_field_access(field),
            };
            let root_lens = [quote!(#class_id), quote!(#index)];
            let projection = project_binding_expression(
                mode_fields(field, mode),
                mode,
                &quote!(value),
                &quote!(&handle.owner),
                &quote!(&field_path),
                &quote!(&handle.path),
                &root_lens,
            );
            let node_pattern = match mode {
                ProjectionMode::Strict => quote!(
                    ::jbotci_syntax::generated_model::NodeRef::#node_ident(node)
                ),
                ProjectionMode::Recovered => quote!(
                    ::jbotci_syntax::generated_model::recovered::NodeRef::#node_ident(node)
                ),
            };
            let root_pattern = match mode {
                ProjectionMode::Strict => quote!(SyntaxRoot::Strict { value: root }),
                ProjectionMode::Recovered => quote!(SyntaxRoot::Recovered { value: root }),
            };
            let root_type = match mode {
                ProjectionMode::Strict => quote!(StrictSyntaxRoot),
                ProjectionMode::Recovered => quote!(RecoveredSyntaxRoot),
            };
            let fn_name = format_ident!("project_{}_{}_{}", mode.mode_name(), class_id, index,);
            let function = quote! {
                #[bityzba::requires(true)]
                #[bityzba::ensures(ret.is_ok() || ret.is_err())]
                #[inline(never)]
                fn #fn_name(
                    py: ::pyo3::Python<'_>,
                    handle: &SyntaxHandle,
                    root: &#root_type,
                ) -> ::pyo3::PyResult<::pyo3::Py<::pyo3::PyAny>> {
                    handle.owner.record_projection();
                    let node = root.node_at_path(&handle.path).ok_or_else(|| {
                        ::pyo3::exceptions::PyValueError::new_err(
                            "generated syntax parent path did not resolve",
                        )
                    })?;
                    let #node_pattern = node else {
                        return Err(::pyo3::exceptions::PyTypeError::new_err(
                            "generated syntax class id does not match its Rust node",
                        ));
                    };
                    let value = #access;
                    let mut field_path = handle.path.clone();
                    field_path.push(::jbotci_tree::TreePathStep::field(#field_name, #index));
                    #projection
                }
            };
            let arm = quote! {
                (#root_pattern, #class_id, #index) => #fn_name(py, handle, root)
            };
            OutlinedMatchArm { function, arm }
        })
        .collect()
}

#[requires(!schema.models.is_empty())]
#[ensures(true)]
fn expand_field_projection(schema: &Schema) -> TokenStream2 {
    let mut arms = Vec::new();
    let mut with_indicators_arms = Vec::new();
    let mut class_id = 0usize;
    for model in &schema.models {
        match model.as_data() {
            data!(Model::Product { common, fields, .. }) => {
                arms.extend(projection_arms_for_fields(
                    &common.strict_name,
                    None,
                    fields,
                    ProjectionMode::Strict,
                    class_id,
                ));
                arms.extend(projection_arms_for_fields(
                    &common.recovered_name,
                    None,
                    fields,
                    ProjectionMode::Recovered,
                    class_id,
                ));
                with_indicators_arms.extend(with_indicators_resolution_arms_for_fields(
                    &common.strict_name,
                    None,
                    fields,
                    ProjectionMode::Strict,
                    class_id,
                ));
                with_indicators_arms.extend(with_indicators_resolution_arms_for_fields(
                    &common.recovered_name,
                    None,
                    fields,
                    ProjectionMode::Recovered,
                    class_id,
                ));
                class_id += 1;
            }
            data!(Model::Sum { common, variants }) => {
                for variant in variants {
                    arms.extend(projection_arms_for_fields(
                        &common.strict_name,
                        Some(&variant.name),
                        &variant.fields,
                        ProjectionMode::Strict,
                        class_id,
                    ));
                    arms.extend(projection_arms_for_fields(
                        &common.recovered_name,
                        Some(&variant.name),
                        &variant.fields,
                        ProjectionMode::Recovered,
                        class_id,
                    ));
                    with_indicators_arms.extend(with_indicators_resolution_arms_for_fields(
                        &common.strict_name,
                        Some(&variant.name),
                        &variant.fields,
                        ProjectionMode::Strict,
                        class_id,
                    ));
                    with_indicators_arms.extend(with_indicators_resolution_arms_for_fields(
                        &common.recovered_name,
                        Some(&variant.name),
                        &variant.fields,
                        ProjectionMode::Recovered,
                        class_id,
                    ));
                    class_id += 1;
                }
            }
        }
    }
    let projection_functions = arms.iter().map(|outlined| &outlined.function);
    let arms = arms.iter().map(|outlined| &outlined.arm);
    let with_indicators_functions = with_indicators_arms
        .iter()
        .map(|outlined| &outlined.function);
    let with_indicators_arms = with_indicators_arms.iter().map(|outlined| &outlined.arm);
    quote! {
        #(#projection_functions)*

        #(#with_indicators_functions)*

        #[bityzba::requires(true)]
        #[bityzba::ensures(ret.is_ok() || ret.is_err())]
        fn project_syntax_field(
            py: ::pyo3::Python<'_>,
            handle: &SyntaxHandle,
            index: usize,
        ) -> ::pyo3::PyResult<::pyo3::Py<::pyo3::PyAny>> {
            match (&handle.owner.root, handle.class_id, index) {
                #(#arms,)*
                _ => Err(::pyo3::exceptions::PyIndexError::new_err(format!(
                    "syntax field index {} is not available on {}",
                    index,
                    handle.class_name(),
                ))),
            }
        }

        #[bityzba::requires(!lens.is_empty())]
        #[bityzba::ensures(true)]
        fn resolve_syntax_with_indicators<'a>(
            owner: &'a SyntaxOwner,
            path: &::jbotci_tree::TreePath,
            lens: &[usize],
        ) -> Option<&'a ::jbotci_syntax::WithIndicators<::jbotci_morphology::WordLike>> {
            let [class_id, field_index, remaining @ ..] = lens else {
                return None;
            };
            match (&owner.root, *class_id, *field_index) {
                #(#with_indicators_arms,)*
                _ => None,
            }
        }

        impl SyntaxOwner {
            #[bityzba::requires(!lens.is_empty())]
            #[bityzba::ensures(true)]
            pub(crate) fn with_indicators_at<'a>(
                &'a self,
                path: &::jbotci_tree::TreePath,
                lens: &[usize],
            ) -> Option<&'a ::jbotci_syntax::WithIndicators<::jbotci_morphology::WordLike>> {
                resolve_syntax_with_indicators(self, path, lens)
            }
        }
    }
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn expand_parity_inventory(schema: Schema) -> Result<TokenStream2> {
    let mut entries = Vec::<(String, String, String, String)>::new();
    for mode in [ProjectionMode::Strict, ProjectionMode::Recovered] {
        let rust_module = match mode {
            ProjectionMode::Strict => "jbotci_syntax::generated_model",
            ProjectionMode::Recovered => "jbotci_syntax::generated_model::recovered",
        };
        let python_module = match mode {
            ProjectionMode::Strict => "jbotci.syntax.strict",
            ProjectionMode::Recovered => "jbotci.syntax.recovered",
        };
        for model in &schema.models {
            let model_name = mode_model_name(model, mode);
            let rust_model = format!("{rust_module}::{model_name}");
            let python_model = format!("{python_module}.{model_name}");
            let model_signature = match model.as_data() {
                data!(Model::Product { .. }) => "product",
                data!(Model::Sum { .. }) => "sum",
            };
            entries.push((
                rust_model.clone(),
                "type".to_owned(),
                model_signature.to_owned(),
                python_model.clone(),
            ));
            entries.push((
                format!("{rust_module}::NodeRef::{model_name}"),
                "variant".to_owned(),
                format!("&{model_name}"),
                python_model.clone(),
            ));
            let walk_name = parity_walk_name(model_name);
            entries.push((
                format!("{rust_module}::TreeWalker::walk_{walk_name}"),
                "trait-method".to_owned(),
                format!("&{model_name}"),
                python_model.clone(),
            ));
            entries.push((
                format!("{rust_module}::walk::{walk_name}"),
                "function".to_owned(),
                format!("&{model_name}"),
                python_model.clone(),
            ));
            if mode == ProjectionMode::Recovered {
                for method in ["from_valid", "from_valid_boxed", "try_into_valid"] {
                    entries.push((
                        format!("{rust_model}::{method}"),
                        "method".to_owned(),
                        method.to_owned(),
                        python_model.clone(),
                    ));
                }
            }
            match model.as_data() {
                data!(Model::Product { fields, .. }) => {
                    for field in fields {
                        entries.push((
                            format!("{rust_model}::{}", field.source_name),
                            "field".to_owned(),
                            mode_fields(field, mode).annotation(false),
                            format!("{python_model}.{}", field.source_name),
                        ));
                    }
                }
                data!(Model::Sum { variants, .. }) => {
                    for variant in variants {
                        let python_variant_name = variant_class_name(model_name, &variant.name);
                        let python_variant = format!("{python_module}.{python_variant_name}");
                        entries.push((
                            format!("{rust_model}::{}", variant.name),
                            "variant".to_owned(),
                            format!("{:?}", variant.shape),
                            python_variant.clone(),
                        ));
                        let variant_walk_name =
                            format!("{walk_name}_{}", parity_snake_case(&variant.name));
                        entries.push((
                            format!("{rust_module}::TreeWalker::walk_{variant_walk_name}"),
                            "trait-method".to_owned(),
                            format!("{model_name}::{}", variant.name),
                            python_variant.clone(),
                        ));
                        entries.push((
                            format!("{rust_module}::walk::{variant_walk_name}"),
                            "function".to_owned(),
                            format!("{model_name}::{}", variant.name),
                            python_variant.clone(),
                        ));
                        for field in &variant.fields {
                            entries.push((
                                format!("{rust_model}::{}::{}", variant.name, field.source_name),
                                "field".to_owned(),
                                mode_fields(field, mode).annotation(false),
                                format!("{python_variant}.{}", field.source_name),
                            ));
                        }
                    }
                }
            }
        }
        let fixed = [
            ("NodeRef", "type", "generated node reference"),
            ("AtomRef", "type", "generated atom reference"),
            ("AtomRef::Token", "variant", "&Token"),
            ("TreeNode", "trait", "generated in-order traversal"),
            (
                "TreeNode::as_node_ref",
                "trait-method",
                "&self -> Option<NodeRef>",
            ),
            ("TreeNode::visit_in_order", "trait-method", "&self, visitor"),
            (
                "TreeNode::path_to_node",
                "trait-method",
                "&self, NodeRef -> Option<TreePath>",
            ),
            (
                "TreeNode::node_at_path",
                "trait-method",
                "&self, &TreePath -> Option<NodeRef>",
            ),
            (
                "TreeNode::path_to_node_from",
                "trait-method",
                "&self, NodeRef, &mut TreePath -> bool",
            ),
            (
                "TreeNode::node_at_path_steps",
                "trait-method",
                "&self, &[TreePathStep] -> Option<NodeRef>",
            ),
            ("TreeWalker", "trait", "generated recursive traversal"),
            ("TreeWalker::walk_atom", "trait-method", "AtomRef"),
            ("TreeWalkable", "trait", "generated recursive dispatch"),
            ("TreeWalkable::walk_with", "trait-method", "&self, walker"),
            ("walk", "module", "generated free descent functions"),
        ];
        for (suffix, kind, signature) in fixed {
            entries.push((
                format!("{rust_module}::{suffix}"),
                kind.to_owned(),
                signature.to_owned(),
                python_module.to_owned(),
            ));
        }
        for function in [
            "with_free_modifiers",
            "boxed",
            "arc",
            "option",
            "tuple2",
            "chain_vec",
            "chain_vec1",
            "vec",
            "vec1",
            "small_vec",
            "small_vec1",
        ] {
            entries.push((
                format!("{rust_module}::walk::{function}"),
                "function".to_owned(),
                "generic generated descent".to_owned(),
                python_module.to_owned(),
            ));
        }
        if mode == ProjectionMode::Recovered {
            entries.push((
                format!("{rust_module}::TreeWalker::walk_recovered_error"),
                "trait-method".to_owned(),
                "&RecoveryTreeItem".to_owned(),
                "jbotci.syntax.RecoveredError".to_owned(),
            ));
            entries.push((
                format!("{rust_module}::walk::recovered"),
                "function".to_owned(),
                "&Recovered<T>".to_owned(),
                "jbotci.syntax.RecoveredField".to_owned(),
            ));
            for alias in ["Recovered", "RecoveryError"] {
                entries.push((
                    format!("{rust_module}::{alias}"),
                    "type-alias".to_owned(),
                    alias.to_owned(),
                    "jbotci.syntax.RecoveredField".to_owned(),
                ));
            }
        }
    }
    entries.sort();
    let entries = entries.into_iter().map(|(rust, kind, signature, python)| {
        let rust = LitStr::new(&rust, proc_macro2::Span::call_site());
        let kind = LitStr::new(&kind, proc_macro2::Span::call_site());
        let signature = LitStr::new(&signature, proc_macro2::Span::call_site());
        let python = LitStr::new(&python, proc_macro2::Span::call_site());
        quote!((#rust, #kind, #signature, #python))
    });
    Ok(quote! {
        const GENERATED_SYNTAX_API: &[(&str, &str, &str, &str)] = &[#(#entries),*];
    })
}

#[requires(!name.is_empty())]
#[ensures(!ret.is_empty())]
fn parity_walk_name(name: &str) -> String {
    parity_snake_case(name.strip_suffix("Syntax").unwrap_or(name))
}

#[requires(!name.is_empty())]
#[ensures(!ret.is_empty())]
fn parity_snake_case(name: &str) -> String {
    let mut output = String::new();
    let mut previous_is_lower_or_digit = false;
    let mut chars = name.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch.is_ascii_uppercase() {
            let next_is_lower = chars.peek().is_some_and(|next| next.is_ascii_lowercase());
            if !output.is_empty() && (previous_is_lower_or_digit || next_is_lower) {
                output.push('_');
            }
            output.push(ch.to_ascii_lowercase());
            previous_is_lower_or_digit = false;
        } else {
            output.push(ch);
            previous_is_lower_or_digit = ch.is_ascii_lowercase() || ch.is_ascii_digit();
        }
    }
    output
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn expand_schema(schema: Schema) -> Result<TokenStream2> {
    let model_count = schema.models.len();
    let variant_count = schema
        .models
        .iter()
        .map(|model| match model.as_data() {
            data!(Model::Product { .. }) => 0,
            data!(Model::Sum { variants, .. }) => variants.len(),
        })
        .sum::<usize>();
    let field_count = schema
        .models
        .iter()
        .map(|model| match model.as_data() {
            data!(Model::Product { fields, .. }) => fields.len(),
            data!(Model::Sum { variants, .. }) => {
                variants.iter().map(|variant| variant.fields.len()).sum()
            }
        })
        .sum::<usize>();
    let strict = render_namespace(&schema, ProjectionMode::Strict);
    let recovered = render_namespace(&schema, ProjectionMode::Recovered);
    let strict_runtime = LitStr::new(&strict.runtime, proc_macro2::Span::call_site());
    let strict_stub = LitStr::new(&strict.stub, proc_macro2::Span::call_site());
    let recovered_runtime = LitStr::new(&recovered.runtime, proc_macro2::Span::call_site());
    let recovered_stub = LitStr::new(&recovered.stub, proc_macro2::Span::call_site());
    let strict_inventory = strict
        .inventory
        .iter()
        .map(|name| LitStr::new(name, proc_macro2::Span::call_site()));
    let recovered_inventory = recovered
        .inventory
        .iter()
        .map(|name| LitStr::new(name, proc_macro2::Span::call_site()));
    let strict_concrete_inventory = strict
        .concrete_inventory
        .iter()
        .map(|name| LitStr::new(name, proc_macro2::Span::call_site()));
    let recovered_concrete_inventory = recovered
        .concrete_inventory
        .iter()
        .map(|name| LitStr::new(name, proc_macro2::Span::call_site()));
    let native_roots = expand_native_roots(&schema);
    let constructors = expand_constructors(&schema);
    let field_projection = expand_field_projection(&schema);
    let lens_with_indicators = LENS_WITH_INDICATORS;
    let lens_with_free_value = LENS_WITH_FREE_VALUE;
    Ok(quote! {
        pub(crate) const SYNTAX_SCHEMA_MODEL_COUNT: usize = #model_count;
        pub(crate) const SYNTAX_SCHEMA_VARIANT_COUNT: usize = #variant_count;
        pub(crate) const SYNTAX_SCHEMA_FIELD_COUNT: usize = #field_count;
        pub(crate) const SYNTAX_STRICT_RUNTIME_SOURCE: &str = #strict_runtime;
        pub(crate) const SYNTAX_STRICT_STUB: &str = #strict_stub;
        pub(crate) const SYNTAX_RECOVERED_RUNTIME_SOURCE: &str = #recovered_runtime;
        pub(crate) const SYNTAX_RECOVERED_STUB: &str = #recovered_stub;
        pub(crate) const SYNTAX_STRICT_INVENTORY: &[&str] = &[#(#strict_inventory),*];
        pub(crate) const SYNTAX_RECOVERED_INVENTORY: &[&str] = &[#(#recovered_inventory),*];
        pub(crate) const SYNTAX_STRICT_CONCRETE_INVENTORY: &[&str] = &[#(#strict_concrete_inventory),*];
        pub(crate) const SYNTAX_RECOVERED_CONCRETE_INVENTORY: &[&str] = &[#(#recovered_concrete_inventory),*];
        #[cfg(test)]
        const SYNTAX_LENS_WITH_INDICATORS: usize = #lens_with_indicators;
        #[cfg(test)]
        const SYNTAX_LENS_WITH_FREE_VALUE: usize = #lens_with_free_value;
        #native_roots
        #constructors
        #field_projection
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[requires(!name.is_empty())]
    #[ensures(ret.source_name == name)]
    fn string_field(name: &str, index: usize) -> Field {
        let binding = new!(BindingType::LeafReference {
            kind: LeafKind::String,
            absolute: false,
            path: vec!["String".to_owned()],
        });
        new!(Field {
            source_name: name.to_owned(),
            rust_name: new!(RustName::Named {
                name: name.to_owned(),
            }),
            index,
            docs: vec![format!("The {name} field.")],
            strict: binding.clone(),
            recovered: binding,
        })
    }

    #[requires(true)]
    #[ensures(ret.models.len() == 2)]
    fn synthetic_schema(extended: bool) -> Schema {
        let mut fields = vec![string_field("first", 0), string_field("second", 1)];
        if extended {
            fields.push(string_field("third", 2));
        }
        let payload = new!(BindingType::ModelReference {
            name: "RecordSyntax".to_owned(),
        });
        let variant = |name: &str| {
            new!(Variant {
                name: name.to_owned(),
                owner_rule: "choice".to_owned(),
                source_rule: name.to_ascii_lowercase(),
                docs: vec![format!("The {name} choice.")],
                constructor: new!(Constructor {
                    name: name.to_owned(),
                    label: None,
                }),
                shape: Shape::Tuple,
                fields: vec![new!(Field {
                    source_name: name.to_ascii_lowercase(),
                    rust_name: new!(RustName::Tuple { index: 0 }),
                    index: 0,
                    docs: vec!["The record payload.".to_owned()],
                    strict: payload.clone(),
                    recovered: payload.clone(),
                })],
            })
        };
        let mut variants = vec![variant("Alpha")];
        if extended {
            variants.push(variant("Beta"));
            variants.push(new!(Variant {
                name: "Named".to_owned(),
                owner_rule: "choice".to_owned(),
                source_rule: "named".to_owned(),
                docs: vec!["A named choice.".to_owned()],
                constructor: new!(Constructor {
                    name: "Named".to_owned(),
                    label: None,
                }),
                shape: Shape::Named,
                fields: vec![string_field("left", 0), string_field("right", 1)],
            }));
            variants.push(new!(Variant {
                name: "Unit".to_owned(),
                owner_rule: "choice".to_owned(),
                source_rule: "unit".to_owned(),
                docs: vec!["A unit choice.".to_owned()],
                constructor: new!(Constructor {
                    name: "Unit".to_owned(),
                    label: None,
                }),
                shape: Shape::Named,
                fields: Vec::new(),
            }));
        }
        new!(Schema {
            version: 1,
            models: vec![
                new!(Model::Product {
                    common: new!(ModelCommon {
                        strict_name: "RecordSyntax".to_owned(),
                        recovered_name: "RecordSyntax".to_owned(),
                        rule: "record".to_owned(),
                        docs: vec!["A synthetic record.".to_owned()],
                        constructor: new!(Constructor {
                            name: "Record".to_owned(),
                            label: None,
                        }),
                    }),
                    shape: Shape::Named,
                    fields,
                }),
                new!(Model::Sum {
                    common: new!(ModelCommon {
                        strict_name: "ChoiceSyntax".to_owned(),
                        recovered_name: "ChoiceSyntax".to_owned(),
                        rule: "choice".to_owned(),
                        docs: vec![
                            "A synthetic choice.".to_owned(),
                            String::new(),
                            "Its alias documentation spans paragraphs.".to_owned(),
                        ],
                        constructor: new!(Constructor {
                            name: "Choice".to_owned(),
                            label: None,
                        }),
                    }),
                    variants,
                }),
            ],
            metadata: Metadata {
                transparent_constructors: Vec::new(),
                transparent_fields: Vec::new(),
                chain_link_element_fields: Vec::new(),
                constructor_labels: Vec::new(),
                elidable_terminators: Vec::new(),
                field_orders: Vec::new(),
            },
        })
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn schema_changes_drive_runtime_stub_and_inventory_together() {
        let base = render_namespace(&synthetic_schema(false), ProjectionMode::Strict);
        let extended = render_namespace(&synthetic_schema(true), ProjectionMode::Strict);

        assert!(!base.runtime.contains("def third"));
        assert!(extended.runtime.contains("def third"));
        assert!(!base.stub.contains("def third"));
        assert!(extended.stub.contains("def third"));
        assert!(!base.inventory.iter().any(|name| name.ends_with("Beta")));
        assert!(
            extended
                .inventory
                .iter()
                .any(|name| name == "ChoiceSyntaxBeta")
        );
        assert!(extended.runtime.contains("ChoiceSyntaxBeta"));
        assert!(extended.stub.contains("ChoiceSyntaxBeta"));
        assert!(extended.runtime.contains("class ChoiceSyntaxNamed"));
        assert!(extended.runtime.contains("def left"));
        assert!(extended.stub.contains("class ChoiceSyntaxNamed"));
        assert!(extended.stub.contains("left: str"));
        assert!(extended.runtime.contains("class ChoiceSyntaxUnit"));
        assert!(extended.runtime.contains("__match_args__ = ()"));
        assert!(extended.stub.contains("class ChoiceSyntaxUnit"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn multiline_sum_documentation_is_fully_commented_in_stubs() {
        let rendered = render_namespace(&synthetic_schema(false), ProjectionMode::Strict);

        assert!(rendered.stub.contains(
            "# A synthetic choice.\n#\n# Its alias documentation spans paragraphs.\nChoiceSyntax: TypeAlias"
        ));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn unsupported_schema_fields_fail_instead_of_being_ignored() {
        let input: TokenStream2 = quote! {
            syntax_binding_schema {
                version(1),
                models [product {
                    names(strict("UnitSyntax"), recovered("UnitSyntax")),
                    rule("unit"),
                    docs ["A unit."],
                    constructor(name("Unit"), label(none)),
                    shape(named),
                    fields []
                }],
                metadata {
                    transparent_constructors [],
                    transparent_fields [],
                    chain_link_element_fields [],
                    constructor_labels [],
                    elidable_terminators [],
                    field_orders []
                },
                unknown("not supported")
            }
        };
        let error = parse_schema(input).expect_err("unknown root field must fail");
        assert!(error.to_string().contains("unsupported trailing"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn blank_documentation_lines_are_preserved() {
        let input: TokenStream2 = quote! {
            syntax_binding_schema {
                version(1),
                models [product {
                    names(strict("UnitSyntax"), recovered("UnitSyntax")),
                    rule("unit"),
                    docs ["First paragraph.", "", "Second paragraph."],
                    constructor(name("Unit"), label(none)),
                    shape(named),
                    fields []
                }],
                metadata {
                    transparent_constructors [],
                    transparent_fields [],
                    chain_link_element_fields [],
                    constructor_labels [],
                    elidable_terminators [],
                    field_orders []
                }
            }
        };
        let schema = parse_schema(input).expect("blank documentation separators are valid");
        let data!(Model::Product { common, .. }) = schema.models[0].as_data() else {
            panic!("fixture is a product model");
        };
        assert_eq!(common.docs, ["First paragraph.", "", "Second paragraph."]);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn unsupported_schema_versions_fail_explicitly() {
        let input: TokenStream2 = quote! {
            syntax_binding_schema {
                version(2),
                models [],
                metadata {
                    transparent_constructors [],
                    transparent_fields [],
                    chain_link_element_fields [],
                    constructor_labels [],
                    elidable_terminators [],
                    field_orders []
                }
            }
        };
        let error = parse_schema(input).expect_err("unknown schema version must fail");
        assert!(
            error
                .to_string()
                .contains("unsupported syntax binding schema version 2")
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn unsupported_schema_wrappers_fail_instead_of_mapping_to_object() {
        let error = parse_binding_type(quote!(mystery(reference(model("UnitSyntax")))))
            .expect_err("unknown wrapper must fail");
        assert!(
            error
                .to_string()
                .contains("unknown normalized binding wrapper")
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn with_indicators_rejects_non_word_like_payloads() {
        let binding = new!(BindingType::WithIndicators {
            value: Box::new(new!(BindingType::LeafReference {
                kind: LeafKind::String,
                absolute: false,
                path: vec!["String".to_owned()],
            })),
        });
        let error = validate_type_references(&binding, &BTreeSet::new())
            .expect_err("only the canonical WordLike indicator payload is supported");
        assert!(error.to_string().contains("canonical WordLike leaf"));
    }
}
