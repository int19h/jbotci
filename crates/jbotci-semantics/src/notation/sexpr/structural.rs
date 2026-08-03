//! Mechanically complete typed structural projection.
//!
//! This is intentionally not a `serde_json::Value` walker. The custom serializer
//! retains struct and enum type information, and recognizes the model's tagged
//! `SemanticObjectId` newtype as a graph reference. Compact semantic forms are
//! built directly from model types elsewhere; this module is their total,
//! non-lossy fallback.

use std::collections::BTreeMap;
use std::fmt;

#[allow(unused_imports)]
use bityzba::{data, ensures, invariant, new, requires};
use serde::Serialize;
use serde::ser::{
    self, SerializeMap, SerializeSeq, SerializeStruct, SerializeStructVariant, SerializeTuple,
    SerializeTupleStruct, SerializeTupleVariant, Serializer,
};

use super::datum::Datum;
use crate::completeness::source_link_surfaces;
use crate::model::{SemanticObject, SemanticObjectData, SemanticObjectId, SemanticObjectKind};
use crate::notation::word_cards::WordCard;

const SEMANTIC_OBJECT_ID_NEWTYPE: &str = "SemanticObjectId";

/// A typed intermediate produced directly by Serde's structural callbacks.
#[invariant(::Reference(value) => !value.is_empty())]
#[invariant(::String(_) => true)]
#[invariant(::Bool(_) => true)]
#[invariant(::Signed(_) => true)]
#[invariant(::Unsigned(_) => true)]
#[invariant(::Float(value) => value.is_finite())]
#[invariant(::Unit => true)]
#[invariant(::Sequence(_) => true)]
#[invariant(::Map(_) => true)]
#[invariant(::Record { name, fields } => !name.is_empty() && fields.iter().all(|(field, _)| !field.is_empty()))]
#[invariant(::Variant { type_name, variant, .. } => !type_name.is_empty() && !variant.is_empty())]
#[derive(Debug, Clone, PartialEq)]
enum StructuralValue {
    Reference(String),
    String(String),
    Bool(bool),
    Signed(i128),
    Unsigned(u128),
    Float(f64),
    Unit,
    Sequence(Vec<StructuralValue>),
    Map(Vec<(StructuralValue, StructuralValue)>),
    Record {
        name: &'static str,
        fields: Vec<(&'static str, StructuralValue)>,
    },
    Variant {
        type_name: &'static str,
        variant: &'static str,
        value: Option<Box<StructuralValue>>,
    },
}

/// Failure from an impossible or malformed model serialization callback.
#[invariant(!message.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
struct StructuralError {
    message: String,
}

impl StructuralError {
    /// Build a nonempty structural serialization error.
    #[requires(true)]
    #[ensures(!ret.message.is_empty())]
    fn new(message: impl fmt::Display) -> Self {
        let message = message.to_string();
        let message = if message.is_empty() {
            "structural serialization failed".to_owned()
        } else {
            message
        };
        new!(StructuralError { message: message })
    }
}

impl fmt::Display for StructuralError {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for StructuralError {}

impl ser::Error for StructuralError {
    #[requires(true)]
    #[ensures(true)]
    fn custom<T: fmt::Display>(message: T) -> Self {
        Self::new(message)
    }
}

/// Stateless serializer for one complete value.
#[invariant(true)]
#[derive(Debug, Clone, Copy)]
struct StructuralSerializer;

macro_rules! signed_serializer {
    ($method:ident, $type:ty) => {
        #[requires(true)]
        #[ensures(ret.is_ok())]
        fn $method(self, value: $type) -> Result<Self::Ok, Self::Error> {
            Ok(new!(StructuralValue::Signed(value as i128)))
        }
    };
}

macro_rules! unsigned_serializer {
    ($method:ident, $type:ty) => {
        #[requires(true)]
        #[ensures(ret.is_ok())]
        fn $method(self, value: $type) -> Result<Self::Ok, Self::Error> {
            Ok(new!(StructuralValue::Unsigned(value as u128)))
        }
    };
}

impl Serializer for StructuralSerializer {
    type Ok = StructuralValue;
    type Error = StructuralError;
    type SerializeSeq = SequenceBuilder;
    type SerializeTuple = SequenceBuilder;
    type SerializeTupleStruct = TupleStructBuilder;
    type SerializeTupleVariant = TupleVariantBuilder;
    type SerializeMap = MapBuilder;
    type SerializeStruct = StructBuilder;
    type SerializeStructVariant = StructVariantBuilder;

    signed_serializer!(serialize_i8, i8);
    signed_serializer!(serialize_i16, i16);
    signed_serializer!(serialize_i32, i32);
    signed_serializer!(serialize_i64, i64);

    #[requires(true)]
    #[ensures(ret.is_ok())]
    fn serialize_i128(self, value: i128) -> Result<Self::Ok, Self::Error> {
        Ok(new!(StructuralValue::Signed(value)))
    }

    unsigned_serializer!(serialize_u8, u8);
    unsigned_serializer!(serialize_u16, u16);
    unsigned_serializer!(serialize_u32, u32);
    unsigned_serializer!(serialize_u64, u64);

    #[requires(true)]
    #[ensures(ret.is_ok())]
    fn serialize_u128(self, value: u128) -> Result<Self::Ok, Self::Error> {
        Ok(new!(StructuralValue::Unsigned(value)))
    }

    #[requires(true)]
    #[ensures(ret.is_ok())]
    fn serialize_bool(self, value: bool) -> Result<Self::Ok, Self::Error> {
        Ok(new!(StructuralValue::Bool(value)))
    }

    #[requires(true)]
    #[ensures(ret.is_ok() == value.is_finite())]
    fn serialize_f32(self, value: f32) -> Result<Self::Ok, Self::Error> {
        if !value.is_finite() {
            return Err(StructuralError::new("non-finite f32 is not representable"));
        }
        Ok(new!(StructuralValue::Float(f64::from(value))))
    }

    #[requires(true)]
    #[ensures(ret.is_ok() == value.is_finite())]
    fn serialize_f64(self, value: f64) -> Result<Self::Ok, Self::Error> {
        if !value.is_finite() {
            return Err(StructuralError::new("non-finite f64 is not representable"));
        }
        Ok(new!(StructuralValue::Float(value)))
    }

    #[requires(true)]
    #[ensures(ret.is_ok())]
    fn serialize_char(self, value: char) -> Result<Self::Ok, Self::Error> {
        Ok(new!(StructuralValue::String(value.to_string())))
    }

    #[requires(true)]
    #[ensures(ret.is_ok())]
    fn serialize_str(self, value: &str) -> Result<Self::Ok, Self::Error> {
        Ok(new!(StructuralValue::String(value.to_owned())))
    }

    #[requires(true)]
    #[ensures(ret.is_ok())]
    fn serialize_bytes(self, value: &[u8]) -> Result<Self::Ok, Self::Error> {
        Ok(new!(StructuralValue::Sequence(
            value
                .iter()
                .map(|byte| new!(StructuralValue::Unsigned(u128::from(*byte))))
                .collect()
        )))
    }

    #[requires(true)]
    #[ensures(ret.is_ok())]
    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Ok(new!(StructuralValue::Unit))
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn serialize_some<T: ?Sized + Serialize>(self, value: &T) -> Result<Self::Ok, Self::Error> {
        value.serialize(self)
    }

    #[requires(true)]
    #[ensures(ret.is_ok())]
    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Ok(new!(StructuralValue::Unit))
    }

    #[requires(true)]
    #[ensures(ret.is_ok())]
    fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok, Self::Error> {
        Ok(new!(StructuralValue::Record {
            name,
            fields: Vec::new()
        }))
    }

    #[requires(true)]
    #[ensures(ret.is_ok())]
    fn serialize_unit_variant(
        self,
        name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        Ok(new!(StructuralValue::Variant {
            type_name: name,
            variant,
            value: None
        }))
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn serialize_newtype_struct<T: ?Sized + Serialize>(
        self,
        name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        let value = value.serialize(self)?;
        if name == SEMANTIC_OBJECT_ID_NEWTYPE {
            return match value.into_data() {
                data!(StructuralValue::String(id)) => Ok(new!(StructuralValue::Reference(id))),
                _ => Err(StructuralError::new(
                    "SemanticObjectId newtype must contain a string",
                )),
            };
        }
        Ok(new!(StructuralValue::Record {
            name,
            fields: vec![("value", value)]
        }))
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        Ok(new!(StructuralValue::Variant {
            type_name: name,
            variant,
            value: Some(Box::new(value.serialize(self)?))
        }))
    }

    #[requires(true)]
    #[ensures(ret.is_ok())]
    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Ok(SequenceBuilder::with_capacity(len.unwrap_or(0)))
    }

    #[requires(true)]
    #[ensures(ret.is_ok())]
    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Ok(SequenceBuilder::with_capacity(len))
    }

    #[requires(true)]
    #[ensures(ret.is_ok())]
    fn serialize_tuple_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        Ok(TupleStructBuilder {
            name,
            values: Vec::with_capacity(len),
        })
    }

    #[requires(true)]
    #[ensures(ret.is_ok())]
    fn serialize_tuple_variant(
        self,
        name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Ok(TupleVariantBuilder {
            type_name: name,
            variant,
            values: Vec::with_capacity(len),
        })
    }

    #[requires(true)]
    #[ensures(ret.is_ok())]
    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Ok(MapBuilder {
            entries: Vec::with_capacity(len.unwrap_or(0)),
            pending_key: None,
        })
    }

    #[requires(true)]
    #[ensures(ret.is_ok())]
    fn serialize_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Ok(StructBuilder {
            name,
            fields: Vec::with_capacity(len),
        })
    }

    #[requires(true)]
    #[ensures(ret.is_ok())]
    fn serialize_struct_variant(
        self,
        name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Ok(StructVariantBuilder {
            type_name: name,
            variant,
            fields: Vec::with_capacity(len),
        })
    }
}

/// Builder shared by sequences and tuples.
#[invariant(true)]
#[derive(Debug)]
struct SequenceBuilder {
    values: Vec<StructuralValue>,
}

impl SequenceBuilder {
    /// Allocate a builder with a serialization size hint.
    #[requires(true)]
    #[ensures(ret.values.is_empty())]
    fn with_capacity(capacity: usize) -> Self {
        Self {
            values: Vec::with_capacity(capacity),
        }
    }

    /// Serialize and append one element.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn push<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), StructuralError> {
        self.values.push(value.serialize(StructuralSerializer)?);
        Ok(())
    }

    /// Finish the sequence.
    #[requires(true)]
    #[ensures(true)]
    fn finish(self) -> StructuralValue {
        new!(StructuralValue::Sequence(self.values))
    }
}

impl SerializeSeq for SequenceBuilder {
    type Ok = StructuralValue;
    type Error = StructuralError;

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
        self.push(value)
    }

    #[requires(true)]
    #[ensures(ret.is_ok())]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(self.finish())
    }
}

impl SerializeTuple for SequenceBuilder {
    type Ok = StructuralValue;
    type Error = StructuralError;

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
        self.push(value)
    }

    #[requires(true)]
    #[ensures(ret.is_ok())]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(self.finish())
    }
}

/// Builder for a named tuple struct.
#[invariant(true)]
#[derive(Debug)]
struct TupleStructBuilder {
    name: &'static str,
    values: Vec<StructuralValue>,
}

impl SerializeTupleStruct for TupleStructBuilder {
    type Ok = StructuralValue;
    type Error = StructuralError;

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
        self.values.push(value.serialize(StructuralSerializer)?);
        Ok(())
    }

    #[requires(true)]
    #[ensures(ret.is_ok())]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(new!(StructuralValue::Record {
            name: self.name,
            fields: self
                .values
                .into_iter()
                .map(|value| ("value", value))
                .collect()
        }))
    }
}

/// Builder for a tuple enum variant.
#[invariant(true)]
#[derive(Debug)]
struct TupleVariantBuilder {
    type_name: &'static str,
    variant: &'static str,
    values: Vec<StructuralValue>,
}

impl SerializeTupleVariant for TupleVariantBuilder {
    type Ok = StructuralValue;
    type Error = StructuralError;

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
        self.values.push(value.serialize(StructuralSerializer)?);
        Ok(())
    }

    #[requires(true)]
    #[ensures(ret.is_ok())]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(new!(StructuralValue::Variant {
            type_name: self.type_name,
            variant: self.variant,
            value: Some(Box::new(new!(StructuralValue::Sequence(self.values))))
        }))
    }
}

/// Builder for maps, with an explicit key/value state invariant.
#[invariant(true)]
#[derive(Debug)]
struct MapBuilder {
    entries: Vec<(StructuralValue, StructuralValue)>,
    pending_key: Option<StructuralValue>,
}

impl SerializeMap for MapBuilder {
    type Ok = StructuralValue;
    type Error = StructuralError;

    #[requires(self.pending_key.is_none())]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn serialize_key<T: ?Sized + Serialize>(&mut self, key: &T) -> Result<(), Self::Error> {
        self.pending_key = Some(key.serialize(StructuralSerializer)?);
        Ok(())
    }

    #[requires(self.pending_key.is_some())]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn serialize_value<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
        let key = self
            .pending_key
            .take()
            .ok_or_else(|| StructuralError::new("map value arrived before its key"))?;
        self.entries
            .push((key, value.serialize(StructuralSerializer)?));
        Ok(())
    }

    #[requires(self.pending_key.is_none())]
    #[ensures(ret.is_ok())]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(new!(StructuralValue::Map(self.entries)))
    }
}

/// Builder for a named struct.
#[invariant(true)]
#[derive(Debug)]
struct StructBuilder {
    name: &'static str,
    fields: Vec<(&'static str, StructuralValue)>,
}

impl SerializeStruct for StructBuilder {
    type Ok = StructuralValue;
    type Error = StructuralError;

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error> {
        self.fields
            .push((key, value.serialize(StructuralSerializer)?));
        Ok(())
    }

    #[requires(true)]
    #[ensures(ret.is_ok())]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(new!(StructuralValue::Record {
            name: self.name,
            fields: self.fields
        }))
    }
}

/// Builder for a struct enum variant.
#[invariant(true)]
#[derive(Debug)]
struct StructVariantBuilder {
    type_name: &'static str,
    variant: &'static str,
    fields: Vec<(&'static str, StructuralValue)>,
}

impl SerializeStructVariant for StructVariantBuilder {
    type Ok = StructuralValue;
    type Error = StructuralError;

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error> {
        self.fields
            .push((key, value.serialize(StructuralSerializer)?));
        Ok(())
    }

    #[requires(true)]
    #[ensures(ret.is_ok())]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        let record = new!(StructuralValue::Record {
            name: self.type_name,
            fields: self.fields
        });
        Ok(new!(StructuralValue::Variant {
            type_name: self.type_name,
            variant: self.variant,
            value: Some(Box::new(record))
        }))
    }
}

/// Whether ordinary-profile provenance fields are retained.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProvenanceDisposition {
    Suppress,
    Retain,
}

/// Project one object to the explicit local typed fallback.
#[requires(true)]
#[ensures(matches!(ret, Datum::List(_)))]
pub fn object_datum(
    kind: SemanticObjectKind,
    object: &SemanticObject,
    provenance: ProvenanceDisposition,
) -> Datum {
    object_datum_with_variables(kind, object, provenance, &BTreeMap::new())
}

/// Project one object while spelling selected graph references as lexical
/// variables. Lookup keys are produced from typed IDs; ID text is never parsed.
#[requires(true)]
#[ensures(matches!(ret, Datum::List(_)))]
pub fn object_datum_with_variables(
    kind: SemanticObjectKind,
    object: &SemanticObject,
    provenance: ProvenanceDisposition,
    variables: &BTreeMap<String, Datum>,
) -> Datum {
    let value = object
        .serialize(StructuralSerializer)
        .expect("the semantic model has a complete structural serialization");
    let mut fields = object_field_datums(value, provenance, variables);
    fields.extend(supplemental_object_field_datums(
        object, provenance, variables,
    ));
    let mut arguments = vec![Datum::atom(kind_name(kind))];
    arguments.extend(fields);
    Datum::form("Object", arguments)
}

/// Project any typed model value through the same complete structural path.
/// This is used for field-local fallback when a compact recognizer declines.
#[requires(true)]
#[ensures(true)]
pub fn typed_value_datum<T: Serialize>(
    value: &T,
    provenance: ProvenanceDisposition,
    variables: &BTreeMap<String, Datum>,
) -> Datum {
    let value = value
        .serialize(StructuralSerializer)
        .expect("model values have complete structural serializations");
    structural_datum(value, variables, provenance)
}

/// Project one structured dictionary word card without introducing a second
/// escaping or formatting path.
#[requires(true)]
#[ensures(matches!(ret, Datum::List(_)))]
pub fn word_card_datum(card: &WordCard) -> Datum {
    let value = card
        .serialize(StructuralSerializer)
        .expect("word cards have complete structural serializations");
    let data!(StructuralValue::Record { fields, .. }) = value.into_data() else {
        unreachable!("WordCard serializes as a named record")
    };
    Datum::form(
        "Word",
        fields.into_iter().map(|(name, value)| {
            field_datum(
                name,
                value,
                &BTreeMap::new(),
                ProvenanceDisposition::Suppress,
            )
        }),
    )
}

/// Project one object definition for the whole-document typed graph.
#[requires(id.object_kind() == object.object_kind())]
#[ensures(matches!(ret, Datum::List(_)))]
pub fn definition_datum(id: SemanticObjectId, object: &SemanticObject) -> Datum {
    let value = object
        .serialize(StructuralSerializer)
        .expect("the semantic model has a complete structural serialization");
    let mut arguments = vec![
        reference_datum(id),
        Datum::atom(kind_name(object.object_kind())),
    ];
    arguments.extend(object_field_datums(
        value,
        ProvenanceDisposition::Retain,
        &BTreeMap::new(),
    ));
    arguments.extend(supplemental_object_field_datums(
        object,
        ProvenanceDisposition::Retain,
        &BTreeMap::new(),
    ));
    Datum::form("Def", arguments)
}

/// Fields present in the typed graph model but intentionally absent from the
/// stable flat JSON serializer. Typed fallback is a model projection, so it
/// must retain these fields without changing JSON compatibility.
#[requires(true)]
#[ensures(true)]
fn supplemental_object_field_datums(
    object: &SemanticObject,
    provenance: ProvenanceDisposition,
    variables: &BTreeMap<String, Datum>,
) -> Vec<Datum> {
    let mut fields = Vec::new();
    match object.as_data() {
        data!(SemanticObject::Eventuality(node)) => {
            if let Some(class) = node.class {
                fields.push(field_datum(
                    "class",
                    class
                        .serialize(StructuralSerializer)
                        .expect("eventuality classes have structural serialization"),
                    variables,
                    provenance,
                ));
            }
            if let Some(kind) = node.abstraction_kind {
                fields.push(field_datum(
                    "abstractionKind",
                    kind.serialize(StructuralSerializer)
                        .expect("abstraction kinds have structural serialization"),
                    variables,
                    provenance,
                ));
            }
        }
        data!(SemanticObject::Referent(node)) => {
            if let Some(kind) = node.abstraction_kind {
                fields.push(field_datum(
                    "abstractionKind",
                    kind.serialize(StructuralSerializer)
                        .expect("abstraction kinds have structural serialization"),
                    variables,
                    provenance,
                ));
            }
        }
        _ => {}
    }
    fields
}

/// Render a graph identity in the dedicated `@` namespace.
#[requires(true)]
#[ensures(ret.as_atom().is_some_and(|atom| atom.starts_with('@')))]
pub fn reference_datum(id: SemanticObjectId) -> Datum {
    reference_text_datum(&id.to_string())
}

/// Convert the object serializer's flat map to ordered `(Field ...)` forms.
#[requires(true)]
#[ensures(true)]
fn object_field_datums(
    value: StructuralValue,
    provenance: ProvenanceDisposition,
    variables: &BTreeMap<String, Datum>,
) -> Vec<Datum> {
    let data!(StructuralValue::Map(entries)) = value.into_data() else {
        panic!("SemanticObject must serialize as a field map")
    };
    entries
        .into_iter()
        .filter_map(|(key, value)| {
            let name = structural_key_text(key);
            if name == "type" || field_is_suppressed(&name, None, provenance) {
                return None;
            }
            Some(match semantic_object_field_name(&name) {
                Some(known) => field_datum(known, value, variables, provenance),
                None => dynamic_field_datum(name, value, variables, provenance),
            })
        })
        .collect()
}

/// Extract an object map key without interpreting its semantic content.
#[requires(true)]
#[ensures(!ret.is_empty())]
fn structural_key_text(key: StructuralValue) -> String {
    // SemanticObject's custom serializer guarantees string keys. Retain the
    // owned spelling here so a newly added field that has not yet reached the
    // fixed public-name table is still rendered losslessly.
    let data!(StructuralValue::String(key)) = key.into_data() else {
        panic!("SemanticObject field keys must serialize as strings")
    };
    assert!(!key.is_empty(), "SemanticObject field keys must be named");
    key
}

/// Return the exact authored object field name for a serializer map key.
#[requires(true)]
#[ensures(ret.is_some() -> ret.unwrap() == name)]
fn semantic_object_field_name(name: &str) -> Option<&'static str> {
    const NAMES: &[&str] = &[
        "type",
        "force",
        "speaker",
        "audience",
        "eventuality",
        "content",
        "deicticGround",
        "asides",
        "vocativeKind",
        "items",
        "connectionClaims",
        "boundEventualities",
        "ordinalLabels",
        "relation",
        "nonlogicalConnection",
        "elidedConnectionOperand",
        "denotation",
        "class",
        "actuality",
        "tenseModal",
        "time",
        "timePath",
        "timeInterval",
        "timeSpan",
        "aspect",
        "aspects",
        "recurrence",
        "intervalModifiers",
        "space",
        "spacePath",
        "spaceInterval",
        "spatialAspect",
        "spatialAspects",
        "spatialRecurrence",
        "spatialIntervalModifiers",
        "category",
        "scopeDependence",
        "sort",
        "indexical",
        "descriptor",
        "composition",
        "relativeClauses",
        "assignedNames",
        "adjuncts",
        "body",
        "parameters",
        "arity",
        "embeddedQuestions",
        "experiencer",
        "target",
        "scale",
        "subscript",
        "deicticReference",
        "personalMassMembership",
        "generatedReferent",
        "abstracted",
        "abstractionKind",
        "role",
        "introducedBy",
        "relationParameter",
        "tanruLink",
        "arguments",
        "placeQuestions",
        "reciprocity",
        "mode",
        "scalarNegation",
        "relationMetadata",
        "operator",
        "predication",
        "children",
        "connector",
        "variable",
        "sourceVariable",
        "selectionSource",
        "restriction",
        "domainImport",
        "quantity",
        "bindings",
        "coequalScope",
        "streams",
        "distinctPartition",
        "kind",
        "text",
        "letterals",
        "quotation",
        "denotes",
        "family",
        "intensity",
        "polarity",
        "phase",
        "modifiers",
        "assertionEffect",
        "targetFocus",
        "anchor",
        "literal",
        "operatorDenotes",
        "endpointInclusion",
        "operands",
        "operatorParameter",
        "form",
        "value",
        "comparisonSet",
        "sourceWords",
        "placeStructure",
        "expansion",
        "asker",
        "respondent",
        "domain",
        "slots",
        "focus",
        "presupposedAnswer",
        "source",
        "diagnostics",
    ];
    NAMES.iter().copied().find(|candidate| *candidate == name)
}

/// Whether a field is suppressed in the ordinary profile. Only the actual
/// `Adjunct.introducedBy` coordinate is provenance; identically named fields
/// on argument values, ordinals, names, recurrence, and other records carry
/// semantic state and must remain visible.
#[requires(true)]
#[ensures(ret == (provenance == ProvenanceDisposition::Suppress
    && ((container.is_none() && matches!(name, "source" | "diagnostics"))
        || (name == "source" && container.is_some_and(|container| source_link_surfaces().contains(&container)))
        || (container == Some("Adjunct") && matches!(name, "introducedBy" | "introduced_by")))))]
fn field_is_suppressed(
    name: &str,
    container: Option<&str>,
    provenance: ProvenanceDisposition,
) -> bool {
    provenance == ProvenanceDisposition::Suppress
        && ((container.is_none() && matches!(name, "source" | "diagnostics"))
            || (name == "source"
                && container.is_some_and(|container| source_link_surfaces().contains(&container)))
            || (container == Some("Adjunct") && matches!(name, "introducedBy" | "introduced_by")))
}

/// Construct one typed field form.
#[requires(true)]
#[ensures(matches!(ret, Datum::List(_)))]
fn field_datum(
    name: &str,
    value: StructuralValue,
    variables: &BTreeMap<String, Datum>,
    provenance: ProvenanceDisposition,
) -> Datum {
    Datum::form(
        "Field",
        [
            Datum::atom(pascal_case(name)),
            structural_datum(value, variables, provenance),
        ],
    )
}

/// Preserve a newly authored object-field key until its stable grammar atom is
/// registered. The explicit `Name` form avoids collisions from case folding.
#[requires(!name.is_empty())]
#[ensures(matches!(ret, Datum::List(_)))]
fn dynamic_field_datum(
    name: String,
    value: StructuralValue,
    variables: &BTreeMap<String, Datum>,
    provenance: ProvenanceDisposition,
) -> Datum {
    Datum::form(
        "Field",
        [
            Datum::form("Name", [Datum::String(name)]),
            structural_datum(value, variables, provenance),
        ],
    )
}

/// Convert structural serialization to the public datum grammar.
#[requires(true)]
#[ensures(true)]
fn structural_datum(
    value: StructuralValue,
    variables: &BTreeMap<String, Datum>,
    provenance: ProvenanceDisposition,
) -> Datum {
    match value.into_data() {
        data!(StructuralValue::Reference(id)) => variables
            .get(&id)
            .cloned()
            .unwrap_or_else(|| reference_text_datum(&id)),
        data!(StructuralValue::String(value)) => Datum::String(value),
        data!(StructuralValue::Bool(value)) => Datum::Bool(value),
        data!(StructuralValue::Signed(value)) => Datum::Signed(value),
        data!(StructuralValue::Unsigned(value)) => Datum::Unsigned(value),
        data!(StructuralValue::Float(value)) => Datum::Float(value),
        data!(StructuralValue::Unit) => Datum::atom("None"),
        data!(StructuralValue::Sequence(values)) => Datum::form(
            "List",
            values
                .into_iter()
                .map(|value| structural_datum(value, variables, provenance)),
        ),
        data!(StructuralValue::Map(entries)) => Datum::form(
            "Map",
            entries.into_iter().map(|(key, value)| {
                Datum::form(
                    "Entry",
                    [
                        structural_datum(key, variables, provenance),
                        structural_datum(value, variables, provenance),
                    ],
                )
            }),
        ),
        data!(StructuralValue::Record { name, fields }) => {
            let mut arguments = vec![Datum::atom(name)];
            arguments.extend(fields.into_iter().filter_map(|(field, value)| {
                if field_is_suppressed(field, Some(name), provenance) {
                    return None;
                }
                Some(Datum::form(
                    "Field",
                    [
                        Datum::atom(pascal_case(field)),
                        structural_datum(value, variables, provenance),
                    ],
                ))
            }));
            Datum::form("Record", arguments)
        }
        data!(StructuralValue::Variant {
            type_name,
            variant,
            value,
        }) => {
            let mut arguments = vec![Datum::atom(type_name), Datum::atom(pascal_case(variant))];
            arguments.extend(value.map(|value| structural_datum(*value, variables, provenance)));
            Datum::form("Variant", arguments)
        }
    }
}

/// Project tagged reference text into the `@` namespace.
#[requires(!id.is_empty())]
#[ensures(ret.as_atom().is_some_and(|atom| atom.starts_with('@')))]
fn reference_text_datum(id: &str) -> Datum {
    Datum::atom(format!("@{}", id.replace(':', "_")))
}

/// Convert a serde camelCase/snake-case field to its grammar field atom.
#[requires(!name.is_empty())]
#[ensures(!ret.is_empty())]
fn pascal_case(name: &str) -> String {
    let mut output = String::with_capacity(name.len());
    let mut uppercase_next = true;
    for character in name.chars() {
        if matches!(character, '_' | '-') {
            uppercase_next = true;
        } else if uppercase_next {
            output.extend(character.to_uppercase());
            uppercase_next = false;
        } else {
            output.push(character);
        }
    }
    output
}

/// Stable semantic object kind spelling.
#[requires(true)]
#[ensures(!ret.is_empty())]
fn kind_name(kind: SemanticObjectKind) -> &'static str {
    match kind {
        SemanticObjectKind::Utterance => "Utterance",
        SemanticObjectKind::Sequence => "Sequence",
        SemanticObjectKind::Eventuality => "Eventuality",
        SemanticObjectKind::Referent => "Referent",
        SemanticObjectKind::Parameter => "Parameter",
        SemanticObjectKind::Predication => "Predication",
        SemanticObjectKind::Formula => "Formula",
        SemanticObjectKind::Abstraction => "Abstraction",
        SemanticObjectKind::Sign => "Sign",
        SemanticObjectKind::DisplayedContent => "DisplayedContent",
        SemanticObjectKind::MathExpression => "MathExpression",
        SemanticObjectKind::Quantity => "Quantity",
        SemanticObjectKind::RelationMetadata => "RelationMetadata",
        SemanticObjectKind::Question => "Question",
    }
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use bityzba::{ensures, requires};

    use super::*;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn graph_id_tag_is_invisible_to_json_and_visible_structurally() {
        let id = SemanticObjectId::referent(7);
        assert_eq!(serde_json::to_string(&id).unwrap(), "\"entity:7\"");
        assert_eq!(
            id.serialize(StructuralSerializer).unwrap().as_data(),
            new!(StructuralValue::Reference("entity:7".to_owned())).as_data()
        );
        assert_eq!(reference_datum(id).as_atom(), Some("@entity_7"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn structural_floats_must_be_finite() {
        assert!(0.0_f32.serialize(StructuralSerializer).is_ok());
        assert!(f32::INFINITY.serialize(StructuralSerializer).is_err());
        assert!(f32::NAN.serialize(StructuralSerializer).is_err());
        assert!(0.0_f64.serialize(StructuralSerializer).is_ok());
        assert!(f64::NEG_INFINITY.serialize(StructuralSerializer).is_err());
        assert!(f64::NAN.serialize(StructuralSerializer).is_err());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn pascal_case_is_a_fixed_name_conversion() {
        assert_eq!(pascal_case("scopeDependence"), "ScopeDependence");
        assert_eq!(pascal_case("source_words"), "SourceWords");
        assert_eq!(pascal_case("veridicalDescription"), "VeridicalDescription");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn structural_enum_variants_use_the_intrinsic_namespace() {
        let value = typed_value_datum(
            &crate::model::PredicationMode::Asserted,
            ProvenanceDisposition::Suppress,
            &BTreeMap::new(),
        );
        let items = value.as_list().expect("variant is a typed form");
        assert_eq!(items[0].as_atom(), Some("Variant"));
        assert_eq!(items[2].as_atom(), Some("Asserted"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn unknown_object_fields_are_preserved_with_their_exact_name() {
        let fields = object_field_datums(
            new!(StructuralValue::Map(vec![(
                new!(StructuralValue::String("future_field".to_owned())),
                new!(StructuralValue::Bool(true)),
            )])),
            ProvenanceDisposition::Suppress,
            &BTreeMap::new(),
        );
        assert_eq!(fields.len(), 1);
        let field = fields[0].as_list().expect("field is a typed form");
        assert_eq!(field[0].as_atom(), Some("Field"));
        let name = field[1].as_list().expect("unknown key uses Name");
        assert_eq!(name[0].as_atom(), Some("Name"));
        assert_eq!(name[1], Datum::String("future_field".to_owned()));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn introduced_by_suppression_is_limited_to_adjunct_records() {
        let introduced_by = || {
            vec![(
                "introducedBy",
                new!(StructuralValue::String("fi'o".to_owned())),
            )]
        };
        let adjunct = structural_datum(
            new!(StructuralValue::Record {
                name: "Adjunct",
                fields: introduced_by()
            }),
            &BTreeMap::new(),
            ProvenanceDisposition::Suppress,
        );
        let argument = structural_datum(
            new!(StructuralValue::Record {
                name: "ArgumentValue",
                fields: introduced_by()
            }),
            &BTreeMap::new(),
            ProvenanceDisposition::Suppress,
        );
        assert_eq!(adjunct.count_forms("Field"), 0);
        assert_eq!(argument.count_forms("Field"), 1);
    }
}
