//! External compile-time consumer used to prove that the exported schema does
//! not depend on proc-macro implementation details or Python bindings.

use std::collections::{BTreeSet, VecDeque};

use bityzba::{data, invariant, new};
use proc_macro::{Delimiter, TokenStream, TokenTree};

#[bityzba::invariant(true)]
struct Cursor {
    tokens: VecDeque<TokenTree>,
}

impl Cursor {
    #[bityzba::requires(true)]
    #[bityzba::ensures(true)]
    fn new(stream: TokenStream) -> Self {
        Self {
            tokens: stream.into_iter().collect(),
        }
    }

    #[bityzba::requires(true)]
    #[bityzba::ensures(true)]
    fn is_done(&self) -> bool {
        self.tokens.is_empty()
    }

    #[bityzba::requires(!self.is_done())]
    #[bityzba::ensures(self.tokens.len() + 1 == old(self.tokens.len()))]
    fn take(&mut self) -> TokenTree {
        self.tokens
            .pop_front()
            .expect("schema cursor precondition guarantees a token")
    }

    #[bityzba::requires(!self.is_done())]
    #[bityzba::ensures(!ret.is_empty())]
    fn take_ident(&mut self) -> String {
        let TokenTree::Ident(ident) = self.take() else {
            panic!("expected schema identifier")
        };
        ident.to_string()
    }

    #[bityzba::requires(!expected.is_empty())]
    #[bityzba::ensures(true)]
    fn expect_ident(&mut self, expected: &str) {
        assert_eq!(self.take_ident(), expected, "unexpected schema identifier");
    }

    #[bityzba::requires(!self.is_done())]
    #[bityzba::ensures(true)]
    fn take_group(&mut self, delimiter: Delimiter) -> TokenStream {
        let TokenTree::Group(group) = self.take() else {
            panic!("expected schema token group")
        };
        assert_eq!(group.delimiter(), delimiter, "unexpected schema delimiter");
        group.stream()
    }

    #[bityzba::requires(!self.is_done())]
    #[bityzba::ensures(!ret.is_empty())]
    fn take_literal(&mut self) -> String {
        let TokenTree::Literal(literal) = self.take() else {
            panic!("expected schema literal")
        };
        literal.to_string()
    }

    #[bityzba::requires(true)]
    #[bityzba::ensures(true)]
    fn expect_punct(&mut self, expected: char) {
        let TokenTree::Punct(punct) = self.take() else {
            panic!("expected schema punctuation")
        };
        assert_eq!(punct.as_char(), expected, "unexpected schema punctuation");
    }

    #[bityzba::requires(true)]
    #[bityzba::ensures(self.is_done())]
    fn finish(&self) {
        assert!(self.is_done(), "unexpected trailing schema tokens");
    }
}

#[bityzba::invariant(
    ::Product => shape == "named" || shape == "tuple",
    "product shapes are normalized to named or tuple"
)]
#[invariant(::Sum => true)]
#[derive(Debug, Clone, PartialEq, Eq)]
enum ModelKind {
    Product { shape: String },
    Sum,
}

impl ModelKind {
    #[bityzba::requires(true)]
    #[bityzba::ensures(true)]
    fn matches_model_shape(&self, field_count: usize, variant_count: usize) -> bool {
        match self.as_data() {
            data!(ModelKind::Product { shape }) => {
                variant_count == 0 && ((shape == "tuple") == (field_count == 1))
            }
            data!(ModelKind::Sum) => field_count == 0 && variant_count > 0,
        }
    }
}

#[bityzba::invariant(!name.is_empty(), "schema models are named")]
#[bityzba::invariant(!rule.is_empty(), "schema models identify their source rule")]
#[bityzba::invariant(*has_nonblank_docs, "schema models have canonical documentation")]
#[bityzba::invariant(
    kind.matches_model_shape(fields.len(), variants.len()),
    "product and sum payloads match their declared model kind"
)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct ModelSchema {
    name: String,
    rule: String,
    has_nonblank_docs: bool,
    kind: ModelKind,
    fields: Vec<FieldSchema>,
    variants: Vec<VariantSchema>,
}

#[bityzba::invariant(!name.is_empty(), "schema variants are named")]
#[bityzba::invariant(!source_rule.is_empty(), "schema variants identify their source rule")]
#[bityzba::invariant(*has_nonblank_docs, "schema variants have canonical documentation")]
#[bityzba::invariant(
    shape == "named" || shape == "tuple",
    "variant shapes are normalized to named or tuple"
)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct VariantSchema {
    name: String,
    source_rule: String,
    has_nonblank_docs: bool,
    shape: String,
    fields: Vec<FieldSchema>,
}

#[bityzba::invariant(!source_name.is_empty(), "schema fields retain a source name")]
#[derive(Debug, Clone, PartialEq, Eq)]
struct FieldSchema {
    source_name: String,
    index: usize,
    strict: BindingType,
    recovered: BindingType,
}

#[bityzba::invariant(::ModelReference => !name.is_empty(), "model references are named")]
#[bityzba::invariant(
    ::LeafReference => !kind.is_empty()
        && !path.is_empty()
        && path.iter().all(|component| !component.is_empty()),
    "leaf references have a kind and a nonempty path"
)]
#[invariant(::Optional => true)]
#[invariant(::Repeated => true)]
#[invariant(::NonEmptyRepeated => true)]
#[invariant(::Boxed => true)]
#[invariant(::Shared => true)]
#[invariant(::RecoveredField => true)]
#[invariant(::WithIndicators => true)]
#[invariant(::WithFreeModifiers => true)]
#[invariant(::Chain => true)]
#[invariant(::Tuple => true)]
#[invariant(::Fixed => true)]
#[derive(Debug, Clone, PartialEq, Eq)]
enum BindingType {
    ModelReference {
        name: String,
    },
    LeafReference {
        kind: String,
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

#[bityzba::invariant(true)]
struct SchemaSummary {
    models: Vec<ModelSchema>,
    transparent_constructors: usize,
    transparent_fields: usize,
    chain_link_element_fields: usize,
    constructor_labels: usize,
    elidable_terminators: usize,
    field_orders: usize,
}

#[bityzba::requires(true)]
#[bityzba::ensures(true)]
fn parse_schema(input: TokenStream) -> SchemaSummary {
    let mut root = Cursor::new(input);
    root.expect_ident("syntax_binding_schema");
    let body = root.take_group(Delimiter::Brace);
    root.finish();

    let mut body = Cursor::new(body);
    body.expect_ident("version");
    let mut version = Cursor::new(body.take_group(Delimiter::Parenthesis));
    assert_eq!(version.take_literal(), "1", "unsupported schema version");
    version.finish();
    body.expect_punct(',');

    body.expect_ident("models");
    let models = parse_models(body.take_group(Delimiter::Bracket));
    body.expect_punct(',');

    body.expect_ident("metadata");
    let mut summary = parse_metadata(body.take_group(Delimiter::Brace));
    body.finish();
    summary.models = models;
    summary
}

#[bityzba::requires(true)]
#[bityzba::ensures(true)]
fn parse_models(stream: TokenStream) -> Vec<ModelSchema> {
    let mut cursor = Cursor::new(stream);
    let mut models = Vec::new();
    while !cursor.is_done() {
        let kind = cursor.take_ident();
        let body = cursor.take_group(Delimiter::Brace);
        models.push(match kind.as_str() {
            "product" => parse_product(body),
            "sum" => parse_sum(body),
            _ => panic!("unknown schema model kind `{kind}`"),
        });
        if !cursor.is_done() {
            cursor.expect_punct(',');
        }
    }
    models
}

#[bityzba::requires(true)]
#[bityzba::ensures(true)]
fn parse_product(stream: TokenStream) -> ModelSchema {
    let mut cursor = Cursor::new(stream);
    let name = parse_names(&mut cursor);
    cursor.expect_punct(',');
    let rule = parse_string_property(&mut cursor, "rule");
    cursor.expect_punct(',');
    let has_nonblank_docs = parse_docs(&mut cursor);
    cursor.expect_punct(',');
    parse_constructor(&mut cursor, None);
    cursor.expect_punct(',');
    let shape = parse_ident_property(&mut cursor, "shape");
    cursor.expect_punct(',');
    let fields = parse_fields_property(&mut cursor);
    cursor.finish();
    new!(ModelSchema {
        name,
        rule,
        has_nonblank_docs,
        kind: new!(ModelKind::Product { shape }),
        fields,
        variants: Vec::new(),
    })
}

#[bityzba::requires(true)]
#[bityzba::ensures(true)]
fn parse_sum(stream: TokenStream) -> ModelSchema {
    let mut cursor = Cursor::new(stream);
    let name = parse_names(&mut cursor);
    cursor.expect_punct(',');
    let rule = parse_string_property(&mut cursor, "rule");
    cursor.expect_punct(',');
    let has_nonblank_docs = parse_docs(&mut cursor);
    cursor.expect_punct(',');
    parse_constructor(&mut cursor, None);
    cursor.expect_punct(',');
    cursor.expect_ident("variants");
    let variants = parse_variants(cursor.take_group(Delimiter::Bracket));
    cursor.finish();
    new!(ModelSchema {
        name,
        rule,
        has_nonblank_docs,
        kind: new!(ModelKind::Sum),
        fields: Vec::new(),
        variants,
    })
}

#[bityzba::requires(true)]
#[bityzba::ensures(true)]
fn parse_names(cursor: &mut Cursor) -> String {
    cursor.expect_ident("names");
    let mut names = Cursor::new(cursor.take_group(Delimiter::Parenthesis));
    let strict = parse_string_property(&mut names, "strict");
    names.expect_punct(',');
    let recovered = parse_string_property(&mut names, "recovered");
    names.finish();
    assert_eq!(strict, recovered, "strict and recovered model names differ");
    strict
}

#[bityzba::requires(true)]
#[bityzba::ensures(true)]
fn parse_variants(stream: TokenStream) -> Vec<VariantSchema> {
    let mut cursor = Cursor::new(stream);
    let mut variants = Vec::new();
    while !cursor.is_done() {
        cursor.expect_ident("variant");
        variants.push(parse_variant(cursor.take_group(Delimiter::Brace)));
        if !cursor.is_done() {
            cursor.expect_punct(',');
        }
    }
    variants
}

#[bityzba::requires(true)]
#[bityzba::ensures(true)]
fn parse_variant(stream: TokenStream) -> VariantSchema {
    let mut cursor = Cursor::new(stream);
    let name = parse_string_property(&mut cursor, "name");
    cursor.expect_punct(',');
    let _owner_rule = parse_string_property(&mut cursor, "owner_rule");
    cursor.expect_punct(',');
    let source_rule = parse_string_property(&mut cursor, "source_rule");
    cursor.expect_punct(',');
    let has_nonblank_docs = parse_docs(&mut cursor);
    cursor.expect_punct(',');
    parse_constructor(&mut cursor, Some(&name));
    cursor.expect_punct(',');
    let shape = parse_ident_property(&mut cursor, "shape");
    cursor.expect_punct(',');
    let fields = parse_fields_property(&mut cursor);
    cursor.finish();
    new!(VariantSchema {
        name,
        source_rule,
        has_nonblank_docs,
        shape,
        fields,
    })
}

#[bityzba::requires(true)]
#[bityzba::ensures(true)]
fn parse_constructor(cursor: &mut Cursor, expected_name: Option<&str>) {
    cursor.expect_ident("constructor");
    let mut constructor = Cursor::new(cursor.take_group(Delimiter::Parenthesis));
    let name = parse_string_property(&mut constructor, "name");
    if let Some(expected_name) = expected_name {
        assert_eq!(name, expected_name, "variant constructor name differs");
    }
    constructor.expect_punct(',');
    constructor.expect_ident("label");
    let mut label = Cursor::new(constructor.take_group(Delimiter::Parenthesis));
    match label.take_ident().as_str() {
        "some" => {
            let _ = parse_string_group(&mut label);
        }
        "none" => {}
        other => panic!("unknown constructor label shape `{other}`"),
    }
    label.finish();
    constructor.finish();
}

#[bityzba::requires(true)]
#[bityzba::ensures(true)]
fn parse_fields_property(cursor: &mut Cursor) -> Vec<FieldSchema> {
    cursor.expect_ident("fields");
    let mut fields = Cursor::new(cursor.take_group(Delimiter::Bracket));
    let mut parsed = Vec::new();
    while !fields.is_done() {
        fields.expect_ident("field");
        parsed.push(parse_field(fields.take_group(Delimiter::Brace)));
        if !fields.is_done() {
            fields.expect_punct(',');
        }
    }
    parsed
}

#[bityzba::requires(true)]
#[bityzba::ensures(true)]
fn parse_field(stream: TokenStream) -> FieldSchema {
    let mut cursor = Cursor::new(stream);
    let source_name = parse_string_property(&mut cursor, "source_name");
    cursor.expect_punct(',');
    parse_rust_name(&mut cursor, &source_name);
    cursor.expect_punct(',');
    let index = parse_usize_property(&mut cursor, "index");
    cursor.expect_punct(',');
    let _ = parse_docs(&mut cursor);
    cursor.expect_punct(',');
    let strict = parse_type_property(&mut cursor, "strict");
    cursor.expect_punct(',');
    let recovered = parse_type_property(&mut cursor, "recovered");
    cursor.finish();
    new!(FieldSchema {
        source_name,
        index,
        strict,
        recovered,
    })
}

#[bityzba::requires(!source_name.is_empty())]
#[bityzba::ensures(true)]
fn parse_rust_name(cursor: &mut Cursor, source_name: &str) {
    cursor.expect_ident("rust_name");
    let mut rust_name = Cursor::new(cursor.take_group(Delimiter::Parenthesis));
    match rust_name.take_ident().as_str() {
        "named" => assert_eq!(parse_string_group(&mut rust_name), source_name),
        "tuple" => {
            let _ = parse_usize_group(&mut rust_name);
        }
        other => panic!("unknown Rust field-name shape `{other}`"),
    }
    rust_name.finish();
}

#[bityzba::requires(true)]
#[bityzba::ensures(true)]
fn parse_docs(cursor: &mut Cursor) -> bool {
    cursor.expect_ident("docs");
    let mut docs = Cursor::new(cursor.take_group(Delimiter::Bracket));
    let mut has_nonblank = false;
    while !docs.is_done() {
        let literal = docs.take_literal();
        assert!(
            is_string_literal(&literal),
            "documentation is not a string literal"
        );
        has_nonblank |= !string_literal_contents(&literal).trim().is_empty();
        if !docs.is_done() {
            docs.expect_punct(',');
        }
    }
    has_nonblank
}

#[bityzba::requires(true)]
#[bityzba::ensures(true)]
fn parse_type_property(cursor: &mut Cursor, name: &str) -> BindingType {
    cursor.expect_ident(name);
    parse_binding_type(cursor.take_group(Delimiter::Parenthesis))
}

#[bityzba::requires(true)]
#[bityzba::ensures(true)]
fn parse_binding_type(stream: TokenStream) -> BindingType {
    let mut cursor = Cursor::new(stream);
    let kind = cursor.take_ident();
    let args = cursor.take_group(Delimiter::Parenthesis);
    cursor.finish();
    match kind.as_str() {
        "reference" => parse_reference(args),
        "optional" => unary_type(args, |value| new!(BindingType::Optional { value })),
        "repeated" => unary_type(args, |value| new!(BindingType::Repeated { value })),
        "non_empty_repeated" => {
            unary_type(args, |value| new!(BindingType::NonEmptyRepeated { value }))
        }
        "boxed" => unary_type(args, |value| new!(BindingType::Boxed { value })),
        "shared" => unary_type(args, |value| new!(BindingType::Shared { value })),
        "recovered_field" => unary_type(args, |value| new!(BindingType::RecoveredField { value })),
        "with_indicators" => unary_type(args, |value| new!(BindingType::WithIndicators { value })),
        "with_free_modifiers" => parse_with_free_modifiers(args),
        "chain" => parse_chain(args),
        "tuple" => new!(BindingType::Tuple {
            elements: parse_type_list(args),
        }),
        "fixed" => parse_fixed(args),
        _ => panic!("unknown normalized binding type `{kind}`"),
    }
}

#[bityzba::requires(true)]
#[bityzba::ensures(true)]
fn unary_type(
    stream: TokenStream,
    constructor: fn(Box<BindingType>) -> BindingType,
) -> BindingType {
    constructor(Box::new(parse_binding_type(stream)))
}

#[bityzba::requires(true)]
#[bityzba::ensures(true)]
fn parse_reference(stream: TokenStream) -> BindingType {
    let mut cursor = Cursor::new(stream);
    let kind = cursor.take_ident();
    let args = cursor.take_group(Delimiter::Parenthesis);
    cursor.finish();
    match kind.as_str() {
        "model" => {
            let mut args = Cursor::new(args);
            let name = parse_string_literal(&mut args);
            args.finish();
            new!(BindingType::ModelReference { name })
        }
        "leaf" => {
            let mut args = Cursor::new(args);
            let kind = parse_ident_property(&mut args, "kind");
            args.expect_punct(',');
            args.expect_ident("path");
            let path = parse_string_list(args.take_group(Delimiter::Parenthesis));
            args.finish();
            new!(BindingType::LeafReference { kind, path })
        }
        _ => panic!("unknown reference kind `{kind}`"),
    }
}

#[bityzba::requires(true)]
#[bityzba::ensures(true)]
fn parse_with_free_modifiers(stream: TokenStream) -> BindingType {
    let mut cursor = Cursor::new(stream);
    let value = Box::new(parse_type_property(&mut cursor, "value"));
    cursor.expect_punct(',');
    let property = cursor.take_ident();
    assert!(
        property == "free_modifier" || property == "free_modifiers",
        "unknown free-modifier property"
    );
    let free_modifiers = Box::new(parse_binding_type(
        cursor.take_group(Delimiter::Parenthesis),
    ));
    cursor.finish();
    new!(BindingType::WithFreeModifiers {
        value,
        free_modifiers,
    })
}

#[bityzba::requires(true)]
#[bityzba::ensures(true)]
fn parse_chain(stream: TokenStream) -> BindingType {
    let mut cursor = Cursor::new(stream);
    let first = Box::new(parse_type_property(&mut cursor, "first"));
    cursor.expect_punct(',');
    let links = Box::new(parse_type_property(&mut cursor, "links"));
    cursor.finish();
    new!(BindingType::Chain { first, links })
}

#[bityzba::requires(true)]
#[bityzba::ensures(true)]
fn parse_fixed(stream: TokenStream) -> BindingType {
    let mut cursor = Cursor::new(stream);
    let length = parse_usize_property(&mut cursor, "length");
    cursor.expect_punct(',');
    let value = Box::new(parse_type_property(&mut cursor, "value"));
    cursor.finish();
    new!(BindingType::Fixed { length, value })
}

#[bityzba::requires(true)]
#[bityzba::ensures(true)]
fn parse_type_list(stream: TokenStream) -> Vec<BindingType> {
    let mut cursor = Cursor::new(stream);
    let mut types = Vec::new();
    while !cursor.is_done() {
        let kind = cursor.take_ident();
        let args = cursor.take_group(Delimiter::Parenthesis);
        let mut expression = TokenStream::new();
        expression.extend([
            TokenTree::Ident(proc_macro::Ident::new(&kind, proc_macro::Span::call_site())),
            TokenTree::Group(proc_macro::Group::new(Delimiter::Parenthesis, args)),
        ]);
        types.push(parse_binding_type(expression));
        if !cursor.is_done() {
            cursor.expect_punct(',');
        }
    }
    types
}

#[bityzba::requires(true)]
#[bityzba::ensures(true)]
fn parse_metadata(stream: TokenStream) -> SchemaSummary {
    let mut cursor = Cursor::new(stream);
    let transparent_constructors =
        parse_literal_list_property(&mut cursor, "transparent_constructors");
    cursor.expect_punct(',');
    let transparent_fields =
        parse_call_list_property(&mut cursor, "transparent_fields", "transparent_field");
    cursor.expect_punct(',');
    let chain_link_element_fields = parse_call_list_property(
        &mut cursor,
        "chain_link_element_fields",
        "chain_link_element_field",
    );
    cursor.expect_punct(',');
    let constructor_labels =
        parse_call_list_property(&mut cursor, "constructor_labels", "constructor_label");
    cursor.expect_punct(',');
    let elidable_terminators =
        parse_call_list_property(&mut cursor, "elidable_terminators", "elidable_terminator");
    cursor.expect_punct(',');
    let field_orders = parse_call_list_property(&mut cursor, "field_orders", "field_order");
    cursor.finish();
    SchemaSummary {
        models: Vec::new(),
        transparent_constructors,
        transparent_fields,
        chain_link_element_fields,
        constructor_labels,
        elidable_terminators,
        field_orders,
    }
}

#[bityzba::requires(true)]
#[bityzba::ensures(true)]
fn parse_literal_list_property(cursor: &mut Cursor, name: &str) -> usize {
    cursor.expect_ident(name);
    parse_string_list(cursor.take_group(Delimiter::Bracket)).len()
}

#[bityzba::requires(true)]
#[bityzba::ensures(true)]
fn parse_call_list_property(cursor: &mut Cursor, name: &str, call: &str) -> usize {
    cursor.expect_ident(name);
    let mut values = Cursor::new(cursor.take_group(Delimiter::Bracket));
    let mut count = 0;
    while !values.is_done() {
        values.expect_ident(call);
        let args = values.take_group(Delimiter::Parenthesis);
        assert!(!args.is_empty(), "metadata call has no arguments");
        count += 1;
        if !values.is_done() {
            values.expect_punct(',');
        }
    }
    count
}

#[bityzba::requires(true)]
#[bityzba::ensures(true)]
fn parse_string_list(stream: TokenStream) -> Vec<String> {
    let mut cursor = Cursor::new(stream);
    let mut strings = Vec::new();
    while !cursor.is_done() {
        strings.push(parse_string_literal(&mut cursor));
        if !cursor.is_done() {
            cursor.expect_punct(',');
        }
    }
    strings
}

#[bityzba::requires(true)]
#[bityzba::ensures(!ret.is_empty())]
fn parse_string_property(cursor: &mut Cursor, name: &str) -> String {
    cursor.expect_ident(name);
    parse_string_group(cursor)
}

#[bityzba::requires(true)]
#[bityzba::ensures(!ret.is_empty())]
fn parse_string_group(cursor: &mut Cursor) -> String {
    let mut value = Cursor::new(cursor.take_group(Delimiter::Parenthesis));
    let parsed = parse_string_literal(&mut value);
    value.finish();
    parsed
}

#[bityzba::requires(true)]
#[bityzba::ensures(!ret.is_empty())]
fn parse_string_literal(cursor: &mut Cursor) -> String {
    let literal = cursor.take_literal();
    assert!(is_string_literal(&literal), "expected a string literal");
    string_literal_contents(&literal).to_owned()
}

#[bityzba::requires(true)]
#[bityzba::ensures(true)]
fn is_string_literal(literal: &str) -> bool {
    literal.len() >= 2 && literal.starts_with('"') && literal.ends_with('"')
}

#[bityzba::requires(is_string_literal(literal))]
#[bityzba::ensures(true)]
fn string_literal_contents(literal: &str) -> &str {
    &literal[1..literal.len() - 1]
}

#[bityzba::requires(true)]
#[bityzba::ensures(!ret.is_empty())]
fn parse_ident_property(cursor: &mut Cursor, name: &str) -> String {
    cursor.expect_ident(name);
    let mut value = Cursor::new(cursor.take_group(Delimiter::Parenthesis));
    let parsed = value.take_ident();
    value.finish();
    parsed
}

#[bityzba::requires(true)]
#[bityzba::ensures(true)]
fn parse_usize_property(cursor: &mut Cursor, name: &str) -> usize {
    cursor.expect_ident(name);
    parse_usize_group(cursor)
}

#[bityzba::requires(true)]
#[bityzba::ensures(true)]
fn parse_usize_group(cursor: &mut Cursor) -> usize {
    let mut value = Cursor::new(cursor.take_group(Delimiter::Parenthesis));
    let literal = value.take_literal();
    let parsed = literal
        .strip_suffix("usize")
        .unwrap_or(&literal)
        .parse()
        .expect("schema integer must fit usize");
    value.finish();
    parsed
}

#[bityzba::requires(true)]
#[bityzba::ensures(true)]
fn validate_schema(summary: &SchemaSummary) {
    assert!(!summary.models.is_empty(), "schema has no models");
    let mut names = BTreeSet::new();
    for model in &summary.models {
        assert!(names.insert(&model.name), "duplicate schema model name");
        assert!(
            model.has_nonblank_docs,
            "model lacks canonical documentation"
        );
        assert!(
            model
                .kind
                .matches_model_shape(model.fields.len(), model.variants.len()),
            "model payload differs from its declared kind"
        );
        match model.kind.as_data() {
            data!(ModelKind::Product { shape }) => {
                assert!(model.variants.is_empty());
                assert_eq!(shape == "tuple", model.fields.len() == 1);
                validate_fields(&model.fields);
            }
            data!(ModelKind::Sum) => {
                assert!(model.fields.is_empty());
                assert!(!model.variants.is_empty());
                for variant in &model.variants {
                    assert!(variant.has_nonblank_docs, "variant lacks documentation");
                    assert_eq!(variant.shape, "tuple");
                    validate_fields(&variant.fields);
                }
            }
        }
    }

    let leading = model_by_name(summary, "LeadingIndicatorSyntax");
    assert_eq!(leading.rule, "leading_indicator");
    assert_eq!(field_names(&leading.fields), ["indicator", "nai"]);
    assert_eq!(leading.fields[0].strict, syntax_token());
    assert_eq!(
        leading.fields[1].strict,
        new!(BindingType::Optional {
            value: Box::new(syntax_token()),
        })
    );
    assert_eq!(
        leading.fields[1].recovered,
        new!(BindingType::Optional {
            value: Box::new(new!(BindingType::RecoveredField {
                value: Box::new(syntax_token()),
            })),
        })
    );

    let text = model_by_name(summary, "TextSyntax");
    assert_eq!(
        text.variants
            .iter()
            .map(|variant| variant.source_rule.as_str())
            .collect::<Vec<_>>(),
        ["explicit_xauha_lohoi_text", "regular_text"]
    );

    let regular = model_by_name(summary, "RegularTextSyntax");
    assert_eq!(
        field_names(&regular.fields),
        [
            "leading_nai",
            "leading_cmevla",
            "leading_indicators",
            "leading_free_modifiers",
            "leading_connective",
            "leading_i_statements",
            "paragraphs",
        ]
    );
    assert_eq!(
        regular.fields[0].strict,
        new!(BindingType::Repeated {
            value: Box::new(syntax_token()),
        })
    );
    assert_eq!(
        regular.fields[6].strict,
        new!(BindingType::Optional {
            value: Box::new(new!(BindingType::Shared {
                value: Box::new(new!(BindingType::ModelReference {
                    name: "TextParagraphsSyntax".to_owned(),
                })),
            })),
        })
    );

    assert!(summary.transparent_constructors > 0);
    assert!(summary.transparent_fields > 0);
    assert!(summary.chain_link_element_fields > 0);
    assert!(summary.constructor_labels > 0);
    assert!(summary.elidable_terminators > 0);
    assert!(summary.field_orders > 0);
}

#[bityzba::requires(true)]
#[bityzba::ensures(true)]
fn validate_fields(fields: &[FieldSchema]) {
    for (index, field) in fields.iter().enumerate() {
        assert_eq!(field.index, index, "schema field index is out of order");
    }
}

#[bityzba::requires(!name.is_empty())]
#[bityzba::ensures(ret.name == name)]
fn model_by_name<'a>(summary: &'a SchemaSummary, name: &str) -> &'a ModelSchema {
    summary
        .models
        .iter()
        .find(|model| model.name == name)
        .expect("representative schema model is missing")
}

#[bityzba::requires(true)]
#[bityzba::ensures(ret.len() == fields.len())]
fn field_names(fields: &[FieldSchema]) -> Vec<&str> {
    fields
        .iter()
        .map(|field| field.source_name.as_str())
        .collect()
}

#[bityzba::requires(true)]
#[bityzba::ensures(true)]
fn syntax_token() -> BindingType {
    new!(BindingType::LeafReference {
        kind: "syntax_token".to_owned(),
        path: vec!["Token".to_owned()],
    })
}

#[bityzba::requires(true)]
#[bityzba::ensures(true)]
#[proc_macro]
pub fn consume_syntax_binding_schema(input: TokenStream) -> TokenStream {
    let summary = parse_schema(input);
    validate_schema(&summary);
    let products = summary
        .models
        .iter()
        .filter(|model| matches!(model.kind.as_data(), data!(ModelKind::Product { .. })))
        .count();
    let sums = summary.models.len() - products;
    let variants = summary
        .models
        .iter()
        .map(|model| model.variants.len())
        .sum::<usize>();
    let fields = summary
        .models
        .iter()
        .map(|model| {
            model.fields.len()
                + model
                    .variants
                    .iter()
                    .map(|variant| variant.fields.len())
                    .sum::<usize>()
        })
        .sum::<usize>();

    format!(
        "const EXTERNAL_SCHEMA_PRODUCT_COUNT: usize = {products};\n\
         const EXTERNAL_SCHEMA_SUM_COUNT: usize = {sums};\n\
         const EXTERNAL_SCHEMA_VARIANT_COUNT: usize = {variants};\n\
         const EXTERNAL_SCHEMA_FIELD_COUNT: usize = {fields};\n\
         const EXTERNAL_SCHEMA_REPRESENTATIVE_RECORDS_VALID: bool = true;",
    )
    .parse()
    .expect("generated schema summary constants must parse")
}
