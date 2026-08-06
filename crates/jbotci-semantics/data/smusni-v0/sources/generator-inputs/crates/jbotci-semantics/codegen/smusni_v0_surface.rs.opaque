//! Exact semantic-coordinate scan for the smusni-v0 disposition ledger.
//!
//! This scanner deliberately works from the Rust semantic model rather than a
//! fixture corpus. Every public semantic data type is either projected into
//! coordinates or appears in the exact checked exclusion set below. Object
//! nodes are projected through their flattened `SemanticObject` surface, while
//! their branch constructors remain distinct coordinates.

#![allow(dead_code)]

use std::collections::{BTreeMap, BTreeSet};

#[allow(unused_imports)]
use bityzba::{ensures, invariant, new, requires};
use syn::{Attribute, Fields, Item, ItemEnum, ItemStruct, LitStr, Meta, Visibility};

const MODEL_RS: &str = include_str!("../src/model.rs");
const SEMANTIC_OBJECT_RS: &str = include_str!("../src/model/semantic_object.rs");

const OBJECT_SURFACES: &[&str] = &[
    "Utterance",
    "Sequence",
    "Eventuality",
    "Referent",
    "Parameter",
    "Predication",
    "Formula",
    "Sign",
    "DisplayedContent",
    "MathExpression",
    "Quantity",
    "RelationMetadata",
    "Question",
];

const OBJECT_NODE_TYPES: &[(&str, &str)] = &[
    ("UtteranceNode", "Utterance"),
    ("SequenceNode", "Sequence"),
    ("EventualityNode", "Eventuality"),
    ("ReferentNode", "Referent"),
    ("ParameterNode", "Parameter"),
    ("PredicationNode", "Predication"),
    ("SignNode", "Sign"),
    ("DisplayedContentNode", "DisplayedContent"),
    ("MathExpressionNode", "MathExpression"),
    ("QuantityNode", "Quantity"),
    ("RelationMetadataNode", "RelationMetadata"),
    ("QuestionNode", "Question"),
];

const FORMULA_NODE_TYPES: &[(&str, &str)] = &[
    ("AtomFormulaNode", "FormulaNode::Atom"),
    ("ConnectiveFormulaNode", "FormulaNode::Connective"),
    ("QuantifiedFormulaNode", "FormulaNode::Quantified"),
    (
        "QuantifierBundleFormulaNode",
        "FormulaNode::QuantifierBundle",
    ),
    (
        "RespectivelyDistributionFormulaNode",
        "FormulaNode::RespectivelyDistribution",
    ),
];

const EXACT_EXCLUSIONS: &[(&str, &str)] = &[
    (
        "FormulaTraversal",
        "read-only traversal projection; it carries no graph-owned semantic datum",
    ),
    (
        "SemanticGraphError",
        "construction error family; unsuccessful graphs never enter semantic projection",
    ),
];

/// The closed owner category used by the authored and minted ledgers.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CoordinateCategory {
    Object,
    ValueStruct,
    Enum,
    Document,
}

/// The semantic role of one coordinate.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CoordinateKind {
    Constructor,
    Discriminator,
    Field,
    EnumVariant,
    VariantField,
    DerivedFact,
}

/// One exact semantic-surface coordinate.
#[invariant(!surface.is_empty() && !member.is_empty())]
#[invariant(qualifier.as_ref().is_none_or(|value| !value.is_empty()))]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SemanticCoordinate {
    pub category: CoordinateCategory,
    pub surface: String,
    pub kind: CoordinateKind,
    pub member: String,
    pub qualifier: Option<String>,
}

impl SemanticCoordinate {
    #[requires(true)]
    #[requires(qualifier.as_ref().is_none_or(|value| !value.is_empty()))]
    #[ensures(!ret.surface.is_empty() && !ret.member.is_empty())]
    pub fn new(
        category: CoordinateCategory,
        surface: impl Into<String>,
        kind: CoordinateKind,
        member: impl Into<String>,
        qualifier: Option<String>,
    ) -> Self {
        new!(SemanticCoordinate {
            category,
            surface: surface.into(),
            kind,
            member: member.into(),
            qualifier,
        })
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn owner(&self) -> String {
        let category = match self.category {
            CoordinateCategory::Object => "Object",
            CoordinateCategory::ValueStruct => "ValueStruct",
            CoordinateCategory::Enum => "Enum",
            CoordinateCategory::Document => "Document",
        };
        let kind = match self.kind {
            CoordinateKind::Constructor => "Constructor",
            CoordinateKind::Discriminator => "Discriminator",
            CoordinateKind::Field => "Field",
            CoordinateKind::EnumVariant => "EnumVariant",
            CoordinateKind::VariantField => "VariantField",
            CoordinateKind::DerivedFact => "DerivedFact",
        };
        let member = self.qualifier.as_ref().map_or_else(
            || self.member.clone(),
            |qualifier| format!("{}@{qualifier}", self.member),
        );
        format!("{category}:{}:{kind}:{member}", self.surface)
    }
}

/// One explicitly excluded public model type.
#[invariant(!type_name.is_empty() && !reason.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SurfaceExclusion {
    pub type_name: String,
    pub reason: String,
}

/// Exact scan result. Public model types cannot disappear between coordinates
/// and exclusions because `accounted_types` is checked during construction.
#[invariant(!coordinates.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticSurfaceScan {
    pub coordinates: BTreeSet<SemanticCoordinate>,
    pub exclusions: BTreeSet<SurfaceExclusion>,
}

/// Scan both semantic-model source modules and append the closed derived-fact
/// coordinate family required by the projection contract.
#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|scan| !scan.coordinates.is_empty()) || ret.is_err())]
pub fn scan_semantic_surface() -> Result<SemanticSurfaceScan, String> {
    let files = [
        syn::parse_file(MODEL_RS).map_err(|error| error.to_string())?,
        syn::parse_file(SEMANTIC_OBJECT_RS).map_err(|error| error.to_string())?,
    ];
    let mut structs = BTreeMap::<String, ItemStruct>::new();
    let mut enums = BTreeMap::<String, ItemEnum>::new();
    for file in files {
        for item in file.items {
            match item {
                Item::Struct(item) if is_public(&item.vis) => {
                    structs.insert(item.ident.to_string(), item);
                }
                Item::Enum(item) if is_public(&item.vis) => {
                    enums.insert(item.ident.to_string(), item);
                }
                _ => {}
            }
        }
    }

    let public_types = structs
        .keys()
        .chain(enums.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut accounted_types = BTreeSet::new();
    let mut coordinates = BTreeSet::new();

    add_object_coordinates(&structs, &enums, &mut coordinates, &mut accounted_types)?;

    for (name, item) in &structs {
        if accounted_types.contains(name) || excluded_reason(name).is_some() {
            continue;
        }
        accounted_types.insert(name.clone());
        let (category, surface) = if name == "SemanticGraph" {
            (CoordinateCategory::Document, "semantic-graph")
        } else {
            (CoordinateCategory::ValueStruct, name.as_str())
        };
        add_struct_fields(category, surface, &item.fields, None, &mut coordinates)?;
    }

    for (name, item) in &enums {
        if accounted_types.contains(name) || excluded_reason(name).is_some() {
            continue;
        }
        accounted_types.insert(name.clone());
        add_enum_coordinates(name, item, &mut coordinates)?;
    }

    let exclusions = EXACT_EXCLUSIONS
        .iter()
        .map(|(type_name, reason)| {
            new!(SurfaceExclusion {
                type_name: (*type_name).to_owned(),
                reason: (*reason).to_owned(),
            })
        })
        .collect::<BTreeSet<_>>();
    accounted_types.extend(exclusions.iter().map(|row| row.type_name.clone()));
    if accounted_types != public_types {
        let missing = public_types
            .difference(&accounted_types)
            .cloned()
            .collect::<Vec<_>>();
        let stale = accounted_types
            .difference(&public_types)
            .cloned()
            .collect::<Vec<_>>();
        return Err(format!(
            "semantic type accounting differs: missing={missing:?}, stale={stale:?}"
        ));
    }

    coordinates.extend(derived_fact_coordinates());
    Ok(new!(SemanticSurfaceScan {
        coordinates,
        exclusions,
    }))
}

#[requires(true)]
#[ensures(ret == matches!(visibility, Visibility::Public(_)))]
fn is_public(visibility: &Visibility) -> bool {
    matches!(visibility, Visibility::Public(_))
}

#[requires(true)]
#[ensures(ret.is_none_or(|reason| !reason.is_empty()))]
fn excluded_reason(name: &str) -> Option<&'static str> {
    EXACT_EXCLUSIONS
        .iter()
        .find_map(|(candidate, reason)| (*candidate == name).then_some(*reason))
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn add_object_coordinates(
    structs: &BTreeMap<String, ItemStruct>,
    enums: &BTreeMap<String, ItemEnum>,
    coordinates: &mut BTreeSet<SemanticCoordinate>,
    accounted_types: &mut BTreeSet<String>,
) -> Result<(), String> {
    let semantic_object = enums
        .get("SemanticObject")
        .ok_or_else(|| "SemanticObject enum disappeared".to_owned())?;
    accounted_types.insert("SemanticObject".to_owned());
    for variant in &semantic_object.variants {
        let name = variant.ident.to_string();
        coordinates.insert(SemanticCoordinate::new(
            CoordinateCategory::Object,
            "SemanticObject",
            CoordinateKind::Constructor,
            name,
            None,
        ));
    }

    let common = structs
        .get("SemanticObjectCommon")
        .ok_or_else(|| "SemanticObjectCommon disappeared".to_owned())?;
    accounted_types.insert("SemanticObjectCommon".to_owned());
    for surface in OBJECT_SURFACES {
        coordinates.insert(SemanticCoordinate::new(
            CoordinateCategory::Object,
            *surface,
            CoordinateKind::Discriminator,
            "type",
            None,
        ));
        add_struct_fields(
            CoordinateCategory::Object,
            surface,
            &common.fields,
            None,
            coordinates,
        )?;
    }

    for (node_name, surface) in OBJECT_NODE_TYPES {
        let node = structs
            .get(*node_name)
            .ok_or_else(|| format!("{node_name} disappeared"))?;
        accounted_types.insert((*node_name).to_owned());
        for field in required_named_fields(&node.fields)? {
            let rust_name = field.ident.as_ref().expect("named field").to_string();
            if rust_name == "common"
                || matches!(
                    (*surface, rust_name.as_str()),
                    ("Predication", "relation" | "tanru_link") | ("MathExpression", "kind")
                )
            {
                continue;
            }
            let member = if *surface == "Sign" && rust_name == "sign_kind" {
                "kind".to_owned()
            } else {
                serde_field_name(field)?
            };
            coordinates.insert(SemanticCoordinate::new(
                CoordinateCategory::Object,
                *surface,
                CoordinateKind::Field,
                member,
                None,
            ));
        }
    }

    for (node_name, qualifier) in FORMULA_NODE_TYPES {
        let node = structs
            .get(*node_name)
            .ok_or_else(|| format!("{node_name} disappeared"))?;
        accounted_types.insert((*node_name).to_owned());
        for field in required_named_fields(&node.fields)? {
            let rust_name = field.ident.as_ref().expect("named field").to_string();
            if rust_name == "common" {
                continue;
            }
            coordinates.insert(SemanticCoordinate::new(
                CoordinateCategory::Object,
                "Formula",
                CoordinateKind::VariantField,
                serde_field_name(field)?,
                Some((*qualifier).to_owned()),
            ));
        }
    }

    // The flat serializer materializes these semantic facts in addition to raw
    // node fields. Their branch identity must not be collapsed.
    for (member, qualifier) in [
        ("operator", "FormulaNode::Atom"),
        ("operator", "FormulaNode::QuantifierBundle"),
        ("coequalScope", "FormulaNode::QuantifierBundle"),
        ("operator", "FormulaNode::RespectivelyDistribution"),
        ("domainImport", "FormulaNode::Quantified"),
    ] {
        coordinates.insert(SemanticCoordinate::new(
            CoordinateCategory::Object,
            "Formula",
            CoordinateKind::VariantField,
            member,
            Some(qualifier.to_owned()),
        ));
    }
    for (surface, member) in [
        ("Eventuality", "category"),
        ("Eventuality", "scopeDependence"),
        ("Sign", "sort"),
    ] {
        coordinates.insert(SemanticCoordinate::new(
            CoordinateCategory::Object,
            surface,
            CoordinateKind::Field,
            member,
            None,
        ));
    }
    coordinates.insert(SemanticCoordinate::new(
        CoordinateCategory::Object,
        "Predication",
        CoordinateKind::VariantField,
        "tanruLink",
        Some("PredicationRelation::Composition".to_owned()),
    ));

    for dispatch in [
        "PredicationRelation",
        "FormulaNode",
        "MathExpressionNodeKind",
    ] {
        let item = enums
            .get(dispatch)
            .ok_or_else(|| format!("{dispatch} disappeared"))?;
        accounted_types.insert(dispatch.to_owned());
        add_enum_constructors(dispatch, item, coordinates)?;
    }
    add_flattened_enum_fields(
        "PredicationRelation",
        enums.get("PredicationRelation").expect("checked above"),
        "Predication",
        coordinates,
    )?;
    add_flattened_enum_fields(
        "MathExpressionNodeKind",
        enums.get("MathExpressionNodeKind").expect("checked above"),
        "MathExpression",
        coordinates,
    )?;
    Ok(())
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn add_enum_constructors(
    name: &str,
    item: &ItemEnum,
    coordinates: &mut BTreeSet<SemanticCoordinate>,
) -> Result<(), String> {
    for variant in &item.variants {
        coordinates.insert(SemanticCoordinate::new(
            CoordinateCategory::Enum,
            name,
            CoordinateKind::Constructor,
            variant.ident.to_string(),
            None,
        ));
    }
    Ok(())
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn add_flattened_enum_fields(
    enum_name: &str,
    item: &ItemEnum,
    object_surface: &str,
    coordinates: &mut BTreeSet<SemanticCoordinate>,
) -> Result<(), String> {
    for variant in &item.variants {
        let qualifier = format!("{enum_name}::{}", variant.ident);
        let Fields::Named(fields) = &variant.fields else {
            continue;
        };
        for field in &fields.named {
            let rust_name = field.ident.as_ref().expect("named field").to_string();
            let member = match (
                enum_name,
                variant.ident.to_string().as_str(),
                rust_name.as_str(),
            ) {
                ("PredicationRelation", "Parameter", "parameter") => "relationParameter".to_owned(),
                ("MathExpressionNodeKind", "Operator", "operator_denotes") => {
                    "operatorDenotes".to_owned()
                }
                ("MathExpressionNodeKind", "Operator", "endpoint_inclusion") => {
                    "endpointInclusion".to_owned()
                }
                ("MathExpressionNodeKind", "QuestionedOperator", "operator_parameter") => {
                    "operatorParameter".to_owned()
                }
                _ => serde_field_name(field)?,
            };
            coordinates.insert(SemanticCoordinate::new(
                CoordinateCategory::Object,
                object_surface,
                CoordinateKind::VariantField,
                member,
                Some(qualifier.clone()),
            ));
        }
    }
    Ok(())
}

#[requires(!name.is_empty())]
#[ensures(ret.is_ok() || ret.is_err())]
fn add_enum_coordinates(
    name: &str,
    item: &ItemEnum,
    coordinates: &mut BTreeSet<SemanticCoordinate>,
) -> Result<(), String> {
    let serde = serde_container_options(&item.attrs)?;
    if let Some(tag) = &serde.tag {
        coordinates.insert(SemanticCoordinate::new(
            CoordinateCategory::ValueStruct,
            name,
            CoordinateKind::Discriminator,
            tag.clone(),
            None,
        ));
    }
    for variant in &item.variants {
        let constructor_kind = if matches!(variant.fields, Fields::Unit) {
            CoordinateKind::EnumVariant
        } else {
            CoordinateKind::Constructor
        };
        coordinates.insert(SemanticCoordinate::new(
            CoordinateCategory::Enum,
            name,
            constructor_kind,
            variant.ident.to_string(),
            None,
        ));
        let qualifier = variant.ident.to_string();
        match &variant.fields {
            Fields::Named(fields) => {
                for field in &fields.named {
                    coordinates.insert(SemanticCoordinate::new(
                        CoordinateCategory::ValueStruct,
                        name,
                        CoordinateKind::VariantField,
                        serde_field_name(field)?,
                        Some(qualifier.clone()),
                    ));
                }
            }
            Fields::Unnamed(fields) if !fields.unnamed.is_empty() => {
                if let Some(content) = &serde.content {
                    coordinates.insert(SemanticCoordinate::new(
                        CoordinateCategory::ValueStruct,
                        name,
                        CoordinateKind::VariantField,
                        content.clone(),
                        Some(qualifier),
                    ));
                }
            }
            Fields::Unit | Fields::Unnamed(_) => {}
        }
    }
    Ok(())
}

#[requires(!surface.is_empty())]
#[ensures(ret.is_ok() || ret.is_err())]
fn add_struct_fields(
    category: CoordinateCategory,
    surface: &str,
    fields: &Fields,
    qualifier: Option<&str>,
    coordinates: &mut BTreeSet<SemanticCoordinate>,
) -> Result<(), String> {
    match fields {
        Fields::Named(_) => {
            for field in required_named_fields(fields)? {
                coordinates.insert(SemanticCoordinate::new(
                    category,
                    surface,
                    qualifier.map_or(CoordinateKind::Field, |_| CoordinateKind::VariantField),
                    serde_field_name(field)?,
                    qualifier.map(str::to_owned),
                ));
            }
        }
        Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
            coordinates.insert(SemanticCoordinate::new(
                category,
                surface,
                CoordinateKind::Field,
                "value",
                qualifier.map(str::to_owned),
            ));
        }
        Fields::Unit => {}
        Fields::Unnamed(_) => {
            return Err(format!(
                "tuple struct {surface} needs explicit semantic field names"
            ));
        }
    }
    Ok(())
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn required_named_fields(
    fields: &Fields,
) -> Result<&syn::punctuated::Punctuated<syn::Field, syn::token::Comma>, String> {
    match fields {
        Fields::Named(fields) => Ok(&fields.named),
        Fields::Unit | Fields::Unnamed(_) => Err("expected named semantic fields".to_owned()),
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|name| !name.is_empty()) || ret.is_err())]
fn serde_field_name(field: &syn::Field) -> Result<String, String> {
    let ident = field
        .ident
        .as_ref()
        .ok_or_else(|| "semantic field has no name".to_owned())?;
    let mut rename = None;
    for attribute in &field.attrs {
        if !attribute.path().is_ident("serde") {
            continue;
        }
        attribute
            .parse_nested_meta(|meta| {
                if meta.path.is_ident("rename") {
                    rename = Some(meta.value()?.parse::<LitStr>()?.value());
                } else if meta.input.peek(syn::Token![=]) {
                    let _ = meta.value()?.parse::<syn::Expr>()?;
                }
                Ok(())
            })
            .map_err(|error| error.to_string())?;
    }
    Ok(rename.unwrap_or_else(|| camel_case(&ident.to_string())))
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct SerdeContainerOptions {
    tag: Option<String>,
    content: Option<String>,
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn serde_container_options(attributes: &[Attribute]) -> Result<SerdeContainerOptions, String> {
    let mut tag = None;
    let mut content = None;
    for attribute in attributes {
        if !attribute.path().is_ident("serde") {
            continue;
        }
        let Meta::List(_) = &attribute.meta else {
            continue;
        };
        attribute
            .parse_nested_meta(|meta| {
                if meta.path.is_ident("tag") {
                    tag = Some(meta.value()?.parse::<LitStr>()?.value());
                } else if meta.path.is_ident("content") {
                    content = Some(meta.value()?.parse::<LitStr>()?.value());
                } else if meta.input.peek(syn::Token![=]) {
                    let _ = meta.value()?.parse::<syn::Expr>()?;
                }
                Ok(())
            })
            .map_err(|error| error.to_string())?;
    }
    Ok(SerdeContainerOptions { tag, content })
}

#[requires(true)]
#[ensures(true)]
fn camel_case(snake: &str) -> String {
    let mut output = String::new();
    let mut uppercase = false;
    for character in snake.chars() {
        if character == '_' {
            uppercase = true;
        } else if uppercase {
            output.extend(character.to_uppercase());
            uppercase = false;
        } else {
            output.push(character);
        }
    }
    output
}

/// Closed non-model facts which need dispositions and, for failure facts,
/// exact fallback-reason joins before planning begins.
#[requires(true)]
#[ensures(!ret.is_empty())]
fn derived_fact_coordinates() -> BTreeSet<SemanticCoordinate> {
    let mut rows = BTreeSet::new();
    for member in [
        "declarations",
        "dense-declaration-one-lining",
        "fact:role-binding-role-for",
        "fact:role-composition-all-hold",
        "id-prefixes-legend",
        "root",
        "short-id-assignment",
    ] {
        rows.insert(SemanticCoordinate::new(
            CoordinateCategory::Document,
            "document",
            CoordinateKind::DerivedFact,
            member,
            None,
        ));
    }
    for (surface, member) in [
        ("Eventuality", "fact:denotation-reading"),
        ("Eventuality", "fact:detail-unspecified"),
        ("Eventuality", "fact:sort-header"),
        ("Quantity", "fact:explicit-counting"),
        ("Referent", "fact:binding-label"),
        ("Referent", "fact:sort-header"),
    ] {
        rows.insert(SemanticCoordinate::new(
            CoordinateCategory::Object,
            surface,
            CoordinateKind::DerivedFact,
            member,
            None,
        ));
    }
    for member in [
        "lexical-policy-entity-failure",
        "lexical-policy-eventuality-failure",
        "lexical-relation-row-missing",
        "de-re-owner-missing",
        "de-re-owner-wrong-kind",
        "de-re-owner-opaque",
        "de-re-owner-unrelated-or-nondominating",
        "de-re-owner-dependency-illegal",
        "dynamic-host-cycle",
        "dynamic-host-not-unique",
        "force-handler-missing-or-illegal",
        "effect-handler-missing-or-illegal",
        "conflicting-binder-owners",
        "binder-does-not-dominate-use",
        "scope-dependency-without-binder",
        "unguarded-or-unrepresentable-scc",
        "definition-site-does-not-dominate-use",
        "declaration-planning-nonconvergence",
        "generated-eventuality-unbound",
        "event-owner-missing-or-nonunique",
        "lexical-signature-missing-or-stale",
        "relation-reduction-unregistered-or-inexact",
        "predicate-fill-type-or-arity-mismatch",
        "computed-fill-domain-noninjective",
        "predicate-closure-unlicensed",
        "higher-order-crossing-unlicensed",
        "question-domain-or-answer-mismatch",
        "reference-description-unrepresentable",
        "dependent-supplement-unrepresentable",
        "quantifier-effect-export-illegal",
        "simultaneous-termset-unlicensed",
        "modal-tag-reduction-unregistered",
        "event-facet-reduction-unregistered",
        "abstraction-crossing-unlicensed",
        "quantity-reduction-unregistered",
        "math-reduction-unregistered",
        "sequence-reduction-unregistered",
        "structured-quotation-transcript-entry-missing",
        "sign-identity-missing",
        "force-reduction-unrepresentable",
        "prelude-reduction-unavailable",
        "place-deletion-evidence-missing",
        "relation-former-reduction-unavailable",
        "unknown-registry-coordinate",
    ] {
        rows.insert(SemanticCoordinate::new(
            CoordinateCategory::Document,
            "semantic-graph",
            CoordinateKind::DerivedFact,
            member,
            None,
        ));
    }
    rows
}
