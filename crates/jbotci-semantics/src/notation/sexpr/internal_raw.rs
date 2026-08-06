//! The jbotci-internal raw debug codec (`docs/smusni/internal-raw.md`).
//!
//! **Nothing in this module is smusni.** The public grammar of specification
//! section 2.2 has exactly one document body and no raw productions; a
//! projection either produces one complete `(Smusni 0 ...)` document or
//! produces none at all. What lives here is the unversioned, unstable capture
//! format the losslessness oracle and the corpus tooling use, kept out of
//! [`super::syntax`] so the acceptance parser cannot accept it by accident.
//!
//! The codec is not a compatibility surface and no consumer of the format is
//! ever handed one of its values.

#![cfg_attr(not(test), allow(dead_code))]

use std::collections::BTreeSet;

#[allow(unused_imports)]
use bityzba::{ensures, invariant, new, requires};
use num_bigint::BigUint;

use super::datum::{Datum, parse_document};
use super::syntax::{
    NfcText, ProjectionReasonId, V0ParseError, is_projection_reason_id, require_len, text_from,
};
use super::type_system::{PositiveInteger, TypeExpr};
use crate::model::SemanticGraph;

/// A positive fallback-object identity.
#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObjectId(PositiveInteger);

impl ObjectId {
    /// Parse `%n`.
    #[requires(true)]
    #[ensures(ret.is_ok() == text.strip_prefix('%').is_some_and(|digits| PositiveInteger::try_new(digits).is_ok()))]
    fn parse(text: &str) -> Result<Self, V0ParseError> {
        let value = text
            .strip_prefix('%')
            .and_then(|digits| PositiveInteger::try_new(digits).ok())
            .ok_or_else(|| V0ParseError::new("fallback object id must be %positive-integer"))?;
        Ok(Self(value))
    }

    /// Borrow the numeric identity for raw-tree ordering validation.
    #[requires(true)]
    #[ensures(*ret > BigUint::from(0u8))]
    fn as_biguint(&self) -> &BigUint {
        self.0.as_biguint()
    }

    /// Serialize the identity token.
    #[requires(true)]
    #[ensures(ret.as_atom().is_some_and(|atom| atom.starts_with('%')))]
    fn to_datum(&self) -> Datum {
        Datum::atom(format!("%{}", self.0))
    }
}

/// One raw fallback field.
#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawField {
    name: NfcText,
    value: RawValue,
}

/// One raw object definition.
#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawObject {
    id: ObjectId,
    type_name: NfcText,
    fields: Vec<RawField>,
}

/// One inline identity-free product/newtype.
#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawRecord {
    type_name: NfcText,
    fields: Vec<RawField>,
}

/// One inline algebraic-sum constructor.
#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawVariant {
    enum_type: NfcText,
    constructor: NfcText,
    fields: Vec<RawField>,
}

/// One typed raw-map entry.
#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawMapEntry {
    key: RawValue,
    value: RawValue,
}

/// The closed raw fallback grammar.
#[invariant(::Object(_) => true)]
#[invariant(::Ref(_) => true)]
#[invariant(::Record(_) => true)]
#[invariant(::Variant(_) => true)]
#[invariant(::List(_) => true)]
#[invariant(::Map(_) => true)]
#[invariant(::Atom(_) => true)]
#[invariant(::TypedAtom { .. } => true)]
#[invariant(::Scalar { .. } => true)]
#[invariant(::String(_) => true)]
#[invariant(::Null => true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RawValue {
    Object(RawObject),
    Ref(ObjectId),
    Record(RawRecord),
    Variant(RawVariant),
    List(Vec<RawValue>),
    Map(Vec<RawMapEntry>),
    Atom(NfcText),
    TypedAtom {
        model_enum_type: NfcText,
        case: NfcText,
    },
    Scalar {
        model_scalar_type: NfcText,
        lexical_value: NfcText,
    },
    String(NfcText),
    Null,
}

impl RawValue {
    /// Serialize one raw production.
    #[requires(true)]
    #[ensures(true)]
    fn to_datum(&self) -> Datum {
        match self {
            Self::Object(object) => {
                let mut values = vec![
                    object.id.to_datum(),
                    Datum::string(object.type_name.as_str()),
                ];
                values.extend(object.fields.iter().map(|field| {
                    Datum::form(
                        "Field",
                        [Datum::string(field.name.as_str()), field.value.to_datum()],
                    )
                }));
                Datum::form("Object", values)
            }
            Self::Ref(id) => Datum::form("Ref", [id.to_datum()]),
            Self::Record(record) => Datum::form(
                "RawRecord",
                std::iter::once(Datum::string(record.type_name.as_str())).chain(
                    record.fields.iter().map(|field| {
                        Datum::form(
                            "Field",
                            [Datum::string(field.name.as_str()), field.value.to_datum()],
                        )
                    }),
                ),
            ),
            Self::Variant(variant) => Datum::form(
                "RawVariant",
                [
                    Datum::string(variant.enum_type.as_str()),
                    Datum::string(variant.constructor.as_str()),
                ]
                .into_iter()
                .chain(variant.fields.iter().map(|field| {
                    Datum::form(
                        "Field",
                        [Datum::string(field.name.as_str()), field.value.to_datum()],
                    )
                })),
            ),
            Self::List(values) => Datum::form("RawList", values.iter().map(Self::to_datum)),
            Self::Map(entries) => Datum::form(
                "RawMap",
                entries.iter().map(|entry| {
                    Datum::form("Entry", [entry.key.to_datum(), entry.value.to_datum()])
                }),
            ),
            Self::Atom(value) => Datum::form("RawAtom", [Datum::string(value.as_str())]),
            Self::TypedAtom {
                model_enum_type,
                case,
            } => Datum::form(
                "RawTypedAtom",
                [
                    Datum::string(model_enum_type.as_str()),
                    Datum::string(case.as_str()),
                ],
            ),
            Self::Scalar {
                model_scalar_type,
                lexical_value,
            } => Datum::form(
                "RawScalar",
                [
                    Datum::string(model_scalar_type.as_str()),
                    Datum::string(lexical_value.as_str()),
                ],
            ),
            Self::String(value) => Datum::form("RawString", [Datum::string(value.as_str())]),
            Self::Null => Datum::form("RawNull", []),
        }
    }
}

/// A raw root with depth-first first-definition/reference ordering proved.
#[invariant(raw_identity_order_is_valid(&root))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawTree {
    root: RawValue,
}

impl RawTree {
    /// Validate a raw identity root.
    #[requires(raw_identity_order_is_valid(&root))]
    #[ensures(ret.root == old(root.clone()))]
    pub fn new(root: RawValue) -> Self {
        new!(RawTree { root })
    }

    /// Borrow the raw root.
    #[requires(true)]
    #[ensures(raw_identity_order_is_valid(ret))]
    pub fn root(&self) -> &RawValue {
        &self.root
    }
}

/// A local fallback that inhabits one known static type.
#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalFallback {
    expected_type: TypeExpr,
    reason: ProjectionReasonId,
    raw: RawTree,
}

impl LocalFallback {
    /// Serialize the exact local fallback grammar.
    #[requires(true)]
    #[ensures(ret.form_head() == Some("Fallback"))]
    fn to_datum(&self) -> Datum {
        Datum::form(
            "Fallback",
            [
                self.expected_type.to_datum(),
                Datum::string(self.reason.as_str()),
                self.raw.root.to_datum(),
            ],
        )
    }
}

/// A whole-document fallback, valid only under `Smusni`.
#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedGraph {
    model_root_type: NfcText,
    reason: ProjectionReasonId,
    raw: RawTree,
}

impl TypedGraph {
    /// Serialize the whole-document fallback grammar.
    #[requires(true)]
    #[ensures(ret.form_head() == Some("TypedGraph"))]
    fn to_datum(&self) -> Datum {
        Datum::form(
            "TypedGraph",
            [
                Datum::string(self.model_root_type.as_str()),
                Datum::string(self.reason.as_str()),
                self.raw.root.to_datum(),
            ],
        )
    }
}

/// Parse one local fallback.
#[requires(items.first().and_then(Datum::as_atom) == Some("Fallback"))]
#[ensures(ret.is_ok() || ret.is_err())]
fn parse_local_fallback(items: &[Datum]) -> Result<LocalFallback, V0ParseError> {
    require_len(items, 4, "Fallback")?;
    let expected_type =
        TypeExpr::parse(&items[1]).map_err(|error| V0ParseError::new(error.to_string()))?;
    let reason = items[2]
        .as_string()
        .ok_or_else(|| V0ParseError::new("Fallback reason must be a string"))?;
    let raw = parse_raw_tree(&items[3])?;
    Ok(LocalFallback {
        expected_type,
        reason: ProjectionReasonId::try_new(reason)?,
        raw,
    })
}

/// Parse whole-document fallback.
#[requires(datum.form_head() == Some("TypedGraph"))]
#[ensures(ret.is_ok() || ret.is_err())]
fn parse_typed_graph(datum: &Datum) -> Result<TypedGraph, V0ParseError> {
    let items = datum.as_list().expect("form head implies list");
    require_len(items, 4, "TypedGraph")?;
    let root_type = items[1]
        .as_string()
        .ok_or_else(|| V0ParseError::new("TypedGraph root type must be a string"))?;
    if root_type.is_empty() {
        return Err(V0ParseError::new("TypedGraph root type must not be empty"));
    }
    let reason = items[2]
        .as_string()
        .ok_or_else(|| V0ParseError::new("TypedGraph reason must be a string"))?;
    let raw = parse_raw_tree(&items[3])?;
    let RawValue::Object(root) = raw.root() else {
        return Err(V0ParseError::new("TypedGraph raw root must be an Object"));
    };
    if root.id.as_biguint() != &BigUint::from(1u8) || root.type_name.as_str() != root_type {
        return Err(V0ParseError::new(
            "TypedGraph raw root must define %1 with the declared root type",
        ));
    }
    Ok(TypedGraph {
        model_root_type: NfcText::new(root_type),
        reason: ProjectionReasonId::try_new(reason)?,
        raw,
    })
}

/// Parse and validate a raw identity root.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn parse_raw_tree(datum: &Datum) -> Result<RawTree, V0ParseError> {
    let root = parse_raw_value(datum)?;
    if !raw_identity_order_is_valid(&root) {
        return Err(V0ParseError::new(
            "raw object identities must be first-defined depth-first as %1, %2, ...",
        ));
    }
    Ok(RawTree::new(root))
}

/// Parse the closed raw grammar recursively.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn parse_raw_value(datum: &Datum) -> Result<RawValue, V0ParseError> {
    let items = datum
        .as_list()
        .ok_or_else(|| V0ParseError::new("raw value must use a registered raw form"))?;
    match items.first().and_then(Datum::as_atom) {
        Some("Object") => {
            if items.len() < 3 {
                return Err(V0ParseError::new("Object requires id and type name"));
            }
            let id = items[1]
                .as_atom()
                .ok_or_else(|| V0ParseError::new("Object id must be an atom"))
                .and_then(ObjectId::parse)?;
            let type_name = text_from(&items[2], "Object type name")?;
            let fields = items[3..]
                .iter()
                .map(parse_raw_field)
                .collect::<Result<Vec<_>, _>>()?;
            Ok(RawValue::Object(RawObject {
                id,
                type_name,
                fields,
            }))
        }
        Some("Ref") => {
            require_len(items, 2, "Ref")?;
            let id = items[1]
                .as_atom()
                .ok_or_else(|| V0ParseError::new("Ref id must be an atom"))
                .and_then(ObjectId::parse)?;
            Ok(RawValue::Ref(id))
        }
        Some("RawRecord") => {
            if items.len() < 2 {
                return Err(V0ParseError::new("RawRecord requires a type name"));
            }
            Ok(RawValue::Record(RawRecord {
                type_name: text_from(&items[1], "RawRecord type name")?,
                fields: items[2..]
                    .iter()
                    .map(parse_raw_field)
                    .collect::<Result<Vec<_>, _>>()?,
            }))
        }
        Some("RawVariant") => {
            if items.len() < 3 {
                return Err(V0ParseError::new(
                    "RawVariant requires an enum type and constructor",
                ));
            }
            Ok(RawValue::Variant(RawVariant {
                enum_type: text_from(&items[1], "RawVariant enum type")?,
                constructor: text_from(&items[2], "RawVariant constructor")?,
                fields: items[3..]
                    .iter()
                    .map(parse_raw_field)
                    .collect::<Result<Vec<_>, _>>()?,
            }))
        }
        Some("RawList") => Ok(RawValue::List(
            items[1..]
                .iter()
                .map(parse_raw_value)
                .collect::<Result<Vec<_>, _>>()?,
        )),
        Some("RawMap") => Ok(RawValue::Map(
            items[1..]
                .iter()
                .map(parse_raw_entry)
                .collect::<Result<Vec<_>, _>>()?,
        )),
        Some("RawAtom") => {
            require_len(items, 2, "RawAtom")?;
            Ok(RawValue::Atom(text_from(&items[1], "RawAtom")?))
        }
        Some("RawTypedAtom") => {
            require_len(items, 3, "RawTypedAtom")?;
            Ok(RawValue::TypedAtom {
                model_enum_type: text_from(&items[1], "RawTypedAtom type")?,
                case: text_from(&items[2], "RawTypedAtom case")?,
            })
        }
        Some("RawScalar") => {
            require_len(items, 3, "RawScalar")?;
            Ok(RawValue::Scalar {
                model_scalar_type: text_from(&items[1], "RawScalar type")?,
                lexical_value: text_from(&items[2], "RawScalar value")?,
            })
        }
        Some("RawString") => {
            require_len(items, 2, "RawString")?;
            Ok(RawValue::String(text_from(&items[1], "RawString")?))
        }
        Some("RawNull") => {
            require_len(items, 1, "RawNull")?;
            Ok(RawValue::Null)
        }
        _ => Err(V0ParseError::new("unknown raw fallback form")),
    }
}

/// Parse one exact `(Entry raw raw)`.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn parse_raw_entry(datum: &Datum) -> Result<RawMapEntry, V0ParseError> {
    let items = datum
        .as_list()
        .ok_or_else(|| V0ParseError::new("raw map entry must be an Entry form"))?;
    if items.first().and_then(Datum::as_atom) != Some("Entry") {
        return Err(V0ParseError::new("RawMap may contain only Entry forms"));
    }
    require_len(items, 3, "Entry")?;
    Ok(RawMapEntry {
        key: parse_raw_value(&items[1])?,
        value: parse_raw_value(&items[2])?,
    })
}

/// Parse one exact `(Field string raw)`.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn parse_raw_field(datum: &Datum) -> Result<RawField, V0ParseError> {
    let items = datum
        .as_list()
        .ok_or_else(|| V0ParseError::new("raw field must be a Field form"))?;
    if items.first().and_then(Datum::as_atom) != Some("Field") {
        return Err(V0ParseError::new("raw object child must be Field"));
    }
    require_len(items, 3, "Field")?;
    Ok(RawField {
        name: text_from(&items[1], "Field name")?,
        value: parse_raw_value(&items[2])?,
    })
}

/// Validate depth-first `%id` definition/reference order.
#[requires(true)]
#[ensures(true)]
fn raw_identity_order_is_valid(root: &RawValue) -> bool {
    #[requires(true)]
    #[ensures(true)]
    fn visit(value: &RawValue, defined: &mut BTreeSet<ObjectId>, next: &mut BigUint) -> bool {
        match value {
            RawValue::Object(object) => {
                if object.id.as_biguint() != next || !defined.insert(object.id.clone()) {
                    return false;
                }
                *next += 1u8;
                object
                    .fields
                    .iter()
                    .all(|field| visit(&field.value, defined, next))
            }
            RawValue::Ref(id) => defined.contains(id),
            RawValue::Record(record) => record
                .fields
                .iter()
                .all(|field| visit(&field.value, defined, next)),
            RawValue::Variant(variant) => variant
                .fields
                .iter()
                .all(|field| visit(&field.value, defined, next)),
            RawValue::List(values) => values.iter().all(|value| visit(value, defined, next)),
            RawValue::Map(entries) => entries.iter().all(|entry| {
                visit(&entry.key, defined, next) && visit(&entry.value, defined, next)
            }),
            RawValue::Atom(_)
            | RawValue::TypedAtom { .. }
            | RawValue::Scalar { .. }
            | RawValue::String(_)
            | RawValue::Null => true,
        }
    }
    visit(root, &mut BTreeSet::new(), &mut BigUint::from(1u8))
}

/// The two capture roots of the internal codec.
#[invariant(::Local(_) => true)]
#[invariant(::WholeGraph(_) => true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Capture {
    Local(LocalFallback),
    WholeGraph(TypedGraph),
}

/// Build one whole-graph capture for the debug/oracle path.
///
/// This is the only producer of a capture root, and it is never reachable from
/// a projection result: section 16 forbids a failed projection from carrying a
/// serialized graph, so only the internal oracle and corpus tooling call it.
#[requires(graph.objects.contains_key(&graph.root))]
#[requires(is_projection_reason_id(reason_id))]
#[ensures(ret.form_head() == Some("TypedGraph"))]
pub fn whole_graph_capture(graph: &SemanticGraph, reason_id: &str) -> Datum {
    Datum::form(
        "TypedGraph",
        [
            Datum::string("SemanticGraph"),
            Datum::string(reason_id),
            super::structural::raw_graph_datum(graph),
        ],
    )
}

/// Parse one capture root. This is the codec's own parser: the public
/// acceptance parser in [`super::syntax`] rejects both of these forms.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
pub fn parse_capture(input: &str) -> Result<Capture, V0ParseError> {
    let datum = parse_document(input).map_err(|error| V0ParseError::new(error.to_string()))?;
    match datum.form_head() {
        Some("TypedGraph") => parse_typed_graph(&datum).map(Capture::WholeGraph),
        Some("Fallback") => {
            let items = datum.as_list().expect("form head implies list");
            parse_local_fallback(items).map(Capture::Local)
        }
        _ => Err(V0ParseError::new(
            "an internal capture root is Fallback or TypedGraph",
        )),
    }
}
